import Combine
import Foundation

@MainActor
final class ScreenSupport: ObservableObject {
    @Published private(set) var preferences = ScreenPreferences()
    @Published var state: ScreenState = .ready
    @Published private(set) var driverRunning: Bool?
    @Published private(set) var busy = false
    @Published var errorMessage = ""
    let preferencesURL: URL
    let agent: NIHardwareAgentControlling
    var loadError: Error?
    var bridgeRunning = false
    var fullProfile = false
    var restoreAfterExit = false
    var generation: UInt64 = 0
    var sendCommand: ((UInt64, ScreenCommand) throws -> Void)?
    var releaseWaiter: CheckedContinuation<Void, Error>?
    var releaseDeadline: DispatchWorkItem?
    private var operationCount = 0
    private var restoration: Task<Void, Error>?

    init(supportURL: URL, agent: NIHardwareAgentControlling) {
        preferencesURL = supportURL.appendingPathComponent("screen-settings.json")
        self.agent = agent
        do {
            if FileManager.default.fileExists(atPath: preferencesURL.path) {
                preferences = try ScreenPreferences.decode(Data(contentsOf: preferencesURL))
            }
        } catch {
            loadError = error
            errorMessage = "Screen preferences could not be loaded: \(error.localizedDescription)"
            state = .failed
        }
        if loadError == nil { state = preferences.enabled ? .ready : .off }
    }

    var needsAttention: Bool {
        preferences.enabled && (state == .blocked || state == .failed)
    }

    func beginOperation() {
        operationCount += 1
        if !busy { busy = true }
    }

    func endOperation() {
        precondition(operationCount > 0)
        operationCount -= 1
        if operationCount == 0 { busy = false }
    }

    func save() throws {
        if let loadError { throw loadError }
        try preferences.save(to: preferencesURL)
    }

    func changePreferences(_ update: (inout ScreenPreferences) -> Void) throws {
        if let loadError { throw loadError }
        var changed = preferences
        update(&changed)
        try changed.save(to: preferencesURL)
        preferences = changed
    }

    func reset() {
        let pending = preferences.restorePending
        preferences = ScreenPreferences()
        preferences.restorePending = pending
        loadError = nil
        errorMessage = ""
        state = .ready
    }

    func refreshDriverStatus() {
        guard preferences.enabled else { return }
        do {
            let running = try agent.isRunning()
            if driverRunning != running { driverRunning = running }
        } catch { fail(error) }
    }

    func prepare(full: Bool, consent: () -> ScreenConsent) async throws -> Bool? {
        guard full && preferences.enabled else {
            state = .off
            return false
        }
        let running = try agent.isRunning()
        driverRunning = running
        guard running else {
            state = .pending
            return true
        }
        let choice = preferences.automaticallyStopNI ? .allow(remember: false) : consent()
        switch choice {
        case .cancel:
            state = .ready
            return nil
        case .withoutScreens:
            state = .blocked
            return false
        case .allow(let remember):
            if remember { try changePreferences { $0.automaticallyStopNI = true } }
            try await stopDriver()
            state = .pending
            return true
        }
    }

    func stopDriver() async throws {
        guard try agent.isRunning() else {
            driverRunning = false
            return
        }
        let blockers = agent.blockingApplications()
        guard blockers.isEmpty else {
            throw AppError.invalid("Close \(blockers.joined(separator: ", ")) before stopping the Native Instruments driver.")
        }
        try changePreferences { $0.restorePending = true }
        do { try await agent.stop() }
        catch {
            if (try? agent.isRunning()) == true {
                try changePreferences { $0.restorePending = false }
            }
            throw error
        }
        driverRunning = false
    }

    func restoreDriver() async throws {
        if let restoration { return try await restoration.value }
        let task = Task { @MainActor in
            try await agent.restore()
            driverRunning = true
            try changePreferences { $0.restorePending = false }
            errorMessage = ""
        }
        restoration = task
        defer { restoration = nil }
        try await task.value
    }

    func restoreOwnedDriver() async throws {
        if preferences.restorePending && preferences.restoreNIOnStop {
            try await restoreDriver()
        }
    }

    func bridgeStarted(screens: Bool, full: Bool) {
        bridgeRunning = true
        fullProfile = full
        generation = 0
        if screens { state = .pending }
        else if state != .blocked { state = .off }
    }

    func bridgeExited(restarting: Bool) async {
        bridgeRunning = false
        sendCommand = nil
        releaseDeadline?.cancel()
        releaseDeadline = nil
        releaseWaiter?.resume()
        releaseWaiter = nil
        guard !restarting else { return }
        do {
            if restoreAfterExit {
                restoreAfterExit = false
                try await restoreDriver()
            } else {
                try await restoreOwnedDriver()
            }
            state = preferences.enabled ? .ready : .off
        } catch { fail(error) }
    }

    func fail(_ error: Error) {
        errorMessage = error.localizedDescription
        state = .failed
    }
}
