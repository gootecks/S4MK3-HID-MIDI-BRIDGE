@testable import DjayLibrary
import Foundation
import SQLite3

struct FixtureRow: Decodable {
    let rowid: Int64
    let key: String
    let data: String
}

enum Fixtures {
    static func url(_ name: String) throws -> URL {
        guard let url = Bundle.module.url(forResource: name, withExtension: "json", subdirectory: "Fixtures") else {
            throw DjayDecodeError.database("missing fixture \(name)")
        }
        return url
    }

    static func realLibrary() throws -> [String: [FixtureRow]] {
        try JSONDecoder().decode([String: [FixtureRow]].self, from: Data(contentsOf: url("real-library")))
    }

    static func collections(_ library: [String: [FixtureRow]]) throws -> [String: [CollectionRow]] {
        try library.mapValues { rows in
            try rows.map { row in
                let blob = Data(base64Encoded: row.data) ?? Data()
                guard let value = try DjayBlob.decode(blob) else { throw DjayDecodeError.unknownBlob }
                return CollectionRow(rowid: row.rowid, key: row.key, value: value)
            }
        }
    }

    static func writeDatabase(_ library: [String: [FixtureRow]], to url: URL) throws {
        var handle: OpaquePointer?
        guard sqlite3_open(url.path, &handle) == SQLITE_OK else { throw DjayDecodeError.database("create failed") }
        defer { sqlite3_close(handle) }
        sqlite3_exec(
            handle, "CREATE TABLE database2 (rowid INTEGER PRIMARY KEY, collection TEXT, key TEXT, data BLOB)",
            nil, nil, nil
        )
        var statement: OpaquePointer?
        sqlite3_prepare_v2(
            handle, "INSERT INTO database2 (rowid, collection, key, data) VALUES (?, ?, ?, ?)", -1, &statement, nil
        )
        defer { sqlite3_finalize(statement) }
        let transient = unsafeBitCast(-1, to: sqlite3_destructor_type.self)
        for (collection, rows) in library {
            for row in rows {
                let blob = Data(base64Encoded: row.data)!
                sqlite3_reset(statement)
                sqlite3_bind_int64(statement, 1, row.rowid)
                sqlite3_bind_text(statement, 2, collection, -1, transient)
                sqlite3_bind_text(statement, 3, row.key, -1, transient)
                _ = blob.withUnsafeBytes {
                    sqlite3_bind_blob(statement, 4, $0.baseAddress, Int32(blob.count), transient)
                }
                sqlite3_step(statement)
            }
        }
    }
}

func djayObject(_ fields: [String: DjayValue]) -> DjayValue {
    .object(className: nil, fields: fields)
}

func djayStrings(_ values: [String]) -> DjayValue {
    .list(values.map { .string($0) })
}

func row(_ key: String, _ value: DjayValue, rowid: Int64 = 0) -> CollectionRow {
    CollectionRow(rowid: rowid, key: key, value: value)
}
