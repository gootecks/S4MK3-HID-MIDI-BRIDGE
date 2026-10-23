import Combine
import Foundation
import XCTest
@testable import S4Bridge

@MainActor
final class ScreenProcessTests: XCTestCase {
    func testLiveScreenToggleUsesSameRealChildAndReceivesReleaseAcknowledgement() async throws {
        // Given a real subprocess with the screen-control pipe protocol.
        let fixture = try resources()
        let executable = fixture.appendingPathComponent("s4mk3-hid-midi-bridge")
        let script = """
        #!/bin/sh
        trap 'exit 0' INT
        printf 'S4_SCREEN_STATUS 0 active\\n'
        while IFS=' ' read -r prefix id action; do
          [ "$prefix" = S4_SCREEN_CONTROL ] || exit 2
          case "$action" in
            OFF) printf 'S4_SCREEN_STATUS %s inactive\\n' "$id";;
            ON|RETRY) printf 'S4_SCREEN_STATUS %s active\\n' "$id";;
            *) exit 3;;
          esac
        done
        """
        try Data(script.utf8).write(to: executable)
        try FileManager.default.setAttributes([.posixPermissions: 0o755], ofItemAtPath: executable.path)
        let agent = TestNIHardwareAgent()
        let model = try BridgeController(resourceURL: fixture, supportURL: fixture.appendingPathComponent("support"), agent: agent)
        let initial = expectation(description: "initial active acknowledgement")
        let disabled = expectation(description: "live off acknowledgement")
        let resumed = expectation(description: "live on acknowledgement")
        let ended = expectation(description: "real child exited after signal")
        var phase = 0
        let screens = model.screens.$state.combineLatest(model.screens.$busy).sink { state, busy in
            guard !busy else { return }
            if state == .active && phase == 0 { phase = 1; initial.fulfill() }
            else if state == .off && phase == 1 { phase = 2; disabled.fulfill() }
            else if state == .active && phase == 2 { phase = 3; resumed.fulfill() }
        }
        let state = model.$state.sink { value in
            if phase == 3 && value == .stopped { ended.fulfill() }
        }
        // When screens are disabled and enabled during the same bridge session.
        model.start()
        await fulfillment(of: [initial], timeout: 10)
        let starts = model.log.components(separatedBy: "LAUNCH:").count
        model.setScreensEnabled(false)
        await fulfillment(of: [disabled], timeout: 10)
        model.setScreensEnabled(true)
        await fulfillment(of: [resumed], timeout: 10)
        // Then no whole-bridge restart occurred and the process still shuts down normally.
        XCTAssertEqual(model.log.components(separatedBy: "LAUNCH:").count, starts)
        XCTAssertEqual(model.state, .running)
        XCTAssertEqual(agent.stops, 0)
        model.stop()
        await fulfillment(of: [ended], timeout: 10)
        withExtendedLifetime((screens, state)) {}
        XCTAssertFalse(model.isBusy)
    }

    func testQuitDuringDriverHandoverRestoresBeforeReplyAndNeverLaunchesChild() async throws {
        // Given a startup waiting for actual NI termination.
        let fixture = try resources()
        let agent = TestNIHardwareAgent()
        agent.running = true
        let model = try BridgeController(resourceURL: fixture, supportURL: fixture.appendingPathComponent("support"), agent: agent)
        try model.screens.changePreferences { $0.automaticallyStopNI = true }
        let requested = expectation(description: "driver termination request reached its event gate")
        let replied = expectation(description: "quit replied after restoration")
        var exitDriver: CheckedContinuation<Void, Never>?
        agent.stopAction = {
            await withCheckedContinuation { continuation in
                exitDriver = continuation
                requested.fulfill()
            }
        }
        model.replyToQuit = { allowed in
            XCTAssertTrue(allowed)
            XCTAssertEqual(agent.restores, 1)
            replied.fulfill()
        }
        // When Quit arrives before NI has exited.
        model.start()
        await fulfillment(of: [requested], timeout: 10)
        XCTAssertEqual(model.prepareToQuit(), .terminateLater)
        XCTAssertEqual(agent.restores, 0)
        exitDriver?.resume()
        await fulfillment(of: [replied], timeout: 10)
        // Then cancellation never starts the hardware bridge and leaves no restoration debt.
        XCTAssertFalse(model.log.contains("LAUNCH:"))
        XCTAssertFalse(model.screens.preferences.restorePending)
        XCTAssertFalse(model.isBusy)
        agent.stopAction = nil
    }

    func testChildExitDuringLiveOffSharesRestorationAndKeepsQuitWaiting() async throws {
        // Given active screens and an owned NI restoration responsibility.
        let fixture = try resources()
        let executable = fixture.appendingPathComponent("s4mk3-hid-midi-bridge")
        let script = """
        #!/bin/sh
        trap 'exit 0' INT
        printf 'S4_SCREEN_STATUS 0 active\\n'
        while IFS=' ' read -r prefix id action; do
          [ "$action" != OFF ] || exit 0
        done
        """
        try Data(script.utf8).write(to: executable)
        try FileManager.default.setAttributes([.posixPermissions: 0o755], ofItemAtPath: executable.path)
        let agent = TestNIHardwareAgent()
        agent.running = true
        let model = try BridgeController(resourceURL: fixture, supportURL: fixture.appendingPathComponent("support"), agent: agent)
        try model.screens.changePreferences { $0.automaticallyStopNI = true }
        let active = expectation(description: "active screens before child exit")
        let restoring = expectation(description: "single NI restoration requested")
        let quit = expectation(description: "quit waits for both cleanup transactions")
        var releaseRestore: CheckedContinuation<Void, Never>?
        agent.restoreAction = {
            await withCheckedContinuation { continuation in
                releaseRestore = continuation
                restoring.fulfill()
            }
        }
        model.replyToQuit = { allowed in
            XCTAssertTrue(allowed)
            XCTAssertEqual(agent.restores, 1)
            quit.fulfill()
        }
        let observation = model.screens.$state.combineLatest(model.screens.$busy)
            .filter { $0 == .active && !$1 }.prefix(1).sink { _ in active.fulfill() }
        model.start()
        await fulfillment(of: [active], timeout: 10)
        // When OFF instead causes child exit while a separate live-disable task is waiting.
        model.setScreensEnabled(false)
        await fulfillment(of: [restoring], timeout: 10)
        // Then there is one shared native launch, and Quit cannot bypass ongoing cleanup.
        XCTAssertTrue(model.isBusy)
        XCTAssertEqual(agent.restores, 1)
        XCTAssertEqual(model.prepareToQuit(), .terminateLater)
        releaseRestore?.resume()
        await fulfillment(of: [quit], timeout: 10)
        XCTAssertFalse(model.isBusy)
        XCTAssertFalse(model.screens.preferences.restorePending)
        withExtendedLifetime(observation) {}
        agent.restoreAction = nil
    }

    private func resources() throws -> URL {
        let root = URL(fileURLWithPath: #filePath)
            .deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
        let fixture = FileManager.default.temporaryDirectory
            .appendingPathComponent("S4-Screen-Process-\(UUID().uuidString)", isDirectory: true)
        try FileManager.default.createDirectory(at: fixture, withIntermediateDirectories: true)
        for name in ["led", "jog", "tension"] {
            try Data(contentsOf: root.appendingPathComponent("examples/\(name)-config.json"))
                .write(to: fixture.appendingPathComponent("default-\(name)-config.json"))
        }
        addTeardownBlock { try FileManager.default.removeItem(at: fixture) }
        return fixture
    }
}
