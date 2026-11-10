import Foundation
import S8Views

public struct TrackKey: Equatable {
    public let title: String?
    public let artist: String?

    public init(_ reading: DeckReading) {
        title = reading.title
        artist = reading.artist
    }
}

public struct DeckTracker {
    public let deck: Int
    public private(set) var trackKey: TrackKey?
    private var entry: CatalogEntry?
    private var reading: DeckReading?
    private var notice: String?
    private var interpolator: TimeInterpolator
    private var debouncer = PlayStateDebouncer()
    private var playing = false

    public init(deck: Int, now: Date = Date()) {
        self.deck = deck
        interpolator = TimeInterpolator(now: now)
    }

    public var needsEntry: Bool {
        trackKey != nil && entry == nil
    }

    public mutating func ingest(_ reading: DeckReading, entry resolved: CatalogEntry?, now: Date = Date()) {
        let key = reading.hasTrack ? TrackKey(reading) : nil
        if key != trackKey {
            trackKey = key
            entry = resolved
            interpolator = TimeInterpolator(now: now)
            debouncer = PlayStateDebouncer()
        } else if entry == nil {
            entry = resolved
        }
        self.reading = reading
        notice = nil
        interpolator.update(
            elapsedTime: reading.elapsedTime, remainingTime: reading.remainingTime,
            knownDuration: knownDuration, isPlaying: reading.isPlaying, bpmPercent: reading.bpmPercent, now: now
        )
        playing = debouncer.update(isPlaying: reading.isPlaying, now: now)
    }

    public mutating func ingestFailure(_ message: String) {
        trackKey = nil
        entry = nil
        reading = nil
        notice = message
        playing = false
    }

    private var knownDuration: Double? {
        entry?.duration ?? entry?.analysis?.duration
    }

    public func snapshot(now: Date = Date()) -> DeckSnapshot {
        guard let reading, reading.hasTrack else {
            return DeckSnapshot(deck: deck, capturedAt: Self.stamp(now), notice: notice)
        }
        let analysis = entry?.analysis.map { analysis in
            DeckSnapshot.Analysis(
                amplitude: analysis.amplitude, amplitudeRate: analysis.amplitudeRate, bands: analysis.bands,
                bandRate: analysis.bandRate, beats: analysis.beats, bars: analysis.bars, bpm: analysis.bpm
            )
        }
        let cues = (entry?.cues ?? []).map {
            DeckSnapshot.Cue(time: $0.time, name: $0.name, color: $0.colorHex)
        }
        return DeckSnapshot(
            deck: deck, capturedAt: Self.stamp(now), title: reading.title, artist: reading.artist,
            bpm: reading.bpm, key: reading.key, elapsed: interpolator.interpolatedElapsed(now: now),
            remaining: interpolator.interpolatedRemaining(now: now), duration: knownDuration, playing: playing,
            analysis: analysis, cues: cues
        )
    }

    static func stamp(_ date: Date) -> String {
        ISO8601DateFormatter().string(from: date)
    }
}
