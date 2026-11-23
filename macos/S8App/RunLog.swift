import Foundation

/// One log file per run under ~/Library/Logs/Kontrol S8 Bridge, newest 30 kept.
final class RunLog: @unchecked Sendable {
    static let folder = FileManager.default.homeDirectoryForCurrentUser
        .appendingPathComponent("Library/Logs/Kontrol S8 Bridge", isDirectory: true)
    static let kept = 30

    let url: URL
    private let handle: FileHandle?
    private let lock = NSLock()
    private let stamp: ISO8601DateFormatter

    init(folder: URL = RunLog.folder, now: Date = Date()) {
        try? FileManager.default.createDirectory(at: folder, withIntermediateDirectories: true)
        let name = ISO8601DateFormatter.string(
            from: now, timeZone: TimeZone(identifier: "UTC") ?? .current,
            formatOptions: [.withYear, .withMonth, .withDay, .withTime, .withTimeZone]
        )
        url = folder.appendingPathComponent("s8-\(name).log")
        FileManager.default.createFile(atPath: url.path, contents: nil)
        handle = try? FileHandle(forWritingTo: url)
        stamp = ISO8601DateFormatter()
        stamp.formatOptions = [.withInternetDateTime, .withFractionalSeconds]
        Self.prune(folder)
    }

    deinit {
        try? handle?.close()
    }

    func write(_ line: String) {
        lock.lock()
        defer { lock.unlock() }
        handle?.write(Data("\(stamp.string(from: Date())) \(line)\n".utf8))
    }

    static func prune(_ folder: URL) {
        let logs = ((try? FileManager.default.contentsOfDirectory(at: folder, includingPropertiesForKeys: nil)) ?? [])
            .filter { $0.lastPathComponent.hasPrefix("s8-") && $0.pathExtension == "log" }
            .sorted { $0.lastPathComponent > $1.lastPathComponent }
        for old in logs.dropFirst(kept) {
            try? FileManager.default.removeItem(at: old)
        }
    }
}
