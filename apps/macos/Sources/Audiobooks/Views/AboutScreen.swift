import SwiftUI

/// Screen 3: about the app — the mark, what it is, the build, and links out.
/// Static information, so no player follows the audio here.
struct AboutScreen: View {
    /// Back to the library; also Esc.
    let onBack: () -> Void

    @EnvironmentObject private var updater: Updater

    var body: some View {
        VStack(spacing: 0) {
            titleBar
            Divider().overlay(Theme.dark.opacity(0.15))

            ScrollView(.vertical, showsIndicators: false) {
                VStack(spacing: 12) {
                    AboutCard()
                    updateRow
                }
                .padding(12)
            }
        }
        .background(Theme.page)
        .onExitCommand { onBack() }
    }

    // MARK: - Updates

    /// The build's version, whether it is current, and the way to change that.
    private var updateRow: some View {
        VStack(spacing: 8) {
            if case .available(let release) = updater.state {
                Text("Version \(release.version) is available")
                    .font(.footnote.weight(.semibold))
                    .foregroundStyle(Theme.dark)
            } else {
                Text(statusLine)
                    .font(.footnote)
                    .foregroundStyle(Theme.dark.opacity(0.7))
            }

            if case .downloading(_, let fraction) = updater.state {
                ProgressBar(fraction: fraction)
                    .frame(maxWidth: 220)
            }

            // Drawn in SwiftUI, not as a system button: this window does not
            // paint AppKit-backed control content, so a `.bordered` title would
            // come out blank. See `PillButton`.
            if case .available(let release) = updater.state {
                PillButton(title: "Install and restart", filled: true) {
                    updater.install(release)
                }
            } else if !updater.state.isBusy {
                PillButton(title: "Check for updates") { updater.checkInteractively() }
            }

            if case .failed(let message) = updater.state {
                Text(message)
                    .font(.caption)
                    .foregroundStyle(Theme.dark.opacity(0.6))
                    .multilineTextAlignment(.center)
            }
        }
        .frame(maxWidth: .infinity)
        .padding(.vertical, 12)
        .padding(.horizontal, 12)
        .background(Color.white)
        .clipShape(RoundedRectangle(cornerRadius: 10))
    }

    private var statusLine: String {
        switch updater.state {
        case .checking: "Checking…"
        case .upToDate: "Audiobooks \(AppInfo.version) is up to date"
        case .downloading(_, let fraction): "Downloading… \(Int(fraction * 100))%"
        case .installing: "Installing…"
        default: "Audiobooks \(AppInfo.version)"
        }
    }

    // MARK: - Header

    /// The white band the other screens use, with the way back on the left — one
    /// line, so the title sits level with the button.
    private var titleBar: some View {
        HStack(spacing: 10) {
            CircleButton(systemImage: "chevron.left", size: 30, iconSize: 12) {
                onBack()
            }
            .help("Back to the library")

            Text("About")
                .font(.title3.weight(.bold))
                .foregroundStyle(Theme.dark)

            Spacer()
        }
        .padding(.horizontal, 12)
        .padding(.vertical, 10)
        .background(Color.white)
    }
}
