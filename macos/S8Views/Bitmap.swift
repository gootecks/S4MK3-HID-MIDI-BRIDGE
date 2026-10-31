import CoreGraphics
import Foundation
import ImageIO

public struct Pixel: Equatable {
    public let red: UInt8
    public let green: UInt8
    public let blue: UInt8

    public init(red: UInt8, green: UInt8, blue: UInt8) {
        self.red = red
        self.green = green
        self.blue = blue
    }
}

public struct Bitmap: Equatable {
    public let width: Int
    public let height: Int
    public let rgba: [UInt8]

    public init(width: Int, height: Int, rgba: [UInt8]) {
        precondition(rgba.count == width * height * 4, "rgba must hold width*height*4 bytes")
        self.width = width
        self.height = height
        self.rgba = rgba
    }

    public func pixel(column: Int, row: Int) -> Pixel {
        let index = (row * width + column) * 4
        return Pixel(red: rgba[index], green: rgba[index + 1], blue: rgba[index + 2])
    }

    public var cgImage: CGImage? {
        guard let provider = CGDataProvider(data: Data(rgba) as CFData) else { return nil }
        return CGImage(
            width: width, height: height, bitsPerComponent: 8, bitsPerPixel: 32, bytesPerRow: width * 4,
            space: CGColorSpace(name: CGColorSpace.sRGB)!,
            bitmapInfo: CGBitmapInfo(rawValue: CGImageAlphaInfo.premultipliedLast.rawValue),
            provider: provider, decode: nil, shouldInterpolate: false, intent: .defaultIntent
        )
    }

    @discardableResult
    public func writePNG(to url: URL) -> Bool {
        guard let image = cgImage,
              let destination = CGImageDestinationCreateWithURL(url as CFURL, "public.png" as CFString, 1, nil)
        else { return false }
        CGImageDestinationAddImage(destination, image, nil)
        return CGImageDestinationFinalize(destination)
    }
}

public enum S8Frame {
    public static let headerSize = 16
    public static let commandSize = 4
    public static let footerSize = 8
    public static let pixelBytes = Metrics.width * Metrics.height * 2
    public static let size = headerSize + commandSize + pixelBytes + footerSize

    public static func rgb565(_ pixel: Pixel) -> UInt16 {
        (UInt16(pixel.red & 0xF8) << 8) | (UInt16(pixel.green & 0xFC) << 3) | UInt16(pixel.blue >> 3)
    }

    public static func header(screen: PanelSide) -> [UInt8] {
        [0x84, 0x00, screen.rawValue, 0x60, 0, 0, 0, 0, 0, 0, 0, 0, 0x01, 0xE0, 0x01, 0x10]
    }

    public static func footer(screen: PanelSide) -> [UInt8] {
        [0x03, 0x00, 0x00, 0x00, 0x40, 0x00, screen.rawValue, 0x00]
    }

    public static func make(screen: PanelSide, bitmap: Bitmap) -> [UInt8]? {
        guard bitmap.width == Metrics.width, bitmap.height == Metrics.height else { return nil }
        var frame = [UInt8]()
        frame.reserveCapacity(size)
        frame += header(screen: screen)
        frame += [0x00, 0x00, 0xFF, 0x00]
        let rgba = bitmap.rgba
        for index in stride(from: 0, to: rgba.count, by: 4) {
            let value = rgb565(Pixel(red: rgba[index], green: rgba[index + 1], blue: rgba[index + 2]))
            frame.append(UInt8(value >> 8))
            frame.append(UInt8(value & 0xFF))
        }
        frame += footer(screen: screen)
        return frame
    }
}
