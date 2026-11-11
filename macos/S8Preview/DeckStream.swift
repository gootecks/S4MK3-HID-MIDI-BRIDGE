import DjayDecks
import Foundation
import S8Views

struct StreamTarget: Equatable {
    let side: PanelSide
    let deck: Int

    var sideName: String {
        side == .left ? "left" : "right"
    }
}

func parseStreamTargets(_ text: String) -> [StreamTarget]? {
    var targets: [StreamTarget] = []
    for part in text.split(separator: ",") {
        let pair = part.split(separator: "=")
        guard pair.count == 2, let deck = Int(pair[1]), (1 ... 4).contains(deck) else { return nil }
        let side: PanelSide
        switch pair[0] {
        case "left": side = .left
        case "right": side = .right
        default: return nil
        }
        guard !targets.contains(where: { $0.side == side }) else { return nil }
        targets.append(StreamTarget(side: side, deck: deck))
    }
    return targets.isEmpty ? nil : targets
}

struct StreamRequest {
    let targets: [StreamTarget]
    let seconds: Double?

    init?(arguments: [String]) {
        guard arguments.count == 2 || arguments.count == 4, arguments[0] == "--stream",
              let targets = parseStreamTargets(arguments[1])
        else { return nil }
        self.targets = targets
        if arguments.count == 4 {
            guard arguments[2] == "--seconds", let seconds = Double(arguments[3]), seconds > 0 else { return nil }
            self.seconds = seconds
        } else {
            seconds = nil
        }
    }
}

private func logLine(_ text: String) {
    FileHandle.standardError.write(Data((text + "\n").utf8))
}

private func describe(_ snapshot: DeckSnapshot) -> String {
    func seconds(_ value: Double?) -> String {
        value.map { String(format: "%.2f", $0) } ?? "-"
    }
    return "deck=\(snapshot.deck) elapsed=\(seconds(snapshot.elapsed)) remaining=\(seconds(snapshot.remaining))"
        + " playing=\(snapshot.playing) title=\(snapshot.title ?? "-")"
}

func runDeckStream(targets: [StreamTarget], seconds: Double?) -> Never {
    signal(SIGPIPE, SIG_IGN)
    let monitor = DeckMonitor.live(decks: Array(Set(targets.map(\.deck))).sorted())
    monitor.poll()
    if let failure = monitor.failure {
        logLine("DECKSTREAM refused: \(failure)")
        exit(1)
    }

    let pollQueue = DispatchQueue(label: "S8Preview.deck-stream-poll")
    let timer = DispatchSource.makeTimerSource(queue: pollQueue)
    timer.schedule(deadline: .now() + 0.25, repeating: 0.25)
    timer.setEventHandler { monitor.poll() }
    timer.resume()

    let output = FileHandle.standardOutput
    let start = Date()
    var sent: [UInt8: Bitmap] = [:]
    var records = 0
    logLine(String(format: "DECKSTREAM start unix=%.3f", start.timeIntervalSince1970))
    while seconds.map({ Date().timeIntervalSince(start) < $0 }) ?? true {
        for target in targets {
            let snapshot = monitor.snapshot(deck: target.deck)
            let bitmap = DeckPanelRenderer.render(snapshot)
            guard sent[target.side.rawValue] != bitmap,
                  let record = S8Record.make(panel: target.side, bitmap: bitmap)
            else { continue }
            do {
                try output.write(contentsOf: record)
            } catch {
                logLine("DECKSTREAM stopped: the reader of stdout is gone after \(records) records")
                exit(0)
            }
            sent[target.side.rawValue] = bitmap
            records += 1
            logLine(String(
                format: "DECKSTREAM %.3f panel=%@ %@", Date().timeIntervalSince(start), target.sideName,
                describe(snapshot)
            ))
        }
        Thread.sleep(forTimeInterval: 0.1)
    }
    timer.cancel()
    try? output.close()
    logLine("DECKSTREAM end records=\(records)")
    exit(0)
}
