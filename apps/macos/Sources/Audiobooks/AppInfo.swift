import Foundation

/// The facts the About screen shows. The version comes from the bundle so it
/// cannot drift from what `Scripts/bundle.sh` writes.
enum AppInfo {
    static let name = "Audiobooks"
    static let siteName = "Huu Thang's Blog"
    static let author = "Huu Thang Le"
    /// The site's own tagline (blog/frontend/src/lib/config/site.js).
    static let tagline =
        "Thắng's digital garden for software architecture, creative coding, and hands-on experiments."
    static let summary = "A native macOS player for the audiobooks on Huu Thang's Blog."
    static let siteURL = URL(string: "https://huuthangle.site")!
    static let githubURL = URL(string: "https://github.com/lhuthng")!

    /// Read from the bundle so it cannot drift from `Scripts/bundle.sh`; a
    /// `swift run` binary has no bundle, hence the fallback.
    static var version: String {
        (Bundle.main.infoDictionary?["CFBundleShortVersionString"] as? String) ?? "0.1.0"
    }
    static var sourceLabel: String {
        githubURL.absoluteString.replacingOccurrences(of: "https://", with: "")
    }
    static var siteLabel: String {
        siteURL.absoluteString.replacingOccurrences(of: "https://", with: "")
    }
}
