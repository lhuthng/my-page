import SwiftUI

/// Three screens in a horizontal strip — [Player | Library | About] — with a
/// seam between them. The library is the resting page. The strip owns a drag
/// gesture, but a drag inside a scroll view never reaches an ancestor, which is
/// why the paging state and the event routing live in `PagerModel`.
struct RootPager: View {
    @EnvironmentObject private var player: PlayerModel
    @EnvironmentObject private var updater: Updater
    @StateObject private var pager = PagerModel()

    /// The boundary drawn between two screens; it travels with the strip, so
    /// the page edge stays visible mid-swipe.
    private let seam: CGFloat = 2

    var body: some View {
        GeometryReader { geo in
            let width = geo.size.width
            // One page of travel: the screen plus the seam it owns.
            let travel = width + seam

            HStack(spacing: 0) {
                PlayerScreen(onBack: { pager.show(1) })
                    .frame(width: width, height: geo.size.height)

                boundary

                BrowserScreen()
                    .frame(width: width, height: geo.size.height)

                boundary

                AboutScreen(onBack: { pager.show(1) })
                    .frame(width: width, height: geo.size.height)
            }
            // dragX follows the finger while dragging; the commit animation moves
            // the page and dragX together, so the strip never snaps back first.
            .offset(x: pager.base + pager.dragX)
            // Simultaneous, so surfaces that do not claim a drag still page.
            // Drags inside a scroll view never get here; see PagerModel.
            .simultaneousGesture(pagerGesture)
            .environmentObject(pager)
            .onAppear {
                pager.pageWidth = travel
                pager.playerAvailable = player.book != nil
                // Strong capture on purpose: the pager outlives this view, so
                // there is no cycle.
                pager.scrubbingProbe = { player.isScrubbing }
                pager.startWatchingEvents()
            }
            .onChange(of: travel) { _, newTravel in pager.pageWidth = newTravel }
            .onChange(of: player.book?.id) { _, _ in
                pager.playerAvailable = player.book != nil
            }
            .onDisappear { pager.stopWatchingEvents() }
        }
        .environmentObject(updater)
        .onAppear { updater.checkSilently() }
        .alert("Update available", isPresented: $updater.shouldOffer) {
            if case .available(let release) = updater.state {
                Button("Install and restart") { updater.install(release) }
                Button("Later") { updater.shouldOffer = false }
            } else {
                Button("OK") { updater.shouldOffer = false }
            }
        } message: {
            // Always a concrete Text, from a String: a message builder that can
            // come out empty makes the alert render "the data couldn't be read",
            // and a LocalizedStringKey here fails to resolve on top of that.
            Text(offerMessage)
        }
    }

    private var offerMessage: String {
        guard case .available(let release) = updater.state else { return "" }
        return "Audiobooks \(release.version) is available. You have \(AppInfo.version)."
    }

    private var boundary: some View {
        Rectangle()
            .fill(Theme.dark)
            .frame(width: seam)
    }

    private var pagerGesture: some Gesture {
        // A mouse swipe is short, so the slop stays small.
        DragGesture(minimumDistance: 6)
            .onChanged { gesture in
                // A drag that began on the timeline belongs to the timeline.
                if player.isScrubbing {
                    pager.cancelDrag(reason: "timeline")
                    return
                }
                pager.drag(
                    from: .gesture,
                    translation: gesture.translation.width,
                    vertical: gesture.translation.height
                )
            }
            .onEnded { _ in
                pager.dragEnded(from: .gesture)
            }
    }
}
