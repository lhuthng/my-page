import Foundation

struct APIError: LocalizedError {
    let message: String
    var errorDescription: String? { message }
}

/// Client for the public audiobook REST surface. Reading is unauthenticated;
/// override the base URL with `AUDIOBOOKS_API_BASE`.
struct AudiobookAPI {
    static let shared = AudiobookAPI()

    let base: URL

    init(base: URL? = nil) {
        let env = ProcessInfo.processInfo.environment["AUDIOBOOKS_API_BASE"]
        self.base = base ?? URL(string: env ?? "") ?? URL(string: "https://api.huuthangle.site")!
    }

    /// Re-root a backend-relative media path (`media/i/<short_name>`) against
    /// the API base, mirroring the web frontend's `fixClientRoute`.
    static func mediaURL(_ path: String?) -> URL? {
        guard let path, !path.isEmpty else { return nil }
        if path.hasPrefix("http://") || path.hasPrefix("https://") { return URL(string: path) }
        guard let resolved = URL(string: path, relativeTo: shared.base) else { return nil }
        return resolved.absoluteURL
    }

    func list(term: String?, tag: String?, limit: Int, offset: Int) async throws -> AudiobookListPage {
        var items = [
            URLQueryItem(name: "limit", value: String(limit)),
            URLQueryItem(name: "offset", value: String(offset)),
        ]
        if let term, !term.isEmpty { items.append(URLQueryItem(name: "term", value: term)) }
        if let tag, !tag.isEmpty { items.append(URLQueryItem(name: "tag", value: tag)) }
        var components = URLComponents(
            url: base.appendingPathComponent("audiobooks/public/all"),
            resolvingAgainstBaseURL: false
        )!
        components.queryItems = items
        return try await get(components.url!)
    }

    /// A book's details plus one window of its chapters.
    ///
    /// The window is what keeps a long book cheap to open: the answer carries
    /// `track_count` and `has_more_tracks` alongside the chapters it holds, so
    /// the player knows how many there are without pulling them all. The rest
    /// are fetched by `ChapterStore` as they are reached.
    func details(
        slug: String,
        tracksOffset: Int = 0,
        tracksLimit: Int = ChapterWindow.size
    ) async throws -> AudiobookDetails {
        var components = URLComponents(
            url: base.appendingPathComponent("audiobooks/public/s/\(slug)"),
            resolvingAgainstBaseURL: false
        )!
        components.queryItems = [
            URLQueryItem(name: "tracks_offset", value: String(tracksOffset)),
            URLQueryItem(name: "tracks_limit", value: String(tracksLimit)),
        ]

        let envelope: AudiobookDetailEnvelope = try await get(components.url!)
        return envelope.audiobook
    }

    /// Fire-and-forget play beacon for a chapter. The server counts one play per
    /// ten seconds of listening, per chapter, with no cap, and answers with the
    /// chapter's new total. It answers 204 when it did not count — the report
    /// landed inside its own ten-second window, or the book is not published —
    /// so a `nil` return means the counter did not move and the caller must not
    /// invent a play locally.
    ///
    /// The request carries no listener identity: the server measures listening
    /// time, not unique listeners, and keeps no per-listener record.
    func recordPlay(audiobookId: Int64, trackId: Int64) async throws -> Int64? {
        var request = URLRequest(
            url: base.appendingPathComponent(
                "audiobooks/id/\(audiobookId)/tracks/\(trackId)/play"
            )
        )
        request.httpMethod = "POST"
        let (data, response) = try await URLSession.shared.data(for: request)
        guard let http = response as? HTTPURLResponse else {
            throw APIError(message: "Invalid response from server")
        }
        if http.statusCode == 204 { return nil }
        guard (200..<300).contains(http.statusCode) else {
            throw APIError(message: "Server returned \(http.statusCode)")
        }
        let decoder = JSONDecoder()
        decoder.keyDecodingStrategy = .convertFromSnakeCase
        guard let body = try? decoder.decode(TrackPlayResponse.self, from: data) else {
            // A 200 we cannot read says nothing about the counter, so treat it
            // as "not counted" rather than guess a number.
            return nil
        }
        return body.playCount
    }

    private func get<T: Decodable>(_ url: URL) async throws -> T {
        let (data, response) = try await URLSession.shared.data(from: url)
        guard let http = response as? HTTPURLResponse else {
            throw APIError(message: "Invalid response from server")
        }
        guard (200..<300).contains(http.statusCode) else {
            throw APIError(message: "Server returned \(http.statusCode)")
        }
        let decoder = JSONDecoder()
        decoder.keyDecodingStrategy = .convertFromSnakeCase
        do {
            return try decoder.decode(T.self, from: data)
        } catch {
            throw APIError(message: "Unexpected response format: \(error.localizedDescription)")
        }
    }
}
