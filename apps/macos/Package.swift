// swift-tools-version:5.9
import PackageDescription

let package = Package(
    name: "Audiobooks",
    // String form: the v15 case only exists in PackageDescription 6.0, and
    // this manifest stays on tools 5.9 so the language mode does not shift.
    // Keep in step with LSMinimumSystemVersion in Scripts/bundle.sh.
    platforms: [.macOS("15.0")],
    targets: [
        .executableTarget(
            name: "Audiobooks",
            path: "Sources/Audiobooks"
        )
    ]
)
