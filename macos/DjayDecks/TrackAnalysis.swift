import Foundation

public struct TrackAnalysis: Equatable {
    public let name: String?
    public let artist: String?
    public let duration: Double?
    public let bpm: Double?
    public let amplitude: [UInt8]
    public let amplitudeRate: Double
    public let bands: [[UInt8]]
    public let bandRate: Double
    public let beats: [Double]
    public let firstDownBeatIndex: Int?

    public static let beatsPerBar = 4

    public var bars: [Double] {
        guard let first = firstDownBeatIndex, !beats.isEmpty else { return [] }
        let positions = beatPositions
        let anchor: Int = if first >= 0, first < beats.count {
            positions[first]
        } else if first < 0 {
            first
        } else {
            positions[beats.count - 1] &+ (first - (beats.count - 1))
        }
        let phase = Self.modulo(anchor)
        return zip(beats, positions).filter { Self.modulo($1) == phase }.map(\.0)
    }

    static func modulo(_ value: Int) -> Int {
        ((value % beatsPerBar) + beatsPerBar) % beatsPerBar
    }

    var beatPositions: [Int] {
        var positions = [Int](repeating: 0, count: beats.count)
        for index in beats.indices.dropFirst() {
            positions[index] = positions[index - 1] + beatsSpanned(by: index - 1)
        }
        return positions
    }

    private func beatsSpanned(by interval: Int) -> Int {
        let window = max(0, interval - 4) ... min(beats.count - 2, interval + 4)
        let gaps = window.map { beats[$0 + 1] - beats[$0] }
        let gap = beats[interval + 1] - beats[interval]
        let reference = gaps.sorted()[gaps.count / 2]
        let ratio = gap / reference
        guard reference.isFinite, reference > 0, ratio.isFinite, ratio >= 1.5, ratio < 1000 else { return 1 }
        let count = ratio.rounded()
        return abs(ratio - count) <= 0.25 ? Int(count) : 1
    }

    public static func load(url: URL) throws -> TrackAnalysis {
        let data = try Data(contentsOf: url)
        return try decode(data)
    }

    public static func decode(_ data: Data) throws -> TrackAnalysis {
        guard let root = try PropertyListSerialization.propertyList(from: data, format: nil) as? [String: Any] else {
            throw AnalysisError.notADictionary
        }
        let info = root["info"] as? [String: Any]
        let beatInfo = root["deepBeatTrackerInfo"] as? [String: Any]
        let waveInfo = root["waveInfoCompact"] as? [String: Any]
        let colorInfo = root["waveColorsInfo"] as? [String: Any]

        let amplitude = (waveInfo?["compressedLowRateWaveSamples"] as? Data).flatMap(Inflate.zlib) ?? []
        let duration = (info?["Duration"] as? NSNumber)?.doubleValue
        let amplitudeRate = (waveInfo?["lowRateWaveFinalSampleRate"] as? NSNumber)?.doubleValue
            ?? duration.flatMap { $0 > 0 && !amplitude.isEmpty ? Double(amplitude.count) / $0 : nil } ?? 0
        let colors = (colorInfo?["compressedLowRateWaveColors"] as? Data).flatMap(Inflate.zlib) ?? []
        let beats = (beatInfo?["compressedBeats"] as? Data).flatMap(Inflate.zlib).map(Self.floats) ?? []
        return TrackAnalysis(
            name: info?["Name"] as? String, artist: info?["Artist"] as? String, duration: duration,
            bpm: (beatInfo?["bpm"] as? NSNumber)?.doubleValue, amplitude: amplitude,
            amplitudeRate: amplitudeRate, bands: Self.bands(colors),
            bandRate: (colorInfo?["lowRateSampleRate"] as? NSNumber)?.doubleValue ?? 0, beats: beats,
            firstDownBeatIndex: (beatInfo?["firstDownBeatIndex"] as? NSNumber)?.intValue
        )
    }

    static func floats(_ bytes: [UInt8]) -> [Double] {
        stride(from: 0, to: bytes.count - 3, by: 4).map { index in
            let bits = UInt32(bytes[index]) << 24 | UInt32(bytes[index + 1]) << 16
                | UInt32(bytes[index + 2]) << 8 | UInt32(bytes[index + 3])
            return Double(Float(bitPattern: bits))
        }
    }

    static func bands(_ bytes: [UInt8]) -> [[UInt8]] {
        let samples = bytes.count / 6
        guard samples > 0 else { return [] }
        var raw = [[Float]](repeating: [], count: 3)
        for channel in 0 ..< 3 {
            raw[channel] = (0 ..< samples).map { sample in
                let offset = sample * 6 + channel * 2
                return Float(bitPattern: UInt32(bytes[offset]) << 24 | UInt32(bytes[offset + 1]) << 16)
            }
        }
        let peak = raw.flatMap { $0 }.filter(\.isFinite).max() ?? 0
        guard peak > 0 else { return [] }
        return raw.map { channel in
            channel.map { $0.isFinite ? UInt8(min(255, max(0, $0 / peak * 255))) : 0 }
        }
    }
}

public enum AnalysisError: Error, Equatable {
    case notADictionary
}
