# Swift SDK Guidelines

## Platform Support

- Target only the latest Swift and OS versions, and do not add backwards compatibility for older versions.
- Prefer latest-language features (for example `@Observable` and Swift concurrency) instead of compatibility workarounds.

## API Surface

- We intentionally support only CBOR encoding in the Swift client. Do not add JSON or vbare protocol support. This is a deliberate simplification.
- We use Swift parameter packs (Swift 5.9+) for variadic typed arguments in `action()` and `send()` methods.
- For event handlers (`on()`/`once()`), we provide overloads up to 5 arguments since closures cannot declare parameter packs.
- Do not expose raw JSON types (e.g., `JSONValue`) in user-facing APIs unless absolutely necessary.
- Prefer typed `Encodable`/`Decodable` overloads so callers can use native Swift types.
- After edits, always build the library or at least one example to ensure changes compile.
- After edits, always run the Swift client driver tests (`swift test`) to validate the client behavior.

### Parameter Packs (Swift 5.9+)

- Use Swift parameter packs to support variadic typed arguments.

```swift
// Supports 0-N typed arguments
try await actor.action("myAction", arg1, arg2, arg3)
actor.send("myAction", arg1, arg2)
```

- Always add `& Sendable` constraint for async contexts
- Shadow parameters before capturing in closures to avoid inference issues

### Examples

**Events (.on / .once)**

```swift
// Single-arg event
conn.on("newCount") { (payload: CounterPayload) in
    print(payload.count)
}

// Two-arg event
conn.on("message") { (from: String, body: String) in
    print("\(from): \(body)")
}

// AsyncStream events
for await value in connection.events("newCount", as: Int.self) {
    print("Count: \(value)")
}
```

**Actions (.action)**

```swift
// No-arg action
let count: Int = try await handle.action("getCount")

// Variadic typed arguments (parameter packs)
let updated: User = try await handle.action("updateUser", UpdateUserInput(name: "Sam"))
let ok: Bool = try await handle.action("setScore", "user-1", 42)
let result: Data = try await handle.action("complex", arg1, arg2, arg3, arg4, arg5, arg6)
```

- Keep unavoidable raw JSON usage (for example debugging or passthrough) in clearly labeled escape-hatch APIs.

### Intentional JSON Exposure (Escape Hatches)

**Error metadata**

```swift
do {
    _ = try await handle.action("doThing", as: String.self)
} catch let error as ActorError {
    // Structured metadata stays as JSONValue
    let metadata = error.metadata
}
```

**Scheduling error details**

```swift
do {
    _ = try await handle.action("doThing", as: String.self)
} catch let error as ActorSchedulingError {
    // Details are JSONValue to preserve backend error payloads
    let details = error.details
}
```

**Raw event args (deprecated)**

```swift
// Use typed overloads first. This is a last-resort escape hatch.
let unsubscribe = await conn.on("event") { args in
    print(args)
}
```

## Deviations from TypeScript Implementation

### ActorCache Cleanup Delay

- **TypeScript (framework-base):** Uses `setTimeout(0)` (0ms async delay)
- **Swift:** Uses `Task.sleep(for: .seconds(5))` (5 second delay)

- The longer delay is required because SwiftUI `@StateObject` in property wrappers may not preserve state across view re-evaluations and `deinit` is dispatched asynchronously via `Task`, so 5 seconds protects connection continuity.

## Logging

- Use the `dev.rivet.*` subsystem prefix for all OSLog categories.

```swift
import os

let logger = Logger(subsystem: "dev.rivet.client", category: "myCategory")
logger.debug("message with \(value, privacy: .public)")
```

- Available categories in `RivetLogger`:
- `client` - RivetKitClient operations
- `connection` - ActorConnection WebSocket lifecycle
- `handle` - ActorHandle HTTP operations
- `manager` - RemoteManager API calls

## Debugging

- Add temporary log statements with `print()` and a clear prefix when code inspection alone is insufficient.

```swift
print("[DEBUG] someFunction called, value=\(value)")
```

- If you cannot run the app yourself, add logs, ask the user to test and share output, and remove temporary logs after resolution.

## Testing Multiple App Instances

- Run multiple instances of the same app when testing features like shared actor connections.
- Launch an additional app instance by running again and clicking **"Add"** instead of "Replace" in Xcode.
- If Xcode kills the first instance without prompting, reset the suppressed dialog with:

```bash
defaults delete com.apple.dt.Xcode IDESuppressStopExecutionWarning
defaults delete com.apple.dt.Xcode IDESuppressStopExecutionWarningTarget
```
