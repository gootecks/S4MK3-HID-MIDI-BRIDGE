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

    func testReaderMergesBothLanguagePrefixes() throws {
        struct Source: DeckAccessibility {
            func labeledValues(prefix: String) throws -> [String: String] {
                prefix == "Deck 1" ? ["Title, Deck 1": "A"] : ["Artiste, Platine 1": "B"]
            }
        }
        let reading = try DeckReader(source: Source()).read(deck: 1)
        XCTAssertEqual(reading.title, "A")
        XCTAssertEqual(reading.artist, "B")
    }
}
