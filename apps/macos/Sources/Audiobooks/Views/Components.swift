import SwiftUI

// Shared pieces styled after the web audiobook components.

/// The pointer-driven timeline: a thin track, the played fill, and a white
/// playhead ringed in primary. Previews while dragging, commits on release, and
/// the playhead is inscribed in the bar (diameter = height, centre on the fill's
/// cap), so fill tip and playhead are one circle.
struct Scrubber: View {
    let value: Double
    let duration: Double
    var barHeight: CGFloat = 8
    /// Thickness of the playhead's ring, matching the web's `border-2` thumb.
    var ringWidth: CGFloat?
    var onPreview: (Double) -> Void
    var onCommit: (Double) -> Void

    @State private var preview: Double?

    var body: some View {
        GeometryReader { geo in
            let width = geo.size.width
            let safeDuration = max(duration, 0.001)
            let shown = preview ?? value
            let ratio = min(1, max(0, shown / safeDuration))
            let knob = barHeight
            let ring = ringWidth ?? 2
            // The fill runs one knob-radius past the centre, so its leading cap is
            // exactly the playhead.
            let filled = knob + max(width - knob, 0) * ratio

            ZStack(alignment: .leading) {
                Capsule()
                    .fill(Theme.dark.opacity(0.15))
                // Translucent, as on the web (`bg-primary/60`).
                Capsule()
                    .fill(Theme.primary.opacity(0.6))
                    .frame(width: filled)
                // Playhead: one disc with a ring — the web's white-bordered thumb.
                Circle()
                    .fill(Theme.primary)
                    .frame(width: knob, height: knob)
                    .overlay(Circle().fill(.white).padding(ring))
                    .offset(x: filled - knob)
            }
            .frame(height: barHeight)
            .clipShape(Capsule())
            .frame(maxHeight: .infinity, alignment: .center)
            .contentShape(Rectangle())
            .gesture(
                DragGesture(minimumDistance: 0)
                    .onChanged { gesture in
                        guard width > 0 else { return }
                        // Pointer maps across the whole track (web parity).
                        let r = min(1, max(0, gesture.location.x / width))
                        preview = r * duration
                        onPreview(preview ?? 0)
                    }
                    .onEnded { _ in
                        if let target = preview { onCommit(target) }
                        preview = nil
                    }
            )
        }
        .frame(height: 16)
    }
}

/// One line that drifts left and right when it is too long for its container,
/// and sits still when it fits — as the web mini player treats its title.
struct MarqueeLine: View {
    let text: String
    var font: Font = .system(size: 13, weight: .semibold)
    var color: Color = Theme.dark
    /// Fixed height: a `GeometryReader` is greedy, so the caller hands it in.
    var height: CGFloat = 16

    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @State private var textWidth: CGFloat = 0

    var body: some View {
        GeometryReader { geo in
            let available = geo.size.width
            let overflow = max(0, textWidth - available)

            // Read off the clock, not driven by an animation: the players redraw
            // twice a second, which cancels a `repeatForever` and left these lines
            // still. No minimum interval, so it advances every refresh.
            TimelineView(
                .animation(minimumInterval: nil, paused: reduceMotion || overflow <= 1)
            ) { timeline in
                Text(text)
                    .font(font)
                    .foregroundStyle(color)
                    .lineLimit(1)
                    .fixedSize()
                    .background(
                        GeometryReader { textGeo in
                            Color.clear.preference(
                                key: MarqueeTextWidth.self,
                                value: textGeo.size.width
                            )
                        }
                    )
                    // Offset shifts the drawing only; the frame below still
                    // measures the text at its natural width and clips the slide.
                    .offset(x: -Self.offset(overflow: overflow, at: timeline.date))
                    .frame(width: available, alignment: .leading)
                    .clipped()
            }
            .frame(height: height)
        }
        .frame(height: height)
        .onPreferenceChange(MarqueeTextWidth.self) { width in
            guard width > 0, width != textWidth else { return }
            textWidth = width
        }
    }

    /// How far into the overflow the line has travelled at `date`: hold, drift
    /// out, hold, drift back — a pure function of the clock.
    static func offset(overflow: CGFloat, at date: Date) -> CGFloat {
        guard overflow > 1 else { return 0 }
        // Seconds per direction: about 25 points a second, with a floor so a
        // short overflow still moves.
        let travel = max(2.5, Double(6 + overflow / 25) / 2)
        // A beat at each end.
        let hold = 0.9
        let cycle = 2 * (travel + hold)
        let time = date.timeIntervalSinceReferenceDate.truncatingRemainder(dividingBy: cycle)

        let progress: Double
        if time < hold {
            progress = 0
        } else if time < hold + travel {
            progress = ease((time - hold) / travel)
        } else if time < 2 * hold + travel {
            progress = 1
        } else {
            progress = 1 - ease((time - 2 * hold - travel) / travel)
        }
        return overflow * CGFloat(progress)
    }

    /// Smoothstep: softens both ends of the slide without easing machinery.
    private static func ease(_ value: Double) -> Double {
        let clamped = min(1, max(0, value))
        return clamped * clamped * (3 - 2 * clamped)
    }
}

private struct MarqueeTextWidth: PreferenceKey {
    static var defaultValue: CGFloat = 0
    static func reduce(value: inout CGFloat, nextValue: () -> CGFloat) {
        value = max(value, nextValue())
    }
}

/// Cover art with the site's white card + 3px dark border treatment.
struct CoverImage: View {
    let url: URL?
    var cornerRadius: CGFloat = 8
    var bordered = true

    @State private var image: NSImage?

    var body: some View {
        ZStack {
            Rectangle().fill(.white)
            if let image {
                Image(nsImage: image)
                    .resizable()
                    .scaledToFill()
            } else {
                Image(systemName: "book.closed.fill")
                    .font(.system(size: 26, weight: .regular))
                    .foregroundStyle(Theme.primary.opacity(0.45))
            }
        }
        .clipShape(RoundedRectangle(cornerRadius: cornerRadius))
        .overlay(
            RoundedRectangle(cornerRadius: cornerRadius)
                .strokeBorder(bordered ? Theme.cardBorder : Color.clear, lineWidth: 3)
        )
        .task(id: url) {
            image = await ImageCache.shared.image(at: url)
        }
    }
}

/// The mini player's secondary control, dimmed at 30% when disabled.
struct CircleButton: View {
    let systemImage: String
    var size: CGFloat = 34
    var iconSize: CGFloat = 14
    var disabled = false
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            Image(systemName: systemImage)
                .font(.system(size: iconSize, weight: .semibold))
                .foregroundStyle(Theme.dark)
                .frame(width: size, height: size)
                .background(Circle().fill(Theme.dark.opacity(0.1)))
                .opacity(disabled ? 0.3 : 1)
        }
        .buttonStyle(.plain)
        .disabled(disabled)
    }
}

/// The transport button: green for play, red for pause, white glyph either way.
struct PlayPauseButton: View {
    let isPlaying: Bool
    var size: CGFloat = 42
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            Image(systemName: isPlaying ? "pause.fill" : "play.fill")
                .font(.system(size: size * 0.42, weight: .bold))
                .foregroundStyle(.white)
                .frame(width: size, height: size)
                .background(Circle().fill(isPlaying ? Theme.accentRed : Theme.accentGreen))
        }
        .buttonStyle(.plain)
    }
}

/// Small Vietnam flag chip: 3:2, red field #DA251D, five-point yellow star.
struct VNFlagBadge: View {
    var height: CGFloat = 18

    var body: some View {
        ZStack {
            Rectangle()
                .fill(Color(red: 0xDA / 255, green: 0x25 / 255, blue: 0x1D / 255))
            FivePointStar()
                .fill(Color(red: 1, green: 1, blue: 0))
                .frame(width: height * 0.62, height: height * 0.62)
        }
        .frame(height: height)
        .aspectRatio(3 / 2, contentMode: .fit)
        .clipShape(RoundedRectangle(cornerRadius: 3))
        .overlay(RoundedRectangle(cornerRadius: 3).strokeBorder(.black.opacity(0.1)))
        .shadow(color: .black.opacity(0.15), radius: 1, y: 0.5)
        .help("Bản dịch tiếng Việt")
    }
}

struct FivePointStar: Shape {
    func path(in rect: CGRect) -> Path {
        var path = Path()
        let center = CGPoint(x: rect.midX, y: rect.midY)
        let outer = min(rect.width, rect.height) / 2
        let inner = outer * 0.382
        for index in 0..<10 {
            // CGFloat throughout: cos/sin have CGFloat and Double overloads, so
            // mixing a Double angle with a CGFloat radius leaves the call
            // ambiguous on some SDKs.
            let angle = (CGFloat(index) * .pi / 5) - .pi / 2
            let radius = index % 2 == 0 ? outer : inner
            let point = CGPoint(
                x: center.x + cos(angle) * radius,
                y: center.y + sin(angle) * radius
            )
            if index == 0 {
                path.move(to: point)
            } else {
                path.addLine(to: point)
            }
        }
        path.closeSubpath()
        return path
    }
}
