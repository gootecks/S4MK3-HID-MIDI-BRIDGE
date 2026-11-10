@testable import DjayDecks
import XCTest

final class DeckParserTests: XCTestCase {
    func testEnglishLabels() {
        let reading = DeckParser.reading(from: [
            "Title, Deck 1": "Soma", "Artist, Deck 1": "Skrillex", "Key, Deck 1": "8A",
            "140.0, Deck 1": "140.0", "+7.3%, Deck 1": "x", "Elapsed time, Deck 1": "00:42",
            "Remaining time, Deck 1": "-02:43", "Play / Pause, Deck 1": "Active", "Line volume, Deck 1": "0.8"
        ])
        XCTAssertEqual(reading.title, "Soma")
        XCTAssertEqual(reading.artist, "Skrillex")
        XCTAssertEqual(reading.key, "8A")
        XCTAssertEqual(reading.bpm, "140.0")
        XCTAssertEqual(reading.bpmPercent, "+7.3%")
        XCTAssertEqual(reading.elapsedTime, "00:42")
        XCTAssertEqual(reading.remainingTime, "-02:43")
        XCTAssertTrue(reading.isPlaying)
        XCTAssertEqual(reading.lineVolume, "0.8")
        XCTAssertTrue(reading.hasTrack)
    }

    func testFrenchLabels() {
        let reading = DeckParser.reading(from: [
            "Titre, Platine 2": "Soma", "Artiste, Platine 2": "Skrillex", "Clé, Platine 2": "8A"
        ])
        XCTAssertEqual(reading.title, "Soma")
        XCTAssertEqual(reading.artist, "Skrillex")
        XCTAssertEqual(reading.key, "8A")
    }

    func testPlayStateOtherThanActiveIsPaused() {
        XCTAssertFalse(DeckParser.reading(from: ["Play / Pause, Deck 1": "Inactive"]).isPlaying)
        XCTAssertFalse(DeckParser.reading(from: [:]).hasTrack)
    }

    func testDeckNumber() {
        XCTAssertEqual(DeckParser.parseDeckNumber(from: "Key, Deck 3"), 3)
        XCTAssertEqual(DeckParser.parseDeckNumber(from: "Clé, Platine 4"), 4)
        XCTAssertNil(DeckParser.parseDeckNumber(from: "Crossfader"))
    }

    func testReaderSplitsOneWalkByDeckInBothLanguages() throws {
        struct Source: DeckAccessibility {
            func labeledValues() throws -> [String: String] {
                [
                    "Title, Deck 1": "A", "Artiste, Platine 1": "B", "Title, Deck 3": "C",
                    "Remaining time, Deck 4": "-02:58", "Play / Pause, Deck 4": "Active"
                ]
            }
        }
        let readings = try DeckReader(source: Source()).readings()
        XCTAssertEqual(readings[1]?.title, "A")
        XCTAssertEqual(readings[1]?.artist, "B")
        XCTAssertEqual(readings[3]?.title, "C")
        XCTAssertEqual(readings[4]?.remainingTime, "-02:58")
        XCTAssertEqual(readings[4]?.isPlaying, true)
        XCTAssertNil(readings[2])
    }
}
