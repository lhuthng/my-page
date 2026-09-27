import SwiftUI

/// The About screen's card: a branded header block over the app's facts and
/// links, in the same card treatment as the player and the library.
struct AboutCard: View {
    var body: some View {
        VStack(spacing: 0) {
            hero
            details
        }
        .background(Color.white)
        .clipShape(RoundedRectangle(cornerRadius: 14))
        .overlay(RoundedRectangle(cornerRadius: 14).strokeBorder(Theme.cardBorder, lineWidth: 3))
        .shadow(color: .black.opacity(0.2), radius: 12, y: 5)
    }

    /// The mark on the app's primary ramp, with the name and the build.
    private var hero: some View {
        VStack(spacing: 12) {
            LogoMark(color: .white)
                .frame(width: 108)

            VStack(spacing: 7) {
                Text(AppInfo.name)
                    .font(.system(size: 21, weight: .bold))
                    .foregroundStyle(.white)
                versionChip
            }
        }
        .frame(maxWidth: .infinity)
        .padding(.vertical, 18)
        .background(
            LinearGradient(
                colors: [Theme.primary, Theme.dark],
                startPoint: .top,
                endPoint: .bottom
            )
        )
    }

    private var versionChip: some View {
        Text(AppInfo.version)
            .font(.system(size: 11, weight: .semibold).monospacedDigit())
            .foregroundStyle(.white)
            .padding(.horizontal, 10)
            .padding(.vertical, 4)
            .background(Capsule().fill(.white.opacity(0.22)))
            .overlay(Capsule().strokeBorder(.white.opacity(0.35)))
    }

    private var details: some View {
        VStack(spacing: 14) {
            VStack(spacing: 6) {
                Text(AppInfo.summary)
                    .font(.system(size: 13, weight: .medium))
                    .foregroundStyle(Theme.dark)
                    .multilineTextAlignment(.center)
                    .fixedSize(horizontal: false, vertical: true)
                Text(AppInfo.tagline)
                    .font(.system(size: 11))
                    .italic()
                    .foregroundStyle(Theme.dark.opacity(0.65))
                    .multilineTextAlignment(.center)
                    .fixedSize(horizontal: false, vertical: true)
            }

            VStack(spacing: 8) {
                factChip("Written by", AppInfo.author)
                factChip("Site", AppInfo.siteLabel)
            }

            links
        }
        .padding(14)
    }

    /// One fact as a full-width capsule, label and value on one line.
    private func factChip(_ label: String, _ value: String) -> some View {
        HStack(spacing: 6) {
            Text(label)
                .font(.system(size: 10, weight: .semibold))
                .foregroundStyle(Theme.dark.opacity(0.6))
            Text(value)
                .font(.system(size: 12, weight: .semibold))
                .foregroundStyle(Theme.dark)
                .lineLimit(1)
        }
        .frame(maxWidth: .infinity)
        .padding(.vertical, 8)
        .background(Capsule().fill(Theme.dark.opacity(0.07)))
        .overlay(Capsule().strokeBorder(Theme.dark.opacity(0.14)))
    }

    private var links: some View {
        HStack(spacing: 8) {
            Link(destination: AppInfo.siteURL) {
                linkLabel("Visit the blog", icon: "safari", filled: true)
            }
            .help(AppInfo.siteLabel)

            Link(destination: AppInfo.githubURL) {
                linkLabel("GitHub", icon: "chevron.left.forwardslash.chevron.right")
            }
            .help(AppInfo.sourceLabel)
        }
        .buttonStyle(.plain)
    }

    private func linkLabel(_ title: String, icon: String, filled: Bool = false) -> some View {
        HStack(spacing: 6) {
            Image(systemName: icon)
            Text(title)
        }
        .font(.system(size: 13, weight: .semibold))
        .foregroundStyle(filled ? Color.white : Theme.dark)
        .lineLimit(1)
        .frame(maxWidth: .infinity)
        .padding(.vertical, 11)
        .background(Capsule().fill(filled ? Theme.dark : Color.white))
        .overlay(Capsule().strokeBorder(filled ? Color.clear : Theme.dark.opacity(0.35)))
    }
}
