import CoreGraphics
import Foundation

struct SidebarTab {
    let id: String
    let label: String
    let node: AXNode
}

struct SidebarElements {
    let tabs: [SidebarTab]
    let scroll: AXNode
    let siblings: [AXNode]

    static let maxDepth = 4
    static let containerRoles = ["AXSplitGroup", "AXGroup"]

    static func find(in node: AXNode, depth: Int = 0) -> SidebarElements? {
        let framed = node.children.map { (node: $0, frame: $0.frame) }
        let buttons = framed.filter { child in
            child.node.role == "AXButton" && !child.node.summary.isEmpty && isTabSized(child.frame)
        }
        let strips = Dictionary(grouping: buttons) { Int($0.frame.minY) }
        let strip = strips.values.first { $0.count >= 3 } ?? []
        let stripTop = strip.first?.frame.minY ?? 0
        let scroll = framed.first { $0.node.role == "AXScrollArea" && $0.frame.minY > stripTop }
        if let scroll, !strip.isEmpty {
            let tabs = strip.sorted { $0.frame.minX < $1.frame.minX }.map { button -> SidebarTab in
                let description = button.node.summary
                let help = button.node.string("AXHelp")
                return SidebarTab(
                    id: description.lowercased(), label: help.isEmpty ? description : help, node: button.node
                )
            }
            return SidebarElements(tabs: tabs, scroll: scroll.node, siblings: framed.map(\.node))
        }
        guard depth < maxDepth else { return nil }
        for child in framed where containerRoles.contains(child.node.role) {
            if let found = find(in: child.node, depth: depth + 1) {
                return found
            }
        }
        return nil
    }

    private static func isTabSized(_ frame: CGRect) -> Bool {
        frame.width >= 40 && frame.width <= 100 && frame.height >= 25 && frame.height <= 60
    }
}
