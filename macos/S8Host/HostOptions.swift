import Foundation

struct HostOptions {
    var bridge: String?
    var screens = false
    var deck: Int?
    var logDeckPolls = false
    var poll: TimeInterval = 1
    var replay: String?
    var records: String?

    static let usage = """
    usage: S8Host --bridge <path to s4mk3-hid-midi-bridge> [--screens [--deck N [--log-deck-polls]] [--poll SECONDS]]
           S8Host --replay <snapshot dir> --records <output dir>
    """

    static func parse(_ arguments: [String]) -> HostOptions? {
        var options = HostOptions()
        var rest = arguments[...]
        while let argument = rest.popFirst() {
            switch argument {
            case "--screens":
                options.screens = true
            case "--log-deck-polls":
                options.logDeckPolls = true
            default:
                guard let value = rest.popFirst(), options.set(argument, to: value) else { return nil }
            }
        }
        return options.isValid ? options : nil
    }

    private mutating func set(_ flag: String, to value: String) -> Bool {
        switch flag {
        case "--bridge":
            bridge = value
        case "--replay":
            replay = value
        case "--records":
            records = value
        case "--deck":
            guard let number = Int(value), (1 ... 4).contains(number) else { return false }
            deck = number
        case "--poll":
            guard let seconds = TimeInterval(value), seconds >= 0.1 else { return false }
            poll = seconds
        default:
            return false
        }
        return true
    }

    private var isValid: Bool {
        if replay != nil {
            return records != nil && bridge == nil && !screens && deck == nil && !logDeckPolls
        }
        return bridge != nil && records == nil && (screens || deck == nil) && (deck != nil || !logDeckPolls)
    }
}
