import Foundation

public struct GetWithIdOptions: Sendable {
    public var params: AnyEncodable?

    public init(params: Encodable? = nil) {
        if let params {
            self.params = AnyEncodable(params)
        } else {
            self.params = nil
        }
    }
}

public struct GetOptions: Sendable {
    public var params: AnyEncodable?

    public init(params: Encodable? = nil) {
        if let params {
            self.params = AnyEncodable(params)
        } else {
            self.params = nil
        }
    }
}

public struct GetOrCreateOptions: Sendable {
    public var params: AnyEncodable?
    public var createInRegion: String?
    public var createWithInput: AnyEncodable?

    public init(
        params: Encodable? = nil,
        createInRegion: String? = nil,
        createWithInput: Encodable? = nil
    ) {
        if let params {
            self.params = AnyEncodable(params)
        } else {
            self.params = nil
        }
        self.createInRegion = createInRegion
        if let createWithInput {
            self.createWithInput = AnyEncodable(createWithInput)
        } else {
            self.createWithInput = nil
        }
    }
}

public struct CreateOptions: Sendable {
    public var params: AnyEncodable?
    public var region: String?
    public var input: AnyEncodable?

    public init(
        params: Encodable? = nil,
        region: String? = nil,
        input: Encodable? = nil
    ) {
        if let params {
            self.params = AnyEncodable(params)
        } else {
            self.params = nil
        }
        self.region = region
        if let input {
            self.input = AnyEncodable(input)
        } else {
            self.input = nil
        }
    }
}

actor ConnectionRegistry {
    private var connections: [ObjectIdentifier: ActorConnection] = [:]

    func add(_ connection: ActorConnection) {
        connections[ObjectIdentifier(connection)] = connection
    }

    func remove(_ connection: ActorConnection) {
        connections.removeValue(forKey: ObjectIdentifier(connection))
    }

    func disposeAll() async {
        let current = connections.values
        for connection in current {
            await connection.dispose()
        }
        connections.removeAll()
    }
}

public final class RivetKitClient: @unchecked Sendable {
    // Safe because this type is immutable and all mutable state is actor-isolated.
    private let manager: RemoteManager
    private let registry: ConnectionRegistry

    public init(config: ClientConfig) {
        self.manager = RemoteManager(config: config)
        self.registry = ConnectionRegistry()
        Task { [manager] in
            await manager.startMetadataLookupIfNeeded()
        }
    }

    public convenience init() throws {
        try self.init(config: ClientConfig())
    }

    public func get(_ name: String, _ key: [String] = [], options: GetOptions = GetOptions()) -> ActorHandle {
        return ActorHandle(
            manager: manager,
            registry: registry,
            query: .getForKey(name: name, key: key),
            params: options.params
        )
    }

    public func getOrCreate(
        _ name: String,
        _ key: [String] = [],
        options: GetOrCreateOptions = GetOrCreateOptions()
    ) -> ActorHandle {
        return ActorHandle(
            manager: manager,
            registry: registry,
            query: .getOrCreateForKey(
                name: name,
                key: key,
                input: options.createWithInput,
                region: options.createInRegion
            ),
            params: options.params
        )
    }

    public func getForId(
        _ name: String,
        _ actorId: String,
        options: GetWithIdOptions = GetWithIdOptions()
    ) -> ActorHandle {
        return ActorHandle(
            manager: manager,
            registry: registry,
            query: .getForId(name: name, actorId: actorId),
            params: options.params
        )
    }

    public func create(
        _ name: String,
        _ key: [String] = [],
        options: CreateOptions = CreateOptions()
    ) async throws -> ActorHandle {
        let actor = try await manager.createActor(
            name: name,
            key: key.isEmpty ? nil : key,
            input: options.input,
            region: options.region
        )
        return ActorHandle(
            manager: manager,
            registry: registry,
            query: .getForId(name: name, actorId: actor.actorId),
            params: options.params
        )
    }

    public func dispose() async {
        await registry.disposeAll()
    }
}
