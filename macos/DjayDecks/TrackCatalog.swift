import DjayLibrary
import Foundation

public struct CatalogEntry {
    public let trackID: String
    public let duration: Double?
    public let cues: [DjayCuePoint]
    public let analysis: TrackAnalysis?

    public init(trackID: String, duration: Double? = nil, cues: [DjayCuePoint] = [], analysis: TrackAnalysis? = nil) {
        self.trackID = trackID
        self.duration = duration
        self.cues = cues
        self.analysis = analysis
    }
}

public protocol TrackCatalog {
    func entry(title: String, artist: String?, length: TrackLengthHint?) -> CatalogEntry?
}

public struct AnalysisStore {
    public static var defaultRoot: URL {
        let support = "Library/Group Containers/VJXTL73S8G.com.algoriddim.userdata/Library/Application Support"
        return FileManager.default.homeDirectoryForCurrentUser
            .appendingPathComponent("\(support)/Algoriddim/Metadata")
    }

    public let root: URL

    public init(root: URL = AnalysisStore.defaultRoot) {
        self.root = root
    }

    public func url(for trackID: String) -> URL? {
        let manager = FileManager.default
        let name = trackID + ".djayMetadata"
        let shards = (try? manager.contentsOfDirectory(at: root, includingPropertiesForKeys: nil)) ?? []
        return shards.map { $0.appendingPathComponent(name) }.first { manager.fileExists(atPath: $0.path) }
    }

    public func analysis(for trackID: String) -> TrackAnalysis? {
        url(for: trackID).flatMap { try? TrackAnalysis.load(url: $0) }
    }
}

struct DatabaseSignature: Equatable {
    struct File: Equatable {
        let modified: Date?
        let size: Int?
    }

    let database: File
    let journal: File

    init(databaseURL: URL) {
        database = Self.stat(databaseURL)
        journal = Self.stat(URL(fileURLWithPath: databaseURL.path + "-wal"))
    }

    private static func stat(_ url: URL) -> File {
        let attributes = try? FileManager.default.attributesOfItem(atPath: url.path)
        return File(
            modified: attributes?[.modificationDate] as? Date, size: (attributes?[.size] as? NSNumber)?.intValue
        )
    }
}

public final class LibraryCatalog: TrackCatalog {
    public typealias Loader = (URL) throws -> DjayLibrarySnapshot

    private let databaseURL: URL
    private let store: AnalysisStore
    private let checkInterval: TimeInterval
    private let loader: Loader
    private let now: () -> Date
    private let didReload: (Bool) -> Void
    private let queue = DispatchQueue(label: "LibraryCatalog.reload")
    private let lock = NSLock()
    private var snapshot: DjayLibrarySnapshot?
    private var loadedSignature: DatabaseSignature?
    private var lastCheck: Date?
    private var reloading = false
    private var analyses: [String: TrackAnalysis] = [:]

    public init(
        databaseURL: URL = DjayLibrarySnapshot.defaultDatabaseURL, store: AnalysisStore = AnalysisStore(),
        checkInterval: TimeInterval = 5, now: @escaping () -> Date = Date.init,
        loader: @escaping Loader = { try DjayLibrarySnapshot.load(databaseURL: $0) },
        didReload: @escaping (Bool) -> Void = { _ in }
    ) {
        self.databaseURL = databaseURL
        self.store = store
        self.checkInterval = checkInterval
        self.now = now
        self.loader = loader
        self.didReload = didReload
    }

    public func entry(title: String, artist: String?, length: TrackLengthHint?) -> CatalogEntry? {
        let current = currentSnapshot()
        guard let current, let matches = candidates(in: current, title: title, artist: artist, length: length),
              let first = matches.first else { return nil }
        var track = first
        var analysis: TrackAnalysis?
        var found = false
        for match in matches where !found {
            if let cached = cachedAnalysis(match.id) {
                (track, analysis, found) = (match, cached, true)
            } else if let loaded = store.analysis(for: match.id) {
                remember(loaded, for: match.id)
                (track, analysis, found) = (match, loaded, true)
            }
        }
        return CatalogEntry(
            trackID: track.id, duration: track.duration, cues: current.cuePoints(trackID: track.id),
            analysis: analysis
        )
    }

    private func candidates(
        in snapshot: DjayLibrarySnapshot, title: String, artist: String?, length: TrackLengthHint?
    ) -> [DjayTrack]? {
        let matches = snapshot.tracks(title: title, artist: artist)
        var tracks = matches.tracks
        if let length {
            let fitting = tracks.filter { $0.duration.map(length.admits) ?? true }
            if !fitting.isEmpty {
                tracks = fitting
            } else if !matches.artistMatched {
                return nil
            }
        }
        if !matches.artistMatched {
            let durations = tracks.compactMap(\.duration)
            let spread = (durations.max() ?? 0) - (durations.min() ?? 0)
            if spread > TrackLengthHint.tolerance {
                return nil
            }
        }
        return tracks.isEmpty ? nil : tracks
    }

    private func cachedAnalysis(_ id: String) -> TrackAnalysis? {
        lock.lock()
        defer { lock.unlock() }
        return analyses[id]
    }

    private func remember(_ analysis: TrackAnalysis, for id: String) {
        lock.lock()
        analyses[id] = analysis
        lock.unlock()
    }

    private func currentSnapshot() -> DjayLibrarySnapshot? {
        let time = now()
        lock.lock()
        let first = lastCheck == nil
        if first || (!reloading && time.timeIntervalSince(lastCheck ?? time) >= checkInterval) {
            lastCheck = time
            let signature = DatabaseSignature(databaseURL: databaseURL)
            if first || signature != loadedSignature {
                reloading = true
                lock.unlock()
                if first {
                    queue.sync { reload(signature) }
                } else {
                    queue.async { self.reload(signature) }
                }
                lock.lock()
            }
        }
        defer { lock.unlock() }
        return snapshot
    }

    private func reload(_ signature: DatabaseSignature) {
        let loaded = try? loader(databaseURL)
        lock.lock()
        if let loaded {
            snapshot = loaded
            loadedSignature = signature
        }
        reloading = false
        lock.unlock()
        didReload(loaded != nil)
    }
}
