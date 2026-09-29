import Foundation

/// Where a book's chapters are cut into windows.
///
/// The player no longer receives a whole book's chapter list at once: the book
/// arrives with its first window, and the rest are fetched as the reader
/// reaches them. These three answers — which window holds a chapter, which
/// windows a book has, and how long the last one is — are the whole of that
/// arithmetic, and they are pure, so they can be tested without a player, a
/// network, or a book.
enum ChapterWindow {
    /// Chapters per fetch. Matches the web player's window, so both clients ask
    /// the backend for the same amount per request.
    static let size = 20

    /// Chapters ahead of the playhead to have in hand before they are needed.
    /// Smaller than a window on purpose: while more than this many chapters
    /// remain in the current window the prefetch lands inside it and costs
    /// nothing, and only near the boundary does it reach for the next one.
    static let prefetchAhead = 3

    /// Start index of the window holding `index`.
    static func start(of index: Int) -> Int {
        max(0, index / size * size)
    }

    /// Every window start covering a book of `count` chapters, in order.
    static func starts(count: Int) -> [Int] {
        Array(stride(from: 0, to: max(0, count), by: size))
    }

    /// Chapters a window holds, clamped to the end of the book.
    static func count(start: Int, total: Int) -> Int {
        max(0, min(size, total - start))
    }
}
