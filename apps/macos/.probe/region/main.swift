// Throwaway probe: print exact pixel colours in a region of a screenshot, so a
// suspected artifact can be identified by its actual RGB rather than by a
// character approximation.
//
//   ./Scripts/toolchain-env.sh swiftc -o /tmp/region .probe/region/main.swift
//   /tmp/region <path.png> <x> <y> <width> <height> [step]

import AppKit
import CoreGraphics
import Foundation

let args = CommandLine.arguments
guard args.count > 5,
      let source = CGImageSourceCreateWithURL(URL(fileURLWithPath: args[1]) as CFURL, nil),
      let image = CGImageSourceCreateImageAtIndex(source, 0, nil)
else {
    print("usage: region <png> <x> <y> <w> <h> [step]")
    exit(1)
}
let x0 = Int(args[2]) ?? 0
let y0 = Int(args[3]) ?? 0
let width = Int(args[4]) ?? 10
let height = Int(args[5]) ?? 10
let step = args.count > 6 ? (Int(args[6]) ?? 1) : 1

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

print("image \(w)x\(h), region x \(x0)..\(x0 + width) y \(y0)..\(y0 + height)")
// Colour histogram of the region, so an artifact's palette shows up first.
var counts: [String: Int] = [:]
var y = y0
while y < min(y0 + height, h) {
    var x = x0
    while x < min(x0 + width, w) {
        let i = (y * w + x) * 4
        let key = String(format: "%02X%02X%02X", bytes[i], bytes[i + 1], bytes[i + 2])
        counts[key, default: 0] += 1
        x += 1
    }
    y += 1
}
print("colours (hex :: count):")
for (key, value) in counts.sorted(by: { $0.value > $1.value }).prefix(12) {
    print("  #\(key) :: \(value)")
}

print("rows (hex, brightest first):")
y = y0
while y < min(y0 + height, h) {
    var line = "y\(y): "
    var x = x0
    while x < min(x0 + width, w) {
        let i = (y * w + x) * 4
        line += String(format: "%02X%02X%02X ", bytes[i], bytes[i + 1], bytes[i + 2])
        x += step
    }
    print(line)
    y += step
}
