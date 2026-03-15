# @rivetkit/react

_React hooks for building real-time apps with Rivet Actors_

[Discord](https://rivet.dev/discord) — [Documentation](https://rivet.dev/docs) — [Issues](https://github.com/rivet-dev/rivetkit/issues)

## Installation

```sh
pnpm add @rivetkit/react @tanstack/react-query
```

## Basic Usage

```tsx
import { createRivetKit } from "@rivetkit/react";
import type { registry } from "./actors.ts";

const { useActor } = createRivetKit<typeof registry>("https://your-api/api/rivet");

function Counter() {
  const counter = useActor({ name: "counter", key: ["global"] });

  return (
    <button onClick={() => counter.connection?.increment(1)}>
      Status: {counter.connStatus}
    </button>
  );
}
```

## TanStack Query Integration

Pass a `QueryClient` to `createRivetKit` and actor connection state is automatically synced into the query cache. Every actor gets one cache entry covering both connection metadata and business state.

### Setup

```tsx
import { createRivetKit } from "@rivetkit/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import type { registry } from "./actors.ts";

const queryClient = new QueryClient();

const { useActor, useActorQuery, createActorQueryKey } = createRivetKit<typeof registry>(
  "https://your-api/api/rivet",
  { queryClient },
);
```

### Cache Entry Shape

Each actor's cache entry holds **connection metadata** pushed automatically by `useActor`:

```ts
{
  connStatus: "connected" | "connecting" | "disconnected" | "idle";
  connection: ActorConn | null;
  handle: ActorHandle | null;
  error: Error | null;
  // ...
}
```

You can merge **business state** into the same entry from actor events (see below). One key covers everything.

### `createActorQueryKey`

Returns the query key for a given actor. When `queryClient` is passed to `createRivetKit`, use the bound version — no registry generic needed:

```ts
const { createActorQueryKey } = createRivetKit<typeof registry>(endpoint, { queryClient });

const key = createActorQueryKey({ name: "counter", key: ["global"] });
// ["rivetkit", "actor", "counter", ["global"], null, false]
```

### `useActorQuery`

Mounts the actor and returns a `UseQueryResult` in one hook. The cache entry stays in sync with connection state automatically:

```tsx
function Counter() {
  const query = useActorQuery({ name: "counter", key: ["global"] });

  return <p>Status: {query.data?.connStatus}</p>;
}
```

### Merging Business State from Events

Actor business state (e.g. `count`) is not in the connection metadata — it arrives via events. Use `useEvent` to merge it into the same cache entry under a `state` field, so consumers read one key for everything:

```tsx
import { type ActorConnState, createRivetKit } from "@rivetkit/react";
import { useQuery } from "@tanstack/react-query";
import { useMemo } from "react";

// Define the merged shape for this actor's cache entry
type CounterEntry = ActorConnState<typeof registry, "counter"> & {
  state?: { count: number };
};

function Counter() {
  const counter = useActor({ name: "counter", key: ["global"] });

  const queryKey = useMemo(
    () => createActorQueryKey({ name: "counter", key: ["global"] }),
    [],
  );

  // Merge business state into the same cache entry on every event
  counter.useEvent("newCount", (count: number) => {
    queryClient.setQueryData<CounterEntry>(queryKey, (prev) => ({
      ...prev,
      state: { count },
    } as CounterEntry));
  });

  // Option A: read via useQuery with the merged type
  const query = useQuery<CounterEntry>({
    queryKey,
    queryFn: () => Promise.resolve(null as unknown as CounterEntry),
    enabled: false,
  });

  // Option B: read via useActorQuery with the merged type as a generic
  const hookQuery = useActorQuery<"counter", CounterEntry>(
    { name: "counter", key: ["global"] },
  );

  return (
    <div>
      <p>Count: {query.data?.state?.count ?? "-"}</p>
      <p>Status: {query.data?.connStatus}</p>
      <button onClick={() => counter.connection?.increment(1)}>+1</button>
    </div>
  );
}
```

### Reading from Cache in Child Components

Any component can read the actor's cache entry without calling `useActor`, as long as `useActor` is mounted somewhere in the tree:

```tsx
function CounterDisplay() {
  const queryKey = createActorQueryKey({ name: "counter", key: ["global"] });

  const query = useQuery<CounterEntry>({
    queryKey,
    queryFn: () => Promise.resolve(null as unknown as CounterEntry),
    enabled: false,
  });

  return <p>{query.data?.state?.count ?? "loading..."}</p>;
}
```

## Planned Features

### `queryActor` — Zero-Boilerplate State Sync

A server-side actor wrapper that broadcasts state after every action. `useActorQuery` will detect it and keep `data.state` up to date with no manual `useEvent`:

```tsx
// No useEvent needed — state syncs automatically
const query = useActorQuery({ name: "counter", key: ["global"] });
query.data?.state?.count;
```

### TanStack DB Integration

Sync actor events into a TanStack DB collection for live queries with filtering and sorting.

See [`@rivetkit/query-core`](../query-core/README.md) for full design notes on both features.

## License

Apache 2.0
