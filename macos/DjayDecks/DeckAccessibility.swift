import ApplicationServices
import Cocoa

public enum DeckAccessError: Error, Equatable {
    case notRunning
    case permissionDenied
}

public protocol DeckAccessibility {
    func labeledValues(prefix: String) throws -> [String: String]
}

public struct DeckReader {
    public let source: DeckAccessibility

    public init(source: DeckAccessibility) {
        self.source = source
    }

    public func read(deck: Int) throws -> DeckReading {
        var labeled: [String: String] = [:]
        for keyword in DeckParser.deckKeywords {
            try labeled.merge(source.labeledValues(prefix: "\(keyword) \(deck)")) { _, new in new }
        }
        return DeckParser.reading(from: labeled)
    }
}

public final class AXDeckSource: DeckAccessibility {
    public static let maximumDepth = 6
    private var cachedApp: (pid: pid_t, element: AXUIElement)?

    public init() {}

    public static func isTrusted() -> Bool {
        AXIsProcessTrusted()
    }

    private func application() throws -> AXUIElement {
        let running = NSWorkspace.shared.runningApplications.first {
            $0.bundleIdentifier?.contains("algoriddim") == true || $0.localizedName?.contains("djay") == true
        }
        guard let running else { throw DeckAccessError.notRunning }
        let pid = running.processIdentifier
        if let cachedApp, cachedApp.pid == pid {
            return cachedApp.element
        }
        let element = AXUIElementCreateApplication(pid)
        cachedApp = (pid, element)
        return element
    }

    public func labeledValues(prefix: String) throws -> [String: String] {
        let app = try application()
        var value: AnyObject?
        let status = AXUIElementCopyAttributeValue(app, kAXChildrenAttribute as CFString, &value)
        if status == .apiDisabled || status == .cannotComplete {
            throw DeckAccessError.permissionDenied
        }
        return Self.collect(app, prefix: prefix, depth: 0)
    }

    static func attribute(_ element: AXUIElement, _ name: String) -> AnyObject? {
        var value: AnyObject?
        return AXUIElementCopyAttributeValue(element, name as CFString, &value) == .success ? value : nil
    }

    static func collect(_ element: AXUIElement, prefix: String, depth: Int) -> [String: String] {
        var results: [String: String] = [:]
        let label = attribute(element, kAXDescriptionAttribute) as? String ?? ""
        let value = attribute(element, kAXValueAttribute) as? String
            ?? attribute(element, kAXTitleAttribute) as? String ?? ""
        if !label.isEmpty, label.contains(prefix), !value.isEmpty {
            results[label] = value
        }
        if depth < maximumDepth, let children = attribute(element, kAXChildrenAttribute) as? [AXUIElement] {
            for child in children {
                results.merge(collect(child, prefix: prefix, depth: depth + 1)) { _, new in new }
            }
        }
        return results
    }
}
