import AppKit
import Foundation

@MainActor
protocol NIHardwareAgentControlling {
    func isRunning() throws -> Bool
    func blockingApplications() -> [String]
    func stop() async throws
    func restore() async throws
}

enum NIHardwareAgentError: LocalizedError, Equatable {
    case invalidInstalledBundle(String)
    case ambiguousRunningIdentity
    case blocked([String])
    case terminationDenied(String)
    case terminationTimedOut
    case launchFailed(String)
    case invalidLaunchIdentity

    var errorDescription: String? {
        switch self {
        case .invalidInstalledBundle(let detail):
            return "NIHardwareAgent could not be verified at its installed location: \(detail)"
        case .ambiguousRunningIdentity:
            return "A process claims the NIHardwareAgent bundle identifier but does not match the installed executable."
        case .blocked(let applications):
            return "Quit \(applications.joined(separator: ", ")) before stopping NIHardwareAgent."
        case .terminationDenied(let detail):
            return "macOS did not allow NIHardwareAgent to stop. Quit Native Instruments applications and try again. \(detail)"
        case .terminationTimedOut:
            return "NIHardwareAgent did not quit before the 10-second deadline. Quit Native Instruments applications and try again."
        case .launchFailed(let detail):
            return "Could not restore NIHardwareAgent. Check that the installed Native Instruments service is available and that macOS allows it to open. \(detail)"
        case .invalidLaunchIdentity:
            return "macOS reported a launch, but it was not the verified NIHardwareAgent executable."
        }
    }
}

struct NIHardwareAgentProcess: Equatable {
    let processIdentifier: pid_t
    let bundleIdentifier: String?
    let executableURL: URL?
    let localizedName: String
}

enum NIHardwareAgentTerminationResult: Equatable {
    case requested
    case alreadyStopped
    case denied
}

@MainActor
protocol NIHardwareAgentNative: AnyObject {
    func runningProcesses() -> [NIHardwareAgentProcess]
    func verifiedExecutableURL(bundleURL: URL, bundleIdentifier: String) throws -> URL
    func terminationEvents(for processIdentifier: pid_t) -> AsyncStream<Void>
    func terminate(processIdentifier: pid_t) -> NIHardwareAgentTerminationResult
    func launch(bundleURL: URL, activates: Bool) async throws -> NIHardwareAgentProcess
}

@MainActor
final class NIHardwareAgent: NIHardwareAgentControlling {
    static let installedBundleURL = URL(fileURLWithPath:
        "/Library/Application Support/Native Instruments/Hardware/NIHardwareAgent.app",
        isDirectory: true
    )
    static let bundleIdentifier = "com.native-instruments.NIHardwareService"
    static let teamIdentifier = "83K5EG6Z9V"

    private let bundleURL: URL
    private let native: NIHardwareAgentNative
    private let terminationDeadline: TimeInterval

    convenience init() {
        self.init(bundleURL: Self.installedBundleURL, native: SystemNIHardwareAgentNative(),
                  terminationDeadline: 10)
    }

    init(
        bundleURL: URL,
        native: NIHardwareAgentNative,
        terminationDeadline: TimeInterval = 10
    ) {
        self.bundleURL = bundleURL
        self.native = native
        self.terminationDeadline = terminationDeadline
    }

    func isRunning() throws -> Bool {
        let matching = native.runningProcesses().filter {
            $0.bundleIdentifier == Self.bundleIdentifier
        }
        guard !matching.isEmpty else { return false }
        let executable = try verifiedExecutableURL()
        guard matching.allSatisfy({ Self.sameFile($0.executableURL, executable) }) else {
            throw NIHardwareAgentError.ambiguousRunningIdentity
        }
        return matching.contains { $0.processIdentifier > 0 }
    }

    func blockingApplications() -> [String] {
        let prefixes = [
            "com.native-instruments.traktor",
            "com.native-instruments.maschine",
            "com.native-instruments.kompletekontrol"
        ]
        return Array(Set(native.runningProcesses().compactMap { process -> String? in
            guard let identifier = process.bundleIdentifier?.lowercased(),
                  prefixes.contains(where: identifier.hasPrefix) else { return nil }
            return process.localizedName.isEmpty ? identifier : process.localizedName
        })).sorted()
    }

    func stop() async throws {
        let executable = try verifiedExecutableURL()
        let matching = native.runningProcesses().filter {
            $0.bundleIdentifier == Self.bundleIdentifier
        }
        guard matching.allSatisfy({ Self.sameFile($0.executableURL, executable) }) else {
            if !matching.isEmpty { throw NIHardwareAgentError.ambiguousRunningIdentity }
            return
        }
        guard let process = matching.first else { return }
        let blockers = blockingApplications()
        guard blockers.isEmpty else { throw NIHardwareAgentError.blocked(blockers) }

        // Constructing the stream registers its native exit observer before termination.
        let events = native.terminationEvents(for: process.processIdentifier)
        let exitTask = Task {
            await Self.awaitExit(events, deadline: terminationDeadline)
        }
        let result = native.terminate(processIdentifier: process.processIdentifier)
        if result != .requested {
            exitTask.cancel()
            _ = await exitTask.value
        }
        switch result {
        case .alreadyStopped: return
        case .denied:
            throw NIHardwareAgentError.terminationDenied(
                "Check Native Instruments app activity and your macOS account permissions."
            )
        case .requested: break
        }
        let exited = await exitTask.value
        guard exited else { throw NIHardwareAgentError.terminationTimedOut }
    }

    func restore() async throws {
        let executable = try verifiedExecutableURL()
        let existing = native.runningProcesses().filter {
            $0.bundleIdentifier == Self.bundleIdentifier
        }
        guard existing.allSatisfy({ Self.sameFile($0.executableURL, executable) }) else {
            if !existing.isEmpty { throw NIHardwareAgentError.ambiguousRunningIdentity }
            throw NIHardwareAgentError.invalidInstalledBundle("The installed executable identity changed.")
        }
        if existing.contains(where: { $0.processIdentifier > 0 }) { return }

        let launched: NIHardwareAgentProcess
        do {
            launched = try await native.launch(bundleURL: bundleURL, activates: false)
        } catch {
            throw NIHardwareAgentError.launchFailed(error.localizedDescription)
        }
        guard launched.processIdentifier > 0,
              launched.bundleIdentifier == Self.bundleIdentifier,
              Self.sameFile(launched.executableURL, executable) else {
            throw NIHardwareAgentError.invalidLaunchIdentity
        }
    }

    private func verifiedExecutableURL() throws -> URL {
        try native.verifiedExecutableURL(bundleURL: bundleURL, bundleIdentifier: Self.bundleIdentifier)
    }

    private static func sameFile(_ candidate: URL?, _ expected: URL) -> Bool {
        guard let candidate else { return false }
        return candidate.standardizedFileURL.resolvingSymlinksInPath().path
            == expected.standardizedFileURL.resolvingSymlinksInPath().path
    }

    private static func awaitExit(
        _ events: AsyncStream<Void>,
        deadline: TimeInterval
    ) async -> Bool {
        await withTaskGroup(of: Bool.self) { group in
            group.addTask {
                for await _ in events { return true }
                return false
            }
            group.addTask {
                do {
                    let nanoseconds = UInt64(max(0, deadline) * 1_000_000_000)
                    try await Task.sleep(nanoseconds: nanoseconds)
                    return false
                } catch {
                    return false
                }
            }
            let result = await group.next() ?? false
            group.cancelAll()
            return result
        }
    }
}
