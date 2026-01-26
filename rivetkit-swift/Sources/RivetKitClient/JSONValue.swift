import Foundation

public enum JSONNumber: Codable, Sendable, Equatable {
    case int(Int64)
    case double(Double)

    public init(from decoder: Decoder) throws {
        let container = try decoder.singleValueContainer()
        if let intValue = try? container.decode(Int64.self) {
            self = .int(intValue)
            return
        }
        let doubleValue = try container.decode(Double.self)
        self = .double(doubleValue)
    }

    public func encode(to encoder: Encoder) throws {
        var container = encoder.singleValueContainer()
        switch self {
        case .int(let value):
            try container.encode(value)
        case .double(let value):
            try container.encode(value)
        }
    }

    var doubleValue: Double {
        switch self {
        case .int(let value):
            return Double(value)
        case .double(let value):
            return value
        }
    }

    var intValue: Int64? {
        switch self {
        case .int(let value):
            return value
        case .double:
            return nil
        }
    }
}

public enum JSONValue: Codable, Sendable, Equatable {
    case null
    case bool(Bool)
    case number(JSONNumber)
    case string(String)
    case array([JSONValue])
    case object([String: JSONValue])

    public init(from decoder: Decoder) throws {
        let container = try decoder.singleValueContainer()
        if container.decodeNil() {
            self = .null
            return
        }
        if let boolValue = try? container.decode(Bool.self) {
            self = .bool(boolValue)
            return
        }
        if let intValue = try? container.decode(Int64.self) {
            self = .number(.int(intValue))
            return
        }
        if let doubleValue = try? container.decode(Double.self) {
            self = .number(.double(doubleValue))
            return
        }
        if let stringValue = try? container.decode(String.self) {
            self = .string(stringValue)
            return
        }
        if let arrayValue = try? container.decode([JSONValue].self) {
            self = .array(arrayValue)
            return
        }
        if let objectValue = try? container.decode([String: JSONValue].self) {
            self = .object(objectValue)
            return
        }
        throw DecodingError.dataCorruptedError(in: container, debugDescription: "Unsupported JSON value")
    }

    public func encode(to encoder: Encoder) throws {
        var container = encoder.singleValueContainer()
        switch self {
        case .null:
            try container.encodeNil()
        case .bool(let value):
            try container.encode(value)
        case .number(let value):
            try container.encode(value)
        case .string(let value):
            try container.encode(value)
        case .array(let value):
            try container.encode(value)
        case .object(let value):
            try container.encode(value)
        }
    }

    static func from(any value: Any) throws -> JSONValue {
        if value is NSNull {
            return .null
        }
        if let result = value as? String {
            return .string(result)
        }
        if let result = value as? Bool {
            return .bool(result)
        }
        if let result = value as? NSNumber {
            if CFGetTypeID(result) == CFBooleanGetTypeID() {
                return .bool(result.boolValue)
            }
            let doubleValue = result.doubleValue
            let intValue = result.int64Value
            if Double(intValue) == doubleValue {
                return .number(.int(intValue))
            }
            return .number(.double(doubleValue))
        }
        if let result = value as? [Any] {
            return .array(try result.map { try JSONValue.from(any: $0) })
        }
        if let result = value as? [String: Any] {
            var mapped: [String: JSONValue] = [:]
            for (key, value) in result {
                mapped[key] = try JSONValue.from(any: value)
            }
            return .object(mapped)
        }
        throw EncodingError.invalidValue(
            value,
            EncodingError.Context(codingPath: [], debugDescription: "Unsupported JSON value")
        )
    }

    var asAny: Any {
        switch self {
        case .null:
            return NSNull()
        case .bool(let value):
            return value
        case .number(let value):
            if let intValue = value.intValue {
                return intValue
            }
            return value.doubleValue
        case .string(let value):
            return value
        case .array(let value):
            return value.map { $0.asAny }
        case .object(let value):
            var mapped: [String: Any] = [:]
            for (key, jsonValue) in value {
                mapped[key] = jsonValue.asAny
            }
            return mapped
        }
    }
}
