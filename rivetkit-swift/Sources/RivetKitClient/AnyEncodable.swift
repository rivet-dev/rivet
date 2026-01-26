import Foundation

public struct AnyEncodable: Encodable, @unchecked Sendable {
    // Safe because the wrapped value is only used synchronously by an encoder on the current task.
    private let encodeBlock: (Encoder) throws -> Void

    public init<T: Encodable>(_ value: T) {
        self.encodeBlock = value.encode
    }

    public func encode(to encoder: Encoder) throws {
        try encodeBlock(encoder)
    }
}
