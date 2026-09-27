import Foundation

/// Per-book resume point, mirroring the web player's localStorage payload.
struct PlaybackProgress: Codable {
    var trackId: Int64
    var trackIndex: Int
    var time: Double
    var rate: Double
    var updatedAt: Date
}

/// On-device progress keyed by audiobook slug, persisted as one small JSON
/// file in Application Support.
@MainActor
final class ProgressStore {
    static let shared = ProgressStore()

    private var progress: [String: PlaybackProgress] = [:]
    private let fileURL: URL?

    private init() {
        let dirs = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)
        let dir = dirs.first?.appendingPathComponent("Audiobooks", isDirectory: true)
        if let dir {
            try? FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        }
        fileURL = dir?.appendingPathComponent("progress.json")
        load()
    }

    func progress(for slug: String) -> PlaybackProgress? {
        progress[slug]
    }

    func save(_ entry: PlaybackProgress, for slug: String) {
        progress[slug] = entry
        persist()
    }

    private func load() {
        guard let fileURL, let data = try? Data(contentsOf: fileURL) else { return }
        let decoder = JSONDecoder()
        decoder.dateDecodingStrategy = .iso8601
        progress = (try? decoder.decode([String: PlaybackProgress].self, from: data)) ?? [:]
    }

    private func persist() {
        guard let fileURL else { return }
        let encoder = JSONEncoder()
        encoder.dateEncodingStrategy = .iso8601
        if let data = try? encoder.encode(progress) {
            try? data.write(to: fileURL, options: .atomic)
        }
    }
}
