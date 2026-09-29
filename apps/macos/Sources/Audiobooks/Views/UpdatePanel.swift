import SwiftUI

/// The update prompt, drawn over whichever page is showing.
///
/// Not a native `.alert`, for two reasons. This window does not paint the
/// content of the AppKit-backed controls it hosts (see README), so an alert's
/// text and buttons are a gamble; and an alert is dismissed the moment its
/// button is tapped, so it can never report the download that button started.
/// This panel stays put from the offer to the relaunch — which is what lets the
/// size be weighed before saying yes, and the progress watched after, even when
/// the update was found by the launch check while the library was on screen.
struct UpdatePanel: View {
    /// The author is done with the panel: "Later" or "OK".
    let onDismiss: () -> Void

    @EnvironmentObject private var updater: Updater

    var body: some View {
        ZStack {
            // Dim the pages and swallow their presses: this is modal. The
            // backdrop deliberately does nothing on tap, so a stray click
            // cannot hide a download that is already running.
            Color.black.opacity(0.22)
                .ignoresSafeArea()
                .contentShape(Rectangle())
                .onTapGesture {}

            card
        }
        // Esc gets out of the offer, but never out of a download in flight.
        .onExitCommand { if !updater.state.isBusy { onDismiss() } }
    }

    private var card: some View {
        VStack(spacing: 10) {
            Text(title)
                .font(.system(size: 15, weight: .bold))
                .foregroundStyle(Theme.dark)

            Text(message)
                .font(.system(size: 12))
                .foregroundStyle(Theme.dark.opacity(0.75))
                .multilineTextAlignment(.center)
                .fixedSize(horizontal: false, vertical: true)

            if let sizeLine {
                Text(sizeLine)
                    .font(.system(size: 11, weight: .semibold).monospacedDigit())
                    .foregroundStyle(Theme.dark.opacity(0.55))
            }

            if let releaseNotes {
                notesBox(releaseNotes)
            }

            if case .downloading(_, let fraction) = updater.state {
                ProgressBar(fraction: fraction)
                    .frame(maxWidth: 220)
            }

            if let detail {
                Text(detail)
                    .font(.system(size: 11, weight: .semibold).monospacedDigit())
                    .foregroundStyle(Theme.dark.opacity(0.6))
                    .multilineTextAlignment(.center)
                    .fixedSize(horizontal: false, vertical: true)
            }

            actions
        }
        .padding(.vertical, 18)
        .padding(.horizontal, 20)
        .frame(maxWidth: 300)
        .background(Color.white)
        .clipShape(RoundedRectangle(cornerRadius: 14))
        .overlay(
            RoundedRectangle(cornerRadius: 14).strokeBorder(Theme.cardBorder, lineWidth: 3)
        )
        .shadow(color: .black.opacity(0.25), radius: 16, y: 6)
        .padding(20)
    }

    // MARK: - Copy

    private var title: String {
        switch updater.state {
        case .available: "Update available"
        case .downloading, .installing: "Updating…"
        case .failed: "Update failed"
        default: "Update"
        }
    }

    /// What is on offer. Stays up through the install, so the download is never a
    /// mystery, and becomes the failure if the install does not land.
    private var message: String {
        guard let release else {
            return "The update could not be installed."
        }
        return "Audiobooks \(release.version) is available. You have \(AppInfo.version)."
    }

    /// How big the download is — the number worth having before saying yes.
    private var sizeLine: String? {
        guard let size = release?.size else { return nil }
        let formatted = ByteCountFormatter.string(fromByteCount: size, countStyle: .file)
        return "\(formatted) to download"
    }

    /// What changed, when the release publishes notes.
    private var releaseNotes: String? {
        guard let notes = release?.notes, !notes.isEmpty else { return nil }
        return notes
    }

    /// A bounded box for the notes.
    ///
    /// Deliberately not a `ScrollView`: a scroll view here would be picked up by
    /// the pager's event router, which drives any scroll view it finds — and a
    /// sideways drag in it could start turning the page behind the panel. Long
    /// notes are cut off by `lineLimit` instead.
    private func notesBox(_ markdown: String) -> some View {
        Text(attributed(markdown))
            .font(.system(size: 11))
            .foregroundStyle(Theme.dark.opacity(0.65))
            .lineLimit(6)
            .fixedSize(horizontal: false, vertical: true)
            .frame(maxWidth: .infinity, alignment: .leading)
            .padding(.horizontal, 8)
            .padding(.vertical, 6)
            .background(RoundedRectangle(cornerRadius: 8).fill(Theme.dark.opacity(0.06)))
    }

    /// GitHub's `body` is Markdown. Inline-only with whitespace preserved: a
    /// bulleted list and a "Full Changelog" link both stay readable at this size
    /// without block styling fighting the small card.
    private func attributed(_ markdown: String) -> AttributedString {
        let options = AttributedString.MarkdownParsingOptions(
            interpretedSyntax: .inlineOnlyPreservingWhitespace
        )
        return (try? AttributedString(markdown: markdown, options: options))
            ?? AttributedString(markdown)
    }

    private var detail: String? {
        switch updater.state {
        case .downloading(_, let fraction): "Downloading… \(Int(fraction * 100))%"
        case .installing: "Installing…"
        case .failed(let message): message
        default: nil
        }
    }

    /// The release being offered, downloaded, or installed.
    private var release: Updater.Release? {
        switch updater.state {
        case .available(let release), .downloading(let release, _), .installing(let release):
            release
        default:
            nil
        }
    }

    // MARK: - Actions

    @ViewBuilder
    private var actions: some View {
        switch updater.state {
        case .available(let release):
            HStack(spacing: 8) {
                PillButton(title: "Install and restart", filled: true) {
                    updater.install(release)
                }
                PillButton(title: "Later") { onDismiss() }
            }
        case .failed:
            PillButton(title: "OK", filled: true) { onDismiss() }
        default:
            // Nothing to press while it is busy: it either relaunches or fails.
            EmptyView()
        }
    }
}
