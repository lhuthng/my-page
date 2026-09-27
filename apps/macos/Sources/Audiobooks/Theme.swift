import SwiftUI

/// Colors lifted from the site's Tailwind theme, so the app matches the web.
enum Theme {
    /// --color-dark #495c83
    static let dark = Color(red: 73 / 255, green: 92 / 255, blue: 131 / 255)
    /// --color-primary #7a86b6
    static let primary = Color(red: 122 / 255, green: 134 / 255, blue: 182 / 255)
    /// --color-background #c8b6e2, the purple page wash behind white cards
    static let page = Color(red: 200 / 255, green: 182 / 255, blue: 226 / 255)
    /// --color-accent-green #68b87e — the play button while paused
    static let accentGreen = Color(red: 104 / 255, green: 184 / 255, blue: 126 / 255)
    /// --color-accent-green-dark #42a05c
    static let accentGreenDark = Color(red: 66 / 255, green: 160 / 255, blue: 92 / 255)
    /// --color-accent-red #b86872 — the pause button while playing
    static let accentRed = Color(red: 184 / 255, green: 104 / 255, blue: 114 / 255)
    /// --color-accent-red-dark #c44a58
    static let accentRedDark = Color(red: 196 / 255, green: 74 / 255, blue: 88 / 255)
    /// The mini player's `border-3 border-dark/50` signature border.
    static let cardBorder = dark.opacity(0.5)
}

/// Mirrors the web's `formatClock`: m:ss, h:mm:ss, `--:--` for unknowns.
func clock(_ seconds: Double) -> String {
    guard seconds.isFinite, seconds >= 0 else { return "--:--" }
    let total = Int(seconds)
    let hours = total / 3600
    let minutes = (total % 3600) / 60
    let secs = total % 60
    if hours > 0 {
        return String(format: "%d:%02d:%02d", hours, minutes, secs)
    }
    return String(format: "%d:%02d", minutes, secs)
}

/// Mirrors the web's `formatDurationLabel` (English wording).
func durationLabel(_ seconds: Int64) -> String {
    guard seconds > 0 else { return "" }
    if seconds < 60 { return "under a minute" }
    let minutes = seconds / 60
    if minutes < 60 { return "\(minutes) min" }
    let hours = minutes / 60
    let rest = minutes % 60
    if rest == 0 { return "\(hours) hr" }
    return "\(hours) hr \(rest) min"
}
