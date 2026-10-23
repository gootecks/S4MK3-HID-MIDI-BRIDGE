import AppKit
import XCTest
@testable import S4Bridge

@MainActor
final class ScreenStatusIconTests: XCTestCase {
    func testBlockedRunningScreensUseRedInsteadOfGreen() throws {
        // Given a bridge that is still running but its requested screens are blocked.
        let image = try XCTUnwrap(bridgeStatusImage(for: .running, screenAttention: true))
        let bitmap = NSBitmapImageRep(cgImage: try XCTUnwrap(
            image.cgImage(forProposedRect: nil, context: nil, hints: nil)
        ))
        // When its real menu-bar symbol is rendered.
        var redPixels = 0
        for y in 0..<bitmap.pixelsHigh {
            for x in 0..<bitmap.pixelsWide {
                if let color = bitmap.colorAt(x: x, y: y)?.usingColorSpace(.deviceRGB),
                   color.alphaComponent > 0.5,
                   color.redComponent > color.greenComponent + 0.1 {
                    redPixels += 1
                }
            }
        }
        // Then a visible red attention symbol is present without claiming the bridge stopped.
        XCTAssertFalse(image.isTemplate)
        XCTAssertGreaterThan(redPixels, 0)
    }
}
