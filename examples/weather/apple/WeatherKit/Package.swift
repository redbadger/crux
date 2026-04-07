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
        .package(path: "../generated/App"),
        .package(path: "../generated/Shared")
    ],
    targets: [
        .target(
            name: "WeatherKit",
            dependencies: [
                .product(name: "App", package: "App"),
                .product(name: "Shared", package: "Shared")
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
