// Throwaway probe (not part of the package: this directory sits outside
// Sources/Audiobooks). Renders the real `Scrubber` with ImageRenderer at a few
// progress ratios and prints the left end of the bar as a character map, so
// the geometry can be eyeballed without a window.
//
//   ./Scripts/toolchain-env.sh swiftc -o /tmp/scrubprobe \
//       .probe/main.swift Sources/Audiobooks/Theme.swift \
//       Sources/Audiobooks/Views/Components.swift Sources/Audiobooks/ImageCache.swift
//   /tmp/scrubprobe

import AppKit
import SwiftUI

let scale: CGFloat = 2

@MainActor
func render<V: View>(_ view: V, width: CGFloat, height: CGFloat) -> CGImage? {
    // Composite over white: the app draws this on white cards/veils, and a
    // transparent backdrop makes the semi-transparent track read as black.
    let content = ZStack { Color.white; view }.frame(width: width, height: height)
    let renderer = ImageRenderer(content: content)
    renderer.scale = scale
    return renderer.cgImage
}

/// Character map of an image region in *points*, one char per 4x2 pixels
/// (terminal cells are twice as tall as they are wide).
func dump(_ image: CGImage, x: CGFloat, y: CGFloat, width: CGFloat, height: CGFloat) -> String {
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
        let r = Double(bytes[i]), g = Double(bytes[i + 1]), b = Double(bytes[i + 2])
        if bytes[i + 3] < 10 { return " " }  // transparent backdrop
        // primary #7a86b6 — played fill and the thumb ring
        if abs(r - 122) < 34 && abs(g - 134) < 34 && abs(b - 182) < 34 { return "B" }
        // dark #495c83 — borders and dark text
        if r < 150 && g < 155 && b < 190 { return "#" }
        if r > 246 && g > 246 && b > 246 { return "." }  // white
        if r > 200 && g > 200 && b > 210 { return "-" }  // the grey track
        return ":"                                       // page / artwork wash
    }

    let x0 = Int(x * scale), x1 = Int((x + width) * scale)
    let y0 = Int(y * scale), y1 = Int((y + height) * scale)
    var out = ""
    var py = y0
    while py < y1 {
        var line = ""
        var px = x0
        while px < x1 {
            line.append(char(min(px + 2, w - 1), min(py + 1, h - 1)))
            px += 4
        }
        out += line + "\n"
        py += 2
    }
    return out
}

@MainActor
func probe(label: String, ratio: Double) {
    let width: CGFloat = 360
    let height: CGFloat = 16
    // Bar height / ring come from argv so one binary can probe both call sites:
    // the mini bar (8pt, 2pt ring) and the player screen (10pt, 2pt ring).
    let bar = CGFloat(Double(CommandLine.arguments.dropFirst().first ?? "10") ?? 10)
    let ring = CommandLine.arguments.count > 2 ? (Double(CommandLine.arguments[2]) ?? 2) : 2
    let view = Scrubber(
        value: ratio * 1000,
        duration: 1000,
        barHeight: bar,
        ringWidth: ring,
        onPreview: { _ in },
        onCommit: { _ in }
    )
    guard let image = render(view, width: width, height: height) else {
        print("\(label): render failed")
        return
    }
    print("--- \(label) (left 72pt, whole height) ---")
    print(dump(image, x: 0, y: 0, width: 72, height: height))
    print("--- \(label) (right end) ---")
    print(dump(image, x: width - 72, y: 0, width: 72, height: height))
    let thumb = bar + (width - bar) * CGFloat(ratio)
    print("--- \(label) (thumb, 26pt window) ---")
    print(dump(image, x: max(0, thumb - 13), y: 0, width: 26, height: height))
}

// Top-level code here is not implicitly main-actor isolated under this
// toolchain, so hop onto the main actor explicitly (we are already on it).
MainActor.assumeIsolated {
    probe(label: "ratio 0.000", ratio: 0)
    probe(label: "ratio 0.010", ratio: 0.01)
    probe(label: "ratio 0.030", ratio: 0.03)
    probe(label: "ratio 0.100", ratio: 0.10)
    probe(label: "ratio 1.000", ratio: 1)
}
