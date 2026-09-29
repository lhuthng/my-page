import AppKit
import CryptoKit
import Foundation

/// In-app updates, straight from the GitHub release the app is published to.
///
/// There is no Sparkle here on purpose: the app is ad-hoc signed and shipped as
/// a plain zip, so there is no publisher identity to verify a signature against
/// anyway. What this does check is that the release is newer, that the archive
/// is byte-for-byte the one the release publishes, that it unpacks to a bundle
/// with *our* identifier, and that the copy already on disk is the one being
/// replaced.
@MainActor
final class Updater: ObservableObject {
    /// What the About screen and the alert render.
    enum State: Equatable {
        case idle
        case checking
        case upToDate
        case available(Release)
        /// Carries how much of the archive has landed, 0...1.
        case downloading(Release, Double)
        case installing(Release)
        case failed(String)

        var isBusy: Bool {
            switch self {
            case .checking, .downloading, .installing: true
            default: false
            }
        }
    }

    struct Release: Equatable {
        let version: String
        let downloadURL: URL
        let digest: Digest
        /// The archive's size in bytes, when the API reports it: the number a
        /// reader weighs before starting the download.
        let size: Int64?
        /// The release's notes, as GitHub wrote them (Markdown), so the panel can
        /// say what actually changed instead of just that something did.
        let notes: String?
    }

    /// The archive's own bytes, in the `sha256:<hex>` spelling the GitHub API
    /// uses for a release asset.
    enum Digest: Equatable {
        case sha256(String)

        init?(_ raw: String) {
            let parts = raw.split(separator: ":", maxSplits: 1)
            guard parts.count == 2, parts[0] == "sha256" else { return nil }
            let hex = parts[1].lowercased()
            guard hex.count == 64, hex.allSatisfy({ $0.isHexDigit }) else { return nil }
            self = .sha256(hex)
        }
    }

    /// The repo `Scripts/bundle.sh` releases from. Hard-coded on purpose: the
    /// app should not be steered anywhere else by a changed setting.
    private static let repo = "lhuthng/my-page"
    private static let assetSuffix = ".app.zip"

    @Published private(set) var state: State = .idle
    /// Set when an update is found, so the alert is raised. The silent launch
    /// check raises it once per launch however many times it runs; an explicit
    /// check from About raises it again, since the author just asked.
    @Published var shouldOffer = false

    private var task: Task<Void, Never>?
    /// Held while an archive streams in, because the session keeps its delegate
    /// only weakly until the task finishes.
    private var downloader: ArchiveDownloader?

    // MARK: - Checking

    /// Silent check for the launch path: records an update without interrupting.
    /// A failure is swallowed on purpose — nobody wants an alert because a
    /// background ping did not land.
    func checkSilently() {
        task = Task { [weak self] in
            guard let self else { return }
            if case .available = self.state {
                self.shouldOffer = true
                return
            }
            do {
                if let release = try await self.latest(),
                    Self.isNewer(release.version, than: AppInfo.version)
                {
                    self.state = .available(release)
                    self.shouldOffer = true
                } else {
                    self.state = .upToDate
                }
            } catch {
                self.state = .idle
            }
        }
    }

    /// Checked from the About screen, so it reports what happened.
    func checkInteractively() {
        task?.cancel()
        state = .checking
        task = Task { [weak self] in
            guard let self else { return }
            do {
                guard let release = try await self.latest() else {
                    self.state = .upToDate
                    return
                }
                if Self.isNewer(release.version, than: AppInfo.version) {
                    self.state = .available(release)
                    // Asked for by hand, so answer the same way the launch
                    // check does: raise the alert rather than only updating
                    // the row the author may not be looking at.
                    self.shouldOffer = true
                } else {
                    self.state = .upToDate
                }
            } catch is CancellationError {
                return
            } catch {
                self.state = .failed(error.localizedDescription)
            }
        }
    }

    /// The newest published release, or nil when this build is already it.
    private func latest() async throws -> Release? {
        let url = URL(
            string: "https://api.github.com/repos/\(Self.repo)/releases/latest"
        )!
        var request = URLRequest(url: url)
        request.setValue("application/vnd.github+json", forHTTPHeaderField: "Accept")
        request.timeoutInterval = 15
        let (data, response) = try await URLSession.shared.data(for: request)
        guard let http = response as? HTTPURLResponse, (200..<300).contains(http.statusCode) else {
            throw UpdateError(message: "GitHub returned \((response as? HTTPURLResponse)?.statusCode ?? 0)")
        }
        return try Self.release(fromJSON: data)
    }

    /// Turn a release payload into the one `Release` this build would install.
    ///
    /// Separated from the fetch so every decision — the tag, the asset, the
    /// digest, the notes — can be exercised against a fixed payload, with no
    /// network involved. Every release this repo publishes carries a digest, so
    /// its absence is treated as a broken release rather than a reason to
    /// install unverified bytes; read here, not after the download, so a bad
    /// release fails before the archive is pulled down.
    nonisolated static func release(fromJSON data: Data) throws -> Release {
        let payload = try JSONDecoder().decode(ReleasePayload.self, from: data)
        guard let tag = payload.tagName, let version = normalised(tag) else {
            throw UpdateError(message: "Release \(payload.tagName ?? "?") has no usable version")
        }
        guard let asset = payload.assets?.first(where: { $0.name.hasSuffix(assetSuffix) }) else {
            throw UpdateError(message: "Release \(version) has no \(assetSuffix) asset")
        }
        guard let raw = asset.digest, let digest = Digest(raw) else {
            throw UpdateError(message: "Release \(version) publishes no sha256 digest to verify")
        }
        let notes = payload.body?.trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
        return Release(
            version: version,
            downloadURL: asset.browserDownloadURL,
            digest: digest,
            size: asset.size,
            notes: notes.isEmpty ? nil : notes
        )
    }

    // MARK: - Installing

    /// Download, verify, unpack, swap, relaunch. The running executable cannot
    /// be overwritten in place, so the installed bundle is moved aside first and
    /// only removed once the new copy is in place.
    func install(_ release: Release) {
        task?.cancel()
        task = Task { [weak self] in
            guard let self else { return }
            do {
                let archive = try await self.download(release)
                defer { try? FileManager.default.removeItem(at: archive) }
                // Nothing on disk is touched until the bytes are known to be
                // the ones the release published.
                try Self.check(archive, matches: release.digest)
                self.state = .installing(release)
                try self.swapIn(archive, release: release)
                await self.relaunch()
            } catch is CancellationError {
                return
            } catch {
                self.state = .failed(error.localizedDescription)
            }
        }
    }

    private func download(_ release: Release) async throws -> URL {
        // Announced before the first byte, so the row reports the download from
        // the start instead of jumping from "available" to "installing".
        state = .downloading(release, 0)
        let destination = FileManager.default.temporaryDirectory
            .appendingPathComponent("Audiobooks-\(release.version)-\(UUID().uuidString).zip")
        var request = URLRequest(url: release.downloadURL)
        request.timeoutInterval = 60

        let downloader = ArchiveDownloader(destination: destination) { [weak self] fraction in
            Task { @MainActor in
                guard let self, case .downloading = self.state else { return }
                self.state = .downloading(release, fraction)
            }
        }
        self.downloader = downloader
        defer { self.downloader = nil }
        return try await downloader.start(request)
    }

    /// Replace the installed bundle with the one inside `archive`.
    private func swapIn(_ archive: URL, release: Release) throws {
        let fm = FileManager.default
        let installed = Bundle.main.bundleURL
        let parent = installed.deletingLastPathComponent()
        let staging = parent.appendingPathComponent(".audiobooks-update-\(UUID().uuidString)")

        try fm.createDirectory(at: staging, withIntermediateDirectories: true)
        defer { try? fm.removeItem(at: staging) }
        try run("/usr/bin/ditto", ["-x", "-k", archive.path, staging.path])

        // The archive must hold a bundle with our identifier, or this is not our
        // app and must not be installed.
        let candidate = staging.appendingPathComponent("\(AppInfo.name).app")
        var isDir: ObjCBool = false
        guard fm.fileExists(atPath: candidate.path, isDirectory: &isDir), isDir.boolValue else {
            throw UpdateError(message: "Archive does not contain \(AppInfo.name).app")
        }
        let info = candidate.appendingPathComponent("Contents/Info.plist")
        guard
            let data = try? Data(contentsOf: info),
            let plist = try? PropertyListSerialization.propertyList(
                from: data, options: [], format: nil
            ) as? [String: Any],
            plist["CFBundleIdentifier"] as? String
                == Bundle.main.bundleIdentifier
        else {
            throw UpdateError(message: "Downloaded bundle is not \(AppInfo.name)")
        }

        guard fm.isWritableFile(atPath: parent.path) else {
            throw UpdateError(message: "Need write access to \(parent.path) to install")
        }

        // Move the old bundle aside rather than deleting it: if the copy fails
        // the app is still where it was, and the old one goes once the new one
        // has landed.
        let retired = parent.appendingPathComponent(".audiobooks-old-\(UUID().uuidString)")
        try fm.moveItem(at: installed, to: retired)
        do {
            try fm.moveItem(at: candidate, to: installed)
        } catch {
            try? fm.moveItem(at: retired, to: installed)
            throw error
        }
        try? fm.removeItem(at: retired)
    }

    // MARK: - Digests

    /// Refuse an archive that is not byte-for-byte what the release published.
    private static func check(_ archive: URL, matches digest: Digest) throws {
        switch digest {
        case .sha256(let expected):
            let actual = try sha256(of: archive)
            guard actual == expected else {
                throw UpdateError(
                    message: "Downloaded archive does not match the release digest"
                )
            }
        }
    }

    /// The file's SHA-256, hashed in chunks so an archive never has to sit whole
    /// in memory. Hex, lowercased, as the API spells it.
    private static func sha256(of url: URL) throws -> String {
        let handle = try FileHandle(forReadingFrom: url)
        defer { try? handle.close() }
        var hasher = SHA256()
        while let chunk = try handle.read(upToCount: 1 << 20), !chunk.isEmpty {
            hasher.update(data: chunk)
        }
        return hasher.finalize().map { String(format: "%02x", $0) }.joined()
    }

    /// Start the new copy, then quit. The old process is still holding the old
    /// bundle, which is why the swap above only moved it aside.
    private func relaunch() async {
        let url = Bundle.main.bundleURL
        let configuration = NSWorkspace.OpenConfiguration()
        configuration.createsNewApplicationInstance = true
        await withCheckedContinuation { continuation in
            NSWorkspace.shared.openApplication(
                at: url,
                configuration: configuration
            ) { _, _ in continuation.resume() }
        }
        NSApp.terminate(nil)
    }

    private func run(_ tool: String, _ args: [String]) throws {
        let process = Process()
        process.executableURL = URL(fileURLWithPath: tool)
        process.arguments = args
        process.standardOutput = FileHandle.nullDevice
        process.standardError = FileHandle.nullDevice
        try process.run()
        process.waitUntilExit()
        guard process.terminationStatus == 0 else {
            throw UpdateError(message: "\(URL(fileURLWithPath: tool).lastPathComponent) failed")
        }
    }

    // MARK: - Versions

    /// `v0.2.0` and `0.2.0` are the same version.
    nonisolated static func normalised(_ raw: String) -> String? {
        let trimmed = raw.trimmingCharacters(in: .whitespacesAndNewlines)
        let body = trimmed.hasPrefix("v") ? String(trimmed.dropFirst()) : trimmed
        guard !body.isEmpty, body.allSatisfy({ $0.isNumber || $0 == "." }) else { return nil }
        return body
    }

    /// Numeric dotted compare, so `0.10.0` beats `0.9.0`. A component that is not
    /// a number sorts as 0, which only happens on a malformed tag.
    nonisolated static func isNewer(_ candidate: String, than current: String) -> Bool {
        let parts: (String) -> [Int] = {
            $0.split(separator: ".").map { Int($0) ?? 0 }
        }
        let (a, b) = (parts(candidate), parts(current))
        for index in 0..<max(a.count, b.count) {
            let left = index < a.count ? a[index] : 0
            let right = index < b.count ? b[index] : 0
            if left != right { return left > right }
        }
        return false
    }
}

struct UpdateError: LocalizedError {
    let message: String
    var errorDescription: String? { message }
}

/// Streams a release archive to a file of our own, reporting how far it has
/// come.
///
/// `URLSession.download(from:)` says nothing until the whole file has landed,
/// which would leave the progress bar below frozen at zero for the length of
/// the download, so the delegate callbacks are read directly. The archive is
/// moved out of the system's temporary location because that file is removed
/// the moment `didFinishDownloadingTo` returns.
private final class ArchiveDownloader: NSObject, URLSessionDownloadDelegate {
    private let destination: URL
    private let onProgress: (Double) -> Void
    private var continuation: CheckedContinuation<URL, Error>?
    private var session: URLSession?
    /// Last fraction handed out, so the delegate's far more frequent callbacks
    /// become at most one update per percent.
    private var lastReported = -1.0

    init(destination: URL, onProgress: @escaping (Double) -> Void) {
        self.destination = destination
        self.onProgress = onProgress
    }

    func start(_ request: URLRequest) async throws -> URL {
        let session = URLSession(configuration: .default, delegate: self, delegateQueue: nil)
        self.session = session
        return try await withTaskCancellationHandler {
            try await withCheckedThrowingContinuation { continuation in
                self.continuation = continuation
                session.downloadTask(with: request).resume()
            }
        } onCancel: {
            // A cancelled install must not keep pulling the archive down; the
            // session reports the cancellation through `didCompleteWithError`.
            session.invalidateAndCancel()
        }
    }

    func urlSession(
        _ session: URLSession,
        downloadTask: URLSessionDownloadTask,
        didWriteData bytesWritten: Int64,
        totalBytesWritten: Int64,
        totalBytesExpectedToWrite: Int64
    ) {
        // An unknown length (chunked response) cannot be turned into a fraction.
        guard totalBytesExpectedToWrite > 0 else { return }
        let fraction = Double(totalBytesWritten) / Double(totalBytesExpectedToWrite)
        guard fraction - lastReported >= 0.01 else { return }
        lastReported = fraction
        onProgress(fraction)
    }

    func urlSession(
        _ session: URLSession,
        downloadTask: URLSessionDownloadTask,
        didFinishDownloadingTo location: URL
    ) {
        do {
            try? FileManager.default.removeItem(at: destination)
            try FileManager.default.moveItem(at: location, to: destination)
        } catch {
            finish(.failure(error))
        }
    }

    func urlSession(
        _ session: URLSession,
        task: URLSessionTask,
        didCompleteWithError error: Error?
    ) {
        if let error {
            finish(.failure(error))
        } else if let http = task.response as? HTTPURLResponse,
            !(200..<300).contains(http.statusCode)
        {
            finish(.failure(UpdateError(message: "Download returned \(http.statusCode)")))
        } else {
            finish(.success(destination))
        }
    }

    private func finish(_ result: Result<URL, Error>) {
        session?.finishTasksAndInvalidate()
        session = nil
        guard let continuation else { return }
        self.continuation = nil
        // A failed download leaves nothing worth unpacking.
        if case .failure = result { try? FileManager.default.removeItem(at: destination) }
        continuation.resume(with: result)
    }
}

/// Only the fields the updater reads out of the release payload: the tag, the
/// notes, and the archive's name, size, digest, and download link.
private struct ReleasePayload: Decodable {
    struct Asset: Decodable {
        let name: String
        let size: Int64?
        let browserDownloadURL: URL
        /// `sha256:<hex>`; optional because the API marks it so.
        let digest: String?

        // GitHub spells these keys with underscores. Without the mapping the
        // decoder looks for "browserDownloadURL", finds nothing, and every
        // check fails with "The data couldn't be read because it is missing".
        enum CodingKeys: String, CodingKey {
            case name
            case size
            case digest
            case browserDownloadURL = "browser_download_url"
        }
    }

    let tagName: String?
    let assets: [Asset]?
    /// The release's notes, in Markdown. Absent on a release with no body.
    let body: String?

    enum CodingKeys: String, CodingKey {
        case tagName = "tag_name"
        case assets
        case body
    }
}
