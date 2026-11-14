import Foundation

final class Bridge {
    private let process = Process()
    private let output = Pipe()
    private let input: Pipe?
    private let lock = NSLock()
    private var inputOpen: Bool
    private var stopping = false

    init(path: String, screens: Bool, leds: Bool) {
        var arguments = ["--device", "s8", "--midi", "full", "--until-stopped", "--app-events"]
        if leds {
            arguments.append("--s8-leds")
        }
        if screens {
            arguments.append("--s8-screens-stdin")
        }
        input = screens ? Pipe() : nil
        inputOpen = screens
        process.executableURL = URL(fileURLWithPath: path)
        process.arguments = arguments
        process.standardOutput = output
        process.standardError = FileHandle.standardError
        process.standardInput = input ?? FileHandle.nullDevice
    }

    func start() throws {
        try process.run()
    }

    @discardableResult
    func send(_ record: Data) -> Bool {
        lock.lock()
        defer { lock.unlock() }
        guard inputOpen, let handle = input?.fileHandleForWriting else { return false }
        do {
            try handle.write(contentsOf: record)
            return true
        } catch {
            inputOpen = false
            return false
        }
    }

    private func closeInput() {
        lock.lock()
        defer { lock.unlock() }
        guard inputOpen else { return }
        inputOpen = false
        try? input?.fileHandleForWriting.close()
    }

    func stop() {
        guard process.isRunning, !stopping else { return }
        stopping = true
        guard input != nil else {
            process.terminate()
            return
        }
        let background = DispatchQueue.global()
        background.async { self.closeInput() }
        background.asyncAfter(deadline: .now() + 5) {
            if self.process.isRunning {
                self.process.interrupt()
            }
        }
        background.asyncAfter(deadline: .now() + 10) {
            if self.process.isRunning {
                self.process.terminate()
            }
        }
    }

    func readLines(_ handle: (String) -> Void) {
        var buffer = Data()
        let reader = output.fileHandleForReading
        while true {
            let chunk = reader.availableData
            if chunk.isEmpty {
                break
            }
            buffer.append(chunk)
            while let newline = buffer.firstIndex(of: 0x0A) {
                let line = buffer.subdata(in: buffer.startIndex ..< newline)
                buffer.removeSubrange(buffer.startIndex ... newline)
                handle(String(bytes: line, encoding: .utf8) ?? "")
            }
        }
    }

    func waitForExit() -> Int32 {
        process.waitUntilExit()
        return process.terminationStatus
    }
}
