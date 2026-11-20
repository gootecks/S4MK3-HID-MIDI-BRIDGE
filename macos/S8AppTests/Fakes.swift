import Foundation
@testable import S8App

@MainActor
final class FakeProbe: SystemProbe {
    var s8Count = 1
    var others: [String] = []
    var agentInstalled = true
    var blockers: [String] = []
    var djay = true
    var trusted = true
    var controllerEditor = true

    func connectedS8Count() -> Int {
        s8Count
    }

    func otherBridgeProcesses() -> [String] {
        others
    }

    func isAgentInstalled() -> Bool {
        agentInstalled
    }

    func blockingApplications() -> [String] {
        blockers
    }

    func isDjayRunning() -> Bool {
        djay
    }

    func isAccessibilityTrusted(prompt _: Bool) -> Bool {
        trusted
    }

    func isControllerEditorRunning() -> Bool {
        controllerEditor
    }
}

@MainActor
final class FakeAgent: AgentControlling {
    var running = true
    var failRestore = false
    var events: [String] = []

    var isInstalled: Bool {
        true
    }

    func isRunning() -> Bool {
        running
    }

    func stop() async throws {
        events.append("stop")
        running = false
    }

    func restore() async throws {
        events.append("restore")
        if failRestore {
            throw AgentError.restoreFailed("test")
        }
        running = true
    }
}

final class FakeRunning: HostRunning {
    var stopped = false
    var onStop: (() -> Void)?

    func stop() {
        stopped = true
        onStop?()
    }
}

@MainActor
final class FakeHost: HostLaunching {
    let bridge = URL(fileURLWithPath: "/App/Contents/MacOS/s4mk3-hid-midi-bridge")
    var launchError: Error?
    var arguments: [String]?
    var onLine: ((String) -> Void)?
    var onExit: ((Int32) -> Void)?
    let running = FakeRunning()

    func launch(
        arguments: [String],
        onLine: @escaping @Sendable (String) -> Void,
        onExit: @escaping @MainActor (Int32) -> Void
    ) throws -> HostRunning {
        if let launchError {
            throw launchError
        }
        self.arguments = arguments
        self.onLine = onLine
        self.onExit = onExit
        return running
    }
}

struct TestFolder {
    let url: URL

    init() {
        url = FileManager.default.temporaryDirectory
            .appendingPathComponent("S8AppTests-\(UUID().uuidString)", isDirectory: true)
        try? FileManager.default.createDirectory(at: url, withIntermediateDirectories: true)
    }

    func folder(_ name: String) -> URL {
        let folder = url.appendingPathComponent(name, isDirectory: true)
        try? FileManager.default.createDirectory(at: folder, withIntermediateDirectories: true)
        return folder
    }

    func remove() {
        try? FileManager.default.removeItem(at: url)
    }
}
