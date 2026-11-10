import Foundation

public struct TimeInterpolator {
    private var lastElapsedSeconds: Double?
    private var lastRemainingSeconds: Double?
    private var lastUpdateTime: Date
    private var isPlaying = false
    private var playbackRate = 1.0
    private var elapsedCorrection = 0.0
    private var elapsedCorrectionStart: Date?
    private var remainingCorrection = 0.0
    private var remainingCorrectionStart: Date?
    private let correctionDuration: TimeInterval = 0.25

    public init(now: Date = Date()) {
        lastUpdateTime = now
    }

    public mutating func update(
        elapsedTime: String?, remainingTime: String?, knownDuration: Double? = nil, isPlaying: Bool,
        bpmPercent: String?, now: Date = Date()
    ) {
        let parsedElapsed = elapsedTime.flatMap { Self.parseTime($0) }
        let newRemaining = remainingTime.flatMap { Self.parseTime($0) }
        let derivedElapsed = knownDuration.flatMap { duration in newRemaining.map { max(0, duration - $0) } }
        let newElapsed = parsedElapsed ?? derivedElapsed

        if elapsedTime == nil, derivedElapsed == nil {
            lastElapsedSeconds = nil
        }
        if remainingTime == nil {
            lastRemainingSeconds = nil
        }
        let predictedElapsed = interpolatedElapsed(now: now)
        let predictedRemaining = interpolatedRemaining(now: now)

        if let elapsed = newElapsed, elapsed != lastElapsedSeconds {
            elapsedCorrection = (predictedElapsed ?? elapsed) - elapsed
            elapsedCorrectionStart = now
            lastElapsedSeconds = elapsed
            lastUpdateTime = now
        }
        if let remaining = newRemaining, remaining != lastRemainingSeconds {
            remainingCorrection = (predictedRemaining ?? remaining) - remaining
            remainingCorrectionStart = now
            lastRemainingSeconds = remaining
            lastUpdateTime = now
        }
        self.isPlaying = isPlaying
        playbackRate = Self.parsePlaybackRate(bpmPercent)
    }

    public func interpolatedElapsed(now: Date = Date()) -> Double? {
        guard let base = lastElapsedSeconds else { return nil }
        let raw = isPlaying ? max(0, base + now.timeIntervalSince(lastUpdateTime) * playbackRate) : base
        let eased = Self.decayedCorrection(
            elapsedCorrection, since: elapsedCorrectionStart, duration: correctionDuration, now: now
        )
        return max(0, raw + eased)
    }

    public func interpolatedRemaining(now: Date = Date()) -> Double? {
        guard let base = lastRemainingSeconds else { return nil }
        let raw = isPlaying ? max(0, base - now.timeIntervalSince(lastUpdateTime) * playbackRate) : base
        let eased = Self.decayedCorrection(
            remainingCorrection, since: remainingCorrectionStart, duration: correctionDuration, now: now
        )
        return max(0, raw + eased)
    }

    private static func decayedCorrection(
        _ correction: Double, since start: Date?, duration: TimeInterval, now: Date
    ) -> Double {
        guard correction != 0, let start else { return 0 }
        let progress = min(1, max(0, now.timeIntervalSince(start) / duration))
        guard progress < 1 else { return 0 }
        return correction * pow(1 - progress, 3)
    }

    public static func parseTime(_ text: String) -> Double? {
        let cleaned = text.trimmingCharacters(in: .whitespaces).replacingOccurrences(of: "-", with: "")
        let parts = cleaned.split(separator: ":")
        guard parts.count == 2, let minutes = Double(parts[0]), let seconds = Double(parts[1]) else { return nil }
        return minutes * 60 + seconds
    }

    private static func parsePlaybackRate(_ bpmPercent: String?) -> Double {
        guard let text = bpmPercent, let percent = Double(text.replacingOccurrences(of: "%", with: "")) else {
            return 1
        }
        return 1 + percent / 100
    }
}
