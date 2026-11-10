import Foundation

public struct DeckSnapshot: Equatable, Codable {
    public struct Cue: Equatable, Codable {
        public let time: Double
        public let name: String?
        public let color: String?

        public init(time: Double, name: String? = nil, color: String? = nil) {
            self.time = time
            self.name = name
            self.color = color
        }
    }

    public struct Analysis: Equatable, Codable {
        public let amplitude: [UInt8]
        public let amplitudeRate: Double
        public let bands: [[UInt8]]
        public let bandRate: Double
        public let beats: [Double]
        public let bars: [Double]
        public let bpm: Double?

        public init(
            amplitude: [UInt8], amplitudeRate: Double, bands: [[UInt8]] = [], bandRate: Double = 0,
            beats: [Double] = [], bars: [Double] = [], bpm: Double? = nil
        ) {
            self.amplitude = amplitude
            self.amplitudeRate = amplitudeRate
            self.bands = bands
            self.bandRate = bandRate
            self.beats = beats
            self.bars = bars
            self.bpm = bpm
        }

        public var duration: Double? {
            amplitudeRate > 0 && !amplitude.isEmpty ? Double(amplitude.count) / amplitudeRate : nil
        }
    }

    public static let supportedSchema = 1

    public let schema: Int
    public let deck: Int
    public let capturedAt: String?
    public let title: String?
    public let artist: String?
    public let bpm: String?
    public let key: String?
    public let elapsed: Double?
    public let remaining: Double?
    public let duration: Double?
    public let playing: Bool
    public let notice: String?
    public let analysis: Analysis?
    public let cues: [Cue]

    public init(
        deck: Int, capturedAt: String? = nil, title: String? = nil, artist: String? = nil, bpm: String? = nil,
        key: String? = nil, elapsed: Double? = nil, remaining: Double? = nil, duration: Double? = nil,
        playing: Bool = false, notice: String? = nil, analysis: Analysis? = nil, cues: [Cue] = []
    ) {
        schema = Self.supportedSchema
        self.deck = deck
        self.capturedAt = capturedAt
        self.title = title
        self.artist = artist
        self.bpm = bpm
        self.key = key
        self.elapsed = elapsed
        self.remaining = remaining
        self.duration = duration
        self.playing = playing
        self.notice = notice
        self.analysis = analysis
        self.cues = cues
    }

    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        schema = try container.decode(Int.self, forKey: .schema)
        deck = try container.decode(Int.self, forKey: .deck)
        capturedAt = try container.decodeIfPresent(String.self, forKey: .capturedAt)
        title = try container.decodeIfPresent(String.self, forKey: .title)
        artist = try container.decodeIfPresent(String.self, forKey: .artist)
        bpm = try container.decodeIfPresent(String.self, forKey: .bpm)
        key = try container.decodeIfPresent(String.self, forKey: .key)
        elapsed = try container.decodeIfPresent(Double.self, forKey: .elapsed)
        remaining = try container.decodeIfPresent(Double.self, forKey: .remaining)
        duration = try container.decodeIfPresent(Double.self, forKey: .duration)
        playing = try container.decodeIfPresent(Bool.self, forKey: .playing) ?? false
        notice = try container.decodeIfPresent(String.self, forKey: .notice)
        analysis = try container.decodeIfPresent(Analysis.self, forKey: .analysis)
        cues = try container.decodeIfPresent([Cue].self, forKey: .cues) ?? []
    }

    public var hasTrack: Bool {
        title != nil || artist != nil
    }

    public var totalDuration: Double? {
        duration ?? analysis?.duration
    }

    public var position: Double? {
        if let elapsed {
            return elapsed
        }
        guard let remaining, let total = totalDuration else { return nil }
        return max(0, total - remaining)
    }

    public static func decode(_ data: Data) throws -> DeckSnapshot {
        let snapshot = try JSONDecoder().decode(DeckSnapshot.self, from: data)
        guard snapshot.schema == supportedSchema else { throw SnapshotError.unsupportedSchema(snapshot.schema) }
        return snapshot
    }

    public static func load(_ url: URL) throws -> DeckSnapshot {
        try decode(Data(contentsOf: url))
    }

    public func encoded() throws -> Data {
        let encoder = JSONEncoder()
        encoder.outputFormatting = [.prettyPrinted, .sortedKeys]
        return try encoder.encode(self)
    }
}
