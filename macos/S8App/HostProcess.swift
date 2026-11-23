import Darwin
import Foundation

protocol HostRunning: AnyObject {
    func stop()
}

@MainActor
protocol HostLaunching {
    var bridge: URL { get }

    /// `onLine` runs on a background thread for each output line; `onExit` on the main queue.
    func launch(
        arguments: [String],
        onLine: @escaping @Sendable (String) -> Void,
        onExit: @escaping @MainActor (Int32) -> Void
    ) throws -> HostRunning
}

struct BundledHost: HostLaunching {
    let host: URL
    let bridge: URL

    func launch(
        arguments: [String],
        onLine: @escaping @Sendable (String) -> Void,
        onExit: @escaping @MainActor (Int32) -> Void
    ) throws -> HostRunning {
        let process = HostProcess(host: host, bridge: bridge, arguments: arguments)
        try process.start(onLine: onLine, onExit: onExit)
        return process
    }
}

/// S8Host as a child. Its stdin stays open for the whole run: closing it, also when this
/// app ends unexpectedly, makes S8Host blank the panels and stop the bridge.
final class HostProcess: HostRunning, @unchecked Sendable {
    private let process = Process()
    private let input = Pipe()
    private let output = Pipe()
    private let bridge: URL
    private let lock = NSLock()
    private var stopping = false

    init(host: URL, bridge: URL, arguments: [String]) {
        self.bridge = bridge
        process.executableURL = host
        process.arguments = arguments
        process.standardInput = input
        process.standardOutput = output
        process.standardError = output
    }

    func start(onLine: @escaping @Sendable (String) -> Void, onExit: @escaping @MainActor (Int32) -> Void) throws {
        process.terminationHandler = { process in
            let status = process.terminationStatus
            DispatchQueue.main.async { onExit(status) }
        }
        try process.run()
        let reader = output.fileHandleForReading
        Thread.detachNewThread {
            var buffer = Data()
            while true {
                let chunk = reader.availableData
                if chunk.isEmpty {
                    break
                }
                buffer.append(chunk)
                while let newline = buffer.firstIndex(of: 0x0A) {
                    let line = buffer.subdata(in: buffer.startIndex ..< newline)
                    buffer.removeSubrange(buffer.startIndex ... newline)
                    onLine(String(bytes: line, encoding: .utf8) ?? "")
                }
            }
            if !buffer.isEmpty {
                onLine(String(bytes: buffer, encoding: .utf8) ?? "")
            }
        }
    }

    /// Closes stdin, as S8Host's own stop path expects. S8Host gives the bridge up to 10 s;
    /// after 20 s both are killed.
    func stop() {
        lock.lock()
        let first = !stopping
        stopping = true
        lock.unlock()
        guard first else { return }
        try? input.fileHandleForWriting.close()
        DispatchQueue.global().asyncAfter(deadline: .now() + 20) { [self] in
            guard process.isRunning else { return }
            kill(process.processIdentifier, SIGKILL)
            let path = bridge.resolvingSymlinksInPath().path
            for entry in ProcessTable.entries() where entry.path == path {
                kill(entry.pid, SIGKILL)
            }
        }
    }
}
