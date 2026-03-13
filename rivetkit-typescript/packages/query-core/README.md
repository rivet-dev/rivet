# @rivetkit/query-core

_Framework-agnostic query cache integration utilities for RivetKit_

[Discord](https://rivet.dev/discord) — [Documentation](https://rivet.dev/docs) — [Issues](https://github.com/rivet-dev/rivetkit/issues)

This package is the shared core used by `@rivetkit/react` and future framework integrations. It provides the primitives for syncing actor connection state into any TanStack Query-compatible cache.

## API

### `createActorQueryKey(actorOpts, queryKeyFn?)`

Returns the default query key for an actor instance:

```ts
["rivetkit", "actor", name, key, params | null, noCreate]
```

Pass a custom `queryKeyFn` to override the default shape.

### `syncActorToQueryClient(options)`

Subscribes to an actor's connection state and pushes updates into the query cache via `setQueryData`. Returns `{ queryKey, unsubscribe }`.

The cache entry holds actor **connection metadata** — `connStatus`, `connection`, `handle`, `error`, `opts`. It does not contain actor business state; that arrives via actor events.

### `QueryClientLike`

Minimal interface required by this package — any object with `setQueryData` qualifies, including the standard TanStack `QueryClient`.

## Planned Features

### TanStack DB Integration

Sync actor state directly into a TanStack DB collection instead of the query cache. Components can then use live queries with filtering and sorting over actor state without extra boilerplate.

```ts
import { syncActorToCollection } from "@rivetkit/query-core";

counter.useEvent("newCount", (count) => {
  syncActorToCollection(counterCollection, actorKey, { count });
});
```

### `queryActor` — Zero-Boilerplate State Sync

A server-side actor wrapper that automatically broadcasts `stateChanged` after every action and exposes a `getState` action. The client-side `useActorQuery` detects these and keeps `data.state` in sync with no manual `useEvent` required.

```ts
// Server
export const counter = queryActor({
  state: { count: 0 },
  actions: {
    increment: (c, amount: number) => { c.state.count += amount; },
  },
});

// Client — data.state is always up to date
const { data } = useActorQuery({ name: "counter", key: ["test-counter"] });
data?.state?.count;
```

See [`.agent/notes/query-integration-next-steps.md`](../../../.agent/notes/query-integration-next-steps.md) for full design notes.

## License

Apache 2.0
