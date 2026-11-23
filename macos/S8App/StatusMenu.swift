import AppKit

@MainActor
enum StatusMenu {
    static func fill(_ menu: NSMenu, runner: S8Runner, preferences: S8Preferences, target: AppDelegate) {
        menu.removeAllItems()
        let state = runner.state
        menu.addItem(label("Kontrol S8 Bridge: \(state.title)"))
        if case let .failed(message) = state {
            menu.addItem(label(message))
        } else if !runner.detail.isEmpty {
            menu.addItem(label(runner.detail))
        }
        menu.addItem(.separator())

        let busy = !state.isIdle && state != .running
        let run = item(state.isIdle ? "Start" : "Stop", #selector(AppDelegate.toggleRun), target)
        run.keyEquivalent = state.isIdle ? "s" : "."
        run.isEnabled = !busy
        menu.addItem(run)
        menu.addItem(.separator())
        addSettings(menu, editable: state.isIdle, preferences: preferences, target: target)
        addTools(menu, runner: runner, target: target)
    }

    private static func addSettings(_ menu: NSMenu, editable: Bool, preferences: S8Preferences, target: AppDelegate) {
        if !editable {
            menu.addItem(label("Stop to change these."))
        }

        let layout = NSMenuItem(title: "Layout", action: nil, keyEquivalent: "")
        let layouts = NSMenu()
        for choice in S8Layout.allCases {
            let entry = item(choice.title, #selector(AppDelegate.chooseLayout(_:)), target)
            entry.representedObject = choice.rawValue
            entry.state = preferences.layout == choice ? .on : .off
            entry.isEnabled = editable
            layouts.addItem(entry)
        }
        layout.submenu = layouts
        menu.addItem(layout)

        let leds = item("Deck LEDs", #selector(AppDelegate.toggleLEDs), target)
        leds.state = preferences.leds ? .on : .off
        leds.isEnabled = editable
        menu.addItem(leds)

        let panels = item("Panels", #selector(AppDelegate.togglePanels), target)
        panels.state = preferences.panels ? .on : .off
        panels.isEnabled = editable
        menu.addItem(panels)

        let right = NSMenuItem(title: "Right Panel", action: nil, keyEquivalent: "")
        let decks = NSMenu()
        let choices = [(0, "Library")] + (1 ... 4).map { ($0, "Deck \($0)") }
        for (tag, title) in choices {
            let entry = item(title, #selector(AppDelegate.chooseRightPanel(_:)), target)
            entry.tag = tag
            entry.state = (preferences.rightPanelDeck ?? 0) == tag ? .on : .off
            entry.isEnabled = editable && preferences.panels
            decks.addItem(entry)
        }
        right.submenu = decks
        right.isEnabled = preferences.panels
        menu.addItem(right)

        let launch = item("Start When the App Opens", #selector(AppDelegate.toggleStartOnLaunch), target)
        launch.state = preferences.startOnLaunch ? .on : .off
        menu.addItem(launch)
        menu.addItem(.separator())
    }

    private static func addTools(_ menu: NSMenu, runner: S8Runner, target: AppDelegate) {
        menu.addItem(item("Install Mappings in djay", #selector(AppDelegate.installMappings), target))
        let log = item("Open Run Log", #selector(AppDelegate.openLog), target)
        log.isEnabled = runner.logURL != nil
        menu.addItem(log)
        menu.addItem(item("Show Log Folder", #selector(AppDelegate.showLogs), target))
        menu.addItem(.separator())
        let quit = NSMenuItem(title: "Quit", action: #selector(NSApplication.terminate(_:)), keyEquivalent: "q")
        menu.addItem(quit)
    }

    private static func label(_ text: String) -> NSMenuItem {
        let entry = NSMenuItem(title: text, action: nil, keyEquivalent: "")
        entry.isEnabled = false
        return entry
    }

    private static func item(_ title: String, _ action: Selector, _ target: AppDelegate) -> NSMenuItem {
        let entry = NSMenuItem(title: title, action: action, keyEquivalent: "")
        entry.target = target
        return entry
    }
}
