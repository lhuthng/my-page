// Throwaway probe (not part of the package: this directory sits outside
// Sources/Audiobooks). Renders the real `LogoMark` with ImageRenderer and
// prints it as a character map, so the SVG paths can be checked without a
// window.
//
//   ./Scripts/toolchain-env.sh swiftc -o /tmp/logoprobe \
//       .probe/logo/main.swift Sources/Audiobooks/Theme.swift \
//       Sources/Audiobooks/Views/LogoMark.swift
//   /tmp/logoprobe

import AppKit
import SwiftUI

let scale: CGFloat = 2

@MainActor
func render<V: View>(_ view: V, width: CGFloat, height: CGFloat) -> CGImage? {
    let content = view.frame(width: width, height: height)
    let renderer = ImageRenderer(content: content)
    renderer.scale = scale
    return renderer.cgImage
}

/// One character per 2x2 pixel block, classified by how dark it is and by its
/// hue: `#` dark navy strokes, `.` white (the halo), `:` the page wash.
func dump(_ image: CGImage) -> String {
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

    func char(_ px: Int, _ py: Int) -> Character {
        let i = (py * w + px) * 4
        let r = Int(bytes[i]), g = Int(bytes[i + 1]), b = Int(bytes[i + 2])
        if bytes[i + 3] < 10 { return " " }
        // Theme.dark #495c83 — the mark's own stroke.
        if r < 130 && g < 140 && b < 175 { return "#" }
        if r > 246 && g > 246 && b > 246 { return "." }
        return ":"  // the page wash behind the halo
    }

    var out = ""
    var py = 0
    while py < h {
        var line = ""
        var px = 0
        while px < w {
            line.append(char(min(px, w - 1), min(py, h - 1)))
            px += 2
        }
        out += line + "\n"
        py += 2
    }
    return out
}

@MainActor
func probe(label: String, halo: Bool) {
    let view = ZStack {
        if halo { Theme.page } else { Color.white }
        LogoMark(halo: halo).frame(width: 62)
    }
    guard let image = render(view, width: 72, height: 62) else {
        print("\(label): render failed")
        return
    }
    print("--- \(label) ---")
    print(dump(image))
}

MainActor.assumeIsolated {
    probe(label: "halo: false, on white", halo: false)
    probe(label: "halo: true, on the page wash", halo: true)
}
