# SQLite Channel Worker Executor

## Goal

Replace the current async lease-based native SQLite connection manager with a single channel-driven SQLite worker. The worker owns the native SQLite connection for its lifetime, and async callers interact with it through bounded messages.

This intentionally removes parallel reader semantics from the active design. Parallel reads are deferred to `.agent/todo/sqlite-parallel-read-workers.md`.

This is the minimal final native SQLite design. Do not keep legacy read-pool or lease-manager behavior behind a long-term compatibility path.

## Current Problem

The current native SQLite path uses `NativeConnectionManager` to lease `NativeConnection` values to async callers. Each query moves a SQLite connection into `tokio::task::spawn_blocking`, runs a closure, then moves the connection back into the manager.

That shape works, but it creates avoidable complexity:

- SQLite handles move across blocking-pool tasks.
- `NativeDatabase` needs `Send` for query execution.
- Connection close/drop must be carefully pushed through blocking contexts.
- Shutdown has to coordinate async manager state with blocking SQLite cleanup.
- Read/write pool state is more complicated than the current safety target requires.

## Implementation Strategy

Prefer deleting and replacing the lease-manager/read-pool implementation over incrementally mutating it into a worker. Build the worker executor as a fresh module with a narrow public surface, port the necessary VFS/query helpers into that shape, then remove the old manager code, read-pool flags, read-pool metrics, `ExecuteRoute::Read`, and separate native `execute_write` path once tests pass.

## Proposed Shape

Introduce one actor-database-local SQLite executor:

```text
NativeDatabaseHandle
  SqliteWorkerHandle
    bounded command sender
    priority close/control signal
    shared state for close/idempotence/metrics/worker death
    OS worker thread
      owns NativeVfsHandle
      owns one readwrite sqlite3*
```

All SQL work runs on that one worker. There are no reader workers, no read pool, no parser admission layer, and no cross-connection routing.

Async callers send a command over a bounded channel and await a `oneshot` reply. The worker executes commands serially against its owned SQLite connection.

## Non-Goals

- Do not run parallel reads in v1.
- Do not add an SQL parser in v1.
- Do not open readonly SQLite connections in v1.
- Do not implement a SQLite lock ladder in v1.
- Do not change depot storage semantics or VFS page format.
- Do not keep `execute_write` as a separate native worker operation.
- Do not keep `ExecuteRoute::Read`.
- Do not keep read-pool metrics or read-pool optimization flags.

## Core Invariants

- Exactly one SQLite connection exists per native actor database handle.
- The worker is the only owner of the `sqlite3*`.
- The worker is the only code path that calls SQLite execution APIs for that handle.
- The VFS is registered once before worker execution and unregistered only after the worker has closed the SQLite connection and exited.
- Commands are executed in receive order.
- Queue-full returns `actor.overloaded`; callers must not await channel capacity.
- Close is idempotent and prevents new work from being accepted.
- Worker failure is fatal to the owning actor. A panicked or unexpectedly exited SQLite worker must stop/crash the actor instead of letting the actor continue with a broken database handle.

## Command API

Public native database methods become worker commands:

```rust
enum SqliteCommand {
	Execute {
		sql: String,
		params: Option<Vec<BindParam>>,
		reply: oneshot::Sender<Result<ExecuteResult>>,
	},
	Exec {
		sql: String,
		reply: oneshot::Sender<Result<QueryResult>>,
	},
}
```

The exact type names can differ. The important boundary:

- Use a bounded channel.
- Use `try_reserve` or `try_send`.
- Map full queue to `actor.overloaded`.
- Do not expose `sqlite3*` or `NativeConnection` outside the worker.
- Treat a closed worker channel as a structured SQLite closed/shutdown error.
- Do not send close over the bounded SQL command queue. Close uses a priority control signal that cannot be blocked by queued SQL work.
- Default SQL command queue capacity is 128 commands per actor database. This is intentionally bounded per actor so a single actor cannot build an unbounded native SQL backlog.

Prefer Tokio-shaped APIs where they fit. `tokio::sync::mpsc::Receiver::blocking_recv` is available, so a standalone sync thread can receive from Tokio mpsc. However, the worker also needs priority close/control observation. If Tokio mpsc cannot express that without polling, sleeps, runtime dependencies, or close messages queued behind SQL work, use `crossbeam_channel::bounded` for worker SQL/control channels and keep `tokio::sync::oneshot` for replies.

The worker itself should be a real OS thread, not a Tokio task. SQLite and the custom VFS are synchronous. A dedicated OS thread gives the connection, final VFS handle, panic boundary, and join handle one clear owner. The concern with `spawn_blocking` is that it is meant for short blocking closures, cannot truly abort running blocking work, depends on the Tokio runtime during shutdown, and can become detached if the join handle is dropped.

Preferred implementation order:

1. Try `tokio::sync::mpsc` with `blocking_recv` only if close can still bypass SQL queue capacity and be observed before queued SQL dispatch.
2. Otherwise use `crossbeam_channel::bounded` for worker SQL/control channels.
3. Use `tokio::sync::oneshot` for async command replies in either case.

Do not use polling sleeps to bridge async and sync channel APIs.

## Worker Execution

The worker opens and configures one readwrite connection:

```text
open shared VFS
sqlite3_open_v2(... SQLITE_OPEN_READWRITE | SQLITE_OPEN_CREATE ...)
configure_connection_for_database
verify_batch_atomic_writes
receive commands until close
close sqlite3*
drop NativeVfsHandle
```

`execute` and `exec` reuse the single-connection query implementation:

- `exec_statements`
- `execute_single_statement`
- `configure_connection_for_database`
- `verify_batch_atomic_writes`

Because all work runs on one connection, statement classification is not used in the native worker executor. `execute_single_statement` remains responsible for single-statement validation. Delete `ExecuteRoute::Read` from the shared SQLite result surface. There is no separate native `execute_write` path in this design. `execute` reports `WriteFallback`.

## Routing Semantics

There is no reader/writer lane selection.

- `exec(sql)` runs on the worker connection.
- `execute(sql, params)` runs on the worker connection and reports `ExecuteRoute::WriteFallback`.
- `query` and `run` remain projections over `execute` where possible.
- Any public `execute_write` compatibility wrapper should be removed or collapsed to `execute` before this design is considered complete.
- Manual transaction statements naturally stay on the same connection because every command uses the same worker-owned handle.
- There is no legacy read route. Remove `ExecuteRoute::Read` rather than preserving a dead route variant.

This intentionally preserves connection-affine behavior for:

- `last_insert_rowid()`
- `changes()`
- `total_changes()`
- temp tables
- PRAGMAs
- explicit transactions
- user/session connection state

This is desired behavior. Treat it as the baseline native SQLite semantics, not as compatibility with the removed read-pool design.

## VFS And Page Cache

Keep one shared `SqliteVfs` / `VfsContext` for the worker connection. The VFS page cache can remain `moka::sync::Cache`.

Since v1 has only one SQLite connection, the current VFS assumptions remain valid:

- no parallel reader handles
- no reader role tagging
- no reader access to dirty writer pages
- no need for per-reader snapshot management
- no read-only open path

The worker design should still keep VFS lifecycle explicit:

- VFS registration happens before worker connection open.
- The worker owns a `NativeVfsHandle` clone for as long as SQLite may call VFS callbacks.
- Close waits for the worker to close SQLite before dropping the final VFS handle.
- A close timeout may return an error to the caller, but must not unregister/drop the VFS while the worker may still be inside SQLite/VFS.
- If a close timeout happens while the worker is still executing SQLite/VFS work, the worker continues to own the SQLite connection and `NativeVfsHandle` until it naturally exits. The actor must remain in stopping/crashed state and must not accept new work.
- If SQLite/VFS attempts envoy/depot transport after actor shutdown has begun, map the rejected transport call to a `sqlite` VFS error, set `last_error`, return the appropriate SQLite I/O error code from the callback, and let that bubble back through SQLite to the worker command result or close path. The VFS may mark itself dead, but the worker still owns and closes the connection before VFS unregister.

## Backpressure

The public command queue is bounded. Sending work must not await capacity.

Required behavior:

- `try_send`/`try_reserve` success enqueues work.
- queue full returns `actor.overloaded`. This is an intentional behavior change from waiting on internal SQLite capacity.
- worker closed returns a structured closed/shutdown error.
- cancelled queued requests are dropped before execution when observed. It is acceptable in v1 for cancelled queued requests to occupy bounded channel capacity until the worker observes them.
- cancelled active requests are allowed to finish; the reply is discarded if the receiver is gone.
- SQL command queue capacity defaults to 128 commands per actor database. Make it a constant first; add configuration later only if production data shows it is needed.

Do not add retry loops or larger waits to hide overload.

## Shutdown

Close sequence:

1. Mark handle closing so new calls fail fast.
2. Send a priority close/control signal that bypasses the bounded SQL command queue.
3. Worker stops accepting further SQL commands.
4. Queued-but-not-active SQL commands are failed with a structured closing/shutdown error when the worker observes them.
5. The active command, if any, is allowed to finish. v1 does not use `sqlite3_interrupt`.
6. Worker closes the SQLite connection on its own thread.
7. Worker exits.
8. Final `NativeVfsHandle` drops after the worker has exited.

If a worker cannot be joined before the close budget, the handle may report close timeout, but implementation must keep the VFS and connection ownership alive until the worker actually exits. Do not unregister the VFS underneath a potentially running SQLite callback.

The worker event loop must give the close/control signal priority over SQL commands. If it waits on both control and SQL channels, it must check the close signal before dispatching any queued SQL. If the worker is currently inside a synchronous SQLite call, the close signal is observed only after that call returns.

The close/control signal does not introduce parallel SQL execution. Acceptable implementations include a priority select over control and SQL channels, a separate atomic closing flag plus wake signal, or another equivalent mechanism. In all cases, the worker dispatches at most one SQL command at a time and checks close state before dispatching the next queued SQL command.

The worker loop is synchronous/blocking. A concrete implementation can look like this:

```rust
struct SqliteWorkerHandle {
	sql_tx: crossbeam_channel::Sender<SqliteCommand>,
	close_tx: crossbeam_channel::Sender<CloseRequest>,
	closing: Arc<AtomicBool>,
	join: std::thread::JoinHandle<WorkerExit>,
}

impl SqliteWorkerHandle {
	fn execute(&self, command: SqliteCommand) -> Result<()> {
		if self.closing.load(Ordering::Acquire) {
			return Err(sqlite_closing_error());
		}
		self.sql_tx.try_send(command).map_err(|err| match err {
			TrySendError::Full(_) => actor_overloaded_error(),
			TrySendError::Disconnected(_) => sqlite_worker_dead_error(),
		})
	}

	fn close(&self, request: CloseRequest) {
		if !self.closing.swap(true, Ordering::AcqRel) {
			// This is a control path, not SQL work, so it must not be blocked by
			// a full SQL queue.
			let _ = self.close_tx.try_send(request);
		}
	}
}

fn worker_loop(
	sql_rx: crossbeam_channel::Receiver<SqliteCommand>,
	close_rx: crossbeam_channel::Receiver<CloseRequest>,
	mut db: NativeConnection,
	vfs: NativeVfsHandle,
) {
	loop {
		if let Ok(close) = close_rx.try_recv() {
			fail_queued_sql(&sql_rx, sqlite_closing_error());
			break;
		}

		crossbeam_channel::select_biased! {
			recv(close_rx) -> close => {
				let _ = close;
				fail_queued_sql(&sql_rx, sqlite_closing_error());
				break;
			}
			recv(sql_rx) -> command => {
				let Ok(command) = command else {
					tracing::error!("sqlite worker command channel dropped without clean close");
					break;
				};
				if command.reply_is_closed() {
					continue;
				}
				run_one_sql_command(&mut db, command);
			}
		}
	}

	drop(db);
	drop(vfs);
}
```

The exact channel library can differ, but the implementation must keep the same properties: non-awaiting bounded SQL send, close path independent of SQL queue capacity, biased close observation before SQL dispatch, and synchronous one-command-at-a-time SQLite execution. `tokio::sync::mpsc::blocking_recv` is acceptable for a single queue, but a two-queue priority close design may be cleaner with `crossbeam_channel::select_biased!`. If Tokio mpsc is used, tests must prove close while the SQL queue is full does not block and queued SQL does not dispatch after close starts. If the close-control channel has capacity one, `Full` means close was already requested and is not an overload condition.

If the SQL command channel is dropped without a clean close request, the worker must stop after logging an unclean-close error. It should close the SQLite connection and drop the VFS in the normal worker-owned order.

If the worker panics or exits unexpectedly, the shared handle state is marked dead and the owning actor is stopped/crashed. Future SQL calls fail with a structured worker-dead error. Queued callers should receive a structured worker-dead error where possible.

Worker failure must cross the depot-client/core boundary as a fatal SQLite runtime event. `rivetkit-core` should translate that event into the actor's crashed/stopping path and report it to envoy with the existing stop/error path. Today that means using the actor stop path that ultimately sends `EnvoyHandle::stop_actor(actor_id, generation, Some(error_message))` or fails the existing stop handle so envoy receives `ActorStateStopped` with `StopCode::Error`. Depot-client should not talk to envoy directly. The actor must not keep serving work after the SQLite worker has died.

## Required Implementation Comments

Add short comments in the implementation for each non-obvious behavior below:

- Why close uses a priority control signal instead of the bounded SQL queue.
- Why the worker checks close state before dispatching queued SQL.
- Why close timeout does not drop or unregister the VFS while the worker may still be inside SQLite/VFS.
- Why envoy/depot shutdown rejection is returned through VFS as a SQLite I/O error.
- Why dropping the SQL command channel without clean close is logged as an unclean close.
- Why worker panic/unexpected exit is fatal to the actor and must be reported through core lifecycle.
- Why `actor.overloaded` is returned instead of waiting for SQL queue capacity.
- Why the SQL command queue capacity is fixed at 128 in the first implementation.
- Why native worker `execute` returns `WriteFallback` and why `execute_write` was removed/collapsed.
- Why cancelled queued commands may occupy bounded queue capacity until the worker observes them.

## Handle Semantics

`NativeDatabaseHandle` remains cloneable, but all clones point at one worker.

- clones share the same worker sender and close state
- `close()` is idempotent across clones
- `take_last_kv_error()` still reads the shared VFS last error
- `snapshot_preload_hints()` still reads shared VFS hints
- worker metrics replace read-pool metrics
- initialization errors fail `open_database_from_envoy`
- dropped handles do not close the worker until explicit close or the owning actor shutdown path
- the owning actor shutdown path holds the authoritative worker handle until join or timeout bookkeeping is complete

## Metrics And Flags

Remove read-pool metrics and flags from the native SQLite path:

- `sqlite_read_pool_*` gauges, counters, histograms, and mode-transition metrics
- `RIVETKIT_SQLITE_OPT_READ_POOL_ENABLED`
- `RIVETKIT_SQLITE_OPT_READ_POOL_MAX_READERS`
- `RIVETKIT_SQLITE_OPT_READ_POOL_IDLE_TTL_MS`

Replace them with worker metrics:

- SQL command queue depth
- SQL command queue overload count
- SQL command duration
- SQL command error count by code
- worker close duration
- worker close timeout count
- worker crash count
- unclean channel close count

## Superseded SQLite Fix Coverage

The worker executor is intended to supersede several SQLite-specific driver-test complaint branches. Do not delete those branches unless the worker implementation covers their edge cases with tests.

Required coverage:

- **Shutdown database stays closed.** Once actor shutdown or SQLite close begins, new SQL work must fail with a structured closing/shutdown error. This covers the edge case from `05-02-fix_sqlite_keep_shutdown_database_closed`.
- **Reject work after shutdown close.** Queued-but-not-active SQL must not run after close starts. Future calls through any clone must fail fast. This covers the edge case from `driver-test-complaints/close-sqlite-on-shutdown`.
- **VFS registration from async context is safe.** Opening the worker database must not panic by calling `Handle::block_on` directly from inside an active Tokio runtime. If synchronous VFS registration still needs async transport, bridge it with a documented blocking-safe path and explicitly fail unsupported current-thread runtimes. This covers the registration edge of `driver-test-complaints/fix-vfs-register-block-on`.
- **Connection close/drop runs on the worker owner.** SQLite close and any final dirty-page flush happen on the worker-owned context, not on arbitrary async tasks. Close timeout must not unregister/drop the VFS while the worker may still be inside SQLite/VFS. This covers the close/drop edge of `driver-test-complaints/fix-vfs-register-block-on`.

## Rollout

This design is intended to replace the current lease manager, not live beside it permanently. During development, a short-lived opt flag is acceptable for testing, but the final implementation should delete legacy read-pool/lease-manager code once the worker path passes the gates below.

Read-pool flags and metrics should be deleted with the lease-manager/read-pool implementation. Parallel readers remain deferred future work in `.agent/todo/sqlite-parallel-read-workers.md`.

Rollout phases:

1. Add worker handle and command loop.
2. Run worker for all SQL methods.
3. Define final route metadata: delete `Read`; `execute` returns `WriteFallback`.
4. Delete legacy read-pool/lease-manager code, read-pool flags, read-pool metrics, and separate native `execute_write` after worker coverage passes.
5. Run depot-client tests.
6. Run native driver test matrix.
7. Run wasm dependency checks to verify native worker code stays behind native features.
8. Consider making worker executor default only after close, overload, and lifecycle tests pass.

## Test Plan

All new worker-executor tests must live in the `depot-client` crate, under `engine/packages/depot-client/`. Higher-layer driver tests can cover integration behavior, but the worker, channel, shutdown, and VFS-lifetime invariants belong in depot-client tests.

Worker behavior:

- executes `exec`, `query`, `run`, and `execute`
- preserves `last_insert_rowid()`, `changes()`, and temp table behavior across calls
- explicit `BEGIN; ...; COMMIT` sequences stay on the same connection
- command ordering is FIFO
- queue full returns `actor.overloaded`
- closed worker returns structured shutdown error
- worker panic stops/crashes the actor
- worker fatal event is surfaced to the owning runtime so the actor reports crashed/stopping to envoy
- worker channel dropped without clean close logs an unclean-close error and exits
- `execute` returns `WriteFallback`
- `ExecuteRoute::Read` is removed from the native/shared SQLite result surface
- separate native `execute_write` is removed or collapsed to `execute`

Shutdown:

- close while idle
- close while command is active
- close with queued commands
- close while SQL command queue is full
- close prevents queued-but-not-active SQL from running
- new SQL through any clone fails after close starts
- close is idempotent across clones
- VFS is not dropped before worker exit
- close timeout does not unregister VFS under a running worker
- VFS registration/open from an async runtime does not panic
- envoy/depot transport failures during shutdown set VFS last error, return SQLite I/O errors through callbacks, and do not break worker-owned close ordering

Handle behavior:

- final route/result behavior for representative read/write statements
- `take_last_kv_error()` behavior preserved
- `snapshot_preload_hints()` behavior preserved
- worker metrics render and read-pool metrics are gone
- initialization failure propagates from `open_database_from_envoy`

Driver gates:

- run depot-client VFS tests
- run native static/http/bare driver verifier files from `.agent/notes/driver-test-progress.md`
- run wasm dependency gate to ensure native worker is not pulled into wasm builds

## Deferred Parallel Read Design

Parallel read workers are intentionally out of scope. The prior design surfaced unresolved issues around SQLite per-connection caches, idle writer handles, role-aware VFS callbacks, temp files, parser admission, and reader-open network assertions. Keep those notes in `.agent/todo/sqlite-parallel-read-workers.md` for a later design pass.
