import ApplicationServices
import Cocoa

public enum DeckAccessError: Error, Equatable {
    case notRunning
    case permissionDenied
    case notAnswering
}

public protocol DeckAccessibility {
    func labeledValues() throws -> [String: String]
}

public struct DeckReader {
    public let source: DeckAccessibility

    public init(source: DeckAccessibility) {
        self.source = source
    }

    public func readings() throws -> [Int: DeckReading] {
        var labeled: [Int: [String: String]] = [:]
        for (label, value) in try source.labeledValues() {
            guard let deck = DeckParser.parseDeckNumber(from: label) else { continue }
            labeled[deck, default: [:]][label] = value
        }
        return labeled.mapValues(DeckParser.reading(from:))
    }
}

public final class AXDeckSource: DeckAccessibility {
    public static let maximumDepth = 6
    static let messagingTimeout: Float = 3
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
        AXUIElementSetMessagingTimeout(element, Self.messagingTimeout)
        cachedApp = (pid, element)
        return element
    }

    public func labeledValues() throws -> [String: String] {
        let app = try application()
        var value: AnyObject?
        let status = AXUIElementCopyAttributeValue(app, kAXChildrenAttribute as CFString, &value)
        if status == .cannotComplete, Self.isTrusted() {
            throw DeckAccessError.notAnswering
        }
        if status == .apiDisabled || status == .cannotComplete {
            throw DeckAccessError.permissionDenied
        }
        var results: [String: String] = [:]
        Self.collect(app, depth: 0, into: &results)
        return results
    }

    static func attribute(_ element: AXUIElement, _ name: String) -> AnyObject? {
        var value: AnyObject?
        return AXUIElementCopyAttributeValue(element, name as CFString, &value) == .success ? value : nil
    }

    static func record(_ label: String, _ value: String, into results: inout [String: String]) {
        if !value.isEmpty || results[label] == nil {
            results[label] = value
        }
    }

    static func collect(_ element: AXUIElement, depth: Int, into results: inout [String: String]) {
        let label = attribute(element, kAXDescriptionAttribute) as? String ?? ""
        if DeckParser.deckKeywords.contains(where: label.contains) {
            let value = attribute(element, kAXValueAttribute) as? String
                ?? attribute(element, kAXTitleAttribute) as? String ?? ""
            record(label, value, into: &results)
        }
        if depth < maximumDepth, let children = attribute(element, kAXChildrenAttribute) as? [AXUIElement] {
            for child in children {
                collect(child, depth: depth + 1, into: &results)
            }
        }
    }
}
