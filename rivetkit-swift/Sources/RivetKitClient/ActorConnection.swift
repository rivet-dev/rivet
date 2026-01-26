import Foundation
import os
import SwiftCBOR

private let logger = RivetLogger.connection

/// Connection status for an actor connection.
///
/// State transitions:
/// - `idle` → `connecting` (via connect())
/// - `connecting` → `connected` (on successful Init message)
/// - `connected` → `disconnected` (on socket close)
/// - `disconnected` → `connecting` (auto-reconnect)
/// - Any state → `disposed` (via dispose())
public enum ActorConnStatus: String, Sendable {
    /// Not connected, no auto-reconnect. Initial state.
    case idle
    /// Attempting to establish connection.
    case connecting
    /// Connection is active and operational.
    case connected
    /// Connection lost, will auto-reconnect.
    case disconnected
    /// Permanently closed, no further operations allowed.
    case disposed
}

private typealias EventUnsubscribe = @Sendable () async -> Void
private typealias StatusChangeCallback = @Sendable (ActorConnStatus) -> Void
private typealias ConnectionStateCallback = @Sendable () -> Void
private typealias ActorErrorCallback = @Sendable (ActorError) -> Void

/// Maintains a live websocket connection to a Rivet Actor using the CBOR wire protocol.
public actor ActorConnection {
    private let manager: RemoteManager
    private let registry: ConnectionRegistry
    private let queryState: ActorHandleState
    private let params: AnyEncodable?

    private var status: ActorConnStatus = .idle
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
    private var errorStreamContinuations: [UUID: AsyncStream<ActorError>.Continuation] = [:]
    private var openStreamContinuations: [UUID: AsyncStream<Void>.Continuation] = [:]
    private var closeStreamContinuations: [UUID: AsyncStream<Void>.Continuation] = [:]
    private var statusStreamContinuations: [UUID: AsyncStream<ActorConnStatus>.Continuation] = [:]
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
        switch status {
        case .disposed:
            logger.debug("connect called after dispose, ignoring")
            return
        case .connecting, .connected:
            logger.debug("connect called but already \(self.status.rawValue, privacy: .public)")
            return
        case .idle, .disconnected:
            break
        }
        logger.debug("initiating connection")
        setStatus(.connecting)
        connectTask?.cancel()
        connectTask = Task { [weak self] in
            await self?.connectLoop()
        }
    }

    /// Performs an action over the websocket using positional arguments encoded as CBOR.
    public func action<Response: Decodable & Sendable>(
        _ name: String,
        args: [AnyEncodable],
        as _: Response.Type = Response.self
    ) async throws -> Response {
        if status == .disposed {
            logger.debug("action \(name, privacy: .public) called after dispose")
            throw ActorConnDisposed()
        }
        let actionId = actionIdCounter
        actionIdCounter += 1

        logger.debug("action \(name, privacy: .public) actionId=\(actionId)")

        let request = ToServerMessage(body: .actionRequest(ActionRequest(id: actionId, name: name, args: args)))

        let data = try await withCheckedThrowingContinuation { (continuation: CheckedContinuation<Data, Error>) in
            actionsInFlight[actionId] = continuation
            logger.debug("added action to in-flight map actionId=\(actionId) actionName=\(name, privacy: .public) inFlightCount=\(self.actionsInFlight.count)")
            sendMessage(request, ephemeral: false)
        }

        logger.debug("received action response actionId=\(actionId) actionName=\(name, privacy: .public)")
        return try CBORSupport.decode(Response.self, from: data)
    }

    /// Performs an action with variadic typed arguments using parameter packs.
    public func action<each Arg: Encodable & Sendable, Response: Decodable & Sendable>(
        _ name: String,
        _ args: repeat each Arg,
        as _: Response.Type = Response.self
    ) async throws -> Response {
        var argsArray: [AnyEncodable] = []
        repeat argsArray.append(AnyEncodable(each args))
        return try await action(name, args: argsArray, as: Response.self)
    }

    /// Returns an AsyncStream of decoded event values.
    public func events<T: Decodable & Sendable>(_ name: String, as _: T.Type = T.self) -> AsyncStream<T> {
        let (stream, continuation) = AsyncStream<T>.makeStream()
        let eventName = name
        let unsubscribe = subscribe(eventName, once: false) { [weak self] args in
            let result = Self.decodeEventArgsSingle(args, as: T.self)
            switch result {
            case .success(let value):
                continuation.yield(value)
            case .failure(let error):
                Task { [weak self] in
                    await self?.emitDecodeError(eventName: eventName, error: error)
                }
            }
        }
        continuation.onTermination = { _ in
            Task {
                await unsubscribe()
            }
        }
        return stream
    }

    /// Returns an AsyncStream of void events.
    public func events(_ name: String, as _: Void.Type = Void.self) -> AsyncStream<Void> {
        let (stream, continuation) = AsyncStream<Void>.makeStream()
        let eventName = name
        let unsubscribe = subscribe(eventName, once: false) { [weak self] args in
            let result = Self.decodeEventArgsZero(args)
            switch result {
            case .success:
                continuation.yield(())
            case .failure(let error):
                Task { [weak self] in
                    await self?.emitDecodeError(eventName: eventName, error: error)
                }
            }
        }
        continuation.onTermination = { _ in
            Task {
                await unsubscribe()
            }
        }
        return stream
    }

    /// Returns an AsyncStream of events with two arguments.
    public func events<A: Decodable & Sendable, B: Decodable & Sendable>(
        _ name: String,
        as _: (A, B).Type
    ) -> AsyncStream<(A, B)> {
        let (stream, continuation) = AsyncStream<(A, B)>.makeStream()
        let eventName = name
        let unsubscribe = subscribe(eventName, once: false) { [weak self] args in
            let result = Self.decodeEventArgsPair(args, as: (A, B).self)
            switch result {
            case .success(let value):
                continuation.yield(value)
            case .failure(let error):
                Task { [weak self] in
                    await self?.emitDecodeError(eventName: eventName, error: error)
                }
            }
        }
        continuation.onTermination = { _ in
            Task {
                await unsubscribe()
            }
        }
        return stream
    }

    /// Returns an AsyncStream of events with three arguments.
    public func events<A: Decodable & Sendable, B: Decodable & Sendable, C: Decodable & Sendable>(
        _ name: String,
        as _: (A, B, C).Type
    ) -> AsyncStream<(A, B, C)> {
        let (stream, continuation) = AsyncStream<(A, B, C)>.makeStream()
        let eventName = name
        let unsubscribe = subscribe(eventName, once: false) { [weak self] args in
            let result = Self.decodeEventArgsTriple(args, as: (A, B, C).self)
            switch result {
            case .success(let value):
                continuation.yield(value)
            case .failure(let error):
                Task { [weak self] in
                    await self?.emitDecodeError(eventName: eventName, error: error)
                }
            }
        }
        continuation.onTermination = { _ in
            Task {
                await unsubscribe()
            }
        }
        return stream
    }

    /// Returns an AsyncStream of events with four arguments.
    public func events<
        A: Decodable & Sendable,
        B: Decodable & Sendable,
        C: Decodable & Sendable,
        D: Decodable & Sendable
    >(
        _ name: String,
        as _: (A, B, C, D).Type
    ) -> AsyncStream<(A, B, C, D)> {
        let (stream, continuation) = AsyncStream<(A, B, C, D)>.makeStream()
        let eventName = name
        let unsubscribe = subscribe(eventName, once: false) { [weak self] args in
            let result = Self.decodeEventArgsQuad(args, as: (A, B, C, D).self)
            switch result {
            case .success(let value):
                continuation.yield(value)
            case .failure(let error):
                Task { [weak self] in
                    await self?.emitDecodeError(eventName: eventName, error: error)
                }
            }
        }
        continuation.onTermination = { _ in
            Task {
                await unsubscribe()
            }
        }
        return stream
    }

    /// Returns an AsyncStream of events with five arguments.
    public func events<
        A: Decodable & Sendable,
        B: Decodable & Sendable,
        C: Decodable & Sendable,
        D: Decodable & Sendable,
        E: Decodable & Sendable
    >(
        _ name: String,
        as _: (A, B, C, D, E).Type
    ) -> AsyncStream<(A, B, C, D, E)> {
        let (stream, continuation) = AsyncStream<(A, B, C, D, E)>.makeStream()
        let eventName = name
        let unsubscribe = subscribe(eventName, once: false) { [weak self] args in
            let result = Self.decodeEventArgsQuint(args, as: (A, B, C, D, E).self)
            switch result {
            case .success(let value):
                continuation.yield(value)
            case .failure(let error):
                Task { [weak self] in
                    await self?.emitDecodeError(eventName: eventName, error: error)
                }
            }
        }
        continuation.onTermination = { _ in
            Task {
                await unsubscribe()
            }
        }
        return stream
    }

    /// Returns an AsyncStream of raw JSON event arguments.
    public func events(_ name: String) -> AsyncStream<[JSONValue]> {
        let (stream, continuation) = AsyncStream<[JSONValue]>.makeStream()
        let unsubscribe = subscribe(name, once: false) { args in
            continuation.yield(args)
        }
        continuation.onTermination = { _ in
            Task {
                await unsubscribe()
            }
        }
        return stream
    }

    /// Returns an AsyncStream of connection errors.
    public func errors() -> AsyncStream<ActorError> {
        let (stream, continuation) = AsyncStream<ActorError>.makeStream()
        let id = UUID()
        errorStreamContinuations[id] = continuation
        errorHandlers[id] = { error in
            continuation.yield(error)
        }
        continuation.onTermination = { [weak self] _ in
            Task { [weak self] in
                await self?.removeErrorHandler(id: id)
            }
        }
        return stream
    }

    /// Returns an AsyncStream that yields each time the connection opens.
    public func opens() -> AsyncStream<Void> {
        let (stream, continuation) = AsyncStream<Void>.makeStream()
        let id = UUID()
        openStreamContinuations[id] = continuation
        openHandlers[id] = {
            continuation.yield(())
        }
        continuation.onTermination = { [weak self] _ in
            Task { [weak self] in
                await self?.removeOpenHandler(id: id)
            }
        }
        return stream
    }

    /// Returns an AsyncStream that yields each time the connection closes.
    public func closes() -> AsyncStream<Void> {
        let (stream, continuation) = AsyncStream<Void>.makeStream()
        let id = UUID()
        closeStreamContinuations[id] = continuation
        closeHandlers[id] = {
            continuation.yield(())
        }
        continuation.onTermination = { [weak self] _ in
            Task { [weak self] in
                await self?.removeCloseHandler(id: id)
            }
        }
        return stream
    }

    /// Returns an AsyncStream of status changes. Immediately yields the current status.
    public func statusChanges() -> AsyncStream<ActorConnStatus> {
        let (stream, continuation) = AsyncStream<ActorConnStatus>.makeStream()
        let id = UUID()
        statusStreamContinuations[id] = continuation
        // Immediately yield current status (BehaviorSubject pattern)
        continuation.yield(status)
        statusHandlers[id] = { status in
            continuation.yield(status)
        }
        continuation.onTermination = { [weak self] _ in
            Task { [weak self] in
                await self?.removeStatusHandler(id: id)
            }
        }
        return stream
    }

    /// The current connection status.
    public var currentStatus: ActorConnStatus {
        status
    }

    public func dispose() async {
        if status == .disposed {
            logger.debug("dispose called but already disposed")
            return
        }
        logger.debug("disposing actor connection")
        let wasConnected = status == .connected
        setStatus(.disposed)
        connectTask?.cancel()
        listenTask?.cancel()
        if let websocket {
            if wasConnected {
                websocket.cancel(with: .normalClosure, reason: nil)
            } else {
                websocket.cancel()
            }
        }
        websocket = nil
        await registry.remove(self)
        rejectAllPending(error: ActorConnDisposed())
        finishAllStreams()
    }

    private func finishAllStreams() {
        for (_, continuation) in errorStreamContinuations {
            continuation.finish()
        }
        errorStreamContinuations.removeAll()
        errorHandlers.removeAll()

        for (_, continuation) in openStreamContinuations {
            continuation.finish()
        }
        openStreamContinuations.removeAll()
        openHandlers.removeAll()

        for (_, continuation) in closeStreamContinuations {
            continuation.finish()
        }
        closeStreamContinuations.removeAll()
        closeHandlers.removeAll()

        for (_, continuation) in statusStreamContinuations {
            continuation.finish()
        }
        statusStreamContinuations.removeAll()
        statusHandlers.removeAll()
    }

    private func connectLoop() async {
        var attempt = 0
        let delay: UInt64 = 500_000_000
        while status != .disposed {
            attempt += 1
            do {
                let query = await queryState.query
                logger.debug("resolving actor for connection attempt=\(attempt)")
                let actorId = try await manager.resolveActorId(for: query)
                self.actorId = actorId
                logger.debug("opening websocket actorId=\(actorId, privacy: .public)")
                let protocols = try buildWebSocketProtocols()
                let ws = try await manager.openWebSocket(actorId: actorId, path: Routing.pathConnect, protocols: protocols)
                self.websocket = ws
                logger.debug("websocket opened actorId=\(actorId, privacy: .public)")
                listenTask?.cancel()
                listenTask = Task { [weak self] in
                    await self?.listen(on: ws)
                }
                return
            } catch {
                logger.warning("failed to connect attempt=\(attempt) error=\(String(describing: error), privacy: .public)")
                if status != .disposed {
                    setStatus(.disconnected)
                }
                let backoff = min(delay * UInt64(attempt), 15_000_000_000)
                try? await Task.sleep(nanoseconds: backoff)
            }
        }
        logger.info("connection retry aborted")
    }

    private func listen(on task: URLSessionWebSocketTask) async {
        while status != .disposed {
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
            logger.trace("received string message len=\(text.count)")
            data = Data(text.utf8)
        case .data(let value):
            logger.trace("received data message len=\(value.count)")
            data = value
        @unknown default:
            logger.warning("received unknown message type")
            return
        }

        do {
            let decoded = try CBORWire.decodeToClientMessage(data)
            switch decoded.body {
            case .initMessage(let value):
                logger.trace("received init message actorId=\(value.actorId, privacy: .public) connId=\(value.connectionId, privacy: .public)")
                await handleInit(value)
            case .error(let value):
                logger.trace("received error message group=\(value.group, privacy: .public) code=\(value.code, privacy: .public)")
                await handleError(value)
            case .actionResponse(let value):
                logger.trace("received action response id=\(value.id)")
                await handleActionResponse(value)
            case .event(let value):
                logger.trace("received event name=\(value.name, privacy: .public)")
                await handleEvent(value)
            }
        } catch {
            logger.error("failed to parse message error=\(String(describing: error), privacy: .public)")
            await handleErrorMessage(group: "client", code: "malformed", message: "malformed response", metadata: nil, actionId: nil)
        }
    }

    private func handleInit(_ initMessage: InitMessage) async {
        actorId = initMessage.actorId
        connectionId = initMessage.connectionId
        logger.debug("connection initialized actorId=\(initMessage.actorId, privacy: .public) connId=\(initMessage.connectionId, privacy: .public)")
        setStatus(.connected)

        // Resubscribe to all events on reconnect
        for eventName in eventSubscriptions.keys {
            sendSubscription(eventName: eventName, subscribe: true)
        }

        // Flush queued messages
        let queued = messageQueue
        if !queued.isEmpty {
            logger.debug("flushing message queue queueLength=\(queued.count)")
        }
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
            logger.warning("action error actionId=\(actionId) group=\(group, privacy: .public) code=\(code, privacy: .public) message=\(message, privacy: .public)")
            if let continuation = actionsInFlight.removeValue(forKey: actionId) {
                logger.debug("removed action from in-flight map actionId=\(actionId) inFlightCount=\(self.actionsInFlight.count)")
                continuation.resume(throwing: actorError)
            } else {
                logger.error("action not found in in-flight map actionId=\(actionId) inFlightCount=\(self.actionsInFlight.count)")
            }
            return
        }

        logger.warning("connection error group=\(group, privacy: .public) code=\(code, privacy: .public) message=\(message, privacy: .public)")

        var errorToThrow: Error = actorError
        if isSchedulingError(group: group, code: code), let actorId {
            logger.info("found actor scheduling error actorId=\(actorId, privacy: .public)")
            let query = await queryState.query
            let name = query.name
            if let details = try? await manager.getActorError(actorId: actorId, name: name) {
                errorToThrow = ActorSchedulingError(group: group, code: code, actorId: actorId, details: details)
            } else {
                logger.warning("failed to fetch actor details for scheduling error check actorId=\(actorId, privacy: .public)")
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
            logger.error("action response not found in in-flight map actionId=\(response.id) inFlightCount=\(self.actionsInFlight.count)")
            return
        }
        logger.debug("removed action from in-flight map actionId=\(response.id) inFlightCount=\(self.actionsInFlight.count)")
        do {
            let data = try CBORSupport.encoder().encode(response.output)
            logger.debug("resolving action promise actionId=\(response.id)")
            continuation.resume(returning: data)
        } catch {
            logger.error("failed to encode action response actionId=\(response.id) error=\(String(describing: error), privacy: .public)")
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

    private static func decodeEventArgsZero(_ args: [JSONValue]) -> Result<Void, Error> {
        if args.isEmpty {
            return .success(())
        }
        return .failure(DecodeIssue.unexpectedArity(expected: 0, actual: args.count))
    }

    private static func decodeEventArgsSingle<T: Decodable>(
        _ args: [JSONValue],
        as _: T.Type
    ) -> Result<T, Error> {
        guard args.count == 1 else {
            return .failure(DecodeIssue.unexpectedArity(expected: 1, actual: args.count))
        }

        do {
            return .success(try decodeEventValue(args[0], as: T.self))
        } catch {
            return .failure(error)
        }
    }

    private static func decodeEventArgsPair<A: Decodable, B: Decodable>(
        _ args: [JSONValue],
        as _: (A, B).Type
    ) -> Result<(A, B), Error> {
        guard args.count == 2 else {
            return .failure(DecodeIssue.unexpectedArity(expected: 2, actual: args.count))
        }

        do {
            let first = try decodeEventValue(args[0], as: A.self)
            let second = try decodeEventValue(args[1], as: B.self)
            return .success((first, second))
        } catch {
            return .failure(error)
        }
    }

    private static func decodeEventArgsTriple<A: Decodable, B: Decodable, C: Decodable>(
        _ args: [JSONValue],
        as _: (A, B, C).Type
    ) -> Result<(A, B, C), Error> {
        guard args.count == 3 else {
            return .failure(DecodeIssue.unexpectedArity(expected: 3, actual: args.count))
        }

        do {
            let first = try decodeEventValue(args[0], as: A.self)
            let second = try decodeEventValue(args[1], as: B.self)
            let third = try decodeEventValue(args[2], as: C.self)
            return .success((first, second, third))
        } catch {
            return .failure(error)
        }
    }

    private static func decodeEventArgsQuad<A: Decodable, B: Decodable, C: Decodable, D: Decodable>(
        _ args: [JSONValue],
        as _: (A, B, C, D).Type
    ) -> Result<(A, B, C, D), Error> {
        guard args.count == 4 else {
            return .failure(DecodeIssue.unexpectedArity(expected: 4, actual: args.count))
        }

        do {
            let first = try decodeEventValue(args[0], as: A.self)
            let second = try decodeEventValue(args[1], as: B.self)
            let third = try decodeEventValue(args[2], as: C.self)
            let fourth = try decodeEventValue(args[3], as: D.self)
            return .success((first, second, third, fourth))
        } catch {
            return .failure(error)
        }
    }

    private static func decodeEventArgsQuint<A: Decodable, B: Decodable, C: Decodable, D: Decodable, E: Decodable>(
        _ args: [JSONValue],
        as _: (A, B, C, D, E).Type
    ) -> Result<(A, B, C, D, E), Error> {
        guard args.count == 5 else {
            return .failure(DecodeIssue.unexpectedArity(expected: 5, actual: args.count))
        }

        do {
            let first = try decodeEventValue(args[0], as: A.self)
            let second = try decodeEventValue(args[1], as: B.self)
            let third = try decodeEventValue(args[2], as: C.self)
            let fourth = try decodeEventValue(args[3], as: D.self)
            let fifth = try decodeEventValue(args[4], as: E.self)
            return .success((first, second, third, fourth, fifth))
        } catch {
            return .failure(error)
        }
    }

    private static func decodeEventValue<T: Decodable>(_ value: JSONValue, as _: T.Type) throws -> T {
        return try CBORSupport.decode(T.self, from: value)
    }

    private func emitDecodeError(eventName: String, error: Error) {
        let metadata: JSONValue = .object([
            "event": .string(eventName),
            "detail": .string(error.localizedDescription)
        ])
        let actorError = ActorError(
            group: "client",
            code: "decode_error",
            message: "failed to decode event \(eventName)",
            metadata: metadata
        )
        for (_, handler) in errorHandlers {
            handler(actorError)
        }
    }

    private func subscribe(
        _ eventName: String,
        once: Bool,
        handler: @escaping @Sendable ([JSONValue]) -> Void
    ) -> EventUnsubscribe {
        let id = UUID()
        let hadExisting = eventSubscriptions[eventName] != nil
        var handlers = eventSubscriptions[eventName] ?? [:]
        handlers[id] = EventSubscription(callback: handler, once: once)
        eventSubscriptions[eventName] = handlers
        if !hadExisting {
            sendSubscription(eventName: eventName, subscribe: true)
        }
        return { [weak self] in
            await self?.removeEventSubscription(eventName: eventName, id: id)
        }
    }

    private func handleClose(error: Error) async {
        if status == .disposed {
            logger.debug("websocket closed after dispose")
            rejectAllPending(error: ActorConnDisposed())
            return
        }
        logger.debug("websocket closed error=\(String(describing: error), privacy: .public)")
        let wasConnected = status == .connected
        setStatus(.disconnected)
        var errorToThrow: Error = error
        if wasConnected, let websocket, let reasonData = websocket.closeReason, let reason = String(data: reasonData, encoding: .utf8) {
            if let parsed = WebSocketCloseReasonParser.parse(reason) {
                let group = parsed.group
                let code = parsed.code
                logger.debug("parsed close reason group=\(group, privacy: .public) code=\(code, privacy: .public)")
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
            } else {
                logger.warning("failed to parse close reason reason=\(reason, privacy: .public)")
            }
        }

        rejectAllPending(error: errorToThrow)
        if let actorError = errorToThrow as? ActorError {
            for (_, handler) in errorHandlers {
                handler(actorError)
            }
        }
        logger.debug("triggering reconnect")
        connect()
    }

    private func sendMessage(_ message: ToServerMessage, ephemeral: Bool) {
        if status == .disposed {
            logger.debug("sendMessage called after dispose, ignoring")
            return
        }
        guard let websocket, status == .connected else {
            if !ephemeral {
                messageQueue.append(message)
                logger.debug("websocket not open, queueing message queueLength=\(self.messageQueue.count)")
            } else {
                logger.debug("no websocket connection, dropping ephemeral message")
            }
            return
        }

        Task { [weak self] in
            do {
                let data = try CBORWire.encodeToServerMessage(message)
                logger.trace("sending websocket message len=\(data.count)")
                try await websocket.send(.data(data))
                logger.trace("sent websocket message len=\(data.count)")
            } catch {
                logger.warning("websocket send failed error=\(String(describing: error), privacy: .public)")
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
        let previousStatus = status
        if previousStatus == newStatus {
            return
        }
        status = newStatus
        logger.debug("status change from=\(previousStatus.rawValue, privacy: .public) to=\(newStatus.rawValue, privacy: .public)")

        // Dispatch open handlers when transitioning to connected
        if newStatus == .connected {
            for (_, handler) in openHandlers {
                handler()
            }
        }

        // Dispatch close handlers when transitioning from connected to disconnected or disposed
        if previousStatus == .connected && (newStatus == .disconnected || newStatus == .disposed) {
            for (_, handler) in closeHandlers {
                handler()
            }
        }

        // Dispatch status change handlers
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
        errorStreamContinuations.removeValue(forKey: id)
    }

    private func removeOpenHandler(id: UUID) {
        openHandlers.removeValue(forKey: id)
        openStreamContinuations.removeValue(forKey: id)
    }

    private func removeCloseHandler(id: UUID) {
        closeHandlers.removeValue(forKey: id)
        closeStreamContinuations.removeValue(forKey: id)
    }

    private func removeStatusHandler(id: UUID) {
        statusHandlers.removeValue(forKey: id)
        statusStreamContinuations.removeValue(forKey: id)
    }

    private func buildWebSocketProtocols() throws -> [String] {
        var protocols: [String] = []
        protocols.append(Routing.wsProtocolStandard)
        protocols.append("\(Routing.wsProtocolEncoding)cbor")
        if let params = params {
            let data = try JSONEncoder().encode(params)
            let json = String(data: data, encoding: .utf8) ?? "{}"
            let encoded = URLUtils.encodeURIComponent(json)
            protocols.append("\(Routing.wsProtocolConnParams)\(encoded)")
        }
        return protocols
    }
}

private struct EventSubscription: Sendable {
    let callback: @Sendable ([JSONValue]) -> Void
    let once: Bool
}

private enum CBORWireError: Error {
    case invalidMessage(String)
}

/// Manual CBOR encoder/decoder for the websocket wire protocol.
// NOTE: Rivet's TS client uses the "bare" protocol that wraps CBOR-encoded payloads
// inside a CBOR map with tagged bignums for IDs. SwiftCBOR's Codable encoder does
// not emit those tags, so we build the wire shape manually to stay compatible with
// the server's parser and avoid immediate connection closes.
private enum CBORWire {
    static func encodeToServerMessage(_ message: ToServerMessage) throws -> Data {
        // Encode to the JSON-shaped protocol, but in CBOR, matching TS bare/cbor output.
        let body: CBOR
        switch message.body {
        case .actionRequest(let request):
            let args = try cborFromEncodable(request.args)
            let payload: [CBOR: CBOR] = [
                .utf8String("id"): encodeBignum(request.id),
                .utf8String("name"): .utf8String(request.name),
                .utf8String("args"): args
            ]
            body = .map([
                .utf8String("tag"): .utf8String("ActionRequest"),
                .utf8String("val"): .map(payload)
            ])
        case .subscriptionRequest(let request):
            let payload: [CBOR: CBOR] = [
                .utf8String("eventName"): .utf8String(request.eventName),
                .utf8String("subscribe"): .boolean(request.subscribe)
            ]
            body = .map([
                .utf8String("tag"): .utf8String("SubscriptionRequest"),
                .utf8String("val"): .map(payload)
            ])
        }
        let root = CBOR.map([
            .utf8String("body"): body
        ])
        return Data(root.encode())
    }

    static func decodeToClientMessage(_ data: Data) throws -> ToClientMessage {
        // Decode the JSON-shaped protocol from CBOR and translate into our Swift models.
        let decoded = try CBOR.decode(Array(data))
        guard let root = decoded else {
            throw CBORWireError.invalidMessage("missing root")
        }
        let rootMap = try mapValue(root)
        let bodyValue = try requireValue(rootMap, key: "body")
        let bodyMap = try mapValue(bodyValue)
        let tagValue = try requireValue(bodyMap, key: "tag")
        let tag = try stringValue(tagValue)
        let valValue = try requireValue(bodyMap, key: "val")

        switch tag {
        case "Init":
            let valMap = try mapValue(valValue)
            let actorId = try stringValue(try requireValue(valMap, key: "actorId"))
            let connectionId = try stringValue(try requireValue(valMap, key: "connectionId"))
            return ToClientMessage(body: .initMessage(InitMessage(actorId: actorId, connectionId: connectionId)))
        case "Error":
            let valMap = try mapValue(valValue)
            let group = try stringValue(try requireValue(valMap, key: "group"))
            let code = try stringValue(try requireValue(valMap, key: "code"))
            let message = try stringValue(try requireValue(valMap, key: "message"))
            let metadata: JSONValue?
            if let metadataValue = valMap[.utf8String("metadata")] {
                metadata = try optionalJSONValue(metadataValue)
            } else {
                metadata = nil
            }
            let actionId = try optionalUInt64(valMap[.utf8String("actionId")])
            return ToClientMessage(body: .error(ErrorMessage(group: group, code: code, message: message, metadata: metadata, actionId: actionId)))
        case "ActionResponse":
            let valMap = try mapValue(valValue)
            let id = try uint64Value(try requireValue(valMap, key: "id"))
            let output = try jsonValue(try requireValue(valMap, key: "output"))
            return ToClientMessage(body: .actionResponse(ActionResponseMessage(id: id, output: output)))
        case "Event":
            let valMap = try mapValue(valValue)
            let name = try stringValue(try requireValue(valMap, key: "name"))
            let args = try jsonValue(try requireValue(valMap, key: "args"))
            return ToClientMessage(body: .event(EventMessage(name: name, args: args)))
        default:
            throw CBORWireError.invalidMessage("unknown tag: \(tag)")
        }
    }

    private static func cborFromEncodable<T: Encodable>(_ value: T) throws -> CBOR {
        let data = try CBORSupport.encoder().encode(value)
        guard let decoded = try CBOR.decode(Array(data)) else {
            throw CBORWireError.invalidMessage("failed to encode payload")
        }
        return decoded
    }

    private static func mapValue(_ value: CBOR) throws -> [CBOR: CBOR] {
        guard case .map(let map) = value else {
            throw CBORWireError.invalidMessage("expected map")
        }
        return map
    }

    private static func requireValue(_ map: [CBOR: CBOR], key: String) throws -> CBOR {
        guard let value = map[.utf8String(key)] else {
            throw CBORWireError.invalidMessage("missing key: \(key)")
        }
        return value
    }

    private static func stringValue(_ value: CBOR) throws -> String {
        guard case .utf8String(let string) = value else {
            throw CBORWireError.invalidMessage("expected string")
        }
        return string
    }

    private static func uint64Value(_ value: CBOR) throws -> UInt64 {
        switch value {
        case .unsignedInt(let uint):
            return uint
        case .tagged(let tag, let inner) where tag == .positiveBignum:
            return try decodeBignum(inner)
        default:
            throw CBORWireError.invalidMessage("expected unsigned int")
        }
    }

    private static func optionalUInt64(_ value: CBOR?) throws -> UInt64? {
        guard let value else {
            return nil
        }
        if case .null = value {
            return nil
        }
        return try uint64Value(value)
    }

    private static func optionalJSONValue(_ value: CBOR) throws -> JSONValue? {
        if case .null = value {
            return nil
        }
        if case .undefined = value {
            return nil
        }
        return try jsonValue(value)
    }

    private static func jsonValue(_ value: CBOR) throws -> JSONValue {
        // Bridge CBOR primitives into JSONValue for typed event/action decoding.
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
        case .half(let value):
            return .number(.double(Double(value)))
        case .float(let value):
            return .number(.double(Double(value)))
        case .double(let value):
            return .number(.double(value))
        case .utf8String(let value):
            return .string(value)
        case .array(let values):
            return .array(try values.map { try jsonValue($0) })
        case .map(let values):
            var object: [String: JSONValue] = [:]
            for (key, value) in values {
                guard case .utf8String(let keyString) = key else {
                    throw CBORWireError.invalidMessage("non-string map key")
                }
                object[keyString] = try jsonValue(value)
            }
            return .object(object)
        case .tagged(let tag, let inner) where tag == .positiveBignum:
            let uint = try decodeBignum(inner)
            if uint <= UInt64(Int64.max) {
                return .number(.int(Int64(uint)))
            }
            return .number(.double(Double(uint)))
        default:
            throw CBORWireError.invalidMessage("unsupported CBOR value")
        }
    }

    private static func encodeBignum(_ value: UInt64) -> CBOR {
        // Rivet's protocol uses CBOR tag 2 (positive bignum) for IDs.
        if value == 0 {
            return .tagged(.positiveBignum, .byteString([]))
        }
        var bytes: [UInt8] = []
        var remaining = value
        while remaining > 0 {
            bytes.append(UInt8(remaining & 0xff))
            remaining >>= 8
        }
        bytes.reverse()
        return .tagged(.positiveBignum, .byteString(bytes))
    }

    private static func decodeBignum(_ value: CBOR) throws -> UInt64 {
        // Decode CBOR tag 2 (positive bignum) into UInt64 for action/event IDs.
        guard case .byteString(let bytes) = value else {
            throw CBORWireError.invalidMessage("expected bignum bytes")
        }
        if bytes.isEmpty {
            return 0
        }
        if bytes.count > MemoryLayout<UInt64>.size {
            throw CBORWireError.invalidMessage("bignum overflow")
        }
        var result: UInt64 = 0
        for byte in bytes {
            result = (result << 8) | UInt64(byte)
        }
        return result
    }
}

private enum DecodeIssue: Error, LocalizedError {
    case unexpectedArity(expected: Int, actual: Int)

    var errorDescription: String? {
        switch self {
        case .unexpectedArity(let expected, let actual):
            return "expected \(expected) args, received \(actual)"
        }
    }
}

private struct InitMessage: Decodable, Sendable {
    let actorId: String
    let connectionId: String
}

private struct ErrorMessage: Decodable, Sendable {
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
        actionId = try container.decodeIfPresent(UInt64.self, forKey: .actionId)
    }

    init(group: String, code: String, message: String, metadata: JSONValue?, actionId: UInt64?) {
        self.group = group
        self.code = code
        self.message = message
        self.metadata = metadata
        self.actionId = actionId
    }
}

private struct ActionResponseMessage: Decodable, Sendable {
    let id: UInt64
    let output: JSONValue

    enum CodingKeys: String, CodingKey {
        case id
        case output
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        id = try container.decode(UInt64.self, forKey: .id)
        output = try container.decode(JSONValue.self, forKey: .output)
    }

    init(id: UInt64, output: JSONValue) {
        self.id = id
        self.output = output
    }
}

private struct EventMessage: Decodable, Sendable {
    let name: String
    let args: JSONValue
}

private struct ToClientMessage: Decodable, Sendable {
    let body: ToClientBody
}

private enum ToClientBody: Decodable, Sendable {
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

private struct ActionRequest: Encodable, Sendable {
    let id: UInt64
    let name: String
    let args: [AnyEncodable]
}

private struct SubscriptionRequest: Encodable, Sendable {
    let eventName: String
    let subscribe: Bool
}

private struct ToServerMessage: Encodable, Sendable {
    let body: ToServerBody
}

private enum ToServerBody: Encodable, Sendable {
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
