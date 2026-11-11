import DjayDecks
@testable import DjayLibrary
import XCTest

private final class Clock {
    var value = Date(timeIntervalSince1970: 2_000_000)
    func advance(_ seconds: TimeInterval) {
        value = value.addingTimeInterval(seconds)
    }
}

private final class Library {
    private let lock = NSLock()
    private var loads = 0
    private var cueTimes: [Double] = []
    private var failing = false

    var loadCount: Int {
        locked { loads }
    }

    func set(cues: [Double]) {
        locked { cueTimes = cues }
    }

    func set(failing value: Bool) {
        locked { failing = value }
    }

    func load(_: URL) throws -> DjayLibrarySnapshot {
        let (cues, fail): ([Double], Bool) = locked {
            loads += 1
            return (cueTimes, failing)
        }
        if fail {
            throw DjayDecodeError.database("copy failed")
        }
        let user = DjayValue.object(className: nil, fields: [
            "cuePoints": .list(cues.map { .object(className: "ADCCuePoint", fields: ["time": .double($0)]) })
        ])
        return DjayLibrarySnapshot(collections: [
            "mediaItems": [CollectionRow(rowid: 1, key: DeckFixtures.trackID, value: .object(
                className: nil, fields: ["title": .string("Soma"), "artistUUIDs": .list([.string("a")])]
            ))],
            "mediaArtists": [CollectionRow(rowid: 2, key: "a", value: .object(
                className: nil, fields: ["name": .string("Skrillex")]
            ))],
            "mediaItemUserData": [CollectionRow(rowid: 3, key: DeckFixtures.trackID, value: user)]
        ])
    }

    private func locked<T>(_ body: () -> T) -> T {
        lock.lock()
        defer { lock.unlock() }
        return body()
    }
}

final class LibraryCatalogTests: XCTestCase {
    private var directory: URL!
    private var database: URL!
    private var metadata: URL!
    private let clock = Clock()
    private let library = Library()
    private var reloaded: XCTestExpectation?
    private var catalog: LibraryCatalog!

    override func setUpWithError() throws {
        directory = FileManager.default.temporaryDirectory.appendingPathComponent("catalog-\(UUID().uuidString)")
        metadata = directory.appendingPathComponent("Metadata")
        try FileManager.default.createDirectory(
            at: metadata.appendingPathComponent("a"), withIntermediateDirectories: true
        )
        database = directory.appendingPathComponent("MediaLibrary.db")
        try Data("one".utf8).write(to: database)
        catalog = LibraryCatalog(
            databaseURL: database, store: AnalysisStore(root: metadata), checkInterval: 5,
            now: { [clock] in clock.value }, loader: library.load, didReload: { [weak self] _ in
                self?.reloaded?.fulfill()
            }
        )
    }

    override func tearDown() {
        try? FileManager.default.removeItem(at: directory)
    }

    private func lookup() -> CatalogEntry? {
        catalog.entry(title: "Soma", artist: "Skrillex", length: nil)
    }

    private func changeDatabase() throws {
        try Data("changed-\(UUID().uuidString)".utf8).write(to: database)
    }

    private func awaitReload(_ trigger: () -> Void) {
        reloaded = expectation(description: "reload")
        trigger()
        wait(for: [reloaded!], timeout: 5)
    }

    private func assertNoReload(_ trigger: () -> Void) {
        let none = expectation(description: "no reload")
        none.isInverted = true
        reloaded = none
        trigger()
        wait(for: [none], timeout: 0.3)
    }

    func testAnalysisCreatedAfterTheFirstLookIsFound() throws {
        XCTAssertNil(lookup()?.analysis)
        XCTAssertNotNil(lookup())
        try FileManager.default.copyItem(
            at: DeckFixtures.url(),
            to: metadata.appendingPathComponent("a/\(DeckFixtures.trackID).djayMetadata")
        )
        XCTAssertNotNil(lookup()?.analysis)
        XCTAssertEqual(library.loadCount, 1)
    }

    func testCuePointWrittenAfterTheFirstLoadAppearsAfterTheReload() throws {
        library.set(cues: [10])
        XCTAssertEqual(lookup()?.cues.map(\.time), [10])
        library.set(cues: [10, 42])
        try changeDatabase()
        clock.advance(5)
        awaitReload { XCTAssertEqual(lookup()?.cues.map(\.time), [10]) }
        XCTAssertEqual(lookup()?.cues.map(\.time), [10, 42])
        XCTAssertEqual(library.loadCount, 2)
    }

    func testUnchangedDatabaseIsNotReloaded() {
        XCTAssertNotNil(lookup())
        assertNoReload {
            for _ in 1 ... 3 {
                clock.advance(5)
                XCTAssertNotNil(lookup())
            }
        }
        XCTAssertEqual(library.loadCount, 1)
    }

    func testChangedDatabaseIsNotCheckedBeforeTheIntervalPassed() throws {
        XCTAssertNotNil(lookup())
        try changeDatabase()
        clock.advance(4)
        assertNoReload { _ = lookup() }
        XCTAssertEqual(library.loadCount, 1)
    }

    func testFailedReloadKeepsTheOldLibraryAndRetries() throws {
        library.set(cues: [10])
        XCTAssertEqual(lookup()?.cues.map(\.time), [10])
        library.set(failing: true)
        try changeDatabase()
        clock.advance(5)
        awaitReload { _ = lookup() }
        XCTAssertEqual(lookup()?.cues.map(\.time), [10])
        library.set(failing: false)
        library.set(cues: [7])
        clock.advance(5)
        awaitReload { _ = lookup() }
        XCTAssertEqual(lookup()?.cues.map(\.time), [7])
        XCTAssertEqual(library.loadCount, 3)
    }
}
