# Envoy Load Balancing

How pegboard picks which envoy hosts a newly-allocated actor, the constraints driving the design, and the knobs available for tuning.

## Constraints

The envoy load balancer must satisfy three constraints simultaneously:

1. **Low subspace contention.** Many engine nodes allocate actors concurrently. Reading the full envoy subspace per allocation is not viable — under surge it pile-ups on the FDB shards owning that range and degrades the whole system. Each allocation must touch a bounded, small number of FDB ranges.
2. **Approximate load balancing.** Allocations should spread across envoys roughly evenly. Exactness is not required; the system tolerates a max-to-mean allocation ratio of ~2× for the uniform variant. A load-aware variant (power-of-K choices over `SlotsKey`) is available as an opt-in strategy for pools where measured slot variance justifies the extra reads.
3. **Surge tolerance.** Bursts of allocations into the same (namespace, pool) must not pile onto one envoy or one FDB shard. Independent randomness across allocators is required; reliance on a single counter or shared cache is not acceptable.

## Design

The strategy is a **virtual-node ring** keyed by xxh3 hashes of `envoy_key`, with a `samples` (K) knob that tunes between uniform pick and power-of-K-choices.

- Each envoy occupies V positions (V=8 by default) on a u128 ring (`[u8; 16]` big-endian positions).
- Positions are `xxh3_128_with_seed(envoy_key, i).to_be_bytes()` for `i in 0..V`. Deterministic and stable for the envoy's lifetime.
- The allocator picks K independent random `[u8; 16]` pivots:
  - **K=1**: returns the first fresh envoy after the pivot. **Short-circuits past the `SlotsKey` read** — no comparison to make. 1 `EnvoyLoadBalancerIdxKey` range read for highest-version discovery + 1 `EnvoyHashIdxKey` range read + 1 `LastPingTsKey` lookup = 3 snapshot reads per allocation.
  - **K≥2**: resolves each pivot to a fresh envoy, reads each candidate's `SlotsKey`, picks min with uniform random tiebreak.
- Single `EnvoyHashIdxKey` index, single connection-init write path. Two expire paths: graceful `pegboard_envoy_expire` (primary, called from envoy disconnect / lost-timeout / eviction) and a per-process read-path `EnvoyExpireScheduler` (secondary, invokes the same `pegboard_envoy_expire` op with `skip_if_fresh: true` when any allocator observes a stale envoy). The scheduler is a stepping stone to a future `pegboard_envoy_stale_sweep` workflow — see [TODO](#todo-replace-with-a-workflow).

### Why this satisfies the constraints

- **Constraint 1**: per-allocation cost is bounded — 3 snapshot reads at K=1 (1 highest-version range read + 1 hash-index range read + 1 `LastPingTsKey` lookup), ≤ 1 + 3K reads at K≥2 (K=2 default = 7). All reads target hash-randomized FDB positions, so allocators land on different shards. No scan of the whole subspace.
- **Constraint 2**:
  - K=1: V vnodes per envoy give an expected max/mean allocation ratio of roughly `1 + ln(N)/V`. At V=8, N=1000 envoys, that's ~1.86×.
  - K≥2: power-of-K choices gives an expected max load of `~ln ln N / ln K` above the mean — exponentially better than uniform when slot variance matters.
- **Constraint 3**: pivots are independently random across allocator processes. Under a surge of M allocations into N envoys, K=1 spreads approximately Poisson; the expected max load is `M/N + √((M log N)/N)`. K≥2 further smooths surges because each allocator draws K independent pivots, so cross-allocator herding requires K-way pivot collisions — exponentially unlikely. The random tiebreak at K≥2 is load-bearing on cold pools where all candidates show slots=0.

### What this does NOT solve

The version-discovery prefix scan (finding the current `highest_version` for `(namespace, pool)`) is shared by all allocation strategies and remains a hot FDB prefix under surge. It is tracked as a separate workstream (`EnvoyPoolMetaKey`-style materialized highest-version cache). Once that lands, the per-allocation cost becomes a single hash-positioned range read with no shared hotspot.

## Virtual nodes (V)

`V` is the number of positions each envoy occupies on the u128 ring. Each envoy at registration is mapped to V points by hashing its `envoy_key` with V different seeds; allocators land on whichever envoy owns the nearest position past the random pivot.

### Naming

The technique is called **virtual nodes** (or "vnodes"), introduced by Karger et al.'s 1997 consistent-hashing paper and popularized by Amazon's Dynamo (2007). Subsequent systems use the same construction under different names:

- **Cassandra** calls each position a **token**; default is 256 tokens per node.
- **Riak** calls them vnodes; default is 64 per node.
- **DynamoDB** uses similar partitioning internally.

These systems use V to balance *data placement* — bigger V means smoother rebalancing on node join/leave. We use it purely for *selection fairness*, so we can tolerate much smaller V values.

### Why V matters

With V=1, each envoy owns a single random point on the ring. The arc *before* an envoy's point determines its selection probability. Under random hashing, arc lengths are exponentially distributed, and the longest arc is roughly `ln(N) × mean` — so the unluckiest envoy gets `ln(N)`× more allocations than the mean.

With V positions per envoy, each envoy's total share is the *sum* of V independent exponential arcs. Sums of independent random variables concentrate around the mean (Gamma distribution); the relative spread shrinks as `1/√V`. Worst-case max/mean imbalance follows approximately `1 + ln(N)/V`.

The algorithm itself is unchanged by V — the allocator still picks one random pivot and walks to one envoy. V only changes the *index shape* (V keys per envoy instead of 1). The smoothing happens because the ring has more, smaller arcs.

### Tradeoff table

| V | Max/mean at N=1000 envoys | Writes per envoy at connection init |
|---|---|---|
| 1 | ~7× | 1 |
| 4 | ~2.7× | 4 |
| 8 | ~1.9× | 8 |
| 16 | ~1.4× | 16 |
| 32 | ~1.2× | 32 |
| 64 | ~1.1× | 64 |

V=8 is the chosen default — it brings the worst-case imbalance well under the ~2× tolerance from constraint 2 while keeping the per-connection write count trivial. V=1 (the V=1 row above is essentially the failure mode of the previous `RandomPingTimestamp` strategy) is what we are migrating *away* from.

`virtual_nodes` is exposed in the `EnvoyLoadBalancer::Hash` config variant. Bumping to V=16 doubles the per-connection write count but halves the imbalance bound; useful if a large pool (N > 5000) needs tighter fairness. Heartbeats are not affected by V.

### V is operationally fixed

Although `virtual_nodes` is in config, treat it as **fixed for the lifetime of the index**. Changing V does not migrate existing index entries:

- **Increasing V** (e.g. 8 → 16) is safe but slow to take effect. New positions get written only when envoys reconnect (since hash writes happen at connection init, not on heartbeat). Existing positions 0..7 are unaffected. Until every envoy has reconnected, the index has a mix of 8-vnode and 16-vnode envoys and allocator fairness drifts.
- **Decreasing V** (e.g. 16 → 8) only affects reconnects. Existing envoys keep their 16 hash entries until they expire, because `VirtualNodesKey` records the V used at registration time. New or reconnected envoys write 8 entries, so the ring has mixed weights until the old connections drain.
- **Mismatched V across engine processes** during a rolling deploy means different connection-init writers may assign different ring weights. Expire still deletes correctly because it reads persisted V, but allocator fairness drifts while the pool is mixed.

Changing V in practice requires either (a) a full reconnect or sweep plan that rewrites every envoy's ring positions, or (b) accepting a transitional window with mixed ring weights. Don't change V casually. The `virtual_nodes` config knob exists primarily so we can pick a value at the time of initial rollout and validate it in staging without rebuilding the binary.

## Index layout

```
EnvoyHashIdxKey(namespace_id, pool_name, -version, hash_pos: [u8; 16], envoy_key) -> ()
```

Tuple-packed as `(NAMESPACE, ENVOY_HASH_IDX, namespace_id, pool_name, -(version as i32), hash_pos, envoy_key)`. Negative version places higher versions first within a `(namespace_id, pool_name)` subspace. `hash_pos` is the big-endian byte representation of `xxh3_128_with_seed(envoy_key, i)`, so byte-lexicographic FDB ordering matches u128 numeric ordering.

Each envoy occupies V keys at positions `xxh3_128_with_seed(envoy_key, i).to_be_bytes()` for `i in 0..V`. **The value is empty.** The hash index is a pure membership marker; freshness lives in `LastPingTsKey` and is checked by the allocator at read time.

`VirtualNodesKey(namespace_id, envoy_key) -> V` records the exact V used at registration time. Expire reads this persisted value so it deletes exactly the hash positions that connection init wrote, even if `virtual_nodes` changes later.

The existing `EnvoyLoadBalancerIdxKey` is kept for the version-drain workflow and as a fallback during migration. New work targets `EnvoyHashIdxKey`.

## Init protocol

V hash entries are written **once per envoy connection**, inside the existing `pegboard-envoy/src/conn.rs` init transaction (alongside the writes for `PoolNameKey`, `VersionKey`, `CreateTsKey`, `LastPingTsKey`, `ProtocolVersionKey`, `ActiveEnvoyKey`, `ActiveEnvoyByNameKey`):

1. Read existing per-envoy keys (`create_ts`, `last_ping_ts`, `version`) for idempotency.
2. Write all existing init keys.
3. Write `VirtualNodesKey(namespace_id, envoy_key) = V`.
4. For `i in 0..V`: write `EnvoyHashIdxKey { ..., hash_pos: xxh3_128_with_seed(envoy_key, i).to_be_bytes() }` with value `()`.

All writes are in one FDB transaction. Reconnects re-run init, which idempotently re-writes the V entries (empty over empty — no-op in the steady state, self-healing if entries went missing).

**Heartbeats do NOT touch the hash index.** `update_ping` writes only `LastPingTsKey` / `LastRttKey` / `EnvoyLoadBalancerIdxKey` (the latter unchanged from today). The hash index entries persist from connection init until envoy expiry.

**Version cannot change for an envoy key.** An envoy's `version` is set at connect time and is operationally fixed for that `envoy_key`. If a reconnect presents the same key with a different version, init logs a warning and proceeds so the connection can recover, but operators must treat this as an invariant violation. The code intentionally does not add cross-version hash cleanup to make version changes safe; changing version requires a new envoy key or an explicit cleanup/sweep plan.

## Allocator flow

`hash::allocate` (in `engine/packages/pegboard/src/workflows/actor2/alloc_serverful/hash.rs`) is a single allocator flow whose behavior is gated by `samples` (K). A shared helper `scan_for_fresh(tx, pool, version, range, ping_threshold, remaining_scan)`:

- Range-reads `EnvoyHashIdxKey` in the given range, snapshot isolation.
- For each candidate hash entry, single-key snapshot reads `LastPingTsKey(envoy_key)` and skips if missing or older than `now - envoy_eligible_threshold`.
- Enqueues stale candidates through `EnvoyExpireScheduler::try_enqueue` and keeps scanning until it finds a fresh envoy or `remaining_scan` reaches zero. `remaining_scan` is a single shared budget initialized to `max_scan` at allocator entry; every stale entry observed by any sample's forward or wrap scan decrements it, so the total stale-walk cost per allocation is bounded by `max_scan`.
- Returns the first fresh `envoy_key` (or `None`).

### K=1 short-circuit (uniform pick)

When `samples == 1`, the allocator must NOT read `SlotsKey` — there's no comparison to make and the read would be pure waste. The implementation explicitly short-circuits:

1. Resolve `highest_version` for `(namespace_id, pool_name)`.
2. Pick `pivot: [u8; 16] = rand::random::<u128>().to_be_bytes()`.
3. Call `scan_for_fresh(.., pivot..)`. If `None`, fall back to `scan_for_fresh(.., ..pivot)` (wrap).
4. If a fresh envoy is found, register a read conflict on its `EnvoyHashIdxKey` when snapshot reads are enabled and return immediately.
5. If both reads return nothing, return `Ok(None)`.

Common-case reads: 1 `EnvoyLoadBalancerIdxKey` range read for highest-version discovery + 1 `EnvoyHashIdxKey` range read + 1 `LastPingTsKey` lookup = **3 snapshot reads**.

The K=1 read-cost is the same as a dedicated uniform strategy would have paid. It also does not emit `envoy_lb_samples_effective`, `envoy_lb_sample_dedupe_total`, or `envoy_lb_tied_min_total`; those metrics are gated on K≥2 because they only make sense in the comparison path.

### K≥2 (power-of-K choices)

When `samples >= 2`:

1. Resolve `highest_version` (same path).
2. For `_ in 0..samples`:
   a. Pick an independent `pivot: [u8; 16] = rand::random::<u128>().to_be_bytes()`.
   b. Call `scan_for_fresh(.., pivot..)`, fall back to `scan_for_fresh(.., ..pivot)` on `None`.
   c. If the resolved envoy is already in the candidate set, skip (dedupe). Otherwise read `SlotsKey(envoy_key)` snapshot and push `(envoy_key, slots)`.
3. If no candidates resolved, return `Ok(None)`.
4. Compute `min_slots = candidates.iter().map(|(_, s)| *s).min()`. Filter to the tied subset and pick uniformly at random.
5. Register a read conflict on the chosen envoy's `EnvoyHashIdxKey` when snapshot reads are enabled and return.

Common-case reads (K=2, no dedupe, no wrap): 1 `EnvoyLoadBalancerIdxKey` range read for highest-version discovery + 2 `EnvoyHashIdxKey` range reads + 2 `LastPingTsKey` lookups + 2 `SlotsKey` lookups = **7 snapshot reads**.

The random tiebreak is required for correctness on cold pools — deterministic tiebreak across K equal-slots candidates collapses cross-allocator behavior to "always pick the first scanned vnode", reintroducing hash-position bias.

## Expiry / GC

Two delete paths coexist.

### Path 1: `pegboard_envoy_expire` (primary)

`pegboard_envoy_expire` (in `engine/packages/pegboard/src/ops/envoy/expire.rs`) is the primary delete site for envoy index entries. When an envoy expires (graceful shutdown, eviction, lost-timeout), the expire operation:

1. Deletes the envoy's `EnvoyLoadBalancerIdxKey` entry (existing).
2. Writes the `ExpiredTsKey` marker (existing).
3. Deletes the envoy's `ActiveEnvoyKey` and `ActiveEnvoyByNameKey` entries (existing).
4. Serializable-reads `VirtualNodesKey` to learn the V written at connection init.
5. **For `i in 0..V`: deletes `EnvoyHashIdxKey { ..., hash_pos: xxh3_128_with_seed(envoy_key, i).to_be_bytes() }`**.
6. Deletes `VirtualNodesKey`.

Positions are recomputed from `envoy_key`, so deletion requires no additional reads.

**Load-bearing invariant:** the `ExpiredTsKey` write, `VirtualNodesKey` read, V hash-position deletes, `VirtualNodesKey` delete, active-index deletes, and legacy load-balancer-index delete all happen in the same FDB transaction. An allocator may observe a stale envoy before expiry, but it must never observe an envoy that has committed `ExpiredTsKey` while its hash entries remain committed.

### Path 2: Per-process read-path expire scheduler (secondary)

The graceful expire path misses three cases: process crash mid-handler, engine restarts where the envoy never reconnects, and slow expire backpressure during mass drains. To handle these, every allocator (`Hash::scan_for_fresh`, `RandomPingTimestamp`, `RandomFullRange`) calls `EnvoyExpireScheduler::try_enqueue(ns, envoy_key)` when it walks past a stale envoy.

Scheduler behavior:
- `scc::HashSet<String>` tracks in-flight `envoy_key`s. `try_enqueue` returns immediately if already present (process-local single-flight).
- `tokio::spawn`-ed worker acquires a permit from a bounded `Semaphore` (default 32 concurrent).
- Worker invokes `pegboard_envoy_expire` with `skip_if_fresh: true`. The op's transaction re-reads `LastPingTsKey` and `ExpiredTsKey` atomically with the deletes and bails out if the envoy is fresh or already expired. No freshness logic lives in the scheduler.
- On task completion or panic, the `pending` set entry is dropped via `scopeguard::defer`. Self-healing if the scheduler is torn down mid-task (next allocator observing the same envoy re-enqueues).

Properties:
- **Pure wrapper around the canonical expire op.** The scheduler does NOT own delete logic, freshness logic, or any per-envoy semantics. Any future expire responsibility (notification, ledger, gauge) is inherited automatically.
- **No write conflicts on the allocator transaction.** The allocator stays read-only and snapshot.
- **Single source of truth for "should this envoy expire."** The freshness check is in the same FDB transaction as the deletes (op's `skip_if_fresh` path). No TOCTOU window: a heartbeat that commits between observation and invocation is correctly observed by the in-tx read.
- **Race with graceful `pegboard_envoy_expire` is benign.** Both run through the same op. First commit wins (writes `ExpiredTsKey`); second invocation's in-tx check sees it and returns `did_expire: false`.

### Retry stack on stale observations

Stale observations are handled by three layers:

1. The allocator treats stale hash entries as non-fatal: skip the entry, enqueue a background expire, and continue scanning until a fresh envoy is found or `max_scan` aborts the scan.
2. The scheduler is fire-and-forget and process-local. It dedupes within the process, bounds concurrent expire workers with a semaphore, rejects new work at `max_pending`, and never changes allocator success or failure.
3. `pegboard_envoy_expire { skip_if_fresh: true }` owns the authoritative retry-safe decision. Its own FDB transaction Serializable-reads `LastPingTsKey` and `ExpiredTsKey` before deleting, so a heartbeat or graceful expire racing after the allocator's stale read is observed inside the commit attempt.

### TODO: replace with a workflow

The per-process scheduler is **not the final design**. FDB op-invocation traffic scales with `O(cluster_size × stale_envoys_observed_per_scan)`: every engine process that observes the same stale envoy independently spawns an expire invocation. Process-local dedup helps within one process but does not coordinate across the cluster. Hot-ring bias means envoys far from any active scan stay alive in the index forever.

The long-term path is a `pegboard_envoy_stale_sweep` gasoline workflow running as a cluster singleton every N minutes:
- Scans for envoys whose `LastPingTsKey` is stale and `ExpiredTsKey` is absent.
- Batches `pegboard_envoy_expire` invocations.
- Emits a gauge of envoys cleaned per pass.

Tracked in `.agent/todo/envoy-stale-sweep-workflow.md`.

## Tuning parameters

| Parameter | Default | Where | Effect |
|---|---|---|---|
| `EnvoyLoadBalancer::Hash.virtual_nodes` | 8 | `engine/packages/config/src/config/pegboard.rs` | Vnodes per envoy. Higher V → more uniform selection at the cost of V writes per envoy connection. Recommended range 4-32. Operationally fixed once the index is populated; see "V is operationally fixed" above. |
| `EnvoyLoadBalancer::Hash.use_snapshot_read` | true | same | Snapshot isolation on the allocator reads. Reduces allocation conflict ranges; only the chosen key gets a read conflict. |
| `EnvoyLoadBalancer::Hash.samples` | 2 | same | K in the power-of-K-choices algorithm. `1` = uniform pick (skips slot read via short-circuit). `2` = power-of-2-choices, recommended default. `3` is the diminishing-returns knee. Each K≥2 sample costs 3 extra single-key snapshot reads per allocation. |
| `EnvoyLoadBalancer::Hash.max_scan` | 16 | same | Maximum total stale hash entries walked **per allocation** (shared across all K samples and the wrap path) before the allocator returns `None`. Higher values tolerate more stale debris during drains but increase worst-case read cost. Valid range 1-256. |
| `EnvoyLoadBalancer::Hash.slot_jitter` | 4 | same | Additive random integer in `0..slot_jitter` added to each candidate's slot count before the min comparison. Decorrelates concurrent allocators reading the same stale `SlotsKey` snapshot. See the `slot_jitter` block in `hash.rs` for the sizing derivation. `0` disables. Range 0-64. |
| `envoy_eligible_threshold` | 10s | pegboard config | How long since an envoy's last heartbeat before allocators consider it dead. Affects how many stale entries the allocator may skip. |

### Picking K

- **`samples = 1`** — pools where actor cost per envoy is uniform (or actor lifetime is short enough that slot counts converge regardless). Lowest read cost (2 snapshot reads). The short-circuit skips `SlotsKey` reads entirely, so this is strictly cheaper than `samples = 2`.
- **`samples = 2`** (default) — pools with high slot variance (long-lived actors, mixed actor sizes, recently-rebooted envoys returning at slots=0). Costs 4 extra reads per allocation. Switch when measurements show `max(slots) / median(slots) > ~3` under steady-state load.
- **`samples = 3-4`** — only if K=2 still shows imbalance under load (rare).

## Hash function notes

`xxh3_128_with_seed` is from the `xxhash-rust` crate. Non-cryptographic, fast (multi-GB/s), and statistically uniform across u128 for the inputs we use (envoy keys). The 128-bit output is stored as `[u8; 16]` big-endian so FDB's lexicographic byte ordering matches numeric ordering.

**Stability across crate versions.** The crate's seeded output must remain stable for the index to function correctly across deploys. The `Cargo.toml` pins an exact version, and a unit test (`#[test] fn xxh3_stability_regression`) asserts that `xxh3_128_with_seed("rivet-envoy-test-key", 0)` equals a fixed expected u128. Any future crate upgrade that changes the output will fail CI.

**Collision risk.** At N envoys × V vnodes items in u128 space, birthday-paradox collision probability ≈ `(N×V)² / 2^129`. For N=10000, V=8 that's ~10⁻²⁸ — effectively impossible. The `envoy_key` lex tiebreaker remains in the key shape for defense-in-depth but never triggers in practice.

## Observability

Bounded-label metrics (always labeled by `(namespace, pool, strategy)`, never by `envoy_key`). `strategy` is the bounded enum string set `{"hash", "random_ping_timestamp", "random_full_range"}`.

Shared across all strategies:

- `envoy_lb_allocation_total` — counter, per successful allocation.
- `envoy_lb_no_envoy_available_total` — counter, per `None` return.
- `envoy_lb_scan_depth` — histogram of entries scanned per allocation (summed across all samples at K≥2).
- `envoy_lb_wrap_total` — counter, wrap-path triggers.
- `envoy_lb_alloc_duration_seconds` — histogram, allocator latency.

`Hash` strategy, only when `samples >= 2`:

- `envoy_lb_samples_effective` — histogram of the unique candidate count per allocation (≤ configured `samples`).
- `envoy_lb_sample_dedupe_total` — counter, a sampled pivot resolved to an already-selected envoy.
- `envoy_lb_tied_min_total` — counter, ≥ 2 candidates tied at min slot count (random tiebreak fired).

At K=1 the three above metrics stay at zero — the short-circuit returns before the samples loop / slot-comparison code runs.

## Related code

- `engine/packages/pegboard/src/keys/ns.rs` — `EnvoyHashIdxKey`, `EnvoyLoadBalancerIdxKey`, subspace keys.
- `engine/packages/pegboard/src/keys/envoy.rs` — per-envoy keys (`SlotsKey`, `LastPingTsKey`, `ExpiredTsKey`, etc.).
- `engine/packages/pegboard-envoy/src/conn.rs` — envoy connection-init write (V hash entries written alongside existing init keys).
- `engine/packages/pegboard/src/ops/envoy/update_ping.rs` — heartbeat write (unchanged; freshness flows through existing `LastPingTsKey`).
- `engine/packages/pegboard/src/ops/envoy/expire.rs` — expiry delete.
- `engine/packages/pegboard/src/workflows/actor2/alloc_serverful/hash.rs` — hash allocator + `scan_for_fresh` helper.
- `engine/packages/config/src/config/pegboard.rs` — config enum.
