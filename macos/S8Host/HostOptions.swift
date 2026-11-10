import Foundation

struct HostOptions {
    var bridge: String?
    var screens = false
    var deck: Int?
    var poll: TimeInterval = 1
    var replay: String?
    var records: String?

    static let usage = """
    usage: S8Host --bridge <path to s4mk3-hid-midi-bridge> [--screens [--deck N] [--poll SECONDS]]
           S8Host --replay <snapshot dir> --records <output dir>
    """

    static func parse(_ arguments: [String]) -> HostOptions? {
        var options = HostOptions()
        var rest = arguments[...]
        while let argument = rest.popFirst() {
            switch argument {
            case "--screens":
                options.screens = true
            case "--bridge":
                options.bridge = rest.popFirst()
            case "--replay":
                options.replay = rest.popFirst()
            case "--records":
                options.records = rest.popFirst()
            case "--deck":
                guard let deck = rest.popFirst().flatMap(Int.init), (1 ... 4).contains(deck) else { return nil }
                options.deck = deck
            case "--poll":
                guard let poll = rest.popFirst().flatMap(TimeInterval.init), poll >= 0.1 else { return nil }
                options.poll = poll
            default:
                return nil
            }
        }
        return options.isValid ? options : nil
    }

    private var isValid: Bool {
        if replay != nil {
            return records != nil && bridge == nil && !screens && deck == nil
        }
        return bridge != nil && records == nil && (screens || deck == nil)
    }
}
