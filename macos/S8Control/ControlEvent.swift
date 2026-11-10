import Foundation

public enum PanelSide: String {
    case left, right
}

public enum ControlEvent: Equatable {
    case displayButton(side: PanelSide, slot: Int, down: Bool)
    case screenEncoder(side: PanelSide, encoder: Int, delta: Int)

    public static let prefix = "S8_CONTROL "

    public static func parse(_ line: String) -> ControlEvent? {
        guard line.hasPrefix(prefix) else { return nil }
        let fields = line.dropFirst(prefix.count).split(separator: " ", omittingEmptySubsequences: true)
        guard fields.count == 4, let side = PanelSide(rawValue: String(fields[1])),
              let number = Int(fields[2]) else { return nil }
        switch fields[0] {
        case "display_button":
            guard [2, 3, 5, 6, 7, 8].contains(number) else { return nil }
            switch fields[3] {
            case "down": return .displayButton(side: side, slot: number, down: true)
            case "up": return .displayButton(side: side, slot: number, down: false)
            default: return nil
            }
        case "screen_encoder":
            guard (1 ... 4).contains(number), let delta = Int(fields[3]) else { return nil }
            return .screenEncoder(side: side, encoder: number, delta: delta)
        default:
            return nil
        }
    }
}
