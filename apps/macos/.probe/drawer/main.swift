// Throwaway probe (not part of the package). Mirrors the structure of the
// library header drawer in BrowserScreen — a header whose height is measured
// *through* the clipping frame, with the grip band below it — to answer the one
// question that decides whether it works at all: does the GeometryReader behind
// the header report the header's natural height, or the clipped frame's height?
//
//   ./Scripts/toolchain-env.sh swiftc -o /tmp/drawerprobe .probe/drawer/main.swift
//   /tmp/drawerprobe

import AppKit
import SwiftUI

struct HeaderHeight: PreferenceKey {
    static var defaultValue: CGFloat = 0
    static func reduce(value: inout CGFloat, nextValue: () -> CGFloat) {
        value = max(value, nextValue())
    }
}

/// Stands in for the real header: known heights, so the report can be checked
/// against arithmetic.
struct FakeHeader: View {
    var body: some View {
        VStack(spacing: 10) {
            HStack {
                VStack(alignment: .leading, spacing: 1) {
                    Text("Audiobooks").font(.title3.weight(.bold))
                    Text("1 title").font(.caption)
                }
                Spacer()
                Circle().frame(width: 30, height: 30)
                Circle().frame(width: 30, height: 30)
            }
            HStack(spacing: 8) {
                Capsule().frame(height: 30)
                Capsule().frame(width: 70, height: 30)
            }
        }
        .padding(.horizontal, 12)
        .padding(.vertical, 12)
        .background(Color.white)
        .background(
            GeometryReader { geo in
                Color.clear.preference(key: HeaderHeight.self, value: geo.size.height)
            }
        )
    }
}

struct Drawer: View {
    @State var reveal: CGFloat
    @State var headerHeight: CGFloat = 0

    var body: some View {
        VStack(spacing: 0) {
            FakeHeader()
                .frame(height: max(reveal, 0), alignment: .top)
                .clipped()
            // Stand-in for the grip band: only its height matters to the
            // question this probe asks, so the grip itself is drawn as a bar.
            Capsule()
                .fill(Theme.dark.opacity(0.28))
                .frame(width: 34, height: 4)
                .frame(maxWidth: .infinity)
                .frame(height: 18)
        }
        .background(Color.white)
        .onPreferenceChange(HeaderHeight.self) { height in
            guard height > 0, abs(height - headerHeight) > 0.5 else { return }
            headerHeight = height
        }
        .overlay(alignment: .bottomLeading) {
            Text("measured: \(Int(headerHeight))pt  reveal: \(Int(reveal))pt")
                .font(.caption2)
                .padding(2)
        }
    }
}

/// Captures the reported height so it can be printed as a number.
final class Box: ObservableObject {
    @Published var height: CGFloat = 0
}

/// The header inside the clipped frame the drawer puts it in — the case that
/// matters: a height of 0 (nothing pulled down yet) must still measure the
/// header at its natural height, or the drawer could never open.
struct Measured: View {
    @ObservedObject var box: Box

    var body: some View {
        FakeHeader()
            .frame(height: 0, alignment: .top)
            .clipped()
            .onPreferenceChange(HeaderHeight.self) { height in box.height = height }
    }
}

let scale: CGFloat = 2

@MainActor
func render<V: View>(
    _ view: V,
    width: CGFloat,
    height: CGFloat,
    alignment: Alignment = .center
) -> CGImage? {
    let renderer = ImageRenderer(
        content: view.frame(width: width, height: height, alignment: alignment)
    )
    renderer.scale = scale
    return renderer.cgImage
}

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
        if r < 130 && g < 140 && b < 175 { return "#" }  // dark ink / text
        if r > 246 && g > 246 && b > 246 { return "." }  // white
        if r > 210 && g > 200 && b > 210 { return ":" }  // light grey (page wash)
        return "+"
    }

    var out = ""
    var py = 0
    while py < h {
        var line = ""
        var px = 0
        while px < w {
            line.append(char(min(px, w - 1), min(py, h - 1)))
            px += 4
        }
        out += line + "\n"
        py += 4
    }
    return out
}

@MainActor
func probe(label: String, reveal: CGFloat) {
    // Top-aligned: the drawer rests against the window's top edge, so the
    // character map should read from row 0.
    guard let image = render(Drawer(reveal: reveal), width: 200, height: 320, alignment: .top) else {
        print("\(label): render failed")
        return
    }
    print("--- \(label) ---")
    print(dump(image))
}

MainActor.assumeIsolated {
    let box = Box()
    _ = render(Measured(box: box), width: 200, height: 160)
    print("measured through a 0pt frame: \(box.height)pt")
    let open = Box()
    _ = render(
        FakeHeader()
            .frame(height: 400, alignment: .top)
            .clipped()
            .onPreferenceChange(HeaderHeight.self) { open.height = $0 },
        width: 200,
        height: 400
    )
    print("measured through a 400pt frame: \(open.height)pt")

    probe(label: "tucked", reveal: 0)
    probe(label: "half open", reveal: 50)
    probe(label: "fully open", reveal: 200)
}
