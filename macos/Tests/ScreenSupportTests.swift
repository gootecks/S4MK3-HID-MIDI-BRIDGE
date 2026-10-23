import Foundation
import XCTest
@testable import S4Bridge

@MainActor
final class ScreenSupportTests: XCTestCase {
    func testDisabledScreensSkipAllAutomaticAgentChecks() async throws {
        // Given disabled screen support and a running NI driver.
        let agent = TestNIHardwareAgent()
        agent.running = true
        let support = try makeScreenSupport(self, agent: agent)
        try support.changePreferences { $0.enabled = false }
        // When a bridge starts, the menu refreshes, and it later stops.
        let enabled = try await support.prepare(full: true) { XCTFail("Unexpected prompt"); return .cancel }
        support.refreshDriverStatus()
        await support.bridgeExited(restarting: false)
        // Then controls can run without inspecting or stopping NI.
        XCTAssertEqual(enabled, false)
        XCTAssertEqual(agent.checks, 0)
        XCTAssertEqual(agent.stops, 0)
        XCTAssertEqual(agent.restores, 0)
        XCTAssertEqual(support.state, .off)
    }

    func testNonFullProfilesNeverInspectNI() async throws {
        // Given enabled screen preferences but a non-full profile.
        let agent = TestNIHardwareAgent()
        agent.running = true
        let support = try makeScreenSupport(self, agent: agent)
        // When profile admission is evaluated.
        let enabled = try await support.prepare(full: false) { XCTFail("Unexpected prompt"); return .cancel }
        // Then no screen or NI action is admitted.
        XCTAssertEqual(enabled, false)
        XCTAssertEqual(agent.checks, 0)
        XCTAssertEqual(agent.stops, 0)
    }

    func testDecliningStopKeepsRequestedScreensBlocked() async throws {
        // Given the driver running, with automatic restore enabled but auto-stop off.
        let agent = TestNIHardwareAgent()
        agent.running = true
        let support = try makeScreenSupport(self, agent: agent)
        var prompts = 0
        // When the user chooses to continue without screens.
        let enabled = try await support.prepare(full: true) { prompts += 1; return .withoutScreens }
        support.bridgeStarted(screens: false, full: true)
        support.acceptStatus("S4_SCREEN_STATUS 0 inactive")
        // Then the requested setting stays on, attention stays red, and NI is untouched.
        XCTAssertEqual(enabled, false)
        XCTAssertEqual(prompts, 1)
        XCTAssertTrue(support.preferences.enabled)
        XCTAssertTrue(support.needsAttention)
        XCTAssertEqual(support.state, .blocked)
        XCTAssertEqual(agent.stops, 0)
    }

    func testAutomaticStopRecordsRecoveryBeforeTerminationAndRestoresOnFinalExit() async throws {
        // Given explicit auto-stop consent and a running driver.
        let agent = TestNIHardwareAgent()
        agent.running = true
        let support = try makeScreenSupport(self, agent: agent)
        try support.changePreferences { $0.automaticallyStopNI = true }
        let savedURL = support.preferencesURL
        agent.stopAction = {
            let saved = try ScreenPreferences.decode(Data(contentsOf: savedURL))
            XCTAssertTrue(saved.restorePending)
        }
        // When startup hands over the screen driver.
        let enabled = try await support.prepare(full: true) { XCTFail("Auto-stop should not prompt"); return .cancel }
        // Then recovery responsibility exists before the process exits.
        XCTAssertEqual(enabled, true)
        XCTAssertEqual(agent.stops, 1)
        XCTAssertTrue(support.preferences.restorePending)
        await support.bridgeExited(restarting: false)
        XCTAssertEqual(agent.restores, 1)
        XCTAssertFalse(support.preferences.restorePending)
    }

    func testInternalRestartDoesNotRestoreNIBetweenChildren() async throws {
        // Given a driver stopped by this app.
        let agent = TestNIHardwareAgent()
        agent.running = true
        let support = try makeScreenSupport(self, agent: agent)
        try await support.stopDriver()
        support.bridgeStarted(screens: true, full: true)
        // When Save and restart ends only the old child.
        await support.bridgeExited(restarting: true)
        // Then the replacement can retain screen ownership; final exit restores once.
        XCTAssertEqual(agent.restores, 0)
        XCTAssertTrue(support.preferences.restorePending)
        support.bridgeStarted(screens: true, full: true)
        await support.bridgeExited(restarting: false)
        XCTAssertEqual(agent.restores, 1)
        XCTAssertFalse(support.preferences.restorePending)
    }

    func testOriginallyStoppedDriverIsNotAutomaticallyStarted() async throws {
        // Given NI was already stopped before this app.
        let agent = TestNIHardwareAgent()
        let support = try makeScreenSupport(self, agent: agent)
        // When an enabled screen session starts and exits.
        _ = try await support.prepare(full: true) { XCTFail("Unexpected prompt"); return .cancel }
        await support.bridgeExited(restarting: false)
        // Then automatic restore does not override the user's original state.
        XCTAssertEqual(agent.stops, 0)
        XCTAssertEqual(agent.restores, 0)
        XCTAssertFalse(support.preferences.restorePending)
    }

    func testManualRecoveryWorksWhenDisabledAndUnowned() async throws {
        // Given screens disabled and no restoration record.
        let agent = TestNIHardwareAgent()
        let support = try makeScreenSupport(self, agent: agent)
        try support.changePreferences { $0.enabled = false; $0.restoreNIOnStop = false }
        // When manual recovery is explicitly requested.
        try await support.restoreDriver()
        // Then NI is restarted without requiring ownership or enabled screens.
        XCTAssertEqual(agent.restores, 1)
        XCTAssertTrue(agent.running)
        XCTAssertFalse(support.preferences.restorePending)
    }

    func testRecoveryResponsibilitySurvivesAppRecreationAndRestoreFailure() async throws {
        // Given a recorded stop followed by loss of the original app object.
        let agent = TestNIHardwareAgent()
        agent.running = true
        let support = try makeScreenSupport(self, agent: agent)
        try await support.stopDriver()
        let reloaded = ScreenSupport(supportURL: support.preferencesURL.deletingLastPathComponent(), agent: agent)
        XCTAssertTrue(reloaded.preferences.restorePending)
        agent.restoreError = AppError.invalid("Fixture launch failure")
        // When a recovery attempt fails.
        do { try await reloaded.restoreDriver(); XCTFail("Expected failure") } catch {}
        // Then recovery remains available after another app launch.
        let again = ScreenSupport(supportURL: support.preferencesURL.deletingLastPathComponent(), agent: agent)
        XCTAssertTrue(again.preferences.restorePending)
        XCTAssertFalse(agent.running)
    }
}
