import Foundation

public struct DeckReading: Equatable {
    public var key: String?
    public var title: String?
    public var artist: String?
    public var bpm: String?
    public var elapsedTime: String?
    public var remainingTime: String?
    public var bpmPercent: String?
    public var isPlaying: Bool
    public var lineVolume: String?

    public init(
        key: String? = nil, title: String? = nil, artist: String? = nil, bpm: String? = nil,
        elapsedTime: String? = nil, remainingTime: String? = nil, bpmPercent: String? = nil,
        isPlaying: Bool = false, lineVolume: String? = nil
    ) {
        self.key = key
        self.title = title
        self.artist = artist
        self.bpm = bpm
        self.elapsedTime = elapsedTime
        self.remainingTime = remainingTime
        self.bpmPercent = bpmPercent
        self.isPlaying = isPlaying
        self.lineVolume = lineVolume
    }

    public var hasTrack: Bool {
        title != nil || artist != nil
    }
}

public enum DeckParser {
    public static let deckKeywords = ["Deck", "Platine"]

    public static func parseDeckNumber(from label: String) -> Int? {
        for keyword in deckKeywords {
            guard let range = label.range(of: "\(keyword) (\\d+)", options: .regularExpression) else { continue }
            return Int(label[range].dropFirst(keyword.count + 1))
        }
        return nil
    }

    public static func labelPrefix(_ label: String) -> String {
        for suffix in [", Deck ", ", Platine "] {
            if let range = label.range(of: suffix) {
                return String(label[label.startIndex ..< range.lowerBound])
            }
        }
        return label
    }

    public static func reading(from labeled: [String: String]) -> DeckReading {
        var reading = DeckReading()
        for (label, value) in labeled where !value.isEmpty {
            let lower = label.lowercased()
            let property = labelPrefix(label)
            if lower.starts(with: "key,") || lower.starts(with: "clé,") {
                reading.key = value
            } else if lower.starts(with: "title,") || lower.starts(with: "titre,") {
                reading.title = value
            } else if lower.starts(with: "artist,") || lower.starts(with: "artiste,") {
                reading.artist = value
            } else if lower.starts(with: "elapsed time,") {
                reading.elapsedTime = value
            } else if lower.starts(with: "remaining time,") {
                reading.remainingTime = value
            } else if lower.starts(with: "play /") {
                reading.isPlaying = value == "Active"
            } else if property.range(of: #"^\d+\.\d+$"#, options: .regularExpression) != nil {
                reading.bpm = property
            } else if property.range(of: #"^[+-]?\d+\.\d+%$"#, options: .regularExpression) != nil {
                reading.bpmPercent = property
            } else if lower.starts(with: "line volume,") {
                reading.lineVolume = value
            }
        }
        return reading
    }
}
