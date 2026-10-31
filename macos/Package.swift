// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "S4Bridge",
    platforms: [.macOS(.v12)],
    products: [.executable(name: "S4Bridge", targets: ["S4Bridge"])],
    targets: [
        .executableTarget(
            name: "S4Bridge",
            path: ".",
            exclude: [
                "build-app.sh", "Info.plist", "README.md", "DESIGN.md", "Tests", "S8Views", "S8Preview",
                "S8ViewsTests"
            ],
            sources: [
                "S4BridgeApp.swift", "ScreenPreferences.swift", "ScreenSupport.swift",
                "ScreenSupport+Control.swift", "ScreenSettingsView.swift",
                "BridgeController+Screens.swift", "ScreenPrompts.swift", "NIHardwareAgent.swift",
                "NIHardwareAgentNative.swift", "BridgeStatusMenu.swift"
            ]
        ),
        .target(name: "DjayLibrary", path: "DjayLibrary", exclude: ["README.md"]),
        .testTarget(
            name: "DjayLibraryTests",
            dependencies: ["DjayLibrary"],
            path: "DjayLibraryTests",
            resources: [.copy("Fixtures")]
        ),
        .testTarget(name: "S4BridgeTests", dependencies: ["S4Bridge"], path: "Tests"),
        .target(name: "S8Views", path: "S8Views", exclude: ["README.md"]),
        .executableTarget(name: "S8Preview", dependencies: ["S8Views"], path: "S8Preview"),
        .testTarget(
            name: "S8ViewsTests",
            dependencies: ["S8Views"],
            path: "S8ViewsTests",
            resources: [.copy("Fixtures")]
        )
    ]
)
