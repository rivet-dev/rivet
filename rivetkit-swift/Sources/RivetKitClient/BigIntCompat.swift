import Foundation

struct BigIntCompat: Codable, Sendable {
    let value: UInt64

    init(_ value: UInt64) {
        self.value = value
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.singleValueContainer()
        if let uint = try? container.decode(UInt64.self) {
            value = uint
            return
        }
        if let string = try? container.decode(String.self), let uint = UInt64(string) {
            value = uint
            return
        }
        if let array = try? container.decode([JSONValue].self) {
            if array.count == 2, case .string("$BigInt") = array[0] {
                switch array[1] {
                case .string(let valueString):
                    guard let uint = UInt64(valueString) else {
                        break
                    }
                    value = uint
                    return
                case .number(let number):
                    if let int = number.intValue, int >= 0 {
                        value = UInt64(int)
                        return
                    }
                default:
                    break
                }
            }
        }
        throw DecodingError.dataCorruptedError(in: container, debugDescription: "invalid BigInt encoding")
    }

    func encode(to encoder: Encoder) throws {
        var container = encoder.unkeyedContainer()
        try container.encode("$BigInt")
        try container.encode(String(value))
    }
}
