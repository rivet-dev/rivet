// swift-tools-version: 6.0
import PackageDescription

let package = Package(
    name: "rivetkit-swift",
    platforms: [
        .iOS(.v17),
        .macOS(.v14),
        .tvOS(.v17),
        .watchOS(.v10),
        .visionOS(.v1)
    ],
    products: [
        .library(name: "RivetKitClient", targets: ["RivetKitClient"])
    ],
    dependencies: [],
    targets: [
        .target(
            name: "RivetKitClient",
            dependencies: []
        ),
        .testTarget(
            name: "RivetKitClientTests",
            dependencies: ["RivetKitClient"]
        )
    ]
)
