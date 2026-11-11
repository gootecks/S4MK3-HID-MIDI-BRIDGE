@testable import DjayDecks
import XCTest

enum DeckFixtures {
    static let trackID = "13bce5297cd61df2a4cebfd2f779ceba"

    static func url() throws -> URL {
        try XCTUnwrap(Bundle.module.url(forResource: trackID, withExtension: "djayMetadata", subdirectory: "Fixtures"))
    }
}

final class AnalysisDecodingTests: XCTestCase {
    func testDecodesRealTrackMetadata() throws {
        let analysis = try TrackAnalysis.load(url: DeckFixtures.url())
        XCTAssertEqual(analysis.name, "Soma")
        XCTAssertEqual(analysis.artist, "Skrillex, Nitepunk")
        XCTAssertEqual(analysis.duration, 205)
        XCTAssertEqual(analysis.bpm, 140)
        XCTAssertEqual(analysis.amplitude.count, 4099)
        XCTAssertEqual(analysis.amplitudeRate, 20.009, accuracy: 0.001)
        XCTAssertEqual(analysis.beats.count, 478)
        XCTAssertEqual(analysis.beats[0], 0.0095, accuracy: 0.001)
        XCTAssertEqual(analysis.beats[1] - analysis.beats[0], 60.0 / 140, accuracy: 0.02)
    }

    func testBarsStartAtDownbeatEveryFourBeats() throws {
        let analysis = try TrackAnalysis.load(url: DeckFixtures.url())
        XCTAssertEqual(analysis.firstDownBeatIndex, 2)
        XCTAssertEqual(analysis.bars.first ?? 0, analysis.beats[2], accuracy: 0.0001)
        XCTAssertEqual(analysis.bars.count, 119)
        XCTAssertEqual(analysis.bars[1], analysis.beats[6], accuracy: 0.0001)
    }

    func testBandsAreThreeNormalizedChannels() throws {
        let analysis = try TrackAnalysis.load(url: DeckFixtures.url())
        XCTAssertEqual(analysis.bands.count, 3)
        XCTAssertEqual(Set(analysis.bands.map(\.count)), [1038])
        XCTAssertEqual(analysis.bandRate, 5.0666, accuracy: 0.001)
        XCTAssertEqual(analysis.bands.flatMap { $0 }.max(), 255)
        XCTAssertTrue(analysis.bands.allSatisfy { $0.contains { $0 > 0 } })
    }

    func testBfloat16Reconstruction() {
        let bytes: [UInt8] = [0x3F, 0x80, 0x40, 0x00, 0x3F, 0x00]
        XCTAssertEqual(TrackAnalysis.bands(bytes), [[127], [255], [63]])
    }

    func testBeatFloatsAreBigEndian() {
        XCTAssertEqual(TrackAnalysis.floats([0x3F, 0x80, 0x00, 0x00, 0x40, 0x00, 0x00, 0x00]), [1, 2])
    }

    func testRejectsNonPropertyList() {
        XCTAssertThrowsError(try TrackAnalysis.decode(Data("not a plist".utf8)))
    }

    func testMissingSectionsDecodeAsEmpty() throws {
        let plist: [String: Any] = ["info": ["Name": "X"]]
        let data = try PropertyListSerialization.data(fromPropertyList: plist, format: .binary, options: 0)
        let analysis = try TrackAnalysis.decode(data)
        XCTAssertEqual(analysis.name, "X")
        XCTAssertTrue(analysis.amplitude.isEmpty)
        XCTAssertTrue(analysis.bands.isEmpty)
        XCTAssertTrue(analysis.bars.isEmpty)
    }

    private func analysis(beats: [Double], firstDownBeat: Int?) -> TrackAnalysis {
        TrackAnalysis(
            name: nil, artist: nil, duration: nil, bpm: nil, amplitude: [], amplitudeRate: 0, bands: [],
            bandRate: 0, beats: beats, firstDownBeatIndex: firstDownBeat
        )
    }

    private func steady(_ count: Int, from start: Double = 0, interval: Double = 0.5) -> [Double] {
        (0 ..< count).map { start + Double($0) * interval }
    }

    func testNegativeDownbeatIndexKeepsBarPhase() {
        XCTAssertEqual(analysis(beats: steady(9), firstDownBeat: -1).bars, [1.5, 3.5])
        XCTAssertEqual(analysis(beats: steady(9), firstDownBeat: Int.min).bars.count, 3)
    }

    func testDownbeatIndexBeyondTheListKeepsBarPhase() {
        XCTAssertEqual(analysis(beats: steady(9), firstDownBeat: 10).bars, [1, 3])
        XCTAssertEqual(analysis(beats: steady(9), firstDownBeat: Int.max).bars, [1.5, 3.5])
    }

    func testBarsKeepPhaseAcrossAGapInTheBeatList() {
        let beats = steady(8) + steady(8, from: 7)
        XCTAssertEqual(analysis(beats: beats, firstDownBeat: 0).bars, [0, 2, 8, 10])
    }

    func testGapNotNearAWholeNumberOfBeatsCountsAsOneBeat() {
        let beats = steady(4) + steady(8, from: 2.8)
        XCTAssertEqual(analysis(beats: beats, firstDownBeat: 0).bars, [0, 2.8, 4.8])
    }

    func testTempoChangeWithoutGapDoesNotInsertBeats() {
        let beats = steady(8) + steady(8, from: 3.875, interval: 0.375)
        XCTAssertEqual(analysis(beats: beats, firstDownBeat: 0).bars, [0, 2, 3.875, 5.375])
    }

    func testStoreFindsFileInShardDirectory() throws {
        let root = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        let shard = root.appendingPathComponent("ab")
        try FileManager.default.createDirectory(at: shard, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: root) }
        try FileManager.default.copyItem(
            at: DeckFixtures.url(), to: shard.appendingPathComponent(DeckFixtures.trackID + ".djayMetadata")
        )
        let store = AnalysisStore(root: root)
        XCTAssertNotNil(store.url(for: DeckFixtures.trackID))
        XCTAssertEqual(store.analysis(for: DeckFixtures.trackID)?.name, "Soma")
        XCTAssertNil(store.url(for: "missing"))
    }
}
