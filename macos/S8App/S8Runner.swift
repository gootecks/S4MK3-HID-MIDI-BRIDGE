import Foundation

enum RunState: Equatable {
    case stopped
    case checking
    case stoppingAgent
    case starting
    case running
    case stopping
    case failed(String)

    var title: String {
        switch self {
        case .stopped: "Stopped"
        case .checking: "Checking"
        case .stoppingAgent: "Stopping NIHardwareAgent"
        case .starting: "Starting"
        case .running: "Running"
        case .stopping: "Stopping"
        case .failed: "Stopped with a problem"
        }
    }

    var isIdle: Bool {
        switch self {
        case .stopped, .failed: true
        default: false
        }
    }
}

enum RunnerPrompt: Equatable {
    case restartDjay([String])
    case controllerEditorClosed
}

/// The kit launcher's sequence as a state machine: checks, mappings, NIHardwareAgent
/// stop, S8Host, and the agent's restore on every way out.
@MainActor
final class S8Runner {
    static let agentStoppedKey = "agentStoppedByApp"

    var onChange: (() -> Void)?
    private(set) var state: RunState = .stopped {
        didSet { onChange?() }
    }

    private(set) var detail = "" {
        didSet { onChange?() }
    }

    private(set) var logURL: URL?
    private(set) var mappingName: String?

    private let probe: SystemProbe
    private let agent: AgentControlling
    private let host: HostLaunching
    private let mappings: MappingInstaller
    private let defaults: UserDefaults
    private let makeLog: () -> RunLog
    private var running: HostRunning?
    private var problem: String?
    private var stopWaiters: [CheckedContinuation<Void, Never>] = []

    init(
        probe: SystemProbe,
        agent: AgentControlling,
        host: HostLaunching,
        mappings: MappingInstaller,
        defaults: UserDefaults = .standard,
        makeLog: @escaping () -> RunLog = { RunLog() }
    ) {
        self.probe = probe
        self.agent = agent
        self.host = host
        self.mappings = mappings
        self.defaults = defaults
        self.makeLog = makeLog
    }

    var agentStoppedByApp: Bool {
        defaults.bool(forKey: Self.agentStoppedKey)
    }

    func start(_ preferences: S8Preferences, ask: (RunnerPrompt) async -> Bool) async {
        guard state.isIdle else { return }
        detail = ""
        problem = nil
        state = .checking
        guard await prepare(preferences, ask: ask) else { return }

        state = .stoppingAgent
        defaults.set(true, forKey: Self.agentStoppedKey)
        do {
            try await agent.stop()
        } catch {
            await restoreAgent()
            return fail(problem ?? error.localizedDescription)
        }
        await launch(preferences)
    }

    private func prepare(_ preferences: S8Preferences, ask: (RunnerPrompt) async -> Bool) async -> Bool {
        if let issue = Preflight.firstIssue(probe) {
            fail(issue.localizedDescription)
            return false
        }
        let installed: [String]
        do {
            mappingName = try mappings.mappingName(for: preferences.layout)
            installed = try mappings.installMissing()
        } catch {
            fail(error.localizedDescription)
            return false
        }
        if !installed.isEmpty, probe.isDjayRunning(), await !ask(.restartDjay(installed)) {
            state = .stopped
            return false
        }
        if !probe.isControllerEditorRunning(), await !ask(.controllerEditorClosed) {
            state = .stopped
            return false
        }
        return true
    }

    private func launch(_ preferences: S8Preferences) async {
        let log = makeLog()
        logURL = log.url
        let arguments = preferences.hostArguments(bridge: host.bridge)
        log.write("Kontrol S8 Bridge \(Self.version): S8Host \(arguments.joined(separator: " "))")
        state = .starting
        do {
            running = try host.launch(
                arguments: arguments,
                onLine: { [weak self] line in
                    log.write(line)
                    guard Self.isNotable(line) else { return }
                    Task { @MainActor in self?.handle(line) }
                },
                onExit: { [weak self] status in
                    log.write("S8Host exited with status \(status)")
                    Task { await self?.hostExited(status) }
                }
            )
        } catch {
            log.write("cannot start S8Host: \(error.localizedDescription)")
            await restoreAgent()
            fail(problem ?? "Cannot start S8Host: \(error.localizedDescription)")
        }
    }

    func stop() async {
        guard let running else { return }
        if state != .stopping {
            state = .stopping
            running.stop()
        }
        await withCheckedContinuation { stopWaiters.append($0) }
    }

    /// Stops S8Host and makes sure the agent runs again; used on quit.
    func shutdown() async {
        await stop()
        if agentStoppedByApp {
            await restoreAgent()
        }
    }

    /// After a crash of this app the agent may still be stopped; start it again.
    func recoverAgent() async {
        guard agentStoppedByApp, running == nil, probe.otherBridgeProcesses().isEmpty else { return }
        await restoreAgent()
        if let problem {
            state = .failed(problem)
        }
    }

    func installMappings() throws -> [String] {
        try mappings.installMissing()
    }

    func handle(_ line: String) {
        if line.contains("MIDI_FULL_READY"), state == .starting {
            state = .running
            if let mappingName {
                detail = "In djay: Kontrol S8 MIDI Full with mapping \(mappingName)."
            }
        } else if line.contains("S8_LEDS_DISABLED") {
            detail = "Deck LEDs are off after a write error; MIDI goes on."
        } else if line.contains("S8_FEEDBACK_DISABLED") {
            detail = "djay feedback is off for this run; MIDI goes on."
        } else if let range = line.range(of: "S8Host: ") {
            detail = String(line[range.upperBound...])
        } else if line.contains(" ERROR ") {
            detail = String(line.suffix(160))
        }
    }

    private func hostExited(_ status: Int32) async {
        running = nil
        let requested = state == .stopping
        await restoreAgent()
        if let problem {
            state = .failed(problem)
        } else if requested {
            state = .stopped
        } else {
            let reason = detail.isEmpty ? "" : " \(detail)"
            state = .failed("S8Host stopped with status \(status).\(reason)")
        }
        let waiters = stopWaiters
        stopWaiters.removeAll()
        waiters.forEach { $0.resume() }
    }

    private func restoreAgent() async {
        do {
            try await agent.restore()
            defaults.set(false, forKey: Self.agentStoppedKey)
        } catch {
            problem = "\(error.localizedDescription) Start it with: "
                + "open -g '/Library/Application Support/Native Instruments/Hardware/NIHardwareAgent.app'"
        }
    }

    private func fail(_ message: String) {
        state = .failed(message)
    }

    nonisolated static func isNotable(_ line: String) -> Bool {
        ["MIDI_FULL_READY", "S8_LEDS_DISABLED", "S8_FEEDBACK_DISABLED", "S8Host: ", " ERROR "]
            .contains { line.contains($0) }
    }

    static var version: String {
        let info = Bundle.main.infoDictionary
        let short = info?["CFBundleShortVersionString"] as? String ?? "dev"
        let commit = info?["S8BridgeCommit"] as? String ?? "dev"
        return "\(short) (\(commit))"
    }
}
