import CoreGraphics
import DjaySidebar
import Foundation

final class FakeNode: AXNode {
    let role: String
    var frame: CGRect
    var children: [AXNode]
    var isSelected = false
    var strings: [String: String]
    var onPress: (() -> Bool)?
    var selectedRows: [FakeNode] = []

    init(
        _ role: String, frame: CGRect = .zero, strings: [String: String] = [:], children: [AXNode] = []
    ) {
        self.role = role
        self.frame = frame
        self.strings = strings
        self.children = children
    }

    func string(_ attribute: String) -> String {
        strings[attribute] ?? ""
    }

    func press() -> Bool {
        onPress?() ?? true
    }

    func selectRows(_ rows: [AXNode]) -> Bool {
        selectedRows = rows.compactMap { $0 as? FakeNode }
        for case let row as FakeNode in children {
            row.isSelected = false
        }
        for row in selectedRows {
            row.isSelected = true
        }
        return true
    }
}

final class FakeRoot: AXRoot {
    var node: AXNode?

    init(_ node: AXNode?) {
        self.node = node
    }

    func window() -> AXNode? {
        node
    }
}

struct BuiltWindow {
    let window: FakeNode
    let tabs: [FakeNode]
    let scroll: FakeNode
}

struct LibraryTree {
    let window: FakeNode
    let tabs: [FakeNode]
    let table: FakeNode
}

struct InfoTree {
    let window: FakeNode
    let relatedButton: FakeNode
}

enum Tree {
    static func text(_ value: String, left: Int, top: Int, width: Int = 100) -> FakeNode {
        FakeNode(
            "AXStaticText", frame: CGRect(x: left, y: top, width: width, height: 14),
            strings: ["AXValue": value]
        )
    }

    static func tabButtons() -> [FakeNode] {
        let names = ["library", "queue", "match", "session", "info"]
        return names.enumerated().map { index, name in
            FakeNode(
                "AXButton", frame: CGRect(x: 10 + index * 70, y: 100, width: 60, height: 40),
                strings: ["AXDescription": name, "AXHelp": name.capitalized]
            )
        }
    }

    static func trackRow(_ title: String, artist: String, bpm: String, key: String, top: Int) -> FakeNode {
        FakeNode("AXRow", frame: CGRect(x: 10, y: top, width: 300, height: 30), children: [
            FakeNode("AXCell", children: [
                text(title, left: 14, top: top, width: 150),
                text(artist, left: 14, top: top + 14, width: 150)
            ]),
            FakeNode("AXCell", children: [
                text("\(bpm) BPM", left: 230, top: top, width: 40),
                text(key, left: 280, top: top, width: 20)
            ])
        ])
    }

    static func headerRow(_ value: String, top: Int) -> FakeNode {
        FakeNode("AXRow", frame: CGRect(x: 10, y: top, width: 300, height: 20), children: [
            text(value, left: 14, top: top)
        ])
    }

    static func subtabButton(_ title: String, left: Int) -> FakeNode {
        FakeNode("AXButton", frame: CGRect(x: left, y: 150, width: 140, height: 24), strings: ["AXTitle": title])
    }

    static func window(title: String, scrollChildren: [AXNode]) -> BuiltWindow {
        let buttons = tabButtons()
        let scroll = FakeNode(
            "AXScrollArea", frame: CGRect(x: 10, y: 150, width: 320, height: 600), strings: ["AXTitle": title],
            children: scrollChildren
        )
        let split = FakeNode("AXSplitGroup", children: buttons + [scroll])
        let window = FakeNode("AXWindow", children: [FakeNode("AXGroup", children: [split])])
        return BuiltWindow(window: window, tabs: buttons, scroll: scroll)
    }

    static func library(selected: Int? = nil) -> LibraryTree {
        let rows = [
            trackRow("One", artist: "A", bpm: "120", key: "8A", top: 200),
            trackRow("Two", artist: "B", bpm: "124", key: "9A", top: 230),
            trackRow("Three", artist: "C", bpm: "128", key: "10A", top: 260),
            trackRow("Four", artist: "D", bpm: "130", key: "11A", top: 290)
        ]
        if let selected {
            rows[selected].isSelected = true
        }
        let table = FakeNode("AXTable", children: rows)
        let built = window(title: "Playlist", scrollChildren: [text("Library", left: 14, top: 155), table])
        return LibraryTree(window: built.window, tabs: built.tabs, table: table)
    }

    static func infoRelated(relatedSelected: Bool = false) -> InfoTree {
        let related = subtabButton("Related", left: 160)
        var content: [AXNode] = [subtabButton("Info", left: 12), related]
        if !relatedSelected {
            content.append(FakeNode("AXOutline", children: [headerRow("ARTIST", top: 300)]))
        }
        return InfoTree(window: window(title: "Info", scrollChildren: content).window, relatedButton: related)
    }
}
