import AppKit

@MainActor
final class AppDelegate: NSObject, NSApplicationDelegate, NSMenuDelegate {
    private let statusItem = NSStatusBar.system.statusItem(withLength: NSStatusItem.variableLength)
    private var preferences = S8Preferences(defaults: .standard)
    private var runner: S8Runner?
    private var shownFailure: String?
    private var quitting = false
    private var readyToQuit = false

    func applicationDidFinishLaunching(_: Notification) {
        let executables = Bundle.main.bundleURL.appendingPathComponent("Contents/MacOS", isDirectory: true)
        let resources = Bundle.main.bundleURL.appendingPathComponent("Contents/Resources", isDirectory: true)
        let agent = NIAgent()
        let runner = S8Runner(
            probe: LiveSystem(agent: agent),
            agent: agent,
            host: BundledHost(
                host: executables.appendingPathComponent("S8Host"),
                bridge: executables.appendingPathComponent("s4mk3-hid-midi-bridge")
            ),
            mappings: MappingInstaller(
                source: resources.appendingPathComponent("Mappings", isDirectory: true),
                destination: MappingInstaller.djayFolder
            )
        )
        runner.onChange = { [weak self] in self?.refresh() }
        self.runner = runner

        let menu = NSMenu()
        menu.delegate = self
        menu.autoenablesItems = false
        statusItem.menu = menu
        if let button = statusItem.button {
            button.image = NSImage(
                systemSymbolName: "slider.horizontal.3", accessibilityDescription: "Kontrol S8 Bridge"
            )
            button.imagePosition = .imageLeading
        }
        refresh()

        Task {
            await runner.recoverAgent()
            if preferences.startOnLaunch {
                await start()
            }
        }
    }

    /// Stops S8Host and restores the agent before the app ends. The reply is never deferred:
    /// a quit from a signal arrives on the main queue, which a deferred reply would block.
    func applicationShouldTerminate(_: NSApplication) -> NSApplication.TerminateReply {
        guard let runner, !readyToQuit, !(runner.state.isIdle && !runner.agentStoppedByApp) else {
            return .terminateNow
        }
        guard !quitting else { return .terminateCancel }
        quitting = true
        Task {
            await runner.shutdown()
            readyToQuit = true
            NSApp.terminate(nil)
        }
        return .terminateCancel
    }

    func menuNeedsUpdate(_ menu: NSMenu) {
        guard let runner else { return }
        StatusMenu.fill(menu, runner: runner, preferences: preferences, target: self)
    }

    private func refresh() {
        guard let runner else { return }
        if let button = statusItem.button {
            button.title = " S8"
            button.appearsDisabled = runner.state != .running
            button.toolTip = "Kontrol S8 Bridge: \(runner.state.title)"
        }
        if let menu = statusItem.menu {
            StatusMenu.fill(menu, runner: runner, preferences: preferences, target: self)
        }
        if case let .failed(message) = runner.state, message != shownFailure {
            shownFailure = message
            DispatchQueue.main.async { Self.alert("Kontrol S8 Bridge stopped", message) }
        } else if !runner.state.isIdle {
            shownFailure = nil
        }
    }

    private func start() async {
        guard let runner else { return }
        await runner.start(preferences) { [weak self] prompt in
            await self?.answer(prompt) ?? false
        }
    }

    private func answer(_ prompt: RunnerPrompt) async -> Bool {
        switch prompt {
        case let .restartDjay(names):
            let response = Self.alert(
                "Mapping installed",
                "Installed \(names.joined(separator: " and ")) in djay's MIDI Mappings folder. "
                    + "djay Pro lists a new mapping after it opens again: quit and reopen djay Pro, "
                    + "then pick it in Settings > MIDI.",
                buttons: ["Continue", "Cancel"]
            )
            return response == .alertFirstButtonReturn
        case .controllerEditorClosed:
            let response = Self.alert(
                "Controller Editor is not open",
                "Started without Controller Editor, the S8 was once dark and unusable. "
                    + "Open it first, or start anyway.",
                buttons: ["Open Controller Editor", "Start Anyway", "Cancel"]
            )
            switch response {
            case .alertFirstButtonReturn: return await openControllerEditor()
            case .alertSecondButtonReturn: return true
            default: return false
            }
        }
    }

    private func openControllerEditor() async -> Bool {
        let workspace = NSWorkspace.shared
        guard let url = workspace.urlForApplication(withBundleIdentifier: LiveSystem.controllerEditorID) else {
            Self.alert("Controller Editor not found", "Install the Native Instruments Controller Editor.")
            return false
        }
        let configuration = NSWorkspace.OpenConfiguration()
        configuration.activates = false
        guard await (try? workspace.openApplication(at: url, configuration: configuration)) != nil else {
            return false
        }
        // Give it time to reach the agent before the agent stops.
        try? await Task.sleep(nanoseconds: 5_000_000_000)
        return true
    }

    @discardableResult
    static func alert(_ title: String, _ message: String, buttons: [String] = ["OK"]) -> NSApplication.ModalResponse {
        NSApp.activate(ignoringOtherApps: true)
        let alert = NSAlert()
        alert.messageText = title
        alert.informativeText = message
        buttons.forEach { alert.addButton(withTitle: $0) }
        return alert.runModal()
    }

    // MARK: Menu actions

    @objc func toggleRun() {
        guard let runner else { return }
        Task {
            if runner.state.isIdle {
                await start()
            } else {
                await runner.stop()
            }
        }
    }

    @objc func chooseLayout(_ sender: NSMenuItem) {
        guard let raw = sender.representedObject as? String, let layout = S8Layout(rawValue: raw) else { return }
        update { $0.layout = layout }
    }

    @objc func toggleLEDs() {
        update { $0.leds.toggle() }
    }

    @objc func togglePanels() {
        update { $0.panels.toggle() }
    }

    @objc func chooseRightPanel(_ sender: NSMenuItem) {
        update { $0.rightPanelDeck = sender.tag == 0 ? nil : sender.tag }
    }

    @objc func toggleStartOnLaunch() {
        update { $0.startOnLaunch.toggle() }
    }

    @objc func installMappings() {
        guard let runner else { return }
        do {
            let installed = try runner.installMappings()
            Self.alert(
                installed.isEmpty ? "Mappings already installed" : "Mappings installed",
                installed.isEmpty
                    ? "djay's MIDI Mappings folder has every mapping this app carries."
                    : "Installed \(installed.joined(separator: " and ")). Quit and reopen djay Pro to see them."
            )
        } catch {
            Self.alert("Mappings not installed", error.localizedDescription)
        }
    }

    @objc func openLog() {
        if let url = runner?.logURL {
            NSWorkspace.shared.open(url)
        }
    }

    @objc func showLogs() {
        try? FileManager.default.createDirectory(at: RunLog.folder, withIntermediateDirectories: true)
        NSWorkspace.shared.open(RunLog.folder)
    }

    private func update(_ change: (inout S8Preferences) -> Void) {
        change(&preferences)
        preferences.save(to: .standard)
        refresh()
    }
}
