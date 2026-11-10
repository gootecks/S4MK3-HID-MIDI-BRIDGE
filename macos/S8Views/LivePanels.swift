import CoreGraphics
import Foundation

public struct LivePanels {
    private struct List: Equatable {
        let tab: String?
        let subtab: String?
    }

    private var list: List?
    private var scrollTop: CGFloat = 0

    public init() {}

    public mutating func state(for snapshot: SidebarSnapshot) -> ViewState {
        let shown = List(tab: snapshot.selectedTabID, subtab: snapshot.selectedSubtabID)
        var state = ViewState(live: snapshot, scrollTop: shown == list ? scrollTop : 0)
        state.setMain(Self.selection(in: snapshot, at: state.selectedIndex))
        list = shown
        scrollTop = state.scrollTop
        return state
    }

    static func selection(in snapshot: SidebarSnapshot, at index: Int?) -> MainSelection? {
        guard let index, snapshot.rows[index].kind == .track else { return nil }
        let tracks = snapshot.rows.indices.filter { snapshot.rows[$0].kind == .track }
        guard let position = tracks.firstIndex(of: index) else { return nil }
        return MainSelection(row: snapshot.rows[index], position: position + 1, total: tracks.count)
    }
}

public struct PanelFeed {
    private var shown: [PanelSide: Bitmap] = [:]

    public init() {}

    public mutating func records(state: ViewState, deck: DeckSnapshot? = nil) -> [Data] {
        let right = deck.map(DeckPanelRenderer.render) ?? PanelRenderer.render(.right, state)
        return records(left: PanelRenderer.render(.left, state), right: right)
    }

    public mutating func records(left: Bitmap, right: Bitmap) -> [Data] {
        var records: [Data] = []
        for (side, bitmap) in [(PanelSide.left, left), (PanelSide.right, right)] where shown[side] != bitmap {
            guard let record = S8Record.make(panel: side, bitmap: bitmap) else { continue }
            shown[side] = bitmap
            records.append(record)
        }
        return records
    }
}
