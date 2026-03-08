import Foundation
import Testing
import RivetKitClient

struct InputPayload: Codable, Equatable {
    let name: String
    let value: Int
    let nested: Nested

    struct Nested: Codable, Equatable {
        let foo: String
    }
}

struct InputsResponse: Decodable, Equatable {
    let initialInput: JSONValue?
    let onCreateInput: JSONValue?
}

struct WelcomeMessage: Decodable, Equatable {
    let type: String
    let connectionCount: Int
}

struct PongMessage: Decodable, Equatable {
    let type: String
    let timestamp: Int64
}

struct RequestInfoMessage: Decodable, Equatable {
    let type: String
    let url: String
    let pathname: String
    let search: String
}

enum TestTimeoutError: Error {
    case timedOut
}

final class AsyncIteratorBox<Element>: @unchecked Sendable {
    // Safe because the iterator is only consumed within a single task at a time in tests.
    var iterator: AsyncStream<Element>.AsyncIterator

    init(_ iterator: AsyncStream<Element>.AsyncIterator) {
        self.iterator = iterator
    }
}

func withTimeout<T: Sendable>(
    seconds: Double,
    operation: @escaping @Sendable () async throws -> T
) async throws -> T {
    try await withThrowingTaskGroup(of: T.self) { group in
        group.addTask {
            try await operation()
        }
        group.addTask {
            try await Task.sleep(nanoseconds: UInt64(seconds * 1_000_000_000))
            throw TestTimeoutError.timedOut
        }
        let result = try await group.next()!
        group.cancelAll()
        return result
    }
}

func nextValue<T: Sendable>(
    from stream: AsyncStream<T>,
    timeoutSeconds: Double
) async throws -> T {
    let box = AsyncIteratorBox(stream.makeAsyncIterator())
    return try await withTimeout(seconds: timeoutSeconds) {
        guard let value = await box.iterator.next() else {
            throw TestTimeoutError.timedOut
        }
        return value
    }
}

func waitNext<T: Sendable>(
    iterator: inout AsyncStream<T>.AsyncIterator,
    timeoutSeconds: Double
) async throws -> T {
    let box = AsyncIteratorBox(iterator)
    let value = try await withTimeout(seconds: timeoutSeconds) {
        guard let next = await box.iterator.next() else {
            throw TestTimeoutError.timedOut
        }
        return next
    }
    iterator = box.iterator
    return value
}

func sleepMilliseconds(_ milliseconds: UInt64) async {
    try? await Task.sleep(nanoseconds: milliseconds * 1_000_000)
}

@Suite("RivetKitClient")
final class RivetKitClientSuite {
    let server: TestServerInfo

    init() async throws {
        server = try await TestServer.shared.start()
    }

    deinit {
        Task {
            await TestServer.shared.stop()
        }
    }

    @Test
    func handleAccessAndResolve() async throws {
        let client = try makeClient()
        let handle = client.getOrCreate("counter", ["swift-handle"]) 
        let count: Int = try await handle.action("increment", 5, as: Int.self)
        #expect(count == 5)

        let actorId = try await handle.resolve()
        let byId = client.getForId("counter", actorId)
        let retrieved: Int = try await byId.action("getCount", as: Int.self)
        #expect(retrieved == 5)
    }

    @Test
    func createWithInputAndGetOrCreateInput() async throws {
        let client = try makeClient()
        let payload = InputPayload(name: "swift-input", value: 42, nested: .init(foo: "bar"))
        let created = try await client.create("inputActor", ["swift-input"], options: CreateOptions(input: payload))
        let createdInputs: InputsResponse = try await created.action("getInputs", as: InputsResponse.self)
        #expect(createdInputs.initialInput == .object([
            "name": .string("swift-input"),
            "value": .number(.int(42)),
            "nested": .object(["foo": .string("bar")])
        ]))
        #expect(createdInputs.onCreateInput == createdInputs.initialInput)

        let getOrCreate = client.getOrCreate(
            "inputActor",
            ["swift-input-2"],
            options: GetOrCreateOptions(createWithInput: payload)
        )
        let inputs: InputsResponse = try await getOrCreate.action("getInputs", as: InputsResponse.self)
        #expect(inputs.initialInput == createdInputs.initialInput)
        #expect(inputs.onCreateInput == createdInputs.initialInput)
    }

    @Test
    func createDuplicateKeyThrows() async throws {
        let client = try makeClient()
        let key = ["swift-dup", UUID().uuidString]
        _ = try await client.create("counter", key)
        do {
            _ = try await client.create("counter", key)
            #expect(Bool(false))
        } catch let error as ActorError {
            #expect(error.group == "actor")
            #expect(error.code == "duplicate_key")
        }
    }

    @Test
    func rawHttpRequests() async throws {
        let client = try makeClient()
        let actor = client.getOrCreate("rawHttpActor", ["swift-http"]) 

        let hello = try await actor.fetch("api/hello")
        #expect(hello.ok)
        let helloJson: [String: String] = try hello.json()
        #expect(helloJson["message"] == "Hello from actor!")

        struct EchoPayload: Encodable {
            let test: String
            let number: Int
        }
        let payload = try JSONEncoder().encode(EchoPayload(test: "data", number: 123))
        let echo = try await actor.fetch(
            "api/echo",
            request: RawHTTPRequest(
                method: "POST",
                headers: ["Content-Type": "application/json"],
                body: payload
            )
        )
        #expect(echo.ok)
        let echoJson: [String: JSONValue] = try echo.json()
        #expect(echoJson["test"] == .string("data"))
    }

    @Test
    func rawWebSocketMessages() async throws {
        let client = try makeClient()
        let actor = client.getOrCreate("rawWebSocketActor", ["swift-ws"]) 
        let websocket = try await actor.websocket(path: "stream?foo=bar")

        let welcomeMessage = try await websocket.receive()
        let welcome = try decodeMessage(WelcomeMessage.self, from: welcomeMessage)
        #expect(welcome.type == "welcome")
        #expect(welcome.connectionCount >= 1)

        let ping = try JSONEncoder().encode(["type": "ping"])
        try await websocket.send(text: String(data: ping, encoding: .utf8) ?? "")
        let pongMessage = try await websocket.receive()
        let pong = try decodeMessage(PongMessage.self, from: pongMessage)
        #expect(pong.type == "pong")

        let requestInfo = try JSONEncoder().encode(["type": "getRequestInfo"])
        try await websocket.send(text: String(data: requestInfo, encoding: .utf8) ?? "")
        let infoMessage = try await websocket.receive()
        let info = try decodeMessage(RequestInfoMessage.self, from: infoMessage)
        #expect(info.pathname.contains("/websocket/stream"))
        #expect(info.search.contains("foo=bar"))

        await websocket.close()
    }

    @Test
    func connectionEventsAndParams() async throws {
        let client = try makeClient()
        let handle1 = client.getOrCreate(
            "counterWithParams",
            ["swift-params"],
            options: GetOrCreateOptions(params: ["name": "user1"])
        )
        let handle2 = client.getOrCreate(
            "counterWithParams",
            ["swift-params"],
            options: GetOrCreateOptions(params: ["name": "user2"])
        )

        let conn1 = handle1.connect()
        let conn2 = handle2.connect()

        struct NewCountEvent: Decodable, Sendable {
            let count: Int
            let by: String
        }

        // Wait for connection to be open before subscribing
        let openStream = await conn1.opens()
        _ = try await nextValue(from: openStream, timeoutSeconds: 5)

        let eventStream = await conn1.events("newCount", as: NewCountEvent.self)

        _ = try await conn1.action("increment", 1, as: Int.self)

        let payload = try await nextValue(from: eventStream, timeoutSeconds: 5)
        #expect(payload.count == 1)
        #expect(payload.by == "user1")

        let initializers: [String] = try await handle1.action("getInitializers", as: [String].self)
        #expect(initializers.contains("user1"))
        #expect(initializers.contains("user2"))

        await conn1.dispose()
        await conn2.dispose()
    }

    @Test
    func actorErrorHandling() async throws {
        let client = try makeClient()
        let handle = client.getOrCreate("errorHandlingActor", ["swift-error"]) 

        do {
            _ = try await handle.action("throwSimpleError", as: String.self)
            #expect(Bool(false))
        } catch let error as ActorError {
            #expect(error.message == "Simple error message")
            #expect(error.code == "user_error")
        }

        do {
            _ = try await handle.action("throwDetailedError", as: String.self)
            #expect(Bool(false))
        } catch let error as ActorError {
            #expect(error.message == "Detailed error message")
            #expect(error.code == "detailed_error")
        }

        let success: String = try await handle.action("successfulAction", as: String.self)
        #expect(success == "success")
    }

    @Test
    func metadataLookup() async throws {
        let config = try ClientConfig(
            endpoint: server.endpoint,
            namespace: server.namespace,
            runnerName: server.runnerName,
            disableMetadataLookup: false
        )
        let client = RivetKitClient(config: config)
        let handle = client.getOrCreate("counter", ["swift-metadata"]) 
        let count: Int = try await handle.action("increment", 2, as: Int.self)
        #expect(count == 2)
    }

    @Test
    func clientConfigParsing() throws {
        let config = try ClientConfig(
            endpoint: "https://default:token@example.com",
            token: nil,
            namespace: nil
        )
        #expect(config.endpoint == "https://example.com")
        #expect(config.namespace == "default")
        #expect(config.token == "token")

        do {
            _ = try ClientConfig(endpoint: "http://example.com?query=1")
            #expect(Bool(false))
        } catch {
        }

        do {
            _ = try ClientConfig(endpoint: "http://example.com#frag")
            #expect(Bool(false))
        } catch {
        }
    }

    @Test
    func getHandleForExistingActor() async throws {
        let client = try makeClient()
        let key = ["swift-get", UUID().uuidString]
        _ = try await client.create("counter", key)
        let handle = client.get("counter", key)
        let count: Int = try await handle.action("increment", 3, as: Int.self)
        #expect(count == 3)
    }

    @Test
    func rawHttpRequestProperties() async throws {
        let client = try makeClient()
        let actor = client.getOrCreate("rawHttpRequestPropertiesActor", ["swift-req-props", UUID().uuidString])

        let payload = try JSONEncoder().encode(["hello": "world"])
        let response = try await actor.fetch(
            "props?foo=bar&baz=qux",
            request: RawHTTPRequest(
                method: "POST",
                headers: ["Content-Type": "application/json", "X-Test": "swift"],
                body: payload
            )
        )
        #expect(response.ok)

        let json: [String: JSONValue] = try response.json()
        #expect(json["method"] == .string("POST"))

        if case .object(let headers) = json["headers"] {
            #expect(headers["x-test"] == .string("swift"))
        } else {
            #expect(Bool(false))
        }

        if case .object(let searchParams) = json["searchParams"] {
            #expect(searchParams["foo"] == .string("bar"))
            #expect(searchParams["baz"] == .string("qux"))
        } else {
            #expect(Bool(false))
        }

        if case .object(let body) = json["body"] {
            #expect(body["hello"] == .string("world"))
        } else {
            #expect(Bool(false))
        }
    }

    @Test
    func rawHttpSpecialMethods() async throws {
        let client = try makeClient()
        let actor = client.getOrCreate("rawHttpRequestPropertiesActor", ["swift-req-methods", UUID().uuidString])

        let head = try await actor.fetch("head", request: RawHTTPRequest(method: "HEAD"))
        #expect(head.statusCode == 200)
        #expect(head.data.isEmpty)

        let options = try await actor.fetch("options", request: RawHTTPRequest(method: "OPTIONS"))
        #expect(options.statusCode == 204)
    }

    @Test
    func rawHttpErrorResponses() async throws {
        let client = try makeClient()
        let noHandler = client.getOrCreate("rawHttpNoHandlerActor", ["swift-raw-nohandler", UUID().uuidString])
        let response = try await noHandler.fetch("missing")
        #expect(!response.ok)

        let voidReturn = client.getOrCreate("rawHttpVoidReturnActor", ["swift-raw-void", UUID().uuidString])
        let response2 = try await voidReturn.fetch("missing")
        #expect(!response2.ok)

        let raw = client.getOrCreate("rawHttpActor", ["swift-raw-text", UUID().uuidString])
        let response3 = try await raw.fetch("missing-path")
        #expect(response3.statusCode == 404)
        #expect(response3.text().contains("Not Found"))
    }

    @Test
    func rawHttpHeadersAndParams() async throws {
        let client = try makeClient()
        let actor = client.getOrCreate(
            "rawHttpActor",
            ["swift-raw-headers", UUID().uuidString],
            options: GetOrCreateOptions(params: ["token": "abc"])
        )

        let response = try await actor.fetch("api/headers")
        #expect(response.ok)
        let headers: [String: JSONValue] = try response.json()
        if case .string(let params) = headers["x-rivet-conn-params"] {
            let decoded = try JSONDecoder().decode([String: String].self, from: Data(params.utf8))
            #expect(decoded["token"] == "abc")
        } else {
            #expect(Bool(false))
        }
    }

    @Test
    func rawHttpHonoRoutes() async throws {
        let client = try makeClient()
        let actor = client.getOrCreate("rawHttpHonoActor", ["swift-hono", UUID().uuidString])

        let root = try await actor.fetch("")
        let rootJson: [String: String] = try root.json()
        #expect(rootJson["message"] == "Welcome to Hono actor!")

        let users = try await actor.fetch("users")
        let usersJson: [[String: JSONValue]] = try users.json()
        #expect(usersJson.count == 2)

        let newUserPayload = try JSONEncoder().encode(["name": "Charlie"])
        let created = try await actor.fetch(
            "users",
            request: RawHTTPRequest(
                method: "POST",
                headers: ["Content-Type": "application/json"],
                body: newUserPayload
            )
        )
        let createdJson: [String: JSONValue] = try created.json()
        #expect(createdJson["name"] == .string("Charlie"))
    }

    @Test
    func rawWebSocketBinary() async throws {
        let client = try makeClient()
        let actor = client.getOrCreate("rawWebSocketBinaryActor", ["swift-ws-bin", UUID().uuidString])
        let websocket = try await actor.websocket(path: "binary")

        let payload = Data([1, 2, 3, 4])
        try await websocket.send(data: payload)
        let message = try await websocket.receive()
        switch message {
        case .data(let data):
            #expect(data == Data([4, 3, 2, 1]))
        case .text:
            #expect(Bool(false))
        }
        await websocket.close()
    }

    @Test
    func connectionLifecycleAndReconnect() async throws {
        let client = try makeClient()
        let handle = client.getOrCreate("connStateActor", ["swift-conn-life", UUID().uuidString])
        let conn = handle.connect()

        let openStream = await conn.opens()
        let closeStream = await conn.closes()
        let statusStream = await conn.statusChanges()

        var openIterator = openStream.makeAsyncIterator()
        _ = try await waitNext(iterator: &openIterator, timeoutSeconds: 5)

        let connStateValue: JSONValue = try await conn.action("getConnectionState", as: JSONValue.self)
        guard case .object(let connState) = connStateValue, case .string = connState["id"] else {
            #expect(Bool(false))
            await conn.dispose()
            return
        }
        do {
            _ = try await conn.action("disconnectSelf", "test.disconnect", as: Bool.self)
        } catch let error as ActorError {
            #expect(error.group == "test")
            #expect(error.code == "disconnect")
        } catch {
            throw error
        }

        _ = try await nextValue(from: closeStream, timeoutSeconds: 5)

        var statusIterator = statusStream.makeAsyncIterator()
        _ = try await waitForStatus(.disconnected, iterator: &statusIterator, timeoutSeconds: 5)
        _ = try await waitNext(iterator: &openIterator, timeoutSeconds: 5)
        _ = try await waitForStatus(.connected, iterator: &statusIterator, timeoutSeconds: 5)

        await conn.dispose()
    }

    @Test
    func connectionOnceAndSubscriptions() async throws {
        let client = try makeClient()
        let handle = client.getOrCreate(
            "counterWithParams",
            ["swift-once", UUID().uuidString],
            options: GetOrCreateOptions(params: ["name": "once"])
        )
        let conn = handle.connect()

        let openStream = await conn.opens()
        _ = try await nextValue(from: openStream, timeoutSeconds: 5)

        struct NewCountEvent: Decodable, Sendable {
            let count: Int
            let by: String
        }

        let eventStream = await conn.events("newCount", as: NewCountEvent.self)

        _ = try await conn.action("increment", 1, as: Int.self)
        _ = try await conn.action("increment", 1, as: Int.self)

        // Get just the first event (simulates once behavior)
        _ = try await nextValue(from: eventStream, timeoutSeconds: 5)

        await conn.dispose()
    }

    @Test
    func connectionErrorCallback() async throws {
        let client = try makeClient()
        let handle = client.getOrCreate("connStateActor", ["swift-conn-error", UUID().uuidString])
        let conn = handle.connect()

        let openStream = await conn.opens()
        _ = try await nextValue(from: openStream, timeoutSeconds: 5)

        let errorStream = await conn.errors()

        do {
            _ = try await conn.action("disconnectSelf", "test.disconnect", as: Bool.self)
        } catch let error as ActorError {
            #expect(error.group == "test")
            #expect(error.code == "disconnect")
        } catch {
            throw error
        }

        let error = try await nextValue(from: errorStream, timeoutSeconds: 8)
        #expect(error.group == "test")
        #expect(error.code == "disconnect")
        await conn.dispose()
    }

    @Test
    func connectionStateActions() async throws {
        let client = try makeClient()
        let key = ["swift-conn-state", UUID().uuidString]
        let handle = client.getOrCreate(
            "connStateActor",
            key,
            options: GetOrCreateOptions(params: ["username": "alice", "role": "admin"])
        )
        let conn1 = handle.connect()
        let conn2 = handle.connect()

        let openStream = await conn1.opens()
        _ = try await nextValue(from: openStream, timeoutSeconds: 5)

        let stateValue: JSONValue = try await conn1.action("getConnectionState", as: JSONValue.self)
        guard case .object(let state1) = stateValue else {
            #expect(Bool(false))
            await conn1.dispose()
            await conn2.dispose()
            return
        }
        guard case .string(let state1Id) = state1["id"] else {
            #expect(Bool(false))
            await conn1.dispose()
            await conn2.dispose()
            return
        }
        #expect(state1["username"] == .string("alice"))
        #expect(state1["role"] == .string("admin"))

        let updatedValue: JSONValue = try await conn1.action(
            "updateConnection",
            ["username": "bob"],
            as: JSONValue.self
        )
        if case .object(let updated) = updatedValue {
            #expect(updated["username"] == .string("bob"))
        } else {
            #expect(Bool(false))
        }

        struct DirectMessageEvent: Decodable, Sendable {
            let from: String
            let message: String
        }

        let messageStream = await conn1.events("directMessage", as: DirectMessageEvent.self)

        let state2Value: JSONValue = try await conn2.action("getConnectionState", as: JSONValue.self)
        guard case .object(let state2) = state2Value, case .string(let state2Id) = state2["id"] else {
            #expect(Bool(false))
            await conn1.dispose()
            await conn2.dispose()
            return
        }
        let sent: Bool = try await conn2.action(
            "sendToConnection",
            args: [AnyEncodable(state1Id), AnyEncodable("hello")],
            as: Bool.self
        )
        #expect(sent)

        let payload = try await nextValue(from: messageStream, timeoutSeconds: 5)
        #expect(payload.from == state2Id)
        #expect(payload.message == "hello")

        await conn1.dispose()
        await conn2.dispose()
    }

    @Test
    func requestAccessTracking() async throws {
        let client = try makeClient()
        let handle = client.getOrCreate(
            "requestAccessActor",
            ["swift-req-access", UUID().uuidString],
            options: GetOrCreateOptions(params: ["trackRequest": true])
        )
        let conn = handle.connect()

        let openStream = await conn.opens()
        _ = try await nextValue(from: openStream, timeoutSeconds: 5)

        struct RequestInfo: Decodable {
            let hasRequest: Bool
            let requestUrl: String?
            let requestMethod: String?
            let requestHeaders: [String: String]
        }

        struct RequestInfoResponse: Decodable {
            let onBeforeConnect: RequestInfo
            let createConnState: RequestInfo
            let onRequest: RequestInfo
            let onWebSocket: RequestInfo
        }

        let info: RequestInfoResponse = try await handle.action("getRequestInfo", as: RequestInfoResponse.self)
        #expect(info.onBeforeConnect.hasRequest)
        #expect(info.createConnState.hasRequest)
        #expect(info.onBeforeConnect.requestMethod != nil)

        let rawResponse = try await handle.fetch("check?foo=bar")
        let rawInfo: RequestInfo = try rawResponse.json()
        #expect(rawInfo.hasRequest)
        #expect(rawInfo.requestUrl?.contains("check") == true)

        let websocket = try await handle.websocket(path: "socket")
        let wsMessage = try await websocket.receive()
        let wsInfo = try decodeMessage(RequestInfo.self, from: wsMessage)
        #expect(wsInfo.hasRequest)
        #expect(wsInfo.requestUrl?.contains("socket") == true)
        await websocket.close()

        await conn.dispose()
    }

    @Test
    func actionTypesAndTimeouts() async throws {
        let client = try makeClient()
        let sync = client.getOrCreate("syncActionActor", ["swift-sync", UUID().uuidString])
        let increment: Int = try await sync.action("increment", 2, as: Int.self)
        #expect(increment == 2)

        let asyncActor = client.getOrCreate("asyncActionActor", ["swift-async", UUID().uuidString])
        struct AsyncData: Decodable { let id: String }
        let data: AsyncData = try await asyncActor.action("fetchData", "swift", as: AsyncData.self)
        #expect(data.id == "swift")

        let promise = client.getOrCreate("promiseActor", ["swift-promise", UUID().uuidString])
        let resolved: String = try await promise.action("resolvedPromise", as: String.self)
        #expect(resolved == "resolved value")

        do {
            _ = try await promise.action("rejectedPromise", as: String.self)
            #expect(Bool(false))
        } catch let error as ActorError {
            #expect(error.code == "user_error")
        }

        let shortTimeout = client.getOrCreate("shortTimeoutActor", ["swift-timeout", UUID().uuidString])
        do {
            _ = try await shortTimeout.action("slowAction", as: String.self)
            #expect(Bool(false))
        } catch let error as ActorError {
            #expect(error.group == "action")
            #expect(error.code == "timed_out")
        }

        let longTimeout = client.getOrCreate("longTimeoutActor", ["swift-timeout-long", UUID().uuidString])
        let delayed: String = try await longTimeout.action("delayedAction", as: String.self)
        #expect(delayed == "delayed response")
    }

    @Test
    func kvAndLargePayloads() async throws {
        let client = try makeClient()
        let kv = client.getOrCreate("kvActor", ["swift-kv", UUID().uuidString])
        let put: Bool = try await kv.action("putText", args: [AnyEncodable("key1"), AnyEncodable("value1")], as: Bool.self)
        #expect(put)

        let value: String? = try await kv.action("getText", args: [AnyEncodable("key1")], as: String?.self)
        #expect(value == "value1")

        struct KvPair: Decodable {
            let key: String
            let value: String
        }
        let list: [KvPair] = try await kv.action("listText", args: [AnyEncodable("key")], as: [KvPair].self)
        #expect(list.contains { $0.key == "key1" && $0.value == "value1" })

        let bufferRoundtrip: [Int]? = try await kv.action(
            "roundtripArrayBuffer",
            args: [AnyEncodable("buf1"), AnyEncodable([1, 2, 3])],
            as: [Int]?.self
        )
        #expect(bufferRoundtrip == [1, 2, 3])

        let large = client.getOrCreate("largePayloadActor", ["swift-large", UUID().uuidString])
        let items = (0..<200).map { "Item \($0)" }
        struct LargeRequest: Encodable { let items: [String] }
        struct LargeResponse: Decodable { let itemCount: Int; let firstItem: String; let lastItem: String }
        let response: LargeResponse = try await large.action("processLargeRequest", LargeRequest(items: items), as: LargeResponse.self)
        #expect(response.itemCount == 200)
        #expect(response.firstItem == "Item 0")

        struct LargeOutput: Decodable { let items: [String] }
        let largeResponse: LargeOutput = try await large.action("getLargeResponse", 150, as: LargeOutput.self)
        #expect(largeResponse.items.count == 150)
    }

    @Test
    func varsAndStateChange() async throws {
        let client = try makeClient()
        let staticVars = client.getOrCreate("staticVarActor", ["swift-vars", UUID().uuidString])
        let vars: [String: JSONValue] = try await staticVars.action("getVars", as: [String: JSONValue].self)
        #expect(vars["name"] == .string("test-actor"))

        let dynamicVars = client.getOrCreate("dynamicVarActor", ["swift-vars-dyn", UUID().uuidString])
        let dynVars: [String: JSONValue] = try await dynamicVars.action("getVars", as: [String: JSONValue].self)
        #expect(dynVars["computed"] != nil)

        let stateChange = client.getOrCreate("onStateChangeActor", ["swift-state-change", UUID().uuidString])
        let before: Int = try await stateChange.action("getChangeCount", as: Int.self)
        _ = try await stateChange.action("setValue", 1, as: Int.self)
        let after: Int = try await stateChange.action("getChangeCount", as: Int.self)
        #expect(after >= before)
    }

    @Test
    func scheduledTasks() async throws {
        let client = try makeClient()
        let handle = client.getOrCreate("scheduled", ["swift-sched", UUID().uuidString])
        _ = try await handle.action("clearHistory", as: Bool.self)
        _ = try await handle.action("scheduleTaskAfter", 50, as: Int64.self)
        await sleepMilliseconds(150)
        let count: Int = try await handle.action("getScheduledCount", as: Int.self)
        #expect(count >= 1)
    }

    private func makeClient() throws -> RivetKitClient {
        let config = try ClientConfig(
            endpoint: server.endpoint,
            namespace: server.namespace,
            runnerName: server.runnerName,
            disableMetadataLookup: true
        )
        return RivetKitClient(config: config)
    }

    private func decodeMessage<T: Decodable>(_ type: T.Type, from message: ActorWebSocketMessage) throws -> T {
        switch message {
        case .text(let text):
            let data = Data(text.utf8)
            return try JSONDecoder().decode(T.self, from: data)
        case .data(let data):
            return try JSONDecoder().decode(T.self, from: data)
        }
    }

    private func forceDisconnect(actorId: String, connId: String) async throws {
        let encodedActor = actorId.addingPercentEncoding(withAllowedCharacters: .urlQueryAllowed) ?? actorId
        let encodedConn = connId.addingPercentEncoding(withAllowedCharacters: .urlQueryAllowed) ?? connId
        guard let url = URL(string: "\(server.endpoint)/.test/force-disconnect?actor=\(encodedActor)&conn=\(encodedConn)") else {
            throw InternalError("invalid force disconnect url")
        }
        var request = URLRequest(url: url)
        request.httpMethod = "POST"
        let (_, response) = try await URLSession.shared.data(for: request)
        guard let http = response as? HTTPURLResponse, http.statusCode == 200 else {
            throw InternalError("force disconnect failed")
        }
    }

    private func waitForStatus(
        _ status: ActorConnStatus,
        iterator: inout AsyncStream<ActorConnStatus>.AsyncIterator,
        timeoutSeconds: Double
    ) async throws -> ActorConnStatus {
        let box = AsyncIteratorBox(iterator)
        let result = try await withTimeout(seconds: timeoutSeconds) {
            while let value = await box.iterator.next() {
                if value == status {
                    return value
                }
            }
            throw TestTimeoutError.timedOut
        }
        iterator = box.iterator
        return result
    }

    // MARK: - AsyncStream Lifecycle Tests

    @Test
    func statusChangesYieldsCurrentStatusImmediately() async throws {
        let client = try makeClient()
        let handle = client.getOrCreate("counter", ["swift-status-immediate", UUID().uuidString])
        let conn = handle.connect()

        // Get status stream - it should yield current status immediately
        let statusStream = await conn.statusChanges()
        var iterator = statusStream.makeAsyncIterator()

        // First value should be the current status (could be idle, connecting, or connected)
        let firstStatus = try await waitNext(iterator: &iterator, timeoutSeconds: 5)
        #expect(firstStatus == .idle || firstStatus == .connecting || firstStatus == .connected)

        // Wait for connected
        if firstStatus != .connected {
            _ = try await waitForStatus(.connected, iterator: &iterator, timeoutSeconds: 5)
        }

        // Verify currentStatus property matches
        let currentStatus = await conn.currentStatus
        #expect(currentStatus == .connected)

        await conn.dispose()
    }

    @Test
    func multipleSubscribersToLifecycleStreams() async throws {
        let client = try makeClient()
        let handle = client.getOrCreate("counter", ["swift-multi-sub", UUID().uuidString])
        let conn = handle.connect()

        // Create multiple status stream subscribers
        let stream1 = await conn.statusChanges()
        let stream2 = await conn.statusChanges()

        var iterator1 = stream1.makeAsyncIterator()
        var iterator2 = stream2.makeAsyncIterator()

        // Both should receive initial status
        let status1 = try await waitNext(iterator: &iterator1, timeoutSeconds: 5)
        let status2 = try await waitNext(iterator: &iterator2, timeoutSeconds: 5)

        #expect(status1 == status2)

        // Wait for both to reach connected
        if status1 != .connected {
            _ = try await waitForStatus(.connected, iterator: &iterator1, timeoutSeconds: 5)
            _ = try await waitForStatus(.connected, iterator: &iterator2, timeoutSeconds: 5)
        }

        await conn.dispose()

        // Both should receive disposed status
        let disposed1 = try await waitForStatus(.disposed, iterator: &iterator1, timeoutSeconds: 5)
        let disposed2 = try await waitForStatus(.disposed, iterator: &iterator2, timeoutSeconds: 5)

        #expect(disposed1 == .disposed)
        #expect(disposed2 == .disposed)
    }

    @Test
    func currentStatusPropertyAtVariousStages() async throws {
        let client = try makeClient()
        let handle = client.getOrCreate("counter", ["swift-current-status", UUID().uuidString])
        let conn = handle.connect()

        // Status may be idle, connecting, or connected immediately after connect()
        let initialStatus = await conn.currentStatus
        #expect(initialStatus == .idle || initialStatus == .connecting || initialStatus == .connected)

        // Wait for connected using opens stream
        let openStream = await conn.opens()
        _ = try await nextValue(from: openStream, timeoutSeconds: 5)

        let connectedStatus = await conn.currentStatus
        #expect(connectedStatus == .connected)

        await conn.dispose()

        let disposedStatus = await conn.currentStatus
        #expect(disposedStatus == .disposed)
    }

    // MARK: - Parameter Pack Action Tests

    @Test
    func parameterPackActionsWithVaryingArgCounts() async throws {
        let client = try makeClient()
        let handle = client.getOrCreate("counter", ["swift-param-pack", UUID().uuidString])
        let conn = handle.connect()

        let openStream = await conn.opens()
        _ = try await nextValue(from: openStream, timeoutSeconds: 5)

        // Zero arguments
        let count0: Int = try await conn.action("getCount")
        #expect(count0 >= 0)

        // One argument
        let count1: Int = try await conn.action("increment", 5)
        #expect(count1 == count0 + 5)

        // Two arguments via handle (using kvActor with putText/getText)
        let kvHandle = client.getOrCreate("kvActor", ["swift-param-pack-kv", UUID().uuidString])
        let setResult: Bool = try await kvHandle.action("putText", "testKey", "testValue")
        #expect(setResult == true)

        let getValue: String? = try await kvHandle.action("getText", "testKey")
        #expect(getValue == "testValue")

        await conn.dispose()
    }

    // MARK: - JSONValue Escape Hatch Tests

    @Test
    func jsonValueActionEscapeHatch() async throws {
        let client = try makeClient()
        let handle = client.getOrCreate("counter", ["swift-json-action", UUID().uuidString])

        // Use JSONValue array for action args
        let result: Int = try await handle.action("increment", args: [.number(.int(10))])
        #expect(result >= 10)

        // Multiple JSONValue args (using kvActor with putText)
        let kvHandle = client.getOrCreate("kvActor", ["swift-json-kv", UUID().uuidString])
        let setResult: Bool = try await kvHandle.action(
            "putText",
            args: [.string("jsonKey"), .string("jsonValue")]
        )
        #expect(setResult == true)
    }

    @Test
    func jsonValueEventsEscapeHatch() async throws {
        let client = try makeClient()
        let handle = client.getOrCreate("counter", ["swift-json-events", UUID().uuidString])
        let conn = handle.connect()

        let openStream = await conn.opens()
        _ = try await nextValue(from: openStream, timeoutSeconds: 5)

        // Use raw JSONValue events stream
        let eventStream = await conn.events("newCount")

        // Trigger event
        let _: Int = try await conn.action("increment", 1)

        // Get the first event
        let args = try await nextValue(from: eventStream, timeoutSeconds: 5)

        // Verify we received the event
        #expect(!args.isEmpty)
        // Verify first arg is the count (a number)
        if case .number = args[0] {
            // Success - it's a number
        } else {
            #expect(Bool(false), "Expected number in event args")
        }

        await conn.dispose()
    }

    @Test
    func typedEventsStream() async throws {
        let client = try makeClient()
        let handle = client.getOrCreate("counter", ["swift-typed-events", UUID().uuidString])
        let conn = handle.connect()

        let openStream = await conn.opens()
        _ = try await nextValue(from: openStream, timeoutSeconds: 5)

        // Use typed events stream
        let eventStream = await conn.events("newCount", as: Int.self)

        // Trigger event
        let _: Int = try await conn.action("increment", 7)

        // Get the first event
        let receivedCount = try await nextValue(from: eventStream, timeoutSeconds: 5)

        #expect(receivedCount >= 7)

        await conn.dispose()
    }

    // MARK: - Edge Case Tests

    @Test
    func actionOnDisposedConnectionThrows() async throws {
        let client = try makeClient()
        let handle = client.getOrCreate("counter", ["swift-disposed-action", UUID().uuidString])
        let conn = handle.connect()

        let openStream = await conn.opens()
        _ = try await nextValue(from: openStream, timeoutSeconds: 5)

        await conn.dispose()

        // Action on disposed connection should throw
        do {
            let _: Int = try await conn.action("getCount")
            #expect(Bool(false), "Expected error on disposed connection")
        } catch is ActorConnDisposed {
            // Expected
        } catch {
            // Also acceptable - any error on disposed connection
        }
    }

    @Test
    func rapidConnectDisposeCycles() async throws {
        let client = try makeClient()

        for i in 0..<3 {
            let handle = client.getOrCreate("counter", ["swift-rapid-cycle", UUID().uuidString, "\(i)"])
            let conn = handle.connect()

            // Brief wait for connection attempt
            await sleepMilliseconds(50)

            let status = await conn.currentStatus
            #expect(status != .disposed)

            await conn.dispose()

            let disposedStatus = await conn.currentStatus
            #expect(disposedStatus == .disposed)
        }
    }

    @Test
    func errorStreamReceivesConnectionErrors() async throws {
        let client = try makeClient()
        let handle = client.getOrCreate("connStateActor", ["swift-error-stream", UUID().uuidString])
        let conn = handle.connect()

        let openStream = await conn.opens()
        _ = try await nextValue(from: openStream, timeoutSeconds: 5)

        let errorStream = await conn.errors()

        // Trigger a disconnect error
        do {
            _ = try await conn.action("disconnectSelf", "test.error_stream", as: Bool.self)
        } catch {
            // Expected to throw
        }

        // Error stream should receive the error
        let error = try await nextValue(from: errorStream, timeoutSeconds: 8)
        #expect(error.group == "test")
        #expect(error.code == "error_stream")

        await conn.dispose()
    }

    @Test
    func opensAndClosesStreams() async throws {
        let client = try makeClient()
        let handle = client.getOrCreate("connStateActor", ["swift-open-close-streams", UUID().uuidString])
        let conn = handle.connect()

        let opensStream = await conn.opens()
        let closesStream = await conn.closes()

        // Wait for open
        _ = try await nextValue(from: opensStream, timeoutSeconds: 5)
        #expect(await conn.currentStatus == .connected)

        // Trigger disconnect
        do {
            _ = try await conn.action("disconnectSelf", "test.close_stream", as: Bool.self)
        } catch {
            // Expected
        }

        // Wait for close
        _ = try await nextValue(from: closesStream, timeoutSeconds: 5)

        // Should reconnect - wait for another open
        _ = try await nextValue(from: opensStream, timeoutSeconds: 5)
        #expect(await conn.currentStatus == .connected)

        await conn.dispose()
    }
}
