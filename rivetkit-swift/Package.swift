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
        .library(name: "RivetKitClient", targets: ["RivetKitClient"]),
        .library(name: "RivetKitSwiftUI", targets: ["RivetKitSwiftUI"])
    ],
    dependencies: [
        .package(url: "https://github.com/valpackett/SwiftCBOR.git", from: "0.6.0")
    ],
    targets: [
        .target(
            name: "RivetKitClient",
            dependencies: ["SwiftCBOR"]
        ),
        .target(
            name: "RivetKitSwiftUI",
            dependencies: ["RivetKitClient", "SwiftCBOR"]
        ),
        .testTarget(
            name: "RivetKitClientTests",
            dependencies: ["RivetKitClient"]
        )
    ]
)
