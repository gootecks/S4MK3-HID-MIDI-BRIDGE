@testable import DjayDecks
import XCTest

final class TimeInterpolatorTests: XCTestCase {
    private let start = Date(timeIntervalSince1970: 1_000_000)

    func testRemainingOnlyWithKnownDurationDerivesElapsed() throws {
        var interpolator = TimeInterpolator(now: start)
        interpolator.update(
            elapsedTime: nil, remainingTime: "-02:23", knownDuration: 185, isPlaying: false, bpmPercent: "0.0%",
            now: start
        )
        XCTAssertEqual(try XCTUnwrap(interpolator.interpolatedElapsed(now: start)), 42, accuracy: 0.001)
    }

    func testMissingElapsedAndDurationStaysUnavailable() {
        var interpolator = TimeInterpolator(now: start)
        interpolator.update(
            elapsedTime: nil, remainingTime: "-02:23", knownDuration: nil, isPlaying: false, bpmPercent: "0.0%",
            now: start
        )
        XCTAssertNil(interpolator.interpolatedElapsed(now: start))
    }

    func testPlayingExtrapolatesBetweenReadings() throws {
        var interpolator = TimeInterpolator(now: start)
        interpolator.update(
            elapsedTime: "01:00", remainingTime: "-01:00", isPlaying: true, bpmPercent: "0.0%", now: start
        )
        let later = start.addingTimeInterval(0.5)
        XCTAssertEqual(try XCTUnwrap(interpolator.interpolatedElapsed(now: later)), 60.5, accuracy: 0.001)
        XCTAssertEqual(try XCTUnwrap(interpolator.interpolatedRemaining(now: later)), 59.5, accuracy: 0.001)
    }

    func testPausedDoesNotAdvance() throws {
        var interpolator = TimeInterpolator(now: start)
        interpolator.update(
            elapsedTime: "01:00", remainingTime: "-01:00", isPlaying: false, bpmPercent: "0.0%", now: start
        )
        XCTAssertEqual(
            try XCTUnwrap(interpolator.interpolatedElapsed(now: start.addingTimeInterval(5))), 60, accuracy: 0.001
        )
    }

    func testPitchScalesRate() throws {
        var interpolator = TimeInterpolator(now: start)
        interpolator.update(
            elapsedTime: "00:10", remainingTime: nil, isPlaying: true, bpmPercent: "+10.0%", now: start
        )
        XCTAssertEqual(
            try XCTUnwrap(interpolator.interpolatedElapsed(now: start.addingTimeInterval(1))), 11.1, accuracy: 0.001
        )
    }

    func testNewReadingEasesCorrectionInsteadOfJumping() throws {
        var interpolator = TimeInterpolator(now: start)
        interpolator.update(elapsedTime: "00:10", remainingTime: nil, isPlaying: true, bpmPercent: nil, now: start)
        let reading = start.addingTimeInterval(1.2)
        interpolator.update(elapsedTime: "00:11", remainingTime: nil, isPlaying: true, bpmPercent: nil, now: reading)
        let justAfter = try XCTUnwrap(interpolator.interpolatedElapsed(now: reading))
        XCTAssertEqual(justAfter, 11.2, accuracy: 0.001)
        let settled = try XCTUnwrap(interpolator.interpolatedElapsed(now: reading.addingTimeInterval(0.3)))
        XCTAssertEqual(settled, 11.3, accuracy: 0.001)
    }

    func testParseTime() {
        XCTAssertEqual(TimeInterpolator.parseTime("-02:23"), 143)
        XCTAssertEqual(TimeInterpolator.parseTime(" 01:05 "), 65)
        XCTAssertNil(TimeInterpolator.parseTime("1:02:03"))
        XCTAssertNil(TimeInterpolator.parseTime("abc"))
    }
}

final class PlayStateDebouncerTests: XCTestCase {
    private let start = Date(timeIntervalSince1970: 1_000_000)

    func testPausedToPlayingNeedsSustainedState() {
        var debouncer = PlayStateDebouncer()
        XCTAssertFalse(debouncer.update(isPlaying: true, now: start))
        XCTAssertFalse(debouncer.update(isPlaying: true, now: start.addingTimeInterval(0.5)))
        XCTAssertTrue(debouncer.update(isPlaying: true, now: start.addingTimeInterval(0.8)))
    }

    func testPlayingToPausedIsImmediate() {
        var debouncer = PlayStateDebouncer()
        _ = debouncer.update(isPlaying: true, now: start)
        XCTAssertTrue(debouncer.update(isPlaying: true, now: start.addingTimeInterval(1)))
        XCTAssertFalse(debouncer.update(isPlaying: false, now: start.addingTimeInterval(1.01)))
    }
}
