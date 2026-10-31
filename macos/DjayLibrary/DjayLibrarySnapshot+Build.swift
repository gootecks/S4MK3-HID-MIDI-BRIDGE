import Foundation

extension DjayLibrarySnapshot {
    static let camelot = [
        "8B", "8A", "3B", "3A", "10B", "10A", "5B", "5A", "12B", "12A", "7B", "7A",
        "2B", "2A", "9B", "9A", "4B", "4A", "11B", "11A", "6B", "6A", "1B", "1A"
    ]

    private static func nonZero(_ value: DjayValue?) -> Double? {
        guard let number = value?.numberValue, number != 0 else { return nil }
        return number
    }

    private static func joined(_ uuids: [String], _ names: [String: String]) -> String? {
        let text = uuids.compactMap { names[$0] }.joined(separator: ", ")
        return text.isEmpty ? nil : text
    }

    struct NameTables {
        let artists: [String: String]
        let albums: [String: String]
        let genres: [String: String]
    }

    struct TrackExtras {
        let user: DjayValue?
        let analyzed: DjayValue?
        let uris: [String]
    }

    static func makeTrack(row: CollectionRow, extras: TrackExtras, names: NameTables) -> DjayTrack {
        let item = row.value
        let (user, analyzed, uris) = (extras.user, extras.analyzed, extras.uris)
        let (artistNames, albumNames, genreNames) = (names.artists, names.albums, names.genres)
        let titleID = item["titleID"]
        let bpm = nonZero(user?["manualBPM"]) ?? nonZero(analyzed?["bpm"]) ?? nonZero(item["bpm"])
        let keyCandidates = [
            user?["manualKeySignatureIndex2"], analyzed?["keySignatureIndex"], item["musicalKeySignatureIndex"]
        ]
        let keyIndex = keyCandidates
            .compactMap { $0?.intValue }.first.flatMap { camelot.indices.contains($0) ? $0 : nil }
        let local = uris.first { $0.hasPrefix("file:") }
        let sourceURI = local ?? uris.first
        return DjayTrack(
            id: row.key,
            title: item["title"]?.stringValue ?? titleID?["title"]?.stringValue,
            artist: joined(item["artistUUIDs"]?.stringList ?? [], artistNames) ?? titleID?["artist"]?.stringValue,
            album: item["albumUUID"]?.stringValue.flatMap { albumNames[$0] },
            genre: joined(item["genreUUIDs"]?.stringList ?? [], genreNames),
            bpm: bpm.map { ($0 * 100).rounded(.toNearestOrEven) / 100 },
            keyIndex: keyIndex,
            key: keyIndex.map { camelot[$0] },
            duration: nonZero(item["duration"]) ?? nonZero(titleID?["duration"]),
            path: local.map { filePath($0) } ?? sourceURI,
            sourceURI: sourceURI,
            originSource: item["originSourceID"]?.stringValue,
            addedDate: item["addedDate"]?.dateValue
        )
    }

    private static func filePath(_ uri: String) -> String {
        if let url = URL(string: uri), url.isFileURL {
            return url.path
        }
        return uri.removingPercentEncoding ?? uri
    }

    static func makePlaylists(
        playlists rows: [CollectionRow], items: [CollectionRow], index: TrackIndex
    ) -> [DjayPlaylist] {
        var itemTrack: [String: String] = [:]
        var itemsByPlaylist: [String: [String]] = [:]
        for item in items {
            guard let mediaID = item.value["mediaItemUUID"]?.stringValue,
                  let track = index.byAlias[mediaID] else { continue }
            itemTrack[item.key] = track
            if let playlistID = item.value["playlistUUID"]?.stringValue {
                itemsByPlaylist[playlistID, default: []].append(track)
            }
        }
        return rows.map { row in
            let kind: DjayPlaylistKind = if row.key == rootPlaylistID {
                .folder
            } else {
                switch row.value["type"]?.intValue {
                case nil: .playlist
                case 1: .folder
                case 2: .smart
                case let other?: .unknown(other)
                }
            }
            let trackIDs: [String] = if let ordered = row.value["itemUUIDs"]?.listValue {
                ordered.compactMap(\.stringValue).compactMap { itemTrack[$0] }
            } else {
                itemsByPlaylist[row.key] ?? []
            }
            return DjayPlaylist(
                id: row.key, name: row.value["name"]?.stringValue, kind: kind,
                parentID: row.value["parentUUID"]?.stringValue,
                childIDs: row.value["childUUIDs"]?.stringList ?? [], trackIDs: trackIDs
            )
        }
    }

    static func makeQueue(
        queues: [CollectionRow], items: [CollectionRow], index: TrackIndex, tracks: [String: DjayTrack]
    ) -> [DjayQueueEntry] {
        guard let queueRow = queues.first(where: { $0.key == defaultQueueID }) ?? queues.first else { return [] }
        let byKey = Dictionary(items.map { ($0.key, $0) }, uniquingKeysWith: { first, _ in first })
        let listKey = queueRow.value["itemUUIDs"] ?? queueRow.value["queueItemUUIDs"]
        let orderedKeys: [String] = if let listed = listKey?.listValue {
            listed.compactMap(\.stringValue)
        } else {
            items.filter { $0.value["queueUUID"]?.stringValue == queueRow.key }.map(\.key)
        }
        return orderedKeys.compactMap { key in
            guard let item = byKey[key] else { return nil }
            let titleID = item.value["titleID"]
            let trackID = index.resolve(
                titleID: titleID?["uuid"]?.stringValue ?? item.value["mediaItemUUID"]?.stringValue,
                isrc: titleID?["isrc"]?.stringValue, title: titleID?["title"]?.stringValue,
                artist: titleID?["artist"]?.stringValue
            )
            let track = trackID.flatMap { tracks[$0] }
            return DjayQueueEntry(
                id: key, trackID: trackID,
                title: titleID?["title"]?.stringValue ?? track?.title,
                artist: titleID?["artist"]?.stringValue ?? track?.artist
            )
        }
    }

    static func makeHistory(
        sessions rows: [CollectionRow], plays: [CollectionRow], index: TrackIndex
    ) -> [DjayHistorySession] {
        let sessionIDs = Set(rows.map(\.key))
        var sessionOfItem: [String: String] = [:]
        for row in rows {
            for item in row.value["itemUUIDs"]?.stringList ?? [] {
                sessionOfItem[item] = row.key
            }
        }
        var grouped: [String: [(Date, CollectionRow)]] = [:]
        for play in plays {
            guard let start = play.value["startTime"]?.dateValue,
                  let session = play.value["sessionUUID"]?.stringValue ?? sessionOfItem[play.key],
                  sessionIDs.contains(session) else { continue }
            grouped[session, default: []].append((start, play))
        }
        let built = rows.map { row -> DjayHistorySession in
            let ordered = (grouped[row.key] ?? []).enumerated()
                .sorted { ($0.element.0, $0.offset) < ($1.element.0, $1.offset) }.map(\.element)
            let items = ordered.enumerated().map { position, entry -> DjayHistoryPlay in
                let titleID = entry.1.value["titleID"]
                let uuid = titleID?["uuid"]?.stringValue, isrc = titleID?["isrc"]?.stringValue
                let title = titleID?["title"]?.stringValue, artist = titleID?["artist"]?.stringValue
                return DjayHistoryPlay(
                    id: entry.1.key, sessionID: row.key, index: position, startTime: entry.0,
                    deck: entry.1.value["deckNumber"]?.intValue, title: title, artist: artist, isrc: isrc,
                    titleID: uuid,
                    duration: titleID?["duration"]?.numberValue,
                    trackID: index.resolve(titleID: uuid, isrc: isrc, title: title, artist: artist)
                )
            }
            return DjayHistorySession(
                id: row.key, deviceName: row.value["deviceName"]?.stringValue,
                startDate: row.value["startDate"]?.dateValue, endDate: row.value["endDate"]?.dateValue, plays: items
            )
        }
        return built.enumerated().sorted { lhs, rhs in
            let left = lhs.element.startDate ?? .distantPast, right = rhs.element.startDate ?? .distantPast
            return left == right ? lhs.offset < rhs.offset : left < right
        }.map(\.element)
    }
}
