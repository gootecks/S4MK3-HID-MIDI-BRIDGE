@testable import S8App
import XCTest

@MainActor
final class S8RunnerTests: XCTestCase {
    private var probe: FakeProbe!
    private var agent: FakeAgent!
    private var host: FakeHost!
    private var folder: TestFolder!
    private var defaults: UserDefaults!
    private var prompts: [RunnerPrompt] = []
    private var answer = true

    override func setUp() async throws {
        probe = FakeProbe()
        agent = FakeAgent()
        host = FakeHost()
        folder = TestFolder()
        defaults = UserDefaults(suiteName: "S8RunnerTests-\(UUID().uuidString)")
        prompts = []
        answer = true
        let source = folder.folder("bundle")
        for name in ["Kontrol S8 Bridge r9", "Kontrol S8 Bridge Evan r4"] {
            FileManager.default.createFile(
                atPath: source.appendingPathComponent("\(name).djayMidiMapping").path, contents: Data("x".utf8)
            )
        }
        _ = folder.folder("djay")
    }

    override func tearDown() async throws {
        folder.remove()
    }

    private func makeRunner() -> S8Runner {
        let logs = folder.folder("logs")
        return S8Runner(
            probe: probe,
            agent: agent,
            host: host,
            mappings: MappingInstaller(source: folder.folder("bundle"), destination: folder.folder("djay")),
            defaults: defaults,
            makeLog: { RunLog(folder: logs) }
        )
    }

    private func start(_ runner: S8Runner, _ preferences: S8Preferences = S8Preferences()) async {
        await runner.start(preferences) { prompt in
            self.prompts.append(prompt)
            return self.answer
        }
    }

    private func settle(until condition: () -> Bool) async {
        for _ in 0 ..< 1000 where !condition() {
            await Task.yield()
        }
    }

    func testPreflightProblemLeavesAgentRunning() async {
        probe.s8Count = 0
        let runner = makeRunner()
        await start(runner)
        XCTAssertEqual(runner.state, .failed(PreflightIssue.noController.localizedDescription))
        XCTAssertEqual(agent.events, [])
        XCTAssertNil(host.arguments)
    }

    func testRunStopsAgentAndStopRestoresIt() async {
        let runner = makeRunner()
        await start(runner, S8Preferences(layout: .evan, leds: true, panels: true, rightPanelDeck: 2))
        XCTAssertEqual(agent.events, ["stop"])
        XCTAssertTrue(runner.agentStoppedByApp)
        XCTAssertEqual(host.arguments, [
            "--bridge", host.bridge.path, "--layout", "evan", "--screens", "--deck", "2", "--leds"
        ])
        XCTAssertEqual(runner.state, .starting)

        host.onLine?("2026-11-20 INFO MIDI_FULL_READY")
        await settle { runner.state == .running }
        XCTAssertEqual(runner.state, .running)
        XCTAssertEqual(runner.mappingName, "Kontrol S8 Bridge Evan r4")

        host.running.onStop = { [host] in host?.onExit?(0) }
        await runner.stop()
        XCTAssertTrue(host.running.stopped)
        XCTAssertEqual(agent.events, ["stop", "restore"])
        XCTAssertEqual(runner.state, .stopped)
        XCTAssertFalse(runner.agentStoppedByApp)
    }

    func testUnexpectedExitRestoresAgentAndReportsFailure() async {
        let runner = makeRunner()
        await start(runner)
        host.onExit?(1)
        await settle { runner.state.isIdle }
        XCTAssertEqual(agent.events, ["stop", "restore"])
        XCTAssertEqual(runner.state, .failed("S8Host stopped with status 1."))
    }

    func testLaunchFailureRestoresAgent() async {
        host.launchError = CocoaError(.fileNoSuchFile)
        let runner = makeRunner()
        await start(runner)
        XCTAssertEqual(agent.events, ["stop", "restore"])
        XCTAssertFalse(runner.agentStoppedByApp)
        guard case let .failed(message) = runner.state else { return XCTFail("not failed") }
        XCTAssertTrue(message.hasPrefix("Cannot start S8Host"))
    }

    func testFailedRestoreIsRetriedOnNextLaunch() async {
        agent.failRestore = true
        let runner = makeRunner()
        await start(runner)
        host.onExit?(0)
        await settle { runner.state.isIdle }
        guard case let .failed(message) = runner.state else { return XCTFail("not failed") }
        XCTAssertTrue(message.contains("open -g"))
        XCTAssertTrue(runner.agentStoppedByApp)

        agent.failRestore = false
        let next = makeRunner()
        await next.recoverAgent()
        XCTAssertEqual(agent.events, ["stop", "restore", "restore"])
        XCTAssertTrue(agent.running)
        XCTAssertFalse(next.agentStoppedByApp)
    }

    func testDecliningControllerEditorPromptStopsNothing() async throws {
        probe.controllerEditor = false
        answer = false
        let runner = makeRunner()
        _ = try runner.installMappings()
        await start(runner)
        XCTAssertEqual(prompts.last, .controllerEditorClosed)
        XCTAssertEqual(runner.state, .stopped)
        XCTAssertEqual(agent.events, [])
    }

    func testNewMappingAsksForDjayRestart() async {
        answer = false
        let runner = makeRunner()
        await start(runner)
        XCTAssertEqual(prompts, [.restartDjay(["Kontrol S8 Bridge Evan r4", "Kontrol S8 Bridge r9"])])
        XCTAssertEqual(agent.events, [])

        await start(runner)
        XCTAssertEqual(prompts.count, 1)
        XCTAssertEqual(agent.events, ["stop"])
    }
}
