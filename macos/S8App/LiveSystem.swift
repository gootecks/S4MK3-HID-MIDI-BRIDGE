import AppKit
import DjaySidebar
import Foundation
import IOKit

@MainActor
struct LiveSystem: SystemProbe {
    static let controllerEditorID = "com.native-instruments.ControllerEditor"
    private static let bridgeNames: Set<String> = ["s4mk3-hid-midi-bridge", "S8Host"]
    private static let blockingPrefixes = [
        "com.native-instruments.traktor",
        "com.native-instruments.maschine",
        "com.native-instruments.kompletekontrol"
    ]

    let agent: AgentControlling

    func connectedS8Count() -> Int {
        guard let matching = IOServiceMatching("IOUSBHostDevice") as NSMutableDictionary? else { return 0 }
        matching["idVendor"] = 0x17CC
        matching["idProduct"] = 0x1370
        var iterator: io_iterator_t = 0
        guard IOServiceGetMatchingServices(kIOMainPortDefault, matching, &iterator) == KERN_SUCCESS else {
            return 0
        }
        defer { IOObjectRelease(iterator) }
        var count = 0
        while case let service = IOIteratorNext(iterator), service != 0 {
            count += 1
            IOObjectRelease(service)
        }
        return count
    }

    func otherBridgeProcesses() -> [String] {
        let own = getpid()
        let names = ProcessTable.entries().compactMap { entry -> String? in
            guard entry.pid != own else { return nil }
            let name = (entry.path as NSString).lastPathComponent
            return Self.bridgeNames.contains(name) ? name : nil
        }
        return Array(Set(names)).sorted()
    }

    func isAgentInstalled() -> Bool {
        agent.isInstalled
    }

    func blockingApplications() -> [String] {
        let names = NSWorkspace.shared.runningApplications.compactMap { app -> String? in
            guard let identifier = app.bundleIdentifier?.lowercased(),
                  Self.blockingPrefixes.contains(where: identifier.hasPrefix) else { return nil }
            return app.localizedName ?? identifier
        }
        return Array(Set(names)).sorted()
    }

    func isDjayRunning() -> Bool {
        LiveRoot.djayIsRunning()
    }

    func isAccessibilityTrusted(prompt: Bool) -> Bool {
        LiveRoot.isTrusted(prompt: prompt)
    }

    func isControllerEditorRunning() -> Bool {
        !NSRunningApplication.runningApplications(withBundleIdentifier: Self.controllerEditorID).isEmpty
    }
}
