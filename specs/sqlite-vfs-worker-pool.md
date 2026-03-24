# SQLite VFS Worker Pool Spec

Date: 2026-03-21
Package: `@rivetkit/sqlite-vfs`
Status: Draft

## Problem

Today every `SqliteVfsPool` instance runs WASM SQLite on the main thread. All
database operations for all actors are serialized through a single-threaded
event loop via `AsyncMutex`. While KV latency (not CPU) is the primary
bottleneck, this design has two scaling problems:

1. **Head-of-line blocking.** One slow KV round-trip on instance A blocks the
   `AsyncMutex` for all 50 actors on that instance. The main event loop is free
   to run other JS, but no other SQLite operation on that instance can start
   until the current one finishes. With 50 actors and ~200ms per write, worst
   case queue time is ~10 seconds.

2. **Main-thread CPU starvation.** WASM decode/compile and SQLite page-cache
   management consume CPU cycles on the same thread that handles HTTP requests,
   WebSocket frames, and actor lifecycle events. Under load, this manifests as
   increased p99 latency for non-SQLite paths.

Moving each pool instance into its own `worker_threads` Worker solves both:
workers get their own event loop (eliminating cross-instance head-of-line
blocking at the event-loop level) and their own V8 isolate (moving WASM CPU
off the main thread).

## Non-goals

- Changing the KV-backed VFS storage format.
- Changing the pool's bin-packing, sticky assignment, or idle-destroy logic.
- Adding multiple threads per WASM instance. Emscripten asyncify only supports
  one suspended call stack per module instance, so the `AsyncMutex` within each
  instance is still required.
- Browser/edge runtime support. This targets Node.js `worker_threads` only.

## Design

### Architecture overview

```
Main thread                          Worker thread (1 per pool instance)
┌──────────────────────┐             ┌───────────────────────────┐
│ SqliteVfsPool        │             │ SqliteVfsWorker           │
│  ├─WorkerInstance[0]─┼──message──→ │  ├─ SqliteVfs             │
│  ├─WorkerInstance[1]─┼──message──→ │  │   ├─ WASM module       │
│  └─WorkerInstance[N]─┼──message──→ │  │   ├─ AsyncMutex        │
│                      │             │  │   └─ SqliteSystem       │
│ PooledSqliteHandle   │             │  └─ KV proxy (→ main)     │
│  └─ proxies calls ───┼──message──→ │                           │
│     via MessagePort   │             └───────────────────────────┘
└──────────────────────┘
```

Each `PoolInstance` is replaced by a `WorkerInstance` that owns a
`worker_threads.Worker`. The worker thread runs `SqliteVfs` (the existing WASM
module + VFS) inside its own isolate. All database operations are sent to the
worker as structured-clone messages and results are returned via the same
channel.

### Message protocol

Communication uses `MessagePort` (from the Worker's `parentPort` on the worker
side) with a simple request/response protocol. Each request carries a
monotonically increasing `id` used to match responses.

```typescript
// Main → Worker
type WorkerRequest =
  | { id: number; type: "open"; fileName: string; kvPort: MessagePort }
  | { id: number; type: "exec"; dbId: number; sql: string }
  | { id: number; type: "run"; dbId: number; sql: string; params?: unknown[] }
  | { id: number; type: "query"; dbId: number; sql: string; params?: unknown[] }
  | { id: number; type: "close"; dbId: number }
  | { id: number; type: "forceCloseByFileName"; fileName: string }
  | { id: number; type: "forceCloseAll" }
  | { id: number; type: "destroy" };

// Worker → Main
type WorkerResponse =
  | { id: number; type: "ok"; value?: unknown }
  | { id: number; type: "error"; message: string; stack?: string };
```

The `open` message transfers a `MessagePort` for KV operations. This lets each
database's KV calls flow through a dedicated channel without blocking the
main request/response port.

### KV proxy

The `KvVfsOptions` interface (get, getBatch, put, putBatch, deleteBatch)
cannot be transferred to a worker because the callbacks close over
main-thread actor state. Instead, KV operations are proxied:

1. On `open`, the main thread creates a `MessageChannel`. One port is
   transferred to the worker with the open message. The other port stays on
   the main thread and is wired to the actor's real KV callbacks.

2. Inside the worker, the VFS's `KvVfsOptions` implementation sends KV
   requests through the port and awaits responses.

3. On the main thread, the retained port listens for KV requests, calls the
   real KV functions, and posts results back.

```typescript
// Worker-side KV proxy (implements KvVfsOptions)
class WorkerKvProxy implements KvVfsOptions {
  #port: MessagePort;
  #nextId = 0;
  #pending: Map<number, { resolve: Function; reject: Function }>;

  async get(key: Uint8Array): Promise<Uint8Array | null> {
    return this.#rpc("get", { key });
  }
  async getBatch(keys: Uint8Array[]): Promise<(Uint8Array | null)[]> {
    return this.#rpc("getBatch", { keys });
  }
  // ... put, putBatch, deleteBatch follow same pattern

  #rpc(method: string, args: unknown): Promise<unknown> {
    const id = this.#nextId++;
    return new Promise((resolve, reject) => {
      this.#pending.set(id, { resolve, reject });
      this.#port.postMessage({ id, method, args });
    });
  }
}
```

`Uint8Array` values are transferred (zero-copy) via the `transfer` list in
`postMessage` to avoid copying KV payloads across the thread boundary.

### WASM module sharing

The compiled `WebAssembly.Module` is serializable across `worker_threads` via
structured clone. The pool compiles the module once on the main thread (as it
does today) and passes it to each worker in the initialization message. The
worker then calls `WebAssembly.instantiate(module, imports)` to create its own
instance, avoiding redundant compilation.

```typescript
// Main thread: compile once, send to workers
const wasmModule = await WebAssembly.compile(wasmBinary);
worker.postMessage({ type: "init", wasmModule });
```

### WorkerInstance lifecycle

A `WorkerInstance` replaces the current `PoolInstance` in the pool's internal
state. It manages:

- The `Worker` thread.
- Actor assignment bookkeeping (actors, shortNames, poisonedNames). These stay
  on the main thread because they are synchronous lookups used during acquire.
- The pending-request map for correlating responses.
- KV proxy ports for each open database.

```typescript
interface WorkerInstance {
  worker: Worker;
  actors: Set<string>;
  shortNameCounter: number;
  actorShortNames: Map<string, string>;
  availableShortNames: Set<string>;
  poisonedShortNames: Set<string>;
  opsInFlight: number;
  idleTimer: ReturnType<typeof setTimeout> | null;
  destroying: boolean;
  // Request/response tracking
  nextRequestId: number;
  pendingRequests: Map<number, { resolve: Function; reject: Function }>;
  // KV proxy ports per database (keyed by dbId from worker)
  kvPorts: Map<number, { port: MessagePort; options: KvVfsOptions }>;
}
```

### Pool changes

`SqliteVfsPool` changes:

1. **`#createWorkerInstance()`** replaces direct `new SqliteVfs(wasmModule)`.
   Spawns a Worker, sends the init message with the compiled WASM module, and
   returns a `WorkerInstance`.

2. **`openForActor()`** sends an `open` message to the worker with a
   transferred KV `MessagePort`, and registers the main-thread KV listener.

3. **`release()`** sends `forceCloseByFileName` to the worker, awaits the
   response, then cleans up KV ports.

4. **`#destroyInstance()`** sends `forceCloseAll` then `destroy`, waits for
   acknowledgment, then calls `worker.terminate()`.

5. **`shutdown()`** iterates all workers, sends destroy, terminates.

### PooledSqliteHandle changes

`PooledSqliteHandle.open()` returns a `WorkerDatabase` (main-thread proxy)
instead of a `TrackedDatabase`. `WorkerDatabase` implements the same
`Database` interface but sends `exec`/`run`/`query`/`close` messages to the
worker.

```typescript
class WorkerDatabase {
  #pool: SqliteVfsPool;
  #actorId: string;
  #instance: WorkerInstance;
  #dbId: number;

  async exec(sql: string, callback?: Function): Promise<void> {
    // Note: row callbacks cannot cross thread boundary efficiently.
    // For exec-with-callback, fall back to query() and iterate rows
    // on the main thread.
    const result = await this.#sendRequest("exec", { dbId: this.#dbId, sql });
    if (callback && result.rows) {
      for (const row of result.rows) {
        callback(row.values, row.columns);
      }
    }
  }

  async run(sql: string, params?: unknown[]): Promise<void> {
    await this.#sendRequest("run", { dbId: this.#dbId, sql, params });
  }

  async query(sql: string, params?: unknown[]): Promise<{ rows: unknown[][]; columns: string[] }> {
    return await this.#sendRequest("query", { dbId: this.#dbId, sql, params });
  }

  async close(): Promise<void> {
    await this.#sendRequest("close", { dbId: this.#dbId });
  }
}
```

### Worker entry point

The worker script (`worker.ts`) runs in the worker thread:

```typescript
// worker.ts (runs inside worker_threads.Worker)
import { parentPort } from "node:worker_threads";
import { SqliteVfs } from "./vfs";

let vfs: SqliteVfs;
const databases: Map<number, Database> = new Map();
let nextDbId = 0;

parentPort.on("message", async (msg) => {
  switch (msg.type) {
    case "init": {
      vfs = new SqliteVfs(msg.wasmModule);
      parentPort.postMessage({ id: msg.id, type: "ok" });
      break;
    }
    case "open": {
      const kvProxy = new WorkerKvProxy(msg.kvPort);
      const db = await vfs.open(msg.fileName, kvProxy);
      const dbId = nextDbId++;
      databases.set(dbId, db);
      parentPort.postMessage({ id: msg.id, type: "ok", value: dbId });
      break;
    }
    case "run": {
      const db = databases.get(msg.dbId);
      await db.run(msg.sql, msg.params);
      parentPort.postMessage({ id: msg.id, type: "ok" });
      break;
    }
    // ... exec, query, close, forceCloseByFileName, forceCloseAll, destroy
  }
});
```

### Error handling

- Worker crashes (uncaught exception, OOM) are detected via the Worker's
  `"error"` and `"exit"` events. All pending requests are rejected. The pool
  marks the instance as destroying, poisons all short names, and removes all
  actor assignments so they can re-acquire on a fresh instance.

- Individual request errors (SQLite errors, KV failures) are serialized as
  `WorkerResponse` with `type: "error"` and re-thrown on the main thread as
  `Error` instances.

### Thread count and resource budget

Each worker thread adds:
- ~16.6 MB WASM linear memory (same as today, no change).
- ~2-4 MB V8 isolate overhead per worker.
- One OS thread.

At 50 actors per instance with 200 actors, that's 4 workers = 4 threads +
~8-16 MB extra isolate overhead. This is a modest cost for removing
main-thread CPU contention.

A config option `useWorkers` (default: `true`) allows disabling workers for
environments where `worker_threads` is unavailable or undesirable. When
`false`, the pool falls back to the current in-process behavior.

### Configuration changes

```typescript
interface SqliteVfsPoolConfig {
  actorsPerInstance: number;
  idleDestroyMs?: number;
  /** Run each pool instance in a worker thread. Default: true. */
  useWorkers?: boolean;
}
```

Registry config (`RegistryConfig`):

```typescript
sqlitePool: z.object({
  actorsPerInstance: z.number().int().min(1).optional().default(50),
  idleDestroyMs: z.number().optional().default(30_000),
  useWorkers: z.boolean().optional().default(true),
}).optional().default(...)
```

## Data flow: a SQL query

1. Actor calls `db.query("SELECT * FROM foo WHERE id = ?", [42])`.
2. `WorkerDatabase.query()` on the main thread creates a request `{ id, type: "query", dbId, sql, params }` and posts it to the `WorkerInstance`'s worker.
3. The main thread promise awaits the response.
4. Worker receives the message, looks up the `Database` by `dbId`.
5. `Database.query()` acquires the `AsyncMutex`, calls `sqlite3.statements()`, `bind_collection()`, `step()`.
6. During `step()`, SQLite calls VFS `xRead` which calls `kvProxy.getBatch()`.
7. `WorkerKvProxy.getBatch()` posts `{ id, method: "getBatch", args: { keys } }` to the KV `MessagePort`, awaits response.
8. On the main thread, the KV port listener calls the actor's real `kvBatchGet()`, posts the result back through the port (transferring `Uint8Array` buffers).
9. Worker receives KV response, returns data to VFS, SQLite continues.
10. Query completes, worker posts `{ id, type: "ok", value: { rows, columns } }` back.
11. Main thread resolves the promise, returns result to the actor.

## Migration plan

### Phase 1: Worker infrastructure

- Create `worker.ts` entry point with message handler.
- Create `WorkerKvProxy` implementing `KvVfsOptions` over `MessagePort`.
- Create main-thread `KvPortListener` that bridges `MessagePort` to real KV callbacks.
- Create `WorkerDatabase` proxy class.

### Phase 2: WorkerInstance in pool

- Replace `PoolInstance` with `WorkerInstance` in `SqliteVfsPool`.
- Update `#createInstance()` to spawn a Worker and send init with WASM module.
- Update `openForActor()` to create MessageChannel and send open message.
- Update `release()` and `#destroyInstance()` to send messages and await responses.
- Keep all assignment bookkeeping (actors, shortNames, etc.) on main thread.

### Phase 3: Fallback path

- Implement `useWorkers: false` fallback that preserves current in-process behavior.
- Wire up `useWorkers` config in `RegistryConfig`.

### Phase 4: Tests

- Unit tests for `WorkerKvProxy` and `KvPortListener` in isolation.
- Integration tests for `SqliteVfsPool` with `useWorkers: true` covering:
  acquire, open, query, release, idle destroy, shutdown.
- Test worker crash recovery: kill a worker and verify actors can re-acquire.
- Test `useWorkers: false` still works identically to current behavior.

## Risks and mitigations

### KV proxy latency overhead

Each KV call adds a main-thread → worker round-trip (~0.01-0.05ms per
`postMessage` hop on Node.js). With 10-15 KV calls per SQLite write, this adds
~0.2-0.75ms overhead. At 150-225ms per write, this is <0.5% and negligible.

**Mitigation:** Benchmark before/after. If overhead is higher than expected,
batch multiple KV calls into single messages.

### Structured clone cost for query results

Large query results must be structured-cloned from worker to main thread.
For typical actor queries (small result sets), this is negligible.

**Mitigation:** For large results, consider transferring `ArrayBuffer` backing
stores. The `query` response could encode rows as a flat `ArrayBuffer` with an
index, transferred zero-copy, instead of structured-cloned arrays.

### Worker startup time

`Worker` construction + WASM instantiation adds latency to the first acquire
on a new instance. Current path: ~50ms for WASM compile + instantiate. Worker
path: ~20ms Worker spawn + ~30ms WASM instantiate (compile is shared).

**Mitigation:** The WASM module is pre-compiled on the main thread and
transferred to the worker, so compile cost is paid once. Worker spawn is a
one-time cost amortized across all actors on that instance.

### exec() row callback limitation

`Database.exec()` accepts a row callback that is called for each result row.
Callbacks cannot cross the thread boundary. Two options:
- Buffer all rows in the worker and return them in the response (matches
  `query()` behavior). The main thread then calls the callback locally.
- Only support exec-without-callback for worker mode, requiring callers to
  use `query()` for result iteration.

**Decision:** Buffer rows in worker, call callback on main thread. This
preserves the existing API contract with minimal behavior change.

### Thread safety of pool bookkeeping

All pool bookkeeping (actor assignment, short name allocation, instance
selection) runs on the main thread's event loop, which is single-threaded.
No mutex is needed for these structures. Only the worker communication is
async/awaited.
