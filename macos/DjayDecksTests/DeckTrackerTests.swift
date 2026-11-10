@testable import DjayDecks
import DjayLibrary
import S8Views
import XCTest

private struct FakeCatalog: TrackCatalog {
    let entries: [String: CatalogEntry]

    func entry(title: String, artist _: String?) -> CatalogEntry? {
        entries[title]
    }
}

private final class FakeSource: DeckAccessibility {
    var values: [String: [String: String]] = [:]
    var failure: Error?

    func labeledValues(prefix: String) throws -> [String: String] {
        if let failure {
            throw failure
        }
        return values[prefix] ?? [:]
    }
}

final class DeckTrackerTests: XCTestCase {
    private let start = Date(timeIntervalSince1970: 1_000_000)

    private func reading(title: String = "Soma", elapsed: String? = "01:00", playing: Bool = false) -> DeckReading {
        DeckReading(
            key: "8A", title: title, artist: "Skrillex", bpm: "140.0", elapsedTime: elapsed,
            remainingTime: "-02:25", isPlaying: playing
        )
    }

    private func entry() throws -> CatalogEntry {
        let analysis = try TrackAnalysis.load(url: DeckFixtures.url())
        return CatalogEntry(
            trackID: DeckFixtures.trackID, duration: 204.904, cues: [], analysis: analysis
        )
    }

    func testSnapshotCarriesAXFieldsAndAnalysis() throws {
        var tracker = DeckTracker(deck: 1, now: start)
        try tracker.ingest(reading(), entry: entry(), now: start)
        let snapshot = tracker.snapshot(now: start)
        XCTAssertEqual(snapshot.title, "Soma")
        XCTAssertEqual(snapshot.artist, "Skrillex")
        XCTAssertEqual(snapshot.bpm, "140.0")
        XCTAssertEqual(snapshot.key, "8A")
        XCTAssertEqual(snapshot.elapsed ?? 0, 60, accuracy: 0.001)
        XCTAssertEqual(snapshot.remaining ?? 0, 145, accuracy: 0.001)
        XCTAssertEqual(snapshot.duration ?? 0, 204.904, accuracy: 0.001)
        XCTAssertEqual(snapshot.analysis?.beats.count, 478)
        XCTAssertFalse(snapshot.playing)
    }

    func testRemainingOnlyLayoutDerivesElapsedFromLibraryDuration() throws {
        var tracker = DeckTracker(deck: 1, now: start)
        try tracker.ingest(reading(elapsed: nil), entry: entry(), now: start)
        XCTAssertEqual(tracker.snapshot(now: start).elapsed ?? 0, 204.904 - 145, accuracy: 0.001)
    }

    func testWithoutCatalogEntryHasNoAnalysisAndNeedsEntry() {
        var tracker = DeckTracker(deck: 2, now: start)
        tracker.ingest(reading(), entry: nil, now: start)
        XCTAssertNil(tracker.snapshot(now: start).analysis)
        XCTAssertTrue(tracker.needsEntry)
    }

    func testLateCatalogEntryIsAdopted() throws {
        var tracker = DeckTracker(deck: 1, now: start)
        tracker.ingest(reading(), entry: nil, now: start)
        try tracker.ingest(reading(), entry: entry(), now: start.addingTimeInterval(1))
        XCTAssertNotNil(tracker.snapshot(now: start.addingTimeInterval(1)).analysis)
        XCTAssertFalse(tracker.needsEntry)
    }

    func testTrackChangeResetsEntry() throws {
        var tracker = DeckTracker(deck: 1, now: start)
        try tracker.ingest(reading(), entry: entry(), now: start)
        tracker.ingest(reading(title: "Other"), entry: nil, now: start.addingTimeInterval(1))
        let snapshot = tracker.snapshot(now: start.addingTimeInterval(1))
        XCTAssertEqual(snapshot.title, "Other")
        XCTAssertNil(snapshot.analysis)
    }

    func testPlayingIsDebouncedAndElapsedAdvances() {
        var tracker = DeckTracker(deck: 1, now: start)
        tracker.ingest(reading(playing: true), entry: nil, now: start)
        XCTAssertFalse(tracker.snapshot(now: start).playing)
        tracker.ingest(reading(playing: true), entry: nil, now: start.addingTimeInterval(1))
        let snapshot = tracker.snapshot(now: start.addingTimeInterval(1.5))
        XCTAssertTrue(snapshot.playing)
        XCTAssertEqual(snapshot.elapsed ?? 0, 61.5, accuracy: 0.001)
    }

    func testEmptyDeckAndFailureNotice() {
        var tracker = DeckTracker(deck: 3, now: start)
        tracker.ingest(DeckReading(), entry: nil, now: start)
        XCTAssertFalse(tracker.snapshot(now: start).hasTrack)
        tracker.ingestFailure("Accessibility permission needed")
        XCTAssertEqual(tracker.snapshot(now: start).notice, "Accessibility permission needed")
    }

    func testMonitorPollsSourceAndResolvesCatalogOnce() throws {
        let source = FakeSource()
        source.values["Deck 1"] = [
            "Title, Deck 1": "Soma", "Artist, Deck 1": "Skrillex", "Elapsed time, Deck 1": "00:30",
            "Remaining time, Deck 1": "-02:55", "Key, Deck 1": "8A"
        ]
        let monitor = try DeckMonitor(
            source: source, catalog: FakeCatalog(entries: ["Soma": entry()]), decks: [1, 2]
        )
        monitor.poll(now: start)
        let first = monitor.snapshot(deck: 1, now: start)
        XCTAssertEqual(first.title, "Soma")
        XCTAssertNotNil(first.analysis)
        XCTAssertFalse(monitor.snapshot(deck: 2, now: start).hasTrack)
        XCTAssertEqual(monitor.snapshot(deck: 9, now: start).deck, 9)
    }

    func testMonitorReportsPermissionAndMissingApp() {
        let source = FakeSource()
        let monitor = DeckMonitor(source: source, catalog: FakeCatalog(entries: [:]), decks: [1])
        source.failure = DeckAccessError.permissionDenied
        monitor.poll(now: start)
        XCTAssertEqual(monitor.snapshot(deck: 1, now: start).notice, "Accessibility permission needed")
        source.failure = DeckAccessError.notRunning
        monitor.poll(now: start)
        XCTAssertEqual(monitor.snapshot(deck: 1, now: start).notice, "djay is not running")
    }
}
