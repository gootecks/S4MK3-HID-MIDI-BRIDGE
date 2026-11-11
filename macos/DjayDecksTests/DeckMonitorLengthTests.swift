@testable import DjayDecks
import XCTest

private struct LabelSource: DeckAccessibility {
    let values: [String: String]

    func labeledValues() throws -> [String: String] {
        values
    }
}

private final class RecordingCatalog: TrackCatalog {
    private(set) var lengths: [TrackLengthHint?] = []

    func entry(title _: String, artist _: String?, length: TrackLengthHint?) -> CatalogEntry? {
        lengths.append(length)
        return nil
    }
}

final class DeckMonitorLengthTests: XCTestCase {
    private func lengths(_ values: [String: String]) -> [TrackLengthHint?] {
        let catalog = RecordingCatalog()
        let monitor = DeckMonitor(source: LabelSource(values: values), catalog: catalog, decks: [1])
        monitor.poll(now: Date(timeIntervalSince1970: 1_000_000))
        return catalog.lengths
    }

    func testLengthFromBothLabelsReachesTheCatalog() {
        let values = [
            "Title, Deck 1": "Soma", "Elapsed time, Deck 1": "00:30", "Remaining time, Deck 1": "-02:55"
        ]
        XCTAssertEqual(lengths(values), [TrackLengthHint(minimum: 203, maximum: 207)])
    }

    func testRemainingLabelAloneReachesTheCatalogAsALowerBound() {
        XCTAssertEqual(
            lengths(["Title, Deck 1": "Soma", "Remaining time, Deck 1": "-02:55"]), [TrackLengthHint(minimum: 174)]
        )
    }

    func testNoTimeLabelsReachTheCatalogAsNoLength() {
        XCTAssertEqual(lengths(["Title, Deck 1": "Soma"]), [nil])
    }
}
