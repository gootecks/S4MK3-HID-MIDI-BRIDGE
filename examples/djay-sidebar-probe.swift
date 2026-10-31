import ApplicationServices
import AppKit
import Foundation

func attr(_ e: AXUIElement, _ a: String) -> AnyObject? {
    var v: CFTypeRef?
    return AXUIElementCopyAttributeValue(e, a as CFString, &v) == .success ? (v as AnyObject?) : nil
}

func str(_ e: AXUIElement, _ a: String) -> String {
    if let s = attr(e, a) as? String { return s }
    if let n = attr(e, a) as? NSNumber { return n.stringValue }
    return ""
}

func frame(_ e: AXUIElement) -> CGRect {
    guard let v = attr(e, "AXFrame") else { return .zero }
    var r = CGRect.zero
    AXValueGetValue(v as! AXValue, .cgRect, &r)
    return r
}

func kids(_ e: AXUIElement) -> [AXUIElement] { (attr(e, "AXChildren") as? [AXUIElement]) ?? [] }
func role(_ e: AXUIElement) -> String { str(e, "AXRole") }
func isSelected(_ e: AXUIElement) -> Bool { (attr(e, "AXSelected") as? NSNumber)?.boolValue ?? false }
func press(_ e: AXUIElement) -> Bool { AXUIElementPerformAction(e, "AXPress" as CFString) == .success }

func descendants(_ e: AXUIElement, depth: Int = 3) -> [AXUIElement] {
    guard depth > 0 else { return [] }
    var out: [AXUIElement] = []
    for c in kids(e) {
        out.append(c)
        out += descendants(c, depth: depth - 1)
    }
    return out
}

func fail(_ message: String, code: Int32 = 2) -> Never {
    FileHandle.standardError.write(Data(("djay-sidebar-probe: " + message + "\n").utf8))
    exit(code)
}

struct Tab {
    let id: String
    let label: String
    let element: AXUIElement
}

struct Sidebar {
    let tabs: [Tab]
    let scroll: AXUIElement?
    let siblings: [AXUIElement]
}

func djayWindow() -> (AXUIElement, AXUIElement) {
    guard let app = NSWorkspace.shared.runningApplications.first(where: {
        $0.bundleIdentifier == "com.algoriddim.djay-iphone-pro" || $0.localizedName == "djay Pro"
    }) else { fail("djay Pro is not running") }
    guard AXIsProcessTrusted() else { fail("Accessibility permission missing for this terminal") }
    let ax = AXUIElementCreateApplication(app.processIdentifier)
    AXUIElementSetMessagingTimeout(ax, 3)
    let windows = (attr(ax, "AXWindows") as? [AXUIElement]) ?? []
    let win = (attr(ax, "AXFocusedWindow") as! AXUIElement?)
        ?? (attr(ax, "AXMainWindow") as! AXUIElement?)
        ?? windows.first
    guard let window = win else {
        fail("djay Pro exposes no window through AXFocusedWindow, AXMainWindow or AXWindows")
    }
    return (ax, window)
}

func findToggle(_ window: AXUIElement) -> AXUIElement? {
    let bar = descendants(window, depth: 2)
    return bar.first { role($0) == "AXButton" && str($0, "AXHelp") == "Show sidebar" }
}

func findSidebar(_ e: AXUIElement, depth: Int = 0) -> Sidebar? {
    let children = kids(e)
    let buttons = children.filter { c in
        let f = frame(c)
        return role(c) == "AXButton" && !str(c, "AXDescription").isEmpty
            && f.width >= 40 && f.width <= 100 && f.height >= 25 && f.height <= 60
    }
    let rows = Dictionary(grouping: buttons) { Int(frame($0).minY) }
    if let strip = rows.values.first(where: { $0.count >= 3 }),
       let scroll = children.first(where: { role($0) == "AXScrollArea" && frame($0).minY > frame(strip[0]).minY }) {
        let tabs = strip.sorted { frame($0).minX < frame($1).minX }.map { b -> Tab in
            let desc = str(b, "AXDescription")
            let help = str(b, "AXHelp")
            return Tab(id: desc.lowercased(), label: help.isEmpty ? desc : help, element: b)
        }
        return Sidebar(tabs: tabs, scroll: scroll, siblings: children)
    }
    guard depth < 4 else { return nil }
    for c in children where ["AXSplitGroup", "AXGroup"].contains(role(c)) {
        if let s = findSidebar(c, depth: depth + 1) { return s }
    }
    return nil
}

let aliasTitles = ["playlist": "library", "history": "session"]

func contentTitle(_ s: Sidebar) -> String {
    guard let scroll = s.scroll else { return "" }
    var t = str(scroll, "AXTitle")
    if t.isEmpty {
        t = kids(scroll).first { role($0) == "AXStaticText" }.map { str($0, "AXValue") } ?? ""
    }
    return t
}

func selectedTabId(_ s: Sidebar) -> String? {
    let title = contentTitle(s).lowercased()
    if let direct = s.tabs.first(where: { $0.id == title || $0.label.lowercased() == title }) { return direct.id }
    if let alias = aliasTitles[title] { return s.tabs.first { $0.id == alias }?.id }
    return nil
}

func subtabButtons(_ s: Sidebar) -> [AXUIElement] {
    guard let scroll = s.scroll else { return [] }
    let bs = kids(scroll).filter { role($0) == "AXButton" && !str($0, "AXTitle").isEmpty && frame($0).width > 100 }
    let rows = Dictionary(grouping: bs) { Int(frame($0).minY) }
    return (rows.values.first { $0.count >= 2 } ?? []).sorted { frame($0).minX < frame($1).minX }
}

func subtabs(_ s: Sidebar) -> [[String: Any]] {
    guard let scroll = s.scroll else { return [] }
    let bs = subtabButtons(s)
    guard bs.count >= 2 else { return [] }
    let hasOutline = kids(scroll).contains { role($0) == "AXOutline" }
    let selectedIndex = hasOutline ? 0 : bs.count - 1
    return bs.enumerated().map { i, b in
        let title = str(b, "AXTitle")
        return ["id": title.lowercased(), "label": title, "selected": i == selectedIndex]
    }
}

let timeRegex = try! NSRegularExpression(pattern: #"^\d+:\d\d(:\d\d)?$"#)

func matchesTime(_ s: String) -> Bool {
    timeRegex.firstMatch(in: s, range: NSRange(s.startIndex..., in: s)) != nil
}

func nonEmpty(_ s: String?) -> Any { (s?.isEmpty ?? true) ? NSNull() : s! }

func bpmValue(_ s: String) -> String? {
    guard s.hasSuffix(" BPM") else { return nil }
    return String(s.dropLast(4))
}

func trackRow(statics: [(String, CGRect)], bounds: CGRect, selected: Bool) -> [String: Any] {
    let splitX = bounds.minX + bounds.width * 0.6
    return trackRow(
        left: statics.filter { $0.1.minX < splitX },
        right: statics.filter { $0.1.minX >= splitX },
        selected: selected)
}

func summaryRow(statics: [(String, CGRect)]) -> [String: Any] {
    let topY = statics.map { $0.1.minY }.min() ?? 0
    return trackRow(
        left: statics.filter { $0.1.minY < topY + 6 },
        right: statics.filter { $0.1.minY >= topY + 6 },
        selected: true)
}

func trackRow(left rawLeft: [(String, CGRect)], right: [(String, CGRect)], selected: Bool) -> [String: Any] {
    let left = rawLeft.sorted { ($0.1.minY, $0.1.minX) < ($1.1.minY, $1.1.minX) }
    var bpm: String?
    var key: String?
    var time: String?
    for (text, _) in right.sorted(by: { $0.1.minX < $1.1.minX }) {
        if let b = bpmValue(text) { bpm = b } else if matchesTime(text) { time = text } else { key = text }
    }
    return [
        "kind": "track", "title": left.first?.0 ?? "", "artist": nonEmpty(left.count > 1 ? left[1].0 : nil),
        "bpm": nonEmpty(bpm), "key": nonEmpty(key), "time": nonEmpty(time), "selected": selected,
    ]
}

func tableRows(_ table: AXUIElement, limit: Int) -> [[String: Any]] {
    var out: [[String: Any]] = []
    for row in kids(table) where role(row) == "AXRow" {
        if out.count >= limit { break }
        let parts = descendants(row, depth: 3)
        let statics = parts.filter { role($0) == "AXStaticText" && !str($0, "AXValue").isEmpty }
            .map { (str($0, "AXValue"), frame($0)) }
        let buttons = parts.filter { role($0) == "AXButton" && !str($0, "AXTitle").isEmpty }
        if statics.isEmpty, let b = buttons.first {
            out.append(["kind": "action", "text": str(b, "AXTitle")])
        } else if statics.count == 1 {
            out.append(["kind": "header", "text": statics[0].0])
        } else if statics.count > 1 {
            out.append(trackRow(statics: statics, bounds: frame(row), selected: isSelected(row)))
        }
    }
    return out
}

func infoFields(_ outline: AXUIElement) -> [(String, String)] {
    var out: [(String, String)] = []
    for row in kids(outline) where role(row) == "AXRow" {
        let parts = descendants(row, depth: 3)
        guard let label = parts.first(where: { role($0) == "AXStaticText" }) else { continue }
        let name = str(label, "AXValue")
        let value = parts.dropFirst().first { !["AXStaticText"].contains(role($0)) || $0 != label }
            .flatMap { p -> String? in
                if p == label { return nil }
                let v = str(p, "AXValue")
                return v.isEmpty ? nil : v
            }
        let valueText = value ?? parts.filter { $0 != label }.map { str($0, "AXValue") }.first { !$0.isEmpty && $0 != "0" } ?? ""
        if !name.isEmpty && !valueText.isEmpty { out.append((name, valueText)) }
    }
    return out
}

func readRows(_ s: Sidebar, limit: Int) -> (String, [[String: Any]]) {
    guard let scroll = s.scroll else { return ("", []) }
    let top = frame(scroll).minY
    let direct = kids(scroll)
    let statics = direct.filter { role($0) == "AXStaticText" && !str($0, "AXValue").isEmpty }
    let heading = statics.first { frame($0).minY < top + 30 }
    var headingText = heading.map { str($0, "AXValue") } ?? ""
    if headingText.isEmpty, let source = direct.first(where: { role($0) == "AXButton" && str($0, "AXDescription") == "Source" }) {
        headingText = str(source, "AXTitle")
    }
    if headingText.isEmpty { headingText = contentTitle(s) }
    var rows: [[String: Any]] = []

    let below = statics.filter { $0 != heading && frame($0).minY >= top + 30 }
    let summary = below.map { (str($0, "AXValue"), frame($0)) }
    let outline = direct.first { role($0) == "AXOutline" }
    if summary.contains(where: { bpmValue($0.0) != nil }) {
        var track = summaryRow(statics: summary)
        let fields = outline.map(infoFields) ?? []
        if let artist = fields.first(where: { $0.0 == "ARTIST" })?.1 { track["artist"] = artist }
        if track["title"] as? String == nil || (track["title"] as? String)!.isEmpty { track["title"] = fields.first { $0.0 == "TITLE" }?.1 ?? "" }
        rows.append(track)
    } else {
        for (text, _) in summary.sorted(by: { $0.1.minY < $1.1.minY }) { rows.append(["kind": "header", "text": text]) }
    }
    if let outline = outline {
        for (name, value) in infoFields(outline) { rows.append(["kind": "header", "text": "\(name): \(value)"]) }
    }
    if let table = direct.first(where: { role($0) == "AXTable" }) {
        rows += tableRows(table, limit: limit)
    }
    let scrollRight = frame(scroll).minX
    for sib in s.siblings where role(sib) == "AXStaticText" && frame(sib).minX > scrollRight {
        let v = str(sib, "AXValue")
        if !v.isEmpty { rows.append(["kind": "header", "text": v]) }
    }
    return (headingText, rows)
}

func snapshot(limit: Int) -> [String: Any] {
    let (_, window) = djayWindow()
    var snap: [String: Any] = [
        "schema": 1,
        "capturedAt": ISO8601DateFormatter().string(from: Date()),
        "sidebarOpen": false, "tabs": [], "subtabs": [], "heading": NSNull(), "rows": [],
    ]
    guard let s = findSidebar(window), s.scroll != nil else { return snap }
    let current = selectedTabId(s)
    let (heading, rows) = readRows(s, limit: limit)
    snap["sidebarOpen"] = true
    snap["tabs"] = s.tabs.map { ["id": $0.id, "label": $0.label, "selected": $0.id == current] as [String: Any] }
    let subs = subtabs(s)
    snap["subtabs"] = subs
    let subLabel = subs.first { $0["selected"] as? Bool == true }?["label"] as? String
    let shown = subLabel ?? heading
    snap["heading"] = shown.isEmpty ? NSNull() : shown
    snap["rows"] = rows
    return snap
}

func render(_ snap: [String: Any], compact: Bool) -> String {
    let opts: JSONSerialization.WritingOptions = compact ? [.sortedKeys, .withoutEscapingSlashes] : [.prettyPrinted, .sortedKeys, .withoutEscapingSlashes]
    let data = try! JSONSerialization.data(withJSONObject: snap, options: opts)
    return String(data: data, encoding: .utf8)!
}

func waitUntil(_ timeout: Double, _ cond: () -> Bool) -> Bool {
    let start = Date()
    while Date().timeIntervalSince(start) < timeout {
        if cond() { return true }
        usleep(40_000)
    }
    return cond()
}

func pressTarget(_ target: String) {
    let (_, window) = djayWindow()
    guard var s = findSidebar(window) else { fail("sidebar is closed or not found; run with --open", code: 3) }
    let parts = target.lowercased().split(separator: "/").map(String.init)
    let wanted = parts[0] == "history" ? "session" : parts[0]
    let subtabTarget = parts.count > 1 ? parts[1] : nil
    if let tab = s.tabs.first(where: { $0.id == wanted }) {
        if selectedTabId(s) != tab.id || subtabTarget != nil {
            guard press(tab.element) else { fail("AXPress failed on tab \(tab.id)", code: 4) }
        }
        if let sub = subtabTarget {
            _ = waitUntil(2) { findSidebar(window).flatMap(selectedTabId) == tab.id }
            s = findSidebar(window) ?? s
            guard let b = subtabButtons(s).first(where: { str($0, "AXTitle").lowercased() == sub }) else {
                fail("no subtab \(sub) under \(tab.id)", code: 4)
            }
            guard press(b) else { fail("AXPress failed on subtab \(sub)", code: 4) }
        }
    } else if parts.count == 1, let tab = s.tabs.first(where: { $0.id == "info" }) {
        if selectedTabId(s) != "info" { _ = press(tab.element) }
        _ = waitUntil(2) { findSidebar(window).flatMap(selectedTabId) == "info" }
        s = findSidebar(window) ?? s
        guard let b = subtabButtons(s).first(where: { str($0, "AXTitle").lowercased() == wanted }) else {
            fail("unknown tab or subtab \(target)", code: 4)
        }
        guard press(b) else { fail("AXPress failed on subtab \(wanted)", code: 4) }
    } else {
        fail("unknown tab \(target); tabs: \(s.tabs.map { $0.id }.joined(separator: ", "))", code: 4)
    }
    usleep(250_000)
}

func setSidebar(open: Bool) {
    let (_, window) = djayWindow()
    let isOpen = findSidebar(window)?.scroll != nil
    guard isOpen != open else { return }
    guard let toggle = findToggle(window) ?? descendants(window, depth: 2).first(where: {
        role($0) == "AXButton" && str($0, "AXDescription").lowercased().hasSuffix("library inspector")
    }) else { fail("sidebar toggle button not found", code: 4) }
    guard press(toggle) else { fail("AXPress failed on sidebar toggle", code: 4) }
    _ = waitUntil(2) { (findSidebar(window)?.scroll != nil) == open }
}

func ms(_ since: Date) -> Double { Date().timeIntervalSince(since) * 1000 }

func stats(_ xs: [Double]) -> [String: Any] {
    guard !xs.isEmpty else { return [:] }
    let sorted = xs.sorted()
    let p95 = sorted[min(sorted.count - 1, Int(Double(sorted.count) * 0.95))]
    return ["n": xs.count, "minMs": Int(sorted[0]), "medianMs": Int(sorted[sorted.count / 2]), "p95Ms": Int(p95), "maxMs": Int(sorted.last!)]
}

func bench(cycles: Int, limit: Int) {
    let (_, window) = djayWindow()
    guard let first = findSidebar(window) else { fail("sidebar is closed or not found", code: 3) }
    let original = selectedTabId(first)
    var locate: [Double] = [], pressMs: [Double] = [], readMs: [Double] = [], subMs: [Double] = []
    var failures: [String] = []
    var presses = 0
    for cycle in 0..<cycles {
        for tab in first.tabs {
            let t0 = Date()
            guard let s = findSidebar(window) else { failures.append("c\(cycle) \(tab.id): sidebar vanished"); continue }
            locate.append(ms(t0))
            guard let el = s.tabs.first(where: { $0.id == tab.id })?.element else { continue }
            presses += 1
            let t1 = Date()
            if !press(el) { failures.append("c\(cycle) \(tab.id): AXPress error"); continue }
            let ok = waitUntil(3) { findSidebar(window).flatMap(selectedTabId) == tab.id }
            pressMs.append(ms(t1))
            if !ok { failures.append("c\(cycle) \(tab.id): content never switched") }
            let t2 = Date()
            _ = snapshot(limit: limit)
            readMs.append(ms(t2))
            if tab.id == "info" {
                for label in ["related", "info"] {
                    presses += 1
                    let t3 = Date()
                    guard let s2 = findSidebar(window),
                          let sub = subtabButtons(s2).first(where: { str($0, "AXTitle").lowercased() == label }) else {
                        failures.append("c\(cycle) info/\(label): button not found"); continue
                    }
                    if !press(sub) { failures.append("c\(cycle) info/\(label): AXPress error"); continue }
                    let sw = waitUntil(3) {
                        guard let s3 = findSidebar(window) else { return false }
                        return subtabs(s3).first { $0["selected"] as? Bool == true }?["id"] as? String == label
                    }
                    subMs.append(ms(t3))
                    if !sw { failures.append("c\(cycle) info/\(label): content never switched") }
                }
            }
        }
    }
    if let original = original, let s = findSidebar(window), let tab = s.tabs.first(where: { $0.id == original }) {
        _ = press(tab.element)
    }
    let report: [String: Any] = [
        "cycles": cycles, "presses": presses, "failures": failures, "locateSidebar": stats(locate),
        "tabPressToContent": stats(pressMs), "snapshotRead": stats(readMs), "subtabPressToContent": stats(subMs),
    ]
    print(render(report, compact: false))
}

let args = Array(CommandLine.arguments.dropFirst())
let compact = args.contains("--json")
let limitArg = args.firstIndex(of: "--limit").flatMap { Int(args[$0 + 1]) } ?? 100

if args.contains("--help") || args.contains("-h") {
    print("usage: swift djay-sidebar-probe.swift [--json] [--press <tab>[/<subtab>]] [--open|--close] [--watch] [--bench N] [--limit N]")
    exit(0)
}
if args.contains("--open") { setSidebar(open: true) }
if args.contains("--close") { setSidebar(open: false) }
if let i = args.firstIndex(of: "--bench") {
    bench(cycles: i + 1 < args.count ? Int(args[i + 1]) ?? 20 : 20, limit: limitArg)
    exit(0)
}
if let i = args.firstIndex(of: "--press") {
    guard i + 1 < args.count else { fail("--press needs a tab id") }
    pressTarget(args[i + 1])
}

if args.contains("--watch") {
    var last = ""
    while true {
        let t0 = Date()
        var snap = snapshot(limit: limitArg)
        snap["capturedAt"] = ""
        let body = render(snap, compact: true)
        if body != last {
            last = body
            snap["capturedAt"] = ISO8601DateFormatter().string(from: Date())
            print(render(snap, compact: true))
            fflush(stdout)
            FileHandle.standardError.write(Data(String(format: "read %.0f ms\n", Date().timeIntervalSince(t0) * 1000).utf8))
        }
        usleep(250_000)
    }
}
print(render(snapshot(limit: limitArg), compact: compact))
