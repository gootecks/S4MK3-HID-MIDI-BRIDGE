@testable import DjayLibrary
import XCTest

private struct Expected: Decodable {
    struct Track: Decodable {
        let id: String
        let title: String?
        let artist: String?
        let album: String?
        let genre: String?
        let bpm: Double?
        let keyIndex: Int?
        let keyCamelot: String?
        let durationSec: Double?
        let path: String?
    }

    struct Session: Decodable {
        let id: String
        let deviceName: String?
        let trackCount: Int
    }

    struct Play: Decodable {
        let playKey: String
        let sessionId: String
        let idx: Int
        let title: String?
        let artist: String?
        let trackId: String?
        let deck: Int?
    }

    let tracks: [Track]
    let sessions: [Session]
    let plays: [Play]
}

final class RealLibraryTests: XCTestCase {
    private var expected: Expected!
    private var library: [String: [FixtureRow]]!

    override func setUpWithError() throws {
        let decoder = JSONDecoder()
        decoder.keyDecodingStrategy = .convertFromSnakeCase
        expected = try decoder.decode(Expected.self, from: Data(contentsOf: Fixtures.url("djaykit-expected")))
        library = try Fixtures.realLibrary()
    }

    private func assertTracks(_ snapshot: DjayLibrarySnapshot) {
        XCTAssertEqual(snapshot.tracks.count, expected.tracks.count)
        for want in expected.tracks {
            guard let got = snapshot.track(id: want.id) else { return XCTFail("missing track \(want.id)") }
            XCTAssertEqual(got.title, want.title)
            XCTAssertEqual(got.artist, want.artist)
            XCTAssertEqual(got.album, want.album)
            XCTAssertEqual(got.genre, want.genre)
            XCTAssertEqual(got.bpm, want.bpm)
            XCTAssertEqual(got.keyIndex, want.keyIndex)
            XCTAssertEqual(got.key, want.keyCamelot)
            XCTAssertEqual(got.duration, want.durationSec)
            XCTAssertEqual(got.path, want.path)
        }
    }

    private func assertHistory(_ snapshot: DjayLibrarySnapshot) {
        XCTAssertEqual(snapshot.historySessions.count, expected.sessions.count)
        for want in expected.sessions {
            guard let got = snapshot.historySessions.first(where: { $0.id == want.id }) else {
                return XCTFail("missing session \(want.id)")
            }
            XCTAssertEqual(got.deviceName, want.deviceName)
            XCTAssertEqual(got.plays.count, want.trackCount)
        }
        let plays = snapshot.historySessions.flatMap(\.plays)
        XCTAssertEqual(plays.count, expected.plays.count)
        for want in expected.plays {
            guard let got = plays.first(where: { $0.id == want.playKey }) else {
                return XCTFail("missing play \(want.playKey)")
            }
            XCTAssertEqual(got.sessionID, want.sessionId)
            XCTAssertEqual(got.index, want.idx)
            XCTAssertEqual(got.title, want.title)
            XCTAssertEqual(got.artist, want.artist)
            XCTAssertEqual(got.trackID, want.trackId)
            XCTAssertEqual(got.deck, want.deck)
        }
    }

    func testDecodedFixturesMatchDjaykit() throws {
        let snapshot = try DjayLibrarySnapshot(collections: Fixtures.collections(library))
        assertTracks(snapshot)
        assertHistory(snapshot)
        XCTAssertEqual(snapshot.playlists.map(\.id), [DjayLibrarySnapshot.rootPlaylistID])
        XCTAssertTrue(snapshot.queue.isEmpty)
    }

    func testLoadFromDatabaseFileMatchesDjaykit() throws {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent("djay-test-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: directory) }
        let database = directory.appendingPathComponent("MediaLibrary.db")
        try Fixtures.writeDatabase(library, to: database)
        let before = try Data(contentsOf: database)
        let snapshot = try DjayLibrarySnapshot.load(databaseURL: database)
        assertTracks(snapshot)
        assertHistory(snapshot)
        XCTAssertEqual(try Data(contentsOf: database), before)
    }

    func testMissingDatabaseThrows() {
        let missing = FileManager.default.temporaryDirectory.appendingPathComponent("absent-\(UUID().uuidString).db")
        XCTAssertThrowsError(try DjayLibrarySnapshot.load(databaseURL: missing))
    }

    func testRealCoPlayNeighbours() throws {
        let snapshot = try DjayLibrarySnapshot(collections: Fixtures.collections(library))
        let anybody = "0e496f3a2a54d2edf10048bb80467009"
        let neighbours = snapshot.coPlayNeighbours(of: anybody)
        XCTAssertFalse(neighbours.before.isEmpty && neighbours.after.isEmpty)
        XCTAssertTrue((neighbours.before + neighbours.after).allSatisfy { $0.trackID != anybody })
    }
}
