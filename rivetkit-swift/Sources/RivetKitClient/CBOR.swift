import Foundation

enum CBOREncoder {
    static func encode(_ value: JSONValue) -> Data {
        var data = Data()
        append(value, to: &data)
        return data
    }

    static func encodeEncodable(_ value: AnyEncodable) throws -> Data {
        let jsonData = try JSONEncoder().encode(value)
        let jsonObject = try JSONSerialization.jsonObject(with: jsonData, options: [])
        let jsonValue = try JSONValue.from(any: jsonObject)
        return encode(jsonValue)
    }

    private static func append(_ value: JSONValue, to data: inout Data) {
        switch value {
        case .null:
            data.append(0xf6)
        case .bool(let flag):
            data.append(flag ? 0xf5 : 0xf4)
        case .number(let number):
            if let intValue = number.intValue {
                appendInt(intValue, to: &data)
            } else {
                appendDouble(number.doubleValue, to: &data)
            }
        case .string(let string):
            let bytes = Data(string.utf8)
            appendLength(majorType: 3, length: UInt64(bytes.count), to: &data)
            data.append(bytes)
        case .array(let values):
            appendLength(majorType: 4, length: UInt64(values.count), to: &data)
            for value in values {
                append(value, to: &data)
            }
        case .object(let dict):
            appendLength(majorType: 5, length: UInt64(dict.count), to: &data)
            for (key, value) in dict {
                append(.string(key), to: &data)
                append(value, to: &data)
            }
        }
    }

    private static func appendInt(_ value: Int64, to data: inout Data) {
        if value >= 0 {
            appendLength(majorType: 0, length: UInt64(value), to: &data)
        } else {
            let encoded = UInt64(-1 - value)
            appendLength(majorType: 1, length: encoded, to: &data)
        }
    }

    private static func appendDouble(_ value: Double, to data: inout Data) {
        data.append(0xfb)
        let bits = value.bitPattern
        var bigEndian = bits.bigEndian
        withUnsafeBytes(of: &bigEndian) { buffer in
            data.append(contentsOf: buffer)
        }
    }

    private static func appendLength(majorType: UInt8, length: UInt64, to data: inout Data) {
        if length <= 23 {
            data.append((majorType << 5) | UInt8(length))
            return
        }
        if length <= UInt64(UInt8.max) {
            data.append((majorType << 5) | 24)
            data.append(UInt8(length))
            return
        }
        if length <= UInt64(UInt16.max) {
            data.append((majorType << 5) | 25)
            var value = UInt16(length).bigEndian
            withUnsafeBytes(of: &value) { buffer in
                data.append(contentsOf: buffer)
            }
            return
        }
        if length <= UInt64(UInt32.max) {
            data.append((majorType << 5) | 26)
            var value = UInt32(length).bigEndian
            withUnsafeBytes(of: &value) { buffer in
                data.append(contentsOf: buffer)
            }
            return
        }
        data.append((majorType << 5) | 27)
        var value = UInt64(length).bigEndian
        withUnsafeBytes(of: &value) { buffer in
            data.append(contentsOf: buffer)
        }
    }
}
