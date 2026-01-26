import Foundation

public struct ActorError: Error, Sendable, Equatable {
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
}

public struct ActorSchedulingError: Error, Sendable, Equatable {
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
}

public struct ActorConnDisposed: Error, Sendable, Equatable {
    public init() {}
}

public struct HttpRequestError: Error, Sendable, Equatable {
    public let message: String

    public init(_ message: String) {
        self.message = message
    }
}

public struct ConfigurationError: Error, Sendable, Equatable {
    public let message: String

    public init(_ message: String) {
        self.message = message
    }
}

public struct InternalError: Error, Sendable, Equatable {
    public let message: String

    public init(_ message: String) {
        self.message = message
    }
}

public struct ManagerApiError: Error, Sendable, Equatable {
    public let group: String
    public let code: String
    public let message: String

    public init(group: String, code: String, message: String) {
        self.group = group
        self.code = code
        self.message = message
    }
}

func isSchedulingError(group: String, code: String) -> Bool {
    return group == "guard" && (code == "actor_ready_timeout" || code == "actor_runner_failed")
}
