import AppKit
import Darwin
import Foundation
import Security

@MainActor
final class SystemNIHardwareAgentNative: NIHardwareAgentNative {
    private let agentBundleURL: URL

    convenience init() {
        self.init(agentBundleURL: NIHardwareAgent.installedBundleURL)
    }

    init(agentBundleURL: URL) {
        self.agentBundleURL = agentBundleURL
    }

    func runningProcesses() -> [NIHardwareAgentProcess] {
        var processes = NSWorkspace.shared.runningApplications.map {
            NIHardwareAgentProcess(
                processIdentifier: $0.processIdentifier,
                bundleIdentifier: $0.bundleIdentifier,
                executableURL: $0.executableURL,
                localizedName: $0.localizedName ?? $0.bundleIdentifier ?? "Unknown application"
            )
        }
        // Headless agents can exist before LaunchServices registers an application.
        if let bundle = Bundle(url: agentBundleURL), let executable = bundle.executableURL {
            let count = proc_listallpids(nil, 0)
            var identifiers = [pid_t](repeating: 0, count: max(0, Int(count)) * 2)
            let found = identifiers.withUnsafeMutableBytes {
                proc_listallpids($0.baseAddress, Int32($0.count))
            }
            for identifier in identifiers.prefix(max(0, Int(found))) where identifier > 0 {
                guard !processes.contains(where: { $0.processIdentifier == identifier }),
                      Self.executableURL(for: identifier) == executable.resolvingSymlinksInPath() else { continue }
                processes.append(NIHardwareAgentProcess(
                    processIdentifier: identifier, bundleIdentifier: bundle.bundleIdentifier,
                    executableURL: executable, localizedName: "NIHardwareAgent"
                ))
            }
        }
        return processes
    }

    func verifiedExecutableURL(bundleURL: URL, bundleIdentifier: String) throws -> URL {
        let bundle = Bundle(url: bundleURL)
        guard let bundle, bundle.bundleIdentifier == bundleIdentifier,
              let executable = bundle.executableURL,
              FileManager.default.isExecutableFile(atPath: executable.path) else {
            throw NIHardwareAgentError.invalidInstalledBundle(
                "The app bundle, identifier, or executable is missing or incorrect."
            )
        }

        var staticCode: SecStaticCode?
        var status = SecStaticCodeCreateWithPath(bundleURL as CFURL, [], &staticCode)
        guard status == errSecSuccess, let staticCode else {
            throw NIHardwareAgentError.invalidInstalledBundle(SecCopyErrorMessageString(status, nil) as String? ?? "No valid code signature.")
        }
        var requirement: SecRequirement?
        status = SecRequirementCreateWithString(
            "anchor apple generic and identifier \"\(bundleIdentifier)\" and certificate leaf[subject.OU] = \"\(NIHardwareAgent.teamIdentifier)\"" as CFString,
            [],
            &requirement
        )
        guard status == errSecSuccess, let requirement else {
            throw NIHardwareAgentError.invalidInstalledBundle("Could not create the Native Instruments identity requirement.")
        }
        let flags = SecCSFlags(rawValue: kSecCSStrictValidate | kSecCSCheckAllArchitectures | kSecCSCheckNestedCode)
        status = SecStaticCodeCheckValidity(staticCode, flags, requirement)
        guard status == errSecSuccess else {
            throw NIHardwareAgentError.invalidInstalledBundle(
                SecCopyErrorMessageString(status, nil) as String? ?? "The code signature does not match the expected identity."
            )
        }
        return executable.standardizedFileURL.resolvingSymlinksInPath()
    }

    func terminationEvents(for processIdentifier: pid_t) -> AsyncStream<Void> {
        return AsyncStream { continuation in
            let source = DispatchSource.makeProcessSource(
                identifier: processIdentifier, eventMask: .exit, queue: .main
            )
            source.setEventHandler {
                continuation.yield(())
                continuation.finish()
            }
            continuation.onTermination = { _ in source.cancel() }
            source.resume()
            if kill(processIdentifier, 0) != 0 && errno == ESRCH {
                continuation.yield(())
                continuation.finish()
            }
        }
    }

    func terminate(processIdentifier: pid_t) -> NIHardwareAgentTerminationResult {
        guard kill(processIdentifier, 0) == 0 || errno != ESRCH else { return .alreadyStopped }
        guard let expected = Bundle(url: agentBundleURL)?.executableURL,
              Self.executableURL(for: processIdentifier) == expected.resolvingSymlinksInPath() else { return .denied }
        // NI's SIGTERM shutdown blanks the S4 panels. Preserve their state for handover.
        if kill(processIdentifier, SIGKILL) == 0 { return .requested }
        return errno == ESRCH ? .alreadyStopped : .denied
    }

    func launch(bundleURL: URL, activates: Bool) async throws -> NIHardwareAgentProcess {
        guard let bundle = Bundle(url: bundleURL), let executable = bundle.executableURL else {
            throw NIHardwareAgentError.invalidInstalledBundle("The driver executable is missing.")
        }
        let process = Process()
        process.executableURL = executable
        process.standardInput = FileHandle.nullDevice
        process.standardOutput = FileHandle.nullDevice
        process.standardError = FileHandle.nullDevice
        try process.run()
        guard process.isRunning, let actual = Self.executableURL(for: process.processIdentifier) else {
            throw NIHardwareAgentError.launchFailed("The driver exited immediately after launch.")
        }
        return NIHardwareAgentProcess(
            processIdentifier: process.processIdentifier,
            bundleIdentifier: bundle.bundleIdentifier,
            executableURL: actual,
            localizedName: "NIHardwareAgent"
        )
    }

    private static func executableURL(for identifier: pid_t) -> URL? {
        var path = [CChar](repeating: 0, count: 4 * Int(MAXPATHLEN))
        guard proc_pidpath(identifier, &path, UInt32(path.count)) > 0 else { return nil }
        return URL(fileURLWithPath: String(cString: path)).resolvingSymlinksInPath()
    }
}
