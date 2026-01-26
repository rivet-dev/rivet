import Foundation
import os
import SwiftCBOR

private let logger = RivetLogger.handle

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

    /// Performs an actor action using positional arguments encoded as CBOR.
    public func action<Response: Decodable & Sendable>(
        _ name: String,
        args: [AnyEncodable],
        as _: Response.Type = Response.self
    ) async throws -> Response {
        logger.debug("handling action name=\(name, privacy: .public) encoding=cbor")
        let actorId = try await resolveActorId()
        logger.debug("found actor for action actorId=\(actorId, privacy: .public)")
        let encodedName = name.addingPercentEncoding(withAllowedCharacters: .urlPathAllowed) ?? name
        let path = "/action/\(encodedName)"
        let requestBody = HttpActionRequest(args: args)
        let bodyData = try CBORSupport.encoder().encode(requestBody)

        var headers: [String: String] = [
            Routing.headerEncoding: "cbor",
            "Content-Type": "application/octet-stream"
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
            logger.warning("action failed name=\(name, privacy: .public) statusCode=\(response.statusCode)")
            throw try await parseActorError(data: data, response: response, actorId: actorId)
        }

        let decoded = try CBORSupport.decode(HttpActionResponse<JSONValue>.self, from: data)
        return try CBORSupport.decode(Response.self, from: decoded.output)
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

    /// Raw JSON arguments for actions.
    public func action<Response: Decodable & Sendable>(
        _ name: String,
        args: [JSONValue],
        as _: Response.Type = Response.self
    ) async throws -> Response {
        return try await action(name, args: args.map { AnyEncodable($0) }, as: Response.self)
    }

    public func connect() -> ActorConnection {
        logger.debug("establishing connection from handle")
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
        logger.debug("found actor for raw http actorId=\(actorId, privacy: .public)")
        let fragmentParts = path.split(separator: "#", maxSplits: 1, omittingEmptySubsequences: false)
        let pathWithoutFragment = String(fragmentParts.first ?? "")
        let normalizedPath = pathWithoutFragment.hasPrefix("/") ? String(pathWithoutFragment.dropFirst()) : pathWithoutFragment
        let fullPath = "/request/\(normalizedPath)"

        var headers = request.headers
        if let paramsHeader = try encodeParamsHeader() {
            headers[Routing.headerConnParams] = paramsHeader
        }

        logger.debug("sending http request url=\(fullPath, privacy: .public) encoding=cbor")
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

    private func parseActorError(data: Data, response: HTTPURLResponse, actorId: String) async throws -> ActorError {
        let rayId = response.value(forHTTPHeaderField: "x-rivet-ray-id")

        func handleDecodedError(_ error: HttpResponseError) async throws -> ActorError {
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

        if let error = try? JSONDecoder().decode(HttpResponseError.self, from: data) {
            return try await handleDecodedError(error)
        }

        if let error = try? CBORSupport.decode(HttpResponseError.self, from: data) {
            return try await handleDecodedError(error)
        }

        if let error = try? CBORSupport.decodeHttpResponseError(from: data) {
            return try await handleDecodedError(error)
        }

        if let error = try? CBORSupport.decodeBareHttpResponseError(from: data) {
            return try await handleDecodedError(error)
        }

        let statusText = HTTPURLResponse.localizedString(forStatusCode: response.statusCode)
        let statusLabel = statusText.isEmpty ? "HTTP \(response.statusCode)" : "\(statusText) (\(response.statusCode))"
        let bodyText = String(data: data, encoding: .utf8) ?? ""
        let baseMessage: String
        if let rayId, !rayId.isEmpty {
            baseMessage = "\(statusLabel) (Ray ID: \(rayId))"
        } else {
            baseMessage = statusLabel
        }
        let message = bodyText.isEmpty ? baseMessage : "\(baseMessage):\n\(bodyText)"
        throw HttpRequestError(message)
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
        protocols.append("\(Routing.wsProtocolEncoding)cbor")
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
