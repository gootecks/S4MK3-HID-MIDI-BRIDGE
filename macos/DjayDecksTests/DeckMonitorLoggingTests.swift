@testable import DjayDecks
import XCTest

private final class PollLabelSource: DeckAccessibility {
    var values: [String: String] = ["Title, Deck 2": "Soma", "Play / Pause, Deck 2": "Active"]
    var error: DeckAccessError?

    func labeledValues() throws -> [String: String] {
        if let error {
            throw error
        }
        return values
    }
}

private struct PollCatalog: TrackCatalog {
    func entry(title _: String, artist _: String?, length _: TrackLengthHint?) -> CatalogEntry? {
        CatalogEntry(trackID: "soma")
    }
}

final class DeckMonitorLoggingTests: XCTestCase {
    func testPollSeparatesAccessibilityAndLibraryAndReportsPlayState() {
        let source = PollLabelSource()
        var lines: [String] = []
        let monitor = DeckMonitor(source: source, catalog: PollCatalog(), decks: [2], log: { lines.append($0) })
        let now = Date(timeIntervalSince1970: 1_000_000)

        var milliseconds: [UInt64] = [0, 100, 101, 110, 710, 715, 750, 850, 851, 855]
        let uptime = { milliseconds.removeFirst() * 1_000_000 }
        monitor.poll(now: now, uptime: uptime)
        source.values["Play / Pause, Deck 2"] = "Inactive"
        monitor.poll(now: now.addingTimeInterval(0.75), uptime: uptime)

        XCTAssertEqual(lines, [
            "deck poll gap=- read=0.714 ax=0.100 library=0.600 lookup=yes state=playing deck=2",
            "deck poll gap=0.750 read=0.104 ax=0.100 library=- lookup=no state=paused deck=2"
        ])
        XCTAssertTrue(milliseconds.isEmpty)
    }

    func testEmptyAndUnavailableDecksAreNotReportedAsPaused() {
        let source = PollLabelSource()
        source.values = ["Play / Pause, Deck 2": "Inactive"]
        var lines: [String] = []
        let monitor = DeckMonitor(source: source, catalog: PollCatalog(), decks: [2], log: { lines.append($0) })
        let now = Date(timeIntervalSince1970: 1_000_000)

        var milliseconds: [UInt64] = [0, 20, 21, 22, 250, 1000, 1001]
        let uptime = { milliseconds.removeFirst() * 1_000_000 }
        monitor.poll(now: now, uptime: uptime)
        source.error = .notAnswering
        monitor.poll(now: now.addingTimeInterval(0.25), uptime: uptime)

        XCTAssertEqual(lines, [
            "deck poll gap=- read=0.021 ax=0.020 library=- lookup=no state=empty deck=2",
            "deck poll gap=0.250 read=0.751 ax=0.750 library=- lookup=no state=unavailable deck=2"
        ])
        XCTAssertTrue(milliseconds.isEmpty)
    }
}
