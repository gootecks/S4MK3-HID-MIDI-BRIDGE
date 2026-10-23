import Foundation
import XCTest
@testable import S4Bridge

final class ScreenPreferencesTests: XCTestCase {
    func testPartialSettingsPreserveExistingScreenDefaults() throws {
        // Given old installations with no new screen settings.
        let defaults = try ScreenPreferences.decode(Data("{}".utf8))
        // When a partial automation setting is decoded.
        let changed = try ScreenPreferences.decode(Data(#"{"automaticallyStopNI":true}"#.utf8))
        // Then screens stay enabled, automatic stopping is opt-in and recovery is retained.
        XCTAssertTrue(defaults.enabled)
        XCTAssertFalse(defaults.automaticallyStopNI)
        XCTAssertTrue(defaults.restoreNIOnStop)
        XCTAssertFalse(defaults.restorePending)
        XCTAssertTrue(changed.automaticallyStopNI)
        XCTAssertEqual(changed.enabled, defaults.enabled)
        XCTAssertEqual(changed.restoreNIOnStop, defaults.restoreNIOnStop)
    }

    func testMalformedScreenSettingsAreRejected() {
        // Given invalid shapes, types and unknown keys.
        for source in ["[]", #"{"enabled":1}"#, #"{"restoreNIOnStop":"yes"}"#, #"{"other":true}"#] {
            // When settings cross the JSON boundary.
            // Then invalid values cannot silently enable automatic process control.
            XCTAssertThrowsError(try ScreenPreferences.decode(Data(source.utf8)))
        }
    }

    @MainActor
    func testInvalidSavedSettingsRemainUntilExplicitReset() throws {
        // Given invalid saved settings.
        let agent = TestNIHardwareAgent()
        let support = try makeScreenSupport(self, agent: agent)
        let invalid = Data(#"{"enabled":"yes"}"#.utf8)
        try invalid.write(to: support.preferencesURL)
        let loaded = ScreenSupport(supportURL: support.preferencesURL.deletingLastPathComponent(), agent: agent)
        // When an ordinary save is attempted.
        XCTAssertThrowsError(try loaded.save())
        // Then the file is preserved and no NI process is inspected.
        XCTAssertEqual(try Data(contentsOf: support.preferencesURL), invalid)
        XCTAssertEqual(agent.checks, 0)
        loaded.reset()
        try loaded.save()
        XCTAssertEqual(try ScreenPreferences.decode(Data(contentsOf: support.preferencesURL)), ScreenPreferences())
    }
}
