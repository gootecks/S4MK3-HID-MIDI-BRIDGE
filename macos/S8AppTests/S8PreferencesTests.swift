@testable import S8App
import XCTest

final class S8PreferencesTests: XCTestCase {
    private let bridge = URL(fileURLWithPath: "/b")

    func testDeckChoiceNeedsPanels() {
        let preferences = S8Preferences(layout: .standard, leds: false, panels: false, rightPanelDeck: 3)
        XCTAssertEqual(preferences.hostArguments(bridge: bridge), ["--bridge", "/b", "--layout", "default"])
    }

    func testPreferencesSurviveRelaunch() throws {
        let defaults = try XCTUnwrap(UserDefaults(suiteName: "S8PreferencesTests-\(UUID().uuidString)"))
        XCTAssertEqual(S8Preferences(defaults: defaults), S8Preferences())
        let chosen = S8Preferences(layout: .evan, leds: false, panels: true, rightPanelDeck: 4, startOnLaunch: true)
        chosen.save(to: defaults)
        XCTAssertEqual(S8Preferences(defaults: defaults), chosen)
    }
}
