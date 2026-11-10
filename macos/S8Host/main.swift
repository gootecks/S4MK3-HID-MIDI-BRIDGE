import DjaySidebar
import Foundation
import S8Control
import S8Session

func report(_ message: String) {
    FileHandle.standardError.write(Data(("S8Host: " + message + "\n").utf8))
}

guard let options = HostOptions.parse(Array(CommandLine.arguments.dropFirst())) else {
    FileHandle.standardError.write(Data((HostOptions.usage + "\n").utf8))
    exit(2)
}

if let snapshots = options.replay, let records = options.records {
    do {
        try Replay.run(snapshots: snapshots, records: records)
        exit(0)
    } catch {
        report("replay failed: \(error)")
        exit(1)
    }
}

guard let path = options.bridge else {
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

signal(SIGPIPE, SIG_IGN)
let bridge = Bridge(path: path, screens: options.screens)

do {
    try bridge.start()
} catch {
    report("cannot start \(path): \(error.localizedDescription)")
    exit(4)
}

let session = PanelSession.live(bridge: bridge, options: options)

func stop() {
    session.shutdown()
    bridge.stop()
}

var sources: [DispatchSourceSignal] = []
for signalNumber in [SIGINT, SIGTERM] {
    signal(signalNumber, SIG_IGN)
    let source = DispatchSource.makeSignalSource(signal: signalNumber, queue: .main)
    source.setEventHandler { stop() }
    source.resume()
    sources.append(source)
}

Thread.detachNewThread {
    _ = FileHandle.standardInput.readDataToEndOfFile()
    DispatchQueue.main.async { stop() }
}

session.start()

Thread.detachNewThread {
    bridge.readLines { line in
        if let event = ControlEvent.parse(line) {
            session.handle(event)
        } else {
            print(line)
            fflush(stdout)
        }
    }
    exit(bridge.waitForExit())
}

dispatchMain()
