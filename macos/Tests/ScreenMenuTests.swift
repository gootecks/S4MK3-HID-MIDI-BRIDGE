import AppKit
import Combine
import XCTest
@testable import S4Bridge

@MainActor
final class ScreenMenuTests: XCTestCase {
    func testNativeMenuToggleUpdatesTheSamePersistedPreference() async throws {
        _ = NSApplication.shared
        // Given a stopped bridge with enabled screens.
        let agent = TestNIHardwareAgent()
        let model = try makeScreenController(self, agent: agent)
        let target = AppDelegate(controller: model)
        let menu = bridgeStatusMenu(controller: model, state: .stopped, target: target)
        let index = try XCTUnwrap(menu.items.firstIndex { $0.identifier?.rawValue == "screens.enabled" })
        XCTAssertEqual(menu.items[index].state, .on)
        let changed = expectation(description: "native action completed its preference transaction")
        let observation = model.screens.$preferences.combineLatest(model.screens.$busy).sink { settings, busy in
            if !settings.enabled && !busy { changed.fulfill() }
        }
        // When the real NSMenu action is invoked.
        withExtendedLifetime(target) { menu.performActionForItem(at: index) }
        await fulfillment(of: [changed], timeout: 5)
        // Then Settings and the next menu read the same saved value, with no NI checks.
        XCTAssertFalse(model.screens.preferences.enabled)
        XCTAssertFalse(try ScreenPreferences.decode(Data(contentsOf: model.screens.preferencesURL)).enabled)
        XCTAssertEqual(agent.checks, 0)
        let updated = bridgeStatusMenu(controller: model, state: .stopped, target: target)
        target.menuWillOpen(updated)
        XCTAssertEqual(updated.items.first { $0.identifier?.rawValue == "screens.enabled" }?.state, .off)
        XCTAssertFalse(updated.autoenablesItems)
        withExtendedLifetime(observation) {}
    }

    func testBlockedMenuExposesRecoveryAndRestoreRemainsAvailableWhenDisabled() throws {
        // Given requested screens blocked by NI.
        let agent = TestNIHardwareAgent()
        let model = try makeScreenController(self, agent: agent)
        let target = AppDelegate(controller: model)
        model.screens.state = .blocked
        model.screens.bridgeStarted(screens: false, full: true)
        // When the menu is built for the running bridge.
        let blocked = bridgeStatusMenu(controller: model, state: .running, target: target)
        // Then the explicit handover action is exposed and the master toggle stays checked.
        XCTAssertEqual(blocked.items.first { $0.identifier?.rawValue == "screens.retry" }?.isEnabled, true)
        XCTAssertEqual(blocked.items.first { $0.identifier?.rawValue == "screens.enabled" }?.state, .on)
        try model.screens.changePreferences { $0.enabled = false }
        let disabled = bridgeStatusMenu(controller: model, state: .stopped, target: target)
        XCTAssertNil(disabled.items.first { $0.identifier?.rawValue == "screens.retry" })
        XCTAssertEqual(disabled.items.first { $0.identifier?.rawValue == "screens.restore" }?.isEnabled, true)
        XCTAssertEqual(agent.checks, 0)
    }

    func testMenuOpenRefreshesManualRecoveryAfterExternalAgentExit() throws {
        // Given a previously running driver and an already constructed menu.
        let agent = TestNIHardwareAgent()
        agent.running = true
        let model = try makeScreenController(self, agent: agent)
        model.screens.refreshDriverStatus()
        let target = AppDelegate(controller: model)
        let menu = bridgeStatusMenu(controller: model, state: .stopped, target: target)
        XCTAssertEqual(menu.items.first { $0.identifier?.rawValue == "screens.restore" }?.isEnabled, false)
        // When the driver exits outside this app and the user opens that same menu.
        agent.running = false
        target.menuWillOpen(menu)
        // Then its visible recovery action is enabled immediately, not on a second opening.
        XCTAssertEqual(menu.items.first { $0.identifier?.rawValue == "screens.restore" }?.isEnabled, true)
    }

    func testNativeManualRecoveryWorksWithScreensOffAndNoOwnership() async throws {
        _ = NSApplication.shared
        // Given screens off, a non-full profile and no restoration record.
        let agent = TestNIHardwareAgent()
        let model = try makeScreenController(self, agent: agent)
        try model.screens.changePreferences { $0.enabled = false; $0.restoreNIOnStop = false }
        model.run.profile = .minimal
        let target = AppDelegate(controller: model)
        let menu = bridgeStatusMenu(controller: model, state: .stopped, target: target)
        let index = try XCTUnwrap(menu.items.firstIndex { $0.identifier?.rawValue == "screens.restore" })
        let recovered = expectation(description: "native recovery action finished")
        let observation = model.screens.$busy.dropFirst().filter { !$0 }.prefix(1)
            .sink { _ in recovered.fulfill() }
        // When the real dropdown recovery action is invoked.
        withExtendedLifetime(target) { menu.performActionForItem(at: index) }
        await fulfillment(of: [recovered], timeout: 5)
        // Then manual recovery ignores automatic ownership and leaves screen support off.
        XCTAssertEqual(agent.restores, 1)
        XCTAssertFalse(model.screens.preferences.enabled)
        XCTAssertFalse(model.screens.preferences.restorePending)
        withExtendedLifetime(observation) {}
    }
}
