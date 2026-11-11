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
    private var interpolator = TimeInterpolator()
    private var debouncer = PlayStateDebouncer()
    private var playing = false

    public init(deck: Int) {
        self.deck = deck
    }

    public var needsEntry: Bool {
        trackKey != nil && entry == nil
    }

    public mutating func ingest(_ reading: DeckReading, entry resolved: CatalogEntry?, now: Date = Date()) {
        let key = reading.hasTrack ? TrackKey(reading) : nil
        if key != trackKey {
            trackKey = key
            entry = resolved
            interpolator = TimeInterpolator()
            debouncer = PlayStateDebouncer()
        } else if let resolved {
            entry = resolved
        }
        self.reading = reading
        notice = nil
        let advanced = interpolator.isNextSecond(
            elapsedTime: reading.elapsedTime, remainingTime: reading.remainingTime
        )
        playing = debouncer.update(isPlaying: reading.isPlaying, timeAdvanced: advanced, now: now)
        interpolator.update(
            elapsedTime: reading.elapsedTime, remainingTime: reading.remainingTime,
            knownDuration: knownDuration, isPlaying: playing, playingSince: debouncer.activeSince,
            bpmPercent: reading.bpmPercent, now: now
        )
    }

    public mutating func ingestFailure(_ message: String) {
        trackKey = nil
        entry = nil
        reading = nil
        notice = message
        playing = false
    }

    public mutating func ingestPause(_ message: String) {
        notice = message
        playing = false
        interpolator = TimeInterpolator()
        debouncer = PlayStateDebouncer()
    }

    private var knownDuration: Double? {
        entry?.duration ?? entry?.analysis?.duration
    }

    public func snapshot(now: Date = Date()) -> DeckSnapshot {
        guard notice == nil, let reading, reading.hasTrack else {
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
