@testable import DjayDecks
import XCTest

private final class CapturedSource: DeckAccessibility {
    var capture: String

    init(_ capture: String) {
        self.capture = capture
    }

    func labeledValues() throws -> [String: String] {
        let url = try XCTUnwrap(Bundle.module.url(forResource: capture, withExtension: "tsv", subdirectory: "Fixtures"))
        var results: [String: String] = [:]
        for line in try String(contentsOf: url, encoding: .utf8).split(separator: "\n") {
            let fields = line.split(separator: "\t", maxSplits: 1, omittingEmptySubsequences: false)
            AXDeckSource.record(String(fields[0]), fields.count > 1 ? String(fields[1]) : "", into: &results)
        }
        return results
    }
}

private struct Labels: DeckAccessibility {
    let values: [String: String]

    func labeledValues() -> [String: String] {
        values
    }
}

private final class TitleCatalog: TrackCatalog {
    private(set) var titles: [String] = []

    func entry(title: String, artist _: String?, length _: TrackLengthHint?) -> CatalogEntry? {
        titles.append(title)
        return CatalogEntry(trackID: title, duration: 185)
    }
}

final class HiddenDeckTests: XCTestCase {
    private let start = Date(timeIntervalSince1970: 1_000_000)

    func testOneDeckViewReadsTheDeckShownAndNamesTheHiddenOne() {
        let monitor = DeckMonitor(source: CapturedSource("one-deck-view-deck-2"), catalog: TitleCatalog())
        monitor.poll(now: start)
        let hidden = monitor.snapshot(deck: 1, now: start)
        XCTAssertEqual(hidden.notice, "Deck 1 is not shown in djay")
        XCTAssertFalse(hidden.hasTrack)
        let shown = monitor.snapshot(deck: 2, now: start)
        XCTAssertNil(shown.notice)
        XCTAssertEqual(shown.title, "Cheeni")
        XCTAssertEqual(shown.artist, "Skrillex, Naisha")
        XCTAssertEqual(shown.key, "7 B")
        XCTAssertEqual(shown.bpm, "140.0")
        XCTAssertEqual(shown.remaining ?? 0, 103.5, accuracy: 0.001)
        XCTAssertNil(monitor.failure)
    }

    func testOneDeckViewCanShowDeck1() {
        let monitor = DeckMonitor(source: CapturedSource("one-deck-view-deck-1"), catalog: TitleCatalog())
        monitor.poll(now: start)
        XCTAssertEqual(monitor.snapshot(deck: 1, now: start).title, "Duro")
        XCTAssertEqual(monitor.snapshot(deck: 2, now: start).notice, "Deck 2 is not shown in djay")
    }

    func testHiddenDeckReturnsWithItsTrackAndNoNewLookup() {
        let source = CapturedSource("one-deck-view-two-decks")
        let catalog = TitleCatalog()
        let monitor = DeckMonitor(source: source, catalog: catalog)
        monitor.poll(now: start)
        XCTAssertEqual(monitor.snapshot(deck: 1, now: start).title, "Duro")
        source.capture = "one-deck-view-deck-2"
        monitor.poll(now: start.addingTimeInterval(1))
        let hidden = monitor.snapshot(deck: 1, now: start.addingTimeInterval(1))
        XCTAssertEqual(hidden.notice, "Deck 1 is not shown in djay")
        source.capture = "one-deck-view-two-decks"
        monitor.poll(now: start.addingTimeInterval(2))
        let back = monitor.snapshot(deck: 1, now: start.addingTimeInterval(2))
        XCTAssertNil(back.notice)
        XCTAssertEqual(back.title, "Duro")
        XCTAssertEqual(back.duration, 185)
        XCTAssertEqual(catalog.titles, ["Duro", "Cheeni"])
    }

    func testTwoDecksViewDoesNotShowDecks3And4() {
        let monitor = DeckMonitor(
            source: CapturedSource("one-deck-view-two-decks"), catalog: TitleCatalog(), decks: [3, 4]
        )
        monitor.poll(now: start)
        XCTAssertEqual(monitor.snapshot(deck: 3, now: start).notice, "Deck 3 is not shown in djay")
        XCTAssertEqual(monitor.snapshot(deck: 4, now: start).notice, "Deck 4 is not shown in djay")
        XCTAssertNil(monitor.failure)
    }

    func testDeckShownWithoutTrackHasNoNotice() {
        var values: [String: String] = [:]
        AXDeckSource.record("Title, Deck 1", "", into: &values)
        AXDeckSource.record("Play / Pause, Deck 1", "", into: &values)
        let catalog = TitleCatalog()
        let monitor = DeckMonitor(source: Labels(values: values), catalog: catalog, decks: [1])
        monitor.poll(now: start)
        let snapshot = monitor.snapshot(deck: 1, now: start)
        XCTAssertNil(snapshot.notice)
        XCTAssertFalse(snapshot.hasTrack)
        XCTAssertEqual(catalog.titles, [])
    }

    func testEmptyValueDoesNotReplaceAValueReadForTheSameLabel() {
        var values: [String: String] = [:]
        AXDeckSource.record("Title, Deck 1", "", into: &values)
        AXDeckSource.record("Title, Deck 1", "Duro", into: &values)
        AXDeckSource.record("Title, Deck 1", "", into: &values)
        XCTAssertEqual(values, ["Title, Deck 1": "Duro"])
    }
}
