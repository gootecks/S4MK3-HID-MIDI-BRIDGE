import DjayDecks
@testable import DjayLibrary
import XCTest

private struct Item {
    let id: String
    let artist: String
    let duration: Double?
}

final class LibraryCatalogMatchingTests: XCTestCase {
    private var directory: URL!

    override func setUpWithError() throws {
        directory = FileManager.default.temporaryDirectory.appendingPathComponent("matching-\(UUID().uuidString)")
        try FileManager.default.createDirectory(
            at: directory.appendingPathComponent("a"), withIntermediateDirectories: true
        )
        try Data("db".utf8).write(to: directory.appendingPathComponent("MediaLibrary.db"))
    }

    override func tearDown() {
        try? FileManager.default.removeItem(at: directory)
    }

    private func catalog(_ items: [Item]) -> LibraryCatalog {
        let rows = items.enumerated().map { index, item -> CollectionRow in
            var fields: [String: DjayValue] = [
                "title": .string("Cheeni"), "artistUUIDs": .list([.string("artist-\(item.artist)")])
            ]
            if let duration = item.duration {
                fields["duration"] = .double(duration)
            }
            return CollectionRow(rowid: Int64(index), key: item.id, value: .object(className: nil, fields: fields))
        }
        let artists = Set(items.map(\.artist)).map { name in
            CollectionRow(rowid: 100, key: "artist-\(name)", value: .object(className: nil, fields: [
                "name": .string(name)
            ]))
        }
        let snapshot = DjayLibrarySnapshot(collections: ["mediaItems": rows, "mediaArtists": artists])
        return LibraryCatalog(
            databaseURL: directory.appendingPathComponent("MediaLibrary.db"),
            store: AnalysisStore(root: directory), loader: { _ in snapshot }
        )
    }

    private func both(_ elapsed: String, _ remaining: String) -> TrackLengthHint? {
        TrackLengthHint(elapsedTime: elapsed, remainingTime: remaining)
    }

    private let pair = [
        Item(id: "short", artist: "Alpha", duration: 180), Item(id: "long", artist: "Beta", duration: 240)
    ]

    func testLengthFromBothLabelsPicksTheEntryThatFitsWhenTheArtistMatchesNeither() {
        let library = catalog(pair)
        let fits = both("01:00", "-02:00")
        XCTAssertEqual(library.entry(title: "Cheeni", artist: "Other", length: fits)?.trackID, "short")
        XCTAssertEqual(library.entry(title: "Cheeni", artist: "Other", length: fits)?.duration, 180)
        let longer = both("00:30", "-03:30")
        XCTAssertEqual(library.entry(title: "Cheeni", artist: "Other", length: longer)?.trackID, "long")
    }

    func testRemainingLabelAloneActsAsALowerBound() {
        let library = catalog(pair)
        let hint = TrackLengthHint(elapsedTime: nil, remainingTime: "-03:20")
        XCTAssertEqual(library.entry(title: "Cheeni", artist: "Other", length: hint)?.trackID, "long")
    }

    func testRemainingLabelAloneGivesNoEntryWhenBothLengthsFit() {
        let library = catalog(pair)
        let hint = TrackLengthHint(elapsedTime: nil, remainingTime: "-02:00")
        XCTAssertNil(library.entry(title: "Cheeni", artist: "Other", length: hint))
    }

    func testUnseparableTitleOnlyEntriesGiveNoEntry() {
        let library = catalog(pair)
        XCTAssertNil(library.entry(title: "Cheeni", artist: "Other", length: nil))
    }

    func testNoConsistentLengthGivesNoTitleOnlyEntry() {
        let library = catalog(pair)
        XCTAssertNil(library.entry(title: "Cheeni", artist: "Other", length: both("01:00", "-09:00")))
        XCTAssertNil(catalog([Item(id: "only", artist: "Alpha", duration: 180)]).entry(
            title: "Cheeni", artist: "Other", length: both("01:00", "-09:00")
        ))
    }

    func testTitleOnlyEntryWithoutDurationStaysACandidate() {
        let library = catalog([
            Item(id: "known", artist: "Alpha", duration: 180), Item(id: "unknown", artist: "Beta", duration: nil)
        ])
        let hint = both("01:00", "-02:00")
        XCTAssertEqual(library.entry(title: "Cheeni", artist: "Other", length: hint)?.trackID, "known")
    }

    func testArtistMatchWithAnInconsistentLengthIsStillReturned() {
        let library = catalog(pair)
        XCTAssertEqual(library.entry(title: "Cheeni", artist: "Beta", length: both("01:00", "-09:00"))?.trackID, "long")
    }

    func testArtistMatchIgnoresTitleOnlyEntries() {
        let library = catalog(pair)
        XCTAssertEqual(library.entry(title: "Cheeni", artist: "Alpha", length: nil)?.trackID, "short")
    }

    func testSameTrackTwiceReturnsTheEntryWithAnAnalysis() throws {
        let library = catalog([
            Item(id: "first", artist: "Alpha", duration: 180), Item(id: "second", artist: "Alpha", duration: 180.5)
        ])
        XCTAssertEqual(library.entry(title: "Cheeni", artist: "Other", length: nil)?.trackID, "first")
        try FileManager.default.copyItem(
            at: DeckFixtures.url(), to: directory.appendingPathComponent("a/second.djayMetadata")
        )
        let found = library.entry(title: "Cheeni", artist: "Other", length: nil)
        XCTAssertEqual(found?.trackID, "second")
        XCTAssertNotNil(found?.analysis)
    }

    func testHintFromLabels() {
        XCTAssertEqual(both("00:30", "-02:55"), TrackLengthHint(minimum: 203, maximum: 207))
        XCTAssertEqual(TrackLengthHint(elapsedTime: nil, remainingTime: "-02:55"), TrackLengthHint(minimum: 174))
        XCTAssertNil(TrackLengthHint(elapsedTime: "00:30", remainingTime: nil))
        XCTAssertNil(TrackLengthHint(elapsedTime: nil, remainingTime: nil))
    }
}
