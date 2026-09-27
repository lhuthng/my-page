// Throwaway probe: list every *saturated* colour in a screenshot with its
// bounding box, so a stray artifact can be located by hue and position.
//
//   ./Scripts/toolchain-env.sh swiftc -o /tmp/scan .probe/scan/main.swift
//   /tmp/scan <path.png> [minSaturation]

import AppKit
import CoreGraphics
import Foundation

let args = CommandLine.arguments
guard args.count > 1,
      let source = CGImageSourceCreateWithURL(URL(fileURLWithPath: args[1]) as CFURL, nil),
      let image = CGImageSourceCreateImageAtIndex(source, 0, nil)
else {
    print("usage: scan <png> [minSaturation]")
    exit(1)
}
let minSat = args.count > 2 ? (Int(args[2]) ?? 24) : 24

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

struct Hit {
    var count = 0
    var minX = Int.max, minY = Int.max, maxX = Int.min, maxY = Int.min
}

print("image \(w)x\(h), saturation ≥ \(minSat)")
var hits: [String: Hit] = [:]
for y in 0..<h {
    for x in 0..<w {
        let i = (y * w + x) * 4
        let r = Int(bytes[i]), g = Int(bytes[i + 1]), b = Int(bytes[i + 2])
        let sat = max(r, max(g, b)) - min(r, min(g, b))
        guard sat >= minSat else { continue }
        // Bucket to /16 so a gradient reads as one colour.
        let key = String(format: "%02X%02X%02X", r / 16 * 16, g / 16 * 16, b / 16 * 16)
        var hit = hits[key] ?? Hit()
        hit.count += 1
        hit.minX = min(hit.minX, x); hit.maxX = max(hit.maxX, x)
        hit.minY = min(hit.minY, y); hit.maxY = max(hit.maxY, y)
        hits[key] = hit
    }
}

for (key, hit) in hits.sorted(by: { $0.value.count > $1.value.count }).prefix(20) {
    print(
        "  #\(key) x\(hit.minX)-\(hit.maxX) y\(hit.minY)-\(hit.maxY) (\(hit.count)px)"
    )
}
