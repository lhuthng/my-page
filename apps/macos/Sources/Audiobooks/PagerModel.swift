import AppKit
import SwiftUI

/// Paging state for the [player | library | about] strip, plus the mouse/scroll
/// routing that feeds it: a drag inside a scroll view never reaches an ancestor
/// gesture, so events are read directly — which is also what lets a drag scroll
/// a list.
final class PagerModel: ObservableObject {
    /// Who is driving an in-flight drag.
    enum Source { case gesture, monitor }

    /// Which page the strip is resting on: 0 = player, 1 = library, 2 = about.
    @Published private(set) var pageIndex = 1
    /// Live offset of the strip while dragging; 0 at rest.
    @Published private(set) var dragX: CGFloat = 0

    /// Whether the player page may be reached at all (a book is loaded).
    var playerAvailable = false
    /// Width of one page: a screen plus the seam it owns.
    var pageWidth: CGFloat = 384
    /// The chapter list's `NSScrollView`, handed over by the list itself (see
    /// `ScrollViewProbe`). Held directly because frame and hit-test spaces
    /// disagree, which put the drag zone above the list instead of on it.
    weak var listReference: NSScrollView?
    /// The timeline's busy flag, mirrored here for the trace only.
    var scrubbingProbe: (() -> Bool)?

    /// Whether the press just ended was a drag. Click handlers ask this, because a
    /// swipe ends in nothing but a release landing on them, which would otherwise
    /// also choose whatever it was let go over.
    private(set) var lastPressWasDrag = false

    /// Resting offset of the strip for the page on screen.
    var base: CGFloat { -CGFloat(pageIndex) * pageWidth }

    /// The pages reachable now: without a book there is no player page to swipe
    /// to, expressed as the range the offset is clamped to.
    private var reachable: ClosedRange<Int> { playerAvailable ? 0...(Self.pages - 1) : 1...(Self.pages - 1) }
    private static let pages = 3

    /// Page-turn thresholds. Deliberately undemanding: a mouse swipe is only a
    /// few points, so a stricter test read swipes as clicks.
    private let commitFraction: CGFloat = 0.15
    private let flickTravel: CGFloat = 14
    private let flickSpeed: CGFloat = 80

    /// Travel below which a press is still a click; matches the gesture's own
    /// minimum distance.
    private let clickSlop: CGFloat = 6

    /// Distance before a press's direction is read, and how far one axis must
    /// lead. Read once per press: letting a scroll turn into a page mid-gesture
    /// made the list and the strip fight each other.
    private let axisSlop: CGFloat = 5
    private let axisMargin: CGFloat = 1.25

    /// What an in-flight press turned out to be, decided once, the moment one
    /// axis first leads the other.
    private enum PressMode {
        /// Nothing decided yet — still short enough to be a click.
        case undecided
        /// Moving the list the press landed in. Never the strip.
        case scroll
        /// Turning the page.
        case page
        /// Belongs to a control under the press (the timeline's playhead).
        case held
    }

    private var pressMode: PressMode = .undecided
    /// Whether the SwiftUI gesture must stand down for this press; set when the
    /// press lands in a scrollable, which the router then drives itself.
    private var routerOwnsPress = false

    private var dragBegan = false
    private var draggingSource: Source?
    private var dragStartedAt: Date?
    /// Travel at the last event, measured from the press so a short swipe stays
    /// short instead of losing its first points to gesture slop.
    private var lastTranslation: CGFloat = 0

    // Event routing state.
    private var monitor: Any?
    private var pressActive = false
    /// Set once the router has claimed the current press, so the SwiftUI
    /// gesture path stands down instead of fighting it.
    private var routerDriving = false
    private var pressInScrollable = false
    private var pressInList = false
    /// The scroll view this press landed in, resolved once at mouse-down.
    private weak var pressScrollView: NSScrollView?
    private var lastPoint: CGPoint?
    private var totalX: CGFloat = 0
    private var totalY: CGFloat = 0
    private var scrollTotal: CGFloat = 0
    private var scrollDragActive = false
    private var scrollEndTask: Task<Void, Never>?

    // MARK: - Pages

    /// Slide to a page, clamped to the ones that exist.
    func show(_ index: Int) {
        move(to: index, allowEmptyPlayer: false)
    }

    /// A book was picked: slide to the player. Bypasses the reachable range on
    /// purpose — the player only becomes reachable once the book is loaded.
    func openPlayer() {
        move(to: 0, allowEmptyPlayer: true)
    }

    private func move(to index: Int, allowEmptyPlayer: Bool) {
        let lower = (playerAvailable || allowEmptyPlayer) ? 0 : 1
        let target = min(Self.pages - 1, max(lower, index))
        guard target != pageIndex else { return }
        withAnimation(.spring(response: 0.42, dampingFraction: 0.92)) {
            pageIndex = target
            dragX = 0
        }
    }

    // MARK: - Drag plumbing

    /// Accumulate a drag; a mostly-vertical one is left alone so it can scroll.
    func drag(from source: Source, translation: CGFloat, vertical: CGFloat) {
        if source == .gesture {
            guard !routerDriving, !routerOwnsPress else { return }
            guard draggingSource != .monitor else { return }
        } else if draggingSource == .gesture {
            // The router overrules a gesture that got in first.
            dragBegan = false
            draggingSource = nil
        }
        if !dragBegan {
            // Direction is decided once, at the start of the drag.
            guard abs(translation) > abs(vertical) else { return }
            dragBegan = true
            dragStartedAt = Date()
            draggingSource = source
        }
        lastTranslation = translation
        // Clamp to the pages that exist.
        let minOffset = -CGFloat(reachable.upperBound) * pageWidth
        let maxOffset = -CGFloat(reachable.lowerBound) * pageWidth
        dragX = min(maxOffset, max(minOffset, base + translation)) - base
        log(
            "    drag[\(source == .gesture ? "gesture" : "router")] t=\(fmt(translation)) x=\(fmt(dragX))"
        )
    }

    /// Release: settle on a page, or snap back. A page turns on enough travel or
    /// on a flick, and the flick decides the direction.
    func dragEnded(from source: Source) {
        if source == .gesture, routerDriving { return }
        guard dragBegan else {
            draggingSource = nil
            dragStartedAt = nil
            return
        }
        dragBegan = false
        draggingSource = nil

        let elapsed = max(Date().timeIntervalSince(dragStartedAt ?? Date()), 0.016)
        dragStartedAt = nil

        let moved = lastTranslation
        lastTranslation = 0
        let speed = abs(moved) / elapsed
        // Travelled is measured after clamping: a pull that cannot move does not
        // count towards a page turn.
        let clampMin = -CGFloat(reachable.upperBound) * pageWidth
        let clampMax = -CGFloat(reachable.lowerBound) * pageWidth
        let travelled = min(clampMax, max(clampMin, base + moved)) - base
        let carried = abs(travelled) > pageWidth * commitFraction
        let flicked = abs(moved) >= flickTravel && speed >= flickSpeed

        guard carried || flicked else {
            dragX = 0
            log("    ended: moved=\(fmt(moved)) speed=\(fmt(speed)) — snap back")
            return
        }
        // One page per swipe; the gesture's direction is the page's direction.
        let step = moved > 0 ? -1 : 1
        let target = min(reachable.upperBound, max(reachable.lowerBound, pageIndex + step))
        log(
            "    ended: moved=\(fmt(moved)) speed=\(fmt(speed)) carried=\(carried)"
                + " flick=\(flicked) → page=\(target)"
        )
        withAnimation(.spring(response: 0.38, dampingFraction: 0.95)) {
            pageIndex = target
            dragX = 0
        }
    }

    /// Abandon an in-flight drag when another view takes the gesture over.
    func cancelDrag(reason: String) {
        guard dragBegan || dragX != 0 else { return }
        log("cancelDrag (\(reason))")
        dragBegan = false
        draggingSource = nil
        dragStartedAt = nil
        lastTranslation = 0
        dragX = 0
    }

    // MARK: - Event routing

    /// Watch the app's mouse and scroll events so drags inside a scroll view can
    /// still turn the page.
    func startWatchingEvents() {
        guard monitor == nil else { return }
        resetLog()
        monitor = NSEvent.addLocalMonitorForEvents(
            matching: [.leftMouseDown, .leftMouseDragged, .leftMouseUp, .scrollWheel]
        ) { [weak self] event in
            guard let self else { return event }
            // Returning nil swallows the event: a drag the router drives must
            // not also reach the scroll view underneath and be undone. Down and
            // up are never swallowed, so taps and controls are untouched.
            return self.route(event) ? event : nil
        }
    }

    func stopWatchingEvents() {
        if let monitor { NSEvent.removeMonitor(monitor) }
        monitor = nil
    }

    /// Route one event. Returns whether it should still be delivered; the router
    /// swallows only the drags it has taken over.
    @discardableResult
    private func route(_ event: NSEvent) -> Bool {
        switch event.type {
        case .leftMouseDown:
            pressActive = true
            routerDriving = false
            pressMode = .undecided
            lastPressWasDrag = false
            totalX = 0
            totalY = 0
            let point = location(event)
            lastPoint = point
            // Resolve the press's scroll view once; the whole drag reuses it.
            let list = listReference
            let inList = list.map { $0.convert($0.bounds, to: nil).contains(point) } ?? false
            let under = scrollView(under: event)
            // The list is only a target when the press is really inside it: its
            // hit region reports as reachable from the artwork above.
            var target: NSScrollView?
            if inList {
                target = list
            } else if let under, under !== list {
                target = under
            } else {
                target = nil
            }
            // A view with nothing to scroll is not a target. The router claims the
            // press and swallows the drag, so without this a short catalogue eats
            // every drag on the header drawer above it — and eats only the events
            // it fails to apply, which is what made the drawer jump between its
            // two resting heights mid-pull.
            if let claimed = target, !scrollsVertically(claimed) { target = nil }
            pressScrollView = target
            pressInList = inList
            pressInScrollable = target != nil
            // A press the router takes over is its own from here on, and the
            // SwiftUI gesture is kept away from it, so there is never a second
            // reading of the same hand.
            routerOwnsPress = pressInScrollable
            log(
                "down at (\(fmt(point.x)),\(fmt(point.y)))"
                    + " inList=\(inList) inScroll=\(pressScrollView != nil)"
                    + " scrubbing=\(isScrubbing())"
                    + " page=\(pageIndex) playerAvailable=\(playerAvailable)"
            )
            return true

        case .leftMouseDragged:
            guard pressActive, pressInScrollable else { return true }
            let point = location(event)
            guard let previous = lastPoint else { return true }
            lastPoint = point
            totalX += point.x - previous.x
            totalY += point.y - previous.y

            // Read the press once: which way the hand is going, and only once it
            // has gone far enough to say.
            if pressMode == .undecided {
                guard max(abs(totalX), abs(totalY)) >= axisSlop else { return true }
                log("reading press: totalX=\(fmt(totalX)) totalY=\(fmt(totalY))")
                if pressInList {
                    // The chapter list scrolls only: it never turns a page,
                    // however sideways the drag goes.
                    pressMode = .scroll
                    log("  chapter list owns this press — scroll only, no swipe")
                } else if isScrubbing() {
                    // The playhead is dragging the timeline and owns it.
                    pressMode = .held
                    log("  timeline owns this press — not routing")
                } else if abs(totalY) > abs(totalX) * axisMargin {
                    pressMode = .scroll
                } else if abs(totalX) > abs(totalY) * axisMargin {
                    pressMode = .page
                    routerDriving = true
                    log("router claims the drag (totalX=\(fmt(totalX)))")
                } else {
                    // Diagonal and undecided: wait for the hand to commit.
                    return true
                }
            }

            switch pressMode {
            case .scroll:
                // A vertical drag moves the list it landed in. Nothing on macOS
                // scrolls for a drag, so the movement is applied directly.
                scrollDrag(delta: point.y - previous.y)
                return false
            case .page:
                log("  drag dx=\(fmt(point.x - previous.x)) totalX=\(fmt(totalX)) y=\(fmt(totalY))")
                drag(from: .monitor, translation: totalX, vertical: totalY)
                // The strip is moving under the hand, so the drag stops here.
                return false
            case .undecided, .held:
                return true
            }

        case .leftMouseUp:
            guard pressActive else { return true }
            pressActive = false
            let drove = routerDriving
            // A press that travelled was a drag, not a click on what it ended over.
            lastPressWasDrag = max(abs(totalX), abs(totalY)) > clickSlop
            routerDriving = false
            pressMode = .undecided
            routerOwnsPress = false
            pressInScrollable = false
            pressInList = false
            pressScrollView = nil
            lastPoint = nil
            log("up totalX=\(fmt(totalX)) drove=\(drove)")
            if drove { dragEnded(from: .monitor) }
            return true

        case .scrollWheel:
            routeScroll(event)
            return true

        default:
            return true
        }
    }

    /// Trackpad swipes arrive as scroll events: same routing, with a quiet gap
    /// standing in for the release.
    private func routeScroll(_ event: NSEvent) {
        let dx = event.scrollingDeltaX
        let dy = event.scrollingDeltaY
        guard event.momentumPhase == [] else { return }
        guard abs(dx) > abs(dy), abs(dx) > 0.5 else { return }

        if !scrollDragActive {
            scrollDragActive = true
            scrollTotal = 0
            routerDriving = true
            draggingSource = nil
            dragBegan = false
        }
        scrollTotal += dx
        drag(from: .monitor, translation: scrollTotal, vertical: 0)
        log("scroll dx=\(fmt(dx)) total=\(fmt(scrollTotal))")

        scrollEndTask?.cancel()
        scrollEndTask = Task { [weak self] in
            try? await Task.sleep(nanoseconds: 160_000_000)
            guard !Task.isCancelled, let self else { return }
            self.scrollDragActive = false
            self.routerDriving = false
            self.dragEnded(from: .monitor)
        }
    }

    // MARK: - Event helpers

    /// The event's location in the window's base coordinates — the same space a
    /// scroll view reports its own frame in, so the two compare directly.
    private func location(_ event: NSEvent) -> CGPoint {
        event.locationInWindow
    }

    /// The scroll view an event landed in, if any.
    private func scrollView(under event: NSEvent) -> NSScrollView? {
        guard let content = event.window?.contentView else { return nil }
        var view = content.hitTest(content.convert(event.locationInWindow, from: nil))
        while let candidate = view {
            if let scrollView = candidate as? NSScrollView { return scrollView }
            view = candidate.superview
        }
        return nil
    }

    /// The chapter list hands over its own scroll view, once it exists.
    func attachList(_ scrollView: NSScrollView) {
        guard listReference !== scrollView else { return }
        listReference = scrollView
        diagnoseList()
    }

    private func listScrollView() -> NSScrollView? { listReference }

    /// Whether the view can move vertically at all. A list shorter than its
    /// viewport goes nowhere, however it is dragged.
    private func scrollsVertically(_ scrollView: NSScrollView) -> Bool {
        let clip = scrollView.contentView
        return (clip.documentView?.bounds.height ?? 0) > clip.bounds.height + 1
    }

    /// Log what the chapter list can scroll — document height against viewport —
    /// without needing a drag.
    func diagnoseList() {
        guard let scrollView = listScrollView() else {
            log("list: NO scroll view found under the chapter list")
            return
        }
        let clip = scrollView.contentView
        let doc = clip.documentView?.bounds.height ?? 0
        let visible = clip.bounds.height
        log(
            "list: doc=\(fmt(doc)) viewport=\(fmt(visible))"
                + " overflow=\(fmt(doc - visible))"
                + " scrollable=\(doc > visible + 1)"
                + " atTop=\(clip.bounds.origin.y <= 0)"
        )
    }

    /// Scroll the press's list by `delta` points, letting AppKit clamp the ends.
    private func scrollDrag(delta: CGFloat) {
        guard let scrollView = pressScrollView else {
            // Nothing to move; logged because it is the difference between a list
            // that scrolls and one that silently does not.
            log("scroll: NO scroll view for this press (delta=\(fmt(delta)))")
            return
        }
        let clip = scrollView.contentView
        let before = clip.bounds.origin
        // The list is dragged, so the content follows the pointer. A downward
        // drag lowers `locationInWindow.y`; the clip view is flipped (origin.y
        // grows downwards), so adding the delta moves the content down.
        let proposed = NSRect(
            x: before.x,
            y: before.y + delta,
            width: clip.bounds.width,
            height: clip.bounds.height
        )
        let clamped = clip.constrainBoundsRect(proposed)
        guard clamped.origin != before else {
            // Clamped back: the list is at an end, or has nothing to scroll.
            log(
                "scroll: clamped at an end delta=\(fmt(delta))"
                    + " origin=\(fmt(before.y)) doc=\(fmt(clip.documentView?.bounds.height ?? 0))"
                    + " clip=\(fmt(clip.bounds.height))"
            )
            return
        }
        clip.scroll(to: clamped.origin)
        scrollView.reflectScrolledClipView(clip)
        log("scroll: \(fmt(before.y)) → \(fmt(clamped.origin.y)) (delta \(fmt(delta)))")
    }

    private func isScrubbing() -> Bool { scrubbingProbe?() ?? false }

    // MARK: - Tracing

    private let logURL = URL(fileURLWithPath: "/tmp/audiobooks-gestures.log")

    private func fmt(_ value: CGFloat) -> String { String(format: "%.1f", value) }

    private func resetLog() {
        try? Data("— audiobooks gesture trace —\n".utf8).write(to: logURL)
    }

    private func log(_ message: String) {
        let line = "\(String(format: "%.3f", Date().timeIntervalSince1970)) \(message)\n"
        guard let data = line.data(using: .utf8) else { return }
        if let handle = try? FileHandle(forWritingTo: logURL) {
            handle.seekToEndOfFile()
            handle.write(data)
            try? handle.close()
        } else {
            try? data.write(to: logURL)
        }
    }
}
