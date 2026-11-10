import Foundation

public struct EncoderAccumulator {
    public static let countsPerRow = 50

    private var key: String?
    private var remainder = 0

    public init() {}

    public mutating func add(side: PanelSide, encoder: Int, delta: Int) -> Int {
        let current = "\(side.rawValue)\(encoder)"
        if current != key || (remainder != 0 && (remainder < 0) != (delta < 0)) {
            remainder = 0
        }
        key = current
        remainder += delta
        let rows = remainder / EncoderAccumulator.countsPerRow
        remainder -= rows * EncoderAccumulator.countsPerRow
        return rows
    }
}
