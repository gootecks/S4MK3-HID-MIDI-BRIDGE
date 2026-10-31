import Foundation

public struct DjayTrack: Equatable, Hashable {
    public let id: String
    public let title: String?
    public let artist: String?
    public let album: String?
    public let genre: String?
    public let bpm: Double?
    public let keyIndex: Int?
    public let key: String?
    public let duration: TimeInterval?
    public let path: String?
    public let sourceURI: String?
    public let originSource: String?
    public let addedDate: Date?
}

public enum DjayPlaylistKind: Equatable {
    case playlist
    case folder
    case smart
    case unknown(Int)
}

public struct DjayPlaylist: Equatable {
    public let id: String
    public let name: String?
    public let kind: DjayPlaylistKind
    public let parentID: String?
    public let childIDs: [String]
    public let trackIDs: [String]
}

public struct DjayQueueEntry: Equatable {
    public let id: String
    public let trackID: String?
    public let title: String?
    public let artist: String?
}

public struct DjayHistoryPlay: Equatable {
    public let id: String
    public let sessionID: String
    public let index: Int
    public let startTime: Date
    public let deck: Int?
    public let title: String?
    public let artist: String?
    public let isrc: String?
    public let titleID: String?
    public let duration: TimeInterval?
    public let trackID: String?
}

public struct DjayHistorySession: Equatable {
    public let id: String
    public let deviceName: String?
    public let startDate: Date?
    public let endDate: Date?
    public let plays: [DjayHistoryPlay]
}
