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

    public func landing(tab: String) -> SidebarSnapshot? {
        let candidates = snapshots.filter { $0.selectedTabID == tab }
        return candidates.first { $0.selectedSubtabID == nil || $0.selectedSubtabID == $0.subtabs.first?.id }
            ?? candidates.first
    }

    public var mainTracks: [SidebarSnapshot.Row] {
        let source = snapshot(tab: "library", subtab: nil) ?? snapshots.first { $0.rows.contains { $0.kind == .track } }
        return source?.rows.filter { $0.kind == .track } ?? []
    }
}

public struct SidebarNavigator {
    public let library: FixtureLibrary
    public private(set) var state: ViewState
    private let mainTracks: [SidebarSnapshot.Row]
    private var mainIndex: Int?

    public init?(library: FixtureLibrary) {
        guard let first = library.landing(tab: "library") ?? library.snapshots.first else { return nil }
        let tracks = library.mainTracks
        let anchor = library.landing(tab: "info")?.rows.first { $0.kind == .track }
            ?? tracks.first { $0.selected }
        let start = anchor.flatMap { target in tracks.firstIndex { $0.sameTrack(as: target) } }
        self.library = library
        mainTracks = tracks
        mainIndex = start ?? (tracks.isEmpty ? nil : 0)
        state = ViewState(snapshot: first)
        show(first)
    }

    public func state(showing snapshot: SidebarSnapshot) -> ViewState {
        var next = ViewState(snapshot: snapshot)
        guard let index = mainIndex else { return next }
        let main = mainTracks[index]
        next.setMain(.init(row: main, position: index + 1, total: mainTracks.count))
        if snapshot.selectedTabID == "library", let row = snapshot.rows.firstIndex(where: { $0.sameTrack(as: main) }) {
            next.select(rowIndex: row)
        }
        return next
    }

    private mutating func show(_ snapshot: SidebarSnapshot) {
        state = state(showing: snapshot)
    }

    @discardableResult
    public mutating func pressLeft(slot: Int) -> Bool {
        guard let index = ButtonSlots.tabIndex(forSlot: slot), state.snapshot.tabs.indices.contains(index),
              let next = library.landing(tab: state.snapshot.tabs[index].id)
        else { return false }
        show(next)
        return true
    }

    @discardableResult
    public mutating func pressRight(slot: Int) -> Bool {
        let current = state.snapshot
        guard let index = ButtonSlots.tabIndex(forSlot: slot), current.subtabs.indices.contains(index),
              let tab = current.selectedTabID,
              let next = library.snapshot(tab: tab, subtab: current.subtabs[index].id),
              !current.subtabs[index].selected
        else { return false }
        show(next)
        return true
    }

    public mutating func move(by delta: Int) {
        guard let index = mainIndex else { return }
        mainIndex = min(max(index + delta, 0), mainTracks.count - 1)
        show(state.snapshot)
    }
}
