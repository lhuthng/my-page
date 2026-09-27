import AppKit
import Foundation

/// In-app updates, straight from the GitHub release the app is published to.
///
/// There is no Sparkle here on purpose: the app is ad-hoc signed and shipped as
/// a plain zip, so there is no publisher identity to verify a signature against
/// anyway. What this does check is that the release is newer, that the archive
/// unpacks to a bundle with *our* identifier, and that the copy already on disk
/// is the one being replaced.
@MainActor
final class Updater: ObservableObject {
    /// What the About screen and the alert render.
    enum State: Equatable {
        case idle
        case checking
        case upToDate
        case available(Release)
        case downloading(Release)
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
    }

    /// The repo `Scripts/bundle.sh` releases from. Hard-coded on purpose: the
    /// app should not be steered anywhere else by a changed setting.
    private static let repo = "lhuthng/my-page"
    private static let assetSuffix = ".app.zip"

    @Published private(set) var state: State = .idle
    /// Set when an update is found, so the alert can be raised once and only
    /// once per launch however many times the check runs.
    @Published var shouldOffer = false

    private var task: Task<Void, Never>?

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
                if let release = try await self.latest(), release.version != AppInfo.version {
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
        let payload = try JSONDecoder().decode(ReleasePayload.self, from: data)
        guard let tag = payload.tagName, let version = Self.normalised(tag) else {
            throw UpdateError(message: "Release \(payload.tagName ?? "?") has no usable version")
        }
        guard let asset = payload.assets?.first(where: { $0.name.hasSuffix(Self.assetSuffix) })
        else {
            throw UpdateError(message: "Release \(version) has no \(Self.assetSuffix) asset")
        }
        return Release(
            version: version,
            downloadURL: asset.browserDownloadURL
        )
    }

    // MARK: - Installing

    /// Download, unpack, swap, relaunch. The running executable cannot be
    /// overwritten in place, so the installed bundle is moved aside first and
    /// only removed once the new copy is in place.
    func install(_ release: Release) {
        task?.cancel()
        task = Task { [weak self] in
            guard let self else { return }
            do {
                let archive = try await self.download(release)
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
        let (temporary, response) = try await URLSession.shared.download(from: release.downloadURL)
        if let http = response as? HTTPURLResponse, !(200..<300).contains(http.statusCode) {
            throw UpdateError(message: "Download returned \(http.statusCode)")
        }
        state = .downloading(release)
        return temporary
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
    static func normalised(_ raw: String) -> String? {
        let trimmed = raw.trimmingCharacters(in: .whitespacesAndNewlines)
        let body = trimmed.hasPrefix("v") ? String(trimmed.dropFirst()) : trimmed
        guard !body.isEmpty, body.allSatisfy({ $0.isNumber || $0 == "." }) else { return nil }
        return body
    }

    /// Numeric dotted compare, so `0.10.0` beats `0.9.0`. A component that is not
    /// a number sorts as 0, which only happens on a malformed tag.
    static func isNewer(_ candidate: String, than current: String) -> Bool {
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

/// Just the two fields the check needs out of the release payload.
private struct ReleasePayload: Decodable {
    struct Asset: Decodable {
        let name: String
        let browserDownloadURL: URL
    }

    let tagName: String?
    let assets: [Asset]?

    enum CodingKeys: String, CodingKey {
        case tagName = "tag_name"
        case assets
    }
}
