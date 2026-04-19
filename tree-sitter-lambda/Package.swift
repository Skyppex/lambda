// swift-tools-version:5.3

import Foundation
import PackageDescription

var sources = ["src/parser.c"]
if FileManager.default.fileExists(atPath: "src/scanner.c") {
    sources.append("src/scanner.c")
}

let package = Package(
    name: "TreeSitterLambda",
    products: [
        .library(name: "TreeSitterLambda", targets: ["TreeSitterLambda"]),
    ],
    dependencies: [
        .package(name: "SwiftTreeSitter", url: "https://github.com/tree-sitter/swift-tree-sitter", from: "0.9.0"),
    ],
    targets: [
        .target(
            name: "TreeSitterLambda",
            dependencies: [],
            path: ".",
            sources: sources,
            resources: [
                .copy("queries")
            ],
            publicHeadersPath: "bindings/swift",
            cSettings: [.headerSearchPath("src")]
        ),
        .testTarget(
            name: "TreeSitterLambdaTests",
            dependencies: [
                "SwiftTreeSitter",
                "TreeSitterLambda",
            ],
            path: "bindings/swift/TreeSitterLambdaTests"
        )
    ],
    cLanguageStandard: .c11
)
