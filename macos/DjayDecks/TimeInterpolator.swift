import Foundation

public struct TimeInterpolator {
    private var elapsed = LabelClock(direction: 1)
    private var remaining = LabelClock(direction: -1)
    private var knownDuration: Double?

    public init() {}

    public mutating func update(
        elapsedTime: String?, remainingTime: String?, knownDuration: Double? = nil, isPlaying: Bool,
        playingSince: Date? = nil, bpmPercent: String?, now: Date = Date()
    ) {
        self.knownDuration = knownDuration
        let motion = LabelClock.Motion(
            isPlaying: isPlaying, credit: playingSince.map { max(0, now.timeIntervalSince($0)) } ?? 0,
            rate: Self.parsePlaybackRate(bpmPercent)
        )
        elapsed.update(label: elapsedTime.flatMap(Self.parseTime), trackStart: 0, motion: motion, now: now)
        remaining.update(
            label: remainingTime.flatMap(Self.parseTime), trackStart: knownDuration, motion: motion, now: now
        )
    }

    public func isNextSecond(elapsedTime: String?, remainingTime: String?) -> Bool {
        elapsed.isTick(to: elapsedTime.flatMap(Self.parseTime))
            || remaining.isTick(to: remainingTime.flatMap(Self.parseTime))
    }

    public func interpolatedElapsed(now: Date = Date()) -> Double? {
        if let value = elapsed.value(trackStart: 0, now: now) {
            return value
        }
        guard let knownDuration, let left = interpolatedRemaining(now: now) else { return nil }
        return max(0, knownDuration - left)
    }

    public func interpolatedRemaining(now: Date = Date()) -> Double? {
        remaining.value(trackStart: knownDuration, now: now)
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

private struct LabelClock {
    struct Motion {
        let isPlaying: Bool
        let credit: TimeInterval
        let rate: Double
    }

    let direction: Double
    private var label: Double?
    private var counted: Double?
    private var countedAt = Date.distantPast
    private var isPlaying = false
    private var rate = 1.0

    init(direction: Double) {
        self.direction = direction
    }

    func isTick(to next: Double?) -> Bool {
        guard let label, let next else { return false }
        return next == label + direction
    }

    func value(trackStart: Double?, now: Date) -> Double? {
        guard let label else { return nil }
        guard let counted else { return Self.resting(label, trackStart: trackStart) }
        let moved = isPlaying ? direction * max(0, now.timeIntervalSince(countedAt)) * rate : 0
        return Self.clamp(counted + moved, to: label)
    }

    mutating func update(label next: Double?, trackStart: Double?, motion: Motion, now: Date) {
        guard let next else {
            self = LabelClock(direction: direction)
            return
        }
        var current = counted == nil ? nil : value(trackStart: trackStart, now: now)
        if motion.isPlaying, !isPlaying, let label {
            let rest = current ?? Self.resting(label, trackStart: trackStart)
            current = Self.clamp(rest + direction * motion.credit * motion.rate, to: label)
        }
        if let label, next != label {
            if motion.isPlaying || isPlaying, isTick(to: next) {
                current = Self.clamp(current ?? Self.resting(label, trackStart: trackStart), to: next)
            } else {
                current = nil
            }
        }
        if motion.isPlaying, current == nil {
            current = Self.resting(next, trackStart: trackStart)
        }
        label = next
        counted = current
        countedAt = now
        isPlaying = motion.isPlaying
        rate = motion.rate
    }

    private static func resting(_ label: Double, trackStart: Double?) -> Double {
        if let trackStart, trackStart.rounded(.down) == label {
            return trackStart
        }
        return label + 0.5
    }

    private static func clamp(_ value: Double, to label: Double) -> Double {
        min(max(value, label), (label + 1).nextDown)
    }
}
