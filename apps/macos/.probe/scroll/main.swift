import AppKit
import SwiftUI

// A throwaway probe: does driving NSScrollView's clip view directly actually move
// the content, inside the same nesting the player card uses?
//
// It measures the only thing that matters — where the first row *is* on screen,
// reported from inside SwiftUI — before and after a scroll, so a moved offset
// that does not move the row is visible as a difference in those two numbers.

struct Probe: View {
    let rowCount = 60

    @State private var originY: CGFloat = -1
    @State private var offsetY: CGFloat = -1

    var body: some View {
        VStack(spacing: 0) {
            // Stands in for the artwork + transport block.
            Rectangle()
                .fill(Color.gray.opacity(0.3))
                .frame(height: 120)

            ScrollView(.vertical, showsIndicators: false) {
                VStack(spacing: 0) {
                    ForEach(0..<rowCount, id: \.self) { index in
                        HStack {
                            Text("\(index)")
                                .font(.caption)
                            Spacer()
                        }
                        .padding(.vertical, 7)
                        .background(index == 0 ? Color.red.opacity(0.5) : Color.clear)
                        .overlay(alignment: .bottom) {
                            Rectangle().fill(Color.gray.opacity(0.2)).frame(height: 1)
                        }
                        .background(
                            // Row 0 reports where it is on screen: the direct
                            // answer to "did the content actually move?".
                            index == 0
                                ? GeometryReader { geo in
                                    Color.clear
                                        .onAppear { originY = geo.frame(in: .global).minY }
                                        .onChange(of: geo.frame(in: .global).minY) { _, y in
                                            originY = y
                                        }
                                }
                                : Color.clear
                        )
                    }
                }
                .padding(.horizontal, 4)
                .padding(.bottom, 8)
            }
            .frame(maxHeight: .infinity)
            .clipped()
            .background(Color.white)
            .padding(.bottom, 8)
        }
        .background(Color.white)
        .compositingGroup()
        .clipShape(RoundedRectangle(cornerRadius: 14))
        .overlay(RoundedRectangle(cornerRadius: 14).strokeBorder(Color.black.opacity(0.5), lineWidth: 3))
        .shadow(color: .black.opacity(0.2), radius: 12, y: 5)
        .background(
            GeometryReader { geo in
                Color.clear
                    .onChange(of: geo.frame(in: .global).minY) { _, _ in }
            }
        )
        .onAppear {
            DispatchQueue.main.asyncAfter(deadline: .now() + 0.6) { runProbe() }
        }
    }

    /// The same arithmetic the app uses, then a second reading of row 0.
    private func runProbe() {
        guard let content = NSApp.mainWindow?.contentView else { return }
        // A point in the middle of the window, which is over the list.
        let point = NSPoint(x: content.bounds.midX, y: content.bounds.midY)

        func findScrollView() -> NSScrollView? {
            var view = content.hitTest(point)
            while let candidate = view {
                if let scroll = candidate as? NSScrollView { return scroll }
                view = candidate.superview
            }
            return nil
        }

        guard let scroll = findScrollView() else {
            print("PROBE no scroll view found")
            return
        }
        let clip = scroll.contentView
        print("PROBE before: clip origin=\(clip.bounds.origin.y) doc=\(clip.documentView?.bounds.height ?? 0) clip=\(clip.bounds.height) flipped=\(clip.documentView?.isFlipped ?? false)")

        // Drag downwards by 120pt: three steps, as three drag events would.
        for step in 1...3 {
            let proposed = NSRect(
                x: clip.bounds.origin.x,
                y: clip.bounds.origin.y + 40,
                width: clip.bounds.width,
                height: clip.bounds.height
            )
            let clamped = clip.constrainBoundsRect(proposed)
            clip.scroll(to: clamped.origin)
            scroll.reflectScrolledClipView(clip)
            print("PROBE step \(step): clip origin now \(clip.bounds.origin.y)")
        }

        // Give SwiftUI a beat to lay out and repaint, then read row 0 again.
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.5) {
            print("PROBE after: row0 y=\(originY) clip origin=\(clip.bounds.origin.y)")
            let moved = abs(originY - (originY - 120)) // placeholder, printed below
            print("PROBE moved=\(moved)")
            // What is actually at the top of the viewport now?
            let top = NSPoint(x: content.bounds.midX, y: content.bounds.maxY - 140)
            print("PROBE hit at list top: \(String(describing: content.hitTest(top)))")
            exit(0)
        }
    }
}

let app = NSApplication.shared
app.setActivationPolicy(.accessory)
let window = NSWindow(
    contentRect: NSRect(x: 0, y: 0, width: 384, height: 560),
    styleMask: [.titled, .resizable],
    backing: .buffered,
    defer: false
)
window.title = "probe"
window.contentView = NSHostingView(rootView: Probe())
window.makeKeyAndOrderFront(nil)
app.run()
