import CoreGraphics
import Foundation

public protocol AXNode {
    var role: String { get }
    var frame: CGRect { get }
    var children: [AXNode] { get }
    var isSelected: Bool { get }
    func string(_ attribute: String) -> String
    func press() -> Bool
    func selectRows(_ rows: [AXNode]) -> Bool
}

public protocol AXRoot {
    func window() -> AXNode?
}

extension AXNode {
    func descendants(depth: Int = 3) -> [AXNode] {
        guard depth > 0 else { return [] }
        return children.flatMap { [$0] + $0.descendants(depth: depth - 1) }
    }

    func firstChild(role wanted: String) -> AXNode? {
        children.first { $0.role == wanted }
    }

    var title: String {
        string("AXTitle")
    }

    var text: String {
        string("AXValue")
    }

    var summary: String {
        string("AXDescription")
    }
}
