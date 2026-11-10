import CoreGraphics
import Foundation
import S8Views

enum SidebarReader {
    static let aliases = ["playlist": "library", "history": "session"]
    static let headingBand: CGFloat = 30

    static func contentTitle(_ elements: SidebarElements) -> String {
        let title = elements.scroll.title
        if !title.isEmpty {
            return title
        }
        return elements.scroll.firstChild(role: "AXStaticText")?.text ?? ""
    }

    static func selectedTabID(_ elements: SidebarElements) -> String? {
        let title = contentTitle(elements).lowercased()
        if let direct = elements.tabs.first(where: { $0.id == title || $0.label.lowercased() == title }) {
            return direct.id
        }
        return aliases[title].flatMap { alias in elements.tabs.first { $0.id == alias }?.id }
    }

    static func subtabButtons(_ elements: SidebarElements) -> [AXNode] {
        let buttons = elements.scroll.children
            .filter { $0.role == "AXButton" && !$0.title.isEmpty }
            .map { (node: $0, frame: $0.frame) }
            .filter { $0.frame.width > 100 }
        let rows = Dictionary(grouping: buttons) { Int($0.frame.minY) }
        let strip = rows.values.first { $0.count >= 2 } ?? []
        return strip.sorted { $0.frame.minX < $1.frame.minX }.map(\.node)
    }

    static func subtabs(_ elements: SidebarElements) -> [SidebarSnapshot.Tab] {
        let buttons = subtabButtons(elements)
        guard buttons.count >= 2 else { return [] }
        let hasOutline = elements.scroll.firstChild(role: "AXOutline") != nil
        let selected = hasOutline ? 0 : buttons.count - 1
        return buttons.enumerated().map { index, button in
            SidebarSnapshot.Tab(id: button.title.lowercased(), label: button.title, selected: index == selected)
        }
    }

    static func snapshot(_ elements: SidebarElements, limit: Int, capturedAt: String? = nil) -> SidebarSnapshot {
        let current = selectedTabID(elements)
        let (heading, rows) = readRows(elements, limit: limit)
        let subs = subtabs(elements)
        let shown = subs.first(where: \.selected)?.label ?? heading
        return SidebarSnapshot(
            capturedAt: capturedAt,
            tabs: elements.tabs.map { SidebarSnapshot.Tab(id: $0.id, label: $0.label, selected: $0.id == current) },
            subtabs: subs,
            heading: shown.isEmpty ? nil : shown,
            rows: rows
        )
    }

    static func closedSnapshot(capturedAt: String? = nil) -> SidebarSnapshot {
        SidebarSnapshot(capturedAt: capturedAt, sidebarOpen: false, tabs: [], rows: [])
    }

    private static func readRows(_ elements: SidebarElements, limit: Int) -> (String, [SidebarSnapshot.Row]) {
        let scroll = elements.scroll
        let top = scroll.frame.minY
        let direct = scroll.children
        let statics = direct.filter { $0.role == "AXStaticText" && !$0.text.isEmpty }
            .map { TextFragment(text: $0.text, frame: $0.frame) }
        let headingFragment = statics.first { $0.frame.minY < top + headingBand }
        var heading = headingFragment?.text ?? ""
        if heading.isEmpty, let source = direct.first(where: { $0.role == "AXButton" && $0.summary == "Source" }) {
            heading = source.title
        }
        if heading.isEmpty {
            heading = contentTitle(elements)
        }

        let below = statics.filter { $0.frame != headingFragment?.frame && $0.frame.minY >= top + headingBand }
        let outline = direct.first { $0.role == "AXOutline" }
        let fields = outline.map(RowReader.infoFields) ?? []
        var rows: [SidebarSnapshot.Row] = []
        if below.contains(where: { RowReader.bpmValue($0.text) != nil }) {
            rows.append(withInfoFields(RowReader.summaryRow(fragments: below), fields: fields))
        } else {
            let ordered = below.sorted { $0.frame.minY < $1.frame.minY }
            rows += ordered.map { SidebarSnapshot.Row(kind: .header, text: $0.text) }
        }
        rows += fields.map { SidebarSnapshot.Row(kind: .header, text: "\($0.name): \($0.value)") }
        if let table = direct.first(where: { $0.role == "AXTable" }) {
            rows += RowReader.tableRows(table, limit: limit).map(\.row)
        }
        let scrollRight = scroll.frame.minX
        for sibling in elements.siblings where sibling.role == "AXStaticText" && sibling.frame.minX > scrollRight {
            if !sibling.text.isEmpty {
                rows.append(SidebarSnapshot.Row(kind: .header, text: sibling.text))
            }
        }
        return (heading, rows)
    }

    private static func withInfoFields(
        _ summary: SidebarSnapshot.Row, fields: [(name: String, value: String)]
    ) -> SidebarSnapshot.Row {
        let artist = fields.first { $0.name == "ARTIST" }?.value ?? summary.artist
        var title = summary.title
        if title?.isEmpty ?? true {
            title = fields.first { $0.name == "TITLE" }?.value ?? ""
        }
        return SidebarSnapshot.Row(
            kind: .track, title: title, artist: artist, bpm: summary.bpm, key: summary.key,
            time: summary.time, selected: true
        )
    }
}
