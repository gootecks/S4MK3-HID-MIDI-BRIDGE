import Foundation

enum S8Layout: String, CaseIterable {
    case standard = "default"
    case evan

    var title: String {
        switch self {
        case .standard: "Default"
        case .evan: "Evan"
        }
    }

    var mappingPrefix: String {
        switch self {
        case .standard: "Kontrol S8 Bridge r"
        case .evan: "Kontrol S8 Bridge Evan r"
        }
    }
}

struct S8Preferences: Equatable {
    var layout: S8Layout = .standard
    var leds = true
    var panels = true
    var rightPanelDeck: Int?
    var startOnLaunch = false

    func hostArguments(bridge: URL) -> [String] {
        var arguments = ["--bridge", bridge.path, "--layout", layout.rawValue]
        if panels {
            arguments.append("--screens")
            if let deck = rightPanelDeck {
                arguments += ["--deck", String(deck)]
            }
        }
        if leds {
            arguments.append("--leds")
        }
        return arguments
    }
}

extension S8Preferences {
    private enum Key {
        static let layout = "layout"
        static let leds = "deckLEDs"
        static let panels = "panels"
        static let rightPanelDeck = "rightPanelDeck"
        static let startOnLaunch = "startOnLaunch"
    }

    init(defaults: UserDefaults) {
        self.init()
        if let raw = defaults.string(forKey: Key.layout), let stored = S8Layout(rawValue: raw) {
            layout = stored
        }
        if defaults.object(forKey: Key.leds) != nil {
            leds = defaults.bool(forKey: Key.leds)
        }
        if defaults.object(forKey: Key.panels) != nil {
            panels = defaults.bool(forKey: Key.panels)
        }
        let deck = defaults.integer(forKey: Key.rightPanelDeck)
        rightPanelDeck = (1 ... 4).contains(deck) ? deck : nil
        startOnLaunch = defaults.bool(forKey: Key.startOnLaunch)
    }

    func save(to defaults: UserDefaults) {
        defaults.set(layout.rawValue, forKey: Key.layout)
        defaults.set(leds, forKey: Key.leds)
        defaults.set(panels, forKey: Key.panels)
        defaults.set(rightPanelDeck ?? 0, forKey: Key.rightPanelDeck)
        defaults.set(startOnLaunch, forKey: Key.startOnLaunch)
    }
}
