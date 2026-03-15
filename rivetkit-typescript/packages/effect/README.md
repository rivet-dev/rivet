# @rivetkit/effect

Effect-TS integration for RivetKit actors with typed context access, lifecycle wrappers, and managed runtime support.

## Installation

```bash
npm install @rivetkit/effect effect
```

## Highlights

- **Typed context service** via `RivetActorContext` — inject Rivet's `ActorContext` into Effect pipelines
- **Effect wrappers** for all actor hooks (`OnCreate`, `OnWake`, `OnDestroy`, etc.) and actions
- **Queue helpers** for message processing with `Queue.next` and `Queue.nextMultiple`
- **ManagedRuntime support** — bring your own Effect layers via the `runtime` option on `actor()`
- **Tagged errors** — `RuntimeExecutionError` and `StatePersistenceError` for precise error handling

## Quick Start

```ts
import { actor, Action, OnCreate, Log } from "@rivetkit/effect";

export default actor({
  state: { count: 0 },

  actions: {
    increment: Action.effect(function* (c, amount: number) {
      yield* Action.updateState(c, (s) => { s.count += amount });
      yield* Log.info("incremented", { amount });
      return yield* Action.state(c);
    }),
  },

  onCreate: OnCreate.effect(function* (c, input) {
    yield* Log.info("actor created");
  }),
});
```

## ManagedRuntime

Bring your own Effect layers (database clients, config, etc.):

```ts
import { actor, Action } from "@rivetkit/effect";
import { Layer, ManagedRuntime } from "effect";

const AppLayer = Layer.mergeAll(DatabaseService.Default, ConfigService.Default);
const AppRuntime = ManagedRuntime.make(AppLayer);

export default actor({
  runtime: AppRuntime,

  actions: {
    query: Action.effect(function* (c) {
      const db = yield* DatabaseService;
      return yield* db.query("SELECT 1");
    }),
  },
});
```

## Error Types

- `RuntimeExecutionError` — wraps unexpected failures during effect execution
- `StatePersistenceError` — wraps failures from `saveState`

Use tag-based handling:

```ts
import { Effect } from "effect";
import { Action } from "@rivetkit/effect";

const save = Action.effect(function* (c) {
  yield* Action.saveState(c, { debounce: 1000 }).pipe(
    Effect.catchTag("StatePersistenceError", (err) =>
      Effect.log(`Failed to save: ${err.message}`),
    ),
  );
});
```

## Queue

```ts
import { Queue, Action, Log } from "@rivetkit/effect";

const processLoop = Action.effect(function* (c) {
  const message = yield* Queue.next(c, "tasks", { timeout: 5000 });
  if (message) {
    yield* Log.info("Processing", { id: message.id, name: message.name });
  }
});
```

## API Reference

### Actor Helpers

| Function | Description |
|---|---|
| `actor(config)` | Creates a RivetKit actor definition with optional `runtime` for ManagedRuntime |
| `RivetActorContext` | Effect Context.Tag for accessing the actor context |
| `Action.effect(fn)` | Wraps a generator as an action handler |
| `Action.state(c)` | Read actor state |
| `Action.updateState(c, fn)` | Mutate actor state |
| `Action.saveState(c, opts)` | Persist state (returns `StatePersistenceError` on failure) |
| `Action.broadcast(c, name, ...args)` | Broadcast to all connections |
| `Action.getConn(c)` | Get the current connection (action context only) |

### Lifecycle Wrappers

All lifecycle namespaces expose an `effect` function that wraps generators:

`OnCreate`, `OnWake`, `OnDestroy`, `OnSleep`, `OnStateChange`, `OnBeforeConnect`, `OnConnect`, `OnDisconnect`, `CreateConnState`, `OnBeforeActionResponse`, `CreateState`, `CreateVars`, `OnRequest`, `OnWebSocket`

### Queue Helpers

| Function | Description |
|---|---|
| `Queue.next(c, name, opts?)` | Receive next message from a single queue |
| `Queue.nextMultiple(c, names, opts?)` | Receive messages from multiple queues |

### Log Helpers

| Function | Description |
|---|---|
| `Log.info(msg, props?)` | Log at info level (requires `RivetActorContext`) |
| `Log.warn(msg, props?)` | Log at warn level |
| `Log.error(msg, props?)` | Log at error level |
| `Log.debug(msg, props?)` | Log at debug level |
