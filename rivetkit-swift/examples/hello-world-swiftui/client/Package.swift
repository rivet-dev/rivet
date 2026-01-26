// swift-tools-version: 6.0
import PackageDescription

let package = Package(
    name: "RivetKitHelloWorld",
    platforms: [
        .macOS(.v14)
    ],
    dependencies: [
        .package(path: "../../..")
    ],
    targets: [
        .executableTarget(
            name: "HelloWorldApp",
            dependencies: [
                .product(name: "RivetKitSwiftUI", package: "rivetkit-swift")
            ]
        )
    ]
)
