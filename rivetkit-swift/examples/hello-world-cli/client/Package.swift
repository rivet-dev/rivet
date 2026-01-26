// swift-tools-version: 6.0
import PackageDescription

let package = Package(
    name: "RivetKitHelloWorldHeadless",
    platforms: [
        .macOS(.v14)
    ],
    dependencies: [
        .package(path: "../../..")
    ],
    targets: [
        .executableTarget(
            name: "HelloWorldHeadless",
            dependencies: [
                .product(name: "RivetKitClient", package: "rivetkit-swift")
            ]
        )
    ]
)
