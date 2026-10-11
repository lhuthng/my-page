import AVFoundation
import CryptoKit
import Foundation
import libzstd

/// Chunked chapters, decoded on this machine.
///
/// A chapter may be uploaded as a sequence of independently compressed zstd
/// frames plus a table saying where each frame's raw bytes begin. Streaming that
/// through the backend works — the backend reassembles a range on request — but
/// it puts every played byte through the server. Reading the table here instead
/// means the app fetches chunks straight from the bucket and decompresses them
/// locally, so a play costs the server nothing.
///
/// **AVPlayer cannot be handed a chunk.** It plays a URL or a file, and it asks
/// for byte ranges. So the chunks are served to it through
/// `AVAssetResourceLoader`, under a scheme of our own: AVFoundation keeps doing
/// the buffering, seeking and timing, and asks this class for bytes, which it
/// answers by fetching and decompressing only the chunks that range covers.
/// Nothing about the player above this file changes — a chapter is a chapter.
///
/// The plain path stays the fallback everywhere: a chapter with no table, or a
/// chunk that will not load, plays exactly as it did before any of this existed.

/// One chunk as the table records it.
struct ChunkEntry: Codable, Hashable {
    let index: Int
    /// Where this chunk's raw bytes begin within the chapter.
    let offset: Int64
    /// Length of the chunk after decompression.
    let raw: Int64
    /// Length of the zstd frame as stored.
    let compressed: Int64
    /// SHA-256 of the chunk's raw bytes, so a decompression can be checked
    /// rather than trusted. Older tables omit it.
    let sha256: String?
    /// Where to fetch the frame: absolute and signed when the audio lives on the
    /// bucket, backend-relative when it does not.
    let url: String
}

/// A chapter's chunk table.
struct ChunkManifest: Codable {
    let version: Int
    let chunkSize: Int64
    /// Length of the chapter once reassembled, which is the length AVPlayer is
    /// told about: the compression is invisible to it.
    let rawSize: Int64
    let contentType: String
    let chunks: [ChunkEntry]

    /// The version this build understands. A newer one is treated as no table at
    /// all rather than guessed at.
    static let supportedVersion = 1

    var isUsable: Bool {
        version == Self.supportedVersion
            && rawSize > 0
            && !chunks.isEmpty
            && chunks.reduce(Int64(0)) { $0 + $1.raw } == rawSize
    }

    /// The chunk holding `offset`; past the end this is the last chunk.
    func chunkIndex(at offset: Int64) -> Int {
        for chunk in chunks where offset < chunk.offset + chunk.raw {
            return chunk.index
        }
        return chunks.count - 1
    }
}

enum ChunkedAudioError: LocalizedError {
    case decodeFailed(String)
    case decodeLength(actual: Int, expected: Int64)
    case hashMismatch(index: Int)
    case fetchFailed(index: Int, status: Int)
    case unusableManifest

    var errorDescription: String? {
        switch self {
        case .decodeFailed(let name): "A zstd frame could not be decompressed (\(name))."
        case .decodeLength(let actual, let expected):
            "A zstd frame decoded to \(actual) bytes; the table says \(expected)."
        case .hashMismatch(let index):
            "Chunk \(index) decoded to bytes that do not match its hash."
        case .fetchFailed(let index, let status):
            "Chunk \(index) could not be fetched (\(status))."
        case .unusableManifest: "The chapter's chunk table is not usable."
        }
    }
}

/// Zstd frame decoding, straight through the C library.
enum Zstd {
    /// Decompresses one frame into `rawSize` bytes.
    ///
    /// The output size comes from the table rather than from the frame's own
    /// header, because the table is what the bytes were produced against and a
    /// frame that disagrees with it is a frame this app should refuse.
    static func decompress(_ frame: Data, rawSize: Int) throws -> Data {
        guard rawSize > 0 else { return Data() }

        var output = Data(count: rawSize)
        let written: Int = output.withUnsafeMutableBytes { destination in
            frame.withUnsafeBytes { source in
                guard let dst = destination.bindMemory(to: UInt8.self).baseAddress,
                      let src = source.bindMemory(to: UInt8.self).baseAddress
                else { return 0 }

                return ZSTD_decompress(dst, rawSize, src, frame.count)
            }
        }

        if ZSTD_isError(written) != 0 {
            let name = ZSTD_getErrorName(written).map { String(cString: $0) } ?? "unknown"
            throw ChunkedAudioError.decodeFailed(name)
        }
        guard written == rawSize else {
            throw ChunkedAudioError.decodeLength(actual: written, expected: Int64(rawSize))
        }

        return output
    }
}

/// A chapter's chunks, fetched and decompressed one at a time and kept.
///
/// An actor rather than a class with a lock: AVPlayer asks for ranges from its
/// own queue, and two requests for the same chunk must not fetch it twice or
/// race on the cache. Decompression is deliberately per chunk — a frame is
/// decoded when it is needed and never held in a decoder, which is the whole
/// reason the chapter is stored this way.
actor ChunkStore {
    private let manifest: ChunkManifest
    private var chunks: [Int: Data] = [:]
    private let session: URLSession

    init(manifest: ChunkManifest, session: URLSession = .shared) throws {
        guard manifest.isUsable else { throw ChunkedAudioError.unusableManifest }
        self.manifest = manifest
        self.session = session
    }

    var rawSize: Int64 { manifest.rawSize }

    /// The chapter's raw bytes in `range`, assembled from the chunks covering
    /// it. Only the covering chunks are fetched, so a seek into the middle of a
    /// chapter costs the chunks it needs rather than the ones before it.
    func bytes(in range: ClosedRange<Int64>) async throws -> Data {
        var result = Data()
        var cursor = range.lowerBound

        while cursor <= range.upperBound {
            let index = manifest.chunkIndex(at: cursor)
            // Named `entry`, not `chunk`: the method below is what fetches one,
            // and a local of that name would shadow it.
            let entry = manifest.chunks[index]
            let raw = try await chunk(at: index)

            let from = Int(cursor - entry.offset)
            let to = Int(min(range.upperBound, entry.offset + entry.raw - 1) - entry.offset)
            guard from <= to, to < raw.count else { break }

            result.append(raw[raw.startIndex + from ... raw.startIndex + to])
            cursor = entry.offset + entry.raw
        }

        return result
    }

    /// One chunk's raw bytes, fetched and decompressed at most once.
    private func chunk(at index: Int) async throws -> Data {
        if let cached = chunks[index] { return cached }

        let entry = manifest.chunks[index]
        guard let url = URL(string: entry.url, relativeTo: AudiobookAPI.shared.base) else {
            throw ChunkedAudioError.fetchFailed(index: index, status: 0)
        }

        let (frame, response) = try await session.data(from: url.absoluteURL)
        if let http = response as? HTTPURLResponse, !(200 ..< 300).contains(http.statusCode) {
            throw ChunkedAudioError.fetchFailed(index: index, status: http.statusCode)
        }

        let raw = try Zstd.decompress(frame, rawSize: Int(entry.raw))

        // The table carries the hash of the bytes before compression, so a frame
        // that decodes to the wrong audio is caught here — nothing downstream
        // could tell.
        if let expected = entry.sha256, !expected.isEmpty {
            let digest = SHA256.hash(data: raw).map { String(format: "%02x", $0) }.joined()
            guard digest == expected else { throw ChunkedAudioError.hashMismatch(index: index) }
        }

        chunks[index] = raw
        return raw
    }
}

/// Answers AVPlayer's byte-range requests from a chapter's zstd chunks.
///
/// The asset handed to AVPlayer carries a scheme this class owns, so every range
/// AVFoundation wants — the header, a seek, the read-ahead — arrives here and is
/// answered with decompressed bytes. To the player it is an ordinary file with
/// an ordinary length, which is what keeps seeking, rate and progress working
/// unchanged.
final class ChunkedAudioResourceLoader: NSObject, AVAssetResourceLoaderDelegate,
    @unchecked Sendable
{
    /// The scheme the asset is registered under. It never resolves on the
    /// network; it exists so AVFoundation routes the request here instead.
    static let scheme = "chunked-audio"

    private let store: ChunkStore
    private let contentType: String
    private let queue = DispatchQueue(label: "audiobooks.chunked-audio.loader", qos: .userInitiated)

    init(manifest: ChunkManifest) throws {
        self.store = try ChunkStore(manifest: manifest)
        self.contentType = ChunkedAudioResourceLoader.uti(for: manifest.contentType)
        super.init()
    }

    /// An asset whose bytes come from this loader.
    ///
    /// The caller must keep this loader alive for as long as the asset plays —
    /// `AVAssetResourceLoader` does not retain its delegate.
    func makeAsset(sourceURL: URL) -> AVURLAsset {
        var components = URLComponents(url: sourceURL, resolvingAgainstBaseURL: false)
        components?.scheme = Self.scheme
        let asset = AVURLAsset(url: components?.url ?? sourceURL)
        asset.resourceLoader.setDelegate(self, queue: queue)
        return asset
    }

    func resourceLoader(
        _ resourceLoader: AVAssetResourceLoader,
        shouldWaitForLoadingOfRequestedResource loadingRequest: AVAssetResourceLoadingRequest
    ) -> Bool {
        // The request is not Sendable, and this is the one place it is ever
        // touched: the task below runs its range to completion and then finishes
        // the request, so nothing else can reach it concurrently.
        Task { [store, contentType] in
            do {
                let size = await store.rawSize

                if let information = loadingRequest.contentInformationRequest {
                    information.contentType = contentType
                    information.contentLength = size
                    information.isByteRangeAccessSupported = true
                }

                guard let dataRequest = loadingRequest.dataRequest else {
                    loadingRequest.finishLoading()
                    return
                }

                // The first request reports its target in `requestedOffset` with
                // `currentOffset` still zero; every later one advances
                // `currentOffset`.
                let start = dataRequest.currentOffset != 0
                    ? dataRequest.currentOffset
                    : dataRequest.requestedOffset

                guard let range = Self.requestedRange(
                    offset: start,
                    length: dataRequest.requestedLength,
                    size: size
                ) else {
                    loadingRequest.finishLoading()
                    return
                }

                let bytes = try await store.bytes(in: range)
                guard !loadingRequest.isCancelled else { return }

                dataRequest.respond(with: bytes)
                loadingRequest.finishLoading()
            } catch {
                // A chunk that will not load fails the request rather than the
                // app: AVFoundation reports it through the item, and the player
                // above falls back to the plain chapter.
                loadingRequest.finishLoading(with: error)
            }
        }

        return true
    }

    /// The byte range a request asks for, clamped to the chapter.
    ///
    /// `requestedLength` is `NSURLResponseUnknownLength` when AVFoundation wants
    /// everything from the offset to the end of the resource — which is what a
    /// request for the header of a file whose length it has not read yet looks
    /// like. Reading that as a length would ask for a range ending before it
    /// starts, answer nothing, and leave the item waiting forever, so "unknown"
    /// means "to the end" here. `nil` means the request is past the chapter and
    /// has no bytes to answer with.
    static func requestedRange(offset: Int64, length: Int, size: Int64) -> ClosedRange<Int64>? {
        guard size > 0 else { return nil }

        let start = max(offset, 0)
        guard start < size else { return nil }

        let end = length <= 0 ? size - 1 : min(start + Int64(length) - 1, size - 1)
        return start ... end
    }

    /// The UTI AVPlayer is told the chapter is. It is the type of the
    /// *reassembled* audio, not of a chunk — the compression is this class's
    /// business and nobody else's.
    private static func uti(for contentType: String) -> String {
        switch contentType {
        case "audio/mpeg": "public.mp3"
        case "audio/ogg": "public.ogg"
        case "audio/wav": "public.wav"
        default: "public.audio"
        }
    }
}
