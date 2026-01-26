import Foundation
import SwiftCBOR

/// Centralized SwiftCBOR configuration and decode helpers for Rivet's CBOR wire format.
/// This keeps CBOR maps keyed by strings to match the server protocol, and it
/// provides numeric coercion plus JSONValue bridging for typed decoding.
enum CBORSupport {
    /// Returns an encoder configured to emit string keys for CBOR maps.
    static func encoder() -> CodableCBOREncoder {
        let encoder = CodableCBOREncoder()
        encoder.useStringKeys = true
        return encoder
    }

    /// Returns a decoder configured to expect string keys for CBOR maps.
    static func decoder() -> CodableCBORDecoder {
        let decoder = CodableCBORDecoder()
        decoder.useStringKeys = true
        return decoder
    }

    /// Decodes CBOR data into the requested type, coercing numeric values when needed.
    static func decode<T: Decodable>(_ type: T.Type, from data: Data) throws -> T {
        do {
            return try decoder().decode(T.self, from: data)
        } catch {
            if let coerced = try? coerceNumeric(T.self, from: data) {
                return coerced
            }
            throw error
        }
    }

    /// Decodes a JSONValue that originated from CBOR into the requested type.
    static func decode<T: Decodable>(_ type: T.Type, from value: JSONValue) throws -> T {
        if let coerced = coerceNumeric(T.self, from: value) {
            return coerced
        }
        let data = try encoder().encode(value)
        return try decode(T.self, from: data)
    }

    static func jsonValue(from data: Data) throws -> JSONValue {
        guard let decoded = try CBOR.decode(Array(data)) else {
            throw CBORSupportError.invalidValue("missing CBOR value")
        }
        return try jsonValue(from: decoded)
    }

    static func decodeHttpResponseError(from data: Data) throws -> HttpResponseError {
        let value = try jsonValue(from: data)
        guard case .object(let object) = value else {
            throw CBORSupportError.invalidValue("expected error object")
        }
        let group = try requireString(object["group"], name: "group")
        let code = try requireString(object["code"], name: "code")
        let message = try requireString(object["message"], name: "message")
        let metadata = optionalValue(object["metadata"])
        return HttpResponseError(group: group, code: code, message: message, metadata: metadata)
    }

    /// Decodes the bare error format that is sent without a CBOR envelope.
    static func decodeBareHttpResponseError(from data: Data) throws -> HttpResponseError {
        guard data.count >= 2 else {
            throw CBORSupportError.invalidValue("missing bare version prefix")
        }
        let version = UInt16(littleEndian: data.withUnsafeBytes { $0.load(as: UInt16.self) })
        guard version == 1 || version == 2 else {
            throw CBORSupportError.invalidValue("unsupported bare version")
        }
        var cursor = BareCursor(bytes: Array(data.dropFirst(2)))
        let group = try cursor.readString()
        let code = try cursor.readString()
        let message = try cursor.readString()
        let hasMetadata = try cursor.readBool()
        let metadata: JSONValue?
        if hasMetadata {
            let raw = try cursor.readData()
            metadata = try? jsonValue(from: raw)
        } else {
            metadata = nil
        }
        return HttpResponseError(group: group, code: code, message: message, metadata: metadata)
    }

    private static func jsonValue(from value: CBOR) throws -> JSONValue {
        switch value {
        case .null:
            return .null
        case .undefined:
            return .null
        case .boolean(let bool):
            return .bool(bool)
        case .unsignedInt(let uint):
            if uint <= UInt64(Int64.max) {
                return .number(.int(Int64(uint)))
            }
            return .number(.double(Double(uint)))
        case .negativeInt(let uint):
            let signed = -1 - Int64(uint)
            return .number(.int(signed))
        case .half(let number):
            return .number(.double(Double(number)))
        case .float(let number):
            return .number(.double(Double(number)))
        case .double(let number):
            return .number(.double(number))
        case .utf8String(let string):
            return .string(string)
        case .array(let values):
            return .array(try values.map { try jsonValue(from: $0) })
        case .map(let values):
            var object: [String: JSONValue] = [:]
            for (key, value) in values {
                guard case .utf8String(let keyString) = key else {
                    throw CBORSupportError.invalidValue("non-string map key")
                }
                object[keyString] = try jsonValue(from: value)
            }
            return .object(object)
        case .tagged(let tag, let inner) where tag == .positiveBignum:
            let uint = try decodeBignum(inner)
            if uint <= UInt64(Int64.max) {
                return .number(.int(Int64(uint)))
            }
            return .number(.double(Double(uint)))
        case .tagged(let tag, let inner) where tag == .negativeBignum:
            let uint = try decodeBignum(inner)
            let signed = -1 - Int64(uint)
            return .number(.int(signed))
        default:
            throw CBORSupportError.invalidValue("unsupported CBOR value")
        }
    }

    private static func decodeBignum(_ value: CBOR) throws -> UInt64 {
        guard case .byteString(let bytes) = value else {
            throw CBORSupportError.invalidValue("expected bignum bytes")
        }
        if bytes.isEmpty {
            return 0
        }
        if bytes.count > MemoryLayout<UInt64>.size {
            throw CBORSupportError.invalidValue("bignum overflow")
        }
        var result: UInt64 = 0
        for byte in bytes {
            result = (result << 8) | UInt64(byte)
        }
        return result
    }

    private static func requireString(_ value: JSONValue?, name: String) throws -> String {
        guard let value, case .string(let string) = value else {
            throw CBORSupportError.invalidValue("missing \(name)")
        }
        return string
    }

    private static func optionalValue(_ value: JSONValue?) -> JSONValue? {
        guard let value else {
            return nil
        }
        if case .null = value {
            return nil
        }
        return value
    }

    private static func coerceNumeric<T: Decodable>(_ type: T.Type, from data: Data) throws -> T? {
        switch type {
        case is Int64.Type:
            return try coerceInteger(Int64.self, from: data) as? T
        case is Int.Type:
            return try coerceInteger(Int.self, from: data) as? T
        case is UInt64.Type:
            return try coerceUnsigned(UInt64.self, from: data) as? T
        case is UInt.Type:
            return try coerceUnsigned(UInt.self, from: data) as? T
        default:
            return nil
        }
    }

    private static func coerceNumeric<T: Decodable>(_ type: T.Type, from value: JSONValue) -> T? {
        switch (type, value) {
        case (is Int64.Type, .number(let number)):
            return coerceInteger(Int64.self, from: number) as? T
        case (is Int.Type, .number(let number)):
            return coerceInteger(Int.self, from: number) as? T
        case (is UInt64.Type, .number(let number)):
            return coerceUnsigned(UInt64.self, from: number) as? T
        case (is UInt.Type, .number(let number)):
            return coerceUnsigned(UInt.self, from: number) as? T
        default:
            return nil
        }
    }

    private static func coerceInteger<T: FixedWidthInteger & SignedInteger>(_ type: T.Type, from number: JSONNumber) -> T? {
        switch number {
        case .int(let value):
            return T(value)
        case .double(let value):
            guard value.rounded() == value else {
                return nil
            }
            guard value >= Double(T.min) && value <= Double(T.max) else {
                return nil
            }
            return T(value)
        }
    }

    private static func coerceUnsigned<T: FixedWidthInteger & UnsignedInteger>(_ type: T.Type, from number: JSONNumber) -> T? {
        switch number {
        case .int(let value):
            guard value >= 0 else {
                return nil
            }
            return T(value)
        case .double(let value):
            guard value.rounded() == value else {
                return nil
            }
            guard value >= 0 && value <= Double(T.max) else {
                return nil
            }
            return T(value)
        }
    }

    private static func coerceInteger<T: FixedWidthInteger & SignedInteger>(_ type: T.Type, from data: Data) throws -> T {
        let value = try decoder().decode(Double.self, from: data)
        guard value.rounded() == value else {
            throw CBORSupportError.invalidValue("non-integer value")
        }
        guard value >= Double(T.min) && value <= Double(T.max) else {
            throw CBORSupportError.invalidValue("integer overflow")
        }
        return T(value)
    }

    private static func coerceUnsigned<T: FixedWidthInteger & UnsignedInteger>(_ type: T.Type, from data: Data) throws -> T {
        let value = try decoder().decode(Double.self, from: data)
        guard value.rounded() == value else {
            throw CBORSupportError.invalidValue("non-integer value")
        }
        guard value >= 0 && value <= Double(T.max) else {
            throw CBORSupportError.invalidValue("integer overflow")
        }
        return T(value)
    }
}

enum CBORSupportError: Error {
    case invalidValue(String)
}

private struct BareCursor {
    var bytes: [UInt8]
    var offset: Int = 0

    mutating func readByte() throws -> UInt8 {
        guard offset < bytes.count else {
            throw CBORSupportError.invalidValue("unexpected end of data")
        }
        let byte = bytes[offset]
        offset += 1
        return byte
    }

    mutating func readBool() throws -> Bool {
        let value = try readByte()
        guard value <= 1 else {
            throw CBORSupportError.invalidValue("invalid bool")
        }
        return value == 1
    }

    mutating func readUintSafe32() throws -> Int {
        var result = Int(try readByte())
        if result >= 0x80 {
            result &= 0x7f
            var shift = 7
            var byteCount = 1
            var byte: UInt8 = 0
            repeat {
                byte = try readByte()
                result += Int(byte & 0x7f) << shift
                shift += 7
                byteCount += 1
            } while byte >= 0x80 && byteCount < 5
            if byte == 0 {
                throw CBORSupportError.invalidValue("non-canonical int")
            }
            if byteCount == 5 && byte > 0x0f {
                throw CBORSupportError.invalidValue("integer overflow")
            }
        }
        return result
    }

    mutating func readBytes(count: Int) throws -> [UInt8] {
        guard count >= 0, offset + count <= bytes.count else {
            throw CBORSupportError.invalidValue("unexpected end of data")
        }
        let slice = Array(bytes[offset..<(offset + count)])
        offset += count
        return slice
    }

    mutating func readData() throws -> Data {
        let length = try readUintSafe32()
        let bytes = try readBytes(count: length)
        return Data(bytes)
    }

    mutating func readString() throws -> String {
        let length = try readUintSafe32()
        let bytes = try readBytes(count: length)
        return String(decoding: bytes, as: UTF8.self)
    }
}
