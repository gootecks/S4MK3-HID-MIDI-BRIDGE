import Foundation

public struct FixtureLibrary {
    public let snapshots: [SidebarSnapshot]

    public init(snapshots: [SidebarSnapshot]) {
        self.snapshots = snapshots
    }

    public init(url: URL) throws {
        var isDirectory: ObjCBool = false
        guard FileManager.default.fileExists(atPath: url.path, isDirectory: &isDirectory) else {
            throw CocoaError(.fileNoSuchFile)
        }
        if isDirectory.boolValue {
            let files = try FileManager.default.contentsOfDirectory(at: url, includingPropertiesForKeys: nil)
                .filter { $0.pathExtension == "json" }
                .sorted { $0.lastPathComponent < $1.lastPathComponent }
            snapshots = try files.map(SidebarSnapshot.load)
        } else {
            snapshots = try [SidebarSnapshot.load(url)]
        }
    }

    public func snapshot(tab: String, subtab: String?) -> SidebarSnapshot? {
        snapshots.first { $0.selectedTabID == tab && $0.selectedSubtabID == subtab }
    }
}

public struct SidebarNavigator {
    public let library: FixtureLibrary
    public private(set) var state: ViewState

    public init?(library: FixtureLibrary) {
        guard let first = library.snapshots.first else { return nil }
        self.library = library
        state = ViewState(snapshot: first)
    }

    @discardableResult
    public mutating func pressLeft(slot: Int) -> Bool {
        guard let index = ButtonSlots.tabIndex(forSlot: slot), state.snapshot.tabs.indices.contains(index),
              let next = library.snapshot(tab: state.snapshot.tabs[index].id, subtab: nil)
        else { return false }
        state = ViewState(snapshot: next)
        return true
    }

    @discardableResult
    public mutating func pressRight(slot: Int) -> Bool {
        let current = state.snapshot
        guard let index = ButtonSlots.tabIndex(forSlot: slot), current.subtabs.indices.contains(index),
              let tab = current.selectedTabID
        else { return false }
        let target = current.subtabs[index]
        let subtab = target.selected ? nil : target.id
        guard let next = library.snapshot(tab: tab, subtab: subtab) else { return false }
        state = ViewState(snapshot: next)
        return true
    }

    public mutating func move(by delta: Int) {
        state.moveSelection(by: delta)
    }
}
