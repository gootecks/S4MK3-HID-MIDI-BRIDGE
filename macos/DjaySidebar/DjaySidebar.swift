import Foundation
import S8Views

public final class DjaySidebar {
    public static let rowLimit = 300

    private let root: AXRoot
    private let settle: (() -> Bool) -> Bool

    public init(root: AXRoot, settle: @escaping (() -> Bool) -> Bool = DjaySidebar.waitUpTo2Seconds) {
        self.root = root
        self.settle = settle
    }

    public static func waitUpTo2Seconds(_ condition: () -> Bool) -> Bool {
        let start = Date()
        while Date().timeIntervalSince(start) < 2 {
            if condition() {
                return true
            }
            usleep(40000)
        }
        return condition()
    }

    private func elements() -> SidebarElements? {
        root.window().flatMap { SidebarElements.find(in: $0) }
    }

    public func snapshot(limit: Int = DjaySidebar.rowLimit) -> SidebarSnapshot {
        guard let found = elements() else { return SidebarReader.closedSnapshot() }
        return SidebarReader.snapshot(found, limit: limit)
    }

    @discardableResult
    public func pressTab(index: Int) -> Bool {
        guard let found = elements(), found.tabs.indices.contains(index) else { return false }
        let tab = found.tabs[index]
        if SidebarReader.selectedTabID(found) == tab.id {
            return true
        }
        let pressed = tab.node.press()
        let switched = settle { self.elements().flatMap(SidebarReader.selectedTabID) == tab.id }
        return pressed || switched
    }

    @discardableResult
    public func pressSubtab(index: Int) -> Bool {
        guard let found = elements() else { return false }
        let buttons = SidebarReader.subtabButtons(found)
        let subs = SidebarReader.subtabs(found)
        guard buttons.indices.contains(index), subs.indices.contains(index) else { return false }
        if subs[index].selected {
            return true
        }
        let label = subs[index].id
        let pressed = buttons[index].press()
        let switched = settle {
            self.elements().map { SidebarReader.subtabs($0).first(where: \.selected)?.id == label } ?? false
        }
        return pressed || switched
    }

    @discardableResult
    public func moveSelection(by delta: Int) -> Bool {
        guard delta != 0, let found = elements(),
              let table = found.scroll.children.first(where: { $0.role == "AXTable" })
        else { return false }
        let rows = RowReader.tableRows(table, limit: DjaySidebar.rowLimit).filter(\.row.isSelectable)
        let current = rows.firstIndex { $0.row.selected }
        guard let target = Self.targetIndex(count: rows.count, current: current, delta: delta)
        else { return false }
        return table.selectRows([rows[target].node])
    }

    public static func targetIndex(count: Int, current: Int?, delta: Int) -> Int? {
        guard count > 0, delta != 0 else { return nil }
        let base = current ?? (delta > 0 ? -1 : count)
        return min(max(base + delta, 0), count - 1)
    }
}
