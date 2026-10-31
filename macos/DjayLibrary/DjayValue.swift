import Foundation

public indirect enum DjayValue: Equatable {
    case null
    case bool(Bool)
    case int(Int64)
    case double(Double)
    case string(String)
    case date(Date)
    case data(Data)
    case list([DjayValue])
    case object(className: String?, fields: [String: DjayValue])

    public var fields: [String: DjayValue]? {
        switch self {
        case let .object(_, fields): fields
        default: nil
        }
    }

    public var className: String? {
        if case let .object(name, _) = self {
            return name
        }
        return nil
    }

    public subscript(key: String) -> DjayValue? {
        fields?[key]
    }

    public var stringValue: String? {
        if case let .string(value) = self, !value.isEmpty {
            return value
        }
        return nil
    }

    public var intValue: Int? {
        switch self {
        case let .int(value): Int(exactly: value)
        default: nil
        }
    }

    public var numberValue: Double? {
        switch self {
        case let .int(value): Double(value)
        case let .double(value): value
        default: nil
        }
    }

    public var dateValue: Date? {
        if case let .date(value) = self {
            return value
        }
        return nil
    }

    public var listValue: [DjayValue]? {
        if case let .list(value) = self {
            return value
        }
        return nil
    }

    public var stringList: [String] {
        (listValue ?? []).compactMap(\.stringValue)
    }
}

public enum DjayDecodeError: Error, Equatable, CustomStringConvertible {
    case notTSAF
    case malformed(String)
    case unknownBlob
    case database(String)

    public var description: String {
        switch self {
        case .notTSAF: "not a TSAF blob"
        case let .malformed(reason): "malformed blob: \(reason)"
        case .unknownBlob: "unknown blob magic"
        case let .database(reason): "database: \(reason)"
        }
    }
}

public enum DjayBlob {
    public static func decode(_ blob: Data) throws -> DjayValue? {
        if blob.isEmpty || !blob.contains(where: { $0 != 0 }) {
            return nil
        }
        if blob.starts(with: Array("TSAF".utf8)) {
            var decoder = try TSAFDecoder(blob)
            return try decoder.decode()
        }
        if blob.starts(with: Array("bplist00".utf8)) {
            return try KeyedArchive.decode(blob)
        }
        throw DjayDecodeError.unknownBlob
    }
}
