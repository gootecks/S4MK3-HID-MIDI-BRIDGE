import AppKit
import ApplicationServices
import CoreGraphics

func attribute<T>(_ element: AXUIElement, _ name: String) -> T? {
    var value: CFTypeRef?
    guard AXUIElementCopyAttributeValue(element, name as CFString, &value) == .success else { return nil }
    return value as? T
}

func deckLayout(menuBar: AXUIElement) -> String? {
    let menus: [AXUIElement] = attribute(menuBar, kAXChildrenAttribute) ?? []
    guard let view = menus.first(where: { attribute($0, kAXTitleAttribute) == "View" }),
          let children: [AXUIElement] = attribute(view, kAXChildrenAttribute),
          let menu = children.first else { return nil }
    let items: [AXUIElement] = attribute(menu, kAXChildrenAttribute) ?? []
    for item in items {
        guard let title: String = attribute(item, kAXTitleAttribute),
              ["One Deck", "Two Decks", "Four Decks"].contains(title),
              let mark: String = attribute(item, kAXMenuItemMarkCharAttribute),
              !mark.isEmpty else { continue }
        return title
    }
    return nil
}

func bounds(of window: AXUIElement) -> CGRect? {
    guard let position: AXValue = attribute(window, kAXPositionAttribute),
          let size: AXValue = attribute(window, kAXSizeAttribute) else { return nil }
    var origin = CGPoint.zero
    var dimensions = CGSize.zero
    guard AXValueGetValue(position, .cgPoint, &origin),
          AXValueGetValue(size, .cgSize, &dimensions) else { return nil }
    return CGRect(origin: origin, size: dimensions)
}

func visibleSpace(pid: pid_t, window: AXUIElement?) -> String {
    guard let windows = CGWindowListCopyWindowInfo(.excludeDesktopElements, kCGNullWindowID)
        as? [[String: Any]] else { return "unknown" }
    let candidates = windows.filter {
        $0[kCGWindowOwnerPID as String] as? pid_t == pid && $0[kCGWindowLayer as String] as? Int == 0
    }
    if let main = candidates.first(where: { $0[kCGWindowName as String] as? String == "djay Pro" }) {
        return main[kCGWindowIsOnscreen as String] as? Bool == true ? "yes" : "no"
    }
    // Window names can be redacted without Screen Recording permission.
    guard let window, attribute(window, kAXTitleAttribute) == "djay Pro",
          let frame = bounds(of: window) else { return "unknown" }
    let match = candidates.first { candidate in
        guard let info = candidate[kCGWindowBounds as String] as? NSDictionary,
              let rect = CGRect(dictionaryRepresentation: info) else { return false }
        // Quartz rounds window bounds to points; allow for fractional AX values.
        return abs(frame.minX - rect.minX) <= 1 && abs(frame.minY - rect.minY) <= 1
            && abs(frame.width - rect.width) <= 1 && abs(frame.height - rect.height) <= 1
    }
    guard let match else { return "unknown" }
    return match[kCGWindowIsOnscreen as String] as? Bool == true ? "yes" : "no"
}

// Query after an answer without activating djay or moving its window.
let djayBundleIDs: Set = ["com.algoriddim.djay-iphone-free", "com.algoriddim.djay-iphone-pro"]
guard let djay = NSWorkspace.shared.runningApplications.first(where: {
    $0.bundleIdentifier.map(djayBundleIDs.contains) == true
}) else {
    exit(1)
}

let axApp = AXUIElementCreateApplication(djay.processIdentifier)
AXUIElementSetMessagingTimeout(axApp, 0.5)
let window: AXUIElement? = attribute(axApp, kAXFocusedWindowAttribute)
let menuBar: AXUIElement? = attribute(axApp, kAXMenuBarAttribute)
let layout = menuBar.flatMap { deckLayout(menuBar: $0) } ?? "unknown"
let visible = visibleSpace(pid: djay.processIdentifier, window: window)
print("layout=\"\(layout)\" visible_space=\(visible)")
exit(menuBar == nil ? 1 : 0)
