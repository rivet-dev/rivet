import Foundation

public enum ActorConnStatus: String, Sendable {
    case idle
    case connecting
    case connected
    case disconnected
}

public typealias EventUnsubscribe = @Sendable () async -> Void
public typealias StatusChangeCallback = @Sendable (ActorConnStatus) -> Void
public typealias ConnectionStateCallback = @Sendable () -> Void
public typealias ActorErrorCallback = @Sendable (ActorError) -> Void

public actor ActorConnection {
    private let manager: RemoteManager
    private let registry: ConnectionRegistry
    private let queryState: ActorHandleState
    private let params: AnyEncodable?

    private var status: ActorConnStatus = .idle
    private var disposed = false
    private var websocket: URLSessionWebSocketTask?
    private var listenTask: Task<Void, Never>?
    private var connectTask: Task<Void, Never>?
    private var messageQueue: [ToServerMessage] = []
    private var actionsInFlight: [UInt64: CheckedContinuation<Data, Error>] = [:]
    private var eventSubscriptions: [String: [UUID: EventSubscription]] = [:]
    private var errorHandlers: [UUID: ActorErrorCallback] = [:]
    private var openHandlers: [UUID: ConnectionStateCallback] = [:]
    private var closeHandlers: [UUID: ConnectionStateCallback] = [:]
    private var statusHandlers: [UUID: StatusChangeCallback] = [:]
    private var actionIdCounter: UInt64 = 0
    private var actorId: String?
    private var connectionId: String?

    init(manager: RemoteManager, registry: ConnectionRegistry, queryState: ActorHandleState, params: AnyEncodable?) {
        self.manager = manager
        self.registry = registry
        self.queryState = queryState
        self.params = params
    }

    public func connect() {
        if disposed {
            return
        }
        if status == .connecting || status == .connected {
            return
        }
        setStatus(.connecting)
        connectTask?.cancel()
        connectTask = Task { [weak self] in
            await self?.connectLoop()
        }
    }

    public func action<Response: Decodable>(
        _ name: String,
        args: [AnyEncodable],
        as _: Response.Type = Response.self
    ) async throws -> Response {
        if disposed {
            throw ActorConnDisposed()
        }
        let actionId = actionIdCounter
        actionIdCounter += 1

        let request = ToServerMessage(body: .actionRequest(ActionRequest(id: actionId, name: name, args: args)))

        let data = try await withCheckedThrowingContinuation { (continuation: CheckedContinuation<Data, Error>) in
            actionsInFlight[actionId] = continuation
            sendMessage(request, ephemeral: false)
        }

        let decoder = JSONDecoder()
        return try decoder.decode(Response.self, from: data)
    }

    public func action<Response: Decodable>(
        _ name: String,
        as _: Response.Type = Response.self
    ) async throws -> Response {
        return try await action(name, args: [], as: Response.self)
    }

    public func action<Arg: Encodable, Response: Decodable>(
        _ name: String,
        arg: Arg,
        as _: Response.Type = Response.self
    ) async throws -> Response {
        return try await action(name, args: [AnyEncodable(arg)], as: Response.self)
    }

    public func on(_ eventName: String, handler: @escaping @Sendable ([JSONValue]) -> Void) async -> EventUnsubscribe {
        let id = UUID()
        let hadExisting = eventSubscriptions[eventName] != nil
        var handlers = eventSubscriptions[eventName] ?? [:]
        handlers[id] = EventSubscription(callback: handler, once: false)
        eventSubscriptions[eventName] = handlers
        if !hadExisting {
            sendSubscription(eventName: eventName, subscribe: true)
        }
        return { [weak self] in
            await self?.removeEventSubscription(eventName: eventName, id: id)
        }
    }

    public func once(_ eventName: String, handler: @escaping @Sendable ([JSONValue]) -> Void) async -> EventUnsubscribe {
        let id = UUID()
        let hadExisting = eventSubscriptions[eventName] != nil
        var handlers = eventSubscriptions[eventName] ?? [:]
        handlers[id] = EventSubscription(callback: handler, once: true)
        eventSubscriptions[eventName] = handlers
        if !hadExisting {
            sendSubscription(eventName: eventName, subscribe: true)
        }
        return { [weak self] in
            await self?.removeEventSubscription(eventName: eventName, id: id)
        }
    }

    public func onError(_ handler: @escaping ActorErrorCallback) async -> EventUnsubscribe {
        let id = UUID()
        errorHandlers[id] = handler
        return { [weak self] in
            await self?.removeErrorHandler(id: id)
        }
    }

    public func onOpen(_ handler: @escaping ConnectionStateCallback) async -> EventUnsubscribe {
        let id = UUID()
        openHandlers[id] = handler
        return { [weak self] in
            await self?.removeOpenHandler(id: id)
        }
    }

    public func onClose(_ handler: @escaping ConnectionStateCallback) async -> EventUnsubscribe {
        let id = UUID()
        closeHandlers[id] = handler
        return { [weak self] in
            await self?.removeCloseHandler(id: id)
        }
    }

    public func onStatusChange(_ handler: @escaping StatusChangeCallback) async -> EventUnsubscribe {
        let id = UUID()
        statusHandlers[id] = handler
        return { [weak self] in
            await self?.removeStatusHandler(id: id)
        }
    }

    public func getStatus() -> ActorConnStatus {
        status
    }

    public func dispose() async {
        if disposed {
            return
        }
        disposed = true
        setStatus(.idle)
        connectTask?.cancel()
        listenTask?.cancel()
        if let websocket {
            websocket.cancel(with: .normalClosure, reason: nil)
        }
        websocket = nil
        await registry.remove(self)
        rejectAllPending(error: ActorConnDisposed())
    }

    private func connectLoop() async {
        var attempt = 0
        let delay: UInt64 = 500_000_000
        while !disposed {
            attempt += 1
            do {
                let query = await queryState.query
                let actorId = try await manager.resolveActorId(for: query)
                self.actorId = actorId
                let protocols = try buildWebSocketProtocols()
                let ws = try await manager.openWebSocket(actorId: actorId, path: Routing.pathConnect, protocols: protocols)
                self.websocket = ws
                listenTask?.cancel()
                listenTask = Task { [weak self] in
                    await self?.listen(on: ws)
                }
                return
            } catch {
                setStatus(.disconnected)
                let backoff = min(delay * UInt64(attempt), 15_000_000_000)
                try? await Task.sleep(nanoseconds: backoff)
            }
        }
    }

    private func listen(on task: URLSessionWebSocketTask) async {
        while !disposed {
            do {
                let message = try await task.receive()
                await handleMessage(message)
            } catch {
                await handleClose(error: error)
                return
            }
        }
    }

    private func handleMessage(_ message: URLSessionWebSocketTask.Message) async {
        let data: Data
        switch message {
        case .string(let text):
            data = Data(text.utf8)
        case .data(let value):
            data = value
        @unknown default:
            return
        }

        do {
            let decoded = try JSONDecoder().decode(ToClientMessage.self, from: data)
            switch decoded.body {
            case .initMessage(let value):
                await handleInit(value)
            case .error(let value):
                await handleError(value)
            case .actionResponse(let value):
                await handleActionResponse(value)
            case .event(let value):
                await handleEvent(value)
            }
        } catch {
            await handleErrorMessage(group: "client", code: "malformed", message: "malformed response", metadata: nil, actionId: nil)
        }
    }

    private func handleInit(_ initMessage: InitMessage) async {
        actorId = initMessage.actorId
        connectionId = initMessage.connectionId
        setStatus(.connected)

        for (_, handler) in openHandlers {
            handler()
        }

        for eventName in eventSubscriptions.keys {
            sendSubscription(eventName: eventName, subscribe: true)
        }

        let queued = messageQueue
        messageQueue.removeAll()
        for message in queued {
            sendMessage(message, ephemeral: false)
        }
    }

    private func handleError(_ error: ErrorMessage) async {
        await handleErrorMessage(
            group: error.group,
            code: error.code,
            message: error.message,
            metadata: error.metadata,
            actionId: error.actionId
        )
    }

    private func handleErrorMessage(group: String, code: String, message: String, metadata: JSONValue?, actionId: UInt64?) async {
        let actorError = ActorError(group: group, code: code, message: message, metadata: metadata)
        if let actionId {
            if let continuation = actionsInFlight.removeValue(forKey: actionId) {
                continuation.resume(throwing: actorError)
            }
            return
        }

        var errorToThrow: Error = actorError
        if isSchedulingError(group: group, code: code), let actorId {
            let query = await queryState.query
            let name = query.name
            if let details = try? await manager.getActorError(actorId: actorId, name: name) {
                errorToThrow = ActorSchedulingError(group: group, code: code, actorId: actorId, details: details)
            }
        }

        rejectAllPending(error: errorToThrow)
        for (_, handler) in errorHandlers {
            if let actorError = errorToThrow as? ActorError {
                handler(actorError)
            }
        }
    }

    private func handleActionResponse(_ response: ActionResponseMessage) async {
        guard let continuation = actionsInFlight.removeValue(forKey: response.id) else {
            return
        }
        do {
            let data = try JSONEncoder().encode(response.output)
            continuation.resume(returning: data)
        } catch {
            continuation.resume(throwing: error)
        }
    }

    private func handleEvent(_ event: EventMessage) async {
        let argsValue = event.args
        let args: [JSONValue]
        if case .array(let array) = argsValue {
            args = array
        } else {
            args = [argsValue]
        }

        guard var handlers = eventSubscriptions[event.name] else {
            return
        }

        for (id, subscription) in handlers {
            subscription.callback(args)
            if subscription.once {
                handlers.removeValue(forKey: id)
            }
        }

        if handlers.isEmpty {
            eventSubscriptions.removeValue(forKey: event.name)
        } else {
            eventSubscriptions[event.name] = handlers
        }
    }

    private func handleClose(error: Error) async {
        if disposed {
            rejectAllPending(error: ActorConnDisposed())
            return
        }
        setStatus(.disconnected)
        var errorToThrow: Error = error
        if let websocket, let reasonData = websocket.closeReason, let reason = String(data: reasonData, encoding: .utf8) {
            if let parsed = WebSocketCloseReasonParser.parse(reason) {
                let group = parsed.group
                let code = parsed.code
                if isSchedulingError(group: group, code: code), let actorId {
                    let query = await queryState.query
                    let name = query.name
                    if let details = try? await manager.getActorError(actorId: actorId, name: name) {
                        errorToThrow = ActorSchedulingError(group: group, code: code, actorId: actorId, details: details)
                    } else {
                        errorToThrow = ActorError(group: group, code: code, message: "Connection closed: \(reason)", metadata: nil)
                    }
                } else {
                    errorToThrow = ActorError(group: group, code: code, message: "Connection closed: \(reason)", metadata: nil)
                }
            }
        }

        for (_, handler) in closeHandlers {
            handler()
        }
        rejectAllPending(error: errorToThrow)
        if let actorError = errorToThrow as? ActorError {
            for (_, handler) in errorHandlers {
                handler(actorError)
            }
        }
        connect()
    }

    private func sendMessage(_ message: ToServerMessage, ephemeral: Bool) {
        if disposed {
            return
        }
        guard let websocket, status == .connected else {
            if !ephemeral {
                messageQueue.append(message)
            }
            return
        }

        Task { [weak self] in
            do {
                let data = try JSONEncoder().encode(message)
                try await websocket.send(.data(data))
            } catch {
                await self?.handleClose(error: error)
            }
        }
    }

    private func sendSubscription(eventName: String, subscribe: Bool) {
        let request = ToServerMessage(body: .subscriptionRequest(SubscriptionRequest(eventName: eventName, subscribe: subscribe)))
        sendMessage(request, ephemeral: true)
    }

    private func rejectAllPending(error: Error) {
        for (_, continuation) in actionsInFlight {
            continuation.resume(throwing: error)
        }
        actionsInFlight.removeAll()
    }

    private func setStatus(_ newStatus: ActorConnStatus) {
        if status == newStatus {
            return
        }
        status = newStatus
        for (_, handler) in statusHandlers {
            handler(newStatus)
        }
    }

    private func removeEventSubscription(eventName: String, id: UUID) {
        guard var handlers = eventSubscriptions[eventName] else {
            return
        }
        handlers.removeValue(forKey: id)
        if handlers.isEmpty {
            eventSubscriptions.removeValue(forKey: eventName)
            sendSubscription(eventName: eventName, subscribe: false)
        } else {
            eventSubscriptions[eventName] = handlers
        }
    }

    private func removeErrorHandler(id: UUID) {
        errorHandlers.removeValue(forKey: id)
    }

    private func removeOpenHandler(id: UUID) {
        openHandlers.removeValue(forKey: id)
    }

    private func removeCloseHandler(id: UUID) {
        closeHandlers.removeValue(forKey: id)
    }

    private func removeStatusHandler(id: UUID) {
        statusHandlers.removeValue(forKey: id)
    }

    private func buildWebSocketProtocols() throws -> [String] {
        var protocols: [String] = []
        protocols.append(Routing.wsProtocolStandard)
        protocols.append("\(Routing.wsProtocolEncoding)json")
        if let params = params {
            let data = try JSONEncoder().encode(params)
            let json = String(data: data, encoding: .utf8) ?? "{}"
            let encoded = URLUtils.encodeURIComponent(json)
            protocols.append("\(Routing.wsProtocolConnParams)\(encoded)")
        }
        return protocols
    }
}

struct EventSubscription: Sendable {
    let callback: @Sendable ([JSONValue]) -> Void
    let once: Bool
}

struct InitMessage: Decodable, Sendable {
    let actorId: String
    let connectionId: String
}

struct ErrorMessage: Decodable, Sendable {
    let group: String
    let code: String
    let message: String
    let metadata: JSONValue?
    let actionId: UInt64?

    enum CodingKeys: String, CodingKey {
        case group
        case code
        case message
        case metadata
        case actionId
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        group = try container.decode(String.self, forKey: .group)
        code = try container.decode(String.self, forKey: .code)
        message = try container.decode(String.self, forKey: .message)
        metadata = try container.decodeIfPresent(JSONValue.self, forKey: .metadata)
        actionId = try container.decodeIfPresent(BigIntCompat.self, forKey: .actionId)?.value
    }
}

struct ActionResponseMessage: Decodable, Sendable {
    let id: UInt64
    let output: JSONValue

    enum CodingKeys: String, CodingKey {
        case id
        case output
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        id = try container.decode(BigIntCompat.self, forKey: .id).value
        output = try container.decode(JSONValue.self, forKey: .output)
    }
}

struct EventMessage: Decodable, Sendable {
    let name: String
    let args: JSONValue
}

struct ToClientMessage: Decodable, Sendable {
    let body: ToClientBody
}

enum ToClientBody: Decodable, Sendable {
    case initMessage(InitMessage)
    case error(ErrorMessage)
    case actionResponse(ActionResponseMessage)
    case event(EventMessage)

    enum CodingKeys: String, CodingKey {
        case tag
        case val
    }

    enum Tag: String, Decodable {
        case Init
        case Error
        case ActionResponse
        case Event
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        let tag = try container.decode(Tag.self, forKey: .tag)
        switch tag {
        case .Init:
            let value = try container.decode(InitMessage.self, forKey: .val)
            self = .initMessage(value)
        case .Error:
            let value = try container.decode(ErrorMessage.self, forKey: .val)
            self = .error(value)
        case .ActionResponse:
            let value = try container.decode(ActionResponseMessage.self, forKey: .val)
            self = .actionResponse(value)
        case .Event:
            let value = try container.decode(EventMessage.self, forKey: .val)
            self = .event(value)
        }
    }
}

struct ActionRequest: Encodable, Sendable {
    let id: UInt64
    let name: String
    let args: [AnyEncodable]

    enum CodingKeys: String, CodingKey {
        case id
        case name
        case args
    }

    func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: CodingKeys.self)
        try container.encode(BigIntCompat(id), forKey: .id)
        try container.encode(name, forKey: .name)
        try container.encode(args, forKey: .args)
    }
}

struct SubscriptionRequest: Encodable, Sendable {
    let eventName: String
    let subscribe: Bool
}

struct ToServerMessage: Encodable, Sendable {
    let body: ToServerBody
}

enum ToServerBody: Encodable, Sendable {
    case actionRequest(ActionRequest)
    case subscriptionRequest(SubscriptionRequest)

    enum CodingKeys: String, CodingKey {
        case tag
        case val
    }

    func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: CodingKeys.self)
        switch self {
        case .actionRequest(let value):
            try container.encode("ActionRequest", forKey: .tag)
            try container.encode(value, forKey: .val)
        case .subscriptionRequest(let value):
            try container.encode("SubscriptionRequest", forKey: .tag)
            try container.encode(value, forKey: .val)
        }
    }
}
