import Combine
import Foundation
import XCTest
@testable import S4Bridge

@MainActor
final class ScreenRestoreConfirmationTests: XCTestCase {
    func testManualRestoreStillRunsIfChildExitsWhileConfirmationIsOpen() async throws {
        // Given a real child and an NI driver that this app did not stop.
        let agent = TestNIHardwareAgent()
        let model = try makeScreenController(self, agent: agent)
        let script = """
        #!/bin/sh
        trap 'exit 0' INT
        printf 'S4_SCREEN_STATUS 0 active\\n'
        while read -r line; do :; done
        """
        try Data(script.utf8).write(to: model.executableURL)
        try FileManager.default.setAttributes([.posixPermissions: 0o755], ofItemAtPath: model.executableURL.path)
        let active = expectation(description: "real child active")
        let asking = expectation(description: "manual restoration confirmation opened")
        let exited = expectation(description: "child exited during confirmation")
        let restored = expectation(description: "manual recovery completed")
        let running = model.screens.$state.combineLatest(model.screens.$busy)
            .filter { $0 == .active && !$1 }.prefix(1).sink { _ in active.fulfill() }
        var consent: CheckedContinuation<Bool, Never>?
        model.confirmDriverRestore = {
            await withCheckedContinuation { continuation in
                consent = continuation
                asking.fulfill()
            }
        }
        model.start()
        await fulfillment(of: [active], timeout: 10)
        let stopped = model.$state.filter { $0 == .stopped }.prefix(1).sink { _ in exited.fulfill() }
        let finished = model.screens.$busy.dropFirst().filter { !$0 }.prefix(1).sink { _ in restored.fulfill() }
        // When the existing child exits before the user answers the confirmation.
        model.restartNIDriver()
        await fulfillment(of: [asking], timeout: 10)
        model.stop()
        await fulfillment(of: [exited], timeout: 10)
        XCTAssertFalse(model.hasChildProcess)
        XCTAssertTrue(model.isBusy)
        XCTAssertEqual(agent.restores, 0)
        consent?.resume(returning: true)
        await fulfillment(of: [restored], timeout: 10)
        // Then the explicit recovery action still starts NI instead of queuing on a dead child.
        XCTAssertEqual(agent.restores, 1)
        XCTAssertFalse(model.screens.restoreAfterExit)
        XCTAssertFalse(model.isBusy)
        withExtendedLifetime((running, stopped, finished)) {}
    }
}
