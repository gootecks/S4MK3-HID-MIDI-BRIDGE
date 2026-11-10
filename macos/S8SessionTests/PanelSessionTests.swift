import S8Control
@testable import S8Session
import S8Views
import XCTest

private func sidebar(_ title: String) -> SidebarSnapshot {
    SidebarSnapshot(
        tabs: [.init(id: "library", label: "Library", selected: true)],
        rows: [.init(kind: .track, title: title, artist: "Artist")]
    )
}

private final class Gate {
    private let condition = NSCondition()
    private var isOpen = false

    func open() {
        condition.lock()
        isOpen = true
        condition.broadcast()
        condition.unlock()
    }

    func pass() {
        condition.lock()
        while !isOpen {
            condition.wait()
        }
        condition.unlock()
    }
}

private final class Recorder<Value> {
    private let lock = NSLock()
    private var stored: [Value] = []

    @discardableResult
    func append(_ value: Value) -> Int {
        lock.lock()
        defer { lock.unlock() }
        stored.append(value)
        return stored.count - 1
    }

    var values: [Value] {
        lock.lock()
        defer { lock.unlock() }
        return stored
    }
}

private final class Trigger {
    private let lock = NSLock()
    private var expectation: XCTestExpectation?

    func arm(_ armed: XCTestExpectation) {
        lock.lock()
        expectation = armed
        lock.unlock()
    }

    func fire() {
        lock.lock()
        let armed = expectation
        lock.unlock()
        armed?.fulfill()
    }
}

final class PanelSessionTests: XCTestCase {
    private func started(_ description: String) -> XCTestExpectation {
        let expectation = expectation(description: description)
        expectation.assertForOverFulfill = false
        return expectation
    }

    private func waitForDeckPolls(_ polls: Trigger, while blocked: String) {
        let polled = started("decks polled while \(blocked)")
        polled.expectedFulfillmentCount = 5
        polls.arm(polled)
        wait(for: [polled], timeout: 5)
    }

    func testDeckPollingContinuesWhileASidebarReadIsBlocked() {
        let gate = Gate()
        let reading = started("sidebar read started")
        let polls = Trigger()
        let session = PanelSession(
            sources: .init(
                route: { _ in },
                readSidebar: {
                    reading.fulfill()
                    gate.pass()
                    return sidebar("A")
                },
                pollDecks: { polls.fire() },
                send: { _ in true }
            ),
            screens: true, sidebarInterval: 0.01, deckInterval: 0.01
        )
        session.start()
        wait(for: [reading], timeout: 5)
        waitForDeckPolls(polls, while: "the sidebar read is blocked")
        session.shutdown()
        gate.open()
    }

    func testDeckPollingContinuesWhileOutputIsBlocked() {
        let gate = Gate()
        let sending = started("first record is being written")
        let polls = Trigger()
        let session = PanelSession(
            sources: .init(
                route: { _ in },
                readSidebar: { sidebar("A") },
                pollDecks: { polls.fire() },
                send: { _ in
                    sending.fulfill()
                    gate.pass()
                    return true
                }
            ),
            screens: true, sidebarInterval: 0.01, deckInterval: 0.01
        )
        session.start()
        wait(for: [sending], timeout: 5)
        waitForDeckPolls(polls, while: "the output is blocked")
        session.shutdown()
        gate.open()
    }

    func testBlockedOutputConvergesToTheNewestSidebarWithoutABacklog() {
        let last = 30
        var panels = LivePanels()
        var feed = PanelFeed()
        let expected = feed.records(state: panels.state(for: sidebar("Track 0")))
            + feed.records(state: panels.state(for: sidebar("Track \(last)")))
        XCTAssertEqual(expected.count, 3)

        let output = Gate()
        let firstSend = Gate()
        let reads = Recorder<Void>()
        let sent = Recorder<Data>()
        let allPublished = started("every sidebar state was published")
        let newestSent = started("the newest state was written")
        let session = PanelSession(
            sources: .init(
                route: { _ in },
                readSidebar: {
                    let index = reads.append(())
                    if index > 0 {
                        firstSend.pass()
                    }
                    if index > last {
                        allPublished.fulfill()
                    }
                    return sidebar("Track \(min(index, last))")
                },
                send: { record in
                    firstSend.open()
                    output.pass()
                    sent.append(record)
                    if record == expected[2] {
                        newestSent.fulfill()
                    }
                    return true
                }
            ),
            screens: true, sidebarInterval: 0.001
        )
        session.start()
        wait(for: [allPublished], timeout: 10)
        output.open()
        wait(for: [newestSent], timeout: 10)
        session.shutdown()
        session.outputQueue.sync {}
        XCTAssertEqual(sent.values, expected)
    }

    func testShutdownReturnsWhileAWriteIsBlockedAndStopsReadsAndSends() {
        let gate = Gate()
        let sending = started("first record is being written")
        let reads = Recorder<Void>()
        let sent = Recorder<Data>()
        let session = PanelSession(
            sources: .init(
                route: { _ in },
                readSidebar: {
                    reads.append(())
                    return sidebar("A")
                },
                send: { record in
                    sent.append(record)
                    sending.fulfill()
                    gate.pass()
                    return true
                }
            ),
            screens: true, sidebarInterval: 0.01
        )
        session.start()
        wait(for: [sending], timeout: 5)

        let returned = expectation(description: "shutdown returned while the write was blocked")
        DispatchQueue.global().async {
            session.shutdown()
            returned.fulfill()
        }
        wait(for: [returned], timeout: 5)
        session.sidebarQueue.sync {}
        let readsAtShutdown = reads.values.count

        gate.open()
        session.handle(.displayButton(side: .left, slot: 2, down: true))
        session.sidebarQueue.sync {}
        session.outputQueue.sync {}
        XCTAssertEqual(sent.values.count, 1)
        XCTAssertEqual(reads.values.count, readsAtShutdown)
    }

    func testSidebarIsReadOnceAfterTheLastWaitingEvent() {
        let gate = Gate()
        let routing = started("first event is being routed")
        let routed = Recorder<ControlEvent>()
        let reads = Recorder<Void>()
        let session = PanelSession(
            sources: .init(
                route: { event in
                    routed.append(event)
                    routing.fulfill()
                    gate.pass()
                },
                readSidebar: {
                    reads.append(())
                    return sidebar("A")
                },
                send: { _ in true }
            ),
            screens: true, sidebarInterval: 3600
        )
        let events = (1 ... 5).map { ControlEvent.screenEncoder(side: .left, encoder: 1, delta: $0) }
        events.forEach(session.handle)
        wait(for: [routing], timeout: 5)
        gate.open()
        session.sidebarQueue.sync {}
        XCTAssertEqual(routed.values, events)
        XCTAssertEqual(reads.values.count, 1)
    }
}
