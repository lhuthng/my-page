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

    func details(slug: String) async throws -> AudiobookDetails {
        let envelope: AudiobookDetailEnvelope = try await get(
            base.appendingPathComponent("audiobooks/public/s/\(slug)")
        )
        return envelope.audiobook
    }

    /// Fire-and-forget play beacon for a chapter. The server counts at most
    /// one play per listener per day and answers 204; the caller decides what
    /// an error means (PlayerModel treats it as "not counted").
    func recordPlay(audiobookId: Int64, trackId: Int64) async throws {
        var request = URLRequest(
            url: base.appendingPathComponent(
                "audiobooks/id/\(audiobookId)/tracks/\(trackId)/play"
            )
        )
        request.httpMethod = "POST"
        let (_, response) = try await URLSession.shared.data(for: request)
        guard let http = response as? HTTPURLResponse else {
            throw APIError(message: "Invalid response from server")
        }
        guard (200..<300).contains(http.statusCode) else {
            throw APIError(message: "Server returned \(http.statusCode)")
        }
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
