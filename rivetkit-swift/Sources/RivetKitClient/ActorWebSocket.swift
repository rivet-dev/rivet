import Foundation

public enum ActorWebSocketMessage: Sendable, Equatable {
    case text(String)
    case data(Data)
}

public final class ActorWebSocket: @unchecked Sendable {
    // Safe because all mutable state is actor-isolated in WebSocketCore.
    private let core: WebSocketCore

    init(task: URLSessionWebSocketTask) {
        self.core = WebSocketCore(task: task)
    }

    public func send(text: String) async throws {
        try await core.send(message: .string(text))
    }

    public func send(data: Data) async throws {
        try await core.send(message: .data(data))
    }

    public func receive() async throws -> ActorWebSocketMessage {
        try await core.receive()
    }

    public func close(code: URLSessionWebSocketTask.CloseCode = .normalClosure, reason: Data? = nil) async {
        await core.close(code: code, reason: reason)
    }
}

actor WebSocketCore {
    private let task: URLSessionWebSocketTask

    init(task: URLSessionWebSocketTask) {
        self.task = task
    }

    func send(message: URLSessionWebSocketTask.Message) async throws {
        try await task.send(message)
    }

    func receive() async throws -> ActorWebSocketMessage {
        let message = try await task.receive()
        switch message {
        case .string(let value):
            return .text(value)
        case .data(let value):
            return .data(value)
        @unknown default:
            throw InternalError("unknown websocket message")
        }
    }

    func close(code: URLSessionWebSocketTask.CloseCode, reason: Data?) {
        task.cancel(with: code, reason: reason)
    }
}
