import Foundation

/// One audiobook's chapters, fetched a window at a time.
///
/// The public detail endpoint answers with the book's own metadata and one
/// window of chapters. This holds the rest: how many chapters there are, which
/// ones have arrived, and which windows are still on the way. The chapter list
/// renders a slot per chapter — a row where one has arrived, a skeleton where it
/// has not — and playback asks for a window before it needs it, so opening a
/// book at chapter 300 loads like opening it at chapter 1.
///
/// A window that never arrives is not an error the listener has to confront:
/// `ensure` answers `false`, the slot stays a skeleton, and the next ask (a
/// scroll, a chapter pick) tries again. Losing one request must never take
/// playback down, so nothing here throws.
@MainActor
final class ChapterStore: ObservableObject {
    /// Every chapter of the book, `nil` where a window has not arrived.
    @Published private(set) var chapters: [AudiobookTrack?]
    /// Window starts already in `chapters`.
    @Published private(set) var loadedWindows: Set<Int> = []
    /// Window starts being fetched right now.
    @Published private(set) var pendingWindows: Set<Int> = []
    /// A refresh of the book's chapter list is in flight.
    @Published private(set) var isRefreshing = false

    private let api: AudiobookAPI
    private let slug: String
    private var inflight: [Int: Task<Bool, Never>] = [:]

    init(slug: String, total: Int, api: AudiobookAPI) {
        self.slug = slug
        self.api = api
        self.chapters = Array(repeating: nil, count: max(0, total))
    }

    /// Chapters in the book, whether or not they have arrived.
    var total: Int { chapters.count }

    /// Every window has arrived.
    var isComplete: Bool {
        ChapterWindow.starts(count: total).allSatisfy { loadedWindows.contains($0) }
    }

    /// The chapter at `index`, or `nil` while its window is on its way.
    func track(at index: Int) -> AudiobookTrack? {
        guard chapters.indices.contains(index) else { return nil }
        return chapters[index]
    }

    /// Whether the window holding `index` is the one being fetched.
    func isPending(_ index: Int) -> Bool {
        pendingWindows.contains(ChapterWindow.start(of: index))
    }

    /// Install a window that arrived with the book's own details.
    func seed(_ tracks: [AudiobookTrack], at offset: Int = 0) {
        install(tracks, at: offset, total: nil)
    }

    /// Make the window holding `index` playable, fetching it if it has not
    /// arrived. Answers `true` once the chapter is there, `false` if the request
    /// failed. Concurrent asks for one window share a single request.
    @discardableResult
    func ensure(_ index: Int) async -> Bool {
        guard total > 0 else { return false }

        let start = ChapterWindow.start(of: index)
        if loadedWindows.contains(start) { return true }
        if let inflight = inflight[start] { return await inflight.value }

        pendingWindows.insert(start)
        // Created on the main actor, so the install below is too.
        let task = Task { () -> Bool in
            do {
                let details = try await api.details(slug: slug, tracksOffset: start)
                if details.trackCount == nil {
                    // A backend that predates windowed chapters ignores the
                    // window and answers with the whole book. Installing that
                    // at the offset it was asked for would shift every chapter,
                    // so it is taken as one window from the top.
                    install(details.tracks, at: 0, total: details.tracks.count)
                } else {
                    install(details.tracks, at: start, total: details.totalTracks)
                }
                return true
            } catch {
                return false
            }
        }
        inflight[start] = task

        let fetched = await task.value
        inflight[start] = nil
        pendingWindows.remove(start)
        // A window that came back without the chapter (the book shrank under
        // us) is not a failure to retry, but it is not a chapter either.
        return fetched && track(at: index) != nil
    }

    /// Re-read the book's chapter list.
    ///
    /// A book can gain chapters while it is open, and the client has no other
    /// way to hear about it: the store was sized from the count the book
    /// arrived with. The opening window is fetched again because every answer
    /// carries `track_count`, and the install regrows the slots to match. When
    /// the book has grown, the window holding the last chapter is fetched too,
    /// so the new rows arrive filled instead of waiting for a scroll.
    ///
    /// Concurrent calls share nothing: the second returns immediately, which
    /// keeps a poll and a button press from stacking requests.
    @discardableResult
    func refresh() async -> Bool {
        guard !isRefreshing else { return false }
        isRefreshing = true
        defer { isRefreshing = false }

        do {
            let before = total
            let details = try await api.details(slug: slug, tracksOffset: 0)
            guard details.trackCount != nil else {
                // A backend that predates windowed chapters answers with the
                // whole book, and its count is the list it just sent.
                install(details.tracks, at: 0, total: details.tracks.count)
                return true
            }

            install(details.tracks, at: 0, total: details.totalTracks)

            let lastStart = ChapterWindow.start(of: details.totalTracks - 1)
            if details.totalTracks > before, lastStart > 0 {
                let tail = try await api.details(slug: slug, tracksOffset: lastStart)
                install(tail.tracks, at: lastStart, total: tail.totalTracks)
            }
            return true
        } catch {
            // The list that is already on screen is still usable; a failed
            // refresh leaves it alone rather than emptying it.
            return false
        }
    }

    /// Keep the chapters just ahead of the playhead in hand, so the next
    /// chapter is ready before the current one ends.
    func prefetch(_ index: Int) {
        let ahead = index + ChapterWindow.prefetchAhead
        guard total > 0, ahead < total else { return }
        Task { await ensure(ahead) }
    }

    /// Write a fresh play count back into whichever chapter holds it.
    func setPlayCount(_ count: Int, trackId: Int64) {
        guard let index = chapters.firstIndex(where: { $0?.id == trackId }) else { return }
        chapters[index]?.playCount = count
    }

    private func install(_ tracks: [AudiobookTrack], at offset: Int, total reported: Int?) {
        // A chapter added or removed since the page was built changes the count
        // the windows were cut against, so the playlist is regrown to match,
        // keeping whatever has already arrived.
        if let reported, reported >= 0, reported != chapters.count {
            if reported > chapters.count {
                chapters.append(contentsOf: Array(repeating: nil, count: reported - chapters.count))
            } else {
                chapters.removeLast(chapters.count - reported)
                // Windows past the new end no longer hold anything, and a later
                // growth must not take their stale marks as "already fetched".
                loadedWindows = loadedWindows.filter { $0 < chapters.count }
            }
        }

        for (position, track) in tracks.enumerated() {
            let index = offset + position
            guard chapters.indices.contains(index) else { continue }
            // Polling re-reads windows the list already holds; only an actual
            // difference is worth a publish, or every poll would repaint the
            // chapter list for nothing.
            if chapters[index] != track { chapters[index] = track }
        }

        // Every window this answer covers counts as loaded — normally exactly
        // one, but a short book, or a whole-book answer from a backend that
        // predates windowed chapters, covers several. A window only counts when
        // the answer fills it to its end, so a partial one is not mistaken for
        // a complete one.
        let covered = offset + tracks.count
        for start in stride(from: ChapterWindow.start(of: offset), to: covered, by: ChapterWindow.size) {
            let end = min(start + ChapterWindow.size, chapters.count)
            if end > start, end <= covered { loadedWindows.insert(start) }
        }
    }
}
