import Foundation
import S8Views

public final class DeckMonitor {
    private let reader: DeckReader
    private let catalog: TrackCatalog
    private let decks: [Int]
    private var trackers: [Int: DeckTracker]
    private let lookInterval: TimeInterval
    private var lastLook: [Int: Date] = [:]
    private let lock = NSLock()

    public init(
        source: DeckAccessibility, catalog: TrackCatalog, decks: [Int] = [1, 2], lookInterval: TimeInterval = 5
    ) {
        self.lookInterval = lookInterval
        reader = DeckReader(source: source)
        self.catalog = catalog
        self.decks = decks
        trackers = Dictionary(uniqueKeysWithValues: decks.map { ($0, DeckTracker(deck: $0)) })
    }

    public static func live(decks: [Int] = [1, 2]) -> DeckMonitor {
        DeckMonitor(source: AXDeckSource(), catalog: LibraryCatalog(), decks: decks)
    }

    public func poll(now: Date = Date()) {
        let readings: [Int: DeckReading]
        do {
            readings = try reader.readings()
        } catch DeckAccessError.notAnswering {
            return pause("djay is not answering")
        } catch DeckAccessError.permissionDenied {
            return fail("Accessibility permission needed")
        } catch {
            return fail("djay is not running")
        }
        for deck in decks {
            let reading = readings[deck] ?? DeckReading()
            let current = withLock { trackers[deck] }
            var entry: CatalogEntry?
            let changed = TrackKey(reading) != current?.trackKey || current?.needsEntry == true
            let due = withLock { lastLook[deck].map { now.timeIntervalSince($0) >= lookInterval } ?? true }
            if reading.hasTrack, let title = reading.title, changed || due {
                entry = catalog.entry(title: title, artist: reading.artist)
                withLock { lastLook[deck] = now }
            }
            withLock { trackers[deck]?.ingest(reading, entry: entry, now: now) }
        }
    }

    public func snapshot(deck: Int, now: Date = Date()) -> DeckSnapshot {
        withLock { trackers[deck]?.snapshot(now: now) } ?? DeckSnapshot(deck: deck)
    }

    private func fail(_ message: String) {
        withLock {
            for deck in decks {
                trackers[deck]?.ingestFailure(message)
            }
        }
    }

    private func pause(_ message: String) {
        withLock {
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
