// swift-tools-version: 6.3
// The swift-tools-version declares the minimum version of Swift required to build this package.

import PackageDescription

let package = Package(
    name: "iotcraft-swift-client",
    platforms: [
        .macOS(.v13) // Support for modern SwiftUI and async/await features
    ],
    products: [
        .executable(
            name: "iotcraft-swift-client",
            targets: ["IoTCraftSwiftClient"]
        ),
        .executable(
            name: "swift-test-runner",
            targets: ["SwiftTestRunner"]
        ),
    ],
    dependencies: [
        // MQTT client library for Swift
        .package(url: "https://github.com/sroebert/mqtt-nio.git", from: "2.8.0"),
        // JSON handling
        .package(url: "https://github.com/Flight-School/AnyCodable.git", from: "0.6.0"),
        // Logging
        .package(url: "https://github.com/apple/swift-log.git", from: "1.5.3"),
        // ArgumentParser for command-line interface
        .package(url: "https://github.com/apple/swift-argument-parser.git", from: "1.3.0"),
    ],
    targets: [
        .executableTarget(
            name: "IoTCraftSwiftClient",
            dependencies: [
                .product(name: "MQTTNIO", package: "mqtt-nio"),
                .product(name: "AnyCodable", package: "AnyCodable"),
                .product(name: "Logging", package: "swift-log"),
                .product(name: "ArgumentParser", package: "swift-argument-parser"),
            ],
            swiftSettings: [
                // Enable Swift 6.3-dev features
                .enableUpcomingFeature("StrictConcurrency"),
                .enableExperimentalFeature("AccessLevelOnImport"),
                .enableExperimentalFeature("BitwiseCopyable"),
                // Fix @main attribute issue in Swift 6.3-dev
                .unsafeFlags(["-parse-as-library"]),
            ]
        ),
        .executableTarget(
            name: "SwiftTestRunner",
            dependencies: [
                .product(name: "Logging", package: "swift-log"),
                .product(name: "ArgumentParser", package: "swift-argument-parser"),
            ],
            swiftSettings: [
                // Enable Swift 6.3-dev features
                .enableUpcomingFeature("StrictConcurrency"),
                .enableExperimentalFeature("AccessLevelOnImport"),
                .enableExperimentalFeature("BitwiseCopyable"),
                // Fix @main attribute issue in Swift 6.3-dev
                .unsafeFlags(["-parse-as-library"]),
            ]
        ),
        .testTarget(
            name: "IoTCraftSwiftClientTests",
            dependencies: ["IoTCraftSwiftClient"]
        ),
    ]
)
