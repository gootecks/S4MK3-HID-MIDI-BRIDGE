import DjaySidebar
import Foundation
import S8Control

extension DjaySidebar: SidebarControlling {}

func report(_ message: String) {
    FileHandle.standardError.write(Data(("S8Host: " + message + "\n").utf8))
}

func bridgePath(_ arguments: [String]) -> String? {
    guard let index = arguments.firstIndex(of: "--bridge"), index + 1 < arguments.count else { return nil }
    return arguments[index + 1]
}

let arguments = Array(CommandLine.arguments.dropFirst())
guard let bridge = bridgePath(arguments) else {
    report("usage: S8Host --bridge <path to s4mk3-hid-midi-bridge>")
    exit(2)
}

guard LiveRoot.isTrusted() else {
    report(
        "Accessibility access is missing. Allow this terminal or app in "
            + "System Settings > Privacy & Security > Accessibility, then start again."
    )
    exit(3)
}

guard LiveRoot.djayIsRunning() else {
    report("djay Pro is not running. Start it, then start S8Host again.")
    exit(3)
}

let child = Process()
child.executableURL = URL(fileURLWithPath: bridge)
child.arguments = ["--device", "s8", "--midi", "full", "--until-stopped", "--app-events"]
let pipe = Pipe()
child.standardOutput = pipe
child.standardError = FileHandle.standardError
child.standardInput = FileHandle.nullDevice

do {
    try child.run()
} catch {
    report("cannot start \(bridge): \(error.localizedDescription)")
    exit(4)
}

func stopChild() {
    if child.isRunning {
        child.terminate()
    }
}

var sources: [DispatchSourceSignal] = []
for signalNumber in [SIGINT, SIGTERM] {
    signal(signalNumber, SIG_IGN)
    let source = DispatchSource.makeSignalSource(signal: signalNumber, queue: .main)
    source.setEventHandler { stopChild() }
    source.resume()
    sources.append(source)
}

Thread.detachNewThread {
    _ = FileHandle.standardInput.readDataToEndOfFile()
    stopChild()
}

let router = ControlRouter(sidebar: DjaySidebar(root: LiveRoot()))

Thread.detachNewThread {
    var buffer = Data()
    let handle = pipe.fileHandleForReading
    while true {
        let chunk = handle.availableData
        if chunk.isEmpty {
            break
        }
        buffer.append(chunk)
        while let newline = buffer.firstIndex(of: 0x0A) {
            let lineData = buffer.subdata(in: buffer.startIndex ..< newline)
            buffer.removeSubrange(buffer.startIndex ... newline)
            let line = String(bytes: lineData, encoding: .utf8) ?? ""
            if !router.handle(line: line) {
                print(line)
                fflush(stdout)
            }
        }
    }
    child.waitUntilExit()
    exit(child.terminationStatus)
}

dispatchMain()
