import Foundation

public struct PlayStateDebouncer {
    private var rawState = false
    private var reportedState = false
    private var lastRawChangeTime = Date.distantPast
    private let threshold: TimeInterval

    public init(threshold: TimeInterval = 0.7) {
        self.threshold = threshold
    }

    public mutating func update(isPlaying: Bool, now: Date = Date()) -> Bool {
        if isPlaying != rawState {
            rawState = isPlaying
            lastRawChangeTime = now
        }
        if rawState, !reportedState {
            if now.timeIntervalSince(lastRawChangeTime) >= threshold {
                reportedState = true
            }
        } else if !rawState, reportedState {
            reportedState = false
        }
        return reportedState
    }
}
