@testable import DjayDecks
import S8Views
import XCTest

private struct SimulatedDeck {
    let duration: Double
    let startPosition: Double
    let presses: [Double]
    let blinkOffset: Double

    func lastPress(before time: Double) -> Double? {
        presses.last { $0 <= time }
    }

    func isPlaying(at time: Double) -> Bool {
        !presses.filter { $0 <= time }.count.isMultiple(of: 2)
    }

    func elapsed(at time: Double) -> Double {
        var position = startPosition
        var playingSince: Double?
        for press in presses where press <= time {
            if let since = playingSince {
                position += press - since
                playingSince = nil
            } else {
                playingSince = press
            }
        }
        return position + (playingSince.map { time - $0 } ?? 0)
    }

    func firstTick(after press: Double) -> Double {
        let remaining = duration - elapsed(at: press)
        return press + remaining - remaining.rounded(.down)
    }

    func isLit(at time: Double) -> Bool {
        if isPlaying(at: time) {
            return true
        }
        let phase = (time - blinkOffset + 1).truncatingRemainder(dividingBy: 1)
        guard phase < 0.5 else { return false }
        return lastPress(before: time).map { time - phase >= $0 } ?? true
    }

    func reading(at time: Double) -> DeckReading {
        let seconds = Int((duration - elapsed(at: time)).rounded(.down))
        return DeckReading(
            title: "Cheeni", artist: "Skrillex",
            remainingTime: String(format: "-%02d:%02d", seconds / 60, seconds % 60), isPlaying: isLit(at: time)
        )
    }
}

private struct PageSample {
    let time: Double
    let label: Double
    let snapshot: DeckSnapshot

    var elapsed: Double {
        snapshot.elapsed ?? .nan
    }

    var remaining: Double {
        snapshot.remaining ?? .nan
    }
}

private func pageSamples(
    duration: Double, pollPhase: Double, until end: Double, reading: (Double) -> DeckReading
) -> [PageSample] {
    let start = Date(timeIntervalSince1970: 1_000_000)
    let entry = CatalogEntry(trackID: "simulated", duration: duration)
    var tracker = DeckTracker(deck: 1)
    var samples: [PageSample] = []
    var polls = 0
    var label = Double.nan
    for frame in 0 ... Int(end * 10) {
        let time = Double(frame) / 10
        while pollPhase + Double(polls) * 0.25 <= time {
            let pollTime = pollPhase + Double(polls) * 0.25
            let polled = reading(pollTime)
            label = polled.remainingTime.flatMap(TimeInterpolator.parseTime) ?? .nan
            tracker.ingest(polled, entry: entry, now: start.addingTimeInterval(pollTime))
            polls += 1
        }
        guard polls > 0 else { continue }
        samples.append(PageSample(
            time: time, label: label, snapshot: tracker.snapshot(now: start.addingTimeInterval(time))
        ))
    }
    return samples
}

final class DeckTimeSimulationTests: XCTestCase {
    private let pollPhases = [0, 0.05, 0.1, 0.15, 0.2]
    private let pollInterval = 0.25

    private func decks() -> [SimulatedDeck] {
        [
            SimulatedDeck(duration: 185.913, startPosition: 27, presses: [3.13, 12.37, 20.62], blinkOffset: 0.3),
            SimulatedDeck(duration: 185.913, startPosition: 27, presses: [3.13, 12.37, 20.62], blinkOffset: 0.8),
            SimulatedDeck(duration: 186.697, startPosition: 0, presses: [2.4, 9.95, 15.2], blinkOffset: 0.55)
        ]
    }

    private func forEachRun(_ body: (SimulatedDeck, [PageSample], String) -> Void) {
        for (index, deck) in decks().enumerated() {
            for phase in pollPhases {
                let samples = pageSamples(duration: deck.duration, pollPhase: phase, until: 30) { deck.reading(at: $0) }
                body(deck, samples, "deck \(index) poll phase \(phase)")
            }
        }
    }

    func testDeckAtTheStartOfATrackShowsZeroElapsed() {
        let deck = SimulatedDeck(duration: 186.697, startPosition: 0, presses: [], blinkOffset: 0.3)
        let samples = pageSamples(duration: deck.duration, pollPhase: 0, until: 3) { deck.reading(at: $0) }
        for sample in samples {
            XCTAssertEqual(sample.elapsed, 0, "at \(sample.time)")
            XCTAssertEqual(sample.remaining, 186.697, "at \(sample.time)")
            XCTAssertFalse(sample.snapshot.playing, "at \(sample.time)")
        }
    }

    func testPageSecondIsAlwaysTheSecondOfTheLabelLastRead() {
        forEachRun { _, samples, name in
            for sample in samples {
                XCTAssertEqual(sample.remaining.rounded(.down), sample.label, "\(name) at \(sample.time)")
            }
        }
    }

    func testTimeNeverStepsBack() {
        forEachRun { _, samples, name in
            for (previous, sample) in zip(samples, samples.dropFirst()) {
                XCTAssertGreaterThanOrEqual(sample.elapsed, previous.elapsed, "\(name) at \(sample.time)")
            }
        }
    }

    func testPlayingTimeIsNeverAheadAndAtMostOnePollBehindAfterTheFirstTick() {
        forEachRun { deck, samples, name in
            for sample in samples where deck.isPlaying(at: sample.time) {
                guard let press = deck.lastPress(before: sample.time),
                      sample.time >= deck.firstTick(after: press) + self.pollInterval else { continue }
                let error = sample.elapsed - deck.elapsed(at: sample.time)
                XCTAssertTrue(sample.snapshot.playing, "\(name) at \(sample.time)")
                XCTAssertLessThanOrEqual(error, 0.001, "\(name) at \(sample.time)")
                XCTAssertGreaterThanOrEqual(error, -self.pollInterval - 0.001, "\(name) at \(sample.time)")
            }
        }
    }

    func testPauseIsReportedWithinOnePollAndHoldsWithinOnePollOfThePausedPosition() {
        forEachRun { deck, samples, name in
            for sample in samples where !deck.isPlaying(at: sample.time) {
                guard let press = deck.lastPress(before: sample.time),
                      sample.time >= press + self.pollInterval else { continue }
                let error = sample.elapsed - deck.elapsed(at: sample.time)
                XCTAssertFalse(sample.snapshot.playing, "\(name) at \(sample.time)")
                XCTAssertLessThanOrEqual(abs(error), self.pollInterval + 0.001, "\(name) at \(sample.time)")
            }
        }
    }

    func testUnconfirmedPlayIsAheadByAtMostABlinkHalfAndBehindByAtMostTheDebounce() {
        forEachRun { deck, samples, name in
            for sample in samples {
                let error = sample.elapsed - deck.elapsed(at: sample.time)
                XCTAssertLessThanOrEqual(error, 0.5 + self.pollInterval + 0.001, "\(name) at \(sample.time)")
                XCTAssertGreaterThanOrEqual(error, -0.7 - 2 * self.pollInterval - 0.001, "\(name) at \(sample.time)")
            }
        }
    }
}

final class DeckTimeReplayTests: XCTestCase {
    private struct Row {
        let time: Double
        let isLit: Bool
        let remaining: String
    }

    private struct Stretch {
        let from: Double
        let until: Double
        let remainingAtFrom: Double
        let isPlaying: Bool
    }

    private let duration = 185.913
    private let stretches = [
        Stretch(from: 9.477, until: 21.414, remainingAtFrom: 158, isPlaying: true),
        Stretch(from: 21.414, until: 40.003, remainingAtFrom: 146.063, isPlaying: false),
        Stretch(from: 40.076, until: 42.879, remainingAtFrom: 146, isPlaying: true),
        Stretch(from: 42.879, until: 47.866, remainingAtFrom: 143.197, isPlaying: false),
        Stretch(from: 48.087, until: 50.7, remainingAtFrom: 143, isPlaying: true)
    ]

    private func rows(deckColumn: Int, deckCount: Int) throws -> [Row] {
        let url = try XCTUnwrap(
            Bundle.module.url(forResource: "c-two-decks-play-djay", withExtension: "txt", subdirectory: "Fixtures")
        )
        return try String(contentsOf: url, encoding: .utf8).split(separator: "\n").dropFirst().map { line in
            let parts = line.split(separator: " ", maxSplits: 1)
            let fields = parts[1].components(separatedBy: " | ")
            return try Row(
                time: XCTUnwrap(Double(parts[0])), isLit: fields[deckColumn] == "Active",
                remaining: fields[deckCount + deckColumn]
            )
        }
    }

    private func samples(deckColumn: Int, duration: Double, pollPhase: Double) throws -> [PageSample] {
        let rows = try rows(deckColumn: deckColumn, deckCount: 2)
        return pageSamples(duration: duration, pollPhase: pollPhase, until: 50.7) { time in
            let row = rows.last { $0.time <= time } ?? rows[0]
            return DeckReading(title: "Track", remainingTime: row.remaining, isPlaying: row.isLit)
        }
    }

    func testRecordedPlayPauseResumeFollowsDjayWithinOnePoll() throws {
        for phase in [0, 0.05, 0.1, 0.15, 0.2] {
            let samples = try samples(deckColumn: 0, duration: duration, pollPhase: phase)
            for (previous, sample) in zip(samples, samples.dropFirst()) {
                XCTAssertEqual(sample.remaining.rounded(.down), sample.label, "phase \(phase) at \(sample.time)")
                XCTAssertLessThanOrEqual(sample.remaining, previous.remaining, "phase \(phase) at \(sample.time)")
            }
            for sample in samples {
                guard let stretch = stretches.first(where: { $0.from + 0.25 <= sample.time && sample.time < $0.until })
                else { continue }
                let played = stretch.isPlaying ? sample.time - stretch.from : 0
                let behind = sample.remaining - (stretch.remainingAtFrom - played)
                XCTAssertEqual(sample.snapshot.playing, stretch.isPlaying, "phase \(phase) at \(sample.time)")
                XCTAssertLessThanOrEqual(behind, 0.27, "phase \(phase) at \(sample.time)")
                XCTAssertGreaterThanOrEqual(
                    behind, stretch.isPlaying ? -0.02 : -0.27, "phase \(phase) at \(sample.time)"
                )
            }
        }
    }

    func testRecordedDeckAtTheStartOfItsTrackStaysAtZero() throws {
        for sample in try samples(deckColumn: 1, duration: 186.697, pollPhase: 0.1) {
            XCTAssertEqual(sample.elapsed, 0, "at \(sample.time)")
            XCTAssertFalse(sample.snapshot.playing, "at \(sample.time)")
        }
    }
}
