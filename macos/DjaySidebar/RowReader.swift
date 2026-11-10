import CoreGraphics
import Foundation
import S8Views

struct TableRow {
    let node: AXNode
    let row: SidebarSnapshot.Row
}

struct TextFragment {
    let text: String
    let frame: CGRect
}

enum RowReader {
    private static let timePattern = try? NSRegularExpression(pattern: #"^\d+:\d\d(:\d\d)?$"#)

    static func tableRows(_ table: AXNode, limit: Int) -> [TableRow] {
        var rows: [TableRow] = []
        for node in table.children where node.role == "AXRow" {
            if rows.count >= limit {
                break
            }
            if let row = read(rowNode: node) {
                rows.append(TableRow(node: node, row: row))
            }
        }
        return rows
    }

    private static func read(rowNode: AXNode) -> SidebarSnapshot.Row? {
        let parts = rowNode.descendants(depth: 3)
        let fragments = parts.filter { $0.role == "AXStaticText" && !$0.text.isEmpty }
            .map { TextFragment(text: $0.text, frame: $0.frame) }
        let button = parts.first { $0.role == "AXButton" && !$0.title.isEmpty }
        if fragments.isEmpty, let button {
            return SidebarSnapshot.Row(kind: .action, text: button.title)
        }
        if fragments.count == 1 {
            return SidebarSnapshot.Row(kind: .header, text: fragments[0].text)
        }
        guard fragments.count > 1 else { return nil }
        return trackRow(fragments: fragments, bounds: rowNode.frame, selected: rowNode.isSelected)
    }

    static func trackRow(fragments: [TextFragment], bounds: CGRect, selected: Bool) -> SidebarSnapshot.Row {
        let splitX = bounds.minX + bounds.width * 0.6
        return trackRow(
            left: fragments.filter { $0.frame.minX < splitX },
            right: fragments.filter { $0.frame.minX >= splitX },
            selected: selected
        )
    }

    static func summaryRow(fragments: [TextFragment]) -> SidebarSnapshot.Row {
        let top = fragments.map(\.frame.minY).min() ?? 0
        return trackRow(
            left: fragments.filter { $0.frame.minY < top + 6 },
            right: fragments.filter { $0.frame.minY >= top + 6 },
            selected: true
        )
    }

    static func trackRow(left: [TextFragment], right: [TextFragment], selected: Bool) -> SidebarSnapshot.Row {
        let ordered = left.sorted { ($0.frame.minY, $0.frame.minX) < ($1.frame.minY, $1.frame.minX) }
        var bpm: String?
        var key: String?
        var time: String?
        for fragment in right.sorted(by: { $0.frame.minX < $1.frame.minX }) {
            if let value = bpmValue(fragment.text) {
                bpm = value
            } else if matchesTime(fragment.text) {
                time = fragment.text
            } else {
                key = fragment.text
            }
        }
        return SidebarSnapshot.Row(
            kind: .track, title: ordered.first?.text ?? "", artist: ordered.count > 1 ? ordered[1].text : nil,
            bpm: bpm, key: key, time: time, selected: selected
        )
    }

    static func bpmValue(_ text: String) -> String? {
        text.hasSuffix(" BPM") ? String(text.dropLast(4)) : nil
    }

    private static func matchesTime(_ text: String) -> Bool {
        timePattern?.firstMatch(in: text, range: NSRange(text.startIndex..., in: text)) != nil
    }

    static func infoFields(_ outline: AXNode) -> [(name: String, value: String)] {
        var fields: [(name: String, value: String)] = []
        for row in outline.children where row.role == "AXRow" {
            let parts = row.descendants(depth: 3)
            guard let labelIndex = parts.firstIndex(where: { $0.role == "AXStaticText" }) else { continue }
            let name = parts[labelIndex].text
            let others = parts.indices.filter { $0 != labelIndex }.map { parts[$0].text }
            let direct = others.first.flatMap { $0.isEmpty ? nil : $0 }
            let value = direct ?? others.first { !$0.isEmpty && $0 != "0" } ?? ""
            if !name.isEmpty, !value.isEmpty {
                fields.append((name: name, value: value))
            }
        }
        return fields
    }
}
