import CoreGraphics

public enum PanelSide: UInt8 {
    case left = 0
    case right = 1
}

public enum ButtonSlots {
    public static let count = 8
    public static let usable = [2, 3, 5, 6, 7, 8]
    public static let reserved: [Int: String] = [1: "NI", 4: "NI"]

    public static func slot(forTabIndex index: Int) -> Int? {
        usable.indices.contains(index) ? usable[index] : nil
    }

    public static func tabIndex(forSlot slot: Int) -> Int? {
        usable.firstIndex(of: slot)
    }
}

public enum Metrics {
    public static let width = 480
    public static let height = 272
    public static let stripHeight: CGFloat = 46
    public static let headingTop: CGFloat = 46
    public static let headingHeight: CGFloat = 30
    public static let listTop: CGFloat = 78
    public static let listBottom: CGFloat = 270
    public static let trackRowHeight: CGFloat = 34
    public static let headerRowHeight: CGFloat = 22
    public static let actionRowHeight: CGFloat = 30
    public static let viewportHeight: CGFloat = listBottom - listTop

    public static func rowHeight(_ kind: SidebarRowKind) -> CGFloat {
        switch kind {
        case .track, .unknown: trackRowHeight
        case .header: headerRowHeight
        case .action: actionRowHeight
        }
    }
}

public struct MainSelection: Equatable {
    public let row: SidebarSnapshot.Row
    public let position: Int
    public let total: Int

    public init(row: SidebarSnapshot.Row, position: Int, total: Int) {
        self.row = row
        self.position = position
        self.total = total
    }
}

public struct ViewState: Equatable {
    public private(set) var snapshot: SidebarSnapshot
    public private(set) var selectedIndex: Int?
    public private(set) var scrollTop: CGFloat = 0
    public private(set) var main: MainSelection?

    public mutating func setMain(_ selection: MainSelection?) {
        main = selection
    }

    public mutating func select(rowIndex: Int) {
        guard snapshot.rows.indices.contains(rowIndex), snapshot.rows[rowIndex].isSelectable else { return }
        selectedIndex = rowIndex
        revealSelection()
    }

    public init(snapshot: SidebarSnapshot) {
        self.snapshot = snapshot
        selectedIndex = snapshot.selectedRowIndex ?? snapshot.rows.firstIndex(where: \.isSelectable)
        revealSelection()
    }

    public func rowOffset(_ index: Int) -> CGFloat {
        snapshot.rows[..<index].reduce(0) { $0 + Metrics.rowHeight($1.kind) }
    }

    public var contentHeight: CGFloat {
        rowOffset(snapshot.rows.count)
    }

    public var selectedRow: SidebarSnapshot.Row? {
        selectedIndex.map { snapshot.rows[$0] }
    }

    public var selectedTrack: SidebarSnapshot.Row? {
        selectedRow.flatMap { $0.kind == .track ? $0 : nil }
    }

    public mutating func moveSelection(by delta: Int) {
        let selectable = snapshot.rows.indices.filter { snapshot.rows[$0].isSelectable }
        guard !selectable.isEmpty else { return }
        let position = selectedIndex.flatMap { selectable.firstIndex(of: $0) } ?? 0
        let target = min(max(position + delta, 0), selectable.count - 1)
        selectedIndex = selectable[target]
        revealSelection()
    }

    private mutating func revealSelection() {
        let maxScroll = max(0, contentHeight - Metrics.viewportHeight)
        guard let index = selectedIndex else {
            scrollTop = 0
            return
        }
        let top = rowOffset(index)
        let bottom = top + Metrics.rowHeight(snapshot.rows[index].kind)
        if top < scrollTop {
            scrollTop = top
            if index > 0, snapshot.rows[index - 1].kind == .header {
                scrollTop -= Metrics.headerRowHeight
            }
        } else if bottom > scrollTop + Metrics.viewportHeight {
            scrollTop = bottom - Metrics.viewportHeight
        }
        scrollTop = min(max(scrollTop, 0), maxScroll)
    }
}
