@testable import S8Views
import XCTest

final class DeckSnapshotTests: XCTestCase {
    func testDecodesMinimalSnapshot() throws {
        let snapshot = try DeckSnapshot.decode(Data(#"{"schema":1,"deck":2,"title":"Soma"}"#.utf8))
        XCTAssertEqual(snapshot.deck, 2)
        XCTAssertEqual(snapshot.title, "Soma")
        XCTAssertFalse(snapshot.playing)
        XCTAssertNil(snapshot.analysis)
        XCTAssertEqual(snapshot.cues, [])
        XCTAssertTrue(snapshot.hasTrack)
    }

    func testRejectsOtherSchema() {
        XCTAssertThrowsError(try DeckSnapshot.decode(Data(#"{"schema":2,"deck":1}"#.utf8))) {
            XCTAssertEqual($0 as? SnapshotError, .unsupportedSchema(2))
        }
    }

    func testRoundTripKeepsAnalysisAndCues() throws {
        let analysis = DeckSnapshot.Analysis(
            amplitude: [1, 2, 3], amplitudeRate: 1, bands: [[1], [2], [3]], bandRate: 0.5, beats: [0.5, 1],
            bars: [0.5], bpm: 120
        )
        let source = DeckSnapshot(
            deck: 1, title: "A", artist: "B", bpm: "120.0", key: "8A", elapsed: 1, remaining: 2, duration: 3,
            playing: true, analysis: analysis, cues: [.init(time: 1.5, name: "Drop", color: "#FF0000")]
        )
        XCTAssertEqual(try DeckSnapshot.decode(source.encoded()), source)
    }

    func testPositionFallsBackToDurationMinusRemaining() {
        let snapshot = DeckSnapshot(deck: 1, title: "A", remaining: 40, duration: 100)
        XCTAssertEqual(snapshot.position, 60)
        XCTAssertNil(DeckSnapshot(deck: 1, title: "A", remaining: 40).position)
    }

    func testTimeFormat() {
        XCTAssertEqual(DeckTime.format(125.9), "2:05")
        XCTAssertEqual(DeckTime.format(5, negative: true), "-0:05")
        XCTAssertEqual(DeckTime.format(nil), "\u{2013}")
    }
}

final class DeckPanelTests: XCTestCase {
    private let background = Pixel(red: 8, green: 10, blue: 14)
    private let surface = Pixel(red: 26, green: 31, blue: 40)
    private let playhead = Pixel(red: 240, green: 243, blue: 248)

    private func differing(_ bitmap: Bitmap, rows: Range<Int>, from color: Pixel) -> Int {
        rows.reduce(0) { total, row in
            total + (0 ..< bitmap.width).filter { bitmap.pixel(column: $0, row: row) != color }.count
        }
    }

    private func analyzed(elapsed: Double? = 50) -> DeckSnapshot {
        let analysis = DeckSnapshot.Analysis(
            amplitude: Array(repeating: 100, count: 100), amplitudeRate: 1,
            bands: [100, 0, 0].map { Array(repeating: $0 == 100 ? 255 : 0, count: 100) },
            bandRate: 1, beats: stride(from: 0.0, to: 100, by: 2).map { $0 },
            bars: stride(from: 0.0, to: 100, by: 4).map { $0 }
        )
        return DeckSnapshot(
            deck: 1, title: "Soma", artist: "Skrillex", bpm: "140.0", key: "8A", elapsed: elapsed, duration: 100,
            playing: true, analysis: analysis, cues: [.init(time: 25, name: "A", color: "#00FF00")]
        )
    }

    func testRendersFullPanel() {
        let bitmap = DeckPanelRenderer.render(analyzed())
        XCTAssertEqual(bitmap.width, Metrics.width)
        XCTAssertEqual(bitmap.height, Metrics.height)
    }

    func testNoTrackIsExplicitEmptyState() {
        let bitmap = DeckPanelRenderer.render(DeckSnapshot(deck: 3))
        XCTAssertGreaterThan(differing(bitmap, rows: 130 ..< 160, from: background), 100)
        XCTAssertEqual(differing(bitmap, rows: 170 ..< 266, from: background), 0)
        XCTAssertNotEqual(bitmap, DeckPanelRenderer.render(DeckSnapshot(deck: 3, notice: "djay is not running")))
    }

    func testTrackWithoutAnalysisShowsNoAnalysisBox() {
        let bitmap = DeckPanelRenderer.render(DeckSnapshot(deck: 1, title: "Soma", artist: "Skrillex"))
        XCTAssertEqual(bitmap.pixel(column: 100, row: 140), surface)
        XCTAssertGreaterThan(differing(bitmap, rows: 190 ..< 210, from: surface), 100)
    }

    func testPlayheadAndWaveformPlacement() {
        let bitmap = DeckPanelRenderer.render(analyzed(elapsed: 50))
        XCTAssertEqual(bitmap.pixel(column: 240, row: 138), playhead)
        XCTAssertEqual(bitmap.pixel(column: 100, row: 138), surface)
        XCTAssertNotEqual(bitmap.pixel(column: 100, row: 194), surface)
        XCTAssertNotEqual(bitmap.pixel(column: 400, row: 194), surface)
    }

    func testPlayedPartIsDimmerThanUnplayed() {
        let bitmap = DeckPanelRenderer.render(analyzed(elapsed: 50))
        let played = bitmap.pixel(column: 100, row: 194)
        let unplayed = bitmap.pixel(column: 400, row: 194)
        XCTAssertLessThan(Int(played.blue), Int(unplayed.blue))
    }

    func testPlayheadMovesWithElapsed() {
        let early = DeckPanelRenderer.render(analyzed(elapsed: 10))
        XCTAssertEqual(early.pixel(column: 12 + 45, row: 138), playhead)
        XCTAssertEqual(early.pixel(column: 240, row: 138), surface)
    }

    func testCueMarkerUsesCueColor() {
        let bitmap = DeckPanelRenderer.render(analyzed())
        XCTAssertEqual(bitmap.pixel(column: 12 + 114, row: 133), Pixel(red: 0, green: 255, blue: 0))
    }

    func testBeatTicksOnlyWhenSpacingAllowsIt() {
        let dense = analyzed()
        let sparse = DeckSnapshot(
            deck: 1, title: "Soma", elapsed: 50, duration: 1000,
            analysis: .init(
                amplitude: Array(repeating: 100, count: 1000), amplitudeRate: 1,
                beats: stride(from: 0.0, to: 1000, by: 1).map { $0 }, bars: [0]
            )
        )
        let denseRows = differing(DeckPanelRenderer.render(dense), rows: 257 ..< 260, from: surface)
        let bare = DeckSnapshot(
            deck: 1, title: "Soma", elapsed: 50, duration: 1000,
            analysis: .init(amplitude: Array(repeating: 100, count: 1000), amplitudeRate: 1, bars: [0])
        )
        let sparseRows = differing(DeckPanelRenderer.render(sparse), rows: 257 ..< 260, from: surface)
        let bareRows = differing(DeckPanelRenderer.render(bare), rows: 257 ..< 260, from: surface)
        XCTAssertGreaterThan(denseRows, 100)
        XCTAssertEqual(sparseRows, bareRows)
    }
}
