import Foundation

public struct DjayLibrarySnapshot {
    public static let rootPlaylistID = "mediaItemPlaylist-root"
    public static let defaultQueueID = "queue-default"

    public let tracks: [DjayTrack]
    public let playlists: [DjayPlaylist]
    public let queue: [DjayQueueEntry]
    public let historySessions: [DjayHistorySession]

    let tracksByID: [String: DjayTrack]

    public static let collectionNames = [
        "mediaItems", "mediaItemUserData", "mediaItemAnalyzedData", "mediaArtists", "mediaAlbums", "mediaGenres",
        "localMediaItemLocations", "globalMediaItemLocations", "mediaItemPlaylists", "mediaItemPlaylistItems",
        "queues", "queueItems", "historySessions", "historySessionItems",
    ]

    public static func load(databaseURL: URL) throws -> DjayLibrarySnapshot {
        let database = try DatabaseSnapshot(copying: databaseURL)
        var collections: [String: [CollectionRow]] = [:]
        for name in collectionNames {
            collections[name] = try database.collection(name)
        }
        return DjayLibrarySnapshot(collections: collections)
    }

    public static var defaultDatabaseURL: URL {
        FileManager.default.homeDirectoryForCurrentUser
            .appendingPathComponent("Music/djay/djay Media Library.djayMediaLibrary/MediaLibrary.db")
    }

    public func track(id: String) -> DjayTrack? {
        tracksByID[id]
    }

    public func playlist(id: String) -> DjayPlaylist? {
        playlists.first { $0.id == id }
    }

    public var rootPlaylist: DjayPlaylist? {
        playlist(id: Self.rootPlaylistID)
    }

    public func playlistTracks(_ playlist: DjayPlaylist) -> [DjayTrack] {
        playlist.trackIDs.compactMap { tracksByID[$0] }
    }

    init(collections: [String: [CollectionRow]]) {
        func rows(_ name: String) -> [CollectionRow] {
            collections[name] ?? []
        }
        func keyed(_ name: String) -> [String: DjayValue] {
            Dictionary(rows(name).map { ($0.key, $0.value) }, uniquingKeysWith: { first, _ in first })
        }
        func names(_ name: String) -> [String: String] {
            var result: [String: String] = [:]
            for row in rows(name) {
                result[row.key] = (row.value["name"] ?? row.value["value"])?.stringValue
            }
            return result
        }

        let nameTables = NameTables(
            artists: names("mediaArtists"), albums: names("mediaAlbums"), genres: names("mediaGenres")
        )
        let userData = keyed("mediaItemUserData"), analyzed = keyed("mediaItemAnalyzedData")
        let items = rows("mediaItems")

        var index = TrackIndex()
        for row in items {
            index.addTrack(row.key)
        }
        for row in items {
            for source in [row.value, userData[row.key], analyzed[row.key]].compactMap({ $0 }) {
                for titleID in Self.titleIDs(of: source) {
                    if let uuid = titleID["uuid"]?.stringValue {
                        index.addAlias(uuid, track: row.key)
                    }
                    if let isrc = titleID["isrc"]?.stringValue {
                        index.addISRC(isrc, track: row.key)
                    }
                }
            }
        }

        var locationURIs: [String: [String]] = [:]
        for name in ["localMediaItemLocations", "globalMediaItemLocations"] {
            for row in rows(name) {
                let uris = Self.uriList(row.value["sourceURIs"])
                guard !uris.isEmpty else { continue }
                let titleIDs = Self.titleIDs(of: row.value)
                let ids = [row.key] + titleIDs.compactMap { $0["uuid"]?.stringValue }
                if let target = ids.compactMap({ index.byAlias[$0] }).first {
                    for titleID in titleIDs {
                        if let isrc = titleID["isrc"]?.stringValue {
                            index.addISRC(isrc, track: target)
                        }
                    }
                }
                for id in ids {
                    if let track = index.byAlias[id] {
                        locationURIs[track, default: []].append(contentsOf: uris)
                    }
                }
            }
        }

        var built: [DjayTrack] = []
        for row in items {
            let extras = TrackExtras(
                user: userData[row.key], analyzed: analyzed[row.key], uris: locationURIs[row.key] ?? []
            )
            let track = Self.makeTrack(row: row, extras: extras, names: nameTables)
            built.append(track)
            if let isrc = row.value["titleID"]?["isrc"]?.stringValue {
                index.addISRC(isrc, track: row.key)
            }
            if let identity = TrackIdentity(title: track.title, artist: track.artist) {
                index.addIdentity(identity, track: row.key)
            }
        }
        tracks = built
        tracksByID = Dictionary(built.map { ($0.id, $0) }, uniquingKeysWith: { first, _ in first })
        playlists = Self.makePlaylists(
            playlists: rows("mediaItemPlaylists"), items: rows("mediaItemPlaylistItems"), index: index
        )
        queue = Self.makeQueue(queues: rows("queues"), items: rows("queueItems"), index: index, tracks: tracksByID)
        historySessions = Self.makeHistory(
            sessions: rows("historySessions"), plays: rows("historySessionItems"), index: index
        )
    }

    static func titleIDs(of value: DjayValue) -> [DjayValue] {
        var result: [DjayValue] = []
        if let single = value["titleID"], single.fields != nil {
            result.append(single)
        }
        for entry in value["titleIDs"]?.listValue ?? [] where entry.fields != nil {
            result.append(entry)
        }
        return result
    }

    static func uriList(_ value: DjayValue?) -> [String] {
        guard let value else { return [] }
        let entries: [DjayValue] = switch value {
        case .string: [value]
        case let .list(list): list
        case let .object(_, fields): fields["NS.objects"]?.listValue ?? []
        default: []
        }
        return entries.compactMap { entry in
            if let text = entry.stringValue {
                return text
            }
            guard let relative = entry["NS.relative"]?.stringValue else { return nil }
            return (entry["NS.base"]?.stringValue ?? "") + relative
        }
    }
}
