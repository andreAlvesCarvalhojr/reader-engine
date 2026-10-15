// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "ReaderEngine",
    platforms: [
        .iOS(.v15),
        .macOS(.v13),
    ],
    products: [
        .library(name: "ReaderEngine", targets: ["ReaderEngine"]),
    ],
    targets: [
        // Produced by ./scripts/ios-build-xcframework.sh (macOS + Xcode).
        // For GitHub Release consumers, replace with:
        // .binaryTarget(
        //   name: "ReaderFfiFFI",
        //   url: "https://github.com/andreAlvesCarvalhojr/reader-engine/releases/download/vX.Y.Z/ReaderFfiFFI.xcframework.zip",
        //   checksum: "<from script>"
        // )
        .binaryTarget(
            name: "ReaderFfiFFI",
            path: "build/ReaderFfiFFI.xcframework"
        ),
        .target(
            name: "ReaderFfi",
            dependencies: ["ReaderFfiFFI"],
            path: "Sources/ReaderFfi"
        ),
        .target(
            name: "ReaderEngine",
            dependencies: ["ReaderFfi"],
            path: "Sources/ReaderEngine"
        ),
    ]
)
