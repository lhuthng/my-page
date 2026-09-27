// Throwaway probe: renders the marquee line the way the players use it, so the
// layout can be checked without a window (the animation itself is not visible
// here — this only proves the line is clipped to its container, sits on the
// leading edge, and does not inflate the layout).
//
//   ./Scripts/toolchain-env.sh swiftc -o /tmp/marqueeprobe .probe/marquee.swift \
//       Sources/Audiobooks/Theme.swift Sources/Audiobooks/Views/Components.swift \
//       Sources/Audiobooks/ImageCache.swift
//   /tmp/marqueeprobe

import AppKit
import SwiftUI

@MainActor
func render<V: View>(_ view: V, width: CGFloat, height: CGFloat) -> CGImage? {
    let content = ZStack { Color.white; view }.frame(width: width, height: height)
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
    for py in stride(from: 0, to: h, by: 2) {
        var line = ""
        for px in 0..<w {
            let i = (py * w + px) * 4
            let r = Double(bytes[i]), g = Double(bytes[i + 1]), b = Double(bytes[i + 2])
            let dark = r < 200 && g < 200 && b < 220
            line.append(dark ? "#" : ".")
        }
        out += "y\(py) \(line)\n"
    }
    return out
}

/// The travel is a pure function of the clock, so it can be checked here: over
/// one cycle the offset should hold at 0, ramp to the full overflow, hold, and
/// come back — i.e. it must actually move.
MainActor.assumeIsolated {
    let overflow: CGFloat = 100
    let travel = max(2.5, Double(6 + overflow / 25) / 2)
    let cycle = 2 * (travel + 0.9)
    print("--- offset over one \(cycle)s cycle (overflow \(Int(overflow))pt) ---")
    var previous: CGFloat = -1
    var moved = 0
    for step in 0...Int(cycle * 4) {
        let date = Date(timeIntervalSinceReferenceDate: Double(step) / 4)
        let value = MarqueeLine.offset(overflow: overflow, at: date)
        if value != previous { moved += 1 }
        previous = value
        if step % 2 == 0 {
            print(String(format: "t=%4.1fs  offset=%6.1f", Double(step) / 4, value))
        }
    }
    print("distinct positions: \(moved) (must be many)")
}

MainActor.assumeIsolated {
    let long = "Luyện Tạo Thần Binh Dao Phay Và Những Chuyện Kể Không Hồi Kết"
    let card = VStack(alignment: .leading, spacing: 2) {
        MarqueeLine(text: long, font: .system(size: 13, weight: .semibold), height: 16)
        MarqueeLine(text: "251 - \(long)", font: .system(size: 11), height: 14)
        MarqueeLine(text: "4:52", font: .system(size: 11), height: 14)
    }
    .frame(maxWidth: .infinity, alignment: .leading)
    .padding(10)

    guard let image = render(card, width: 200, height: 70) else {
        print("render failed")
        return
    }
    print("--- marquee lines in a 180pt column ---")
    print(map(image))
}
