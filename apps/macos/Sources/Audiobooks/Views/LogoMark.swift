import SwiftUI

/// The site's logo mark, redrawn as vectors so it stays sharp at any size and
/// needs no bundled asset. The halo and the mark share one geometry, which keeps
/// them aligned at every size.
struct LogoMark: View {
    /// Ink of the mark: dark on the page wash, white on the hero block.
    var color: Color = Theme.dark
    /// Draw the white halo; on for cover art and the page wash, off on solid fills.
    var halo = false

    /// The artwork's coordinate space, straight from the SVG's `viewBox`.
    private static let art = CGSize(width: 92.09, height: 61.75)
    private static let haloWidth: CGFloat = 9
    private static let strokeWidth: CGFloat = 4

    var body: some View {
        Canvas { context, size in
            guard size.width > 0, size.height > 0 else { return }
            // Fit the artwork, centred, so callers can size it by width alone.
            let scale = min(size.width / Self.art.width, size.height / Self.art.height)
            context.translateBy(
                x: (size.width - Self.art.width * scale) / 2,
                y: (size.height - Self.art.height * scale) / 2
            )
            context.scaleBy(x: scale, y: scale)

            if halo {
                context.stroke(
                    Self.path,
                    with: .color(.white),
                    style: StrokeStyle(lineWidth: Self.haloWidth, lineCap: .round, lineJoin: .round)
                )
            }
            context.stroke(
                Self.path,
                with: .color(color),
                style: StrokeStyle(lineWidth: Self.strokeWidth, lineCap: .round, lineJoin: .round)
            )
        }
        .aspectRatio(Self.art.width / Self.art.height, contentMode: .fit)
        .accessibilityHidden(true)
    }

    /// The three paths, in the SVG's coordinates.
    private static let path: Path = {
        var path = Path()
        // The curled scrawl: down through a loop, then the two rising strokes.
        path.move(to: p(27.7, 24.17))
        path.addCurve(to: p(20.78, 20.68), control1: p(27.14, 23.49), control2: p(24.56, 20.4))
        path.addCurve(to: p(11.61, 31.43), control1: p(13.53, 21.23), control2: p(11.56, 26.89))
        path.addCurve(to: p(22.69, 41.04), control1: p(11.68, 36.69), control2: p(17.26, 41.01))
        path.addCurve(to: p(54.18, 14.41), control1: p(47.56, 41.19), control2: p(54.18, 14.41))
        path.addLine(to: p(54.63, 40.7))
        path.addCurve(to: p(71.09, 14.21), control1: p(57.34, 41.56), control2: p(71.09, 14.21))
        path.addLine(to: p(71.29, 40.69))
        // The tall stem on the right.
        path.move(to: p(87.59, 7.61))
        path.addCurve(to: p(82.78, 4.5), control1: p(87.08, 6.84), control2: p(85.53, 4.53))
        path.addCurve(to: p(77.51, 9.31), control1: p(80.2, 4.47), control2: p(77.51, 7.32))
        path.addLine(to: p(77.39, 57.25))
        // The underline.
        path.move(to: p(4.5, 47.39))
        path.addLine(to: p(87.55, 47.39))
        return path
    }()

    private static func p(_ x: CGFloat, _ y: CGFloat) -> CGPoint { CGPoint(x: x, y: y) }
}
