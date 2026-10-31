import Foundation

public struct DjayCoPlay: Equatable {
    public let trackID: String?
    public let title: String?
    public let artist: String?
    public let count: Int
    public let lastPlayed: Date
}

public struct DjayCoPlayNeighbours: Equatable {
    public let before: [DjayCoPlay]
    public let after: [DjayCoPlay]
}

public extension DjayLibrarySnapshot {
    func coPlayNeighbours(of trackID: String, limit: Int = 10) -> DjayCoPlayNeighbours {
        var before: [String: DjayCoPlay] = [:]
        var after: [String: DjayCoPlay] = [:]
        func record(_ play: DjayHistoryPlay, into table: inout [String: DjayCoPlay]) {
            let identity = TrackIdentity(title: play.title, artist: play.artist)
            let key = play.trackID ?? identity.map { "\($0.title)\t\($0.artist)" } ?? play.titleID ?? play.id
            let previous = table[key]
            table[key] = DjayCoPlay(
                trackID: play.trackID,
                title: previous?.title ?? play.title ?? play.trackID.flatMap { tracksByID[$0]?.title },
                artist: previous?.artist ?? play.artist ?? play.trackID.flatMap { tracksByID[$0]?.artist },
                count: (previous?.count ?? 0) + 1, lastPlayed: max(previous?.lastPlayed ?? .distantPast, play.startTime)
            )
        }
        for session in historySessions {
            for (position, play) in session.plays.enumerated() where play.trackID == trackID {
                if position > 0, session.plays[position - 1].trackID != trackID {
                    record(session.plays[position - 1], into: &before)
                }
                if position + 1 < session.plays.count, session.plays[position + 1].trackID != trackID {
                    record(session.plays[position + 1], into: &after)
                }
            }
        }
        func ranked(_ table: [String: DjayCoPlay]) -> [DjayCoPlay] {
            let sorted = table.values.sorted {
                if $0.count != $1.count {
                    return $0.count > $1.count
                }
                if $0.lastPlayed != $1.lastPlayed {
                    return $0.lastPlayed > $1.lastPlayed
                }
                return ($0.title ?? "") < ($1.title ?? "")
            }
            return Array(sorted.prefix(max(limit, 0)))
        }
        return DjayCoPlayNeighbours(before: ranked(before), after: ranked(after))
    }
}
