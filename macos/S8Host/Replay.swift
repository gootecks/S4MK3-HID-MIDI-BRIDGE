import Foundation
import S8Views

enum ReplayError: Error, CustomStringConvertible {
    case noSnapshots(String)

    var description: String {
        switch self {
        case let .noSnapshots(directory): "no snapshot .json files in \(directory)"
        }
    }
}

enum Replay {
    static func run(snapshots: String, records: String) throws {
        let directory = URL(fileURLWithPath: snapshots)
        let output = URL(fileURLWithPath: records)
        let files = try FileManager.default.contentsOfDirectory(at: directory, includingPropertiesForKeys: nil)
            .filter { $0.pathExtension == "json" }
            .sorted { $0.lastPathComponent < $1.lastPathComponent }
        guard !files.isEmpty else { throw ReplayError.noSnapshots(snapshots) }
        try FileManager.default.createDirectory(at: output, withIntermediateDirectories: true)
        var panels = LivePanels()
        var feed = PanelFeed()
        for file in files {
            let state = try panels.state(for: SidebarSnapshot.load(file))
            let deckFile = directory.appendingPathComponent("decks").appendingPathComponent(file.lastPathComponent)
            let deck = FileManager.default.fileExists(atPath: deckFile.path) ? try DeckSnapshot.load(deckFile) : nil
            let sent = feed.records(state: state, deck: deck)
            let name = file.deletingPathExtension().lastPathComponent + ".s8fr"
            try sent.reduce(into: Data()) { $0.append($1) }.write(to: output.appendingPathComponent(name))
            print("\(name): \(sent.count) record(s)")
        }
    }
}
