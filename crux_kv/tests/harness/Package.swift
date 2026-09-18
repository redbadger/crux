// swift-tools-version: 5.8
import PackageDescription

/// A harness beside the generated package, depending on it by path.
///
/// Nothing here is generated: the test writes this file and `main.swift` next
/// to whatever `swift()` emitted, so that the shipped source is exercised
/// exactly as an app would exercise it — through the module's public API.
let package = Package(
    name: "Harness",
    dependencies: [
        .package(path: "../generated/App")
    ],
    targets: [
        .executableTarget(
            name: "Harness",
            dependencies: [.product(name: "App", package: "App")]
        )
    ]
)
