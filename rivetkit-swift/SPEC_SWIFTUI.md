# RivetKit SwiftUI API Specification (React-Aligned)

## Overview

SwiftUI-compatible API for RivetKit, aligned with React `useActor` / `useEvent` behavior:
- get-or-create actor identity
- lifecycle driven by `enabled`
- events deliver positional args
- actions are async
- optional fire-and-forget convenience

## Design Principles

1. **Declarative** — state-driven UI updates via events, not imperative return values
2. **Familiar** — follows Apple conventions (`@PropertyWrapper`, view modifiers)
3. **Safe** — automatic lifecycle management, no manual cleanup
4. **Minimal** — small API surface, progressive disclosure

---

## Configuration (React-aligned)

SwiftUI accepts configuration the same way React’s `createRivetKit` does: endpoint string or full config.

```swift
ContentView()
    .rivetKit("https://example.com/api/rivet")

// or

ContentView()
    .rivetKit(ClientConfig(endpoint: "...", token: "...", namespace: "..."))
```

### View Modifier

```swift
extension View {
    func rivetKit(_ endpoint: String) -> some View
    func rivetKit(_ config: ClientConfig) -> some View
}
```

- Stores a `RivetKitClient` in SwiftUI environment.
- `@Actor` reads the client from environment. If none is set, it uses a default `ClientConfig()`.
- Configuration errors are surfaced via `onActorError` using `ActorError(group: "client", code: "config_error", ...)`.

---

## Core API

### @Actor Property Wrapper

```swift
@Actor("counter", key: ["my-counter"]) var counter
```

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `name` | `String` | Yes | Actor name from registry |
| `key` | `String` or `[String]` | Yes | Actor instance key |
| `params` | `Encodable?` | No | Connection parameters |
| `createWithInput` | `Encodable?` | No | Input for actor creation |
| `createInRegion` | `String?` | No | Region hint for creation |
| `enabled` | `Bool` | No | Whether the actor is active (default `true`) |

**Behavior (React parity):**
- Always uses **get-or-create** semantics.
- When `enabled == false`, any active connection is disposed and state resets to idle.
- When re-enabled, a new connection is created.
- No manual `connect()` / `disconnect()` API.

**Exposed Properties:**

```swift
actor.connStatus   // ActorConnStatus (idle, connecting, connected, disconnected)
actor.error        // ActorError? (connection + decode errors)
actor.connection   // ActorConnection? (nil until connected)
actor.handle       // ActorHandle? (stateless handle for HTTP actions)
actor.hash         // String (stable identity hash)
actor.opts         // ActorOptions (normalized inputs)
actor.isConnected  // Bool (derived: connStatus == .connected)
```

### ActorConnStatus Enum

```swift
public enum ActorConnStatus: String, Sendable {
    case idle
    case connecting
    case connected
    case disconnected
}
```

---

## Actions (Async)

Async actions match React behavior. Action errors are delivered via `throws` (not `onActorError`).

```swift
let count: Int = try await actor.action("getCount")
let user: User = try await actor.action("getUser", userId)
```

**Overloads:**

```swift
func action<R: Decodable>(_ name: String) async throws -> R
func action<A: Encodable, R: Decodable>(_ name: String, _ a: A) async throws -> R
func action<A: Encodable, B: Encodable, R: Decodable>(_ name: String, _ a: A, _ b: B) async throws -> R
func action<A: Encodable, B: Encodable, C: Encodable, R: Decodable>(_ name: String, _ a: A, _ b: B, _ c: C) async throws -> R
func action<R: Decodable>(_ name: String, args: [any Encodable]) async throws -> R
```

---

## Fire-and-Forget (Convenience)

Mirror JS by ignoring the async result.

```swift
Button("+") {
    counter.send("increment", 1)
}
```

**Overloads:**

```swift
func send(_ name: String)
func send<A: Encodable>(_ name: String, _ a: A)
func send<A: Encodable, B: Encodable>(_ name: String, _ a: A, _ b: B)
func send<A: Encodable, B: Encodable, C: Encodable>(_ name: String, _ a: A, _ b: B, _ c: C)
func send(_ name: String, args: [any Encodable])
```

**Behavior:**
- Implemented as `Task { _ = try? await action(...) }`.
- Errors are intentionally dropped (do not trigger `onActorError`).

---

## Event Handling

### View Modifier (Primary)

```swift
.onActorEvent(actor, "newCount") { (count: Int) in
    self.count = count
}
```

**Overloads (positional args):**

```swift
.onActorEvent(actor, "tick") { () in }
.onActorEvent(actor, "newCount") { (count: Int) in }
.onActorEvent(actor, "move") { (x: Double, y: Double) in }
.onActorEvent(actor, "triple") { (a: String, b: Int, c: Bool) in }

// Raw (deprecated) - receives all args
.onActorEvent(actor, "event") { args in }
```

**Decoding Rules:**
- Typed overloads decode each positional argument independently from the event args array.
- Expected arity must match exactly. Mismatched arity is treated as a decode error.
- Decode failures are surfaced to `onActorError` using `ActorError(group: "client", code: "decode_error", ...)`.

### AsyncSequence (Advanced)

```swift
.task {
    for await message in actor.events("message", as: Message.self) {
        messages.append(message)
    }
}
```

---

## Error Handling

### Connection + Decode Errors

```swift
.onActorError(counter) { error in
    showAlert(error.message)
}
```

- Connection-level errors (socket close, scheduling errors).
- Decode errors from typed `onActorEvent` overloads.
- Configuration errors (invalid endpoint / config).

---

## Lifecycle (Enabled-Driven)

```swift
@State private var isEnabled = true
@Actor("chat", key: "room-1", enabled: isEnabled) var chat

var body: some View {
    Toggle("Connected", isOn: $isEnabled)
}
```

- `enabled == false` disposes the connection and resets status to `.idle`.
- `enabled == true` reconnects.

---

## Appendix: Public API Reference

### ActorObservable

```swift
@MainActor
public final class ActorObservable: ObservableObject {
    @Published public private(set) var connStatus: ActorConnStatus
    @Published public private(set) var error: ActorError?
    @Published public private(set) var connection: ActorConnection?
    @Published public private(set) var handle: ActorHandle?
    @Published public private(set) var hash: String
    @Published public private(set) var opts: ActorOptions

    public var isConnected: Bool { connStatus == .connected }

    // Async actions
    public func action<R: Decodable>(_ name: String) async throws -> R
    public func action<A: Encodable, R: Decodable>(_ name: String, _ a: A) async throws -> R
    public func action<A: Encodable, B: Encodable, R: Decodable>(_ name: String, _ a: A, _ b: B) async throws -> R
    public func action<A: Encodable, B: Encodable, C: Encodable, R: Decodable>(_ name: String, _ a: A, _ b: B, _ c: C) async throws -> R
    public func action<R: Decodable>(_ name: String, args: [any Encodable]) async throws -> R

    // Fire-and-forget
    public func send(_ name: String)
    public func send<A: Encodable>(_ name: String, _ a: A)
    public func send<A: Encodable, B: Encodable>(_ name: String, _ a: A, _ b: B)
    public func send<A: Encodable, B: Encodable, C: Encodable>(_ name: String, _ a: A, _ b: B, _ c: C)
    public func send(_ name: String, args: [any Encodable])

    // Event streams
    public func events<T: Decodable>(_ name: String, as: T.Type) -> AsyncStream<T>
    public func events<T: Decodable & Sendable>(_ name: String, as: T.Type = T.self) -> AsyncStream<T>
    public func events(_ name: String, as: Void.Type = Void.self) -> AsyncStream<Void>
    public func events<A: Decodable & Sendable, B: Decodable & Sendable>(_ name: String, as: (A, B).Type) -> AsyncStream<(A, B)>
    public func events<A: Decodable & Sendable, B: Decodable & Sendable, C: Decodable & Sendable>(_ name: String, as: (A, B, C).Type) -> AsyncStream<(A, B, C)>
    @available(*, deprecated)
    public func events(_ name: String) -> AsyncStream<[JSONValue]>
}
```

### View Modifiers

```swift
extension View {
    func onActorEvent(
        _ actor: ActorObservable,
        _ event: String,
        perform: @escaping () -> Void
    ) -> some View

    func onActorEvent<A: Decodable>(
        _ actor: ActorObservable,
        _ event: String,
        perform: @escaping (A) -> Void
    ) -> some View

    func onActorEvent<A: Decodable, B: Decodable>(
        _ actor: ActorObservable,
        _ event: String,
        perform: @escaping (A, B) -> Void
    ) -> some View

    func onActorEvent<A: Decodable, B: Decodable, C: Decodable>(
        _ actor: ActorObservable,
        _ event: String,
        perform: @escaping (A, B, C) -> Void
    ) -> some View

    func onActorEvent(
        _ actor: ActorObservable,
        _ event: String,
        perform: @escaping ([JSONValue]) -> Void
    ) -> some View

    func onActorError(
        _ actor: ActorObservable,
        perform: @escaping (ActorError) -> Void
    ) -> some View
}
```

### @Actor Property Wrapper

```swift
@propertyWrapper
public struct Actor: DynamicProperty {
    public init(
        _ name: String,
        key: String,
        params: (any Encodable)? = nil,
        createWithInput: (any Encodable)? = nil,
        createInRegion: String? = nil,
        enabled: Bool = true
    )

    public init(
        _ name: String,
        key: [String],
        params: (any Encodable)? = nil,
        createWithInput: (any Encodable)? = nil,
        createInRegion: String? = nil,
        enabled: Bool = true
    )

    public var wrappedValue: ActorObservable { get }
    public var projectedValue: ActorObservable { get }
}
```
