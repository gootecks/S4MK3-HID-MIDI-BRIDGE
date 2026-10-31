import Foundation

public enum SidebarRowKind: String {
    case header, track, action, unknown
}

public struct SidebarSnapshot: Equatable {
    public struct Tab: Equatable {
        public let id: String
        public let label: String
        public let selected: Bool

        public init(id: String, label: String, selected: Bool) {
            self.id = id
            self.label = label
            self.selected = selected
        }
    }

    public struct Row: Equatable {
        public let kind: SidebarRowKind
        public let text: String?
        public let title: String?
        public let artist: String?
        public let bpm: String?
        public let key: String?
        public let time: String?
        public let selected: Bool

        public init(
            kind: SidebarRowKind, text: String? = nil, title: String? = nil, artist: String? = nil,
            bpm: String? = nil, key: String? = nil, time: String? = nil, selected: Bool = false
        ) {
            self.kind = kind
            self.text = text
            self.title = title
            self.artist = artist
            self.bpm = bpm
            self.key = key
            self.time = time
            self.selected = selected
        }

        public var isSelectable: Bool {
            kind == .track || kind == .action
        }
    }

    public static let supportedSchema = 1

    public let capturedAt: String?
    public let sidebarOpen: Bool
    public let tabs: [Tab]
    public let subtabs: [Tab]
    public let heading: String?
    public let rows: [Row]

    public init(
        capturedAt: String? = nil, sidebarOpen: Bool = true, tabs: [Tab], subtabs: [Tab] = [],
        heading: String? = nil, rows: [Row]
    ) {
        self.capturedAt = capturedAt
        self.sidebarOpen = sidebarOpen
        self.tabs = tabs
        self.subtabs = subtabs
        self.heading = heading
        self.rows = rows
    }

    public var selectedTabID: String? {
        tabs.first(where: \.selected)?.id
    }

    public var selectedSubtabID: String? {
        subtabs.first(where: \.selected)?.id
    }

    public var selectedRowIndex: Int? {
        rows.firstIndex(where: { $0.selected && $0.isSelectable })
    }
}

public enum SnapshotError: Error, Equatable {
    case unsupportedSchema(Int)
}

extension SidebarSnapshot {
    public static func decode(_ data: Data) throws -> SidebarSnapshot {
        let wire = try JSONDecoder().decode(Wire.self, from: data)
        guard wire.schema == supportedSchema else { throw SnapshotError.unsupportedSchema(wire.schema) }
        return SidebarSnapshot(
            capturedAt: wire.capturedAt,
            sidebarOpen: wire.sidebarOpen ?? true,
            tabs: (wire.tabs ?? []).map(\.model),
            subtabs: (wire.subtabs ?? []).map(\.model),
            heading: wire.heading,
            rows: (wire.rows ?? []).map(\.model)
        )
    }

    public static func load(_ url: URL) throws -> SidebarSnapshot {
        try decode(Data(contentsOf: url))
    }

    private struct Wire: Decodable {
        let schema: Int
        let capturedAt: String?
        let sidebarOpen: Bool?
        let tabs: [WireTab]?
        let subtabs: [WireTab]?
        let heading: String?
        let rows: [WireRow]?
    }

    private struct WireTab: Decodable {
        let id: String
        let label: String
        let selected: Bool?
        var model: Tab {
            Tab(id: id, label: label, selected: selected ?? false)
        }
    }

    private struct WireRow: Decodable {
        let kind: String
        let text: String?
        let title: String?
        let artist: String?
        let bpm: Flexible?
        let key: String?
        let time: String?
        let selected: Bool?

        var model: Row {
            Row(
                kind: SidebarRowKind(rawValue: kind) ?? .unknown, text: text, title: title, artist: artist,
                bpm: bpm?.text, key: key, time: time, selected: selected ?? false
            )
        }
    }

    private struct Flexible: Decodable {
        let text: String

        init(from decoder: Decoder) throws {
            let container = try decoder.singleValueContainer()
            if let value = try? container.decode(String.self) {
                text = value
            } else if let value = try? container.decode(Int.self) {
                text = String(value)
            } else {
                text = try String(container.decode(Double.self))
            }
        }
    }
}
