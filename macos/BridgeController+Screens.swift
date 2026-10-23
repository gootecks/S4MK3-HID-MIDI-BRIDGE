import Foundation

extension BridgeController {
    func setScreensEnabled(_ enabled: Bool) {
        guard !screens.busy && state != .stopping else { return }
        screens.beginOperation()
        Task { @MainActor in
            defer { finishScreenOperation() }
            do {
                try await screens.setEnabled(enabled, consent: screenConsent)
            } catch {
                screens.fail(error)
                report(error)
            }
        }
    }

    func setScreenAutomation(stop: Bool? = nil, restore: Bool? = nil) {
        do {
            try screens.changePreferences {
                if let stop { $0.automaticallyStopNI = stop }
                if let restore { $0.restoreNIOnStop = restore }
            }
        } catch { report(error) }
    }

    func enableScreensByStoppingDriver() {
        guard !screens.busy && state == .running && screens.fullProfile else { return }
        screens.beginOperation()
        Task { @MainActor in
            defer { finishScreenOperation() }
            do { try await screens.enableAfterConsent() }
            catch {
                screens.fail(error)
                report(error)
            }
        }
    }

    func stopNIDriver() {
        guard !screens.busy else { return }
        screens.beginOperation()
        Task { @MainActor in
            defer { finishScreenOperation() }
            do {
                if screens.bridgeRunning && screens.fullProfile
                    && screens.preferences.enabled && screens.state != .active {
                    try await screens.enableAfterConsent()
                } else {
                    try await screens.stopDriver()
                }
            }
            catch {
                screens.fail(error)
                report(error)
            }
        }
    }

    func restartNIDriver() {
        guard !screens.busy && state != .stopping else { return }
        let needsConfirmation = hasChildProcess
        screens.beginOperation()
        Task { @MainActor in
            defer { finishScreenOperation() }
            if needsConfirmation {
                guard await confirmDriverRestore() else { return }
                if hasChildProcess {
                    screens.restoreAfterExit = true
                    stop()
                    return
                }
            }
            do {
                try await screens.restoreDriver()
                screens.state = screens.preferences.enabled ? .ready : .off
            } catch {
                screens.fail(error)
                report(error)
            }
        }
    }
}
