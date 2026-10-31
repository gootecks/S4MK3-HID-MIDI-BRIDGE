import Foundation
import SQLite3

struct CollectionRow {
    let rowid: Int64
    let key: String
    let value: DjayValue
}

final class DatabaseSnapshot {
    private let directory: URL
    private var handle: OpaquePointer?

    init(copying source: URL) throws {
        let manager = FileManager.default
        guard manager.fileExists(atPath: source.path) else {
            throw DjayDecodeError.database("not found: \(source.path)")
        }
        directory = manager.temporaryDirectory
            .appendingPathComponent("djay-snap-\(UUID().uuidString)", isDirectory: true)
        try manager.createDirectory(at: directory, withIntermediateDirectories: true)
        let name = source.lastPathComponent
        do {
            for suffix in ["", "-wal", "-shm"] {
                let side = source.deletingLastPathComponent().appendingPathComponent(name + suffix)
                if manager.fileExists(atPath: side.path) {
                    try manager.copyItem(at: side, to: directory.appendingPathComponent(name + suffix))
                }
            }
        } catch {
            try? manager.removeItem(at: directory)
            throw error
        }
        let copy = directory.appendingPathComponent(name)
        guard sqlite3_open_v2(copy.path, &handle, SQLITE_OPEN_READONLY, nil) == SQLITE_OK else {
            let reason = handle.map { String(cString: sqlite3_errmsg($0)) } ?? "open failed"
            sqlite3_close(handle)
            try? manager.removeItem(at: directory)
            throw DjayDecodeError.database(reason)
        }
    }

    deinit {
        sqlite3_close(handle)
        try? FileManager.default.removeItem(at: directory)
    }

    private func prepare(_ sql: String) throws -> OpaquePointer {
        var statement: OpaquePointer?
        guard sqlite3_prepare_v2(handle, sql, -1, &statement, nil) == SQLITE_OK, let statement else {
            throw DjayDecodeError.database(String(cString: sqlite3_errmsg(handle)))
        }
        return statement
    }

    func collection(_ name: String) throws -> [CollectionRow] {
        let statement = try prepare("SELECT rowid, key, data FROM database2 WHERE collection = ? ORDER BY rowid")
        defer { sqlite3_finalize(statement) }
        sqlite3_bind_text(statement, 1, name, -1, unsafeBitCast(-1, to: sqlite3_destructor_type.self))
        var rows: [CollectionRow] = []
        while sqlite3_step(statement) == SQLITE_ROW {
            guard let keyText = sqlite3_column_text(statement, 1) else { continue }
            let count = Int(sqlite3_column_bytes(statement, 2))
            let blob = count > 0 && sqlite3_column_blob(statement, 2) != nil
                ? Data(bytes: sqlite3_column_blob(statement, 2), count: count) : Data()
            guard let value = try? DjayBlob.decode(blob), value.fields != nil else { continue }
            rows.append(CollectionRow(
                rowid: sqlite3_column_int64(statement, 0), key: String(cString: keyText), value: value
            ))
        }
        return rows
    }
}
