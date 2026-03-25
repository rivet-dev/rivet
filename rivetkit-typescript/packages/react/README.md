# @rivetkit/react

React hooks for connecting to [Rivet Actors](https://rivet.dev/docs) with automatic state management, realtime events, and Suspense support.

[Documentation](https://rivet.dev/docs) — [Discord](https://rivet.dev/discord) — [Issues](https://github.com/rivet-dev/rivet/issues)

## Installation

```bash
npm install @rivetkit/react rivetkit
```

## Quick Start

```tsx
import { createRivetKit } from "@rivetkit/react";

// Define your actor registry (shared with the server)
import type { Registry } from "./actors";

const { useActor } = createRivetKit<Registry>("https://your-endpoint");

function Counter() {
  const { connection, connStatus } = useActor({ name: "counter", key: "my-counter" });

  return (
    <div>
      <p>Status: {connStatus}</p>
      <button onClick={() => connection?.increment()}>Increment</button>
    </div>
  );
}
```

## API

### `createRivetKit(endpoint?, opts?)`

Creates the React hooks bound to your actor registry. Call this once at the module level and export the hooks.

```ts
import { createRivetKit } from "@rivetkit/react";
import type { Registry } from "./actors";

export const { useActor } = createRivetKit<Registry>("https://your-endpoint");
```

**Parameters**

| Parameter | Type | Description |
|-----------|------|-------------|
| `endpoint` | `string \| ClientConfigInput \| undefined` | Your Rivet endpoint URL or full client config. Omit to use `RIVET_ENDPOINT` env var. |
| `opts` | `CreateRivetKitOptions` | Optional. See [Options](#createrivetkit-options). |

**Returns** `{ useActor }`

---

### `createRivetKitWithClient(client, opts?)`

Same as `createRivetKit` but accepts a pre-constructed `Client` instance. Useful when you need to share a single client across multiple hook factories or in tests.

```ts
import { createClient } from "rivetkit/client";
import { createRivetKitWithClient } from "@rivetkit/react";
import type { Registry } from "./actors";

const client = createClient<Registry>("https://your-endpoint");
export const { useActor } = createRivetKitWithClient(client);
```

---

### `useActor(opts)`

Hook that connects to a Rivet Actor and returns its live connection state. Multiple components calling `useActor` with the same `name` and `key` share a single underlying connection.

```tsx
const { connection, connStatus, error, useEvent } = useActor({
  name: "chatRoom",
  key: roomId,
});
```

**Options**

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `name` | `string` | required | Type-safe name of the actor, matching the registry. |
| `key` | `string \| string[]` | required | Unique key identifying this actor instance. Use an array for multi-segment keys, e.g. `["user", userId]`. |
| `params` | `object` | — | Parameters passed to the actor on connect. Type is inferred from the registry. |
| `enabled` | `boolean` | `true` | Set to `false` to skip connecting. The hook returns `connStatus: "idle"`. Useful for conditional connections. |
| `noCreate` | `boolean` | `false` | If `true`, only connects to an existing actor. Throws if the actor does not exist. |
| `createInRegion` | `string` | — | Region hint for where to create the actor if it doesn't exist. |
| `createWithInput` | `unknown` | — | Input data forwarded to the actor's `onCreate` handler. |
| `suspense` | `boolean` | `false` | If `true`, the component suspends while connecting. Use with `<Suspense>` for declarative loading states. Errors are thrown to the nearest `<ErrorBoundary>`. |

**Return value**

| Field | Type | Description |
|-------|------|-------------|
| `connection` | `ActorConn \| null` | The live actor connection. Use this to call actor actions. `null` until connected. |
| `connStatus` | `ActorConnStatus` | Current connection status: `"idle"`, `"connecting"`, `"connected"`, or `"disconnected"`. |
| `error` | `Error \| null` | Set when the connection fails. |
| `useEvent` | `(event, handler) => void` | Hook to subscribe to actor events. See [useEvent](#useevent). |

---

### `useEvent(event, handler)`

Returned from `useActor`. Subscribes to a typed event emitted by the actor. The listener is automatically re-registered when the connection changes and cleaned up on unmount.

```tsx
const { useEvent } = useActor({ name: "chatRoom", key: roomId });

useEvent("message", (msg) => {
  setMessages((prev) => [...prev, msg]);
});
```

The `handler` reference is kept stable via a ref so you can pass an inline function without triggering re-subscriptions.

---

## Suspense

Set `suspense: true` to suspend the component while the actor is connecting. Wrap the component in `<Suspense>` to show a fallback, and in an `<ErrorBoundary>` to handle connection failures.

```tsx
function ChatRoom({ roomId }: { roomId: string }) {
  // Guaranteed connected — connStatus is always "connected" here
  const { connection } = useActor({ name: "chatRoom", key: roomId, suspense: true });

  return <MessageList connection={connection} />;
}

function App() {
  return (
    <ErrorBoundary fallback={<p>Connection failed.</p>}>
      <Suspense fallback={<p>Connecting…</p>}>
        <ChatRoom roomId="general" />
      </Suspense>
    </ErrorBoundary>
  );
}
```

---

## Conditional Connections

Use `enabled: false` to defer connecting until a condition is met, such as a user being logged in.

```tsx
const { connStatus } = useActor({
  name: "userSession",
  key: userId,
  enabled: !!userId,
});
```

---

## Exports

| Export | Description |
|--------|-------------|
| `createRivetKit` | Create hooks from an endpoint URL or config. |
| `createRivetKitWithClient` | Create hooks from an existing `Client` instance. |
| `createClient` | Re-exported from `rivetkit/client` for convenience. |
| `ActorConnDisposed` | Error class thrown when calling a disposed connection. |
| `ActorConnStatus` | TypeScript type for connection status values. |

## License

Apache 2.0
