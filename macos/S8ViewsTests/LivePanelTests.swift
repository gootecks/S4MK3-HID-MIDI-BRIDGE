@testable import S8Views
import XCTest

private func sidebar(tab: String = "library", selectedTrack: Int? = nil, action: Bool = false) -> SidebarSnapshot {
    var rows = [SidebarSnapshot.Row(kind: .header, text: "Today")]
    rows += (0 ..< 12).map { (index: Int) in
        SidebarSnapshot.Row(kind: .track, title: "Track \(index)", artist: "Artist", selected: index == selectedTrack)
    }
    rows.append(SidebarSnapshot.Row(kind: .action, text: "Show More", selected: action))
    return SidebarSnapshot(tabs: [.init(id: tab, label: tab, selected: true)], rows: rows)
}

final class RecordTests: XCTestCase {
    private func solid(_ red: UInt8, _ green: UInt8, _ blue: UInt8) -> Bitmap {
        var rgba = [UInt8]()
        for _ in 0 ..< (Metrics.width * Metrics.height) {
            rgba += [red, green, blue, 255]
        }
        return Bitmap(width: Metrics.width, height: Metrics.height, rgba: rgba)
    }

    func testRecordHeaderLengthAndBigEndianPixels() throws {
        let record = try XCTUnwrap(S8Record.make(panel: .right, bitmap: solid(255, 0, 0)))
        XCTAssertEqual(record.count, 12 + 261_120)
        XCTAssertEqual(Array(record.prefix(12)), [0x53, 0x38, 0x46, 0x52, 1, 0, 0, 0, 0x00, 0xFC, 0x03, 0x00])
        XCTAssertEqual(Array(record[12 ..< 14]), [0xF8, 0x00])
        XCTAssertEqual(Array(record.suffix(2)), [0xF8, 0x00])
    }

    func testRecordPixelsEqualFramePixels() throws {
        let bitmap = PanelRenderer.render(.left, ViewState(snapshot: sidebar(selectedTrack: 2)))
        let record = try XCTUnwrap(S8Record.make(panel: .left, bitmap: bitmap))
        let frame = try XCTUnwrap(S8Frame.make(screen: .left, bitmap: bitmap))
        let start = S8Frame.headerSize + S8Frame.commandSize
        XCTAssertEqual(Array(record.dropFirst(S8Record.headerSize)), Array(frame[start ..< start + S8Frame.pixelBytes]))
    }

    func testRecordRejectsWrongSize() {
        let small = Bitmap(width: 2, height: 2, rgba: [UInt8](repeating: 0, count: 16))
        XCTAssertNil(S8Record.make(panel: .left, bitmap: small))
    }
}

final class LivePanelsTests: XCTestCase {
    func testShowsNoSelectionWhenDjayHasNone() {
        var panels = LivePanels()
        let state = panels.state(for: sidebar())
        XCTAssertNil(state.selectedIndex)
        XCTAssertNil(state.main)
        XCTAssertEqual(state.scrollTop, 0)
    }

    func testSelectedTrackIsCountedAmongTracksOnly() {
        var panels = LivePanels()
        let state = panels.state(for: sidebar(selectedTrack: 4))
        XCTAssertEqual(state.selectedIndex, 5)
        XCTAssertEqual(state.main?.row.title, "Track 4")
        XCTAssertEqual(state.main?.position, 5)
        XCTAssertEqual(state.main?.total, 12)
    }

    func testSelectedActionHasNoTrackDetail() {
        var panels = LivePanels()
        let state = panels.state(for: sidebar(action: true))
        XCTAssertEqual(state.selectedIndex, 13)
        XCTAssertNil(state.main)
    }

    func testSelectionDoesNotOutliveItsList() {
        var panels = LivePanels()
        XCTAssertNotNil(panels.state(for: sidebar(selectedTrack: 4)).main)
        XCTAssertNil(panels.state(for: sidebar(tab: "queue")).main)
    }

    func testScrollStaysWhileSelectionIsVisibleAndResetsOnAnotherList() {
        var panels = LivePanels()
        let bottom = panels.state(for: sidebar(selectedTrack: 11))
        XCTAssertEqual(bottom.scrollTop, 238)
        let above = panels.state(for: sidebar(selectedTrack: 10))
        XCTAssertEqual(above.scrollTop, 238)
        XCTAssertEqual(ViewState(live: sidebar(selectedTrack: 10)).scrollTop, 204)
        let other = panels.state(for: sidebar(tab: "queue", selectedTrack: 0))
        XCTAssertEqual(other.scrollTop, 0)
    }
}

final class PanelFeedTests: XCTestCase {
    private func panel(_ record: Data) -> UInt8 {
        record[record.startIndex + 4]
    }

    func testSendsBothPanelsFirstThenOnlyChanges() {
        var panels = LivePanels()
        var feed = PanelFeed()
        let first = panels.state(for: sidebar(selectedTrack: 1))
        XCTAssertEqual(feed.records(state: first).map(panel), [0, 1])
        XCTAssertEqual(feed.records(state: first).count, 0)
        let moved = panels.state(for: sidebar(selectedTrack: 2))
        XCTAssertEqual(feed.records(state: moved).map(panel), [0, 1])
    }

    func testDeckPageReplacesRightPanelOnly() throws {
        var panels = LivePanels()
        var feed = PanelFeed()
        let state = panels.state(for: sidebar(selectedTrack: 1))
        _ = feed.records(state: state)
        let deck = feed.records(state: state, deck: DeckSnapshot(deck: 1))
        XCTAssertEqual(deck.map(panel), [1])
        let page = DeckPanelRenderer.render(DeckSnapshot(deck: 1))
        let expected = try XCTUnwrap(S8Record.make(panel: .right, bitmap: page))
        XCTAssertEqual(
            Array(deck[0].dropFirst(S8Record.headerSize)),
            Array(expected.dropFirst(S8Record.headerSize))
        )
        XCTAssertEqual(feed.records(state: state, deck: DeckSnapshot(deck: 1)).count, 0)
        XCTAssertEqual(feed.records(state: state).map(panel), [1])
    }
}
