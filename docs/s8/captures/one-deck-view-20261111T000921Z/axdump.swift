import ApplicationServices
import Cocoa

func attr(_ e: AXUIElement, _ n: String) -> AnyObject? {
    var v: AnyObject?
    return AXUIElementCopyAttributeValue(e, n as CFString, &v) == .success ? v : nil
}

func text(_ v: AnyObject?) -> String {
    guard let v else { return "" }
    if let s = v as? String { return s }
    if let n = v as? NSNumber { return n.stringValue }
    return ""
}

let maxDepth = Int(CommandLine.arguments.dropFirst().first ?? "") ?? 12
guard AXIsProcessTrusted() else { print("NOT TRUSTED"); exit(2) }
guard let app = NSWorkspace.shared.runningApplications.first(where: {
    $0.bundleIdentifier?.contains("algoriddim") == true || $0.localizedName?.contains("djay") == true
}) else { print("djay not running"); exit(1) }
let root = AXUIElementCreateApplication(app.processIdentifier)
AXUIElementSetMessagingTimeout(root, 3)
var count = 0
let started = Date()

func walk(_ e: AXUIElement, _ depth: Int) {
    count += 1
    let role = text(attr(e, kAXRoleAttribute))
    if role == "AXMenuBar" || role == "AXTable" || role == "AXOutline" { 
        print("\(depth)\t\(role)\t[skipped subtree]")
        return
    }
    let desc = text(attr(e, kAXDescriptionAttribute))
    let title = text(attr(e, kAXTitleAttribute))
    let value = text(attr(e, kAXValueAttribute))
    let ident = text(attr(e, kAXIdentifierAttribute))
    if !desc.isEmpty || !title.isEmpty || !value.isEmpty || depth <= 3 {
        let clean = { (s: String) in s.replacingOccurrences(of: "\n", with: "⏎").replacingOccurrences(of: "\t", with: " ") }
        print("\(depth)\t\(role)\tdesc=\(clean(desc))\ttitle=\(clean(title))\tvalue=\(clean(value))\tid=\(clean(ident))")
    }
    guard depth < maxDepth, let children = attr(e, kAXChildrenAttribute) as? [AXUIElement] else { return }
    for c in children { walk(c, depth + 1) }
}

walk(root, 0)
FileHandle.standardError.write("elements=\(count) seconds=\(String(format: "%.3f", Date().timeIntervalSince(started)))\n".data(using: .utf8)!)
