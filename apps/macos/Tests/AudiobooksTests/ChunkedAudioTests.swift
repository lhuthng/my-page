import AVFoundation
import XCTest

@testable import Audiobooks

/// Covers the client half of chunked audio: decoding a zstd frame, reading the
/// table the server sends, and turning a byte offset into a chunk.
///
/// Pure on purpose. A frame that decodes to the wrong bytes, or an offset that
/// resolves to the wrong chunk, is a chapter that plays the wrong audio from the
/// wrong place — and neither needs a network, a player or a running app to
/// catch. The frame below is the one the browser's tests decode as well, so both
/// halves of the feature are checked against the same compressed bytes.
final class ChunkedAudioTests: XCTestCase {
    private let rawFixture = "ID3-chunk-zero-of-a-chapter-the-client-decompresses"
    /// A real `zstd -19` frame holding `rawFixture`. Base64 because a test cannot
    /// run the CLI, and 64 bytes stored for 51 bytes of audio.
    private let frameBase64 =
        "KLUv/QRomQEASUQzLWNodW5rLXplcm8tb2YtYS1jaGFwdGVyLXRoZS1jbGllbnQtZGVjb21wcmVzc2VzO/k7lg=="

    private func frameData() throws -> Data {
        try XCTUnwrap(Data(base64Encoded: frameBase64))
    }

    /// A table shaped exactly like the backend's `ChunkManifestResponse`: the
    /// snake_case keys are the contract, and `level` is present in the response
    /// without being in the model.
    private func manifest(
        rawSize: Int,
        version: Int = 1,
        chunkSize: Int? = nil,
        chunks: [(index: Int, offset: Int, raw: Int, compressed: Int)]
    ) throws -> ChunkManifest {
        let body = chunks
            .map { chunk in
                """
                {"index":\(chunk.index),"offset":\(chunk.offset),"raw":\(chunk.raw),\
                "compressed":\(chunk.compressed),"sha256":null,\
                "url":"media/i/abt-1/chunk/\(chunk.index)"}
                """
            }
            .joined(separator: ",")

        let json = """
        {"version":\(version),"chunk_size":\(chunkSize ?? rawSize),"raw_size":\(rawSize),\
        "level":19,"content_type":"audio/mpeg","chunks":[\(body)]}
        """

        let decoder = JSONDecoder()
        decoder.keyDecodingStrategy = .convertFromSnakeCase
        return try decoder.decode(ChunkManifest.self, from: Data(json.utf8))
    }

    func testDecodesARealZstdFrameToItsBytes() throws {
        let compiled = try Zstd.decompress(try frameData(), rawSize: rawFixture.utf8.count)

        XCTAssertEqual(String(data: compiled, encoding: .utf8), rawFixture)
    }

    func testRefusesAFrameThatDoesNotHoldThePromisedLength() throws {
        // The table is what the bytes were produced against, so a frame that
        // disagrees with it is refused rather than played.
        XCTAssertThrowsError(try Zstd.decompress(try frameData(), rawSize: rawFixture.utf8.count + 1))
    }

    func testManifestDecodesTheShapeTheServerSends() throws {
        let size = rawFixture.utf8.count

        let table = try manifest(rawSize: size * 2, chunks: [
            (index: 0, offset: 0, raw: size, compressed: 64),
            (index: 1, offset: size, raw: size, compressed: 64),
        ])

        XCTAssertTrue(table.isUsable)
        XCTAssertEqual(table.rawSize, Int64(size * 2))
        XCTAssertEqual(table.chunks.count, 2)
        XCTAssertEqual(table.contentType, "audio/mpeg")
        XCTAssertEqual(table.chunks[1].offset, Int64(size))
        XCTAssertEqual(table.chunks[0].sha256, nil, "a table written before hashes existed still decodes")
    }

    func testManifestIsUnusableWhenItDoesNotDescribeTheChapter() throws {
        let size = rawFixture.utf8.count

        // Lengths that add up to less than the chapter: a position near the end
        // would have nothing to resolve to.
        let short = try manifest(rawSize: size * 2, chunks: [(index: 0, offset: 0, raw: size, compressed: 64)])
        XCTAssertFalse(short.isUsable)

        let unknownVersion = try manifest(
            rawSize: size, version: 99, chunks: [(index: 0, offset: 0, raw: size, compressed: 64)]
        )
        XCTAssertFalse(unknownVersion.isUsable, "a table this build does not know is not guessed at")

        let empty = try manifest(rawSize: size, chunks: [])
        XCTAssertFalse(empty.isUsable)
    }

    func testOffsetsResolveToTheirChunk() throws {
        let size = rawFixture.utf8.count
        let table = try manifest(rawSize: size * 3, chunks: [
            (index: 0, offset: 0, raw: size, compressed: 64),
            (index: 1, offset: size, raw: size, compressed: 64),
            (index: 2, offset: size * 2, raw: size, compressed: 64),
        ])

        XCTAssertEqual(table.chunkIndex(at: 0), 0)
        XCTAssertEqual(table.chunkIndex(at: Int64(size - 1)), 0)
        XCTAssertEqual(table.chunkIndex(at: Int64(size)), 1)
        XCTAssertEqual(table.chunkIndex(at: Int64(size * 2 - 1)), 1)
        XCTAssertEqual(table.chunkIndex(at: Int64(size * 2)), 2)
        // Past the end clamps to the last chunk rather than trapping.
        XCTAssertEqual(table.chunkIndex(at: Int64(size * 99)), 2)
    }

    // MARK: - Byte ranges out of the chunks

    /// Real `zstd -19` frames for three chunks of one chapter: 51 bytes of `a`,
    /// 40 of `b`, 30 of `c`. Base64 for the same reason as the frame above.
    private let chunk0 = "KLUv/SQzRQAAEGFhAQBGAAs06RF9"
    private let chunk1 = "KLUv/SQoRQAAEGJiAQAfgAWJE/c1"
    private let chunk2 = "KLUv/SQeRQAAEGNjAQAMwAI3EBc7"

    private var chapter: Data {
        Data(repeating: UInt8(ascii: "a"), count: 51)
            + Data(repeating: UInt8(ascii: "b"), count: 40)
            + Data(repeating: UInt8(ascii: "c"), count: 30)
    }

    private func serveChapter() {
        StubURLProtocol.reset()
        StubURLProtocol.serve("/chunk/0", frame: chunk0)
        StubURLProtocol.serve("/chunk/1", frame: chunk1)
        StubURLProtocol.serve("/chunk/2", frame: chunk2)
    }

    /// A table whose chunks live on a stub host, so the fetching half can run
    /// without a bucket. Offsets are the running sum of the chunk lengths, which
    /// is what the backend writes.
    private func chunkManifest(
        frames: [String],
        rawSizes: [Int],
        hashes: [String?]? = nil,
        contentType: String
    ) throws -> ChunkManifest {
        var offset = 0
        var entries: [String] = []

        for (index, size) in rawSizes.enumerated() {
            let stored = try XCTUnwrap(Data(base64Encoded: frames[index])).count
            let hash = (hashes?[index] ?? nil).map { "\"\($0)\"" } ?? "null"
            entries.append(
                """
                {"index":\(index),"offset":\(offset),"raw":\(size),"compressed":\(stored),\
                "sha256":\(hash),"url":"http://stub.test/chunk/\(index)"}
                """
            )
            offset += size
        }

        let json = """
        {"version":1,"chunk_size":\(rawSizes[0]),"raw_size":\(offset),"level":19,\
        "content_type":"\(contentType)","chunks":[\(entries.joined(separator: ","))]}
        """

        let decoder = JSONDecoder()
        decoder.keyDecodingStrategy = .convertFromSnakeCase
        return try decoder.decode(ChunkManifest.self, from: Data(json.utf8))
    }

    private func networkManifest(hashes: [String?] = [nil, nil, nil]) throws -> ChunkManifest {
        try chunkManifest(
            frames: [chunk0, chunk1, chunk2], rawSizes: [51, 40, 30], hashes: hashes,
            contentType: "audio/mpeg"
        )
    }

    private func stubSession() -> URLSession {
        let config = URLSessionConfiguration.ephemeral
        config.protocolClasses = [StubURLProtocol.self]
        return URLSession(configuration: config)
    }

    /// The heart of it: what AVPlayer asks for, answered from the chunks that
    /// cover it. Compression is invisible above this point, so the bytes have to
    /// come back as the chapter's own.
    func testServesARangeFromExactlyTheChunksThatCoverIt() async throws {
        serveChapter()
        let store = try ChunkStore(manifest: try networkManifest(), session: stubSession())
        let chapter = self.chapter

        let head = try await store.bytes(in: 0 ... 10)
        XCTAssertEqual(head, chapter[0 ... 10], "a range inside one chunk")

        let join = try await store.bytes(in: 45 ... 60)
        XCTAssertEqual(join, chapter[45 ... 60], "a range across a chunk boundary joins exactly")

        let tail = try await store.bytes(in: 100 ... 120)
        XCTAssertEqual(tail, chapter[100 ... 120], "a range inside the last chunk")
    }

    func testASeekOnlyFetchesTheChunksItLandsIn() async throws {
        serveChapter()
        let store = try ChunkStore(manifest: try networkManifest(), session: stubSession())

        _ = try await store.bytes(in: 100 ... 110)

        XCTAssertEqual(
            StubURLProtocol.requestedPaths, ["/chunk/2"],
            "a read near the end must not pull the chunks before it"
        )
    }

    func testAChunkIsFetchedOnceHoweverManyRangesTouchIt() async throws {
        serveChapter()
        let store = try ChunkStore(manifest: try networkManifest(), session: stubSession())

        _ = try await store.bytes(in: 0 ... 10)
        _ = try await store.bytes(in: 20 ... 30)

        XCTAssertEqual(StubURLProtocol.requestCount("/chunk/0"), 1)
    }

    func testARangePastTheEndStopsAtTheLastChunk() async throws {
        serveChapter()
        let store = try ChunkStore(manifest: try networkManifest(), session: stubSession())

        let asked = try await store.bytes(in: 0 ... 9_999)

        XCTAssertEqual(asked, chapter, "asking for more than there is returns the chapter, and stops")
        XCTAssertEqual(StubURLProtocol.requestedPaths, ["/chunk/0", "/chunk/1", "/chunk/2"])
    }

    func testAChunkIsRefusedWhenItsBytesDoNotHashToWhatTheTableSays() async throws {
        serveChapter()
        let real = "e26d2da3ab585c9840b157a4a9b5639fda905c2ad33d04d2c598be8883b51236"

        let honest = try ChunkStore(
            manifest: try networkManifest(hashes: [nil, real, nil]), session: stubSession()
        )
        let middle = try await honest.bytes(in: 60 ... 70)
        XCTAssertEqual(middle, chapter[60 ... 70], "sha256 of the bytes before compression")

        let lying = try ChunkStore(
            manifest: try networkManifest(hashes: [String(repeating: "0", count: 64), nil, nil]),
            session: stubSession()
        )
        do {
            _ = try await lying.bytes(in: 0 ... 10)
            XCTFail("a chunk that does not hash to its table entry must not be played")
        } catch ChunkedAudioError.hashMismatch(let index) {
            XCTAssertEqual(index, 0)
        }
    }

    func testAChunkThatWillNotLoadFailsRatherThanAnsweringShort() async throws {
        serveChapter()
        StubURLProtocol.serve("/chunk/1", frame: chunk1, status: 404)
        let store = try ChunkStore(manifest: try networkManifest(), session: stubSession())

        do {
            _ = try await store.bytes(in: 60 ... 70)
            XCTFail("a chunk that did not load must fail the range, not answer it short")
        } catch ChunkedAudioError.fetchFailed(let index, let status) {
            XCTAssertEqual(index, 1)
            XCTAssertEqual(status, 404)
        }
    }

    func testATableThatDoesNotDescribeTheChapterIsRefusedUpFront() throws {
        let short = try manifest(rawSize: 9_999, chunks: [(index: 0, offset: 0, raw: 51, compressed: 28)])

        XCTAssertThrowsError(try ChunkStore(manifest: short)) { error in
            guard case ChunkedAudioError.unusableManifest = error else {
                return XCTFail("expected an unusable table, got \(error)")
            }
        }
    }

    /// The delegate is the one piece nothing else can reach: AVFoundation calls
    /// it, not a test. So this asks AVFoundation to open the asset and watches
    /// the loader's own fetch, which only happens if the callback ran.
    func testAVFoundationAsksTheLoaderForTheChaptersBytes() async throws {
        serveChapter()
        URLProtocol.registerClass(StubURLProtocol.self)
        defer { URLProtocol.unregisterClass(StubURLProtocol.self) }

        let loader = try ChunkedAudioResourceLoader(manifest: try networkManifest())
        let asset = loader.makeAsset(
            sourceURL: try XCTUnwrap(URL(string: "https://cdn.example.test/media/i/abt-1/chapter-1.mp3"))
        )

        // Whatever AVFoundation makes of the bytes, it has to ask this loader for
        // them: the scheme it was given resolves nowhere else.
        let opening = Task { try? await asset.load(.isPlayable) }
        let deadline = Date().addingTimeInterval(5)
        while StubURLProtocol.requestedPaths.isEmpty && Date() < deadline {
            try await Task.sleep(nanoseconds: 50_000_000)
        }
        _ = await opening.value

        let requested = StubURLProtocol.requestedPaths
        XCTAssertFalse(
            requested.isEmpty,
            "AVFoundation never asked the loader for bytes, so the scheme is not routing here"
        )
        XCTAssertTrue(
            requested.allSatisfy { $0.hasPrefix("/chunk/") },
            "AVFoundation asked for something other than the chapter's chunks: \(requested)"
        )
    }

    // MARK: - A chapter AVPlayer actually plays

    /// A real WAV: 0.5 seconds of 440 Hz at 8 kHz, 8044 bytes, stored as four
    /// `zstd -19` chunks. The fixture above is only bytes that look like audio,
    /// which is enough to prove the loader is asked for them and nothing beyond
    /// it — AVPlayer gives up on it before it can play. This one it can parse,
    /// so readiness, duration and seeking are decided by AVFoundation and not by
    /// this test's idea of how the loader gets called.
    private let wavChunk0 =
        "KLUv/WQABx0OAMQaUklGRmQfAABXQVZFZm10IBAAAAABQIA+AAACABAAZGF0YQAA4A/hHVgoCy5MLhMp/x5BEXgBhPFG423YQdJ/0TzW6t9j7Q/9Ew2NG8QmZi2qLmkqJSH1E2kEWPSn5RTa/dI40fvU1d256iH6OQodGQkllCzaLpUrKSOVFlUHOPck6OLb5tMg0ebT4tsk6Dj3VQeVFivaLpQsCSUdGTkKIfq56tXd+9Q40f3SFNqn5Vj0aQT1EyUhaSqqLmYtxCaNGxMND/1j7erfPNZ/0UHSbdhG44TxeAFBEf8eEylMLgsuWCjhHeAPAAAg8B/iqNf10bTR7dYB4b/uiP58Drockye/LYEuxCkWIJ0S8QLt8nPkPNma0lbRl9Xb3gvsl/uoC1ka7CUDLcguBSsrIkcV3wXH9ePm99ps0ybRa9TX3Gvpq/jICNwXHiQaLOAuGiweJNwXyAir+Gvp1CbRbNP32uPmx/XfBUcVKyIFK8guAy3sJVkaqAuX+wvs296X1VbRmtI82XPk7fLxAp0SFiDEKYEuvy2TJ7ocfA6I/r/uAeHt1rTR9dGo1x/iIPAGAFOQk+uPRQCI/BPOzIDeBMQAxwTCE3lR"
    private let wavChunk1 =
        "KLUv/WQAB60MAKYYYSYQfQwv2rz97vmOkANhCNAMJA2BYn2HOo6AZhCG0ATUu2/LP859dFYAVgBWAGkqx5r2ltWgusoQmxQ/pVPxd1HEEKKSSazuHbYTjL3iqapqvhtBJ+czwg1C74RMt79M2UTRi3AByTfcNTE7y6L6qq2tzuyKAwM8R0uzL0E+7CtIsww5GzgHAGiMM2TS7Cs+rATR7B2tAC8ONLu2umqL6suyxGx3Ld9YQC5CUTRl+0vTrROC0Am3+Qyd7Eaqearqigdj2c57J7GqZEIIFPF3T5OiIVZXqUHesjUoyx+jLCAN5XuY2plr3Fd7xVFF1ly3njbTpQWx5OU5nQp/mhqE2haaQok3thVhL9ZIlVWPXNZ5J3cL4g8kZ1KK/l8KOhpinoyePviGkgNZN7VJpWuV5Y0xAADnlDdaJZVObWbd5MAbqg8yepjnaJiC/l8pypk8kIK43H3ecVnVI1WGNYa9bQXeqFBaaKF2avCnp4LnJHkVxHRp2tytW3NU0XWNszMPU8oXpAkDAFebk+uPRQCI+8A8A+Iw3rU="
    private let wavChunk2 =
        "KLUv/WQAB60MAKYYYSYQfQwv2rz97vmOkANhCNAMJA2BYn2HOo6AZhCG0ATUu2/LP859dFcAVgBWAKZbJwShE27zGTrZjVTzVNUVD8aynfdOYlXJhBAo4u+eJkVDrK5Sg7xla5rKsaa9ZTWorjLEJsVP6VT8XRQxhKhkEqt7h+0EY694qqqa70bQyfmMcIPQA4qwF2ukyqpHLuu8k7sF8QeSMylF/y8FHQ0xT0ZPH3xDyYGsm9qk0rXK8gbOAQAa4wyZNPuKDytBNHtHK8CLA82ura7aovqyLDHbXcs3FpCLUBRN2f4yMqwx7G0r8EaF0kILtVODPz0VPCfJqyCmS9Pmbt2ao4qua5ydeZhSviANyvLHKAtIQ/kepnbmGvfVXnFUkTXXrafNdGlBLHl5TqfCn6YGobaFplDije2dkOn2lymbKHoRLiD5hrsmZmdZVF+1tdWZXXFggOdoafYlyId9BWmWITfGAACcU95olVQ6tZl1kwNvqD7I6GGeo2EK+n+lKGfyQAricvd5x2VVj9QDAIc2J9cviwCQ/sCEAuk1Vts="
    private let wavChunk3 =
        "KLUv/WRsBq0MAKYYYSYQfQwv2rz97vmOkANhCNAMJA2BYn2HOo6AZhCG0ATUu2/LP859dFYAVwBWAOMMmTT7ig8rQTR7RyvAiwPNrq2u2qL6siwx213LNxaQi1AUTdn+0nTrhCB0wm0+Qye7kWqeqrriwVi2895JrCqZEAJF/N1T8VOaFA2xukoN8pataSonIA3le5jamWvcV3vFUUXWXLeeNtOlBbHk5TmdCn+aGoTaFppCiTe2FWEv1kiVVY9c1nkndwviDyRnUor+Xwo6GmKejJ4++IaSA1k3tUmla5XlDZwDADQCcE55o1VS6dRm1k0OvKH6IKOHeY6GKej/laKcyQMpiMvd5x2XVT1SZVhj2NtW4I0KpYUWaqcGf3oqeE6SV0FMl6bN3bo1RxVd1zg78zClfEEalOWPUZbWtLesBtVVhtjkVPxdFDGEqGQSq3uH7QRjr3iqqprvRtDJ+Yxwg9A7IdPtL1M2UfQiXEDyDXdNzM6yqL5qa6szu+LAAM/R0uxLkA/7CtIsQ26MAYADANlN7u9TdwuQ1gemFKinvOo="

    private func serveWavChapter() {
        StubURLProtocol.reset()
        for (index, frame) in [wavChunk0, wavChunk1, wavChunk2, wavChunk3].enumerated() {
            StubURLProtocol.serve("/chunk/\(index)", frame: frame)
        }
    }

    private func wavManifest() throws -> ChunkManifest {
        try chunkManifest(
            frames: [wavChunk0, wavChunk1, wavChunk2, wavChunk3],
            rawSizes: [2048, 2048, 2048, 1900],
            hashes: [
                "2f7eee44b82e269645bbfb5a07f6fc445c81451e33cc377f86d5bdcb80e35366",
                "98127fe24dd1d5e4664267dad39982ab0223b5765b2006fb4e766ade7405c1db",
                "a079b1316d405fa749988fdd8c83ab665a9e885bc721ac55fa532cd9ef6eb23b",
                "ad439716dafd97b8e95b22910fc5410ab98780e011042e663b49fd69fb2f658f",
            ],
            contentType: "audio/wav"
        )
    }

    /// The one check the others cannot stand in for: AVFoundation, not a test,
    /// decides whether the chapter the loader assembled is playable. An answer
    /// that is short, empty or missing leaves the item stuck at `.unknown`
    /// forever, which is exactly how a byte-range mistake shows up.
    func testAVPlayerPlaysAChapterAssembledFromChunks() async throws {
        serveWavChapter()
        URLProtocol.registerClass(StubURLProtocol.self)
        defer { URLProtocol.unregisterClass(StubURLProtocol.self) }

        let loader = try ChunkedAudioResourceLoader(manifest: try wavManifest())
        let source = try XCTUnwrap(URL(string: "https://cdn.example.test/media/i/abt-1/chapter.wav"))
        let item = AVPlayerItem(asset: loader.makeAsset(sourceURL: source))
        let player = AVPlayer(playerItem: item)

        let deadline = Date().addingTimeInterval(20)
        while item.status == .unknown && Date() < deadline {
            try await Task.sleep(nanoseconds: 50_000_000)
        }

        XCTAssertEqual(
            item.status, .readyToPlay,
            "AVPlayer never became ready, so the bytes it was served are not playable"
        )

        let duration = try await item.asset.load(.duration)
        XCTAssertEqual(CMTimeGetSeconds(duration), 0.5, accuracy: 0.05)

        XCTAssertFalse(StubURLProtocol.requestedPaths.isEmpty)
        XCTAssertTrue(
            StubURLProtocol.requestedPaths.allSatisfy { $0.hasPrefix("/chunk/") },
            "AVPlayer asked for something other than the chapter's chunks: \(StubURLProtocol.requestedPaths)"
        )

        // The item has to outlive the player for its status to stay readable.
        withExtendedLifetime(player) {}
    }

    /// What AVFoundation asks for is not always a plain length: `NSURLResponseUnknownLength`
    /// means "from here to the end", and reading it as a length would ask for a
    /// range that ends before it begins.
    func testTheRangeARequestAsksForIsClampedToTheChapter() {
        // `NSURLResponseUnknownLength` is -1 on Darwin and is not visible to
        // Swift, so it is spelled out: it is the value AVFoundation uses for
        // "from the offset to the end of the resource".
        let unknownLength = -1

        XCTAssertEqual(
            ChunkedAudioResourceLoader.requestedRange(offset: 0, length: 100, size: 8044),
            0 ... 99
        )
        XCTAssertEqual(
            ChunkedAudioResourceLoader.requestedRange(
                offset: 100, length: unknownLength, size: 8044
            ),
            100 ... 8043,
            "an unknown length is the rest of the chapter, not none of it"
        )
        XCTAssertEqual(
            ChunkedAudioResourceLoader.requestedRange(offset: 8000, length: 100, size: 8044),
            8000 ... 8043,
            "a request past the end stops at the last byte"
        )
        XCTAssertEqual(
            ChunkedAudioResourceLoader.requestedRange(offset: 8044, length: 10, size: 8044), nil,
            "a request starting past the chapter has nothing to answer with"
        )
        XCTAssertEqual(ChunkedAudioResourceLoader.requestedRange(offset: 0, length: 10, size: 0), nil)
    }

    /// AVFoundation only calls the loader for a scheme the loader owns, so the
    /// asset it is handed has to carry that scheme and still name the chapter.
    func testTheAssetAVPlayerPlaysCarriesTheSchemeThisLoaderOwns() throws {
        let loader = try ChunkedAudioResourceLoader(manifest: try networkManifest())
        let source = try XCTUnwrap(URL(string: "https://cdn.example.test/media/i/abt-1/chapter-1.mp3"))

        let asset = loader.makeAsset(sourceURL: source)

        XCTAssertEqual(asset.url.scheme, ChunkedAudioResourceLoader.scheme)
        XCTAssertEqual(asset.url.path, source.path)
    }
}

/// Serves the frames a test registers: the smallest thing that can stand in for
/// the bucket, so the fetch half of chunked playback is exercised rather than
/// assumed.
final class StubURLProtocol: URLProtocol {
    private static let lock = NSLock()
    private static var frames: [String: Data] = [:]
    private static var statuses: [String: Int] = [:]
    private static var requested: [String] = []

    static func reset() {
        lock.lock()
        frames = [:]
        statuses = [:]
        requested = []
        lock.unlock()
    }

    static func serve(_ path: String, frame: String, status: Int = 200) {
        lock.lock()
        frames[path] = Data(base64Encoded: frame)
        statuses[path] = status
        lock.unlock()
    }

    static var requestedPaths: [String] {
        lock.lock()
        defer { lock.unlock() }
        return requested
    }

    static func requestCount(_ path: String) -> Int {
        requestedPaths.filter { $0 == path }.count
    }

    override class func canInit(with request: URLRequest) -> Bool { true }
    override class func canonicalRequest(for request: URLRequest) -> URLRequest { request }

    override func startLoading() {
        let path = request.url?.path ?? ""

        Self.lock.lock()
        Self.requested.append(path)
        let body = Self.frames[path] ?? Data()
        let status = Self.statuses[path] ?? 404
        Self.lock.unlock()

        guard let url = request.url,
              let response = HTTPURLResponse(
                  url: url, statusCode: status, httpVersion: "HTTP/1.1",
                  headerFields: ["Content-Length": "\(body.count)"]
              )
        else {
            client?.urlProtocolDidFinishLoading(self)
            return
        }

        client?.urlProtocol(self, didReceive: response, cacheStoragePolicy: .notAllowed)
        if !body.isEmpty { client?.urlProtocol(self, didLoad: body) }
        client?.urlProtocolDidFinishLoading(self)
    }

    override func stopLoading() {}
}
