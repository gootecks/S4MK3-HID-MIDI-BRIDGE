import Foundation

@MainActor
protocol SystemProbe {
    func connectedS8Count() -> Int
    func otherBridgeProcesses() -> [String]
    func isAgentInstalled() -> Bool
    func blockingApplications() -> [String]
    func isDjayRunning() -> Bool
    func isAccessibilityTrusted(prompt: Bool) -> Bool
    func isControllerEditorRunning() -> Bool
}

enum PreflightIssue: LocalizedError, Equatable {
    case otherBridge([String])
    case noController
    case severalControllers(Int)
    case agentMissing
    case blocked([String])
    case djayNotRunning
    case accessibility

    var errorDescription: String? {
        switch self {
        case let .otherBridge(names):
            "Another bridge is running (\(names.joined(separator: ", "))). Quit it first."
        case .noController:
            "Connect the Kontrol S8."
        case let .severalControllers(count):
            "\(count) Kontrol S8 are connected. Connect exactly one."
        case .agentMissing:
            "NIHardwareAgent is not installed, so the usual S8 setup could not be restored. "
                + "Install the Native Instruments software for the S8."
        case let .blocked(names):
            "Quit \(names.joined(separator: ", ")) first."
        case .djayNotRunning:
            "Open djay Pro first."
        case .accessibility:
            "Allow Kontrol S8 Bridge in System Settings > Privacy & Security > "
                + "Accessibility, then start again."
        }
    }
}

enum Preflight {
    /// The checks the kit launcher runs before it stops anything, in the same order.
    @MainActor
    static func firstIssue(_ probe: SystemProbe) -> PreflightIssue? {
        let others = probe.otherBridgeProcesses()
        if !others.isEmpty {
            return .otherBridge(others)
        }
        let count = probe.connectedS8Count()
        if count == 0 {
            return .noController
        }
        if count > 1 {
            return .severalControllers(count)
        }
        if !probe.isAgentInstalled() {
            return .agentMissing
        }
        let blockers = probe.blockingApplications()
        if !blockers.isEmpty {
            return .blocked(blockers)
        }
        if !probe.isDjayRunning() {
            return .djayNotRunning
        }
        if !probe.isAccessibilityTrusted(prompt: true) {
            return .accessibility
        }
        return nil
    }
}
