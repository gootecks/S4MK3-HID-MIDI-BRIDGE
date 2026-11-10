import DjayDecks
import DjaySidebar
import Foundation
import S8Control
import S8Views

extension DjaySidebar: SidebarControlling {}

final class LiveSession {
    private static let deckInterval: TimeInterval = 0.25

    private let bridge: Bridge
    private let options: HostOptions
    private let sidebar = DjaySidebar(root: LiveRoot())
    private let router: ControlRouter
    private let monitor: DeckMonitor?
    private let queue = DispatchQueue(label: "S8Host.work")
    private let lock = NSLock()
    private var waiting = 0
    private var timers: [DispatchSourceTimer] = []
    private var panels = LivePanels()
    private var feed = PanelFeed()
    private var state: ViewState?

    init(bridge: Bridge, options: HostOptions) {
        self.bridge = bridge
        self.options = options
        router = ControlRouter(sidebar: sidebar)
        monitor = options.deck.map { DeckMonitor.live(decks: [$0]) }
    }

    func start() {
        guard options.screens else { return }
        every(options.poll) { [unowned self] in draw(readSidebar: true) }
        if monitor != nil {
            every(Self.deckInterval) { [unowned self] in draw(readSidebar: false) }
        }
    }

    func handle(line: String) -> Bool {
        guard let event = ControlEvent.parse(line) else { return false }
        count(1)
        queue.async { [self] in
            router.handle(event)
            if count(-1) == 0, options.screens {
                draw(readSidebar: true)
            }
        }
        return true
    }

    @discardableResult
    private func count(_ change: Int) -> Int {
        lock.lock()
        defer { lock.unlock() }
        waiting += change
        return waiting
    }

    private func every(_ interval: TimeInterval, _ work: @escaping () -> Void) {
        let timer = DispatchSource.makeTimerSource(queue: queue)
        timer.schedule(deadline: .now(), repeating: interval)
        timer.setEventHandler(handler: work)
        timer.resume()
        timers.append(timer)
    }

    private func draw(readSidebar: Bool) {
        if readSidebar || state == nil {
            state = panels.state(for: sidebar.snapshot())
        }
        guard let state else { return }
        monitor?.poll()
        let deck = options.deck.flatMap { monitor?.snapshot(deck: $0) }
        for record in feed.records(state: state, deck: deck) where !bridge.send(record) {
            return
        }
    }
}
