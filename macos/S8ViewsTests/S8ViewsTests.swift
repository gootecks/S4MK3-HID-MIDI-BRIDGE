@testable import S8Views
import XCTest

final class SnapshotTests: XCTestCase {
    func testDecodesContractExample() throws {
        let json = """
        {"schema":1,"capturedAt":"2026-10-30T19:22:00Z","sidebarOpen":true,
         "tabs":[{"id":"history","label":"History","selected":true}],
         "subtabs":[{"id":"related","label":"Related","selected":false}],
         "heading":"History",
         "rows":[{"kind":"header","text":"10/28/26, 12:40 AM"},
                 {"kind":"track","title":"Anybody","artist":"Skrillex, ISOxo","bpm":"150","key":"7A",
                  "time":"02:43","selected":false},
                 {"kind":"track","title":"Bare"},
                 {"kind":"action","text":"Show More"}]}
        """
        let snapshot = try SidebarSnapshot.decode(Data(json.utf8))
        XCTAssertEqual(snapshot.tabs, [.init(id: "history", label: "History", selected: true)])
        XCTAssertEqual(snapshot.subtabs.first?.id, "related")
        XCTAssertEqual(snapshot.rows.map(\.kind), [.header, .track, .track, .action])
        XCTAssertEqual(snapshot.rows[1].bpm, "150")
        XCTAssertNil(snapshot.rows[2].artist)
        XCTAssertFalse(snapshot.rows[2].selected)
        XCTAssertEqual(snapshot.rows[3].text, "Show More")
    }

    func testRejectsOtherSchema() {
        XCTAssertThrowsError(try SidebarSnapshot.decode(Data(#"{"schema":2}"#.utf8))) {
            XCTAssertEqual($0 as? SnapshotError, .unsupportedSchema(2))
        }
    }

    func testNumericBpmAndUnknownKind() throws {
        let json = #"{"schema":1,"rows":[{"kind":"banner","text":"x"},{"kind":"track","title":"t","bpm":128}]}"#
        let snapshot = try SidebarSnapshot.decode(Data(json.utf8))
        XCTAssertEqual(snapshot.rows[0].kind, .unknown)
        XCTAssertEqual(snapshot.rows[1].bpm, "128")
    }
}

final class FrameTests: XCTestCase {
    private func solid(_ red: UInt8, _ green: UInt8, _ blue: UInt8) -> Bitmap {
        var rgba = [UInt8]()
        for _ in 0 ..< (Metrics.width * Metrics.height) {
            rgba += [red, green, blue, 255]
        }
        return Bitmap(width: Metrics.width, height: Metrics.height, rgba: rgba)
    }

    func testRgb565KnownColors() {
        func code(_ red: UInt8, _ green: UInt8, _ blue: UInt8) -> UInt16 {
            S8Frame.rgb565(Pixel(red: red, green: green, blue: blue))
        }
        XCTAssertEqual(code(255, 0, 0), 0xF800)
        XCTAssertEqual(code(0, 255, 0), 0x07E0)
        XCTAssertEqual(code(0, 0, 255), 0x001F)
        XCTAssertEqual(code(255, 255, 255), 0xFFFF)
        XCTAssertEqual(code(0, 0, 0), 0x0000)
        XCTAssertEqual(code(0x80, 0x80, 0x80), 0x8410)
    }

    func testFrameSizeAndRedPixelIsF800BigEndian() throws {
        let frame = try XCTUnwrap(S8Frame.make(screen: .left, bitmap: solid(255, 0, 0)))
        XCTAssertEqual(frame.count, 261_148)
        XCTAssertEqual(Array(frame[20 ..< 24]), [0xF8, 0x00, 0xF8, 0x00])
    }

    func testHeaderCommandFooterMatchValidatedBytes() throws {
        let left = try XCTUnwrap(S8Frame.make(screen: .left, bitmap: solid(0, 0, 0)))
        let right = try XCTUnwrap(S8Frame.make(screen: .right, bitmap: solid(0, 0, 0)))
        let header: [UInt8] = [0x84, 0, 0, 0x60, 0, 0, 0, 0, 0, 0, 0, 0, 0x01, 0xE0, 0x01, 0x10]
        XCTAssertEqual(Array(left[0 ..< 16]), header)
        XCTAssertEqual(Array(left[16 ..< 20]), [0x00, 0x00, 0xFF, 0x00])
        XCTAssertEqual(Array(left.suffix(8)), [0x03, 0, 0, 0, 0x40, 0, 0x00, 0])
        XCTAssertEqual(right[2], 0x01)
        XCTAssertEqual(Array(right.suffix(8)), [0x03, 0, 0, 0, 0x40, 0, 0x01, 0])
        XCTAssertEqual(0xFF00 * 4, left.count - 16 - 4 - 8)
    }

    func testPixelsAreRowMajorTopDown() throws {
        var rgba = [UInt8](repeating: 0, count: Metrics.width * Metrics.height * 4)
        func put(_ column: Int, _ row: Int, _ red: UInt8, _ green: UInt8, _ blue: UInt8) {
            let index = (row * Metrics.width + column) * 4
            rgba[index] = red
            rgba[index + 1] = green
            rgba[index + 2] = blue
            rgba[index + 3] = 255
        }
        put(0, 0, 255, 0, 0)
        put(479, 0, 0, 255, 0)
        put(0, 271, 0, 0, 255)
        put(479, 271, 255, 255, 255)
        let bitmap = Bitmap(width: Metrics.width, height: Metrics.height, rgba: rgba)
        let frame = try XCTUnwrap(S8Frame.make(screen: .right, bitmap: bitmap))
        func pixel(_ column: Int, _ row: Int) -> [UInt8] {
            let offset = 20 + (row * Metrics.width + column) * 2
            return Array(frame[offset ..< offset + 2])
        }
        XCTAssertEqual(pixel(0, 0), [0xF8, 0x00])
        XCTAssertEqual(pixel(479, 0), [0x07, 0xE0])
        XCTAssertEqual(pixel(0, 271), [0x00, 0x1F])
        XCTAssertEqual(pixel(479, 271), [0xFF, 0xFF])
        XCTAssertEqual(pixel(1, 0), [0x00, 0x00])
    }

    func testRejectsWrongSize() {
        let small = Bitmap(width: 2, height: 2, rgba: [UInt8](repeating: 0, count: 16))
        XCTAssertNil(S8Frame.make(screen: .left, bitmap: small))
    }
}

final class RenderTests: XCTestCase {
    private func library() throws -> FixtureLibrary {
        let url = try XCTUnwrap(Bundle.module.url(forResource: "Fixtures", withExtension: nil))
        return try FixtureLibrary(url: url)
    }

    func testFixturesDecode() throws {
        XCTAssertEqual(try library().snapshots.count, 6)
    }

    func testRenderedPanelHasBackgroundAndHighlight() throws {
        var navigator = try XCTUnwrap(try SidebarNavigator(library: library()))
        XCTAssertTrue(navigator.pressLeft(slot: 3))
        let bitmap = PanelRenderer.render(.left, navigator.state)
        XCTAssertEqual(bitmap.width, 480)
        XCTAssertEqual(bitmap.height, 272)
        XCTAssertEqual(bitmap.pixel(column: 479, row: 271), Pixel(red: 8, green: 10, blue: 14))
        let rowY = Int(Metrics.listTop) + 32
        XCTAssertEqual(bitmap.pixel(column: 200, row: rowY), Pixel(red: 18, green: 84, blue: 164))
        XCTAssertEqual(bitmap.pixel(column: 125, row: 5), Pixel(red: 0, green: 168, blue: 255))
    }

    func testRightPanelDiffersFromLeftAndChangesWithSelection() throws {
        var navigator = try XCTUnwrap(try SidebarNavigator(library: library()))
        navigator.pressLeft(slot: 3)
        let before = PanelRenderer.render(.right, navigator.state)
        navigator.move(by: 1)
        let after = PanelRenderer.render(.right, navigator.state)
        XCTAssertNotEqual(before, after)
        XCTAssertNotEqual(before, PanelRenderer.render(.left, navigator.state))
    }

    func testSlotMappingSkipsNativeInstrumentsButtons() {
        XCTAssertEqual(ButtonSlots.usable, [2, 3, 5, 6, 7, 8])
        XCTAssertNil(ButtonSlots.tabIndex(forSlot: 1))
        XCTAssertNil(ButtonSlots.tabIndex(forSlot: 4))
        XCTAssertEqual(ButtonSlots.tabIndex(forSlot: 5), 2)
        XCTAssertEqual(ButtonSlots.slot(forTabIndex: 5), 8)
        XCTAssertNil(ButtonSlots.slot(forTabIndex: 6))
    }

    func testNavigatorTabsSubtabsAndSelection() throws {
        var navigator = try XCTUnwrap(try SidebarNavigator(library: library()))
        XCTAssertEqual(navigator.state.snapshot.selectedTabID, "track")
        XCTAssertFalse(navigator.pressLeft(slot: 1))
        XCTAssertFalse(navigator.pressLeft(slot: 4))
        XCTAssertTrue(navigator.pressLeft(slot: 6))
        XCTAssertEqual(navigator.state.snapshot.selectedTabID, "history")
        XCTAssertEqual(navigator.state.selectedIndex, 1)
        navigator.move(by: 3)
        XCTAssertEqual(navigator.state.selectedIndex, 5)
        navigator.move(by: 100)
        XCTAssertEqual(navigator.state.selectedIndex, 8)
        XCTAssertEqual(navigator.state.selectedRow?.kind, .action)
        XCTAssertTrue(navigator.pressLeft(slot: 7))
        XCTAssertTrue(navigator.pressRight(slot: 2))
        XCTAssertEqual(navigator.state.snapshot.selectedSubtabID, "related")
        XCTAssertTrue(navigator.pressRight(slot: 2))
        XCTAssertNil(navigator.state.snapshot.selectedSubtabID)
    }

    func testScrollKeepsSelectionVisible() throws {
        var navigator = try XCTUnwrap(try SidebarNavigator(library: library()))
        navigator.pressLeft(slot: 3)
        XCTAssertEqual(navigator.state.scrollTop, 0)
        navigator.move(by: 12)
        let state = navigator.state
        let index = try XCTUnwrap(state.selectedIndex)
        let top = state.rowOffset(index) - state.scrollTop
        XCTAssertGreaterThanOrEqual(top, 0)
        XCTAssertLessThanOrEqual(top + Metrics.trackRowHeight, Metrics.viewportHeight)
        XCTAssertGreaterThan(state.scrollTop, 0)
    }
}
