// Throwaway probe: renders the player page's loading skeleton at the real
// window size and prints it as a character map, so the silhouette can be
// checked without a window.
//
//   ./Scripts/toolchain-env.sh swiftc -o /tmp/skelprobe .probe/skeleton/main.swift \
//       $(ls Sources/Audiobooks/*.swift Sources/Audiobooks/Views/*.swift | grep -v AudiobooksApp)
//   /tmp/skelprobe

import AppKit
import SwiftUI

@MainActor
func render<V: View>(_ view: V, width: CGFloat, height: CGFloat) -> CGImage? {
    let content = ZStack { Theme.page; view }.frame(width: width, height: height)
    let renderer = ImageRenderer(content: content)
    renderer.scale = 1
    return renderer.cgImage
}

func map(_ image: CGImage) -> String {
    let w = image.width, h = image.height
    var bytes = [UInt8](repeating: 0, count: w * h * 4)
    guard let ctx = CGContext(
        data: &bytes, width: w, height: h, bitsPerComponent: 8,
        bytesPerRow: w * 4, space: CGColorSpaceCreateDeviceRGB(),
        bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue
    ) else { return "context failed" }
    ctx.draw(image, in: CGRect(x: 0, y: 0, width: w, height: h))

    var out = ""
    for py in stride(from: 0, to: h, by: 4) {
        var line = ""
        for px in stride(from: 0, to: w, by: 3) {
            let i = (py * w + px) * 4
            let r = Double(bytes[i]), g = Double(bytes[i + 1]), b = Double(bytes[i + 2])
            if r > 248 && g > 248 && b > 248 { line.append(".") }        // card white
            else if r > 228 && g > 232 && b > 234 { line.append(":") }   // placeholder grey
            else if r > 150 && b > 190 { line.append(" ") }              // page wash / shadow
            else { line.append("#") }                                    // ink
        }
        out += String(format: "%3d ", py) + line + "\n"
    }
    return out
}

MainActor.assumeIsolated {
    let view = PlayerSkeleton(title: "Some Book")
        .padding(10)
    guard let image = render(view, width: 384, height: 520) else {
        print("render failed")
        return
    }
    print("--- PlayerSkeleton in the 384x520 window ---")
    print(map(image))
}
