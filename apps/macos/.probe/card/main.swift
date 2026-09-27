// Throwaway probe (not part of the package). The player's card is the only view
// in the app that wraps itself in `.compositingGroup()` before clipping and
// shadowing, and the player page is reported to come out transparent. This
// renders the same card with and without that group, on the page wash, and
// reports how much of the page the card actually covers — so we can tell whether
// the offscreen pass is what fails to paint.
//
//   ./Scripts/toolchain-env.sh swiftc -o /tmp/cardprobe \
//       .probe/card/main.swift Sources/Audiobooks/Theme.swift
//   /tmp/cardprobe

import AppKit
import SwiftUI

/// A stand-in for the player card: the same order of background → white →
/// clipShape → stroke → shadow as PlayerScreen.playerCard, with or without the
/// compositing group.
private struct Card: View {
    var grouped: Bool

    var body: some View {
        content
            .background(Color.white)
            .modifier(MaybeGroup(grouped: grouped))
            .clipShape(RoundedRectangle(cornerRadius: 14))
            .overlay(
                RoundedRectangle(cornerRadius: 14)
                    .strokeBorder(Theme.cardBorder, lineWidth: 3)
            )
            .shadow(color: .black.opacity(0.2), radius: 12, y: 5)
            .padding(10)
    }

    /// What the two halves hold in common: a veiled "artwork" block over a white
    /// chapter list, exactly as the real card stacks them.
    private var content: some View {
        VStack(spacing: 0) {
            VStack(spacing: 0) {
                Rectangle().fill(Theme.primary.opacity(0.5)).frame(height: 90)
                HStack(spacing: 8) {
                    Rectangle().fill(Theme.dark.opacity(0.3)).frame(width: 60, height: 9)
                    Spacer(minLength: 0)
                    Rectangle().fill(Theme.dark.opacity(0.3)).frame(width: 90, height: 9)
                }
                .padding(14)
                .frame(maxWidth: .infinity)
            }
            VStack(spacing: 0) {
                ForEach(0..<6, id: \.self) { _ in
                    HStack(spacing: 8) {
                        Rectangle().fill(Theme.dark.opacity(0.3)).frame(width: 22, height: 8)
                        Rectangle().fill(Theme.dark.opacity(0.3)).frame(width: 140, height: 9)
                        Spacer(minLength: 0)
                    }
                    .padding(.horizontal, 10)
                    .padding(.vertical, 7)
                }
            }
            .frame(maxWidth: .infinity)
            .background(Color.white)
        }
    }
}

private struct MaybeGroup: ViewModifier {
    var grouped: Bool

    func body(content: Content) -> some View {
        if grouped {
            content.compositingGroup()
        } else {
            content
        }
    }
}

let scale: CGFloat = 2

/// How much of the page the card covers, and whether the wash shows through it:
/// counts pixels that are neither the wash nor the card's own white.
@MainActor
func coverage<V: View>(_ view: V, width: CGFloat, height: CGFloat) -> String {
    let renderer = ImageRenderer(content: view.frame(width: width, height: height))
    renderer.scale = scale
    guard let image = renderer.cgImage else { return "render failed" }
    let w = image.width, h = image.height
    var bytes = [UInt8](repeating: 0, count: w * h * 4)
    guard let ctx = CGContext(
        data: &bytes, width: w, height: h, bitsPerComponent: 8,
        bytesPerRow: w * 4, space: CGColorSpaceCreateDeviceRGB(),
        bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue
    ) else { return "context failed" }
    ctx.draw(image, in: CGRect(x: 0, y: 0, width: w, height: h))

    var white = 0, wash = 0, other = 0, clear = 0
    for y in 0..<h {
        for x in 0..<w {
            let i = (y * w + x) * 4
            let a = bytes[i + 3]
            let (r, g, b) = (bytes[i], bytes[i + 1], bytes[i + 2])
            if a < 8 { clear += 1 }
            else if r > 245 && g > 245 && b > 245 { white += 1 }
            else if abs(Int(r) - 200) < 24 && abs(Int(g) - 182) < 24 && abs(Int(b) - 226) < 24 { wash += 1 }
            else { other += 1 }
        }
    }
    let total = Double(w * h)
    return String(
        format: "clear %.1f%%  white %.1f%%  wash %.1f%%  other %.1f%%",
        Double(clear) / total * 100,
        Double(white) / total * 100,
        Double(wash) / total * 100,
        Double(other) / total * 100
    )
}

MainActor.assumeIsolated {
    for grouped in [true, false] {
        let label = grouped ? "with .compositingGroup()" : "without .compositingGroup()"
        print("\(label): \(coverage(ZStack { Theme.page; Card(grouped: grouped) }, width: 384, height: 400))")
    }
}
