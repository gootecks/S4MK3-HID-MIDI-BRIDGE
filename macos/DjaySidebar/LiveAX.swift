import AppKit
import ApplicationServices
import Foundation

public struct LiveNode: AXNode {
    let element: AXUIElement

    private func value(_ name: String) -> AnyObject? {
        var result: CFTypeRef?
        let status = AXUIElementCopyAttributeValue(element, name as CFString, &result)
        return status == .success ? (result as AnyObject?) : nil
    }

    public var role: String {
        string("AXRole")
    }

    public var frame: CGRect {
        guard let raw = value("AXFrame") else { return .zero }
        var rect = CGRect.zero
        // swiftlint:disable:next force_cast
        AXValueGetValue(raw as! AXValue, .cgRect, &rect)
        return rect
    }

    public var children: [AXNode] {
        ((value("AXChildren") as? [AXUIElement]) ?? []).map(LiveNode.init)
    }

    public var isSelected: Bool {
        (value("AXSelected") as? NSNumber)?.boolValue ?? false
    }

    public func string(_ attribute: String) -> String {
        if let text = value(attribute) as? String {
            return text
        }
        if let number = value(attribute) as? NSNumber {
            return number.stringValue
        }
        return ""
    }

    public func press() -> Bool {
        AXUIElementPerformAction(element, "AXPress" as CFString) == .success
    }

    public func selectRows(_ rows: [AXNode]) -> Bool {
        let elements = rows.compactMap { ($0 as? LiveNode)?.element }
        guard elements.count == rows.count else { return false }
        let status = AXUIElementSetAttributeValue(element, "AXSelectedRows" as CFString, elements as CFArray)
        return status == .success
    }
}

public struct LiveRoot: AXRoot {
    public static let bundleID = "com.algoriddim.djay-iphone-pro"

    public init() {}

    public static func isTrusted() -> Bool {
        AXIsProcessTrusted()
    }

    public static func djayIsRunning() -> Bool {
        application() != nil
    }

    private static func application() -> NSRunningApplication? {
        NSWorkspace.shared.runningApplications.first {
            $0.bundleIdentifier == bundleID || $0.localizedName == "djay Pro"
        }
    }

    private static func element(_ app: AXUIElement, _ name: String) -> AXUIElement? {
        var result: CFTypeRef?
        guard AXUIElementCopyAttributeValue(app, name as CFString, &result) == .success, let value = result else {
            return nil
        }
        // swiftlint:disable:next force_cast
        return (value as! AXUIElement)
    }

    public func window() -> AXNode? {
        guard let running = LiveRoot.application() else { return nil }
        let app = AXUIElementCreateApplication(running.processIdentifier)
        AXUIElementSetMessagingTimeout(app, 3)
        if let window = LiveRoot.element(app, "AXFocusedWindow") ?? LiveRoot.element(app, "AXMainWindow") {
            return LiveNode(element: window)
        }
        var list: CFTypeRef?
        guard AXUIElementCopyAttributeValue(app, "AXWindows" as CFString, &list) == .success,
              let first = (list as? [AXUIElement])?.first else { return nil }
        return LiveNode(element: first)
    }
}
