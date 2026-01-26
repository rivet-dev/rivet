# Swift SDK Guidelines

## Platform Support

Only target the latest Swift and OS versions. Do not add backwards compatibility for older versions. This allows us to use the latest language features (e.g., `@Observable` macro, Swift concurrency) without workarounds.

## API Surface

- We intentionally support only CBOR encoding in the Swift client. Do not add JSON or vbare protocol support. This is a deliberate simplification.
- Provide positional overloads up to 5 arguments for actions and events. For more than 5 arguments, expose a raw `JSONValue` array overload as the fallback.
- Do not expose raw JSON types (e.g., `JSONValue`) in user-facing APIs unless absolutely necessary.
- Prefer typed `Encodable`/`Decodable` overloads so callers can use native Swift types.
- Provide overloads for multiple-argument callbacks and calls instead of forcing `[JSONValue]` or manual decoding.
- After edits, always build the library or at least one example to ensure changes compile.
- After edits, always run the Swift client driver tests (`swift test`) to validate the client behavior.

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
```

**Actions (.action)**

```swift
// No-arg action
let count: Int = try await handle.action("getCount")

// One-arg action
let updated: User = try await handle.action("updateUser", arg: UpdateUserInput(name: "Sam"))

// Two-arg action
let ok: Bool = try await handle.action("setScore", "user-1", 42)
```

If a raw JSON value is unavoidable (e.g., debugging or passthrough), keep it in a clearly labeled escape hatch API.

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

The longer delay is needed because SwiftUI's `@StateObject` inside property wrappers may not preserve state correctly across view re-evaluations, and `deinit` is dispatched asynchronously via Task. The 5-second delay ensures connections survive SwiftUI's view lifecycle churn.

## Debugging

When you cannot figure out an issue through code inspection alone, add temporary log statements to trace execution flow. Use `print()` with a clear prefix:

```swift
print("[DEBUG] someFunction called, value=\(value)")
```

If you cannot run the app yourself, add the logs and ask the user to test and share the output. Remove all temporary logs once the issue is resolved.

## Testing Multiple App Instances

When testing features like shared actor connections, you may need to run multiple instances of the same app simultaneously.

Run the app again while it's already running. When Xcode prompts "Replace?", click **"Add"** instead of "Replace" to launch an additional instance.

If Xcode automatically kills the first instance without prompting, you may have suppressed the dialog. Reset it with:

```bash
defaults delete com.apple.dt.Xcode IDESuppressStopExecutionWarning
defaults delete com.apple.dt.Xcode IDESuppressStopExecutionWarningTarget
```
