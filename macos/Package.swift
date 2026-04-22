// swift-tools-version: 6.1
import PackageDescription

let package = Package(
    name: "ChickenScratch",
    platforms: [
        .macOS("26.0"),
    ],
    products: [
        .executable(name: "ChickenScratch", targets: ["ChickenScratchApp"]),
        .library(name: "ChiknKit", targets: ["ChiknKit"]),
    ],
    dependencies: [
        .package(url: "https://github.com/jpsim/Yams", from: "5.1.3"),
    ],
    targets: [
        .executableTarget(
            name: "ChickenScratchApp",
            dependencies: ["ChiknKit"],
            path: "Sources/ChickenScratchApp"
        ),
        .target(
            name: "ChiknKit",
            dependencies: [
                .product(name: "Yams", package: "Yams"),
            ],
            path: "Sources/ChiknKit"
        ),
    ]
)
