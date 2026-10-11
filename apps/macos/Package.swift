// swift-tools-version:5.9
import PackageDescription

let package = Package(
    name: "Audiobooks",
    // String form: the v15 case only exists in PackageDescription 6.0, and
    // this manifest stays on tools 5.9 so the language mode does not shift.
    // Keep in step with LSMinimumSystemVersion in Scripts/bundle.sh.
    platforms: [.macOS("15.0")],
    dependencies: [
        // The decoder for chunked chapters. Facebook's own package rather than
        // a wrapper around it: the repository carries a Package.swift that
        // compiles the C library directly, so a wrapper would only add a layer
        // (and a third-party maintainer) between this app and the same code.
        // It has no dependencies of its own.
        .package(url: "https://github.com/facebook/zstd.git", from: "1.5.7")
    ],
    targets: [
        .executableTarget(
            name: "Audiobooks",
            dependencies: [
                // Only for the zstd frame decode; see `ChunkedAudio.swift`.
                .product(name: "libzstd", package: "zstd")
            ],
            path: "Sources/Audiobooks"
        ),
        .testTarget(
            name: "AudiobooksTests",
            dependencies: ["Audiobooks"],
            path: "Tests/AudiobooksTests"
        )
    ]
)
