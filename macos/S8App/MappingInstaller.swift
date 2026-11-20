import Foundation

enum MappingError: LocalizedError, Equatable {
    case notBundled(S8Layout)
    case djayFolderMissing(String)

    var errorDescription: String? {
        switch self {
        case let .notBundled(layout):
            "The app has no \(layout.title) mapping. Rebuild it with mise run s8:app."
        case let .djayFolderMissing(path):
            "djay's mapping folder is missing: \(path). Open djay Pro once, then start again."
        }
    }
}

struct MappingInstaller {
    static let djayFolder = FileManager.default.homeDirectoryForCurrentUser
        .appendingPathComponent("Music/djay/MIDI Mappings", isDirectory: true)

    let source: URL
    let destination: URL

    func bundled() -> [URL] {
        let files = (try? FileManager.default.contentsOfDirectory(
            at: source, includingPropertiesForKeys: nil
        )) ?? []
        return files.filter { $0.pathExtension == "djayMidiMapping" }
            .sorted { $0.lastPathComponent < $1.lastPathComponent }
    }

    func mappingName(for layout: S8Layout) throws -> String {
        let names = bundled().map { $0.deletingPathExtension().lastPathComponent }
        let matching = names.filter { name in
            guard name.hasPrefix(layout.mappingPrefix) else { return false }
            return Int(name.dropFirst(layout.mappingPrefix.count)) != nil
        }
        guard let name = matching.max(by: { revision($0, layout) < revision($1, layout) }) else {
            throw MappingError.notBundled(layout)
        }
        return name
    }

    /// Copies every bundled mapping that djay's folder lacks and never overwrites a file.
    func installMissing() throws -> [String] {
        var isFolder: ObjCBool = false
        guard FileManager.default.fileExists(atPath: destination.path, isDirectory: &isFolder),
              isFolder.boolValue
        else {
            throw MappingError.djayFolderMissing(destination.path)
        }
        var installed: [String] = []
        for file in bundled() {
            let target = destination.appendingPathComponent(file.lastPathComponent)
            guard !FileManager.default.fileExists(atPath: target.path) else { continue }
            try FileManager.default.copyItem(at: file, to: target)
            installed.append(file.deletingPathExtension().lastPathComponent)
        }
        return installed
    }

    private func revision(_ name: String, _ layout: S8Layout) -> Int {
        Int(name.dropFirst(layout.mappingPrefix.count)) ?? 0
    }
}
