import Foundation

enum KeyedArchive {
    private indirect enum Node {
        case uid(Int)
        case scalar(DjayValue)
        case array([Node])
        case dict([(String, Node)])
    }

    static func decode(_ blob: Data) throws -> DjayValue? {
        let reader = try PlistReader(blob)
        guard case let .dict(top) = try reader.object(at: reader.topObject),
              let objectsNode = top.first(where: { $0.0 == "$objects" })?.1,
              case let .array(objects) = objectsNode, objects.count > 1
        else { throw DjayDecodeError.malformed("no $objects in archive") }
        return resolve(objects[1], objects, 0)
    }

    private static func resolve(_ node: Node, _ objects: [Node], _ depth: Int) -> DjayValue? {
        if depth > 100 {
            return .string("<max depth>")
        }
        switch node {
        case let .uid(index):
            guard objects.indices.contains(index) else { return .string("<uid:\(index)>") }
            return resolve(objects[index], objects, depth + 1)
        case let .array(items):
            return .list(items.map { resolve($0, objects, depth + 1) ?? .null })
        case let .scalar(value):
            if case .string("$null") = value {
                return nil
            }
            return value
        case let .dict(entries):
            var resolved: [String: DjayValue] = [:]
            for (key, child) in entries where key != "$class" {
                resolved[key] = resolve(child, objects, depth + 1) ?? .null
            }
            if case let .double(seconds)? = resolved["NS.time"] {
                return .date(Date(timeIntervalSinceReferenceDate: seconds))
            }
            let indexed = resolved.keys.compactMap { key -> (Int, String)? in
                let prefix = "NS.object."
                guard key.hasPrefix(prefix), let index = Int(key.dropFirst(prefix.count)) else { return nil }
                return (index, key)
            }.sorted { $0.0 < $1.0 }
            if !indexed.isEmpty {
                return .list(indexed.map { resolved[$0.1] ?? .null })
            }
            if resolved.count == 1, resolved["NS.empty"] != nil {
                return nil
            }
            if resolved.count == 1, case let .list(items)? = resolved["NS.objects"] {
                return .list(items)
            }
            return .object(className: nil, fields: resolved)
        }
    }

    private struct PlistReader {
        let bytes: [UInt8]
        let offsetSize: Int
        let refSize: Int
        let offsets: [Int]
        let topObject: Int

        init(_ blob: Data) throws {
            let bytes = [UInt8](blob)
            self.bytes = bytes
            guard bytes.count > 40, blob.starts(with: Array("bplist00".utf8)) else {
                throw DjayDecodeError.malformed("not a bplist")
            }
            let trailer = bytes.count - 32
            let offsetSize = Int(bytes[trailer + 6])
            let refSize = Int(bytes[trailer + 7])
            self.offsetSize = offsetSize
            self.refSize = refSize
            let count = Self.unsigned(bytes, trailer + 8, 8)
            topObject = Self.unsigned(bytes, trailer + 16, 8)
            let tableStart = Self.unsigned(bytes, trailer + 24, 8)
            guard offsetSize > 0, refSize > 0, count > 0, tableStart + count * offsetSize <= trailer else {
                throw DjayDecodeError.malformed("bad bplist trailer")
            }
            offsets = (0 ..< count).map { Self.unsigned(bytes, tableStart + $0 * offsetSize, offsetSize) }
        }

        static func unsigned(_ bytes: [UInt8], _ start: Int, _ size: Int) -> Int {
            var value = 0
            for index in 0 ..< size where start + index < bytes.count {
                value = value << 8 | Int(bytes[start + index])
            }
            return value
        }

        func length(_ position: inout Int, _ nibble: Int) -> Int {
            guard nibble == 0xF else { return nibble }
            let size = 1 << Int(bytes[position] & 0x0F)
            let value = Self.unsigned(bytes, position + 1, size)
            position += 1 + size
            return value
        }

        func object(at index: Int) throws -> Node {
            guard offsets.indices.contains(index) else { throw DjayDecodeError.malformed("bad object ref \(index)") }
            var pos = offsets[index]
            guard pos < bytes.count else { throw DjayDecodeError.malformed("bad object offset") }
            let marker = bytes[pos]
            let nibble = Int(marker & 0x0F)
            pos += 1
            switch marker >> 4 {
            case 0x0:
                switch marker {
                case 0x08: return .scalar(.bool(false))
                case 0x09: return .scalar(.bool(true))
                default: return .scalar(.null)
                }
            case 0x1:
                let size = 1 << nibble
                let raw = Self.unsigned(bytes, pos, min(size, 8))
                return .scalar(.int(size == 8 ? Int64(bitPattern: UInt64(raw)) : Int64(raw)))
            case 0x2:
                let size = 1 << nibble
                let raw = Self.unsigned(bytes, pos, size)
                let value = size == 4 ? Double(Float(bitPattern: UInt32(raw))) : Double(bitPattern: UInt64(raw))
                return .scalar(.double(value))
            case 0x3:
                let seconds = Double(bitPattern: UInt64(Self.unsigned(bytes, pos, 8)))
                return .scalar(.date(Date(timeIntervalSinceReferenceDate: seconds)))
            case 0x4:
                let count = length(&pos, nibble)
                return .scalar(.data(Data(bytes[pos ..< min(pos + count, bytes.count)])))
            case 0x5:
                let count = length(&pos, nibble)
                let text = String(bytes: bytes[pos ..< min(pos + count, bytes.count)], encoding: .utf8)
                return .scalar(.string(text ?? ""))
            case 0x6:
                let count = length(&pos, nibble)
                let end = min(pos + count * 2, bytes.count)
                let units = stride(from: pos, to: end - 1, by: 2).map { UInt16(bytes[$0]) << 8 | UInt16(bytes[$0 + 1]) }
                return .scalar(.string(String(decoding: units, as: UTF16.self)))
            case 0x8:
                return .uid(Self.unsigned(bytes, pos, nibble + 1))
            case 0xA, 0xC:
                let count = length(&pos, nibble)
                return try .array((0 ..< count).map {
                    try object(at: Self.unsigned(bytes, pos + $0 * refSize, refSize))
                })
            case 0xD:
                let count = length(&pos, nibble)
                var entries: [(String, Node)] = []
                for item in 0 ..< count {
                    let keyRef = Self.unsigned(bytes, pos + item * refSize, refSize)
                    let valueRef = Self.unsigned(bytes, pos + (count + item) * refSize, refSize)
                    guard case let .scalar(.string(key)) = try object(at: keyRef) else { continue }
                    try entries.append((key, object(at: valueRef)))
                }
                return .dict(entries)
            default:
                throw DjayDecodeError.malformed("unsupported bplist marker 0x\(String(marker, radix: 16))")
            }
        }
    }
}
