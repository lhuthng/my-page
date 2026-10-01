import AppKit
import SwiftUI

/// Screen 2: the player — one card on the page wash, with the transport row and
/// scrubber above an always-open chapter list that scrolls inside.
struct PlayerScreen: View {
    @EnvironmentObject private var player: PlayerModel
    /// The strip's paging state; the chapter list hands it its scroll view.
    @EnvironmentObject private var pager: PagerModel
    let onBack: () -> Void

    static let rates: [Double] = [0.75, 1, 1.25, 1.5, 1.75, 2]

    /// Where the current chapter's own position is measured from. Named on the
    /// list's *content* rather than on the scroll view, so the value reported is
    /// the row's position in the book and does not move as the list scrolls —
    /// which is what lets it survive the row being recycled off-screen.
    private static let listContent = "chapterListContent"

    /// The playing chapter's position in the list, in content coordinates.
    @State private var rowOrigin: ChapterRowOrigin?
    /// How far the list has scrolled, and how tall its viewport is. Together
    /// these are the visible slice of content, which is what the origin above is
    /// tested against.
    @State private var scrollOffset: CGFloat = 0
    @State private var viewportHeight: CGFloat = 0

    var body: some View {
        ZStack {
            Theme.page.ignoresSafeArea()

            switch player.phase {
            case .empty:
                emptyPrompt
            case .loading(let title):
                // The card's own shape, with a spinner where the artwork lands.
                PlayerSkeleton(title: title)
                    .padding(10)
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
                    .transition(.opacity)
            case .failed(let message):
                VStack(spacing: 10) {
                    Image(systemName: "exclamationmark.triangle")
                        .font(.system(size: 26))
                        .foregroundStyle(Theme.dark.opacity(0.6))
                    Text(message)
                        .font(.callout)
                        .foregroundStyle(Theme.dark.opacity(0.7))
                        .multilineTextAlignment(.center)
                    PillButton(title: "Try again") {
                        Task { await player.retry() }
                    }
                }
                .padding(30)
            case .ready:
                if let book = player.book {
                    playerCard(book)
                        .padding(10)
                        .frame(maxWidth: .infinity, maxHeight: .infinity)
                        // The finished card pops in as the details land.
                        .transition(.scale(scale: 0.94).combined(with: .opacity))
                }
            }

            if let prompt = player.resumePrompt {
                resumeQuestion(prompt)
                    .transition(.opacity)
            }
        }
        // Keyed to the phase, so only opening a book animates.
        .animation(.spring(response: 0.42, dampingFraction: 0.8), value: player.phase)
        .animation(.easeInOut(duration: 0.2), value: player.resumePrompt)
        .onExitCommand {
            // Esc closes the question first: it is the thing in the way.
            if player.resumePrompt != nil {
                player.dismissResumePrompt()
            } else {
                onBack()
            }
        }
    }

    // MARK: - The saved position

    /// The question a saved position asks; playback starts only if answered.
    private func resumeQuestion(_ prompt: PlayerModel.ResumePrompt) -> some View {
        ZStack {
            // A wash behind, so a click anywhere else drops the question.
            Color.black.opacity(0.12)
                .ignoresSafeArea()
                .contentShape(Rectangle())
                .onTapGesture { player.dismissResumePrompt() }

            VStack(spacing: 12) {
                VStack(spacing: 3) {
                    Text("Pick up where you left off?")
                        .font(.system(size: 14, weight: .semibold))
                        .foregroundStyle(Theme.dark)
                    Text("Chapter \(prompt.chapter) · \(clock(prompt.time))")
                        .font(.system(size: 12).monospacedDigit())
                        .foregroundStyle(Theme.dark.opacity(0.7))
                }

                HStack(spacing: 8) {
                    Button {
                        player.resumeFromSavedPoint()
                    } label: {
                        answer("Resume at \(clock(prompt.time))", filled: true)
                    }
                    Button {
                        player.restartFromBeginning()
                    } label: {
                        answer("Start over", filled: false)
                    }
                }
                .buttonStyle(.plain)
            }
            .padding(14)
            .frame(maxWidth: 300)
            .background(Color.white)
            .clipShape(RoundedRectangle(cornerRadius: 14))
            .overlay(
                RoundedRectangle(cornerRadius: 14)
                    .strokeBorder(Theme.cardBorder, lineWidth: 3)
            )
            .shadow(color: .black.opacity(0.24), radius: 14, y: 6)
            .padding(20)
        }
    }

    /// One answer to that question, as a capsule.
    private func answer(_ title: String, filled: Bool) -> some View {
        Text(title)
            .font(.system(size: 12, weight: .semibold))
            .foregroundStyle(filled ? Color.white : Theme.dark)
            .lineLimit(1)
            .frame(maxWidth: .infinity)
            .padding(.vertical, 8)
            .background(Capsule().fill(filled ? Theme.dark : Color.white))
            .overlay(Capsule().strokeBorder(filled ? Color.clear : Theme.dark.opacity(0.35)))
    }

    private var emptyPrompt: some View {
        VStack(spacing: 10) {
            Image(systemName: "book.closed")
                .font(.system(size: 30))
                .foregroundStyle(Theme.dark.opacity(0.45))
            Text("Pick an audiobook from the library")
                .font(.callout)
                .foregroundStyle(Theme.dark.opacity(0.6))
        }
    }

    // MARK: - The mini player card, chapters included

    private func playerCard(_ book: AudiobookDetails) -> some View {
        let track = player.currentTrack

        return VStack(spacing: 0) {
            // Cover + veil are the background of this fixed block: `.background`
            // cannot inflate the layout, so the image never rescales.
            VStack(spacing: 0) {
                HStack(spacing: 10) {
                    VStack(alignment: .leading, spacing: 3) {
                        // One fixed-height line each; long titles drift.
                        MarqueeLine(
                            text: book.title,
                            font: .system(size: 15, weight: .semibold),
                            height: 19
                        )
                        .help(book.title)
                        MarqueeLine(
                            text: track.map { "\($0.number) - \($0.title)" } ?? "",
                            font: .system(size: 12, weight: .medium),
                            color: Theme.dark.opacity(0.9),
                            height: 15
                        )
                        .help(track?.title ?? "")
                    }
                    .frame(maxWidth: .infinity, alignment: .leading)

                    CircleButton(
                        systemImage: "backward.end.fill",
                        size: 36,
                        iconSize: 13,
                        disabled: !player.hasPrevious
                    ) {
                        player.previous()
                    }

                    PlayPauseButton(isPlaying: player.isPlaying, size: 46) {
                        player.togglePlayPause()
                    }

                    CircleButton(
                        systemImage: "forward.end.fill",
                        size: 36,
                        iconSize: 13,
                        disabled: !player.hasNext
                    ) {
                        player.next()
                    }

                    CircleButton(systemImage: "chevron.right", size: 28, iconSize: 11) {
                        onBack()
                    }
                    .help("Back to the library")
                }
                .padding(14)

                HStack(spacing: 12) {
                    CircleButton(systemImage: "gobackward.15", size: 30, iconSize: 13) {
                        player.skip(by: -15)
                    }
                    rateMenu
                    CircleButton(systemImage: "goforward.30", size: 30, iconSize: 13) {
                        player.skip(by: 30)
                    }
                }
                .frame(maxWidth: .infinity)

                // Progress sits with the transport controls, clear of the card's
                // rounded corner, with elapsed / total flanking it.
                HStack(spacing: 8) {
                    Text(clock(player.displayTime))
                        .frame(width: 60, alignment: .leading)
                    Scrubber(
                        value: player.displayTime,
                        duration: player.duration,
                        barHeight: 10,
                        onPreview: { player.scrubPreview(to: $0) },
                        onCommit: { _ in player.scrubCommit() }
                    )
                    Text(clock(player.duration))
                        .frame(width: 60, alignment: .trailing)
                }
                .font(.system(size: 12).monospacedDigit())
                .foregroundStyle(Theme.dark.opacity(0.7))
                .padding(.horizontal, 14)
                .padding(.top, 10)
                .padding(.bottom, 10)

                chaptersHeader(book)
            }
            .background(BackgroundCover(url: book.coverURL))
            .clipped()

            // Chapter list: always open, scrolling inside a clipped viewport that
            // stops short of the card's bottom, so rows never run past the corner.
            ScrollViewReader { listProxy in
                ScrollView(.vertical, showsIndicators: false) {
                    // Lazy, because the book is the unit here and a long one has
                    // hundreds of chapters: only the slots near the viewport are
                    // built, and building one is what asks for its window.
                    LazyVStack(spacing: 0) {
                        ForEach(0..<chapterCount, id: \.self) { index in
                            chapterSlot(index: index, isLast: index == chapterCount - 1)
                                .id(index)
                                // Only the playing chapter is measured; the others
                                // have nothing to report.
                                .background {
                                    if index == player.trackIndex {
                                        GeometryReader { geo in
                                            Color.clear.preference(
                                                key: ChapterRowOriginKey.self,
                                                value: ChapterRowOrigin(
                                                    index: index,
                                                    top: geo.frame(in: .named(Self.listContent))
                                                        .minY,
                                                    height: geo.size.height
                                                )
                                            )
                                        }
                                    }
                                }
                                .onAppear { player.windowNeeded(at: index) }
                        }
                    }
                    // A little inset so a row's highlight never runs into the
                    // card's stroke, and an incomplete last row has room to be
                    // cut off cleanly.
                    .padding(.horizontal, 4)
                    .padding(.bottom, 8)
                    .coordinateSpace(name: Self.listContent)
                    .onPreferenceChange(ChapterRowOriginKey.self) { origin in
                        // Preferences are rebuilt from scratch every layout pass,
                        // so a row the lazy stack has recycled stops contributing
                        // and the collected value falls back to `nil`. Keeping
                        // the last real reading is the whole point: it is what
                        // still answers "which edge did it leave through" after
                        // the row itself is gone.
                        if let origin { rowOrigin = origin }
                    }
                    // Hand the pager this list's scroll view, so it recognises a
                    // press here by identity rather than by frame or hit test.
                    .background(ScrollViewProbe { pager.attachList($0) })
                }
                .onScrollGeometryChange(for: ListMetrics.self) { geometry in
                    // Built from the offset and the container rather than from
                    // `visibleRect`, because those two are unambiguous: the
                    // visible slice of content runs from the offset for one
                    // viewport's worth.
                    ListMetrics(
                        offset: geometry.contentOffset.y,
                        viewport: geometry.containerSize.height
                    )
                } action: { _, metrics in
                    scrollOffset = metrics.offset
                    viewportHeight = metrics.viewport
                }
                .frame(maxHeight: .infinity)
                .clipped()
                .background(Color.white)
                .modifier(ScrollEdgeFades())
                // The bar floats over the list rather than taking a row of its
                // own, so its appearing and disappearing cannot reflow the
                // chapters the reader is looking at.
                .overlay(alignment: pinnedEdge == .top ? .top : .bottom) {
                    if let edge = pinnedEdge, let track = player.currentTrack {
                        pinnedChapterBar(track, edge: edge) {
                            withAnimation(.easeInOut(duration: 0.25)) {
                                listProxy.scrollTo(player.trackIndex, anchor: .center)
                            }
                        }
                        .transition(.opacity)
                    }
                }
                .animation(.easeInOut(duration: 0.18), value: pinnedEdge)
                .padding(.bottom, 8)
                // A different book is different content, so an old reading's
                // position means nothing in it. A different *chapter* is not:
                // that is precisely what `pinnedEdge` carries forward.
                .onChange(of: player.book?.id) { _, _ in rowOrigin = nil }
            }
        }
        // The card is white end to end, so the list's inset and fade land on it.
        .background(Color.white)
        .compositingGroup()
        .clipShape(RoundedRectangle(cornerRadius: 14))
        .overlay(RoundedRectangle(cornerRadius: 14).strokeBorder(Theme.cardBorder, lineWidth: 3))
        .shadow(color: .black.opacity(0.2), radius: 12, y: 5)
    }


    /// Static label row between the artwork and the chapter list.
    private func chaptersHeader(_ book: AudiobookDetails) -> some View {
        HStack(spacing: 6) {
            Text("Chapters")
                .font(.footnote.weight(.semibold))
                .foregroundStyle(Theme.dark)
            Text("\(chapterCount)")
                .font(.caption)
                .monospacedDigit()
                .foregroundStyle(Theme.dark.opacity(0.65))
            Spacer()
        }
        .padding(.horizontal, 14)
        .padding(.vertical, 8)
    }

    /// Chapters in the loaded book, whether or not their windows have arrived.
    private var chapterCount: Int {
        player.chapters?.total ?? player.book?.totalTracks ?? 0
    }

    // MARK: - Keeping the playing chapter in sight

    /// Which edge of the list the playing chapter has scrolled off through, or
    /// `nil` while it is on screen where it belongs.
    ///
    /// A reader browses a long book by scrolling away from what is playing, and
    /// a row that simply left the viewport said nothing about the audio still
    /// running. Holding it at the edge it left through gives that back without
    /// taking the reader's own scrolling away from them.
    private var pinnedEdge: VerticalEdge? {
        guard let origin = rowOrigin else { return nil }
        return Self.pinnedEdge(
            measuredIndex: origin.index,
            measuredTop: origin.top,
            rowHeight: origin.height,
            playingIndex: player.trackIndex,
            offset: scrollOffset,
            viewport: viewportHeight
        )
    }

    /// Which edge the playing chapter has left the list through, or `nil` while
    /// it is on screen.
    ///
    /// Pure and static so it can be tested without a window: an edge that is off
    /// by a row shows the bar while the chapter is still visible, or hides it
    /// while the chapter is off screen, and neither needs a running app to catch.
    ///
    /// `measuredIndex` may differ from `playingIndex`: the playhead moves on
    /// while the reader is scrolled away, and the new row — being off screen —
    /// never renders and so never reports. Carrying the reading forward is
    /// arithmetic rather than a guess, because the rows are a fixed height, and
    /// the height used is a measured one rather than a constant.
    static func pinnedEdge(
        measuredIndex: Int,
        measuredTop: CGFloat,
        rowHeight: CGFloat,
        playingIndex: Int,
        offset: CGFloat,
        viewport: CGFloat
    ) -> VerticalEdge? {
        guard viewport > 0, rowHeight > 0 else { return nil }

        let steps = CGFloat(playingIndex - measuredIndex)
        let top = measuredTop + steps * rowHeight
        let bottom = top + rowHeight

        // A point of tolerance at each boundary: at exact equality the row is
        // still a hair on screen, and pinning it there would make the bar
        // flicker against the row it stands in for.
        if bottom <= offset + 1 { return .top }
        if top >= offset + viewport - 1 { return .bottom }
        return nil
    }

    /// The playing chapter, held at the edge of the list it scrolled past.
    ///
    /// Deliberately built from the app's own controls — the same flat circular
    /// transport button as the mini player, a hairline rule rather than the
    /// web's raised `duo-btn` — so the bar reads as part of this app instead of
    /// as a piece of the site dropped into it.
    private func pinnedChapterBar(
        _ track: AudiobookTrack,
        edge: VerticalEdge,
        onReturn: @escaping () -> Void
    ) -> some View {
        let ratio = player.duration > 0 ? min(1, max(0, player.displayTime / player.duration)) : 0

        return VStack(spacing: 0) {
            HStack(spacing: 8) {
                // The same tap-to-return target as the row itself: the whole
                // label area takes the reader back to the playhead.
                Button(action: onReturn) {
                    HStack(spacing: 8) {
                        Image(systemName: player.isPlaying ? "speaker.wave.2.fill" : "pause.fill")
                            .font(.system(size: 9, weight: .bold))
                            .foregroundStyle(Theme.primary)
                            .frame(width: 12)
                        Text("\(track.number) - \(track.title)")
                            .font(.system(size: 12, weight: .semibold))
                            .foregroundStyle(Theme.dark)
                            .lineLimit(1)
                            .truncationMode(.middle)
                        Spacer(minLength: 6)
                        Text(clock(player.displayTime))
                            .font(.caption.monospacedDigit())
                            .foregroundStyle(Theme.dark.opacity(0.55))
                    }
                    .contentShape(Rectangle())
                }
                .buttonStyle(.plain)
                .help("Show the chapter that is playing")

                PlayPauseButton(isPlaying: player.isPlaying, size: 26) {
                    player.togglePlayPause()
                }
            }
            .padding(.horizontal, 10)
            .padding(.vertical, 6)

            // How far into the chapter the listener is, so the bar reports
            // position and not only identity.
            ProgressBar(fraction: ratio, height: 3)
                .padding(.horizontal, 10)
                .padding(.bottom, 6)
        }
        .background(Color.white)
        // The rule sits on the face the list is scrolling towards, so the bar
        // reads as an edge of the viewport rather than a floating panel.
        .overlay(alignment: edge == .top ? .bottom : .top) {
            Rectangle()
                .fill(Theme.dark.opacity(0.15))
                .frame(height: 1)
        }
        .shadow(
            color: .black.opacity(0.12),
            radius: 5,
            y: edge == .top ? 2 : -2
        )
    }

    /// One slot of the chapter list: the chapter when its window has arrived,
    /// and the row's own shape in grey blocks while it is on its way.
    @ViewBuilder
    private func chapterSlot(index: Int, isLast: Bool) -> some View {
        if let track = player.chapters?.track(at: index) {
            chapterRow(index: index, track: track, isLast: isLast)
        } else {
            chapterSkeleton(index: index, isLast: isLast)
        }
    }

    /// What a chapter row looks like before the chapter does: the same layout in
    /// blocks, so a window that is still arriving keeps the list's rhythm and
    /// the numbers stay lined up with the rows around it.
    private func chapterSkeleton(index: Int, isLast: Bool) -> some View {
        HStack(spacing: 8) {
            Rectangle()
                .fill(Color.clear)
                .frame(width: 2)
            SkeletonBar(width: 18, height: 9)
            SkeletonBar(width: index.isMultiple(of: 2) ? 132 : 168, height: 9)
            Spacer(minLength: 0)
            SkeletonBar(width: 26, height: 9)
        }
        .padding(.trailing, 10)
        .padding(.vertical, 7)
        .overlay(alignment: .bottom) {
            if !isLast {
                Rectangle()
                    .fill(Theme.dark.opacity(0.1))
                    .frame(height: 1)
            }
        }
        .allowsHitTesting(false)
        .accessibilityLabel("Loading chapters")
    }

    /// One chapter: a tinted row with a primary rule while playing, and a
    /// hairline between rows.
    private func chapterRow(index: Int, track: AudiobookTrack, isLast: Bool) -> some View {
        let isCurrent = index == player.trackIndex

        return HStack(spacing: 8) {
            // Always present, clear when idle, so every row's text lines up.
            Rectangle()
                .fill(isCurrent ? Theme.primary : Color.clear)
                .frame(width: 2)
            // Room for three digits, right-aligned, so the figures line up.
            Text("\(track.number)")
                .font(.caption.weight(.semibold))
                .monospacedDigit()
                .foregroundStyle(isCurrent ? Theme.primary : Theme.dark.opacity(0.5))
                .lineLimit(1)
                .fixedSize(horizontal: true, vertical: false)
                .frame(width: 28, alignment: .trailing)
            if isCurrent {
                Image(systemName: "speaker.wave.2.fill")
                    .font(.system(size: 8, weight: .bold))
                    .foregroundStyle(Theme.primary)
            }
            Text(track.title)
                .font(.system(size: 12, weight: isCurrent ? .semibold : .regular))
                .foregroundStyle(isCurrent ? Theme.primary : Theme.dark)
                .lineLimit(1)
            Spacer()
            // Play count, matching the web playlist's icon + figure. Hidden
            // entirely on older backends that do not report the field.
            if let playCount = track.playCount {
                HStack(spacing: 2) {
                    Image(systemName: "play.fill")
                        .font(.system(size: 7, weight: .bold))
                    Text("\(playCount)")
                        .font(.caption.monospacedDigit())
                }
                .foregroundStyle(Theme.dark.opacity(0.55))
            }
            if let seconds = track.durationSeconds {
                Text(clock(Double(seconds)))
                    .font(.caption.monospacedDigit())
                    .foregroundStyle(Theme.dark.opacity(0.55))
            }
        }
        .padding(.trailing, 10)
        .padding(.vertical, 7)
        .background(isCurrent ? Theme.primary.opacity(0.1) : Color.clear)
        .overlay(alignment: .bottom) {
            if !isLast {
                Rectangle()
                    .fill(Theme.dark.opacity(0.1))
                    .frame(height: 1)
            }
        }
        .contentShape(Rectangle())
        // A tap, not a Button or a drag gesture: a tap hands the press back as
        // soon as the pointer moves, so a drag still reaches the scroll view.
        .onTapGesture {
            guard !pager.lastPressWasDrag else { return }
            player.playTrack(at: index)
        }
    }

    private var rateMenu: some View {
        Menu {
            ForEach(Self.rates, id: \.self) { value in
                Button(value == 1 ? "Normal" : String(format: "%g×", value)) {
                    player.rate = value
                }
            }
        } label: {
            Text(String(format: "%g×", player.rate))
                .font(.footnote.weight(.semibold))
                .monospacedDigit()
                .foregroundStyle(Theme.dark)
                .padding(.horizontal, 12)
                .padding(.vertical, 4)
                .background(Capsule().fill(Theme.dark.opacity(0.1)))
        }
        .menuStyle(.borderlessButton)
        .menuIndicator(.hidden)
        .focusable(false)
        .fixedSize()
    }
}

/// The player page while a book opens: the same card in blocks, so the page
/// never flashes empty and the layout does not jump.
private struct PlayerSkeleton: View {
    let title: String

    /// Deterministic widths: stable across redraws (random ones would flicker).
    private let rowWidths: [CGFloat] = [150, 118, 166, 132, 158, 124, 146, 170, 136]

    var body: some View {
        VStack(spacing: 0) {
            VStack(spacing: 0) {
                HStack(spacing: 10) {
                    VStack(alignment: .leading, spacing: 6) {
                        SkeletonBar(width: 152, height: 13)
                        SkeletonBar(width: 110, height: 10)
                    }
                    Spacer(minLength: 0)
                    SkeletonDot(size: 36)
                    SkeletonDot(size: 46)
                    SkeletonDot(size: 36)
                    SkeletonDot(size: 28)
                }
                .padding(14)

                HStack(spacing: 12) {
                    SkeletonDot(size: 30)
                    SkeletonBar(width: 46, height: 16)
                    SkeletonDot(size: 30)
                }
                .frame(maxWidth: .infinity)

                HStack(spacing: 8) {
                    SkeletonBar(width: 60, height: 9)
                    SkeletonBar(height: 10)
                    SkeletonBar(width: 60, height: 9)
                }
                .padding(.horizontal, 14)
                .padding(.top, 10)
                .padding(.bottom, 10)

                HStack(spacing: 6) {
                    SkeletonBar(width: 92, height: 10)
                    Spacer(minLength: 0)
                }
                .padding(.horizontal, 14)
                .padding(.vertical, 8)
            }
            // The spinner sits where the artwork will be.
            .overlay {
                ProgressView()
                    .controlSize(.large)
                    .padding(16)
                    .background(Circle().fill(.white.opacity(0.92)))
                    .shadow(color: .black.opacity(0.12), radius: 6, y: 2)
            }

            ScrollView(.vertical, showsIndicators: false) {
                VStack(spacing: 0) {
                    ForEach(Array(rowWidths.enumerated()), id: \.offset) { index, width in
                        HStack(spacing: 8) {
                            Rectangle().fill(Color.clear).frame(width: 2)
                            SkeletonBar(width: 22, height: 8)
                            SkeletonBar(width: width, height: 9)
                            Spacer(minLength: 0)
                            SkeletonBar(width: 30, height: 8)
                        }
                        .padding(.trailing, 10)
                        .padding(.vertical, 9)
                        .overlay(alignment: .bottom) {
                            if index < rowWidths.count - 1 {
                                Rectangle()
                                    .fill(Theme.dark.opacity(0.06))
                                    .frame(height: 1)
                            }
                        }
                    }
                }
                .padding(.horizontal, 4)
                .padding(.bottom, 8)
            }
            .frame(maxHeight: .infinity)
            .clipped()
            .background(Color.white)
            .modifier(ScrollEdgeFades())
            .padding(.bottom, 8)
        }
        .background(Color.white)
        .clipShape(RoundedRectangle(cornerRadius: 14))
        .overlay(RoundedRectangle(cornerRadius: 14).strokeBorder(Theme.cardBorder, lineWidth: 3))
        .shadow(color: .black.opacity(0.2), radius: 12, y: 5)
        .accessibilityElement()
        .accessibilityLabel("Opening \(title)")
    }
}

/// Hands the pager the `NSScrollView` hosting the `ScrollView` this sits in —
/// walking up the superview chain, since the frame spaces cannot be compared.
private struct ScrollViewProbe: NSViewRepresentable {
    let onFound: (NSScrollView) -> Void

    func makeNSView(context: Context) -> NSView {
        let view = NSView(frame: .zero)
        report(from: view, attempts: 0)
        return view
    }

    func updateNSView(_ nsView: NSView, context: Context) {
        report(from: nsView, attempts: 0)
    }

    /// The scroll view only enters the chain after a layout pass, so retry briefly.
    private func report(from view: NSView, attempts: Int) {
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.05 * Double(attempts)) {
            var candidate: NSView? = view.superview
            while let node = candidate {
                if let scrollView = node as? NSScrollView {
                    onFound(scrollView)
                    return
                }
                candidate = node.superview
            }
            if attempts < 20 { report(from: view, attempts: attempts + 1) }
        }
    }
}

/// Where the playing chapter sits in the list, in content coordinates.
///
/// `top` is measured in the list's *content* space, so it does not move as the
/// list scrolls. That is what lets the value outlive the row: a chapter scrolled
/// far enough away is recycled by the lazy stack and stops reporting, and the
/// last reading — held in content space — is still a valid answer for which edge
/// it left through.
private struct ChapterRowOrigin: Equatable {
    /// Which chapter this reading is about, so a carried-forward position knows
    /// how far it has to travel.
    var index: Int
    var top: CGFloat
    var height: CGFloat
}

private struct ChapterRowOriginKey: PreferenceKey {
    static var defaultValue: ChapterRowOrigin?
    static func reduce(value: inout ChapterRowOrigin?, nextValue: () -> ChapterRowOrigin?) {
        value = nextValue() ?? value
    }
}

/// The list's scroll position and the height of its viewport.
private struct ListMetrics: Equatable {
    var offset: CGFloat
    var viewport: CGFloat
}

/// A faint white wash at both ends of a scroll viewport, so rows dissolve into
/// the card instead of being cut. Purely visual.
private struct ScrollEdgeFades: ViewModifier {
    var height: CGFloat = 8

    func body(content: Content) -> some View {
        content
            .overlay(alignment: .top) {
                LinearGradient(
                    colors: [.white, .white.opacity(0)],
                    startPoint: .top,
                    endPoint: .bottom
                )
                .frame(height: height)
                .allowsHitTesting(false)
            }
            .overlay(alignment: .bottom) {
                LinearGradient(
                    colors: [.white.opacity(0), .white],
                    startPoint: .top,
                    endPoint: .bottom
                )
                .frame(height: height)
                .allowsHitTesting(false)
            }
    }
}

/// A placeholder bar or dot for the skeleton, in the card's own greys.
private struct SkeletonBar: View {
    var width: CGFloat?
    var height: CGFloat = 10

    var body: some View {
        RoundedRectangle(cornerRadius: height / 2)
            .fill(Theme.dark.opacity(0.1))
            .frame(width: width, height: height)
            .frame(maxWidth: .infinity, alignment: .leading)
    }
}

private struct SkeletonDot: View {
    let size: CGFloat

    var body: some View {
        Circle()
            .fill(Theme.dark.opacity(0.1))
            .frame(width: size, height: size)
    }
}

/// The cover full-bleed under a white veil that thickens towards the bottom, so
/// the artwork dissolves into the chapter list instead of stopping at a seam.
private struct BackgroundCover: View {
    let url: URL?
    @State private var image: NSImage?

    var body: some View {
        ZStack {
            Rectangle().fill(.white)
            if let image {
                Image(nsImage: image)
                    .resizable()
                    .scaledToFill()
            }
            LinearGradient(
                stops: [
                    .init(color: .white.opacity(0.8), location: 0),
                    .init(color: .white.opacity(0.8), location: 0.4),
                    .init(color: .white, location: 1),
                ],
                startPoint: .top,
                endPoint: .bottom
            )
        }
        .clipped()
        .task(id: url) {
            image = await ImageCache.shared.image(at: url)
        }
    }
}
