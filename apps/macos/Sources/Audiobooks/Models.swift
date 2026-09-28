import Foundation

/// Wire models for the public audiobook API. JSON keys are snake_case; the
/// shared decoder uses `.convertFromSnakeCase`, so property names are camelCase.

struct AudiobookSummary: Identifiable, Codable, Hashable {
    let id: Int64
    let title: String
    let slug: String
    let description: String
    let translator: String?
    let status: String
    /// Backend-relative cover path (`media/i/<short_name>`).
    let url: String?
    let trackCount: Int64
    let totalDurationSeconds: Int64
    let tags: [String]
    let tagSlugs: [String]
    let createdAt: String
    let publishedAt: String?
    /// Absent keys decode to nil; the backend skips these when unset.
    let ownerUsername: String?
    let ownerDisplayName: String?

    var coverURL: URL? { AudiobookAPI.mediaURL(url) }
    var isVietnameseTranslation: Bool { tagSlugs.contains("vietnamese-translated") }
}

struct AudiobookTag: Codable, Hashable {
    let id: Int64
    let name: String
    let slug: String
    let description: String?
    let audiobookCount: Int64
}

struct AudiobookTrack: Identifiable, Codable, Hashable {
    let id: Int64
    let title: String
    let number: Int64
    /// May be null, in which case the asset's own duration is used once loaded.
    let durationSeconds: Int64?
    /// How many times listeners have actually played this chapter. Absent on
    /// older backends; PlayerModel bumps it locally when the server accepts a
    /// play beacon, so the list updates without a reload.
    var playCount: Int?
    let shortName: String
    /// Backend-relative stream path, served with HTTP Range support.
    let url: String
    let fileType: String

    var streamURL: URL? { AudiobookAPI.mediaURL(url) }
}

struct AudiobookDetails: Codable, Identifiable, Hashable {
    let id: Int64
    let title: String
    let slug: String
    let description: String
    let translator: String?
    let status: String
    let url: String?
    let totalDurationSeconds: Int64
    let ownerUsername: String
    let ownerDisplayName: String
    let tags: [AudiobookTag]
    /// `var` so an accepted play beacon can bump a chapter's count in place.
    var tracks: [AudiobookTrack]
    let createdAt: String
    let publishedAt: String?

    var coverURL: URL? { AudiobookAPI.mediaURL(url) }
}

struct AudiobookListPage: Codable {
    let audiobooks: [AudiobookSummary]
    /// Older backends predate `has_more`; treat its absence as "no more pages".
    let hasMore: Bool

    private enum CodingKeys: String, CodingKey {
        case audiobooks
        case hasMore
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        audiobooks = try container.decode([AudiobookSummary].self, forKey: .audiobooks)
        hasMore = try container.decodeIfPresent(Bool.self, forKey: .hasMore) ?? false
    }
}

struct AudiobookDetailEnvelope: Codable {
    let audiobook: AudiobookDetails
}
