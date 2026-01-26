import Foundation
import RivetKitClient

public struct ActorOptions: Sendable {
    public let name: String
    public let key: [String]
    public let params: AnyEncodable?
    public let createWithInput: AnyEncodable?
    public let createInRegion: String?
    public let enabled: Bool

    public init(
        name: String,
        key: [String],
        params: AnyEncodable?,
        createWithInput: AnyEncodable?,
        createInRegion: String?,
        enabled: Bool
    ) {
        self.name = name
        self.key = key
        self.params = params
        self.createWithInput = createWithInput
        self.createInRegion = createInRegion
        self.enabled = enabled
    }
}

@MainActor
public final class ActorObservable: ObservableObject {
    @Published public private(set) var connStatus: ActorConnStatus = .idle
    @Published public private(set) var error: ActorError?
    @Published public private(set) var connection: ActorConnection?
    @Published public private(set) var handle: ActorHandle?
    @Published public private(set) var hash: String
    @Published public private(set) var opts: ActorOptions

    public var isConnected: Bool { connStatus == .connected }

    private var client: RivetKitClient?
    private var contextError: ActorError?
    private var connectionId: UUID?
    private var statusUnsubscribe: EventUnsubscribe?
    private var errorUnsubscribe: EventUnsubscribe?
    private var errorHandlers: [UUID: (ActorError) -> Void] = [:]
    private var connectionWaiters: [CheckedContinuation<ActorConnection, Never>] = []

    init(options: ActorOptions) {
        self.opts = options
        self.hash = Self.computeHash(options)
    }

    func update(context: RivetKitContext, options: ActorOptions) {
        let newHash = Self.computeHash(options)
        let clientChanged: Bool
        switch (client, context.client) {
        case (nil, nil):
            clientChanged = false
        case (nil, _), (_, nil):
            clientChanged = true
        case (let current?, let updated?):
            clientChanged = current !== updated
        }

        let needsReset = newHash != hash || clientChanged
        let enabledChanged = options.enabled != opts.enabled

        if context.error != contextError {
            contextError = context.error
            if let error = context.error {
                emitError(error)
            }
        }

        if needsReset {
            disposeConnection(setIdle: true)
            opts = options
            hash = newHash
            client = context.client
            if options.enabled {
                createConnectionIfPossible()
            }
            return
        }

        if enabledChanged {
            opts = options
            if options.enabled {
                createConnectionIfPossible()
            } else {
                disposeConnection(setIdle: true)
            }
            return
        }

        opts = options
        if opts.enabled && connection == nil {
            createConnectionIfPossible()
        }
    }

    func addErrorHandler(_ handler: @escaping (ActorError) -> Void) -> UUID {
        let id = UUID()
        errorHandlers[id] = handler
        if let error {
            handler(error)
        }
        return id
    }

    func removeErrorHandler(id: UUID) {
        errorHandlers.removeValue(forKey: id)
    }

    func reportDecodeError(eventName: String, error: Error) {
        let metadata: JSONValue = .object([
            "event": .string(eventName),
            "detail": .string(error.localizedDescription)
        ])
        emitError(ActorError.clientError(code: "decode_error", message: "failed to decode event \(eventName)", metadata: metadata))
    }

    func connectionToken() -> UUID? {
        connectionId
    }

    public func action<R: Decodable & Sendable>(_ name: String) async throws -> R {
        try await action(name, args: [], as: R.self)
    }

    public func action<A: Encodable, R: Decodable & Sendable>(_ name: String, _ a: A) async throws -> R {
        try await action(name, args: [AnyEncodable(a)], as: R.self)
    }

    public func action<A: Encodable, B: Encodable, R: Decodable & Sendable>(_ name: String, _ a: A, _ b: B) async throws -> R {
        try await action(name, args: [AnyEncodable(a), AnyEncodable(b)], as: R.self)
    }

    public func action<A: Encodable, B: Encodable, C: Encodable, R: Decodable & Sendable>(
        _ name: String,
        _ a: A,
        _ b: B,
        _ c: C
    ) async throws -> R {
        try await action(name, args: [AnyEncodable(a), AnyEncodable(b), AnyEncodable(c)], as: R.self)
    }

    public func action<R: Decodable & Sendable>(_ name: String, args: [any Encodable]) async throws -> R {
        let encoded = args.map { AnyEncodable($0) }
        return try await action(name, args: encoded, as: R.self)
    }

    public func send(_ name: String) {
        Task { [weak self] in
            _ = try? await self?.action(name, args: [], as: JSONValue.self)
        }
    }

    public func send<A: Encodable>(_ name: String, _ a: A) {
        Task { [weak self] in
            _ = try? await self?.action(name, args: [AnyEncodable(a)], as: JSONValue.self)
        }
    }

    public func send<A: Encodable, B: Encodable>(_ name: String, _ a: A, _ b: B) {
        Task { [weak self] in
            _ = try? await self?.action(name, args: [AnyEncodable(a), AnyEncodable(b)], as: JSONValue.self)
        }
    }

    public func send<A: Encodable, B: Encodable, C: Encodable>(
        _ name: String,
        _ a: A,
        _ b: B,
        _ c: C
    ) {
        Task { [weak self] in
            _ = try? await self?.action(name, args: [AnyEncodable(a), AnyEncodable(b), AnyEncodable(c)], as: JSONValue.self)
        }
    }

    public func send(_ name: String, args: [any Encodable]) {
        let encoded = args.map { AnyEncodable($0) }
        Task { [weak self] in
            _ = try? await self?.action(name, args: encoded, as: JSONValue.self)
        }
    }

    public func events<T: Decodable & Sendable>(_ name: String, as _: T.Type = T.self) -> AsyncStream<T> {
        AsyncStream { continuation in
            Task { [weak self] in
                guard let self else {
                    continuation.finish()
                    return
                }
                let connection = await waitForConnection()
                let unsubscribe = await connection.on(name) { [weak self] args in
                    Task { @MainActor in
                        guard let self else { return }
                        let decoder = EventDecoder(actor: self, eventName: name)
                        guard let value = decoder.decodeSingle(args: args, as: T.self) else { return }
                        continuation.yield(value)
                    }
                }
                continuation.onTermination = { _ in
                    Task {
                        await unsubscribe()
                    }
                }
            }
        }
    }

    public func events(_ name: String) -> AsyncStream<[JSONValue]> {
        AsyncStream { continuation in
            Task { [weak self] in
                guard let self else {
                    continuation.finish()
                    return
                }
                let connection = await waitForConnection()
                let unsubscribe = await connection.on(name) { args in
                    Task { @MainActor in
                        continuation.yield(args)
                    }
                }
                continuation.onTermination = { _ in
                    Task {
                        await unsubscribe()
                    }
                }
            }
        }
    }

    private func action<R: Decodable & Sendable>(_ name: String, args: [AnyEncodable], as _: R.Type) async throws -> R {
        if let connection {
            return try await connection.action(name, args: args, as: R.self)
        }
        if let handle {
            return try await handle.action(name, args: args, as: R.self)
        }
        if let contextError {
            throw contextError
        }
        throw ActorError.clientError(code: "not_ready", message: "actor connection is not ready")
    }

    private func createConnectionIfPossible() {
        guard opts.enabled else { return }
        guard let client else {
            if let contextError {
                emitError(contextError)
                connStatus = .disconnected
            }
            return
        }

        error = nil
        connStatus = .connecting

        let handle = client.getOrCreate(
            opts.name,
            opts.key,
            options: GetOrCreateOptions(
                params: opts.params,
                createInRegion: opts.createInRegion,
                createWithInput: opts.createWithInput
            )
        )
        self.handle = handle

        let connection = handle.connect()
        self.connection = connection
        connectionId = UUID()
        resumeConnectionWaiters(connection)

        Task { [weak self] in
            await self?.attachConnectionHandlers(connection)
        }
    }

    private func attachConnectionHandlers(_ connection: ActorConnection) async {
        if let statusUnsubscribe {
            await statusUnsubscribe()
        }
        if let errorUnsubscribe {
            await errorUnsubscribe()
        }

        statusUnsubscribe = await connection.onStatusChange { [weak self] status in
            Task { @MainActor in
                guard let self, self.connection === connection else { return }
                self.connStatus = status
                if status == .connected {
                    self.error = nil
                }
            }
        }

        errorUnsubscribe = await connection.onError { [weak self] error in
            Task { @MainActor in
                guard let self, self.connection === connection else { return }
                self.emitError(error)
            }
        }
    }

    private func disposeConnection(setIdle: Bool) {
        let connection = self.connection
        self.connection = nil
        self.handle = nil
        connectionId = nil
        if setIdle {
            connStatus = .idle
        }

        if let statusUnsubscribe {
            Task {
                await statusUnsubscribe()
            }
        }
        statusUnsubscribe = nil

        if let errorUnsubscribe {
            Task {
                await errorUnsubscribe()
            }
        }
        errorUnsubscribe = nil

        if let connection {
            Task {
                await connection.dispose()
            }
        }
    }

    private func emitError(_ error: ActorError) {
        self.error = error
        for handler in errorHandlers.values {
            handler(error)
        }
    }

    private func waitForConnection() async -> ActorConnection {
        if let connection {
            return connection
        }

        return await withCheckedContinuation { continuation in
            connectionWaiters.append(continuation)
        }
    }

    private func resumeConnectionWaiters(_ connection: ActorConnection) {
        let waiters = connectionWaiters
        connectionWaiters.removeAll()
        for waiter in waiters {
            waiter.resume(returning: connection)
        }
    }

    private static func computeHash(_ options: ActorOptions) -> String {
        let payload = ActorHashPayload(
            name: options.name,
            key: options.key,
            params: options.params
        )
        if let data = try? JSONEncoder().encode(payload),
           let json = String(data: data, encoding: .utf8) {
            return json
        }
        return UUID().uuidString
    }
}

private struct ActorHashPayload: Encodable {
    let name: String
    let key: [String]
    let params: AnyEncodable?
}
