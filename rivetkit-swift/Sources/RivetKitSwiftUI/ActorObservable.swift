import Foundation
import Observation
import os
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
    public private(set) var opts: ActorOptions

    /// The hash identifying this actor. Thread-safe for `Identifiable` conformance.
    public var hash: String {
        get { _hashStorage.withLock { $0 } }
        set { _hashStorage.withLock { $0 = newValue } }
    }

    /// Lock-protected storage for hash to enable nonisolated Identifiable conformance.
    private let _hashStorage: OSAllocatedUnfairLock<String>

    public var isConnected: Bool { connStatus == .connected }

    /// Returns true if there's an active connection (connected or connecting).
    /// Used by the cache to determine if a staged update is needed.
    var hasActiveConnection: Bool {
        connection != nil && connStatus != .disposed && connStatus != .idle
    }

    private var client: RivetKitClient?
    private var contextError: ActorError?
    private var statusTask: Task<Void, Never>?
    private var errorTask: Task<Void, Never>?
    private var errorHandlers: [UUID: (ActorError) -> Void] = [:]
    private var connectionWaiters: [CheckedContinuation<ActorConnection, Error>] = []
    private var stagedOptions: ActorOptions?
    private var stagedContext: RivetKitContext?
    private var updateScheduled = false
    private var connectionGeneration: UInt64 = 0

    init(options: ActorOptions) {
        self.opts = options
        self._hashStorage = OSAllocatedUnfairLock(initialState: Self.computeHash(options))
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

    /// Performs an action with variadic typed arguments using parameter packs.
    public func action<each Arg: Encodable & Sendable, R: Decodable & Sendable>(
        _ name: String,
        _ args: repeat each Arg,
        as _: R.Type = R.self
    ) async throws -> R {
        var argsArray: [AnyEncodable] = []
        repeat argsArray.append(AnyEncodable(each args))
        return try await action(name, args: argsArray, as: R.self)
    }

    /// Raw JSON arguments for actions.
    public func action<R: Decodable & Sendable>(_ name: String, args: [JSONValue]) async throws -> R {
        let encoded = args.map { AnyEncodable($0) }
        return try await action(name, args: encoded, as: R.self)
    }

    /// Sends an action with variadic typed arguments, ignoring the response.
    public func send<each Arg: Encodable & Sendable>(_ name: String, _ args: repeat each Arg) {
        var argsArray: [AnyEncodable] = []
        repeat argsArray.append(AnyEncodable(each args))
        Task { [weak self] in
            _ = try? await self?.action(name, args: argsArray, as: JSONValue.self)
        }
    }

    /// Raw JSON arguments for send.
    public func send(_ name: String, args: [JSONValue]) {
        Task { [weak self] in
            let encoded = args.map { AnyEncodable($0) }
            _ = try? await self?.action(name, args: encoded, as: JSONValue.self)
        }
    }

    public func events<T: Decodable & Sendable>(_ name: String, as _: T.Type = T.self) -> AsyncStream<T> {
        let (stream, continuation) = AsyncStream<T>.makeStream()
        let eventName = name
        let forwardingTask = Task { [weak self] in
            guard let self else {
                continuation.finish()
                return
            }
            guard let connection = try? await waitForConnection() else {
                continuation.finish()
                return
            }
            let generation = connectionGeneration
            let connStream = await connection.events(eventName, as: T.self)
            for await value in connStream {
                guard !Task.isCancelled else { break }
                await MainActor.run { [weak self] in
                    guard let self, self.connectionGeneration == generation else { return }
                    continuation.yield(value)
                }
            }
            continuation.finish()
        }
        continuation.onTermination = { _ in
            forwardingTask.cancel()
        }
        return stream
    }

    public func events(_ name: String, as _: Void.Type = Void.self) -> AsyncStream<Void> {
        let (stream, continuation) = AsyncStream<Void>.makeStream()
        let eventName = name
        let forwardingTask = Task { [weak self] in
            guard let self else {
                continuation.finish()
                return
            }
            guard let connection = try? await waitForConnection() else {
                continuation.finish()
                return
            }
            let generation = connectionGeneration
            let connStream = await connection.events(eventName, as: Void.self)
            for await _ in connStream {
                guard !Task.isCancelled else { break }
                await MainActor.run { [weak self] in
                    guard let self, self.connectionGeneration == generation else { return }
                    continuation.yield(())
                }
            }
            continuation.finish()
        }
        continuation.onTermination = { _ in
            forwardingTask.cancel()
        }
        return stream
    }

    public func events<A: Decodable & Sendable, B: Decodable & Sendable>(
        _ name: String,
        as _: (A, B).Type
    ) -> AsyncStream<(A, B)> {
        let (stream, continuation) = AsyncStream<(A, B)>.makeStream()
        let eventName = name
        let forwardingTask = Task { [weak self] in
            guard let self else {
                continuation.finish()
                return
            }
            guard let connection = try? await waitForConnection() else {
                continuation.finish()
                return
            }
            let generation = connectionGeneration
            let connStream = await connection.events(eventName, as: (A, B).self)
            for await value in connStream {
                guard !Task.isCancelled else { break }
                await MainActor.run { [weak self] in
                    guard let self, self.connectionGeneration == generation else { return }
                    continuation.yield(value)
                }
            }
            continuation.finish()
        }
        continuation.onTermination = { _ in
            forwardingTask.cancel()
        }
        return stream
    }

    public func events<A: Decodable & Sendable, B: Decodable & Sendable, C: Decodable & Sendable>(
        _ name: String,
        as _: (A, B, C).Type
    ) -> AsyncStream<(A, B, C)> {
        let (stream, continuation) = AsyncStream<(A, B, C)>.makeStream()
        let eventName = name
        let forwardingTask = Task { [weak self] in
            guard let self else {
                continuation.finish()
                return
            }
            guard let connection = try? await waitForConnection() else {
                continuation.finish()
                return
            }
            let generation = connectionGeneration
            let connStream = await connection.events(eventName, as: (A, B, C).self)
            for await value in connStream {
                guard !Task.isCancelled else { break }
                await MainActor.run { [weak self] in
                    guard let self, self.connectionGeneration == generation else { return }
                    continuation.yield(value)
                }
            }
            continuation.finish()
        }
        continuation.onTermination = { _ in
            forwardingTask.cancel()
        }
        return stream
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
        let (stream, continuation) = AsyncStream<(A, B, C, D)>.makeStream()
        let eventName = name
        let forwardingTask = Task { [weak self] in
            guard let self else {
                continuation.finish()
                return
            }
            guard let connection = try? await waitForConnection() else {
                continuation.finish()
                return
            }
            let generation = connectionGeneration
            let connStream = await connection.events(eventName, as: (A, B, C, D).self)
            for await value in connStream {
                guard !Task.isCancelled else { break }
                await MainActor.run { [weak self] in
                    guard let self, self.connectionGeneration == generation else { return }
                    continuation.yield(value)
                }
            }
            continuation.finish()
        }
        continuation.onTermination = { _ in
            forwardingTask.cancel()
        }
        return stream
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
        let (stream, continuation) = AsyncStream<(A, B, C, D, E)>.makeStream()
        let eventName = name
        let forwardingTask = Task { [weak self] in
            guard let self else {
                continuation.finish()
                return
            }
            guard let connection = try? await waitForConnection() else {
                continuation.finish()
                return
            }
            let generation = connectionGeneration
            let connStream = await connection.events(eventName, as: (A, B, C, D, E).self)
            for await value in connStream {
                guard !Task.isCancelled else { break }
                await MainActor.run { [weak self] in
                    guard let self, self.connectionGeneration == generation else { return }
                    continuation.yield(value)
                }
            }
            continuation.finish()
        }
        continuation.onTermination = { _ in
            forwardingTask.cancel()
        }
        return stream
    }

    /// Raw JSON event arguments. Use this when you need more than 5 positional arguments.
    public func events(_ name: String) -> AsyncStream<[JSONValue]> {
        let (stream, continuation) = AsyncStream<[JSONValue]>.makeStream()
        let eventName = name
        let forwardingTask = Task { [weak self] in
            guard let self else {
                continuation.finish()
                return
            }
            guard let connection = try? await waitForConnection() else {
                continuation.finish()
                return
            }
            let generation = connectionGeneration
            let connStream = await connection.events(eventName)
            for await value in connStream {
                guard !Task.isCancelled else { break }
                await MainActor.run { [weak self] in
                    guard let self, self.connectionGeneration == generation else { return }
                    continuation.yield(value)
                }
            }
            continuation.finish()
        }
        continuation.onTermination = { _ in
            forwardingTask.cancel()
        }
        return stream
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
        statusTask?.cancel()
        errorTask?.cancel()

        // Sync current status immediately.
        let currentStatus = await connection.currentStatus
        if connectionGeneration == generation && connection === self.connection {
            connStatus = currentStatus
            if currentStatus == .connected {
                error = nil
            }
        }

        let statusStream = await connection.statusChanges()
        statusTask = Task { [weak self] in
            for await status in statusStream {
                guard !Task.isCancelled else { break }
                await MainActor.run { [weak self] in
                    guard let self,
                          self.connectionGeneration == generation,
                          self.connection === connection else { return }
                    self.connStatus = status
                    if status == .connected {
                        self.error = nil
                    }
                }
            }
        }

        let errorStream = await connection.errors()
        errorTask = Task { [weak self] in
            for await error in errorStream {
                guard !Task.isCancelled else { break }
                await MainActor.run { [weak self] in
                    guard let self,
                          self.connectionGeneration == generation,
                          self.connection === connection else { return }
                    self.emitError(error)
                }
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

        statusTask?.cancel()
        statusTask = nil

        errorTask?.cancel()
        errorTask = nil

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

extension ActorObservable: Identifiable {
    /// The unique identifier for this actor, based on its hash.
    /// Thread-safe access via OSAllocatedUnfairLock.
    public nonisolated var id: String {
        _hashStorage.withLock { $0 }
    }
}
