import Foundation

public struct DjayCuePoint: Equatable {
    public let time: TimeInterval
    public let endTime: TimeInterval?
    public let name: String?
    public let colorHex: String?
    public let number: Int?

    static let defaultColors = [
        1: "#CC0000", 2: "#CC8800", 3: "#0000CC", 4: "#CCCC00",
        5: "#00CC00", 6: "#CC00CC", 7: "#00CCCC", 8: "#8800CC"
    ]
}

public struct DjayTrackMatches: Equatable {
    public let tracks: [DjayTrack]
    public let artistMatched: Bool
}

public extension DjayLibrarySnapshot {
    internal static func makeCuePoints(_ user: DjayValue?) -> [DjayCuePoint] {
        let entries = user?["cuePoints"]?.listValue ?? []
        return entries.compactMap { entry -> DjayCuePoint? in
            guard entry.className == nil || entry.className == "ADCCuePoint",
                  let time = entry["time"]?.numberValue, time.isFinite, time >= 0 else { return nil }
            let end = entry["endTime"]?.numberValue.flatMap { $0 >= 0 ? $0 : nil }
            return DjayCuePoint(
                time: time, endTime: end, name: entry["comment"]?.stringValue,
                colorHex: entry["color"]?.stringValue ?? DjayCuePoint.defaultColors[entry["number"]?.intValue ?? 1],
                number: entry["number"]?.intValue
            )
        }.sorted { $0.time < $1.time }
    }

    func cuePoints(trackID: String) -> [DjayCuePoint] {
        cuePointsByID[trackID] ?? []
    }

    func tracks(title: String, artist: String?) -> DjayTrackMatches {
        guard let wanted = TrackIdentity(title: title, artist: artist) else {
            return DjayTrackMatches(tracks: [], artistMatched: false)
        }
        let sameTitle = tracks.filter { TrackIdentity.normalize($0.title) == wanted.title }
        let exact = sameTitle.filter { TrackIdentity.normalize($0.artist) == wanted.artist }
        return exact.isEmpty
            ? DjayTrackMatches(tracks: sameTitle, artistMatched: false)
            : DjayTrackMatches(tracks: exact, artistMatched: true)
    }
}
