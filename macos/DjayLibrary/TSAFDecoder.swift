import Foundation

struct TSAFDecoder {
    private static let maxDepth = 64
    private let data: [UInt8]
    private var pos = 20
    private var strings: [String] = []
    private var depth = 0

    init(_ blob: Data) throws {
        guard blob.count >= 20, blob.starts(with: Array("TSAF".utf8)) else { throw DjayDecodeError.notTSAF }
        data = [UInt8](blob)
    }

    mutating func decode() throws -> DjayValue {
        let value = try parseValue()
        guard case .object = value else { throw DjayDecodeError.malformed("top-level value is not an object") }
        return value
    }

    private func fail(_ message: String) -> DjayDecodeError {
        .malformed(message)
    }

    private func align(_ offset: Int, _ size: Int) -> Int {
        offset + ((size - offset % size) % size)
    }

    private func cString(at start: Int) throws -> (String, Int) {
        guard start < data.count, let end = data[start...].firstIndex(of: 0) else {
            throw fail("unterminated string at \(start)")
        }
        return (String(bytes: data[start ..< end], encoding: .utf8) ?? "", end + 1)
    }

    private mutating func readString() throws -> String {
        let (text, next) = try cString(at: pos)
        pos = next
        strings.append(text)
        return text
    }

    private mutating func fixed<T: FixedWidthInteger>(_: T.Type, _ tag: UInt8) throws -> T {
        let size = MemoryLayout<T>.size
        pos = align(pos, size)
        guard pos + size <= data.count else { throw fail("truncated tag 0x\(String(tag, radix: 16)) at \(pos)") }
        var value: T = 0
        for index in 0 ..< size {
            value |= T(truncatingIfNeeded: data[pos + index]) << (8 * index)
        }
        pos += size
        return value
    }

    private mutating func byte(_ what: String) throws -> UInt8 {
        guard pos < data.count else { throw fail("truncated \(what)") }
        defer { pos += 1 }
        return data[pos]
    }

    private func stringRef(_ index: UInt8) throws -> String {
        guard Int(index) < strings.count else { throw fail("string ref \(index) out of range") }
        return strings[Int(index)]
    }

    private mutating func parseValue() throws -> DjayValue {
        guard pos < data.count else { throw fail("unexpected end of data") }
        let tag = data[pos]
        pos += 1
        switch tag {
        case 0x2B: return try parseObject()
        case 0x08: return try .string(readString())
        case 0x21:
            if pos < data.count, data[pos] == 0x08 {
                pos += 1
            }
            let text = try readString()
            if pos < data.count, data[pos] == 0x00 {
                pos += 1
            }
            return .string(text)
        case 0x05: return try .string(stringRef(byte("0x05 ref")))
        case 0x0F, 0x02: return try .int(Int64(byte("uint8")))
        case 0x10: return try .int(Int64(fixed(UInt16.self, tag)))
        case 0x11: return try .int(Int64(Int32(bitPattern: fixed(UInt32.self, tag))))
        case 0x12: return try .int(Int64(bitPattern: fixed(UInt64.self, tag)))
        case 0x14: return try .double(Double(bitPattern: fixed(UInt64.self, tag)))
        case 0x13: return try .double(Double(Float(bitPattern: fixed(UInt32.self, tag))))
        case 0x30:
            let seconds = try Double(bitPattern: fixed(UInt64.self, tag))
            return .date(Date(timeIntervalSinceReferenceDate: seconds))
        case 0x0D: return .bool(true)
        case 0x0E: return .bool(false)
        case 0x2D, 0x2E, 0x00: return .null
        case 0x0A, 0x0B, 0x0C, 0x1A, 0x1B:
            let count = try Int(fixed(UInt32.self, tag))
            var items: [DjayValue] = []
            for _ in 0 ..< count {
                try items.append(parseValue())
            }
            return .list(items)
        case 0x15: return try parseBlob()
        case 0x18: return try .int(Int64(fixed(UInt32.self, tag)))
        case 0x19:
            guard pos + 2 <= data.count else { throw fail("truncated 0x19 at \(pos)") }
            let value = UInt16(data[pos]) | UInt16(data[pos + 1]) << 8
            pos += 2
            return .int(Int64(value))
        default: throw fail("unknown tag 0x\(String(tag, radix: 16)) at \(pos - 1)")
        }
    }

    private mutating func parseBlob() throws -> DjayValue {
        pos = align(pos, 4)
        let length = try Int(fixed(UInt32.self, 0x15))
        guard length <= data.count - pos else { throw fail("truncated 0x15 data") }
        let raw = Data(data[pos ..< pos + length])
        pos += length
        let padded = align(pos, 4)
        if padded <= data.count, data[pos ..< padded].allSatisfy({ $0 == 0 }) {
            pos = padded
        }
        return .data(raw)
    }

    private mutating func parseObject() throws -> DjayValue {
        if depth >= Self.maxDepth {
            skipPastObject()
            return .object(className: nil, fields: [:])
        }
        depth += 1
        defer { depth -= 1 }
        return try parseObjectBody()
    }

    private static let valueStartTags: Set<UInt8> = [
        0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F, 0x02, 0x10, 0x11, 0x12, 0x13, 0x14,
        0x18, 0x19, 0x1A, 0x1B, 0x21, 0x2B, 0x30,
    ]

    private mutating func parseObjectBody() throws -> DjayValue {
        var className: String?
        if pos < data.count, data[pos] == 0x08 {
            pos += 1
            className = try readString()
        } else if pos < data.count, data[pos] == 0x05 {
            pos += 1
            className = try stringRef(byte("object class ref"))
        }
        var fields: [String: DjayValue] = [:]
        while pos < data.count {
            let peek = data[pos]
            if peek == 0x00 {
                pos += 1
                break
            }
            if peek == 0x05, pos + 3 < data.count, data[pos + 2] == 0x05 {
                let valueRef = data[pos + 1], keyRef = data[pos + 3]
                pos += 4
                try fields[stringRef(keyRef)] = try .string(stringRef(valueRef))
                continue
            }
            let value = try parseValue()
            guard pos < data.count else { break }
            let next = data[pos]
            switch next {
            case 0x00:
                if pos + 2 < data.count, data[pos + 1] == 0x08 {
                    pos += 2
                    try fields[readString()] = value
                } else {
                    pos += 1
                    return .object(className: className, fields: fields)
                }
            case 0x08:
                pos += 1
                try fields[readString()] = value
            case 0x05:
                pos += 1
                try fields[stringRef(byte("key binder"))] = value
            case 0x2D, 0x2E: break
            default:
                if !Self.valueStartTags.contains(next) {
                    throw fail("expected key binder at \(pos), got 0x\(String(next, radix: 16))")
                }
            }
        }
        return .object(className: className, fields: fields)
    }

    private mutating func skipPastObject() {
        var level = 1
        while level > 0, pos < data.count {
            let tag = data[pos]
            pos += 1
            switch tag {
            case 0x2B:
                level += 1
                if pos < data.count {
                    if data[pos] == 0x08, let (_, next) = try? cString(at: pos + 1) {
                        pos = next
                    } else if data[pos] == 0x05 {
                        pos += 2
                    }
                }
            case 0x00: level -= 1
            case 0x08, 0x21:
                if let (_, next) = try? cString(at: pos) {
                    pos = next
                } else {
                    pos = data.count
                }
            case 0x05: pos += 2
            case 0x0F, 0x0D, 0x0E, 0x2D, 0x2E, 0x18: pos += 1
            case 0x02, 0x10: pos = align(pos, 2) + 2
            case 0x11, 0x13, 0x0A, 0x0B, 0x0C, 0x1B, 0x19: pos = align(pos, 4) + 4
            case 0x12, 0x14, 0x30: pos = align(pos, 8) + 8
            case 0x15:
                pos = align(pos, 4)
                if pos + 4 <= data.count {
                    let length = (0 ..< 4).reduce(0) { $0 | Int(data[pos + $1]) << (8 * $1) }
                    pos += 4 + length
                } else {
                    pos = data.count
                }
            case 0x1A: pos += 4
            default: pos += 1
            }
        }
    }
}
