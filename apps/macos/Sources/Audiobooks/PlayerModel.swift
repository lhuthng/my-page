import AppKit
import AVFoundation
import Foundation
import MediaPlayer

/// Drives one AVPlayer through a book's track list: auto-advance, scrubbing,
/// resume, playback rate, and Now Playing / remote-command integration.
@MainActor
final class PlayerModel: NSObject, ObservableObject {
    enum Phase: Equatable {
        case empty
        case loading(title: String)
        case ready
        case failed(message: String)
    }

    @Published private(set) var phase: Phase = .empty
    @Published private(set) var book: AudiobookDetails?
    /// The book's chapters, fetched a window at a time, so a long book opens
    /// on its first window and the rest arrive as they are reached.
    @Published private(set) var chapters: ChapterStore?
    /// The chapter waiting on a window, or nil. The list renders that row as a
    /// skeleton until it lands.
    @Published private(set) var pendingChapterIndex: Int?
    @Published private(set) var trackIndex = 0
    @Published private(set) var isPlaying = false
    @Published var displayTime: Double = 0
    @Published private(set) var duration: Double = 0
    @Published var rate: Double = 1 {
        didSet {
            guard oldValue != rate else { return }
            if isPlaying { player.rate = Float(rate) }
            updateNowPlaying()
            saveProgress()
        }
    }

    /// A book opened on a saved position: nothing plays until it is answered.
    struct ResumePrompt: Equatable {
        /// Seconds into the chapter where the book was left off.
        let time: Double
        /// The chapter that point sits in, numbered as the list numbers it.
        let chapter: Int
    }

    @Published private(set) var resumePrompt: ResumePrompt?

    /// True while a scrubber is dragged; time updates must not fight it.
    var isScrubbing = false

    var hasPrevious: Bool {
        guard book != nil else { return false }
        return trackIndex > 0
    }

    var hasNext: Bool {
        guard book != nil else { return false }
        return trackIndex < chapterCount - 1
    }

    /// Chapters in the loaded book, whether or not their windows have arrived.
    var chapterCount: Int { chapters?.total ?? book?.tracks.count ?? 0 }

    var currentTrack: AudiobookTrack? { chapters?.track(at: trackIndex) }
    var remainingTime: Double { max(0, duration - displayTime) }

    private let api: AudiobookAPI
    private let player = AVPlayer()
    private let progressStore = ProgressStore.shared
    private var timeObserver: Any?
    private var endObserver: NSObjectProtocol?
    private var terminateObserver: NSObjectProtocol?
    private var statusObservation: NSKeyValueObservation?
    private var durationTask: Task<Void, Never>?
    private var artworkTask: Task<Void, Never>?
    private var lastSavedTime: Double = 0
    private var scrubTarget: Double = 0
    /// Set while a seek is in flight: the player reports its old playhead until
    /// the seek lands, so ticks are ignored.
    private var pendingSeekTarget: Double?
    private var pendingSummary: AudiobookSummary?
    /// Listening seconds accumulated on the current chapter since the last
    /// report. Carries the remainder across a report rather than being zeroed,
    /// so the cadence does not drift with the tick boundaries.
    private var playedSeconds: Double = 0
    /// The playhead at the previous tick, so only real elapsed audio time counts.
    private var lastTickSeconds: Double = 0
    private var playReportTask: Task<Void, Never>?

    /// Real seconds of listening per reported play; mirrors the web player's
    /// interval so both surfaces measure the same way.
    ///
    /// Deliberately listener time, not audio time: `tick(at:)` divides the
    /// playhead advance by the playback rate, so ten seconds is ten seconds at
    /// 0.75x and at 2x. There is no once-per-chapter cap — a chapter played for
    /// an hour reports 360 times, because the number measures how long somebody
    /// listened, not how many people opened it.
    private static let playReportIntervalSeconds: Double = 10

    init(api: AudiobookAPI) {
        self.api = api
        super.init()

        statusObservation = player.observe(\.timeControlStatus, options: [.new]) {
            avPlayer, _ in
            let playing = avPlayer.timeControlStatus == .playing
            Task { @MainActor in
                self.isPlaying = playing
                // Pauses, stalls, and ends all want a checkpoint.
                if !playing { self.saveProgress() }
            }
        }

        timeObserver = player.addPeriodicTimeObserver(
            forInterval: CMTime(seconds: 0.5, preferredTimescale: 600),
            queue: .main
        ) { [weak self] time in
            Task { @MainActor in self?.tick(at: time.seconds) }
        }

        endObserver = NotificationCenter.default.addObserver(
            forName: .AVPlayerItemDidPlayToEndTime,
            object: nil,
            queue: .main
        ) { [weak self] _ in
            Task { @MainActor in self?.trackDidEnd() }
        }

        terminateObserver = NotificationCenter.default.addObserver(
            forName: NSApplication.willTerminateNotification,
            object: nil,
            queue: .main
        ) { [weak self] _ in
            Task { @MainActor in self?.saveProgress() }
        }

        registerRemoteCommands()
    }

    deinit {
        if let timeObserver { player.removeTimeObserver(timeObserver) }
        if let endObserver { NotificationCenter.default.removeObserver(endObserver) }
        if let terminateObserver { NotificationCenter.default.removeObserver(terminateObserver) }
        playReportTask?.cancel()
    }

    // MARK: - Opening books

    /// Show the loading state without starting the fetch, so the page the card
    /// slides to has something on it. The load runs after the slide, via `open`.
    func prepare(_ summary: AudiobookSummary) {
        guard book?.id != summary.id else { return }
        pendingSummary = summary
        resumePrompt = nil
        phase = .loading(title: summary.title)
        displayTime = 0
        duration = 0
    }

    /// Load a book and hold it at the saved point. Nothing plays on its own: a
    /// saved position is asked about first. Opening the loaded book is a no-op.
    func open(_ summary: AudiobookSummary) async {
        if book?.id == summary.id { return }
        pendingSummary = summary
        resumePrompt = nil
        phase = .loading(title: summary.title)
        displayTime = 0
        duration = 0
        do {
            let details = try await api.details(slug: summary.slug)
            guard pendingSummary?.id == summary.id else { return }
            applyDetails(details)
        } catch {
            guard pendingSummary?.id == summary.id else { return }
            phase = .failed(message: error.localizedDescription)
        }
    }

    func retry() async {
        guard let pendingSummary else { return }
        await open(pendingSummary)
    }

    private func applyDetails(_ details: AudiobookDetails) {
        book = details
        phase = .ready

        // The details carry the opening window; the store fetches the rest.
        let chapters = ChapterStore(slug: details.slug, total: details.totalTracks, api: api)
        chapters.seed(details.tracks)
        self.chapters = chapters

        var index = 0
        var resumeAt: Double?
        if let saved = progressStore.progress(for: details.slug) {
            // The track id is the reliable match, but the saved chapter may sit
            // in a window that has not arrived — its index is then what finds
            // it, and `loadCurrent` waits for that window.
            if let savedIndex = details.tracks.firstIndex(where: { $0.id == saved.trackId }) {
                index = savedIndex
            } else if chapters.total > 0 {
                index = min(max(0, saved.trackIndex), chapters.total - 1)
            }
            resumeAt = saved.time > 3 ? saved.time : nil
            if saved.rate > 0 { rate = saved.rate }
        }
        trackIndex = index
        // Held, not played: the page comes up on the saved position, silent.
        loadCurrent(autoplay: false, resumeAt: resumeAt)
        if let resumeAt {
            resumePrompt = ResumePrompt(time: resumeAt, chapter: chapterNumber(at: index))
        }
    }

    /// A chapter's own number, or its place in the book while the window that
    /// would say is still on its way — chapters are numbered from one.
    private func chapterNumber(at index: Int) -> Int {
        if let number = chapters?.track(at: index)?.number { return Int(number) }
        return index + 1
    }

    // MARK: - The saved position

    /// Carry on from where the book was left.
    func resumeFromSavedPoint() {
        guard let prompt = resumePrompt else { return }
        resumePrompt = nil
        seek(to: prompt.time)
        player.playImmediately(atRate: Float(rate))
        updateNowPlaying()
    }

    /// Restart from the first chapter, held there.
    func restartFromBeginning() {
        guard resumePrompt != nil else { return }
        resumePrompt = nil
        trackIndex = 0
        loadCurrent(autoplay: false)
    }

    /// Drop the question unanswered; the book stays where it was left, paused.
    func dismissResumePrompt() {
        resumePrompt = nil
    }

    /// Ask for the window holding `index`.
    ///
    /// The chapter list calls this as rows scroll into view, which is what
    /// turns scrolling into loading; playback asks through `loadCurrent`. A
    /// window already loaded answers immediately, so firing this per row is
    /// cheap.
    func windowNeeded(at index: Int) {
        guard let chapters, index >= 0, index < chapters.total else { return }
        Task { await chapters.ensure(index) }
    }

    /// Start a specific chapter from its beginning.
    ///
    /// The pick counts whether or not the chapter's window has arrived:
    /// `loadCurrent` waits for it rather than letting the press do nothing.
    func playTrack(at index: Int) {
        guard book != nil, index >= 0, index < chapterCount else { return }
        trackIndex = index
        loadCurrent(autoplay: true)
    }

    // MARK: - Transport

    func togglePlayPause() {
        guard currentTrack != nil else { return }
        if isPlaying {
            player.pause()
        } else {
            // Playing by any route answers the saved-position question.
            clearResumePrompt()
            player.playImmediately(atRate: Float(rate))
        }
        updateNowPlaying()
    }

    /// Drop the question because something else decided where playback is.
    private func clearResumePrompt() {
        if resumePrompt != nil { resumePrompt = nil }
    }

    func previous() {
        guard hasPrevious else { return }
        trackIndex -= 1
        loadCurrent(autoplay: true)
    }

    func next() {
        guard hasNext else { return }
        trackIndex += 1
        loadCurrent(autoplay: true)
    }

    func skip(by delta: Double) {
        guard book != nil else { return }
        seek(to: displayTime + delta)
    }

    // MARK: - Scrubbing
    // Preview while dragging, commit the captured target on release, so a release
    // without a usable position never seeks to a stale or zero value.

    func scrubPreview(to time: Double) {
        isScrubbing = true
        scrubTarget = min(max(time, 0), max(duration, 0))
        displayTime = scrubTarget
    }

    func scrubCommit() {
        guard isScrubbing else { return }
        isScrubbing = false
        seek(to: scrubTarget)
    }

    func seek(to time: Double) {
        let clamped = min(max(time, 0), max(duration, 0))
        clearResumePrompt()
        pendingSeekTarget = clamped
        displayTime = clamped
        player.seek(
            to: CMTime(seconds: clamped, preferredTimescale: 600),
            toleranceBefore: .zero,
            toleranceAfter: .zero
        ) { [weak self] _ in
            Task { @MainActor in
                guard let self, self.pendingSeekTarget == clamped else { return }
                self.pendingSeekTarget = nil
                self.displayTime = clamped
                self.lastSavedTime = clamped
                self.saveProgress()
                self.updateNowPlaying()
            }
        }
    }

    func saveNow() {
        saveProgress()
    }

    // MARK: - Internals

    private func loadCurrent(autoplay: Bool, resumeAt: Double? = nil) {
        // Playing another chapter answers the question too.
        if autoplay { clearResumePrompt() }
        guard let track = currentTrack, let url = track.streamURL else {
            // Nothing to play yet. When the chapter exists but its window has
            // not arrived, wait for it and come back: a book opened at chapter
            // 300 then loads exactly like one opened at chapter 1. `track`
            // itself gates the retry, so a chapter that never arrives leaves
            // the player on the one it had rather than looping.
            if let chapters, trackIndex < chapters.total, chapters.track(at: trackIndex) == nil {
                pendingChapterIndex = trackIndex
                let wanted = trackIndex
                Task { [weak self] in
                    guard let self else { return }
                    let ready = await chapters.ensure(wanted)
                    guard !Task.isCancelled, self.trackIndex == wanted else { return }
                    self.pendingChapterIndex = nil
                    if ready { self.loadCurrent(autoplay: autoplay, resumeAt: resumeAt) }
                }
            } else {
                player.replaceCurrentItem(with: nil)
            }
            return
        }
        pendingChapterIndex = nil
        durationTask?.cancel()
        artworkTask?.cancel()

        let item = AVPlayerItem(url: url)
        player.replaceCurrentItem(with: item)
        pendingSeekTarget = nil
        displayTime = 0
        lastSavedTime = 0
        // The listening clock is per chapter, so a partial interval is dropped
        // at the boundary — the same as the web player's `load`.
        playedSeconds = 0
        lastTickSeconds = 0

        if let reported = track.durationSeconds, reported > 0 {
            duration = Double(reported)
        } else {
            duration = 0
            let asset = item.asset
            durationTask = Task { [weak self] in
                if let loaded = try? await asset.load(.duration) {
                    await MainActor.run {
                        guard !Task.isCancelled, loaded.seconds.isFinite, loaded.seconds > 0
                        else { return }
                        self?.duration = loaded.seconds
                        self?.updateNowPlaying()
                    }
                }
            }
        }

        if let resumeAt {
            // The playhead lands on the saved position even when nothing plays.
            displayTime = resumeAt
            lastSavedTime = resumeAt
            // The completion arrives off the main actor; hop back before touching
            // `rate` or the player.
            player.seek(to: CMTime(seconds: resumeAt, preferredTimescale: 600)) {
                [weak self] _ in
                Task { @MainActor in
                    guard let self, autoplay else { return }
                    self.player.playImmediately(atRate: Float(self.rate))
                }
            }
        } else if autoplay {
            player.playImmediately(atRate: Float(rate))
        }
        // Keep the chapters just ahead in hand, so the seam between two
        // chapters is not where the network shows up.
        chapters?.prefetch(trackIndex)
        updateNowPlaying()
        loadArtwork()
        saveProgress()
    }

    private func trackDidEnd() {
        guard book != nil else { return }
        if hasNext {
            trackIndex += 1
            loadCurrent(autoplay: true)
        } else {
            isPlaying = false
            displayTime = duration
            saveProgress()
            updateNowPlaying()
        }
    }

    private func tick(at seconds: Double) {
        if !isScrubbing, pendingSeekTarget == nil {
            displayTime = seconds
        }
        // Measure real listening: only advance while actually playing, and let
        // the delta guard absorb the jump a seek produces (a paused seek also
        // moves the playhead, which is not listening).
        //
        // The playhead advances at `rate` audio seconds per wall-clock second,
        // so dividing gives the listener's own seconds — the report cadence is
        // then the same at every speed. A rate of 0 is harmless: the quotient is
        // infinite and fails the guard below.
        let delta = seconds - lastTickSeconds
        lastTickSeconds = seconds
        let listened = delta / rate
        if isPlaying, pendingSeekTarget == nil, listened > 0, listened < 5 {
            accumulatePlay(listened)
        }
        if abs(seconds - lastSavedTime) >= 4 {
            lastSavedTime = seconds
            saveProgress()
        }
    }

    /// Add a stretch of genuine listening, reporting once per whole interval.
    ///
    /// Uncapped: the loop keeps firing for as long as the listener keeps going.
    /// The remainder is subtracted rather than zeroed so the next report lands a
    /// full interval later, not at the next tick after a rounded-up one.
    private func accumulatePlay(_ listened: Double) {
        playedSeconds += listened
        while playedSeconds >= Self.playReportIntervalSeconds {
            playedSeconds -= Self.playReportIntervalSeconds
            reportPlay()
        }
    }

    /// Fire one play beacon for the current chapter.
    ///
    /// The server owns the count. It answers with the chapter's new total, or
    /// 204 when it did not count (the report landed inside its own ten-second
    /// window, the chapter is unknown, or the book is not published) — in which
    /// case the row must be left alone rather than incremented, or the list
    /// would show a play that never happened.
    private func reportPlay() {
        guard let book, let track = currentTrack else { return }
        let bookId = book.id
        let trackId = track.id
        playReportTask?.cancel()
        playReportTask = Task { [weak self] in
            guard let self else { return }
            guard let total = try? await self.api.recordPlay(audiobookId: bookId, trackId: trackId)
            else { return }
            guard !Task.isCancelled, self.book?.id == bookId else { return }
            // Written back wherever the chapter actually lives: `book.tracks`
            // holds only the window the book opened with.
            self.chapters?.setPlayCount(Int(total), trackId: trackId)
        }
    }

    private func saveProgress() {
        guard let book, let track = currentTrack else { return }
        progressStore.save(
            PlaybackProgress(
                trackId: track.id,
                trackIndex: trackIndex,
                time: displayTime,
                rate: rate,
                updatedAt: Date()
            ),
            for: book.slug
        )
    }

    private func updateNowPlaying() {
        guard let book, let track = currentTrack else {
            MPNowPlayingInfoCenter.default().nowPlayingInfo = nil
            return
        }
        MPNowPlayingInfoCenter.default().nowPlayingInfo = [
            MPMediaItemPropertyTitle: track.title,
            MPMediaItemPropertyAlbumTitle: book.title,
            MPMediaItemPropertyArtist: book.translator ?? book.ownerDisplayName,
            MPMediaItemPropertyPlaybackDuration: duration,
            MPNowPlayingInfoPropertyElapsedPlaybackTime: displayTime,
            MPNowPlayingInfoPropertyPlaybackRate: isPlaying ? rate : 0,
            MPNowPlayingInfoPropertyDefaultPlaybackRate: rate,
        ]
    }

    private func loadArtwork() {
        artworkTask?.cancel()
        guard let book, let coverURL = book.coverURL else { return }
        let bookId = book.id
        artworkTask = Task { [weak self] in
            guard let image = await ImageCache.shared.image(at: coverURL) else { return }
            await MainActor.run {
                guard !Task.isCancelled, self?.book?.id == bookId else { return }
                let artwork = MPMediaItemArtwork(boundsSize: image.size) { _ in image }
                MPNowPlayingInfoCenter.default().nowPlayingInfo?[MPMediaItemPropertyArtwork] =
                    artwork
            }
        }
    }

    private func registerRemoteCommands() {
        let center = MPRemoteCommandCenter.shared()
        center.playCommand.addTarget { [weak self] _ in
            Task { @MainActor in
                if let self, self.book != nil, !self.isPlaying { self.togglePlayPause() }
            }
            return .success
        }
        center.pauseCommand.addTarget { [weak self] _ in
            Task { @MainActor in
                if let self, self.isPlaying { self.togglePlayPause() }
            }
            return .success
        }
        center.togglePlayPauseCommand.addTarget { [weak self] _ in
            Task { @MainActor in self?.togglePlayPause() }
            return .success
        }
        center.nextTrackCommand.addTarget { [weak self] _ in
            Task { @MainActor in self?.next() }
            return .success
        }
        center.previousTrackCommand.addTarget { [weak self] _ in
            Task { @MainActor in self?.previous() }
            return .success
        }
        center.skipForwardCommand.preferredIntervals = [30]
        center.skipForwardCommand.addTarget { [weak self] _ in
            Task { @MainActor in self?.skip(by: 30) }
            return .success
        }
        center.skipBackwardCommand.preferredIntervals = [15]
        center.skipBackwardCommand.addTarget { [weak self] _ in
            Task { @MainActor in self?.skip(by: -15) }
            return .success
        }
        center.changePlaybackPositionCommand.addTarget { [weak self] event in
            guard let positionEvent = event as? MPChangePlaybackPositionCommandEvent else {
                return .commandFailed
            }
            Task { @MainActor in self?.seek(to: positionEvent.positionTime) }
            return .success
        }
    }
}
