import Foundation
import XCTest
@testable import S4Bridge

@MainActor
final class NIHardwareAgentTests: XCTestCase {
    private let installedBundle = URL(fileURLWithPath: "/fixture/NIHardwareAgent.app", isDirectory: true)

    func testRunningRequiresExactBundleAndExecutableIdentity() throws {
        let native = FixtureNative()
        native.processes = [process()]
        let agent = makeAgent(native)

        XCTAssertTrue(try agent.isRunning())

        native.processes = [process(executableURL: URL(fileURLWithPath: "/tmp/Impostor"))]
        XCTAssertThrowsError(try agent.isRunning()) { error in
            XCTAssertEqual(error as? NIHardwareAgentError, .ambiguousRunningIdentity)
        }
    }

    func testStopRefusesWhileNativeInstrumentsUsersAreActive() async throws {
        let native = FixtureNative()
        native.processes = [
            process(),
            NIHardwareAgentProcess(
                processIdentifier: 420,
                bundleIdentifier: "com.native-instruments.Traktor",
                executableURL: URL(fileURLWithPath: "/Applications/Traktor.app/Contents/MacOS/Traktor"),
                localizedName: "Traktor"
            )
        ]

        do {
            try await makeAgent(native).stop()
            XCTFail("Stopping should be blocked while Traktor is active.")
        } catch let error as NIHardwareAgentError {
            XCTAssertEqual(error, .blocked(["Traktor"]))
        }
        XCTAssertFalse(native.terminationRequested)
    }

    func testStopSubscribesBeforeCooperativeTerminationAndAwaitsExit() async throws {
        let native = FixtureNative()
        native.processes = [process()]
        native.emitExitWhenTerminated = true

        try await makeAgent(native).stop()

        XCTAssertTrue(native.observerRegisteredBeforeTermination)
        XCTAssertTrue(native.terminationRequested)
    }

    func testStopReportsNativePermissionFailure() async {
        let native = FixtureNative()
        native.processes = [process()]
        native.allowTermination = false

        do {
            try await makeAgent(native).stop()
            XCTFail("A denied cooperative termination should fail.")
        } catch let error as NIHardwareAgentError {
            guard case .terminationDenied = error else {
                return XCTFail("Expected a native termination-denied result.")
            }
        } catch {
            XCTFail("Unexpected error: \(error)")
        }
        XCTAssertTrue(native.observerRegisteredBeforeTermination)
    }

    func testRestoreIsIdempotentAndLaunchesWithoutActivation() async throws {
        let native = FixtureNative()
        native.processes = [process()]
        try await makeAgent(native).restore()
        XCTAssertFalse(native.launchRequested)

        native.processes = []
        try await makeAgent(native).restore()
        XCTAssertTrue(native.launchRequested)
        XCTAssertFalse(native.launchActivated)
    }

    func testRestoreRejectsUnexpectedLaunchedExecutable() async {
        let native = FixtureNative()
        native.launchResult = process(executableURL: URL(fileURLWithPath: "/tmp/Impostor"))

        do {
            try await makeAgent(native).restore()
            XCTFail("A launch completion for another executable must be rejected.")
        } catch let error as NIHardwareAgentError {
            XCTAssertEqual(error, .invalidLaunchIdentity)
        } catch {
            XCTFail("Unexpected error: \(error)")
        }
    }

    func testNoRunningAgentDoesNotRequireAnInstalledBundle() throws {
        // Given a DJ laptop without the optional NI agent installed.
        let native = FixtureNative()
        native.verificationFails = true
        // When automatic preflight only checks whether the agent is running.
        XCTAssertFalse(try makeAgent(native).isRunning())
        // Then a missing agent cannot block ordinary bridge startup.
        XCTAssertEqual(native.verifications, 0)
    }

    func testTerminationDoesNotCompleteUntilTheExitEvent() async throws {
        // Given a subscribed native process with no exit event yet.
        let native = FixtureNative()
        native.processes = [process()]
        let requested = expectation(description: "cooperative termination requested")
        native.onTermination = { requested.fulfill() }
        var completed = false
        let agent = makeAgent(native)
        // When the termination request is accepted but the process remains alive.
        let stopping = Task { try await agent.stop(); completed = true }
        await fulfillment(of: [requested], timeout: 5)
        // Then the service waits for the subscribed exit rather than trusting the request.
        XCTAssertFalse(completed)
        native.emitExit()
        try await stopping.value
        XCTAssertTrue(completed)
    }

    func testAlreadyStoppedResultNeedsNoFurtherExitNotification() async throws {
        // Given a process that exits between observer registration and termination.
        let native = FixtureNative()
        native.processes = [process()]
        native.alreadyStopped = true
        let agent = NIHardwareAgent(bundleURL: installedBundle, native: native, terminationDeadline: 0)
        // When native termination reports that exact process is already gone.
        try await agent.stop()
        // Then success does not depend on an additional notification.
        XCTAssertTrue(native.terminationRequested)
    }

    func testMissingExitReportsBoundedTerminationTimeout() async {
        // Given a cooperative request that never gets an exit notification.
        let native = FixtureNative()
        native.processes = [process()]
        let agent = NIHardwareAgent(bundleURL: installedBundle, native: native, terminationDeadline: 0)
        // When the explicit deadline expires.
        do { try await agent.stop(); XCTFail("Expected timeout") }
        catch {
            // Then the typed timeout is reported rather than a false successful stop.
            XCTAssertEqual(error as? NIHardwareAgentError, .terminationTimedOut)
        }
    }

    private func makeAgent(_ native: FixtureNative) -> NIHardwareAgent {
        NIHardwareAgent(
            bundleURL: installedBundle,
            native: native,
            terminationDeadline: 10
        )
    }

    private func process(
        executableURL: URL? = URL(fileURLWithPath: "/fixture/NIHardwareAgent.app/Contents/MacOS/NIHardwareAgent")
    ) -> NIHardwareAgentProcess {
        NIHardwareAgentProcess(
            processIdentifier: 123,
            bundleIdentifier: NIHardwareAgent.bundleIdentifier,
            executableURL: executableURL,
            localizedName: "NIHardwareAgent"
        )
    }
}

@MainActor
private final class FixtureNative: NIHardwareAgentNative {
    var processes: [NIHardwareAgentProcess] = []
    var allowTermination = true
    var emitExitWhenTerminated = false
    var terminationRequested = false
    var observerRegisteredBeforeTermination = false
    var launchRequested = false
    var launchActivated = true
    var alreadyStopped = false
    var verificationFails = false
    var verifications = 0
    var onTermination: (() -> Void)?
    var launchResult = NIHardwareAgentProcess(
        processIdentifier: 456,
        bundleIdentifier: NIHardwareAgent.bundleIdentifier,
        executableURL: URL(fileURLWithPath: "/fixture/NIHardwareAgent.app/Contents/MacOS/NIHardwareAgent"),
        localizedName: "NIHardwareAgent"
    )

    private var exitContinuation: AsyncStream<Void>.Continuation?

    func runningProcesses() -> [NIHardwareAgentProcess] {
        processes
    }

    func verifiedExecutableURL(bundleURL: URL, bundleIdentifier: String) throws -> URL {
        verifications += 1
        if verificationFails { throw NIHardwareAgentError.invalidInstalledBundle("Missing fixture bundle") }
        guard bundleURL.path == "/fixture/NIHardwareAgent.app",
              bundleIdentifier == NIHardwareAgent.bundleIdentifier else {
            throw NIHardwareAgentError.invalidInstalledBundle("Unexpected fixture identity.")
        }
        return URL(fileURLWithPath: "/fixture/NIHardwareAgent.app/Contents/MacOS/NIHardwareAgent")
    }

    func terminationEvents(for processIdentifier: pid_t) -> AsyncStream<Void> {
        observerRegisteredBeforeTermination = true
        return AsyncStream { continuation in
            exitContinuation = continuation
        }
    }

    func terminate(processIdentifier: pid_t) -> NIHardwareAgentTerminationResult {
        terminationRequested = true
        guard allowTermination else { return .denied }
        if alreadyStopped { return .alreadyStopped }
        onTermination?()
        if emitExitWhenTerminated {
            emitExit()
        }
        return .requested
    }

    func emitExit() {
        exitContinuation?.yield(())
        exitContinuation?.finish()
    }

    func launch(bundleURL: URL, activates: Bool) async throws -> NIHardwareAgentProcess {
        launchRequested = true
        launchActivated = activates
        return launchResult
    }
}
