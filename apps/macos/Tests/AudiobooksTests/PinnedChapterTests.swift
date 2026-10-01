import SwiftUI
import XCTest

@testable import Audiobooks

/// Covers the decision behind the pinned chapter bar: which edge of the list the
/// playing chapter has scrolled off through.
///
/// Pure on purpose. The rule is easy to get subtly wrong — an edge off by a row
/// leaves the bar showing over a chapter that is plainly visible, or hides it
/// over one that plainly is not — and neither symptom needs a window, a scroll
/// view or a network to catch.
final class PinnedChapterTests: XCTestCase {
    /// Rows are 30pt tall in these cases, offset 0, viewport 100pt: chapters 0,
    /// 1, 2 are on screen and chapter 3 starts exactly at the bottom edge.
    private func edge(
        measuredIndex: Int = 0,
        measuredTop: CGFloat = 0,
        rowHeight: CGFloat = 30,
        playingIndex: Int = 0,
        offset: CGFloat = 0,
        viewport: CGFloat = 100
    ) -> VerticalEdge? {
        PlayerScreen.pinnedEdge(
            measuredIndex: measuredIndex,
            measuredTop: measuredTop,
            rowHeight: rowHeight,
            playingIndex: playingIndex,
            offset: offset,
            viewport: viewport
        )
    }

    func testNoEdgeWhileTheChapterIsOnScreen() {
        XCTAssertNil(edge(playingIndex: 0, offset: 0))
        XCTAssertNil(edge(playingIndex: 1, offset: 0))
        XCTAssertNil(edge(playingIndex: 2, offset: 0))
        // Scrolled a little: chapter 0 is still partly on screen at the top.
        XCTAssertNil(edge(playingIndex: 0, offset: 10))
    }

    func testPinsToTheTopOnceTheChapterHasLeftThroughIt() {
        // Scrolled 60pt: chapter 0 (0...30) is entirely above the viewport.
        XCTAssertEqual(edge(playingIndex: 0, offset: 60), .top)
        // Chapter 1 (30...60) has just gone too, and is exactly at the boundary.
        XCTAssertEqual(edge(playingIndex: 1, offset: 60), .top)
    }

    func testPinsToTheBottomWhileTheChapterIsBelow() {
        // Viewport shows 0...100; chapter 5 (150...180) is well below it.
        XCTAssertEqual(edge(playingIndex: 5, offset: 0), .bottom)
        // And staying below after a scroll, so long as it is still not reached.
        XCTAssertEqual(edge(playingIndex: 5, offset: 40), .bottom)
    }

    func testTheEdgeFlipsAsTheListScrollsPastTheChapter() {
        // Chapter 5 sits at 150...180. At offset 0 its top is below the 0...100
        // viewport, at offset 60 it is partly on screen, and by offset 180 it
        // has gone out through the top.
        XCTAssertEqual(edge(playingIndex: 5, offset: 0), .bottom)
        XCTAssertNil(edge(playingIndex: 5, offset: 60))
        XCTAssertEqual(edge(playingIndex: 5, offset: 180), .top)
    }

    func testTheReadingCarriesForwardWhenTheChapterAdvancesOffScreen() {
        // The reader is scrolled deep into the book with chapter 0 playing and
        // off the top; the playhead then advances without the new row ever
        // rendering, so the measurement still says chapter 0. Row height carries
        // it: chapter 3's row is 90...120, still above the viewport at offset 200.
        XCTAssertEqual(
            edge(measuredIndex: 0, measuredTop: 0, playingIndex: 3, offset: 200),
            .top
        )
        // And when the playhead runs far enough ahead that its row is in view,
        // the bar gives the list back.
        XCTAssertNil(edge(measuredIndex: 0, measuredTop: 0, playingIndex: 7, offset: 200))
    }

    func testABookWithNoViewportOrNoMeasuredRowShowsNoBar() {
        // Before the list has laid out, or before any row has reported, there is
        // nothing to decide from — and a bar placed on no information is worse
        // than no bar.
        XCTAssertNil(edge(viewport: 0))
        XCTAssertNil(edge(rowHeight: 0))
    }

    func testABackwardCarryStaysAtTheTop() {
        // The measured row is chapter 5, sitting near the bottom of the viewport
        // at 90...120 and so still reporting. The playhead then jumps backwards
        // to chapter 1, whose carried row (90 + (1 - 5) * 30 = -30) has left
        // through the top by the time it would have been drawn.
        XCTAssertEqual(
            edge(measuredIndex: 5, measuredTop: 90, playingIndex: 1, offset: 0),
            .top
        )
    }
}
