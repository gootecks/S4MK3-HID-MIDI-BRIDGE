@testable import DjayLibrary
import XCTest

final class SyntheticLibraryTests: XCTestCase {
    private func item(_ id: String, _ title: String, _ artist: String) -> CollectionRow {
        row(id, djayObject(["title": .string(title), "artistUUIDs": djayStrings(["art-\(artist)"])]))
    }

    private func artists(_ names: [String]) -> [CollectionRow] {
        names.map { row("art-\($0)", djayObject(["name": .string($0)])) }
    }

    private func session(_ key: String, at seconds: Double) -> CollectionRow {
        row(key, djayObject(["startDate": .date(Date(timeIntervalSinceReferenceDate: seconds))]))
    }

    private func titleID(uuid: String?, title: String, artist: String) -> DjayValue {
        var fields: [String: DjayValue] = ["title": .string(title), "artist": .string(artist)]
        fields["uuid"] = uuid.map { .string($0) }
        return djayObject(fields)
    }

    private func play(_ key: String, session: String, at time: Double, title: String, artist: String) -> CollectionRow {
        row(key, djayObject([
            "sessionUUID": .string(session),
            "startTime": .date(Date(timeIntervalSinceReferenceDate: time)),
            "titleID": titleID(uuid: nil, title: title, artist: artist),
        ]))
    }

    func testPlaylistOrderFollowsItemUUIDs() throws {
        let snapshot = DjayLibrarySnapshot(collections: [
            "mediaItems": [item("a", "Alpha", "X"), item("b", "Beta", "X"), item("c", "Gamma", "X")],
            "mediaArtists": artists(["X"]),
            "mediaItemPlaylistItems": [
                row("pi1", djayObject(["mediaItemUUID": .string("a")])),
                row("pi2", djayObject(["mediaItemUUID": .string("b")])),
                row("pi3", djayObject(["mediaItemUUID": .string("c")])),
            ],
            "mediaItemPlaylists": [
                row("list", djayObject(["name": .string("Set"), "itemUUIDs": djayStrings(["pi3", "pi1", "pi2"])])),
                row("folder", djayObject([
                    "name": .string("Dir"), "type": .int(1), "childUUIDs": djayStrings(["list"]),
                ])),
            ],
        ])
        XCTAssertEqual(snapshot.playlist(id: "list")?.trackIDs, ["c", "a", "b"])
        XCTAssertEqual(snapshot.playlist(id: "list")?.kind, .playlist)
        XCTAssertEqual(snapshot.playlist(id: "folder")?.kind, .folder)
        XCTAssertEqual(snapshot.playlist(id: "folder")?.childIDs, ["list"])
        let list = try XCTUnwrap(snapshot.playlist(id: "list"))
        XCTAssertEqual(snapshot.playlistTracks(list).compactMap(\.title), ["Gamma", "Alpha", "Beta"])
    }

    func testQueueResolvesTracksInOrder() {
        let snapshot = DjayLibrarySnapshot(collections: [
            "mediaItems": [item("a", "Alpha", "X"), item("b", "Beta", "X")],
            "mediaArtists": artists(["X"]),
            "queues": [row("queue-default", djayObject(["itemUUIDs": djayStrings(["q2", "q1"])]))],
            "queueItems": [
                row("q1", djayObject(["titleID": titleID(uuid: "a", title: "Alpha", artist: "X")])),
                row("q2", djayObject(["titleID": titleID(uuid: nil, title: "Beta", artist: "x")])),
            ],
        ])
        XCTAssertEqual(snapshot.queue.map(\.trackID), ["b", "a"])
        XCTAssertEqual(snapshot.queue.map(\.title), ["Beta", "Alpha"])
    }

    func testBpmKeyPrecedenceAndRounding() {
        let itemFields: [String: DjayValue] = [
            "title": .string("A"),
            "bpm": .double(100),
            "musicalKeySignatureIndex": .int(0),
        ]
        let userFields: [String: DjayValue] = [
            "manualBPM": .double(0),
            "manualKeySignatureIndex2": .int(11),
        ]
        let analyzedFields: [String: DjayValue] = ["bpm": .double(120.456), "keySignatureIndex": .int(1)]
        let snapshot = DjayLibrarySnapshot(collections: [
            "mediaItems": [row("a", djayObject(itemFields))],
            "mediaItemAnalyzedData": [row("a", djayObject(analyzedFields))],
            "mediaItemUserData": [row("a", djayObject(userFields))],
        ])
        let track = snapshot.track(id: "a")
        XCTAssertEqual(track?.bpm, 120.46)
        XCTAssertEqual(track?.key, "7A")
    }

    func testCoPlayNeighboursRankByCountThenRecency() {
        var collections: [String: [CollectionRow]] = [
            "mediaItems": [
                item("t", "Target", "X"), item("p", "Prev", "X"), item("q", "Other", "X"), item("n", "Next", "X"),
            ],
            "mediaArtists": artists(["X"]),
        ]
        collections["historySessions"] = [session("s1", at: 100), session("s2", at: 200), session("s3", at: 300)]
        collections["historySessionItems"] = [
            play("p1", session: "s1", at: 101, title: "Prev", artist: "X"),
            play("p2", session: "s1", at: 102, title: "Target", artist: "X"),
            play("p3", session: "s1", at: 103, title: "Next", artist: "X"),
            play("p4", session: "s2", at: 201, title: "Prev", artist: "X"),
            play("p5", session: "s2", at: 202, title: "Target", artist: "X"),
            play("p6", session: "s2", at: 203, title: "Other", artist: "X"),
            play("p7", session: "s3", at: 301, title: "Other", artist: "X"),
            play("p8", session: "s3", at: 302, title: "Target", artist: "X"),
            play("p9", session: "s3", at: 303, title: "Unrelated", artist: "Y"),
        ]
        let snapshot = DjayLibrarySnapshot(collections: collections)
        let neighbours = snapshot.coPlayNeighbours(of: "t")
        XCTAssertEqual(neighbours.before.map(\.title), ["Prev", "Other"])
        XCTAssertEqual(neighbours.before.map(\.count), [2, 1])
        XCTAssertEqual(neighbours.after.map(\.title), ["Unrelated", "Other", "Next"])
        XCTAssertEqual(neighbours.after.first?.title, "Unrelated")
        XCTAssertEqual(snapshot.coPlayNeighbours(of: "t", limit: 1).after.count, 1)
        XCTAssertEqual(snapshot.coPlayNeighbours(of: "missing"), DjayCoPlayNeighbours(before: [], after: []))
    }

    func testCoPlayDoesNotCrossSessions() {
        var collections: [String: [CollectionRow]] = [
            "mediaItems": [item("t", "Target", "X"), item("n", "Next", "X")],
            "mediaArtists": artists(["X"]),
        ]
        collections["historySessions"] = [session("s1", at: 100), session("s2", at: 200)]
        collections["historySessionItems"] = [
            play("p1", session: "s1", at: 101, title: "Target", artist: "X"),
            play("p2", session: "s2", at: 201, title: "Next", artist: "X"),
        ]
        let neighbours = DjayLibrarySnapshot(collections: collections).coPlayNeighbours(of: "t")
        XCTAssertTrue(neighbours.before.isEmpty)
        XCTAssertTrue(neighbours.after.isEmpty)
    }

    func testBlobDecodeRejectsGarbageAndEmpty() throws {
        XCTAssertNil(try DjayBlob.decode(Data()))
        XCTAssertNil(try DjayBlob.decode(Data(repeating: 0, count: 8)))
        XCTAssertThrowsError(try DjayBlob.decode(Data("XXXXXXXX".utf8)))
        XCTAssertThrowsError(try DjayBlob.decode(Data("TSAF".utf8) + Data(repeating: 1, count: 30)))
    }

    func testKeyedArchiveDecodes() throws {
        let when = Date(timeIntervalSinceReferenceDate: 12345)
        let archive = try NSKeyedArchiver.archivedData(
            withRootObject: ["Deck", when] as NSArray, requiringSecureCoding: false
        )
        let value = try DjayBlob.decode(archive)
        XCTAssertEqual(value, .list([.string("Deck"), .date(when)]))
    }
}
