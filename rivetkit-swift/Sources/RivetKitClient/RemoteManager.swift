import Foundation
import os
import SwiftCBOR

private let logger = RivetLogger.manager

actor RemoteManager {
    private var config: ClientConfig
    private let session: URLSession
    private var metadataTask: Task<Void, Error>?

    init(config: ClientConfig) {
        self.config = config
        let configuration = URLSessionConfiguration.default
        self.session = URLSession(configuration: configuration)
    }

    func currentConfig() -> ClientConfig {
        return config
    }

    func startMetadataLookupIfNeeded() {
        if config.disableMetadataLookup {
            return
        }
        if metadataTask == nil {
            metadataTask = Task { try await self.lookupMetadataWithRetry() }
        }
    }

    func ensureMetadata() async throws {
        if let task = metadataTask {
            try await task.value
        }
    }

    private func lookupMetadataWithRetry() async throws {
        let endpoint = config.endpoint
        var attempt = 0
        var delay: UInt64 = 500_000_000
        logger.debug("starting metadata lookup endpoint=\(endpoint, privacy: .public)")
        while true {
            attempt += 1
            do {
                let metadata: MetadataResponse = try await sendApiCall(
                    method: "GET",
                    path: "/metadata"
                )
                logger.debug("metadata lookup succeeded runtime=\(metadata.runtime, privacy: .public) version=\(metadata.version, privacy: .public)")
                applyMetadata(metadata)
                return
            } catch {
                logger.warning("metadata lookup failed attempt=\(attempt) error=\(String(describing: error), privacy: .public)")
                if attempt > 1 {
                    delay = min(delay * 2, 15_000_000_000)
                }
                try await Task.sleep(nanoseconds: delay)
                if config.endpoint != endpoint {
                    logger.debug("endpoint changed during metadata lookup, aborting")
                    return
                }
            }
        }
    }

    private func applyMetadata(_ metadata: MetadataResponse) {
        if let clientEndpoint = metadata.clientEndpoint {
            config.endpoint = clientEndpoint
            if let clientNamespace = metadata.clientNamespace {
                config.namespace = clientNamespace
            }
            if let clientToken = metadata.clientToken {
                config.token = clientToken
            }
        }
    }

    func getForId(name: String, actorId: String) async throws -> ActorOutput? {
        try await ensureMetadata()
        let response: ActorListResponse = try await sendApiCall(
            method: "GET",
            path: "/actors?actor_ids=\(actorId.addingPercentEncoding(withAllowedCharacters: .urlQueryAllowed) ?? actorId)"
        )
        guard let actor = response.actors.first else {
            return nil
        }
        if actor.name != name {
            return nil
        }
        return actor
    }

    func getWithKey(name: String, key: [String]) async throws -> ActorOutput? {
        try await ensureMetadata()
        let serializedKey = ActorKeyCodec.serialize(key)
        logger.debug("querying actor name=\(name, privacy: .public) key=\(key, privacy: .public)")
        let response: ActorListResponse = try await sendApiCall(
            method: "GET",
            path: "/actors?name=\(name.addingPercentEncoding(withAllowedCharacters: .urlQueryAllowed) ?? name)&key=\(serializedKey.addingPercentEncoding(withAllowedCharacters: .urlQueryAllowed) ?? serializedKey)"
        )
        if let actor = response.actors.first {
            logger.debug("actor query result actorId=\(actor.actorId, privacy: .public)")
        } else {
            logger.debug("actor query result: not found")
        }
        return response.actors.first
    }

    func getOrCreateWithKey(
        name: String,
        key: [String],
        input: AnyEncodable?,
        region: String?
    ) async throws -> ActorOutput {
        try await ensureMetadata()
        logger.debug("get or create actor name=\(name, privacy: .public) key=\(key, privacy: .public) region=\(region ?? "nil", privacy: .public)")
        let request = ActorGetOrCreateRequest(
            datacenter: region,
            name: name,
            key: ActorKeyCodec.serialize(key),
            runnerNameSelector: config.runnerName,
            crashPolicy: "sleep",
            input: try input.map { try encodeInputBase64($0) }
        )
        let response: ActorGetOrCreateResponse = try await sendApiCall(
            method: "PUT",
            path: "/actors",
            body: request
        )
        logger.debug("get or create result actorId=\(response.actor.actorId, privacy: .public) created=\(response.created)")
        return response.actor
    }

    func createActor(
        name: String,
        key: [String]?,
        input: AnyEncodable?,
        region: String?
    ) async throws -> ActorOutput {
        try await ensureMetadata()
        logger.debug("creating actor name=\(name, privacy: .public) key=\(key ?? [], privacy: .public) region=\(region ?? "nil", privacy: .public)")
        let request = ActorCreateRequest(
            datacenter: region,
            name: name,
            runnerNameSelector: config.runnerName,
            crashPolicy: "sleep",
            key: key.map { ActorKeyCodec.serialize($0) },
            input: try input.map { try encodeInputBase64($0) }
        )
        let response: ActorCreateResponse = try await sendApiCall(
            method: "POST",
            path: "/actors",
            body: request
        )
        logger.debug("created actor actorId=\(response.actor.actorId, privacy: .public)")
        return response.actor
    }

    func listActors(name: String) async throws -> [ActorOutput] {
        try await ensureMetadata()
        let response: ActorListResponse = try await sendApiCall(
            method: "GET",
            path: "/actors?name=\(name.addingPercentEncoding(withAllowedCharacters: .urlQueryAllowed) ?? name)"
        )
        return response.actors
    }

    func kvGet(actorId: String, key: String) async throws -> String? {
        try await ensureMetadata()
        let response: ActorKvGetResponse = try await sendApiCall(
            method: "GET",
            path: "/actors/\(actorId.addingPercentEncoding(withAllowedCharacters: .urlPathAllowed) ?? actorId)/kv/keys/\(key.addingPercentEncoding(withAllowedCharacters: .urlPathAllowed) ?? key)"
        )
        return response.value
    }

    func resolveActorId(for query: ActorQuery) async throws -> String {
        switch query {
        case .getForId(_, let actorId):
            return actorId
        case .getForKey(let name, let key):
            guard let actor = try await getWithKey(name: name, key: key) else {
                throw ActorError(group: "actor", code: "not_found", message: "actor not found")
            }
            return actor.actorId
        case .getOrCreateForKey(let name, let key, let input, let region):
            let actor = try await getOrCreateWithKey(name: name, key: key, input: input, region: region)
            return actor.actorId
        case .create:
            throw InternalError("actorQuery cannot be create")
        }
    }

    func getActorError(actorId: String, name: String) async throws -> JSONValue? {
        guard let actor = try await getForId(name: name, actorId: actorId) else {
            return nil
        }
        return actor.error
    }

    func sendHttpRequestToActor(
        actorId: String,
        path: String,
        method: String,
        headers: [String: String],
        body: Data?
    ) async throws -> (Data, HTTPURLResponse) {
        try await ensureMetadata()
        let url = try URLUtils.buildActorGatewayUrl(
            endpoint: config.endpoint,
            actorId: actorId,
            token: config.token,
            path: path
        )

        logger.debug("sending http request to actor actorId=\(actorId, privacy: .public) method=\(method, privacy: .public) path=\(path, privacy: .public)")

        var request = URLRequest(url: url)
        request.httpMethod = method
        request.httpBody = body

        var finalHeaders = headers
        for (key, value) in config.headers {
            finalHeaders[key] = value
        }
        if let token = config.token {
            finalHeaders[Routing.headerRivetToken] = token
        }
        for (key, value) in finalHeaders {
            request.setValue(value, forHTTPHeaderField: key)
        }

        let (data, response) = try await session.data(for: request)
        guard let httpResponse = response as? HTTPURLResponse else {
            logger.error("invalid response from actor request")
            throw HttpRequestError("invalid response")
        }
        logger.debug("http request complete statusCode=\(httpResponse.statusCode) responseLen=\(data.count)")
        return (data, httpResponse)
    }

    func openWebSocket(
        actorId: String,
        path: String,
        protocols: [String]
    ) async throws -> URLSessionWebSocketTask {
        try await ensureMetadata()
        let httpUrl = try URLUtils.buildActorGatewayUrl(
            endpoint: config.endpoint,
            actorId: actorId,
            token: config.token,
            path: path
        )
        let wsUrl = try URLUtils.toWebSocketUrl(httpUrl)
        logger.debug("opening websocket actorId=\(actorId, privacy: .public) path=\(path, privacy: .public)")
        let task = session.webSocketTask(with: wsUrl, protocols: protocols)
        task.resume()
        return task
    }

    private func sendApiCall<TOutput: Decodable>(
        method: String,
        path: String
    ) async throws -> TOutput {
        return try await sendApiCall(method: method, path: path, body: Optional<String>.none as String?)
    }

    private func sendApiCall<TInput: Encodable, TOutput: Decodable>(
        method: String,
        path: String,
        body: TInput?
    ) async throws -> TOutput {
        let url = try URLUtils.combineUrlPath(
            endpoint: config.endpoint,
            path: path,
            queryParams: ["namespace": config.namespace]
        )
        var request = URLRequest(url: url)
        request.httpMethod = method
        request.setValue("application/json", forHTTPHeaderField: "Accept")
        if let token = config.token {
            request.setValue("Bearer \(token)", forHTTPHeaderField: "Authorization")
        }
        for (key, value) in config.headers {
            request.setValue(value, forHTTPHeaderField: key)
        }

        if let body {
            let encoder = JSONEncoder()
            request.httpBody = try encoder.encode(body)
            request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        }

        logger.trace("sending api call method=\(method, privacy: .public) path=\(path, privacy: .public)")
        let (data, response) = try await session.data(for: request)
        guard let httpResponse = response as? HTTPURLResponse else {
            logger.error("invalid response from api call")
            throw HttpRequestError("invalid response")
        }

        if !(200..<300).contains(httpResponse.statusCode) {
            if let error = try? decodeHttpError(from: data) {
                logger.warning("api call failed statusCode=\(httpResponse.statusCode) group=\(error.group, privacy: .public) code=\(error.code, privacy: .public)")
                throw ActorError(group: error.group, code: error.code, message: error.message, metadata: error.metadata)
            }
            logger.warning("api call failed statusCode=\(httpResponse.statusCode)")
            throw HttpRequestError("HTTP \(httpResponse.statusCode)")
        }

        logger.trace("api call complete statusCode=\(httpResponse.statusCode)")
        let decoder = JSONDecoder()
        return try decoder.decode(TOutput.self, from: data)
    }

    private func decodeHttpError(from data: Data) throws -> HttpResponseError {
        let decoder = JSONDecoder()
        return try decoder.decode(HttpResponseError.self, from: data)
    }

    private func encodeInputBase64(_ input: AnyEncodable) throws -> String {
        let cborData = try CBORSupport.encoder().encode(input)
        return cborData.base64EncodedString()
    }
}

struct HttpResponseError: Decodable, Sendable {
    let group: String
    let code: String
    let message: String
    let metadata: JSONValue?
}

struct MetadataResponse: Decodable, Sendable {
    let runtime: String
    let version: String
    let runner: MetadataRunner?
    let actorNames: [String: MetadataActorName]
    let clientEndpoint: String?
    let clientNamespace: String?
    let clientToken: String?
}

struct MetadataRunner: Decodable, Sendable {
    let kind: [String: EmptyObject]
    let version: Int?
}

struct MetadataActorName: Decodable, Sendable {
    let metadata: [String: JSONValue]
}

struct EmptyObject: Decodable, Sendable {}

struct ActorGetOrCreateRequest: Encodable, Sendable {
    let datacenter: String?
    let name: String
    let key: String
    let runnerNameSelector: String
    let crashPolicy: String
    let input: String?

    enum CodingKeys: String, CodingKey {
        case datacenter
        case name
        case key
        case runnerNameSelector = "runner_name_selector"
        case crashPolicy = "crash_policy"
        case input
    }
}

struct ActorCreateRequest: Encodable, Sendable {
    let datacenter: String?
    let name: String
    let runnerNameSelector: String
    let crashPolicy: String
    let key: String?
    let input: String?

    enum CodingKeys: String, CodingKey {
        case datacenter
        case name
        case runnerNameSelector = "runner_name_selector"
        case crashPolicy = "crash_policy"
        case key
        case input
    }
}
