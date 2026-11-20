import AppKit
import Darwin
import Foundation

@MainActor
protocol AgentControlling {
    var isInstalled: Bool { get }
    func isRunning() -> Bool
    func stop() async throws
    func restore() async throws
}

enum AgentError: LocalizedError, Equatable {
    case notInstalled
    case stopFailed
    case restoreFailed(String)

    var errorDescription: String? {
        switch self {
        case .notInstalled: "NIHardwareAgent is not installed."
        case .stopFailed: "NIHardwareAgent did not stop."
        case let .restoreFailed(reason): "NIHardwareAgent did not start again: \(reason)"
        }
    }
}

/// Stops NIHardwareAgent with SIGKILL and starts it again like `open -g`, as the
/// kit launcher and the attended S8 runners do.
@MainActor
final class NIAgent: AgentControlling {
    static let bundleURL = URL(
        fileURLWithPath: "/Library/Application Support/Native Instruments/Hardware/NIHardwareAgent.app",
        isDirectory: true
    )

    private var executable: String? {
        Bundle(url: Self.bundleURL)?.executableURL?.resolvingSymlinksInPath().path
    }

    var isInstalled: Bool {
        executable.map { FileManager.default.isExecutableFile(atPath: $0) } ?? false
    }

    func isRunning() -> Bool {
        !processes().isEmpty
    }

    func stop() async throws {
        guard isInstalled else { throw AgentError.notInstalled }
        for pid in processes() {
            kill(pid, SIGKILL)
        }
        guard await wait(until: { !self.isRunning() }) else { throw AgentError.stopFailed }
    }

    /// LaunchServices reports the start of a background agent only after a 30 s timeout,
    /// so this waits for the process instead of the completion.
    func restore() async throws {
        guard isInstalled else { throw AgentError.notInstalled }
        guard !isRunning() else { return }
        let configuration = NSWorkspace.OpenConfiguration()
        configuration.activates = false
        configuration.addsToRecentItems = false
        NSWorkspace.shared.openApplication(at: Self.bundleURL, configuration: configuration, completionHandler: nil)
        guard await wait(until: { self.isRunning() }) else {
            throw AgentError.restoreFailed("it is not running after 10 s.")
        }
    }

    private func processes() -> [pid_t] {
        guard let executable else { return [] }
        return ProcessTable.entries().filter { $0.path == executable }.map(\.pid)
    }

    private func wait(until condition: @escaping () -> Bool) async -> Bool {
        for _ in 0 ..< 100 {
            if condition() {
                return true
            }
            try? await Task.sleep(nanoseconds: 100_000_000)
        }
        return condition()
    }
}

enum ProcessTable {
    struct Entry {
        let pid: pid_t
        let path: String
    }

    static func entries() -> [Entry] {
        let count = proc_listallpids(nil, 0)
        var identifiers = [pid_t](repeating: 0, count: max(0, Int(count)) * 2)
        let found = identifiers.withUnsafeMutableBytes {
            proc_listallpids($0.baseAddress, Int32($0.count))
        }
        return identifiers.prefix(max(0, Int(found))).compactMap { pid in
            guard pid > 0 else { return nil }
            var buffer = [CChar](repeating: 0, count: 4 * Int(MAXPATHLEN))
            guard proc_pidpath(pid, &buffer, UInt32(buffer.count)) > 0 else { return nil }
            let path = URL(fileURLWithPath: String(cString: buffer)).resolvingSymlinksInPath().path
            return Entry(pid: pid, path: path)
        }
    }
}
