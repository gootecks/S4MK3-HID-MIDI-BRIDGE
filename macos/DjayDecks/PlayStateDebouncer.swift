import Foundation

public struct PlayStateDebouncer {
    private var rawState = false
    private var reportedState = false
    private var lastRawChangeTime = Date.distantPast
    private var lastSampleTime = Date.distantPast
    private let threshold: TimeInterval
    private let maximumGap: TimeInterval

    public init(threshold: TimeInterval = 0.7, maximumGap: TimeInterval = 0.45) {
        self.threshold = threshold
        self.maximumGap = maximumGap
    }

    public var activeSince: Date? {
        rawState ? lastRawChangeTime : nil
    }

    public mutating func update(isPlaying: Bool, timeAdvanced: Bool = false, now: Date = Date()) -> Bool {
        let gapped = now.timeIntervalSince(lastSampleTime) > maximumGap
        lastSampleTime = now
        if isPlaying != rawState || (isPlaying && !reportedState && gapped) {
            rawState = isPlaying
            lastRawChangeTime = now
        }
        if rawState, !reportedState {
            if timeAdvanced || now.timeIntervalSince(lastRawChangeTime) >= threshold {
                reportedState = true
            }
        } else if !rawState, reportedState {
            reportedState = false
        }
        return reportedState
    }
}
