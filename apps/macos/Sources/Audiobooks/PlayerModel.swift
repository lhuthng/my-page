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
        guard let book else { return false }
        return trackIndex < book.tracks.count - 1
    }
    var currentTrack: AudiobookTrack? {
        guard let book, book.tracks.indices.contains(trackIndex) else { return nil }
        return book.tracks[trackIndex]
    }
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

        var index = 0
        var resumeAt: Double?
        if let saved = progressStore.progress(for: details.slug),
           let savedIndex = details.tracks.firstIndex(where: { $0.id == saved.trackId }) {
            index = savedIndex
            resumeAt = saved.time > 3 ? saved.time : nil
            if saved.rate > 0 { rate = saved.rate }
        }
        trackIndex = index
        // Held, not played: the page comes up on the saved position, silent.
        loadCurrent(autoplay: false, resumeAt: resumeAt)
        if let resumeAt, details.tracks.indices.contains(index) {
            resumePrompt = ResumePrompt(
                time: resumeAt,
                chapter: Int(details.tracks[index].number)
            )
        }
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

    /// Start a specific chapter from its beginning.
    func playTrack(at index: Int) {
        guard let book, book.tracks.indices.contains(index) else { return }
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
            player.replaceCurrentItem(with: nil)
            return
        }
        durationTask?.cancel()
        artworkTask?.cancel()

        let item = AVPlayerItem(url: url)
        player.replaceCurrentItem(with: item)
        pendingSeekTarget = nil
        displayTime = 0
        lastSavedTime = 0

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
        if abs(seconds - lastSavedTime) >= 4 {
            lastSavedTime = seconds
            saveProgress()
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
