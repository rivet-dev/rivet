# Spec: SQLite Remote Performance Remediation Plan

## Status

Draft

## Summary

This plan addresses the severe latency in remote SQLite writes when RivetKit actors persist SQLite pages over the envoy KV channel.

The current design pays too much fixed cost per page write:

1. The SQLite VFS flushes too many remote writes for one logical SQL operation.
2. The server handles SQLite pages through the generic actor KV path instead of a page-oriented fast path.

The universal fix is not workload tuning. The universal fix in this document is to reduce per-transaction remote overhead for large page sets while preserving SQLite durability and correctness.

## Problem Statement

The current implementation performs well enough for small local examples, but it collapses under larger remote writes and under repeated auto-commit write patterns.

Observed benchmark shape:

- Single inserts scale almost linearly at roughly 12 ms per operation.
- Wrapping those inserts in one transaction improves total time by an order of magnitude.
- Large payload inserts are dominated by actor-side database time before end-to-end action overhead is counted.
- Existing `examples/sqlite-raw/BENCH_RESULTS.md` shows a 1 MiB insert generating hundreds of KV round-trips and hundreds of `put(...)` calls.

This indicates the dominant cost is repeated remote page flush work, not raw SQLite execution time.

This spec is therefore centered on large transaction flushes and large payload writes. Any benefit to small writes is welcome, but it is a side effect, not the main objective.

## Goals

- Improve remote SQLite performance for large transaction flushes and large payload writes without workload-specific tuning.
- Preserve SQLite correctness and failure semantics.
- Preserve current public actor database APIs.
- Reduce fixed overhead per large SQLite transaction.
- Reduce repeated server-side work for SQLite page writes.
- Keep the scope tight enough that unrelated cleanup does not dilute the large-write fix.

## Non-Goals

- No workload-specific page-size tuning modes.
- No user-visible database behavior changes.
- No silent weakening of SQLite durability semantics.
- No broad refactor of unrelated actor KV behavior beyond what the SQLite path requires.
- No modification of an existing published `*.bare` protocol version in place.
- No read-side caching work in this spec.
- No startup preload or open-path work in this spec.
- No transport cleanup work in this spec unless Phase 0 proves it is still blocking large write batches after the storage fix lands.
- No journal-mode redesign in this spec.
- No speculative optimization for small writes beyond what naturally falls out of the large-write fix.

## Design Principles

- Keep the universal behavior. Small writes must not be sacrificed to rescue large writes.
- Keep SQLite as the atomicity authority. The storage path must fail closed and propagate SQLite I/O failures correctly.
- Treat buffering as in-memory coalescing only. Do not turn it into write-behind or early commit acknowledgment.
- Preserve durability boundaries. A commit must not be reported as successful until the same durable storage boundary SQLite expects has been satisfied.
- Optimize transaction flushes, not just packet counts.
- Use specialized server operations only where the generic KV abstraction is actively harmful.
- Prefer fewer remote commits and fewer server transactions over clever client-side heuristics.

## Compatibility Contract

### Protocol compatibility

The preferred MVP does not change the SQLite page-key layout. It changes the transport and server execution path for page mutations.

If we add new internal envoy operations for SQLite page writes:

- we must add them through a new versioned envoy schema, not by mutating the existing published `v1.bare` in place
- we must keep the old generic KV path as a fallback
- a new client talking to an old server must detect missing capability and fall back cleanly
- an old client talking to a new server must continue to use the old generic KV path unchanged

Mixed-version support matrix for the MVP:

- old client + old server: existing generic KV path
- old client + new server: existing generic KV path
- new client + old server: fallback to existing generic KV path
- new client + new server: SQLite fast path when capability is advertised

### Storage compatibility

The preferred MVP keeps the current SQLite KV layout unchanged:

- same page keys
- same file tags
- same file metadata encoding

This avoids forced migration and downgrade complexity for the first release.

If a later phase changes the storage layout, that must be a separate compatibility section with:

- explicit storage version marker
- upgrade path
- downgrade path
- mixed-layout read behavior
- rollout gates

### Retry and idempotency contract

Any new internal SQLite write operation must be safe under duplicate delivery after timeout or reconnect.

For the MVP, replay safety requires both idempotency and fencing:

- every mutating request must carry a monotonic per-file or per-connection commit token, generation, or equivalent compare-and-swap precondition
- the server must reject stale mutating requests whose fencing token is older than the currently committed state
- duplicate delivery of the same mutating request must be safe to replay without changing the already-committed result
- a timed-out old request must not be allowed to overwrite a newer successful commit that completed later

The doc and implementation must not rely on at-most-once delivery assumptions.

For clarity:

- `sqlite_write_batch` is not safe enough if it only means exact page replacement plus optional exact size update
- `sqlite_truncate` is not safe enough if it only means exact target-size convergence
- both operations must also be fenced against stale replay after newer committed state exists

## Evidence From The Current Code

### Client-side database path

- `rivetkit-typescript/packages/rivetkit/src/db/mod.ts` serializes database access through `AsyncMutex`, so each `c.db.execute()` pays one full async database round-trip.
- `rivetkit-typescript/packages/rivetkit-native/src/database.rs` routes every query and mutation through `spawn_blocking` and a single native database mutex.

### SQLite VFS path

- `rivetkit-typescript/packages/sqlite-native/src/kv.rs` stores SQLite files as 4 KiB chunks.
- `rivetkit-typescript/packages/sqlite-native/src/vfs.rs` performs page reads and writes through `batch_get`, `batch_put`, and `delete_range`.
- `kv_io_write` can perform immediate `kv_put` calls when not in batch mode.
- `kv_io_sync` can issue another metadata `kv_put`.
- `SQLITE_FCNTL_BEGIN_ATOMIC_WRITE` and `SQLITE_FCNTL_COMMIT_ATOMIC_WRITE` already exist, but the current path still leaves too much write amplification on the table.

### Server-side storage path

- `engine/packages/pegboard/src/actor_kv/mod.rs` handles SQLite page data through generic actor KV `put`, `get`, and `delete_range`.
- `actor_kv::put` currently estimates total KV size, validates generic limits, clears existing key subspaces, writes generic metadata, and chunks values again before commit.
- That path is structurally more expensive than a SQLite page-store needs to be.

### Transport path

- `engine/sdks/rust/envoy-client/src/connection.rs` uses a single outbound writer path and serializes messages before send.
- `engine/packages/pegboard-envoy/src/ws_to_tunnel_task.rs` handles KV requests inline and sends one response per request.
- The wire format is already binary. JSON and base64 are not the primary bottleneck for the SQLite hot path.

## Root Cause

The main problem is not that packets are tiny. The main problem is that one logical SQLite write becomes too many remotely synchronized page writes, and each page write travels through:

1. A SQLite VFS callback.
2. The native bridge.
3. A websocket request and response.
4. Pegboard-envoy request handling.
5. The generic actor KV transaction path.

The packet count is a symptom of page-level remote commit amplification.

## Proposed Workstreams

## 1. Measure And Strengthen Existing Transaction-Scoped Buffering In The VFS

### Proposal

The VFS already has buffered atomic-write support. The first job is to measure how often SQLite actually enters that path, how often it misses it, and how often the current batch ceiling prevents it from helping.

Only after that measurement should we change the VFS behavior.

Buffering in this spec means collecting the transaction's dirty page set in memory until SQLite reaches its existing commit and sync boundaries. It does not mean acknowledging commit early, flushing in the background after success, or weakening crash durability.

### Changes

- Instrument how often the current engine path reaches `BEGIN_ATOMIC_WRITE` and `COMMIT_ATOMIC_WRITE`.
- Instrument how often writes fall back to immediate `kv_put` from `kv_io_write`.
- Instrument how often `KV_MAX_BATCH_KEYS` prevents the buffered commit path from succeeding.
- If atomic-write coverage is low, investigate whether that is caused by SQLite behavior, our VFS capability signaling, journal-mode interaction, or specific SQL patterns.
- If atomic-write coverage is high but capped by `KV_MAX_BATCH_KEYS`, prioritize reducing the number of page mutations per transaction and/or using a more efficient server-side write path for the existing buffered page set.
- Explicitly evaluate whether the SQLite fast path can safely use a larger batch envelope than the generic actor-KV `128` entry cap, provided it still respects real UniversalDB transaction-size, timeout, and retry constraints.
- Only then expand the VFS write path so page changes stay buffered for the transaction lifecycle wherever SQLite correctness allows it.
- Track dirty pages, file-size changes, and file metadata changes as one page-set mutation.
- Commit the buffered page set once at transaction commit.
- Keep rollback behavior fail-closed. If the buffered commit fails, return SQLite I/O failure and let SQLite unwind.

### Correctness contract

#### Buffering and durability semantics

This spec permits only one kind of buffering:

- in-memory coalescing of dirty pages before the same durable commit boundary SQLite already requires

This spec does not permit:

- write-behind after commit success is reported
- acknowledging `xSync` before the remote durable write has completed
- exposing partially committed page sets as durable state
- relying on best-effort replay of process-local buffers after actor or process death

The durability rule is simple:

- before commit and sync complete, buffered data may be lost and SQLite must treat it as uncommitted
- after commit success and the required sync boundary complete, the page set and size metadata must already be durably stored remotely

The success boundary is also explicit:

- do not return success when pegboard accepts the operation
- do not return success after an internal queue handoff
- do not return success after a logical wrapper transaction starts
- return success only after the underlying storage transaction for all affected page keys, deletion ranges, and file metadata has committed durably

Concrete example:

- acceptable: collect 200 dirty pages in memory during the transaction, write them as one remote durable batch at commit, then return success
- unacceptable: collect 200 dirty pages in memory, return commit success, and let a background task flush them later

If the buffered flush fails:

- return SQLite I/O failure
- do not claim the transaction committed
- let SQLite preserve or recover correctness using its normal failure handling

In other words, buffering is an optimization of how we reach the durable boundary, not a relaxation of the durable boundary itself.

When Phase 1 changes buffering behavior, it must preserve SQLite's existing rollback-journal ordering assumptions for the current `journal_mode = DELETE` path. The spec does not permit reordering journal durability and main-file visibility in a way that would weaken crash recovery.

The MVP preserves the current SQLite operating model instead of broadening it:

- supported journal mode remains `DELETE` unless separately changed
- supported locking model remains the current single-connection `EXCLUSIVE` model
- `xSync` remains a synchronous durability boundary from SQLite’s perspective
- `xLock` and `xUnlock` semantics must remain at least as strict as they are today
- WAL and SHM behavior are not part of the MVP unless explicitly promoted in a later phase

Failure matrix that must be tested:

- successful commit
- rollback before commit
- storage failure during commit
- process death before commit
- process death after commit acknowledgment
- actor stop during write
- reconnect and retry after timeout

### Why it is faster

- One transaction flush can replace many per-page `kv_put` calls.
- Metadata write cost is amortized over the transaction.
- The client pays one remote commit for the page set instead of many remote commits for individual pages.

### Why it is universal

- Small write transactions get lower fixed cost.
- Large write transactions get dramatically fewer remote commits.
- Mixed workloads keep the same SQLite semantics.
- Durability does not get weaker. We are reducing remote chatter, not changing when a write becomes durable.

### Scope guardrail

This workstream exists to fix large-write amplification. Do not expand it into general VFS cleanup unless the cleanup directly removes remote commit amplification for large page sets.

### Risks

- SQLite may not always invoke the atomic-write file-control path the way we want.
- Over-aggressive buffering can break rollback or crash recovery semantics if we are sloppy.

### Mitigation

- Keep SQLite as the atomicity authority.
- Verify behavior with transaction commit, rollback, process exit, actor stop, and crash-style failure tests.

## 2. Add A SQLite-Specific Fast Path On The Server

### Proposal

Introduce a small internal SQLite page-store API between the native VFS path and pegboard instead of routing SQLite pages through the generic actor KV operations.

The MVP is intentionally narrower than the full future surface:

- required in MVP: `sqlite_write_batch`
- likely required in MVP: `sqlite_truncate`
- explicitly not in MVP: `sqlite_read_batch`, `sqlite_open_state`

### New internal operations

#### `sqlite_write_batch` (MVP)

Input:

- `actor_id`
- `file_tag`
- `new_size` when changed
- `pages` as exact page replacements
- optional deletion range for truncated tail pages

Behavior:

- Apply the full page-set mutation in one storage transaction with one atomic visibility boundary.

Correctness requirements:

- page replacements, deletion range updates, and file metadata changes must become visible atomically together
- readers must never observe new metadata with old pages, old metadata with new pages, or a torn subset of the page set
- success may be returned only after the underlying storage transaction has committed durably
- the operation must be fenced so a stale replay cannot overwrite newer committed state

Why faster:

- One server transaction instead of many generic KV puts.
- Direct page-key replacement instead of clear-subspace plus generic metadata rewrite.
- Easier quota accounting for SQLite page storage.

#### `sqlite_truncate` (likely MVP)

Input:

- `actor_id`
- `file_tag`
- `new_size`

Behavior:

- Update file size, trim the last partial page if needed, and delete subsequent pages in one storage transaction with one atomic visibility boundary.

Correctness requirements:

- truncate must be fenced against stale replay the same way as `sqlite_write_batch`
- readers must never observe a truncated size without the matching page deletions, or the reverse
- success may be returned only after the underlying storage transaction has committed durably

Why faster:

- Replaces multiple VFS round-trips with one operation.

### Why this is faster

The generic actor KV path currently does work that SQLite page storage does not need:

- store-size estimation
- generic KV validation and metadata handling
- clear-subspace behavior
- generic chunking logic for arbitrary values

SQLite page storage already has fixed keys, fixed page semantics, and a stronger higher-level authority for correctness.

### Why this is universal

- Small writes benefit from lower fixed server-side cost.
- Large writes benefit from lower transaction count and lower repeated metadata work.
- No workload tuning is required.

### Batch-limit evaluation

The current SQLite buffered commit path inherits a practical batch ceiling from the generic actor-KV path.

That ceiling should not be treated as sacred for the SQLite fast path.

We should explicitly evaluate whether a SQLite-specific write path can raise the effective batch limit safely by using:

- a larger per-request page count
- a larger total request payload
- or both

The decision must be based on the real backend limits and failure modes, not on the current generic actor-KV envelope.

Evaluation criteria:

- serialized request size
- server transaction size
- commit latency at representative dirty-page counts
- timeout behavior
- retry and duplicate-delivery idempotency
- mixed-version fallback behavior

Success condition:

- the SQLite fast path is allowed to exceed the generic `128` entry cap if and only if it remains comfortably within real UniversalDB transaction and operational limits.

### Risks

- Requires internal protocol and server changes.
- Must not accidentally fork semantics between SQLite page storage and generic actor KV.

### Mitigation

- Keep the API internal to the SQLite path.
- Keep quotas and namespace checks explicit.
- Keep current generic actor KV unchanged for all non-SQLite callers.

## 3. Replace Generic Actor KV Storage Work With Page-Oriented Storage Logic

### Proposal

Implement the SQLite fast path in pegboard with direct page-store semantics instead of adapting the generic actor KV machinery.

### Changes

- Store exact page blobs by page key.
- Store file metadata separately and minimally.
- Replace clear-and-rebuild logic with direct key replacement.
- Enforce SQLite page-store quotas without calling `estimate_kv_size(...)` on every write batch.
- Handle truncate as direct page-range deletion and size update.

### MVP storage-layout decision

The MVP keeps the current page-key layout and file metadata format.

That means:

- no actor data migration in the first release
- no downgrade hazard caused by data re-encoding
- the primary change is server execution path, not persisted layout

If later work needs a new persisted layout, it must be split into a separate migration plan instead of being smuggled into this performance remediation.

### Why it is faster

- Avoids repeated store-size estimation on the hot path.
- Avoids rewriting generic metadata for every page batch.
- Avoids extra chunk-splitting for already page-sized data.
- Makes server work scale with changed pages rather than with generic KV abstraction overhead.

### Why it is universal

This is not workload-specific. It removes waste from the current server path for every SQLite operation.

### Risks

- Need a quota model that preserves current product limits.
- Need clear accounting for main DB, journal, WAL, and SHM files.

### Mitigation

- Define explicit SQLite page-store accounting.
- Document how SQLite file tags map to quota and limits.

## Deferred Follow-Up Areas

These are intentionally out of scope for this spec unless Phase 0 shows they are still on the critical path after the large-write fix lands:

- read-side caching and locality
- startup preload and open-path work
- transport cleanup after the storage path is fixed
- extra internal read operations beyond what large-write correctness requires

## Rejected Or Deferred Ideas

### Increase SQLite page size as the universal default

Rejected as the primary plan.

Reason:

- Larger pages can help large payloads.
- Larger pages can also increase write amplification for small random writes.
- That is workload-sensitive, so it is not the universal default we want.

### Workload-specific tuning modes

Rejected.

Reason:

- The goal is one good default path.

### Pure transport optimization without storage changes

Rejected as insufficient.

Reason:

- It attacks symptoms instead of the dominant source of cost.

### WAL or alternative journal modes as the MVP fix

Deferred.

Reason:

- The current implementation explicitly uses `journal_mode = DELETE`.
- The codebase already has WAL and SHM file tags, so this is a real design option.
- Changing journal mode changes correctness and recovery assumptions, not just performance.
- We should evaluate WAL separately after measuring how much the existing buffered commit path and server-side write fast path already improve things.

## Rollout Plan

### Phase 0: Instrumentation

- Add end-to-end tracing for VFS reads, writes, syncs, buffered commits, page counts, and bytes.
- Add server tracing for SQLite page-store reads, writes, truncates, and quota accounting.
- Keep the `examples/sqlite-raw` benchmark as the running baseline and comparison harness.
- Add measurement for:
  - atomic-write coverage
  - buffered-commit batch-cap failures
  - server time spent in `estimate_kv_size`
  - server time spent in clear-and-rewrite work
  - effective request sizes and dirty-page counts at failure points
  - whether larger SQLite-specific batch envelopes remain below real UniversalDB limits

Decision gate:

- If improved use of the existing buffered VFS path removes most of the write amplification, defer new protocol work.
- If server generic-KV overhead still dominates, proceed to the fast-path protocol design.

### Phase 1: VFS buffering improvements

- Improve transaction-scoped page buffering and commit behavior.
- Verify correctness before touching protocol shape.

### Phase 2: Internal SQLite write fast path

- Add internal write and truncate operations with explicit capability negotiation.
- Route the native SQLite path through them when the server advertises support.
- Fall back to the existing generic KV path otherwise.
- Test whether the SQLite fast path can safely use a larger batch ceiling than generic actor KV.

### Phase 3: Server page-store implementation

- Implement direct page-store logic in pegboard.
- Preserve quotas, namespace validation, and failure semantics.

## Verification Plan

- Re-run `examples/sqlite-raw` large insert benchmark against a fresh engine and rebuilt native layer.
- Add focused correctness tests for:
  - commit
  - rollback
  - truncate
  - repeated page overwrite
  - actor stop during write
  - simulated storage failure
- Add protocol and rollout tests for:
  - new client + old server fallback
  - old client + new server behavior
  - duplicate request replay
  - timeout followed by retry
  - stale timed-out request replay after a newer successful commit
  - server restart during in-flight page batch
- Add explicit tests for:
  - atomic-write coverage on representative SQL shapes
  - batch-cap failure behavior
  - larger SQLite fast-path batch envelopes versus generic actor-KV batch limits
  - mixed-version canary rollout
- Add performance assertions or benchmark notes for:
  - large payload inserts
  - large transaction inserts

## Success Criteria

- Significant drop in remote large-payload insert latency in `examples/sqlite-raw`.
- Significant drop in total time for large transactions that dirty many pages.
- No regression in rollback or failure behavior.
- No need for workload-specific tuning knobs.

## Open Questions

1. How much of the large-write regression disappears after transaction-scoped buffering alone?
2. Can `sqlite_write_batch` plus `sqlite_truncate` carry most of the gain without broadening the protocol surface?
3. Should SQLite quota accounting live beside generic actor KV quotas or under a dedicated SQLite page-store accounting path?

## Recommendation

Implement the universal fixes in this order:

1. Measure existing atomic-write coverage and strengthen buffered commit behavior where needed.
2. Add a capability-gated internal SQLite write fast path with fallback to generic KV.
3. Implement direct page-oriented pegboard execution behind that path.
4. Do not expand into read-path or transport cleanup unless Phase 0 proves the large-write bottleneck moved there.

This keeps the spec focused on the real disease: large write batches paying too many remote durable page commits.
