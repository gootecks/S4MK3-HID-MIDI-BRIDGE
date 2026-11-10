import Foundation
import zlib

enum Inflate {
    static func zlib(_ input: Data) -> [UInt8]? {
        guard !input.isEmpty else { return nil }
        var capacity = max(input.count * 8, 4096)
        for _ in 0 ..< 6 {
            var output = [UInt8](repeating: 0, count: capacity)
            var length = uLongf(capacity)
            let status = input.withUnsafeBytes { buffer -> Int32 in
                uncompress(&output, &length, buffer.bindMemory(to: UInt8.self).baseAddress, uLong(input.count))
            }
            if status == Z_OK {
                return Array(output.prefix(Int(length)))
            }
            guard status == Z_BUF_ERROR else { return nil }
            capacity *= 4
        }
        return nil
    }
}
