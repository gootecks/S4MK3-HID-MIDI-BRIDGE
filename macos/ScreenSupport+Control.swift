import Foundation

extension ScreenSupport {
    func acceptStatus(_ line: String) {
        let fields = line.split(separator: " ")
        guard fields.count == 3, fields[0] == "S4_SCREEN_STATUS",
              let received = UInt64(fields[1]), received == generation else { return }
        switch fields[2] {
        case "active":
            if preferences.enabled { state = .active }
        case "inactive":
            if state != .blocked { state = preferences.enabled ? .ready : .off }
            finishRelease()
        case "failed":
            state = .failed
            errorMessage = "The S4 screens could not be opened. Check the NI driver, then try Enable screens again."
            finishRelease()
        case "pending":
            if preferences.enabled && state != .blocked { state = .pending }
        default:
            return
        }
    }

    private func finishRelease() {
        releaseDeadline?.cancel()
        releaseDeadline = nil
        let waiter = releaseWaiter
        releaseWaiter = nil
        waiter?.resume()
    }

    func request(_ command: ScreenCommand) throws {
        guard bridgeRunning && fullProfile, let sendCommand else {
            throw AppError.invalid("Start the full-controller bridge before changing live screens.")
        }
        generation += 1
        try sendCommand(generation, command)
    }

    func releaseScreens() async throws {
        guard bridgeRunning && fullProfile else { return }
        guard releaseWaiter == nil else {
            throw AppError.invalid("A screen release is already in progress.")
        }
        state = .releasing
        try await withCheckedThrowingContinuation { continuation in
            releaseWaiter = continuation
            let deadline = DispatchWorkItem { [weak self] in
                guard let self else { return }
                let waiter = self.releaseWaiter
                self.releaseWaiter = nil
                waiter?.resume(throwing: AppError.invalid(
                    "The bridge did not confirm releasing its screens. Stop the bridge before restoring the NI driver."
                ))
            }
            releaseDeadline = deadline
            DispatchQueue.main.asyncAfter(deadline: .now() + 5, execute: deadline)
            do { try request(.off) }
            catch {
                releaseDeadline?.cancel()
                releaseDeadline = nil
                releaseWaiter = nil
                continuation.resume(throwing: error)
            }
        }
    }

    func setEnabled(_ enabled: Bool, consent: () -> ScreenConsent) async throws {
        let previous = preferences.enabled
        try changePreferences { $0.enabled = enabled }
        errorMessage = ""
        guard enabled else {
            try await releaseScreens()
            state = .off
            return
        }
        guard bridgeRunning && fullProfile else {
            state = .ready
            refreshDriverStatus()
            return
        }
        guard let useScreens = try await prepare(full: true, consent: consent) else {
            try changePreferences { $0.enabled = previous }
            state = previous ? .ready : .off
            return
        }
        if useScreens { try request(.on) }
    }

    func enableAfterConsent() async throws {
        guard bridgeRunning && fullProfile else {
            throw AppError.invalid("Start the full-controller bridge to enable its screens.")
        }
        try await stopDriver()
        state = .pending
        try request(.retry)
    }
}
