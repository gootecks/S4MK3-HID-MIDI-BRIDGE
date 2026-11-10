@testable import DjaySidebar
import S8Views
import XCTest

final class DjaySidebarTests: XCTestCase {
    private func sidebar(_ root: FakeRoot) -> DjaySidebar {
        DjaySidebar(root: root, settle: { $0() })
    }

    func testLibraryTreeBecomesSnapshot() {
        let tree = Tree.library(selected: 1)
        let snapshot = sidebar(FakeRoot(tree.window)).snapshot()
        XCTAssertTrue(snapshot.sidebarOpen)
        XCTAssertEqual(snapshot.tabs.map(\.id), ["library", "queue", "match", "session", "info"])
        XCTAssertEqual(snapshot.selectedTabID, "library")
        XCTAssertEqual(snapshot.heading, "Library")
        XCTAssertEqual(snapshot.rows.count, 4)
        XCTAssertEqual(snapshot.rows[1].title, "Two")
        XCTAssertEqual(snapshot.rows[1].artist, "B")
        XCTAssertEqual(snapshot.rows[1].bpm, "124")
        XCTAssertEqual(snapshot.rows[1].key, "9A")
        XCTAssertEqual(snapshot.selectedRowIndex, 1)
    }

    func testInfoTreeHasSubtabsWithInfoSelected() {
        let tree = Tree.infoRelated()
        let snapshot = sidebar(FakeRoot(tree.window)).snapshot()
        XCTAssertEqual(snapshot.selectedTabID, "info")
        XCTAssertEqual(snapshot.subtabs.map(\.id), ["info", "related"])
        XCTAssertEqual(snapshot.selectedSubtabID, "info")
    }

    func testMissingWindowOrSidebarGivesClosedSnapshot() {
        XCTAssertFalse(sidebar(FakeRoot(nil)).snapshot().sidebarOpen)
        let bare = FakeNode("AXWindow", children: [FakeNode("AXGroup")])
        XCTAssertFalse(sidebar(FakeRoot(bare)).snapshot().sidebarOpen)
    }

    func testPressTabPressesTheNthButton() {
        let tree = Tree.library()
        let root = FakeRoot(tree.window)
        var pressed: [Int] = []
        for (index, button) in tree.tabs.enumerated() {
            button.onPress = {
                pressed.append(index)
                return true
            }
        }
        let controller = sidebar(root)
        XCTAssertTrue(controller.pressTab(index: 3))
        XCTAssertEqual(pressed, [3])
        XCTAssertTrue(controller.pressTab(index: 0))
        XCTAssertEqual(pressed, [3], "the library tab is already selected")
        XCTAssertFalse(controller.pressTab(index: 5))
    }

    func testPressTabSucceedsWhenContentSwitchesDespiteErrorCode() {
        let tree = Tree.library()
        let root = FakeRoot(tree.window)
        tree.tabs[1].onPress = {
            root.node = Tree.window(title: "Queue", scrollChildren: []).window
            return false
        }
        XCTAssertTrue(sidebar(root).pressTab(index: 1))
    }

    func testPressTabFailsWhenPressFailsAndContentStays() {
        let tree = Tree.library()
        tree.tabs[2].onPress = { false }
        XCTAssertFalse(sidebar(FakeRoot(tree.window)).pressTab(index: 2))
    }

    func testPressSubtabSwitchesToRelated() {
        let tree = Tree.infoRelated()
        let root = FakeRoot(tree.window)
        tree.relatedButton.onPress = {
            root.node = Tree.infoRelated(relatedSelected: true).window
            return true
        }
        let controller = sidebar(root)
        XCTAssertTrue(controller.pressSubtab(index: 1))
        XCTAssertEqual(sidebar(root).snapshot().selectedSubtabID, "related")
        XCTAssertFalse(controller.pressSubtab(index: 2))
        XCTAssertFalse(sidebar(FakeRoot(Tree.library().window)).pressSubtab(index: 0))
    }

    func testMoveSelectionSelectsTheTargetRowOnTheTable() {
        let tree = Tree.library(selected: 1)
        let table = tree.table
        let controller = sidebar(FakeRoot(tree.window))
        XCTAssertTrue(controller.moveSelection(by: 2))
        XCTAssertEqual(table.selectedRows.count, 1)
        XCTAssertTrue(table.selectedRows[0] === table.children[3] as AnyObject)
        XCTAssertTrue(controller.moveSelection(by: -10))
        XCTAssertTrue(table.selectedRows[0] === table.children[0] as AnyObject)
        XCTAssertFalse(controller.moveSelection(by: 0))
    }

    func testMoveSelectionWithoutSelectionStartsAtEnds() {
        XCTAssertEqual(DjaySidebar.targetIndex(count: 4, current: nil, delta: 1), 0)
        XCTAssertEqual(DjaySidebar.targetIndex(count: 4, current: nil, delta: 3), 2)
        XCTAssertEqual(DjaySidebar.targetIndex(count: 4, current: nil, delta: -1), 3)
        XCTAssertEqual(DjaySidebar.targetIndex(count: 4, current: 3, delta: 5), 3)
        XCTAssertNil(DjaySidebar.targetIndex(count: 0, current: nil, delta: 1))
    }

    func testMoveSelectionSkipsNonSelectableRows() {
        let tree = Tree.library(selected: 0)
        let table = tree.table
        table.children.insert(Tree.headerRow("Section", top: 190), at: 1)
        let controller = sidebar(FakeRoot(tree.window))
        XCTAssertTrue(controller.moveSelection(by: 1))
        XCTAssertTrue(table.selectedRows[0] === table.children[2] as AnyObject)
    }

    func testMoveSelectionNeedsATable() {
        let tree = Tree.infoRelated()
        XCTAssertFalse(sidebar(FakeRoot(tree.window)).moveSelection(by: 1))
    }
}
