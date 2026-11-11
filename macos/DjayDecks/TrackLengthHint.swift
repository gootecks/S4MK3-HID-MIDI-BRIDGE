import Foundation

public struct TrackLengthHint: Equatable {
    public static let tolerance = 2.0

    public let minimum: Double
    public let maximum: Double?

    public init(minimum: Double, maximum: Double? = nil) {
        self.minimum = minimum
        self.maximum = maximum
    }

    public init?(elapsedTime: String?, remainingTime: String?) {
        let remaining = remainingTime.flatMap(TimeInterpolator.parseTime)
        guard let remaining else { return nil }
        if let elapsed = elapsedTime.flatMap(TimeInterpolator.parseTime) {
            let length = elapsed + remaining
            self.init(minimum: length - Self.tolerance, maximum: length + Self.tolerance)
        } else {
            self.init(minimum: remaining - 1)
        }
    }

    public init?(_ reading: DeckReading) {
        self.init(elapsedTime: reading.elapsedTime, remainingTime: reading.remainingTime)
    }

    public func admits(_ duration: Double) -> Bool {
        duration >= minimum && duration <= (maximum ?? .infinity)
    }
}
