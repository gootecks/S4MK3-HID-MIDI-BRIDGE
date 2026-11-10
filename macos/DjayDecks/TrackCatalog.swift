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
    func entry(title: String, artist: String?) -> CatalogEntry?
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

public final class LibraryCatalog: TrackCatalog {
    private let databaseURL: URL
    private let store: AnalysisStore
    private let reloadInterval: TimeInterval
    private var snapshot: DjayLibrarySnapshot?
    private var loadedAt = Date.distantPast
    private var analyses: [String: TrackAnalysis] = [:]
    private let lock = NSLock()

    public init(
        databaseURL: URL = DjayLibrarySnapshot.defaultDatabaseURL, store: AnalysisStore = AnalysisStore(),
        reloadInterval: TimeInterval = 30
    ) {
        self.databaseURL = databaseURL
        self.store = store
        self.reloadInterval = reloadInterval
    }

    public func entry(title: String, artist: String?) -> CatalogEntry? {
        lock.lock()
        defer { lock.unlock() }
        var matches = snapshot?.tracks(title: title, artist: artist) ?? []
        if matches.isEmpty, Date().timeIntervalSince(loadedAt) >= reloadInterval {
            loadedAt = Date()
            snapshot = try? DjayLibrarySnapshot.load(databaseURL: databaseURL)
            matches = snapshot?.tracks(title: title, artist: artist) ?? []
        }
        guard let snapshot, let track = matches.first(where: { store.url(for: $0.id) != nil }) ?? matches.first else {
            return nil
        }
        if analyses[track.id] == nil {
            analyses[track.id] = store.analysis(for: track.id)
        }
        return CatalogEntry(
            trackID: track.id, duration: track.duration, cues: snapshot.cuePoints(trackID: track.id),
            analysis: analyses[track.id]
        )
    }
}
