import Foundation
import S8Control
import S8Views

public final class PanelSession {
    public struct Sources {
        public var route: (ControlEvent) -> Void
        public var readSidebar: () -> SidebarSnapshot
        public var pollDecks: (() -> Void)?
        public var deckSnapshot: () -> DeckSnapshot?
        public var send: (Data) -> Bool
        public var log: ((String) -> Void)?

        public init(
            route: @escaping (ControlEvent) -> Void,
            readSidebar: @escaping () -> SidebarSnapshot,
            pollDecks: (() -> Void)? = nil,
            deckSnapshot: @escaping () -> DeckSnapshot? = { nil },
            send: @escaping (Data) -> Bool,
            log: ((String) -> Void)? = nil
        ) {
            self.route = route
            self.readSidebar = readSidebar
            self.pollDecks = pollDecks
            self.deckSnapshot = deckSnapshot
            self.send = send
            self.log = log
        }
    }

    public static let deckInterval: TimeInterval = 0.25

    let sidebarQueue = DispatchQueue(label: "S8Session.sidebar")
    let deckQueue = DispatchQueue(label: "S8Session.deck")
    let outputQueue = DispatchQueue(label: "S8Session.output")

    private let sources: Sources
    private let screens: Bool
    private let sidebarInterval: TimeInterval
    private let deckInterval: TimeInterval

    private let lock = NSLock()
    private var waitingEvents = 0
    private var pendingSidebar: SidebarSnapshot?
    private var renderQueued = false
    private var stopped = false
    private var timers: [DispatchSourceTimer] = []

    private var lastDeckPoll: DispatchTime?

    private var panels = LivePanels()
    private var feed = PanelFeed()
    private var state: ViewState?

    public init(
        sources: Sources,
        screens: Bool,
        sidebarInterval: TimeInterval,
        deckInterval: TimeInterval = PanelSession.deckInterval
    ) {
        self.sources = sources
        self.screens = screens
        self.sidebarInterval = sidebarInterval
        self.deckInterval = deckInterval
    }

    public func start() {
        guard screens else { return }
        var started = [timer(on: sidebarQueue, every: sidebarInterval) { $0.publishSidebar() }]
        if let poll = sources.pollDecks {
            started.append(timer(on: deckQueue, every: deckInterval) { $0.pollDecks(poll) })
        }
        let keep = locked { () -> Bool in
            guard !stopped else { return false }
            timers += started
            return true
        }
        for timer in started {
            timer.resume()
            if !keep {
                timer.cancel()
            }
        }
    }

    public func handle(_ event: ControlEvent) {
        locked { waitingEvents += 1 }
        sidebarQueue.async { [self] in
            sources.route(event)
            let waiting = locked { () -> Int in
                waitingEvents -= 1
                return waitingEvents
            }
            if waiting == 0, screens {
                publishSidebar()
            }
        }
    }

    public func shutdown() {
        let running = locked { () -> [DispatchSourceTimer] in
            stopped = true
            defer { timers = [] }
            return timers
        }
        for timer in running {
            timer.cancel()
        }
    }

    private var isStopped: Bool {
        locked { stopped }
    }

    private func timer(
        on queue: DispatchQueue, every interval: TimeInterval, _ work: @escaping (PanelSession) -> Void
    ) -> DispatchSourceTimer {
        let timer = DispatchSource.makeTimerSource(queue: queue)
        timer.schedule(deadline: .now(), repeating: interval)
        timer.setEventHandler { [weak self] in
            if let self {
                work(self)
            }
        }
        return timer
    }

    private func publishSidebar() {
        guard !isStopped else { return }
        requestRender(sources.readSidebar())
    }

    private func pollDecks(_ poll: () -> Void) {
        guard !isStopped else { return }
        let started = DispatchTime.now()
        poll()
        if let log = sources.log {
            let read = Self.seconds(from: started, to: .now())
            let gap = lastDeckPoll.map { String(format: "%.3f", Self.seconds(from: $0, to: started)) } ?? "-"
            log("deck poll gap=\(gap) read=\(String(format: "%.3f", read))")
        }
        lastDeckPoll = started
        requestRender(nil)
    }

    private static func seconds(from start: DispatchTime, to end: DispatchTime) -> Double {
        Double(end.uptimeNanoseconds - start.uptimeNanoseconds) / 1_000_000_000
    }

    private func requestRender(_ snapshot: SidebarSnapshot?) {
        let schedule = locked { () -> Bool in
            guard !stopped else { return false }
            if let snapshot {
                pendingSidebar = snapshot
            }
            guard !renderQueued else { return false }
            renderQueued = true
            return true
        }
        if schedule {
            outputQueue.async { [weak self] in self?.render() }
        }
    }

    private func render() {
        let snapshot = locked { () -> SidebarSnapshot? in
            renderQueued = false
            defer { pendingSidebar = nil }
            return pendingSidebar
        }
        guard !isStopped else { return }
        if let snapshot {
            state = panels.state(for: snapshot)
        }
        guard let state else { return }
        for record in feed.records(state: state, deck: sources.deckSnapshot()) {
            guard !isStopped, sources.send(record) else { return }
        }
    }

    private func locked<T>(_ body: () -> T) -> T {
        lock.lock()
        defer { lock.unlock() }
        return body()
    }
}
