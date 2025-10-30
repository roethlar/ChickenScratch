// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "ChickenScratchEditor",
    platforms: [
        .macOS(.v13)
    ],
    products: [
        .executable(
            name: "ChickenScratchEditor",
            targets: ["ChickenScratchEditor"]
        )
    ],
    dependencies: [
        .package(url: "https://github.com/jpsim/Yams.git", from: "5.1.0")
    ],
    targets: [
        .executableTarget(
            name: "ChickenScratchEditor",
            dependencies: [
                "Yams"
            ],
            path: "Sources/ChickenScratchEditor",
            linkerSettings: [
                .linkedFramework("SwiftUI"),
                .linkedFramework("AppKit")
            ]
        ),
        .testTarget(
            name: "ChickenScratchEditorTests",
            dependencies: ["ChickenScratchEditor"],
            path: "Tests/ChickenScratchEditorTests"
        )
    ]
)
