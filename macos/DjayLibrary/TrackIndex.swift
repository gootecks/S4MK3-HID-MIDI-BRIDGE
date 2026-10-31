import Foundation

struct TrackIdentity: Hashable {
    let title: String
    let artist: String

    init?(title: String?, artist: String?) {
        let normalizedTitle = TrackIdentity.normalize(title)
        guard !normalizedTitle.isEmpty else { return nil }
        self.title = normalizedTitle
        self.artist = TrackIdentity.normalize(artist)
    }

    static func normalize(_ text: String?) -> String {
        var output = ""
        var pendingSpace = false
        for scalar in (text ?? "").decomposedStringWithCompatibilityMapping.lowercased().unicodeScalars {
            if scalar.properties.canonicalCombiningClass != .notReordered {
                continue
            }
            let value = scalar.value
            let isAlnum = (0x61 ... 0x7A).contains(value) || (0x30 ... 0x39).contains(value)
            if isAlnum {
                if pendingSpace, !output.isEmpty {
                    output.append(" ")
                }
                pendingSpace = false
                output.unicodeScalars.append(scalar)
            } else {
                pendingSpace = true
            }
        }
        return output
    }
}

struct TrackIndex {
    private(set) var byAlias: [String: String] = [:]
    private(set) var byISRC: [String: String] = [:]
    private(set) var byIdentity: [TrackIdentity: String] = [:]

    mutating func addTrack(_ id: String) {
        byAlias[id] = id
    }

    mutating func addAlias(_ alias: String, track: String) {
        if byAlias[alias] == nil {
            byAlias[alias] = track
        }
    }

    mutating func addISRC(_ isrc: String, track: String) {
        if byISRC[isrc] == nil {
            byISRC[isrc] = track
        }
    }

    mutating func addIdentity(_ identity: TrackIdentity, track: String) {
        if byIdentity[identity] == nil {
            byIdentity[identity] = track
        }
    }

    func resolve(titleID: String?, isrc: String?, title: String?, artist: String?) -> String? {
        if let titleID, let id = byAlias[titleID] {
            return id
        }
        if let isrc, let id = byISRC[isrc] {
            return id
        }
        if let identity = TrackIdentity(title: title, artist: artist) {
            return byIdentity[identity]
        }
        return nil
    }
}
