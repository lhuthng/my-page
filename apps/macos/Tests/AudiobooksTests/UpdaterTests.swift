import Foundation
import XCTest

@testable import Audiobooks

/// Covers the decisions `Updater` makes about a release payload. These are the
/// parts that broke in production (the asset key was never mapped, so every
/// check failed), and they need no network and no running app.
final class UpdaterTests: XCTestCase {
    // MARK: - Fixtures

    /// A release payload in the shape the GitHub API actually returns.
    private func payload(
        tag: String = "v0.5.0",
        assetName: String = "Audiobooks-0.5.0.app.zip",
        digest: String? = String(repeating: "a", count: 64),
        size: Int? = 520_527,
        body: String? = "**Full Changelog**: https://example.com/compare"
    ) throws -> Data {
        var asset: [String: Any] = [
            "name": assetName,
            "browser_download_url": "https://example.com/\(assetName)",
        ]
        if let digest { asset["digest"] = "sha256:\(digest)" }
        if let size { asset["size"] = size }

        var json: [String: Any] = ["tag_name": tag, "assets": [asset]]
        if let body { json["body"] = body }
        return try JSONSerialization.data(withJSONObject: json)
    }

    // MARK: - The regression that broke every update

    /// GitHub spells the asset's link `browser_download_url`. When that property
    /// had no `CodingKeys` mapping, the decoder looked for "browserDownloadURL",
    /// found nothing, and each check died with "The data couldn't be read because
    /// it is missing" — so no update was ever offered or installed.
    func testDecodesSnakeCaseDownloadURL() throws {
        let release = try Updater.release(fromJSON: payload())
        XCTAssertEqual(
            release.downloadURL.absoluteString,
            "https://example.com/Audiobooks-0.5.0.app.zip"
        )
    }

    func testNormalisesTheTagAndCarriesTheSize() throws {
        let release = try Updater.release(fromJSON: payload())
        XCTAssertEqual(release.version, "0.5.0")
        XCTAssertEqual(release.size, Int64(520_527))
    }

    func testPicksTheAppZipAssetOutOfSeveral() throws {
        let data = try JSONSerialization.data(withJSONObject: [
            "tag_name": "v0.5.0",
            "assets": [
                ["name": "Audiobooks-0.5.0.dmg", "browser_download_url": "https://example.com/a.dmg"],
                [
                    "name": "Audiobooks-0.5.0.app.zip",
                    "browser_download_url": "https://example.com/a.zip",
                    "digest": "sha256:" + String(repeating: "b", count: 64),
                    "size": 1,
                ],
                ["name": "checksums.txt", "browser_download_url": "https://example.com/c.txt"],
            ],
        ])
        // The `.app.zip` entry's own URL, not the `.dmg` or the checksums file.
        let release = try Updater.release(fromJSON: data)
        XCTAssertEqual(release.downloadURL.absoluteString, "https://example.com/a.zip")
    }

    // MARK: - Notes

    func testKeepsTheNotesAndTrimsThem() throws {
        let release = try Updater.release(fromJSON: payload(body: "\n\n  What changed  \n\n"))
        XCTAssertEqual(release.notes, "What changed")
    }

    func testDropsABlankOrAbsentBody() throws {
        XCTAssertNil(try Updater.release(fromJSON: payload(body: "  \n  ")).notes)
        XCTAssertNil(try Updater.release(fromJSON: payload(body: nil)).notes)
    }

    // MARK: - Refusals

    func testRejectsAReleaseWithNoAppZipAsset() throws {
        let data = try JSONSerialization.data(withJSONObject: [
            "tag_name": "v0.5.0",
            "assets": [["name": "checksums.txt", "browser_download_url": "https://example.com/c.txt"]],
        ])
        assertThrows("no usable version|has no") { _ = try Updater.release(fromJSON: data) }
    }

    func testRejectsAReleaseWithNoDigest() throws {
        let data = try payload(digest: nil)
        assertThrows("no sha256 digest") { _ = try Updater.release(fromJSON: data) }
    }

    func testRejectsAnUnusableDigest() throws {
        let data = try payload(digest: "not-hex")
        assertThrows("no sha256 digest") { _ = try Updater.release(fromJSON: data) }
    }

    func testRejectsATagWithNoUsableVersion() throws {
        let data = try payload(tag: "release-2026")
        assertThrows("no usable version") { _ = try Updater.release(fromJSON: data) }
    }

    /// The payload's fields are all optional, so an empty object decodes and is
    /// then refused for having no version rather than for being unreadable.
    func testRejectsAPayloadWithNoFields() throws {
        assertThrows("no usable version") { _ = try Updater.release(fromJSON: Data("{}".utf8)) }
    }

    func testRejectsSomethingThatIsNotJSON() throws {
        XCTAssertThrowsError(try Updater.release(fromJSON: Data("not json".utf8)))
    }

    /// A `browser_download_url` that is not a URL is a decode failure, not a
    /// silently missing asset.
    func testRejectsANonURLDownloadLink() throws {
        let data = try JSONSerialization.data(withJSONObject: [
            "tag_name": "v0.5.0",
            "assets": [["name": "Audiobooks-0.5.0.app.zip", "browser_download_url": ""]],
        ])
        XCTAssertThrowsError(try Updater.release(fromJSON: data))
    }

    // MARK: - Digests

    func testDigestAcceptsSha256HexInEitherCase() throws {
        let lower = Updater.Digest("sha256:" + String(repeating: "a", count: 64))
        let upper = Updater.Digest("sha256:" + String(repeating: "A", count: 64))
        XCTAssertNotNil(lower)
        XCTAssertEqual(lower, upper)
    }

    func testDigestRejectsAnythingElse() {
        let hex = String(repeating: "a", count: 64)
        XCTAssertNil(Updater.Digest("md5:\(hex)"))
        XCTAssertNil(Updater.Digest("sha256:abc"))
        XCTAssertNil(Updater.Digest("sha256:" + String(repeating: "z", count: 64)))
        XCTAssertNil(Updater.Digest("sha256:"))
        XCTAssertNil(Updater.Digest("sha256"))
        XCTAssertNil(Updater.Digest(""))
    }

    // MARK: - Versions

    func testNormalisedStripsTheVPrefixAndSurroundingSpace() {
        XCTAssertEqual(Updater.normalised("v0.2.0"), "0.2.0")
        XCTAssertEqual(Updater.normalised("0.2.0"), "0.2.0")
        XCTAssertEqual(Updater.normalised(" 0.2.0\n"), "0.2.0")
        XCTAssertNil(Updater.normalised("release-2026"))
        XCTAssertNil(Updater.normalised("v"))
        XCTAssertNil(Updater.normalised(""))
    }

    func testIsNewerComparesNumericallySoTenBeatsNine() {
        XCTAssertTrue(Updater.isNewer("0.10.0", than: "0.9.0"))
        XCTAssertTrue(Updater.isNewer("0.5.1", than: "0.5.0"))
        XCTAssertTrue(Updater.isNewer("1.0", than: "0.9.9"))
        XCTAssertFalse(Updater.isNewer("0.5.0", than: "0.5.0"))
        XCTAssertFalse(Updater.isNewer("0.4.0", than: "0.5.0"))
    }

    // MARK: - Helpers

    /// `XCTAssertThrowsError` with a message check, kept terse at the call site.
    private func assertThrows(
        _ pattern: String,
        _ body: () throws -> Void,
        file: StaticString = #filePath,
        line: UInt = #line
    ) {
        XCTAssertThrowsError(try body(), file: file, line: line) { error in
            XCTAssertNotNil(
                error.localizedDescription.range(of: pattern, options: .regularExpression),
                "expected /\(pattern)/ in \"\(error.localizedDescription)\"",
                file: file,
                line: line
            )
        }
    }
}
