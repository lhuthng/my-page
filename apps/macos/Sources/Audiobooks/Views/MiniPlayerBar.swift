import SwiftUI

/// The web mini player: one white card with the scrubber as its top rule, the
/// book title and chapter line, transport controls, and a way into the player.
struct MiniPlayerBar: View {
    @EnvironmentObject private var player: PlayerModel
    let onOpenPlayer: () -> Void

    var body: some View {
        Group {
            if let book = player.book {
                card(book: book)
                    .padding(.horizontal, 12)
                    .padding(.vertical, 10)
            }
        }
        .animation(.snappy(duration: 0.25), value: player.book?.id)
    }

    private func card(book: AudiobookDetails) -> some View {
        let track = player.currentTrack

        return VStack(spacing: 0) {
            // The timeline as the card's top rule, inset clear of the corner.
            Scrubber(
                value: player.displayTime,
                duration: player.duration,
                barHeight: 8,
                onPreview: { player.scrubPreview(to: $0) },
                onCommit: { _ in player.scrubCommit() }
            )
            .padding(.horizontal, 12)
            .padding(.top, 6)

            HStack(spacing: 10) {
                infoBlock(book: book, track: track)

                CircleButton(
                    systemImage: "backward.end.fill",
                    size: 34,
                    iconSize: 13,
                    disabled: !player.hasPrevious
                ) {
                    player.previous()
                }

                PlayPauseButton(isPlaying: player.isPlaying, size: 40) {
                    player.togglePlayPause()
                }

                CircleButton(
                    systemImage: "forward.end.fill",
                    size: 34,
                    iconSize: 13,
                    disabled: !player.hasNext
                ) {
                    player.next()
                }

                CircleButton(systemImage: "chevron.left", size: 28, iconSize: 11) {
                    onOpenPlayer()
                }
                .help("Open the player")
            }
            .padding(10)
        }
        .background(Color.white)
        .clipShape(RoundedRectangle(cornerRadius: 12))
        .overlay(RoundedRectangle(cornerRadius: 12).strokeBorder(Theme.cardBorder, lineWidth: 3))
        .shadow(color: .black.opacity(0.2), radius: 10, y: 4)
        .contentShape(RoundedRectangle(cornerRadius: 12))
        .onTapGesture {
            onOpenPlayer()
        }
    }

    private func infoBlock(book: AudiobookDetails, track: AudiobookTrack?) -> some View {
        VStack(alignment: .leading, spacing: 2) {
            // Title and chapter line drift when they do not fit.
            MarqueeLine(
                text: book.title,
                font: .system(size: 13, weight: .semibold),
                height: 16
            )
            .help(book.title)
            MarqueeLine(
                text: track.map { "\($0.number) - \($0.title)" } ?? "",
                font: .system(size: 11),
                color: Theme.dark.opacity(0.8),
                height: 14
            )
            .help(track?.title ?? "")
            Text("\(clock(player.displayTime)) / \(clock(player.duration))")
                .font(.system(size: 11).monospacedDigit())
                .foregroundStyle(Theme.dark.opacity(0.5))
        }
        .frame(maxWidth: .infinity, alignment: .leading)
    }
}
