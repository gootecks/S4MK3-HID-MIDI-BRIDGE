@testable import DjayDecks
@testable import DjayLibrary
import S8Views
import XCTest

private struct FakeCatalog: TrackCatalog {
    let entries: [String: CatalogEntry]

    func entry(title: String, artist _: String?, length _: TrackLengthHint?) -> CatalogEntry? {
        entries[title]
    }
}

private final class MutableCatalog: TrackCatalog {
    var current: CatalogEntry?
    private(set) var lookups = 0

    func entry(title _: String, artist _: String?, length _: TrackLengthHint?) -> CatalogEntry? {
        lookups += 1
        return current
    }
}

private final class FakeSource: DeckAccessibility {
    var values: [String: String] = [:]
    var failure: Error?

    func labeledValues() throws -> [String: String] {
        if let failure {
            throw failure
        }
        return values
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
        var tracker = DeckTracker(deck: 1)
        try tracker.ingest(reading(), entry: entry(), now: start)
        let snapshot = tracker.snapshot(now: start)
        XCTAssertEqual(snapshot.title, "Soma")
        XCTAssertEqual(snapshot.artist, "Skrillex")
        XCTAssertEqual(snapshot.bpm, "140.0")
        XCTAssertEqual(snapshot.key, "8A")
        XCTAssertEqual(snapshot.elapsed ?? 0, 60.5, accuracy: 0.001)
        XCTAssertEqual(snapshot.remaining ?? 0, 145.5, accuracy: 0.001)
        XCTAssertEqual(snapshot.duration ?? 0, 204.904, accuracy: 0.001)
        XCTAssertEqual(snapshot.analysis?.beats.count, 478)
        XCTAssertFalse(snapshot.playing)
    }

    func testRemainingOnlyLayoutDerivesElapsedFromLibraryDuration() throws {
        var tracker = DeckTracker(deck: 1)
        try tracker.ingest(reading(elapsed: nil), entry: entry(), now: start)
        XCTAssertEqual(tracker.snapshot(now: start).elapsed ?? 0, 204.904 - 145.5, accuracy: 0.001)
    }

    func testWithoutCatalogEntryHasNoAnalysisAndNeedsEntry() {
        var tracker = DeckTracker(deck: 2)
        tracker.ingest(reading(), entry: nil, now: start)
        XCTAssertNil(tracker.snapshot(now: start).analysis)
        XCTAssertTrue(tracker.needsEntry)
    }

    func testLateCatalogEntryIsAdopted() throws {
        var tracker = DeckTracker(deck: 1)
        tracker.ingest(reading(), entry: nil, now: start)
        try tracker.ingest(reading(), entry: entry(), now: start.addingTimeInterval(1))
        XCTAssertNotNil(tracker.snapshot(now: start.addingTimeInterval(1)).analysis)
        XCTAssertFalse(tracker.needsEntry)
    }

    func testRefreshedEntryForTheSameTrackKeepsPlayStateAndPosition() throws {
        func run(refreshing: Bool) throws -> DeckSnapshot {
            var tracker = DeckTracker(deck: 1)
            for step in 0 ... 4 {
                tracker.ingest(reading(playing: true), entry: nil, now: start.addingTimeInterval(Double(step) * 0.25))
            }
            XCTAssertTrue(tracker.snapshot(now: start.addingTimeInterval(1.2)).playing)
            let fresh = refreshing ? try entry() : nil
            tracker.ingest(reading(playing: true), entry: fresh, now: start.addingTimeInterval(1.25))
            return tracker.snapshot(now: start.addingTimeInterval(1.5))
        }
        let plain = try run(refreshing: false), refreshed = try run(refreshing: true)
        XCTAssertTrue(refreshed.playing)
        XCTAssertEqual(refreshed.elapsed, plain.elapsed)
        XCTAssertNotNil(refreshed.analysis)
    }

    func testTrackChangeResetsEntry() throws {
        var tracker = DeckTracker(deck: 1)
        try tracker.ingest(reading(), entry: entry(), now: start)
        tracker.ingest(reading(title: "Other"), entry: nil, now: start.addingTimeInterval(1))
        let snapshot = tracker.snapshot(now: start.addingTimeInterval(1))
        XCTAssertEqual(snapshot.title, "Other")
        XCTAssertNil(snapshot.analysis)
    }

    func testPlayingIsDebouncedAndElapsedAdvances() {
        var tracker = DeckTracker(deck: 1)
        tracker.ingest(reading(playing: true), entry: nil, now: start)
        XCTAssertFalse(tracker.snapshot(now: start).playing)
        for step in 1 ... 4 {
            tracker.ingest(reading(playing: true), entry: nil, now: start.addingTimeInterval(Double(step) * 0.25))
        }
        let held = tracker.snapshot(now: start.addingTimeInterval(1.2))
        XCTAssertTrue(held.playing)
        XCTAssertEqual((held.elapsed ?? 0).rounded(.down), 60)
        XCTAssertEqual((held.remaining ?? 0).rounded(.down), 145)
        tracker.ingest(reading(elapsed: "01:01", playing: true), entry: nil, now: start.addingTimeInterval(1.25))
        XCTAssertEqual(tracker.snapshot(now: start.addingTimeInterval(1.5)).elapsed ?? 0, 61.25, accuracy: 0.001)
    }

    func testBlinkingPlayButtonOnPausedDeckHoldsTime() {
        var tracker = DeckTracker(deck: 1)
        for step in 0 ... 40 {
            let now = start.addingTimeInterval(Double(step) * 0.25)
            tracker.ingest(reading(playing: (step / 2).isMultiple(of: 2)), entry: nil, now: now)
            let snapshot = tracker.snapshot(now: now.addingTimeInterval(0.1))
            XCTAssertFalse(snapshot.playing, "step \(step)")
            XCTAssertEqual(snapshot.elapsed ?? 0, 60.5, accuracy: 0.001, "step \(step)")
            XCTAssertEqual(snapshot.remaining ?? 0, 145.5, accuracy: 0.001, "step \(step)")
        }
    }

    func testBlinkSampledOncePerBlinkPeriodHoldsTime() {
        var tracker = DeckTracker(deck: 3)
        for step in 0 ... 8 {
            let now = start.addingTimeInterval(Double(step) * 0.93)
            tracker.ingest(reading(playing: true), entry: nil, now: now)
            let snapshot = tracker.snapshot(now: now.addingTimeInterval(0.5))
            XCTAssertFalse(snapshot.playing, "step \(step)")
            XCTAssertEqual(snapshot.elapsed ?? 0, 60.5, accuracy: 0.001, "step \(step)")
        }
    }

    func testResumeAfterPauseContinuesFromPausedPosition() {
        var tracker = DeckTracker(deck: 1)
        tracker.ingest(reading(), entry: nil, now: start)
        for step in 0 ... 4 {
            let now = start.addingTimeInterval(30 + Double(step) * 0.25)
            tracker.ingest(reading(playing: true), entry: nil, now: now)
        }
        let snapshot = tracker.snapshot(now: start.addingTimeInterval(31))
        XCTAssertTrue(snapshot.playing)
        XCTAssertEqual((snapshot.elapsed ?? 0).rounded(.down), 60)
        XCTAssertGreaterThan(snapshot.elapsed ?? 0, 60.5)
        XCTAssertEqual((snapshot.remaining ?? 0).rounded(.down), 145)
        XCTAssertLessThan(snapshot.remaining ?? 0, 145.5)
    }

    func testLabelTickWhileLitConfirmsPlayingAtTheSecondBoundary() {
        var tracker = DeckTracker(deck: 1)
        tracker.ingest(reading(), entry: nil, now: start)
        tracker.ingest(reading(playing: true), entry: nil, now: start.addingTimeInterval(30))
        tracker.ingest(reading(playing: true), entry: nil, now: start.addingTimeInterval(30.25))
        XCTAssertFalse(tracker.snapshot(now: start.addingTimeInterval(30.25)).playing)
        tracker.ingest(reading(elapsed: "01:01", playing: true), entry: nil, now: start.addingTimeInterval(30.5))
        let confirmed = tracker.snapshot(now: start.addingTimeInterval(30.5))
        XCTAssertTrue(confirmed.playing)
        XCTAssertEqual(confirmed.elapsed ?? 0, 61)
        XCTAssertEqual(tracker.snapshot(now: start.addingTimeInterval(31)).elapsed ?? 0, 61.5, accuracy: 0.001)
    }

    func testEmptyDeckAndFailureNotice() {
        var tracker = DeckTracker(deck: 3)
        tracker.ingest(DeckReading(), entry: nil, now: start)
        XCTAssertFalse(tracker.snapshot(now: start).hasTrack)
        tracker.ingestFailure("Accessibility permission needed")
        XCTAssertEqual(tracker.snapshot(now: start).notice, "Accessibility permission needed")
    }

    func testMonitorPollsSourceAndResolvesCatalogOnce() throws {
        let source = FakeSource()
        source.values = [
            "Title, Deck 1": "Soma", "Artist, Deck 1": "Skrillex", "Elapsed time, Deck 1": "00:30",
            "Remaining time, Deck 1": "-02:55", "Key, Deck 1": "8A",
            "Title, Deck 4": "Scut 2", "Remaining time, Deck 4": "-02:58"
        ]
        let monitor = try DeckMonitor(
            source: source, catalog: FakeCatalog(entries: ["Soma": entry()]), decks: [1, 2, 4]
        )
        monitor.poll(now: start)
        let first = monitor.snapshot(deck: 1, now: start)
        XCTAssertEqual(first.title, "Soma")
        XCTAssertNotNil(first.analysis)
        XCTAssertFalse(monitor.snapshot(deck: 2, now: start).hasTrack)
        let fourth = monitor.snapshot(deck: 4, now: start)
        XCTAssertEqual(fourth.title, "Scut 2")
        XCTAssertEqual(fourth.remaining ?? 0, 178.5, accuracy: 0.001)
        XCTAssertEqual(monitor.snapshot(deck: 9, now: start).deck, 9)
    }

    private func loadedSource() -> FakeSource {
        let source = FakeSource()
        source.values = [
            "Title, Deck 1": "Soma", "Artist, Deck 1": "Skrillex", "Elapsed time, Deck 1": "00:30",
            "Remaining time, Deck 1": "-02:55"
        ]
        return source
    }

    func testMonitorPicksUpAnalysisThatAppearsLaterAfterTheLookInterval() throws {
        let catalog = MutableCatalog()
        catalog.current = CatalogEntry(trackID: DeckFixtures.trackID, duration: 204.904)
        let monitor = DeckMonitor(source: loadedSource(), catalog: catalog, decks: [1], lookInterval: 5)
        monitor.poll(now: start)
        XCTAssertNil(monitor.snapshot(deck: 1, now: start).analysis)
        catalog.current = try entry()
        monitor.poll(now: start.addingTimeInterval(4.75))
        XCTAssertNil(monitor.snapshot(deck: 1, now: start.addingTimeInterval(4.75)).analysis)
        XCTAssertEqual(catalog.lookups, 1)
        monitor.poll(now: start.addingTimeInterval(5))
        XCTAssertNotNil(monitor.snapshot(deck: 1, now: start.addingTimeInterval(5)).analysis)
        XCTAssertEqual(catalog.lookups, 2)
    }

    func testMonitorPicksUpChangedCuePoints() {
        let catalog = MutableCatalog()
        catalog.current = CatalogEntry(trackID: DeckFixtures.trackID, duration: 204.904)
        let monitor = DeckMonitor(source: loadedSource(), catalog: catalog, decks: [1], lookInterval: 5)
        monitor.poll(now: start)
        XCTAssertTrue(monitor.snapshot(deck: 1, now: start).cues.isEmpty)
        let cue = DjayCuePoint(time: 90, endTime: nil, name: nil, colorHex: nil, number: nil)
        catalog.current = CatalogEntry(trackID: DeckFixtures.trackID, duration: 204.904, cues: [cue])
        monitor.poll(now: start.addingTimeInterval(5))
        XCTAssertEqual(monitor.snapshot(deck: 1, now: start.addingTimeInterval(5)).cues.map(\.time), [90])
        catalog.current = CatalogEntry(trackID: DeckFixtures.trackID, duration: 204.904)
        monitor.poll(now: start.addingTimeInterval(10))
        XCTAssertTrue(monitor.snapshot(deck: 1, now: start.addingTimeInterval(10)).cues.isEmpty)
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

    func testUnansweredDjayKeepsTrackAndResumesWithoutNewLookup() throws {
        let source = loadedSource()
        let catalog = MutableCatalog()
        catalog.current = try entry()
        let monitor = DeckMonitor(source: source, catalog: catalog, decks: [1], lookInterval: 5)
        monitor.poll(now: start)
        source.failure = DeckAccessError.notAnswering
        monitor.poll(now: start.addingTimeInterval(1))
        let paused = monitor.snapshot(deck: 1, now: start.addingTimeInterval(1))
        XCTAssertEqual(paused.notice, "djay is not answering")
        XCTAssertFalse(paused.hasTrack)
        source.failure = nil
        source.values["Elapsed time, Deck 1"] = "00:32"
        source.values["Remaining time, Deck 1"] = "-02:53"
        monitor.poll(now: start.addingTimeInterval(2))
        let resumed = monitor.snapshot(deck: 1, now: start.addingTimeInterval(2))
        XCTAssertNil(resumed.notice)
        XCTAssertEqual(resumed.title, "Soma")
        XCTAssertNotNil(resumed.analysis)
        XCTAssertEqual(resumed.elapsed ?? 0, 32.5, accuracy: 0.001)
        XCTAssertFalse(resumed.playing)
        XCTAssertEqual(catalog.lookups, 1)
    }

    func testPermissionFailureStillResetsTheDeck() throws {
        let source = FakeSource()
        source.values = ["Title, Deck 1": "Soma", "Remaining time, Deck 1": "-02:55"]
        let catalog = MutableCatalog()
        catalog.current = try entry()
        let monitor = DeckMonitor(source: source, catalog: catalog, decks: [1])
        monitor.poll(now: start)
        source.failure = DeckAccessError.permissionDenied
        monitor.poll(now: start)
        source.failure = nil
        monitor.poll(now: start)
        XCTAssertEqual(catalog.lookups, 2)
    }
}
