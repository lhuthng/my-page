// Throwaway probe: load a PNG and print it as a character map plus a colour
// histogram, so a screenshot can be inspected as text.
//
//   ./Scripts/toolchain-env.sh swiftc -o /tmp/imgmap .probe/imgmap.swift
//   /tmp/imgmap <path.png>

import AppKit
import CoreGraphics
import Foundation

let path = CommandLine.arguments.count > 1 ? CommandLine.arguments[1] : ""
guard let source = CGImageSourceCreateWithURL(URL(fileURLWithPath: path) as CFURL, nil),
      let image = CGImageSourceCreateImageAtIndex(source, 0, nil)
else {
    print("could not load \(path)")
    exit(1)
}

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
) else {
    print("context failed")
    exit(1)
}
ctx.draw(image, in: CGRect(x: 0, y: 0, width: w, height: h))

func rgb(_ x: Int, _ y: Int) -> (Int, Int, Int) {
    let i = (y * w + x) * 4
    return (Int(bytes[i]), Int(bytes[i + 1]), Int(bytes[i + 2]))
}

print("size: \(w)x\(h)")

// Colour histogram, bucketed to /8 so near-identical shades merge.
var counts: [String: Int] = [:]
for y in 0..<h {
    for x in 0..<w {
        let (r, g, b) = rgb(x, y)
        counts["\(r / 8 * 8),\(g / 8 * 8),\(b / 8 * 8)", default: 0] += 1
    }
}
let top = counts.sorted { $0.value > $1.value }.prefix(10)
print("top colours (r,g,b :: count):")
for (key, value) in top { print("  \(key) :: \(value)") }

// Character map: one char per 2x2 pixel block (terminal cells are ~1:2).
func classify(_ r: Int, _ g: Int, _ b: Int) -> Character {
    let maxc = max(r, max(g, b))
    let minc = min(r, min(g, b))
    let sat = maxc - minc
    if sat < 12 {
        if maxc > 245 { return "." }        // white
        if maxc > 225 { return "-" }        // very light grey
        if maxc > 170 { return "+" }        // mid grey
        return "@"                          // dark
    }
    // Blue-ish: primary #7a86b6 is (122,134,182), dark #495c83 is (73,92,131).
    if b >= r && b > g {
        if maxc > 170 { return "B" }        // the light primary blue
        return "#"                          // the darker navy
    }
    return ":"                              // anything else (warm/artwork)
}

let step = 2
var line = ""
var y = 0
while y < h {
    line = ""
    var x = 0
    while x < w {
        let (r, g, b) = rgb(min(x, w - 1), min(y, h - 1))
        line.append(classify(r, g, b))
        x += step
    }
    print(line)
    y += step
}
