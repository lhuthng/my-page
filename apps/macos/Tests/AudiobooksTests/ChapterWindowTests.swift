import XCTest

@testable import Audiobooks

/// Covers the arithmetic that cuts a book into chapter windows. Pure on
/// purpose: a window boundary that is off by one is what would make the player
/// skip a chapter or ask for the same one forever, and neither needs a network
/// or a running app to catch.
final class ChapterWindowTests: XCTestCase {
    func testStartRoundsAnIndexDownToItsWindow() {
        XCTAssertEqual(ChapterWindow.start(of: 0), 0)
        XCTAssertEqual(ChapterWindow.start(of: 19), 0)
        XCTAssertEqual(ChapterWindow.start(of: 20), 20)
        XCTAssertEqual(ChapterWindow.start(of: 21), 20)
        XCTAssertEqual(ChapterWindow.start(of: 999), 980)
        // A negative index is the head of the book, not a window before it.
        XCTAssertEqual(ChapterWindow.start(of: -4), 0)
    }

    func testStartsCoversTheBookAndStopsAtTheLastChapter() {
        XCTAssertEqual(ChapterWindow.starts(count: 0), [])
        XCTAssertEqual(ChapterWindow.starts(count: 1), [0])
        XCTAssertEqual(ChapterWindow.starts(count: ChapterWindow.size), [0])
        XCTAssertEqual(ChapterWindow.starts(count: ChapterWindow.size + 1), [0, ChapterWindow.size])
        XCTAssertEqual(ChapterWindow.starts(count: 2000).count, 100)
    }

    func testCountClampsTheLastWindowToTheEndOfTheBook() {
        XCTAssertEqual(ChapterWindow.count(start: 0, total: 45), 20)
        XCTAssertEqual(ChapterWindow.count(start: 40, total: 45), 5)
        XCTAssertEqual(ChapterWindow.count(start: 60, total: 45), 0)
        XCTAssertEqual(ChapterWindow.count(start: 0, total: 3), 3)
    }
}
