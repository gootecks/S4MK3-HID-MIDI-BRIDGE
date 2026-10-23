import Foundation
import XCTest
@testable import S4Bridge

@MainActor
final class TestNIHardwareAgent: NIHardwareAgentControlling {
    var running = false
    var blockers: [String] = []
    var checks = 0
    var stops = 0
    var restores = 0
    var stopError: Error?
    var restoreError: Error?
    var stopAction: (() async throws -> Void)?
    var restoreAction: (() async throws -> Void)?

    func isRunning() throws -> Bool {
        checks += 1
        return running
    }

    func blockingApplications() -> [String] { blockers }

    func stop() async throws {
        stops += 1
        if let stopError { throw stopError }
        try await stopAction?()
        running = false
    }

    func restore() async throws {
        restores += 1
        if let restoreError { throw restoreError }
        try await restoreAction?()
        running = true
    }
}

@MainActor
func makeScreenSupport(_ test: XCTestCase, agent: TestNIHardwareAgent) throws -> ScreenSupport {
    let url = FileManager.default.temporaryDirectory
        .appendingPathComponent("S4-Screen-Tests-\(UUID().uuidString)", isDirectory: true)
    try FileManager.default.createDirectory(at: url, withIntermediateDirectories: true)
    test.addTeardownBlock { try FileManager.default.removeItem(at: url) }
    return ScreenSupport(supportURL: url, agent: agent)
}

@MainActor
func makeScreenController(_ test: XCTestCase, agent: TestNIHardwareAgent) throws -> BridgeController {
    let source = URL(fileURLWithPath: #filePath)
        .deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
    let directory = FileManager.default.temporaryDirectory
        .appendingPathComponent("S4-Screen-Controller-\(UUID().uuidString)", isDirectory: true)
    try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
    test.addTeardownBlock { try FileManager.default.removeItem(at: directory) }
    for name in ["led", "jog", "tension"] {
        try Data(contentsOf: source.appendingPathComponent("examples/\(name)-config.json"))
            .write(to: directory.appendingPathComponent("default-\(name)-config.json"))
    }
    return try BridgeController(resourceURL: directory,
                                supportURL: directory.appendingPathComponent("support"), agent: agent)
}
