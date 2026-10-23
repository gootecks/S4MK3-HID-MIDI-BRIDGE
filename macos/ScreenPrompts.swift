import AppKit

@MainActor
enum ScreenPrompts {
    static func confirmStop() -> ScreenConsent {
        let (alert, remember) = stopAlert()
        switch alert.runModal() {
        case .alertFirstButtonReturn: return .allow(remember: remember.state == .on)
        case .alertSecondButtonReturn: return .withoutScreens
        default: return .cancel
        }
    }

    static func stopAlert() -> (NSAlert, NSButton) {
        let alert = NSAlert()
        alert.messageText = "Enable the S4 screens?"
        alert.informativeText = "The Native Instruments screen driver software is running in the background and will prevent the screens from functioning in this app. We can temporarily stop it and restore it when you quit."
        alert.addButton(withTitle: "Enable screens")
        alert.addButton(withTitle: "Continue without screens")
        alert.addButton(withTitle: "Cancel")
        let remember = NSButton(checkboxWithTitle: "Do this automatically in future", target: nil, action: nil)
        alert.accessoryView = remember
        return (alert, remember)
    }

    static func confirmStopBridge() -> Bool {
        let alert = NSAlert()
        alert.messageText = "Restore the Native Instruments screen driver?"
        alert.informativeText = "Stop the bridge first so it releases the S4 screens. The driver can then be restarted for Traktor."
        alert.addButton(withTitle: "Stop bridge & restore NI driver")
        alert.addButton(withTitle: "Cancel")
        return alert.runModal() == .alertFirstButtonReturn
    }
}
