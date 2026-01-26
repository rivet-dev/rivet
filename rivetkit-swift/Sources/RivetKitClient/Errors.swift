import Foundation

public struct ActorError: Error, Sendable, Equatable, LocalizedError, CustomStringConvertible {
    public let group: String
    public let code: String
    public let message: String
    public let metadata: JSONValue?

    public init(group: String, code: String, message: String, metadata: JSONValue? = nil) {
        self.group = group
        self.code = code
        self.message = message
        self.metadata = metadata
    }

    public var description: String {
        "[\(group):\(code)] \(message)"
    }

    public var errorDescription: String? { description }
}

public struct ActorSchedulingError: Error, Sendable, Equatable, LocalizedError, CustomStringConvertible {
    public let group: String
    public let code: String
    public let actorId: String
    public let details: JSONValue?

    public init(group: String, code: String, actorId: String, details: JSONValue?) {
        self.group = group
        self.code = code
        self.actorId = actorId
        self.details = details
    }

    public var description: String {
        "[\(group):\(code)] Actor scheduling failed for \(actorId)"
    }

    public var errorDescription: String? { description }
}

public struct ActorConnDisposed: Error, Sendable, Equatable, LocalizedError, CustomStringConvertible {
    public init() {}

    public var description: String {
        "Actor connection has been disposed"
    }

    public var errorDescription: String? { description }
}

public struct HttpRequestError: Error, Sendable, Equatable, LocalizedError, CustomStringConvertible {
    public let message: String

    public init(_ message: String) {
        self.message = message
    }

    public var description: String { message }
    public var errorDescription: String? { description }
}

public struct ConfigurationError: Error, Sendable, Equatable, LocalizedError, CustomStringConvertible {
    public let message: String

    public init(_ message: String) {
        self.message = message
    }

    public var description: String { message }
    public var errorDescription: String? { description }
}

public struct InternalError: Error, Sendable, Equatable, LocalizedError, CustomStringConvertible {
    public let message: String

    public init(_ message: String) {
        self.message = message
    }

    public var description: String { message }
    public var errorDescription: String? { description }
}

public struct ManagerApiError: Error, Sendable, Equatable, LocalizedError, CustomStringConvertible {
    public let group: String
    public let code: String
    public let message: String

    public init(group: String, code: String, message: String) {
        self.group = group
        self.code = code
        self.message = message
    }

    public var description: String {
        "[\(group):\(code)] \(message)"
    }

    public var errorDescription: String? { description }
}

func isSchedulingError(group: String, code: String) -> Bool {
    return group == "guard" && (code == "actor_ready_timeout" || code == "actor_runner_failed")
}
