# Actor KV Preloading

Internal reference for the actor startup KV preloading system. For the full design spec, see `/.agent/notes/actor-startup-kv-preload-spec.md`.

## Overview

Actor startup preloads KV data from the engine in a single FoundationDB transaction and delivers it alongside the `CommandStartActor` protocol message. Each subsystem receives its data from a `PreloadMap` instead of performing individual KV round-trips.

This is a one-shot data injection, not a cache. After startup completes, the preload map is discarded. Preloaded KV is NOT persisted in the command queue or workflow history. It is fetched fresh from FDB at send time.

## Key Prefixes

All internal KV keys are defined in `keys.ts`. Each subsystem owns a reserved prefix:

| Prefix | Subsystem | Preload Type | Default Max | Engine Config Key | Notes |
|--------|-----------|-------------|------------|-------------------|-------|
| `[1]` | Persist data (state) | get | n/a | n/a | Single key, always preloaded |
| `[2]` | Connections | list prefix | 64 KB | `preload_max_connections_bytes` | Bounded |
| `[3]` | Inspector token | get | n/a | n/a | Single key, always preloaded |
| `[4]` | User KV | (not preloaded) | n/a | n/a | Deferred to v2 |
| `[5,1,1]` | Queue metadata | get | n/a | n/a | Single key, always preloaded |
| `[6]` | Workflows | list prefix | 128 KB | `preload_max_workflow_bytes` | Released after startup; fallback to KV on miss |
| `[7]` | Traces | (not preloaded) | n/a | n/a | Write-only during normal operation |
| `[8]` | SQLite VFS | list prefix | 768 KB | `preload_max_sqlite_bytes` | Partial preload with KV fallback |

Global hard cap across all prefixes: 1 MB (`preload_max_total_bytes` in engine config). All limits are configurable in `engine/packages/config/src/config/pegboard.rs`.

## PreloadMap Interface

```typescript
interface PreloadMap {
    // Returns value if preloaded, null if preloaded-but-missing, undefined if not preloaded.
    get(key: Uint8Array): Uint8Array | null | undefined;

    // Returns entries if prefix was preloaded, undefined if not preloaded.
    listPrefix(prefix: Uint8Array): [Uint8Array, Uint8Array][] | undefined;
}
```

The three-way return on `get()` is important:
- `Uint8Array` = key exists, here is the value.
- `null` = key was requested but does not exist in storage. Subsystem should treat as "not found" without issuing a KV read.
- `undefined` = key was not part of the preload request. Subsystem must fall back to a normal KV read.

Each prefix scan has a `partial` flag that controls truncation behavior:

- **`partial: true` (SQLite `[8]`):** If data exceeds `max_bytes`, return whatever fits. The VFS does per-key lookups and falls back to KV on miss. Partial data is still useful.
- **`partial: false` (connections `[2]`, workflows `[6]`):** If data exceeds `max_bytes`, return nothing. These subsystems call `listPrefix` expecting the complete set. Partial data would drop entries.

Only the fixed keys (`[1]`, `[3]`, `[5,1,1]`) use the "requested but not found = doesn't exist" semantic.

## Preload Data Lifecycle

| Prefix | Consumed By | Discarded When |
|--------|------------|----------------|
| `[1]` persist data | `#loadState()` | After `#loadState()` returns |
| `[2]` connections | `#restoreExistingActor()` | After `#loadState()` returns |
| `[3]` inspector token | `#initializeInspectorToken()` | After init returns |
| `[5,1,1]` queue metadata | `queueManager.initialize()` | After init returns |
| `[8]` SQLite | `#setupDatabase()` via VFS wrapper | After `#setupDatabase()` completes |
| `[6]` workflows | Workflow engine `loadStorage()` | After startup completes (`#started = true`) |

Preloading is NOT part of the core KV engine. Each subsystem receives data from the `PreloadMap` directly. The core KV layer is unaware of preloading.

## Subsystem Integration Pattern

Every subsystem that reads KV during startup follows the same pattern:

```typescript
async initialize(preload?: PreloadMap) {
    const preloaded = preload?.get(MY_KEY);
    let value: Uint8Array | null;
    if (preloaded !== undefined) {
        // Hit or confirmed-missing from preload
        value = preloaded;
    } else {
        // Not preloaded, fall back to KV
        value = await this.driver.kvBatchGet(this.actorId, [MY_KEY]);
    }
    // ... use value
}
```

For list operations (connections, SQLite, workflows):

```typescript
const preloaded = preload?.listPrefix(MY_PREFIX);
let entries: [Uint8Array, Uint8Array][];
if (preloaded !== undefined) {
    entries = preloaded;
} else {
    entries = await this.driver.kvListPrefix(this.actorId, MY_PREFIX);
}
```

## SQLite Preloading

SQLite data lives under the `[8]` prefix as 4096-byte chunks. The preload is partial: the engine sends up to `preload_max_sqlite_bytes` (default 768 KB, ~192 pages) of data. The VFS wrapper uses a sorted array with binary search for lookups, falling back to KV on miss.

Key points:
- **Partial preload is safe.** A miss in the preload map falls back to KV. The VFS wrapper does NOT use "prefix was fully preloaded" semantics.
- **Write consistency is handled by SQLite.** SQLite's internal page cache ensures that pages written via `xWrite` are served from memory on subsequent reads. The preload map does not need write-through.
- **Preload map is cleared after onMigrate.** After database setup completes, the preload map is discarded. All subsequent VFS operations go through normal KV.
- **No hex string keys.** The preload data is stored as a sorted `[Uint8Array, Uint8Array][]` array with binary search, avoiding string conversion overhead.

## Workflow Preloading

Workflow data lives under the `[6]` prefix. For actors that use workflows, the engine preloads up to `preload_max_workflow_bytes` (default 128 KB). Typical workflow data is 2-15 KB for simple workflows.

Workflow preload data is released eagerly after startup completes (`#started = true`). If the workflow engine hasn't consumed it by then, the data is discarded and the workflow engine falls back to normal KV reads. This bounds memory lifetime to the startup window.

## Unexpected Round-Trip Detection

The `#expectNoKvRoundTrips` flag on `ActorInstance` warns when KV reads happen during startup that should have been preloaded:

1. Set to `true` after preload data is consumed, before subsystem init.
2. Paused (`false`) during user code callbacks (`onWake`, `createVars`, `createState`).
3. Restored after user code returns.
4. Cleared after startup completes (`#started = true`).

Any KV read while the flag is `true` logs a structured warning via `this.#rLog.warn`. The flag flips to `false` after the first warning to avoid log spam.

## Write Batching (New Actors)

New actors batch their initialization writes into a single `kvBatchPut`:

| Write | Key | Previously |
|-------|-----|-----------|
| Persist data | `[1]` | Separate write after `createState()` |
| Queue metadata | `[5,1,1]` | Separate write in `queueManager.initialize()` |
| Inspector token | `[3]` | Separate write in `#initializeInspectorToken()` |

The `WriteCollector` interface collects writes from each subsystem and flushes them in a single round-trip after all subsystem initialization completes.

## Protocol

Preloaded data is delivered alongside `CommandStartActor` (protocol v8). The preloaded KV is populated at send time (not persisted in workflow history or command queue) to avoid storage bloat.

The engine determines what to preload from the actor name metadata sent during runner init (`prepopulateActorNames`). This metadata is refreshed on every WebSocket connection/reconnection.

If the FDB transaction to fetch preloaded data fails, the actor start fails (no silent fallback).

## Adding New KV Reads to Startup

When adding a new KV read to the actor startup path:

1. Add the key/prefix to the automatic preload list in the engine.
2. Update the subsystem to accept a `PreloadMap` parameter and check it before issuing a KV read.
3. Verify the `#expectNoKvRoundTrips` flag does not fire warnings for the new key.
4. If the key uses a new prefix, reserve a slot in `keys.ts` and document it in this file.
5. If the prefix is unbounded, add a `preload_max_*_bytes` limit in the engine config (`engine/packages/config/src/config/pegboard.rs`) with a per-actor override in actor options. Do not hard-code constants.
