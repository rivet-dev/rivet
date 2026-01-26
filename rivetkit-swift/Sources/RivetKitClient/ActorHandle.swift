import Foundation

public final class ActorHandle: @unchecked Sendable {
    // Safe because all mutable state is actor-isolated in ActorHandleState.
    private let manager: RemoteManager
    private let registry: ConnectionRegistry
    private let state: ActorHandleState
    private let params: AnyEncodable?

    init(manager: RemoteManager, registry: ConnectionRegistry, query: ActorQuery, params: AnyEncodable?) {
        self.manager = manager
        self.registry = registry
        self.state = ActorHandleState(query: query)
        self.params = params
    }

    public func action<Response: Decodable>(
        _ name: String,
        args: [AnyEncodable],
        as _: Response.Type = Response.self
    ) async throws -> Response {
        let actorId = try await resolveActorId()
        let encodedName = name.addingPercentEncoding(withAllowedCharacters: .urlPathAllowed) ?? name
        let path = "/action/\(encodedName)"
        let requestBody = HttpActionRequest(args: args)
        let bodyData = try JSONEncoder().encode(requestBody)

        var headers: [String: String] = [
            Routing.headerEncoding: "json",
            "Content-Type": "application/json"
        ]
        if let paramsHeader = try encodeParamsHeader() {
            headers[Routing.headerConnParams] = paramsHeader
        }

        let (data, response) = try await manager.sendHttpRequestToActor(
            actorId: actorId,
            path: path,
            method: "POST",
            headers: headers,
            body: bodyData
        )

        if !(200..<300).contains(response.statusCode) {
            throw try await parseActorError(data: data, actorId: actorId)
        }

        let decoded = try JSONDecoder().decode(HttpActionResponse<Response>.self, from: data)
        return decoded.output
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

    public func connect() -> ActorConnection {
        let connection = ActorConnection(
            manager: manager,
            registry: registry,
            queryState: state,
            params: params
        )
        Task {
            await registry.add(connection)
            await connection.connect()
        }
        return connection
    }

    public func resolve() async throws -> String {
        let (actorId, name) = try await resolveAndUpdateQuery()
        _ = name
        return actorId
    }

    public func getGatewayUrl() async throws -> URL {
        let actorId = try await resolveActorId()
        let config = await manager.currentConfig()
        return try URLUtils.buildActorGatewayUrl(
            endpoint: config.endpoint,
            actorId: actorId,
            token: config.token
        )
    }

    public func fetch(
        _ path: String,
        request: RawHTTPRequest = RawHTTPRequest()
    ) async throws -> RawHTTPResponse {
        let actorId = try await resolveActorId()
        let fragmentParts = path.split(separator: "#", maxSplits: 1, omittingEmptySubsequences: false)
        let pathWithoutFragment = String(fragmentParts.first ?? "")
        let normalizedPath = pathWithoutFragment.hasPrefix("/") ? String(pathWithoutFragment.dropFirst()) : pathWithoutFragment
        let fullPath = "/request/\(normalizedPath)"

        var headers = request.headers
        if let paramsHeader = try encodeParamsHeader() {
            headers[Routing.headerConnParams] = paramsHeader
        }

        let (data, response) = try await manager.sendHttpRequestToActor(
            actorId: actorId,
            path: fullPath,
            method: request.method,
            headers: headers,
            body: request.body
        )

        return RawHTTPResponse(data: data, response: response)
    }

    public func websocket(
        path: String? = nil
    ) async throws -> ActorWebSocket {
        let actorId = try await resolveActorId()
        let rawPath = path ?? ""
        let fragmentParts = rawPath.split(separator: "#", maxSplits: 1, omittingEmptySubsequences: false)
        let pathWithoutFragment = String(fragmentParts.first ?? "")
        let normalizedPath = pathWithoutFragment.hasPrefix("/") ? String(pathWithoutFragment.dropFirst()) : pathWithoutFragment
        let fullPath = "\(Routing.pathWebSocketPrefix)\(normalizedPath)"
        let protocols = try buildWebSocketProtocols()
        let task = try await manager.openWebSocket(actorId: actorId, path: fullPath, protocols: protocols)
        return ActorWebSocket(task: task)
    }

    private func resolveActorId() async throws -> String {
        let query = await state.query
        return try await manager.resolveActorId(for: query)
    }

    private func resolveAndUpdateQuery() async throws -> (String, String) {
        let query = await state.query
        let actorId = try await manager.resolveActorId(for: query)
        let name = query.name
        switch query {
        case .getForKey, .getOrCreateForKey:
            await state.update(query: .getForId(name: name, actorId: actorId))
        case .getForId:
            break
        case .create:
            break
        }
        return (actorId, name)
    }

    private func parseActorError(data: Data, actorId: String) async throws -> ActorError {
        if let error = try? JSONDecoder().decode(HttpResponseError.self, from: data) {
            if isSchedulingError(group: error.group, code: error.code) {
                let query = await state.query
                let name = query.name
                let details = try? await manager.getActorError(actorId: actorId, name: name)
                if let details {
                    throw ActorSchedulingError(group: error.group, code: error.code, actorId: actorId, details: details)
                }
            }
            return ActorError(group: error.group, code: error.code, message: error.message, metadata: error.metadata)
        }
        return ActorError(group: "http", code: "error", message: "request failed", metadata: nil)
    }

    private func encodeParamsHeader() throws -> String? {
        guard let params else {
            return nil
        }
        let data = try JSONEncoder().encode(params)
        return String(data: data, encoding: .utf8)
    }

    private func buildWebSocketProtocols() throws -> [String] {
        var protocols: [String] = []
        protocols.append(Routing.wsProtocolStandard)
        protocols.append("\(Routing.wsProtocolEncoding)json")
        if let paramsHeader = try encodeParamsHeader() {
            let encoded = URLUtils.encodeURIComponent(paramsHeader)
            protocols.append("\(Routing.wsProtocolConnParams)\(encoded)")
        }
        return protocols
    }
}

actor ActorHandleState {
    private(set) var query: ActorQuery

    init(query: ActorQuery) {
        self.query = query
    }

    func update(query: ActorQuery) {
        self.query = query
    }
}

struct HttpActionRequest: Encodable {
    let args: [AnyEncodable]
}

struct HttpActionResponse<T: Decodable>: Decodable {
    let output: T
}
