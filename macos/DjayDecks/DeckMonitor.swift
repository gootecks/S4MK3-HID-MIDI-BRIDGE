import Foundation
import S8Views

public final class DeckMonitor {
    private let reader: DeckReader
    private let catalog: TrackCatalog
    private let decks: [Int]
    private var trackers: [Int: DeckTracker]
    private var lastFailure: String?
    private let lookInterval: TimeInterval
    private var lastLook: [Int: Date] = [:]
    private let lock = NSLock()
    private let log: ((String) -> Void)?
    private var lastPollStart: UInt64?

    public init(
        source: DeckAccessibility, catalog: TrackCatalog, decks: [Int] = [1, 2], lookInterval: TimeInterval = 5,
        log: ((String) -> Void)? = nil
    ) {
        self.lookInterval = lookInterval
        reader = DeckReader(source: source)
        self.catalog = catalog
        self.decks = decks
        self.log = log
        trackers = Dictionary(uniqueKeysWithValues: decks.map { ($0, DeckTracker(deck: $0)) })
    }

    public static func live(decks: [Int] = [1, 2], log: ((String) -> Void)? = nil) -> DeckMonitor {
        DeckMonitor(source: AXDeckSource(), catalog: LibraryCatalog(), decks: decks, log: log)
    }

    public func poll(now: Date = Date()) {
        poll(now: now, uptime: { DispatchTime.now().uptimeNanoseconds })
    }

    func poll(now: Date, uptime: () -> UInt64) {
        let start = log.map { _ in uptime() }
        let result = Result { try reader.readings() }
        let timing = start.map { DeckPollTiming(start: $0, previous: lastPollStart, axFinished: uptime()) }
        lastPollStart = start
        switch result {
        case let .success(readings):
            withLock { lastFailure = nil }
            for deck in decks {
                let reading = readings[deck]
                let workStart = timing.map { _ in uptime() }
                let library = update(deck: deck, reading: reading, now: now, uptime: uptime)
                if let timing, let workStart {
                    log?(timing.line(
                        deck: deck, reading: reading, work: uptime() - workStart, library: library
                    ))
                }
            }
        case let .failure(error):
            recordFailure(error)
            if let timing {
                let work = uptime() - timing.axFinished
                for deck in decks {
                    log?(timing.line(deck: deck, reading: nil, work: work, library: nil))
                }
            }
        }
    }

    private func update(deck: Int, reading: DeckReading?, now: Date, uptime: () -> UInt64) -> UInt64? {
        guard let reading else {
            withLock { trackers[deck]?.ingestPause("Deck \(deck) is not shown in djay") }
            return nil
        }
        let current = withLock { trackers[deck] }
        var entry: CatalogEntry?
        var library: UInt64?
        let changed = TrackKey(reading) != current?.trackKey || current?.needsEntry == true
        let due = withLock { lastLook[deck].map { now.timeIntervalSince($0) >= lookInterval } ?? true }
        if reading.hasTrack, let title = reading.title, changed || due {
            let length = TrackLengthHint(reading)
            let started = log.map { _ in uptime() }
            entry = catalog.entry(title: title, artist: reading.artist, length: length)
            library = started.map { uptime() - $0 }
            withLock { lastLook[deck] = now }
        }
        withLock { trackers[deck]?.ingest(reading, entry: entry, now: now) }
        return library
    }

    private func recordFailure(_ error: Error) {
        switch error {
        case DeckAccessError.notAnswering:
            pause("djay is not answering")
        case DeckAccessError.permissionDenied:
            fail("Accessibility permission needed")
        default:
            fail("djay is not running")
        }
    }

    public func snapshot(deck: Int, now: Date = Date()) -> DeckSnapshot {
        withLock { trackers[deck]?.snapshot(now: now) } ?? DeckSnapshot(deck: deck)
    }

    public var failure: String? {
        withLock { lastFailure }
    }

    private func fail(_ message: String) {
        withLock {
            lastFailure = message
            for deck in decks {
                trackers[deck]?.ingestFailure(message)
            }
        }
    }

    private func pause(_ message: String) {
        withLock {
            lastFailure = message
            for deck in decks {
                trackers[deck]?.ingestPause(message)
            }
        }
    }

    private func withLock<T>(_ body: () -> T) -> T {
        lock.lock()
        defer { lock.unlock() }
        return body()
    }
}

private struct DeckPollTiming {
    let start: UInt64
    let previous: UInt64?
    let axFinished: UInt64

    func line(deck: Int, reading: DeckReading?, work: UInt64, library: UInt64?) -> String {
        let gap = previous.map { Self.seconds(start - $0) } ?? "-"
        let axDuration = axFinished - start
        let lookup = library == nil ? "no" : "yes"
        return "deck poll gap=\(gap) read=\(Self.seconds(axDuration + work)) ax=\(Self.seconds(axDuration))"
            + " library=\(library.map(Self.seconds) ?? "-") lookup=\(lookup)"
            + " state=\(Self.state(reading)) deck=\(deck)"
    }

    private static func state(_ reading: DeckReading?) -> String {
        guard let reading else { return "unavailable" }
        guard reading.hasTrack else { return "empty" }
        return reading.isPlaying ? "playing" : "paused"
    }

    private static func seconds(_ nanoseconds: UInt64) -> String {
        String(format: "%.3f", Double(nanoseconds) / 1_000_000_000)
    }
}
