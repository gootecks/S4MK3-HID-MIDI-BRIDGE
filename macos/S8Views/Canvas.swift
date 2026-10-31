import CoreGraphics
import CoreText
import Foundation

struct RGB {
    let red: UInt8
    let green: UInt8
    let blue: UInt8

    init(_ red: UInt8, _ green: UInt8, _ blue: UInt8) {
        self.red = red
        self.green = green
        self.blue = blue
    }

    var cgColor: CGColor {
        CGColor(
            colorSpace: CGColorSpace(name: CGColorSpace.sRGB)!,
            components: [CGFloat(red) / 255, CGFloat(green) / 255, CGFloat(blue) / 255, 1]
        )!
    }
}

enum TextAlign {
    case left, center, right
}

struct TextStyle {
    var size: CGFloat
    var bold = false
    var color: RGB
    var align = TextAlign.left
    var maxWidth: CGFloat?
}

final class Canvas {
    let context: CGContext

    init() {
        context = CGContext(
            data: nil, width: Metrics.width, height: Metrics.height, bitsPerComponent: 8,
            bytesPerRow: Metrics.width * 4, space: CGColorSpace(name: CGColorSpace.sRGB)!,
            bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue
        )!
        context.translateBy(x: 0, y: CGFloat(Metrics.height))
        context.scaleBy(x: 1, y: -1)
        context.textMatrix = CGAffineTransform(scaleX: 1, y: -1)
    }

    func fill(_ rect: CGRect, _ color: RGB, radius: CGFloat = 0) {
        context.setFillColor(color.cgColor)
        if radius > 0 {
            context.addPath(CGPath(roundedRect: rect, cornerWidth: radius, cornerHeight: radius, transform: nil))
            context.fillPath()
        } else {
            context.fill(rect)
        }
    }

    func stroke(_ rect: CGRect, _ color: RGB, radius: CGFloat, width: CGFloat = 1) {
        context.setStrokeColor(color.cgColor)
        context.setLineWidth(width)
        let inset = rect.insetBy(dx: width / 2, dy: width / 2)
        context.addPath(CGPath(roundedRect: inset, cornerWidth: radius, cornerHeight: radius, transform: nil))
        context.strokePath()
    }

    private func line(_ string: String, _ style: TextStyle) -> CTLine {
        let font = CTFontCreateUIFontForLanguage(style.bold ? .emphasizedSystem : .system, style.size, nil)!
        var attributes: [NSAttributedString.Key: Any] = [:]
        attributes[NSAttributedString.Key(kCTFontAttributeName as String)] = font
        attributes[NSAttributedString.Key(kCTForegroundColorAttributeName as String)] = style.color.cgColor
        return CTLineCreateWithAttributedString(NSAttributedString(string: string, attributes: attributes))
    }

    private func width(_ line: CTLine) -> CGFloat {
        CGFloat(CTLineGetTypographicBounds(line, nil, nil, nil))
    }

    func measure(_ string: String, _ style: TextStyle) -> CGFloat {
        width(line(string, style))
    }

    func text(_ string: String, at anchor: CGPoint, _ style: TextStyle) {
        var drawn = line(string, style)
        if let maxWidth = style.maxWidth, width(drawn) > maxWidth {
            let token = line("\u{2026}", style)
            drawn = CTLineCreateTruncatedLine(drawn, Double(maxWidth), .end, token) ?? drawn
        }
        let drawnWidth = width(drawn)
        let origin: CGFloat = switch style.align {
        case .left: anchor.x
        case .center: anchor.x - drawnWidth / 2
        case .right: anchor.x - drawnWidth
        }
        context.textPosition = CGPoint(x: origin, y: anchor.y)
        CTLineDraw(drawn, context)
    }

    func wrap(_ string: String, _ style: TextStyle, maxWidth: CGFloat, maxLines: Int) -> [String] {
        var lines: [String] = []
        var current = ""
        let words = string.split(separator: " ").map(String.init)
        for (index, word) in words.enumerated() {
            let candidate = current.isEmpty ? word : current + " " + word
            if measure(candidate, style) <= maxWidth || current.isEmpty {
                current = candidate
            } else if lines.count == maxLines - 1 {
                current = ([current] + words[index...]).joined(separator: " ")
                break
            } else {
                lines.append(current)
                current = word
            }
        }
        if !current.isEmpty {
            lines.append(current)
        }
        return lines
    }

    var bitmap: Bitmap {
        let count = Metrics.width * Metrics.height * 4
        let buffer = UnsafeBufferPointer(start: context.data!.assumingMemoryBound(to: UInt8.self), count: count)
        return Bitmap(width: Metrics.width, height: Metrics.height, rgba: Array(buffer))
    }
}
