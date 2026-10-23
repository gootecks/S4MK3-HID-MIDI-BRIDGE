import AppKit
import Darwin
import XCTest
@testable import S4Bridge

@MainActor
final class NIHardwareAgentNativeTests: XCTestCase {
    func testTerminationObservationSurvivesAutoreleasePoolUntilOwnedApplicationExits() async throws {
        let bundle = try makeFixture(source:
            "import AppKit\nNSApplication.shared.setActivationPolicy(.prohibited)\nNSApplication.shared.run()\n"
        )
        let configuration = NSWorkspace.OpenConfiguration()
        configuration.activates = false
        var application: NSRunningApplication? = try await NSWorkspace.shared.openApplication(
            at: bundle, configuration: configuration
        )
        let identifier = try XCTUnwrap(application).processIdentifier
        defer { if kill(identifier, 0) == 0 { kill(identifier, SIGTERM) } }
        application = nil

        let native = SystemNIHardwareAgentNative()
        let events = autoreleasepool { native.terminationEvents(for: identifier) }
        let exited = expectation(description: "owned application exit observed")
        let consumer = Task {
            for await _ in events { exited.fulfill(); return }
        }
        defer { consumer.cancel() }
        XCTAssertEqual(kill(identifier, SIGTERM), 0)
        await fulfillment(of: [exited], timeout: 10)
        consumer.cancel()
        await consumer.value
    }

    func testBackgroundLaunchCompletesOnProcessAppearanceWithoutAppKitHandshake() async throws {
        let bundle = try makeFixture(source:
            "import Foundation\nRunLoop.main.run()\n"
        )
        let restored = expectation(description: "background process restored without launch handshake")
        let native = SystemNIHardwareAgentNative(agentBundleURL: bundle)
        var result: Result<NIHardwareAgentProcess, Error>?
        let launch = Task {
            do { result = .success(try await native.launch(bundleURL: bundle, activates: false)) }
            catch { result = .failure(error) }
            restored.fulfill()
        }
        defer { launch.cancel() }
        await fulfillment(of: [restored], timeout: 5)
        let process = try XCTUnwrap(result).get()
        defer { kill(process.processIdentifier, SIGTERM) }
        XCTAssertTrue(native.runningProcesses().contains {
            $0.processIdentifier == process.processIdentifier
        })
        XCTAssertEqual(process.executableURL?.resolvingSymlinksInPath(),
                       bundle.appendingPathComponent("Contents/MacOS/ObserverFixture").resolvingSymlinksInPath())
        XCTAssertEqual(SystemNIHardwareAgentNative().terminate(
            processIdentifier: process.processIdentifier
        ), .denied)
        let events = native.terminationEvents(for: process.processIdentifier)
        let exited = expectation(description: "headless process exit observed")
        let consumer = Task {
            for await _ in events { exited.fulfill(); return }
        }
        XCTAssertEqual(native.terminate(processIdentifier: process.processIdentifier), .requested)
        await fulfillment(of: [exited], timeout: 5)
        consumer.cancel()
        await consumer.value
    }

    func testVerifiedStopBypassesCooperativeShutdown() async throws {
        let bundle = try makeFixture(source:
            "import Foundation\nRunLoop.main.run()\n"
        )
        let process = Process()
        process.executableURL = bundle.appendingPathComponent("Contents/MacOS/ObserverFixture")
        let exited = expectation(description: "verified fixture process exited")
        process.terminationHandler = { _ in exited.fulfill() }
        try process.run()
        defer {
            if process.isRunning { kill(process.processIdentifier, SIGKILL) }
        }
        let native = SystemNIHardwareAgentNative(agentBundleURL: bundle)

        XCTAssertEqual(native.terminate(processIdentifier: process.processIdentifier), .requested)
        await fulfillment(of: [exited], timeout: 5)

        guard !process.isRunning else {
            XCTFail("Verified fixture did not exit")
            return
        }
        XCTAssertEqual(process.terminationReason, .uncaughtSignal)
        XCTAssertEqual(process.terminationStatus, SIGKILL)
    }

    private func makeFixture(source fixtureSource: String) throws -> URL {
        let directory = FileManager.default.temporaryDirectory
            .appendingPathComponent("S4Observer-\(UUID().uuidString)", isDirectory: true)
        let bundle = directory.appendingPathComponent("Observer Fixture.app", isDirectory: true)
        addTeardownBlock { @MainActor in
            let native = SystemNIHardwareAgentNative(agentBundleURL: bundle)
            for process in native.runningProcesses() where
                process.executableURL?.deletingLastPathComponent().deletingLastPathComponent()
                    .deletingLastPathComponent().resolvingSymlinksInPath() == bundle.resolvingSymlinksInPath() {
                kill(process.processIdentifier, SIGTERM)
            }
            try FileManager.default.removeItem(at: directory)
        }
        let executable = bundle.appendingPathComponent("Contents/MacOS/ObserverFixture")
        try FileManager.default.createDirectory(
            at: executable.deletingLastPathComponent(), withIntermediateDirectories: true
        )
        let compiler = Process()
        compiler.executableURL = URL(fileURLWithPath: "/usr/bin/xcrun")
        compiler.arguments = ["swiftc", "-framework", "AppKit", "-o", executable.path, "-"]
        let source = Pipe()
        compiler.standardInput = source
        try compiler.run()
        try source.fileHandleForWriting.write(contentsOf: Data(fixtureSource.utf8))
        try source.fileHandleForWriting.close()
        compiler.waitUntilExit()
        XCTAssertEqual(compiler.terminationStatus, 0)
        let plist: [String: Any] = [
            "CFBundleIdentifier": "com.s4mk3.observer-fixture.\(UUID().uuidString)",
            "CFBundleExecutable": "ObserverFixture",
            "CFBundlePackageType": "APPL",
            "LSBackgroundOnly": true
        ]
        try PropertyListSerialization.data(fromPropertyList: plist, format: .xml, options: 0)
            .write(to: bundle.appendingPathComponent("Contents/Info.plist"))
        let signing = Process()
        signing.executableURL = URL(fileURLWithPath: "/usr/bin/codesign")
        signing.arguments = ["--force", "--sign", "-", bundle.path]
        try signing.run()
        signing.waitUntilExit()
        XCTAssertEqual(signing.terminationStatus, 0)
        return bundle
    }
}
