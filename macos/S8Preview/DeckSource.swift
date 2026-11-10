import DjayDecks
import Foundation
import S8Views

protocol DeckFeed {
    func current() -> DeckSnapshot
}

struct RecordedDeckFeed: DeckFeed {
    let snapshot: DeckSnapshot

    func current() -> DeckSnapshot {
        snapshot
    }
}

final class LiveDeckFeed: DeckFeed {
    private let monitor: DeckMonitor
    private let deck: Int
    private let queue = DispatchQueue(label: "S8Preview.deck-poll")
    private var timer: DispatchSourceTimer?

    init(deck: Int) {
        self.deck = deck
        monitor = DeckMonitor.live(decks: [deck])
    }

    func start(interval: TimeInterval = 0.25) {
        let timer = DispatchSource.makeTimerSource(queue: queue)
        timer.schedule(deadline: .now(), repeating: interval)
        timer.setEventHandler { [monitor] in monitor.poll() }
        timer.resume()
        self.timer = timer
    }

    func settle(polls: Int, interval: TimeInterval) {
        for index in 0 ..< polls {
            monitor.poll()
            if index < polls - 1 {
                Thread.sleep(forTimeInterval: interval)
            }
        }
    }

    func current() -> DeckSnapshot {
        monitor.snapshot(deck: deck)
    }
}

func dumpDeck(_ snapshot: DeckSnapshot, options: Options) {
    let bitmap = DeckPanelRenderer.render(snapshot)
    let prefix = "deck-\(snapshot.deck)"
    if let dir = options.framesDir, let frame = S8Frame.make(screen: .right, bitmap: bitmap) {
        try? FileManager.default.createDirectory(atPath: dir, withIntermediateDirectories: true)
        try? Data(frame).write(to: URL(fileURLWithPath: "\(dir)/\(prefix).s8frame"))
    }
    if let dir = options.pngDir {
        try? FileManager.default.createDirectory(atPath: dir, withIntermediateDirectories: true)
        bitmap.writePNG(to: URL(fileURLWithPath: "\(dir)/\(prefix).png"))
        try? snapshot.encoded().write(to: URL(fileURLWithPath: "\(dir)/\(prefix).json"))
    }
}
