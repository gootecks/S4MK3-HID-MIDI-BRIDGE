import Combine
import Foundation
import XCTest
@testable import S4Bridge

@MainActor
final class ScreenControlTests: XCTestCase {
    func testLiveOffWaitsForReleaseAndKeepsNIRestoreUntilBridgeExit() async throws {
        // Given active screens and an NI driver stopped by this app.
        let agent = TestNIHardwareAgent()
        agent.running = true
        let support = try makeScreenSupport(self, agent: agent)
        try await support.stopDriver()
        support.bridgeStarted(screens: true, full: true)
        support.acceptStatus("S4_SCREEN_STATUS 0 active")
        let command = expectation(description: "OFF sent through live control")
        var request: UInt64 = 0
        support.sendCommand = { id, action in
            XCTAssertEqual(action, .off)
            request = id
            command.fulfill()
        }
        // When disable is requested but release has not yet been acknowledged.
        let disabling = Task { try await support.setEnabled(false) { .cancel } }
        await fulfillment(of: [command], timeout: 5)
        support.acceptStatus("S4_SCREEN_STATUS 0 inactive")
        support.acceptStatus("S4_SCREEN_STATUS 0 active")
        // Then stale messages do not restore NI against a possibly held interface.
        XCTAssertEqual(agent.restores, 0)
        XCTAssertEqual(support.state, .releasing)
        support.acceptStatus("S4_SCREEN_STATUS \(request) inactive")
        try await disabling.value
        XCTAssertEqual(agent.restores, 0)
        XCTAssertEqual(support.state, .off)
        XCTAssertFalse(support.needsAttention)
        XCTAssertTrue(support.preferences.restorePending)
        await support.bridgeExited(restarting: false)
        XCTAssertEqual(agent.restores, 1)
        XCTAssertFalse(support.preferences.restorePending)
    }

    func testDeclinedLiveEnableCanBeRecoveredWithoutRestartingBridge() async throws {
        // Given a running full bridge with screens off and NI running.
        let agent = TestNIHardwareAgent()
        agent.running = true
        let support = try makeScreenSupport(self, agent: agent)
        try support.changePreferences { $0.enabled = false }
        support.bridgeStarted(screens: false, full: true)
        var commands: [(UInt64, ScreenCommand)] = []
        support.sendCommand = { commands.append(($0, $1)) }
        // When consent is declined, then the explicit recovery action is chosen.
        try await support.setEnabled(true) { .withoutScreens }
        XCTAssertTrue(support.needsAttention)
        XCTAssertTrue(commands.isEmpty)
        try await support.enableAfterConsent()
        // Then the same bridge receives a live retry with a fresh generation.
        XCTAssertEqual(agent.stops, 1)
        XCTAssertEqual(commands.count, 1)
        XCTAssertEqual(commands[0].1, .retry)
        XCTAssertTrue(support.bridgeRunning)
        support.acceptStatus("S4_SCREEN_STATUS \(commands[0].0) active")
        XCTAssertEqual(support.state, .active)
        XCTAssertFalse(support.needsAttention)
    }

    func testCancelledEnableRestoresPreviousMasterSetting() async throws {
        // Given a running bridge whose screens are disabled.
        let agent = TestNIHardwareAgent()
        agent.running = true
        let support = try makeScreenSupport(self, agent: agent)
        try support.changePreferences { $0.enabled = false }
        support.bridgeStarted(screens: false, full: true)
        support.sendCommand = { _, _ in XCTFail("Cancelled enable must not issue a command") }
        // When the user cancels the handover.
        try await support.setEnabled(true) { .cancel }
        // Then the previous preference and NI process remain unchanged.
        XCTAssertFalse(support.preferences.enabled)
        XCTAssertEqual(agent.stops, 0)
        XCTAssertEqual(support.state, .off)
    }

    func testRequestedScreenFailureIsRedButDisabledScreensAreNot() throws {
        // Given requested screens with a real screen-service failure.
        let agent = TestNIHardwareAgent()
        let support = try makeScreenSupport(self, agent: agent)
        support.bridgeStarted(screens: true, full: true)
        // When the current worker reports failure.
        support.acceptStatus("S4_SCREEN_STATUS 0 failed")
        // Then attention is shown without changing bridge-running ownership.
        XCTAssertTrue(support.needsAttention)
        XCTAssertTrue(support.bridgeRunning)
        try support.changePreferences { $0.enabled = false }
        XCTAssertFalse(support.needsAttention)
    }
}
