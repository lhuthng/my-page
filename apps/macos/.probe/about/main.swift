// Throwaway probe (not part of the package). Renders the real `AboutCard` on
// the page wash and prints it as a character map, so the card's layout can be
// checked without a window.
//
//   ./Scripts/toolchain-env.sh swiftc -o /tmp/aboutprobe \
//       .probe/about/main.swift Sources/Audiobooks/Theme.swift \
//       Sources/Audiobooks/AppInfo.swift Sources/Audiobooks/Views/LogoMark.swift \
//       Sources/Audiobooks/Views/AboutCard.swift
//   /tmp/aboutprobe

import AppKit
import SwiftUI

let scale: CGFloat = 2

@MainActor
func render<V: View>(_ view: V, width: CGFloat, height: CGFloat) -> CGImage? {
    let renderer = ImageRenderer(content: view.frame(width: width, height: height))
    renderer.scale = scale
    return renderer.cgImage
}

/// One character per 6x12 pixel block (terminal cells are about 1:2), so the
/// whole card fits on screen at readable proportions.
func dump(_ image: CGImage, blockW: Int = 6, blockH: Int = 12) -> String {
    let w = image.width, h = image.height
    var bytes = [UInt8](repeating: 0, count: w * h * 4)
    guard let ctx = CGContext(
        data: &bytes,
        width: w,
        height: h,
        bitsPerComponent: 8,
        bytesPerRow: w * 4,
        space: CGColorSpaceCreateDeviceRGB(),
        bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue
    ) else { return "context failed" }
    ctx.draw(image, in: CGRect(x: 0, y: 0, width: w, height: h))

    /// Average luminance of a block, mapped to a character.
    func char(_ bx: Int, _ by: Int) -> Character {
        var total = 0.0
        var count = 0.0
        var navy = 0.0
        for y in by..<min(by + blockH, h) {
            for x in bx..<min(bx + blockW, w) {
                let i = (y * w + x) * 4
                let r = Double(bytes[i]), g = Double(bytes[i + 1]), b = Double(bytes[i + 2])
                total += (r * 0.299 + g * 0.587 + b * 0.114)
                // How blue-and-dark the pixel is: the ink of text and borders.
                if b > r + 10 { navy += 1 }
                count += 1
            }
        }
        let luma = total / max(count, 1)
        if navy / max(count, 1) > 0.5, luma < 200 { return "#" }  // ink
        if luma > 246 { return "." }                              // white card
        if luma > 228 { return ":" }                              // page wash
        return "+"
    }

    var out = ""
    var by = 0
    while by < h {
        var line = ""
        var bx = 0
        while bx < w {
            line.append(char(bx, by))
            bx += blockW
        }
        out += line + "\n"
        by += blockH
    }
    return out
}

MainActor.assumeIsolated {
    // The real width: the window minus the screen's 12pt side margins.
    let view = ZStack { Theme.page; AboutCard() }
    guard let image = render(view, width: 360, height: 430) else {
        print("render failed")
        exit(1)
    }
    print("--- AboutCard on the page wash (360pt wide) ---")
    print(dump(image))
}
