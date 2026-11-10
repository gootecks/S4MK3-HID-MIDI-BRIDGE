@testable import DjayDecks
import XCTest

final class TimeInterpolatorTests: XCTestCase {
    private let start = Date(timeIntervalSince1970: 1_000_000)

    private func at(_ seconds: TimeInterval) -> Date {
        start.addingTimeInterval(seconds)
    }

    func testPausedLabelRestsInTheMiddleOfItsSecond() throws {
        var interpolator = TimeInterpolator()
        interpolator.update(
            elapsedTime: "01:00", remainingTime: "-02:23", isPlaying: false, bpmPercent: "0.0%", now: start
        )
        XCTAssertEqual(try XCTUnwrap(interpolator.interpolatedElapsed(now: at(5))), 60.5)
        XCTAssertEqual(try XCTUnwrap(interpolator.interpolatedRemaining(now: at(5))), 143.5)
    }

    func testRemainingOnlyWithKnownDurationDerivesElapsed() throws {
        var interpolator = TimeInterpolator()
        interpolator.update(
            elapsedTime: nil, remainingTime: "-02:23", knownDuration: 185, isPlaying: false, bpmPercent: "0.0%",
            now: start
        )
        XCTAssertEqual(try XCTUnwrap(interpolator.interpolatedElapsed(now: start)), 41.5, accuracy: 0.001)
    }

    func testStartOfTrackIsZeroElapsedAndFullDuration() throws {
        var interpolator = TimeInterpolator()
        interpolator.update(
            elapsedTime: "00:00", remainingTime: "-03:06", knownDuration: 186.697, isPlaying: false,
            bpmPercent: "0.0%", now: start
        )
        XCTAssertEqual(try XCTUnwrap(interpolator.interpolatedElapsed(now: start)), 0)
        XCTAssertEqual(try XCTUnwrap(interpolator.interpolatedRemaining(now: start)), 186.697)
        interpolator.update(
            elapsedTime: nil, remainingTime: "-03:06", knownDuration: 186.697, isPlaying: false,
            bpmPercent: "0.0%", now: start
        )
        XCTAssertEqual(try XCTUnwrap(interpolator.interpolatedElapsed(now: start)), 0)
    }

    func testMissingElapsedAndDurationStaysUnavailable() {
        var interpolator = TimeInterpolator()
        interpolator.update(
            elapsedTime: nil, remainingTime: "-02:23", knownDuration: nil, isPlaying: false, bpmPercent: "0.0%",
            now: start
        )
        XCTAssertNil(interpolator.interpolatedElapsed(now: start))
    }

    func testPlayingStaysInsideTheLabelSecondUntilTheLabelTicks() throws {
        var interpolator = TimeInterpolator()
        interpolator.update(
            elapsedTime: "01:00", remainingTime: "-01:00", isPlaying: true, bpmPercent: "0.0%", now: start
        )
        let elapsed = try XCTUnwrap(interpolator.interpolatedElapsed(now: at(3)))
        let remaining = try XCTUnwrap(interpolator.interpolatedRemaining(now: at(3)))
        XCTAssertEqual(elapsed.rounded(.down), 60)
        XCTAssertEqual(elapsed, 61, accuracy: 0.001)
        XCTAssertEqual(remaining, 60)
    }

    func testTickWhilePlayingIsTheSecondBoundaryAndCountsFromThere() throws {
        var interpolator = TimeInterpolator()
        interpolator.update(
            elapsedTime: "01:00", remainingTime: "-01:00", isPlaying: true, bpmPercent: "0.0%", now: start
        )
        interpolator.update(
            elapsedTime: "01:01", remainingTime: "-00:59", isPlaying: true, bpmPercent: "0.0%", now: at(0.2)
        )
        XCTAssertEqual(try XCTUnwrap(interpolator.interpolatedElapsed(now: at(0.2))), 61)
        XCTAssertEqual(try XCTUnwrap(interpolator.interpolatedRemaining(now: at(0.2))), 60, accuracy: 0.001)
        XCTAssertEqual(try XCTUnwrap(interpolator.interpolatedElapsed(now: at(0.7))), 61.5, accuracy: 0.001)
        XCTAssertEqual(try XCTUnwrap(interpolator.interpolatedRemaining(now: at(0.7))), 59.5, accuracy: 0.001)
    }

    func testPauseKeepsTheCountedPosition() throws {
        var interpolator = TimeInterpolator()
        interpolator.update(elapsedTime: "01:00", remainingTime: nil, isPlaying: true, bpmPercent: nil, now: start)
        interpolator.update(elapsedTime: "01:01", remainingTime: nil, isPlaying: true, bpmPercent: nil, now: at(1))
        interpolator.update(elapsedTime: "01:01", remainingTime: nil, isPlaying: false, bpmPercent: nil, now: at(1.75))
        XCTAssertEqual(try XCTUnwrap(interpolator.interpolatedElapsed(now: at(20))), 61.75, accuracy: 0.001)
    }

    func testResumeCountsFromThePausedPositionAndFromWhenPlayStarted() throws {
        var interpolator = TimeInterpolator()
        interpolator.update(elapsedTime: "01:00", remainingTime: nil, isPlaying: true, bpmPercent: nil, now: start)
        interpolator.update(elapsedTime: "01:01", remainingTime: nil, isPlaying: true, bpmPercent: nil, now: at(1))
        interpolator.update(elapsedTime: "01:01", remainingTime: nil, isPlaying: false, bpmPercent: nil, now: at(1.25))
        interpolator.update(
            elapsedTime: "01:01", remainingTime: nil, isPlaying: true, playingSince: at(30), bpmPercent: nil,
            now: at(30.5)
        )
        XCTAssertEqual(try XCTUnwrap(interpolator.interpolatedElapsed(now: at(30.5))), 61.75, accuracy: 0.001)
    }

    func testLabelChangeWhilePausedRestsInTheNewSecond() throws {
        var interpolator = TimeInterpolator()
        interpolator.update(elapsedTime: "01:00", remainingTime: nil, isPlaying: true, bpmPercent: nil, now: start)
        interpolator.update(elapsedTime: "01:01", remainingTime: nil, isPlaying: false, bpmPercent: nil, now: at(1))
        interpolator.update(elapsedTime: "00:30", remainingTime: nil, isPlaying: false, bpmPercent: nil, now: at(2))
        XCTAssertEqual(try XCTUnwrap(interpolator.interpolatedElapsed(now: at(2))), 30.5)
    }

    func testJumpWhilePlayingCountsFromTheMiddleOfTheNewSecond() throws {
        var interpolator = TimeInterpolator()
        interpolator.update(elapsedTime: "01:00", remainingTime: nil, isPlaying: true, bpmPercent: nil, now: start)
        interpolator.update(elapsedTime: "00:30", remainingTime: nil, isPlaying: true, bpmPercent: nil, now: at(1))
        XCTAssertEqual(try XCTUnwrap(interpolator.interpolatedElapsed(now: at(1.25))), 30.75, accuracy: 0.001)
    }

    func testPitchScalesRate() throws {
        var interpolator = TimeInterpolator()
        interpolator.update(
            elapsedTime: "00:10", remainingTime: nil, isPlaying: true, bpmPercent: "+10.0%", now: start
        )
        interpolator.update(
            elapsedTime: "00:11", remainingTime: nil, isPlaying: true, bpmPercent: "+10.0%", now: at(1)
        )
        XCTAssertEqual(try XCTUnwrap(interpolator.interpolatedElapsed(now: at(1.5))), 11.55, accuracy: 0.001)
    }

    func testNextSecondIsOneStepInThePlayingDirection() {
        var interpolator = TimeInterpolator()
        XCTAssertFalse(interpolator.isNextSecond(elapsedTime: "01:01", remainingTime: "-00:59"))
        interpolator.update(
            elapsedTime: "01:00", remainingTime: "-01:00", isPlaying: false, bpmPercent: nil, now: start
        )
        XCTAssertTrue(interpolator.isNextSecond(elapsedTime: "01:01", remainingTime: nil))
        XCTAssertTrue(interpolator.isNextSecond(elapsedTime: nil, remainingTime: "-00:59"))
        XCTAssertFalse(interpolator.isNextSecond(elapsedTime: "01:00", remainingTime: "-01:00"))
        XCTAssertFalse(interpolator.isNextSecond(elapsedTime: "00:59", remainingTime: "-01:01"))
        XCTAssertFalse(interpolator.isNextSecond(elapsedTime: "01:02", remainingTime: "-00:58"))
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
        XCTAssertFalse(debouncer.update(isPlaying: true, now: start.addingTimeInterval(0.25)))
        XCTAssertFalse(debouncer.update(isPlaying: true, now: start.addingTimeInterval(0.5)))
        XCTAssertTrue(debouncer.update(isPlaying: true, now: start.addingTimeInterval(0.75)))
    }

    func testPlayingToPausedIsImmediate() {
        var debouncer = PlayStateDebouncer()
        for step in 0 ... 3 {
            _ = debouncer.update(isPlaying: true, now: start.addingTimeInterval(Double(step) * 0.25))
        }
        XCTAssertTrue(debouncer.update(isPlaying: true, now: start.addingTimeInterval(1)))
        XCTAssertFalse(debouncer.update(isPlaying: false, now: start.addingTimeInterval(1.01)))
    }

    func testConfirmedPlayingSurvivesASlowPoll() {
        var debouncer = PlayStateDebouncer()
        for step in 0 ... 3 {
            _ = debouncer.update(isPlaying: true, now: start.addingTimeInterval(Double(step) * 0.25))
        }
        XCTAssertTrue(debouncer.update(isPlaying: true, now: start.addingTimeInterval(3)))
        XCTAssertEqual(debouncer.activeSince, start)
    }

    func testActiveSamplesTooFarApartToRuleOutABlinkNeverConfirmPlaying() {
        var debouncer = PlayStateDebouncer()
        for step in 0 ... 8 {
            let now = start.addingTimeInterval(Double(step) * 0.93)
            XCTAssertFalse(debouncer.update(isPlaying: true, now: now), "step \(step)")
        }
    }

    func testDenseSamplesAfterAGapConfirmPlaying() {
        var debouncer = PlayStateDebouncer()
        _ = debouncer.update(isPlaying: true, now: start)
        var confirmed = false
        for step in 0 ... 3 {
            confirmed = debouncer.update(isPlaying: true, now: start.addingTimeInterval(2 + Double(step) * 0.25))
        }
        XCTAssertTrue(confirmed)
        XCTAssertEqual(debouncer.activeSince, start.addingTimeInterval(2))
    }

    func testLabelTickWhileLitConfirmsPlayingAtOnce() {
        var debouncer = PlayStateDebouncer()
        XCTAssertFalse(debouncer.update(isPlaying: true, now: start))
        XCTAssertTrue(debouncer.update(isPlaying: true, timeAdvanced: true, now: start.addingTimeInterval(0.25)))
        XCTAssertEqual(debouncer.activeSince, start)
    }

    func testLabelTickWhileUnlitDoesNotConfirmPlaying() {
        var debouncer = PlayStateDebouncer()
        XCTAssertFalse(debouncer.update(isPlaying: false, timeAdvanced: true, now: start))
        XCTAssertFalse(debouncer.update(isPlaying: true, now: start.addingTimeInterval(0.25)))
    }
}
