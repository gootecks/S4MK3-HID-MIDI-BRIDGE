import Foundation
import S8Views

public protocol SidebarControlling: AnyObject {
    @discardableResult func pressTab(index: Int) -> Bool
    @discardableResult func pressSubtab(index: Int) -> Bool
    @discardableResult func moveSelection(by delta: Int) -> Bool
}

public final class ControlRouter {
    private let sidebar: SidebarControlling
    private var accumulator = EncoderAccumulator()

    public init(sidebar: SidebarControlling) {
        self.sidebar = sidebar
    }

    @discardableResult
    public func handle(line: String) -> Bool {
        guard let event = ControlEvent.parse(line) else { return false }
        handle(event)
        return true
    }

    public func handle(_ event: ControlEvent) {
        switch event {
        case let .displayButton(side, slot, down):
            guard down, let index = ButtonSlots.tabIndex(forSlot: slot) else { return }
            switch side {
            case .left: sidebar.pressTab(index: index)
            case .right: sidebar.pressSubtab(index: index)
            }
        case let .screenEncoder(side, encoder, delta):
            let rows = accumulator.add(side: side, encoder: encoder, delta: delta)
            if rows != 0 {
                sidebar.moveSelection(by: rows)
            }
        }
    }
}
