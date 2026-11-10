import Foundation
import S8Views

public final class DeckMonitor {
    private let reader: DeckReader
    private let catalog: TrackCatalog
    private let decks: [Int]
    private var trackers: [Int: DeckTracker]
    private let lock = NSLock()

    public init(source: DeckAccessibility, catalog: TrackCatalog, decks: [Int] = [1, 2]) {
        reader = DeckReader(source: source)
        self.catalog = catalog
        self.decks = decks
        trackers = Dictionary(uniqueKeysWithValues: decks.map { ($0, DeckTracker(deck: $0)) })
    }

    public static func live(decks: [Int] = [1, 2]) -> DeckMonitor {
        DeckMonitor(source: AXDeckSource(), catalog: LibraryCatalog(), decks: decks)
    }

    public func poll(now: Date = Date()) {
        for deck in decks {
            do {
                let reading = try reader.read(deck: deck)
                let current = withLock { trackers[deck] }
                var entry: CatalogEntry?
                let changed = TrackKey(reading) != current?.trackKey || current?.needsEntry == true
                if reading.hasTrack, let title = reading.title, changed {
                    entry = catalog.entry(title: title, artist: reading.artist)
                }
                withLock { trackers[deck]?.ingest(reading, entry: entry, now: now) }
            } catch DeckAccessError.permissionDenied {
                fail(deck, "Accessibility permission needed")
            } catch {
                fail(deck, "djay is not running")
            }
        }
    }

    public func snapshot(deck: Int, now: Date = Date()) -> DeckSnapshot {
        withLock { trackers[deck]?.snapshot(now: now) } ?? DeckSnapshot(deck: deck)
    }

    private func fail(_ deck: Int, _ message: String) {
        withLock { trackers[deck]?.ingestFailure(message) }
    }

    private func withLock<T>(_ body: () -> T) -> T {
        lock.lock()
        defer { lock.unlock() }
        return body()
    }
}
