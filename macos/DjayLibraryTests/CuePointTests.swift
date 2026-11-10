@testable import DjayLibrary
import XCTest

final class CuePointTests: XCTestCase {
    private func cue(_ time: Double, comment: String? = nil, color: String? = nil, number: Int64? = nil) -> DjayValue {
        var fields: [String: DjayValue] = ["time": .double(time)]
        if let comment {
            fields["comment"] = .string(comment)
        }
        if let color {
            fields["color"] = .string(color)
        }
        if let number {
            fields["number"] = .int(number)
        }
        return .object(className: "ADCCuePoint", fields: fields)
    }

    func testCuePointsAreSortedAndKeepFields() {
        let user = DjayValue.object(className: nil, fields: [
            "cuePoints": .list([cue(30, comment: "Drop", color: "#FF0000", number: 2), cue(5)])
        ])
        let cues = DjayLibrarySnapshot.makeCuePoints(user)
        XCTAssertEqual(cues.map(\.time), [5, 30])
        XCTAssertEqual(cues[1].name, "Drop")
        XCTAssertEqual(cues[1].colorHex, "#FF0000")
        XCTAssertEqual(cues[1].number, 2)
        XCTAssertNil(cues[0].name)
    }

    func testInvalidAndMissingCuePointsAreDropped() {
        let user = DjayValue.object(className: nil, fields: [
            "cuePoints": .list([cue(-1), .object(className: "Other", fields: ["time": .double(3)]), .null])
        ])
        XCTAssertEqual(DjayLibrarySnapshot.makeCuePoints(user), [])
        XCTAssertEqual(DjayLibrarySnapshot.makeCuePoints(nil), [])
    }
}
