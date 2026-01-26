import Foundation
import SwiftCBOR

public struct RawHTTPResponse: Sendable {
    public let data: Data
    public let response: HTTPURLResponse

    public var statusCode: Int { response.statusCode }
    public var ok: Bool { (200..<300).contains(response.statusCode) }
    public var headers: [AnyHashable: Any] { response.allHeaderFields }

    public func text() -> String {
        String(data: data, encoding: .utf8) ?? ""
    }

    public func json<T: Decodable>(_ type: T.Type = T.self) throws -> T {
        try JSONDecoder().decode(T.self, from: data)
    }

    public func cbor<T: Decodable>(_ type: T.Type = T.self) throws -> T {
        try CBORSupport.decode(T.self, from: data)
    }
}

public struct RawHTTPRequest: Sendable {
    public var method: String
    public var headers: [String: String]
    public var body: Data?

    public init(method: String = "GET", headers: [String: String] = [:], body: Data? = nil) {
        self.method = method
        self.headers = headers
        self.body = body
    }
}
