import S8Control
import XCTest

private final class FakeSidebar: SidebarControlling {
    var calls: [String] = []

    func pressTab(index: Int) -> Bool {
        calls.append("tab \(index)")
        return true
    }

    func pressSubtab(index: Int) -> Bool {
        calls.append("subtab \(index)")
        return true
    }

    func moveSelection(by delta: Int) -> Bool {
        calls.append("move \(delta)")
        return true
    }
}

final class S8ControlTests: XCTestCase {
    func testParsesButtonsAndEncoders() {
        XCTAssertEqual(
            ControlEvent.parse("S8_CONTROL display_button left 2 down"),
            .displayButton(side: .left, slot: 2, down: true)
        )
        XCTAssertEqual(
            ControlEvent.parse("S8_CONTROL display_button right 8 up"),
            .displayButton(side: .right, slot: 8, down: false)
        )
        XCTAssertEqual(
            ControlEvent.parse("S8_CONTROL screen_encoder right 3 -120"),
            .screenEncoder(side: .right, encoder: 3, delta: -120)
        )
    }

    func testIgnoresOtherLinesAndMalformedEvents() {
        let lines = [
            "", "2026-11-10 INFO S8_CONTROL display_button left 2 down", "S8_SCREEN_STATUS active",
            "S8_CONTROL display_button left 1 down", "S8_CONTROL display_button left 4 down",
            "S8_CONTROL display_button middle 2 down", "S8_CONTROL display_button left 2 sideways",
            "S8_CONTROL screen_encoder left 5 10", "S8_CONTROL screen_encoder left 1 x",
            "S8_CONTROL screen_encoder left 1", "S8_CONTROL unknown left 2 down"
        ]
        for line in lines {
            XCTAssertNil(ControlEvent.parse(line), line)
        }
    }

    func testAccumulatorKeepsRemainderAndResetsOnDirectionOrEncoder() {
        var accumulator = EncoderAccumulator()
        XCTAssertEqual(accumulator.add(side: .left, encoder: 1, delta: 30), 0)
        XCTAssertEqual(accumulator.add(side: .left, encoder: 1, delta: 30), 1)
        XCTAssertEqual(accumulator.add(side: .left, encoder: 1, delta: 40), 1)
        XCTAssertEqual(accumulator.add(side: .left, encoder: 1, delta: -30), 0)
        XCTAssertEqual(accumulator.add(side: .left, encoder: 1, delta: -30), -1)
        XCTAssertEqual(accumulator.add(side: .left, encoder: 1, delta: 49), 0)
        XCTAssertEqual(accumulator.add(side: .right, encoder: 1, delta: 1), 0)
        XCTAssertEqual(accumulator.add(side: .left, encoder: 1, delta: 49), 0)
        XCTAssertEqual(accumulator.add(side: .left, encoder: 1, delta: 1), 1)
        XCTAssertEqual(accumulator.add(side: .left, encoder: 2, delta: 275), 5)
        XCTAssertEqual(accumulator.add(side: .left, encoder: 2, delta: -300), -6)
    }

    func testRouterMapsButtonsToTabsAndSubtabs() {
        let sidebar = FakeSidebar()
        let router = ControlRouter(sidebar: sidebar)
        let expectedTabs = [2: 0, 3: 1, 5: 2, 6: 3, 7: 4, 8: 5]
        for (slot, index) in expectedTabs.sorted(by: { $0.key < $1.key }) {
            router.handle(line: "S8_CONTROL display_button left \(slot) down")
            router.handle(line: "S8_CONTROL display_button left \(slot) up")
            XCTAssertEqual(sidebar.calls.last, "tab \(index)")
        }
        XCTAssertEqual(sidebar.calls.count, 6)
        sidebar.calls = []
        router.handle(line: "S8_CONTROL display_button right 2 down")
        router.handle(line: "S8_CONTROL display_button right 3 down")
        XCTAssertEqual(sidebar.calls, ["subtab 0", "subtab 1"])
    }

    func testRouterMovesSelectionAtFiftyCountsPerRow() {
        let sidebar = FakeSidebar()
        let router = ControlRouter(sidebar: sidebar)
        XCTAssertTrue(router.handle(line: "S8_CONTROL screen_encoder left 1 20"))
        XCTAssertTrue(router.handle(line: "S8_CONTROL screen_encoder left 1 140"))
        XCTAssertTrue(router.handle(line: "S8_CONTROL screen_encoder left 1 -50"))
        XCTAssertFalse(router.handle(line: "bridge started"))
        XCTAssertEqual(sidebar.calls, ["move 3", "move -1"])
    }
}
