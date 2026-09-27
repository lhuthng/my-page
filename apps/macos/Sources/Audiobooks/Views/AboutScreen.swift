import SwiftUI

/// Screen 3: about the app — the mark, what it is, the build, and links out.
/// Static information, so no player follows the audio here.
struct AboutScreen: View {
    /// Back to the library; also Esc.
    let onBack: () -> Void

    var body: some View {
        VStack(spacing: 0) {
            titleBar
            Divider().overlay(Theme.dark.opacity(0.15))

            ScrollView(.vertical, showsIndicators: false) {
                AboutCard()
                    .padding(12)
            }
        }
        .background(Theme.page)
        .onExitCommand { onBack() }
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
