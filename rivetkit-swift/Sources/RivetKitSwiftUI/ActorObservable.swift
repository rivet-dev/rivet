import Foundation
import Observation
import RivetKitClient
import SwiftCBOR

/// Error thrown when waiting for a connection that was disposed.
public struct ActorConnectionDisposed: Error, Sendable {
    public init() {}
}

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
@Observable
public final class ActorObservable {
    public private(set) var connStatus: ActorConnStatus = .idle
    public private(set) var error: ActorError?
    public private(set) var connection: ActorConnection?
    public private(set) var handle: ActorHandle?
    public private(set) var hash: String
    public private(set) var opts: ActorOptions

    public var isConnected: Bool { connStatus == .connected }

    /// Returns true if there's an active connection (connected or connecting).
    /// Used by the cache to determine if a staged update is needed.
    var hasActiveConnection: Bool {
        connection != nil && connStatus != .disposed && connStatus != .idle
    }

    private var client: RivetKitClient?
    private var contextError: ActorError?
    private var statusUnsubscribe: EventUnsubscribe?
    private var errorUnsubscribe: EventUnsubscribe?
    private var errorHandlers: [UUID: (ActorError) -> Void] = [:]
    private var connectionWaiters: [CheckedContinuation<ActorConnection, Error>] = []
    private var stagedOptions: ActorOptions?
    private var stagedContext: RivetKitContext?
    private var updateScheduled = false
    private var connectionGeneration: UInt64 = 0

    init(options: ActorOptions) {
        self.opts = options
        self.hash = Self.computeHash(options)
    }

    func stage(context: RivetKitContext, options: ActorOptions) {
        stagedOptions = options
        stagedContext = context
        // Note: Do NOT set client here. Setting client immediately can cause race conditions
        // when multiple scheduleUpdate calls happen before runStagedUpdate runs. The client
        // comparison in update() would then compare against the wrong value. Instead, let
        // update() handle client assignment after properly determining if a reset is needed.
        // The action() method already falls back to stagedContext.client for early actions.
        contextError = context.error
    }

    func scheduleUpdate(context: RivetKitContext, options: ActorOptions) {
        stage(context: context, options: options)
        if updateScheduled {
            return
        }
        updateScheduled = true
        Task { @MainActor [weak self] in
            await Task.yield()
            self?.runStagedUpdate()
        }
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

    private func runStagedUpdate() {
        updateScheduled = false
        guard let stagedContext, let stagedOptions else {
            return
        }
        let context = stagedContext
        let options = stagedOptions
        self.stagedContext = nil
        self.stagedOptions = nil
        update(context: context, options: options)
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

    func connectionToken() -> ObjectIdentifier? {
        connection.map { ObjectIdentifier($0) }
    }

    /// Convenience overloads for 0-5 positional arguments.
    /// Prefer these over the raw array-based form to keep decoding strongly typed.
    public func action<R: Decodable & Sendable>(_ name: String) async throws -> R {
        try await action(name, args: [] as [AnyEncodable], as: R.self)
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

    public func action<A: Encodable, B: Encodable, C: Encodable, D: Encodable, R: Decodable & Sendable>(
        _ name: String,
        _ a: A,
        _ b: B,
        _ c: C,
        _ d: D
    ) async throws -> R {
        try await action(
            name,
            args: [AnyEncodable(a), AnyEncodable(b), AnyEncodable(c), AnyEncodable(d)],
            as: R.self
        )
    }

    public func action<A: Encodable, B: Encodable, C: Encodable, D: Encodable, E: Encodable, R: Decodable & Sendable>(
        _ name: String,
        _ a: A,
        _ b: B,
        _ c: C,
        _ d: D,
        _ e: E
    ) async throws -> R {
        try await action(
            name,
            args: [AnyEncodable(a), AnyEncodable(b), AnyEncodable(c), AnyEncodable(d), AnyEncodable(e)],
            as: R.self
        )
    }

    public func action<R: Decodable & Sendable>(_ name: String, args: [any Encodable]) async throws -> R {
        let encoded = args.map { AnyEncodable($0) }
        return try await action(name, args: encoded, as: R.self)
    }

    /// Raw JSON arguments for actions. Use this when you need more than 5 positional arguments.
    public func action<R: Decodable & Sendable>(_ name: String, args: [JSONValue]) async throws -> R {
        let encoded = args.map { AnyEncodable($0) }
        return try await action(name, args: encoded, as: R.self)
    }

    public func send(_ name: String) {
        Task { [weak self] in
            _ = try? await self?.action(name, args: [] as [AnyEncodable], as: JSONValue.self)
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

    public func send<A: Encodable, B: Encodable, C: Encodable, D: Encodable>(
        _ name: String,
        _ a: A,
        _ b: B,
        _ c: C,
        _ d: D
    ) {
        Task { [weak self] in
            _ = try? await self?.action(
                name,
                args: [AnyEncodable(a), AnyEncodable(b), AnyEncodable(c), AnyEncodable(d)],
                as: JSONValue.self
            )
        }
    }

    public func send<A: Encodable, B: Encodable, C: Encodable, D: Encodable, E: Encodable>(
        _ name: String,
        _ a: A,
        _ b: B,
        _ c: C,
        _ d: D,
        _ e: E
    ) {
        Task { [weak self] in
            _ = try? await self?.action(
                name,
                args: [AnyEncodable(a), AnyEncodable(b), AnyEncodable(c), AnyEncodable(d), AnyEncodable(e)],
                as: JSONValue.self
            )
        }
    }

    public func send(_ name: String, args: [any Encodable]) {
        let encoded = args.map { AnyEncodable($0) }
        Task { [weak self] in
            _ = try? await self?.action(name, args: encoded, as: JSONValue.self)
        }
    }

    /// Raw JSON arguments for actions. Use this when you need more than 5 positional arguments.
    public func send(_ name: String, args: [JSONValue]) {
        Task { [weak self] in
            let encoded = args.map { AnyEncodable($0) }
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
                guard let connection = try? await waitForConnection() else {
                    continuation.finish()
                    return
                }
                let generation = connectionGeneration
                let unsubscribe = await connection.on(name) { [weak self] (value: T) in
                    Task { @MainActor in
                        guard let self, self.connectionGeneration == generation else { return }
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

    public func events(_ name: String, as _: Void.Type = Void.self) -> AsyncStream<Void> {
        AsyncStream { continuation in
            Task { [weak self] in
                guard let self else {
                    continuation.finish()
                    return
                }
                guard let connection = try? await waitForConnection() else {
                    continuation.finish()
                    return
                }
                let generation = connectionGeneration
                let unsubscribe = await connection.on(name) { [weak self] in
                    Task { @MainActor in
                        guard let self, self.connectionGeneration == generation else { return }
                        continuation.yield(())
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

    public func events<A: Decodable & Sendable, B: Decodable & Sendable>(
        _ name: String,
        as _: (A, B).Type
    ) -> AsyncStream<(A, B)> {
        AsyncStream { continuation in
            Task { [weak self] in
                guard let self else {
                    continuation.finish()
                    return
                }
                guard let connection = try? await waitForConnection() else {
                    continuation.finish()
                    return
                }
                let generation = connectionGeneration
                let unsubscribe = await connection.on(name) { [weak self] (first: A, second: B) in
                    Task { @MainActor in
                        guard let self, self.connectionGeneration == generation else { return }
                        continuation.yield((first, second))
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

    public func events<A: Decodable & Sendable, B: Decodable & Sendable, C: Decodable & Sendable>(
        _ name: String,
        as _: (A, B, C).Type
    ) -> AsyncStream<(A, B, C)> {
        AsyncStream { continuation in
            Task { [weak self] in
                guard let self else {
                    continuation.finish()
                    return
                }
                guard let connection = try? await waitForConnection() else {
                    continuation.finish()
                    return
                }
                let generation = connectionGeneration
                let unsubscribe = await connection.on(name) { [weak self] (first: A, second: B, third: C) in
                    Task { @MainActor in
                        guard let self, self.connectionGeneration == generation else { return }
                        continuation.yield((first, second, third))
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

    public func events<
        A: Decodable & Sendable,
        B: Decodable & Sendable,
        C: Decodable & Sendable,
        D: Decodable & Sendable
    >(
        _ name: String,
        as _: (A, B, C, D).Type
    ) -> AsyncStream<(A, B, C, D)> {
        AsyncStream { continuation in
            Task { [weak self] in
                guard let self else {
                    continuation.finish()
                    return
                }
                guard let connection = try? await waitForConnection() else {
                    continuation.finish()
                    return
                }
                let generation = connectionGeneration
                let unsubscribe = await connection.on(name) { [weak self] (first: A, second: B, third: C, fourth: D) in
                    Task { @MainActor in
                        guard let self, self.connectionGeneration == generation else { return }
                        continuation.yield((first, second, third, fourth))
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
        AsyncStream { continuation in
            Task { [weak self] in
                guard let self else {
                    continuation.finish()
                    return
                }
                guard let connection = try? await waitForConnection() else {
                    continuation.finish()
                    return
                }
                let generation = connectionGeneration
                let unsubscribe = await connection.on(name) { [weak self] (first: A, second: B, third: C, fourth: D, fifth: E) in
                    Task { @MainActor in
                        guard let self, self.connectionGeneration == generation else { return }
                        continuation.yield((first, second, third, fourth, fifth))
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

    @available(*, deprecated, message: "use typed events overloads instead of raw JSON values")
    public func events(_ name: String) -> AsyncStream<[JSONValue]> {
        AsyncStream { continuation in
            Task { [weak self] in
                guard let self else {
                    continuation.finish()
                    return
                }
                guard let connection = try? await waitForConnection() else {
                    continuation.finish()
                    return
                }
                let generation = connectionGeneration
                let unsubscribe = await connection.on(name) { [weak self] args in
                    Task { @MainActor in
                        guard let self, self.connectionGeneration == generation else { return }
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
        if let stagedContext, let stagedError = stagedContext.error {
            throw stagedError
        }
        if let stagedOptions, let stagedContext, let stagedClient = stagedContext.client, stagedOptions.enabled {
            let handle = buildHandle(client: stagedClient, options: stagedOptions)
            self.handle = handle
            return try await handle.action(name, args: args, as: R.self)
        }
        if let client, opts.enabled {
            let handle = buildHandle(client: client, options: opts)
            self.handle = handle
            return try await handle.action(name, args: args, as: R.self)
        }
        if let contextError {
            throw contextError
        }
        throw ActorError.clientError(code: "not_ready", message: "actor connection is not ready")
    }

    private func buildHandle(client: RivetKitClient, options: ActorOptions) -> ActorHandle {
        client.getOrCreate(
            options.name,
            options.key,
            options: GetOrCreateOptions(
                params: options.params,
                createInRegion: options.createInRegion,
                createWithInput: options.createWithInput
            )
        )
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

        connectionGeneration &+= 1
        let generation = connectionGeneration

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
        resumeConnectionWaiters(connection)

        Task { [weak self] in
            await self?.attachConnectionHandlers(connection, generation: generation)
        }
    }

    private func attachConnectionHandlers(_ connection: ActorConnection, generation: UInt64) async {
        if let statusUnsubscribe {
            await statusUnsubscribe()
        }
        if let errorUnsubscribe {
            await errorUnsubscribe()
        }

        statusUnsubscribe = await connection.onStatusChange { [weak self] status in
            Task { @MainActor in
                guard let self,
                      self.connectionGeneration == generation,
                      self.connection === connection else { return }
                self.connStatus = status
                if status == .connected {
                    self.error = nil
                }
            }
        }

        errorUnsubscribe = await connection.onError { [weak self] error in
            Task { @MainActor in
                guard let self,
                      self.connectionGeneration == generation,
                      self.connection === connection else { return }
                self.emitError(error)
            }
        }

        // Sync current status after subscribing.
        // The onStatusChange callback fires immediately (BehaviorSubject pattern),
        // but it wraps the update in a Task which may not run before the view renders.
        // This explicit sync ensures the UI sees the correct status immediately.
        let currentStatus = await connection.getStatus()
        if connectionGeneration == generation && connection === self.connection {
            connStatus = currentStatus
            if currentStatus == .connected {
                error = nil
            }
        }
    }

    private func disposeConnection(setIdle: Bool) {
        connectionGeneration &+= 1
        failConnectionWaiters()

        let connection = self.connection
        self.connection = nil
        self.handle = nil
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

    private func waitForConnection() async throws -> ActorConnection {
        if let connection {
            return connection
        }

        return try await withCheckedThrowingContinuation { continuation in
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

    private func failConnectionWaiters() {
        let waiters = connectionWaiters
        connectionWaiters.removeAll()
        for waiter in waiters {
            waiter.resume(throwing: ActorConnectionDisposed())
        }
    }

    /// Computes a hash for the given actor options.
    /// This hash is used to deduplicate actor connections in the cache.
    public static func computeHash(_ options: ActorOptions) -> String {
        // Use simple string concatenation for deterministic hashing.
        // CBOR encoding was causing non-deterministic results.
        var components = [options.name]
        components.append(contentsOf: options.key)
        if let params = options.params {
            let encoder = CodableCBOREncoder()
            encoder.useStringKeys = true
            if let data = try? encoder.encode(params) {
                components.append(data.base64EncodedString())
            }
        }
        return components.joined(separator: ":")
    }

    /// Disposes the connection and cleans up resources.
    /// Called by the cache when the ref count reaches 0.
    public func dispose() {
        disposeConnection(setIdle: true)
    }

    /// Runs a staged update if one is pending.
    /// Called by the cache after mount to trigger connection creation.
    public func runStagedUpdateIfNeeded() {
        // Don't run if there's already an active connection - prevents
        // needsReset from disposing an existing good connection.
        guard !hasActiveConnection else { return }
        if stagedContext != nil && stagedOptions != nil {
            runStagedUpdate()
        }
    }
}

