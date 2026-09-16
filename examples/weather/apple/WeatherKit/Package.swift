// swift-tools-version: 6.2

import PackageDescription

let package = Package(
    name: "WeatherKit",
    platforms: [
        .macOS(.v15),
        .iOS(.v18)
    ],
    products: [
        .library(
            name: "WeatherKit",
            targets: ["WeatherKit"]
        )
    ],
    dependencies: [
        .package(path: "../generated/App")
    ],
    targets: [
        .target(
            name: "WeatherKit",
            dependencies: [
                .product(name: "App", package: "App")
            ],
            resources: [
                .process("Core/KeyValueModel.xcdatamodeld")
            ],
            swiftSettings: [
                .swiftLanguageMode(.v6),
                .defaultIsolation(MainActor.self)
            ]
        )
    ]
)
