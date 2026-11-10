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
                "S8ViewsTests", "DjaySidebar", "DjaySidebarTests", "S8Control", "S8ControlTests", "S8Host",
                "DjayDecks", "DjayDecksTests"
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
        .executableTarget(name: "S8Preview", dependencies: ["S8Views", "DjayDecks", "DjayLibrary"], path: "S8Preview"),
        .testTarget(
            name: "S8ViewsTests",
            dependencies: ["S8Views"],
            path: "S8ViewsTests",
            resources: [.copy("Fixtures")]
        ),
        .target(name: "DjaySidebar", dependencies: ["S8Views"], path: "DjaySidebar"),
        .testTarget(
            name: "DjaySidebarTests",
            dependencies: ["DjaySidebar"],
            path: "DjaySidebarTests"
        ),
        .target(name: "S8Control", dependencies: ["S8Views"], path: "S8Control"),
        .testTarget(name: "S8ControlTests", dependencies: ["S8Control"], path: "S8ControlTests"),
        .executableTarget(
            name: "S8Host",
            dependencies: ["DjayDecks", "DjaySidebar", "S8Control", "S8Views"],
            path: "S8Host",
            exclude: ["README.md"]
        ),
        .target(
            name: "DjayDecks",
            dependencies: ["DjayLibrary", "S8Views"],
            path: "DjayDecks",
            exclude: ["README.md", "NOTICE"],
            linkerSettings: [.linkedLibrary("z")]
        ),
        .testTarget(
            name: "DjayDecksTests",
            dependencies: ["DjayDecks", "DjayLibrary", "S8Views"],
            path: "DjayDecksTests",
            resources: [.copy("Fixtures")]
        )
    ]
)
