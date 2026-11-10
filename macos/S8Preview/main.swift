import AppKit
import Foundation
import S8Views

struct Options {
    var path: String
    var zoom: CGFloat = 1
    var framesDir: String?
    var pngDir: String?
    var windowless = false
    var deck: Int?
    var deckSnapshot: String?
}

func parseOptions() -> Options? {
    var args = Array(CommandLine.arguments.dropFirst())
    var path: String?
    var options = Options(path: "")
    while !args.isEmpty {
        let arg = args.removeFirst()
        switch arg {
        case "--zoom": options.zoom = args.isEmpty ? 1 : (args.removeFirst() == "2" ? 2 : 1)
        case "--frames": options.framesDir = args.isEmpty ? nil : args.removeFirst()
        case "--png": options.pngDir = args.isEmpty ? nil : args.removeFirst()
        case "--no-window": options.windowless = true
        case "--deck": options.deck = args.isEmpty ? nil : Int(args.removeFirst())
        case "--deck-snapshot": options.deckSnapshot = args.isEmpty ? nil : args.removeFirst()
        default: path = arg
        }
    }
    guard let path else { return nil }
    options.path = path
    return options
}

let usage = """
usage: S8Preview <snapshot.json | fixtures-dir> [--zoom 2] [--frames DIR] [--png DIR] [--no-window]
                 [--deck N | --deck-snapshot deck.json]
keys: 1-8 left display buttons, q w e r t y u i right display buttons,
      up/down screen encoder, z zoom, f dump current state to --frames/--png dirs
--deck N shows live djay deck N (Accessibility) in the right panel; --deck-snapshot shows a recorded DeckSnapshot.
"""

func dump(_ state: ViewState, prefix: String, options: Options) {
    for side in [PanelSide.left, .right] {
        let name = side == .left ? "left" : "right"
        let bitmap = PanelRenderer.render(side, state)
        if let dir = options.framesDir, let frame = S8Frame.make(screen: side, bitmap: bitmap) {
            try? FileManager.default.createDirectory(atPath: dir, withIntermediateDirectories: true)
            try? Data(frame).write(to: URL(fileURLWithPath: "\(dir)/\(prefix)-\(name).s8frame"))
        }
        if let dir = options.pngDir {
            try? FileManager.default.createDirectory(atPath: dir, withIntermediateDirectories: true)
            bitmap.writePNG(to: URL(fileURLWithPath: "\(dir)/\(prefix)-\(name).png"))
        }
    }
}

final class PreviewView: NSView {
    var navigator: SidebarNavigator
    var zoom: CGFloat
    let options: Options
    var deckFeed: DeckFeed?
    static let margin: CGFloat = 16

    init(navigator: SidebarNavigator, options: Options) {
        self.navigator = navigator
        zoom = options.zoom
        self.options = options
        super.init(frame: .zero)
    }

    @available(*, unavailable)
    required init?(coder _: NSCoder) {
        fatalError()
    }

    override var isFlipped: Bool {
        true
    }

    override var acceptsFirstResponder: Bool {
        true
    }

    static func contentSize(zoom: CGFloat) -> NSSize {
        NSSize(width: (margin * 3 + 960) * zoom, height: (margin * 2 + 272) * zoom)
    }

    override func draw(_: NSRect) {
        NSColor(calibratedWhite: 0.13, alpha: 1).setFill()
        bounds.fill()
        guard let context = NSGraphicsContext.current?.cgContext else { return }
        context.interpolationQuality = .none
        for (index, side) in [PanelSide.left, .right].enumerated() {
            let rendered: Bitmap = if index == 1, let deckFeed {
                DeckPanelRenderer.render(deckFeed.current())
            } else {
                PanelRenderer.render(side, navigator.state)
            }
            guard let image = rendered.cgImage else { continue }
            let rect = CGRect(
                x: (Self.margin + CGFloat(index) * (480 + Self.margin)) * zoom, y: Self.margin * zoom,
                width: 480 * zoom, height: 272 * zoom
            )
            context.saveGState()
            context.translateBy(x: 0, y: rect.minY + rect.maxY)
            context.scaleBy(x: 1, y: -1)
            context.draw(image, in: rect)
            context.restoreGState()
        }
    }

    override func keyDown(with event: NSEvent) {
        let leftKeys = Array("12345678")
        let rightKeys = Array("qwertyui")
        guard let chars = event.charactersIgnoringModifiers, let key = chars.first else { return }
        if let slot = leftKeys.firstIndex(of: key) {
            navigator.pressLeft(slot: slot + 1)
        } else if let slot = rightKeys.firstIndex(of: key) {
            navigator.pressRight(slot: slot + 1)
        } else if event.keyCode == 126 {
            navigator.move(by: -1)
        } else if event.keyCode == 125 {
            navigator.move(by: 1)
        } else if key == "z" {
            zoom = zoom == 1 ? 2 : 1
            window?.setContentSize(Self.contentSize(zoom: zoom))
        } else if key == "f" {
            dump(navigator.state, prefix: "current", options: options)
            if let deckFeed {
                dumpDeck(deckFeed.current(), options: options)
            }
        } else {
            return
        }
        needsDisplay = true
    }
}

guard let options = parseOptions() else {
    FileHandle.standardError.write(Data((usage + "\n").utf8))
    exit(2)
}

let library: FixtureLibrary
do {
    library = try FixtureLibrary(url: URL(fileURLWithPath: options.path))
} catch {
    FileHandle.standardError.write(Data("cannot load \(options.path): \(error)\n".utf8))
    exit(1)
}

guard let navigator = SidebarNavigator(library: library) else {
    FileHandle.standardError.write(Data("no snapshots in \(options.path)\n".utf8))
    exit(1)
}

for (index, snapshot) in library.snapshots.enumerated() {
    dump(navigator.state(showing: snapshot), prefix: String(format: "%02d", index), options: options)
}

var deckFeed: DeckFeed?
if let path = options.deckSnapshot {
    do {
        deckFeed = try RecordedDeckFeed(snapshot: DeckSnapshot.load(URL(fileURLWithPath: path)))
    } catch {
        FileHandle.standardError.write(Data("cannot load \(path): \(error)\n".utf8))
        exit(1)
    }
} else if let deck = options.deck {
    let live = LiveDeckFeed(deck: deck)
    live.settle(polls: 4, interval: 0.25)
    deckFeed = live
}

if let deckFeed {
    dumpDeck(deckFeed.current(), options: options)
}

if options.windowless {
    exit(0)
}

let app = NSApplication.shared
app.setActivationPolicy(.regular)
let view = PreviewView(navigator: navigator, options: options)
view.deckFeed = deckFeed
if let live = deckFeed as? LiveDeckFeed {
    live.start()
    Timer.scheduledTimer(withTimeInterval: 0.1, repeats: true) { _ in view.needsDisplay = true }
}

let window = NSWindow(
    contentRect: NSRect(origin: .zero, size: PreviewView.contentSize(zoom: options.zoom)),
    styleMask: [.titled, .closable, .miniaturizable], backing: .buffered, defer: false
)
window.title = "S8Preview"
window.contentView = view
window.center()
window.makeKeyAndOrderFront(nil)
window.makeFirstResponder(view)
app.activate(ignoringOtherApps: true)
app.run()
