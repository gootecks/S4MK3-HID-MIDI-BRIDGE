import Foundation

public enum S8Record {
    public static let magic = Array("S8FR".utf8)
    public static let headerSize = 12
    public static let size = headerSize + S8Frame.pixelBytes

    public static func header(panel: PanelSide) -> [UInt8] {
        let length = UInt32(S8Frame.pixelBytes)
        return magic + [panel.rawValue, 0, 0, 0] + (0 ..< 4).map { UInt8((length >> ($0 * 8)) & 0xFF) }
    }

    public static func make(panel: PanelSide, bitmap: Bitmap) -> Data? {
        guard bitmap.width == Metrics.width, bitmap.height == Metrics.height else { return nil }
        var bytes = header(panel: panel)
        bytes.reserveCapacity(size)
        S8Frame.appendPixels(of: bitmap, to: &bytes)
        return Data(bytes)
    }
}
