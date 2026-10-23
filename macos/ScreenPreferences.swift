import Foundation

struct ScreenPreferences: Codable, Equatable {
    var enabled = true
    var automaticallyStopNI = false
    var restoreNIOnStop = true
    var restorePending = false

    static func decode(_ data: Data) throws -> ScreenPreferences {
        guard let values = try JSONSerialization.jsonObject(with: data) as? [String: Any] else {
            throw AppError.invalid("Screen preferences must be a JSON object.")
        }
        let allowed: Set<String> = [
            "enabled", "automaticallyStopNI", "restoreNIOnStop", "restorePending"
        ]
        guard Set(values.keys).isSubset(of: allowed) else {
            throw AppError.invalid("Screen preferences contain unknown settings.")
        }
        var merged: [String: Any] = [
            "enabled": true, "automaticallyStopNI": false,
            "restoreNIOnStop": true, "restorePending": false
        ]
        merged.merge(values) { _, override in override }
        return try JSONDecoder().decode(Self.self, from: JSONSerialization.data(withJSONObject: merged))
    }

    func save(to url: URL) throws {
        let encoder = JSONEncoder()
        encoder.outputFormatting = [.prettyPrinted, .sortedKeys]
        let data = try encoder.encode(self)
        try FileManager.default.createDirectory(
            at: url.deletingLastPathComponent(), withIntermediateDirectories: true
        )
        try data.write(to: url, options: .atomic)
    }
}

enum ScreenState: String, Equatable {
    case off = "Screens off"
    case ready = "Ready to start"
    case pending = "Waiting for controller"
    case active = "Screens active"
    case blocked = "Screens blocked by NI driver"
    case failed = "Screens need attention"
    case releasing = "Releasing screens"
}

enum ScreenConsent: Equatable {
    case allow(remember: Bool)
    case withoutScreens
    case cancel
}

enum ScreenCommand: String {
    case on = "ON"
    case off = "OFF"
    case retry = "RETRY"
}
