# Performance review: sqlite-pitr-fork.md

Adversarial review of `/home/nathan/r2/.agent/specs/sqlite-pitr-fork.md` against
the stateless baseline (`sqlite-storage-stateless.md`) and the CF DO research
target. References to `sqlite-pitr-fork.md` are bare line numbers; the
stateless spec uses `stateless:N`.

## Hot-path regressions

### 1. The "0 RTT change on commit" claim is misleading (table line 508)

The spec asserts `commit (steady state)` is "1 RTT, 0 change vs stateless".
Real added work inside the same UDB tx:

- **+1 in-tx `tx.get()`** of `COMMITS/{head_txid}` for rolling-checksum fold
  (line 263). FDB in-tx point reads are ~0.5-2ms each. Stateless commit reads
  only `/META/head`. The new read MUST be `try_join!`-pipelined with `/META/head`
  or it adds a real internal RTT. The spec doesn't say so.
- **+1 KV write** of `COMMITS/{T}` (~24 bytes, line 243), plus its write
  conflict range.
- **Larger key prefix** on every op: `[BR][branch_id_bytes]` adds 16 bytes per
  key. With 6-8 keys touched per commit, that's ~96-128 extra bytes of key data
  per commit through the FDB mutation log + resolver.

**Quantification.** At 100 commits/s: +2.4 KB/s mutation log, +12 KB/s key
overhead, +0.5-2ms wall on cold cache.

**Fix.** Either (a) explicitly pipeline the COMMITS read with `try_join!`, or
(b) move the rolling checksum onto `/META/head` as another `u64` field (LiteFS
puts it in `Pos{TXID, PostApplyChecksum}`). With (b), commit reads only
`/META/head` even on cold cache — true 0-RTT-change.

### 2. Rolling-checksum cache cold start (line 263)

> "Reuses cached value on subsequent commits without re-reading prior entries."

The cache lives on `ActorDb`, which is per-WS-conn (`stateless:117`). Envoy
reconnect drops it (`stateless:155-160`). First commit on a fresh `ActorDb`
re-reads `COMMITS/{head_txid}` → +1 in-tx get on cold path.

**Quantification.** Reconnect freq is deployment-churn-bounded (minutes). Hot
actor at 100 commits/s with 1 reconnect/min = ~0.02% extra reads (1 per 6000
commits). Small in aggregate but the spec hides it; combined with issue 1, the
"steady state 0 change" claim is wrong on the first commit of every conn.

**Alternative.** Per issue 1's fix (b), checksum on `/META/head` eliminates
the cache entirely.

### 3. Parent fall-through walk is O(N) per fork-chain depth (line 365)

The spec describes one-level parent fall-through. A 5-deep fork chain reading
a page touched only at the root: 2 in-tx gets (PIDX+SHARD) **per chain level**.
Spec mentions `parent_pidx_cache` (line 375) but only one parent, no
flattening.

**Quantification.** Chain depth 10 × `get_pages` of 100 unmodified pages =
2000 serialized in-tx gets at 0.5-2ms each = **1-4s wall-clock for one
get_pages**. Turso bottomless explicitly caps `.dep` chains at 100; this spec
has no cap. Compare Neon: `LayerMap` is a flat `BTreeMap<(KeyRange, LSN)>` —
one btree lookup per page regardless of timeline ancestry depth.

**Fix.** (a) cap fork chain depth (~10), or (b) flatten parent PIDX into a
single per-`ActorDb` BTreeMap keyed by (depth, pgno) on first fall-through —
one prefix scan per ancestor, then RAM-only lookups. Option (b) is closer to
Neon and avoids the per-pgno walk.

### 4. PIDX cold-scan amortization breaks under fork (stateless:154 + line 365)

Stateless spec amortizes one PIDX cold scan per WS conn. With fork, every
ancestor in the fall-through chain pays its own cold scan on first miss. 5
ancestors × ~30-60ms (1 GiB DB → 256k PIDX entries → ~3 MiB prefix scan at
50-100 MB/s) = **150-300ms tail latency** on first cross-branch read after WS
reconnect.

**Fix.** Bound parent_pidx_cache to top-K ancestors with LRU eviction; or
share the cache across actors at the conn level (since multiple actors may
fork the same parent template).

## Cold-path latency cliffs

### 5. Cold-path read on a hot fork is brutal (line 365 step 4)

For a fork at txid older than `cold_pass_interval_secs = 3600`, every
previously-untouched page read in the fork hits S3:

1. UDB get on parent PIDX (miss).
2. UDB get on parent SHARD (may miss — image layer in cold tier only).
3. S3 GET manifest (~10-30ms uncached).
4. S3 GET layer file (~30-200ms warm-region p50, 200-500ms p99).
5. LTX decode + page extract.

**Quantification.** A SQLite query scanning 1000 pages, all cold-tier: **30-500
seconds** of S3 latency. SQLite has no notion of multi-second page reads; the
actor is effectively offline. Workers-Free-tier-equivalent users running
`getBookmarkForTime(t = -25 days)` and querying recent rows: unusable on
1 GiB DB.

The spec has no fork-warmup, no layer-level prefetch, no sequential-prefetch
heuristic. S3 has no read-through cache layer.

**Fix.** Add fork-warmup: on fork creation, the cold compactor copies the
top-N most-recent image layers into the new branch's hot SHARD prefix. Cost is
O(image_layers_in_window) S3 GETs, paid once at fork time, off the read hot
path. Without this, fork descendants are perpetually slow.

### 6. Bookmark resolution gap problem (line 341, 582)

Failure-modes table says: "Bookmark index gap (cold compactor never ran) →
falls back to scanning hot COMMITS/* (at most 30 days, bounded)."

Hot COMMITS is GC'd at `txid < retention_pin_txid` (line 462). For an actor
offline 25 days then online 5 days: BookmarkIndex has a 25-day gap (no cold
passes), AND hot COMMITS only covers 5 recent days. **Any bookmark in the
offline window is unresolvable**, even though it's within retention. The spec
claims "bounded" but it isn't bounded if the actor wasn't running.

**Quantification.** "Offline 25d, online 5d" is realistic for game backends or
scheduled jobs. The spec returns `BookmarkExpired` (or interpolates wrong)
across most of retention.

**Fix.** When `t` precedes the oldest BookmarkIndex entry but layers within
retention exist, binary-search layer-file names (which encode `min_txid` and
`max_txid`) and read the LTX trailer for wall-clock. +1 S3 GET, fills gap.

### 7. S3 cold-manifest rewrite-per-pass cost at fleet scale (line 428)

Manifest is rewritten every pass via single PUT. Long-lived actor: 720
L0 layers/month + L1/L2/L3 entries → ~50 KB manifest. 720 rewrites/month/actor
× $0.005/1000 PUT = $0.0036/actor/month.

**At 1M actors: $3,600/month** in PUT cost just for manifest rewrites — dwarfs
layer-file PUT cost for low-activity actors.

**Fix.** Append-only manifest segments + periodic compaction. Or daily rewrite
+ in-flight delta log.

## Resource scaling issues

### 8. ActorDb-per-branch memory at 1M-actor scale

`ActorDb` is keyed by `actor_id`, not branch — so 1M actors → 1M `ActorDb`
instances, NOT 10M (the charter's worry). But `parent_pidx_cache` (line 375)
holds caches for each ancestor in the fork chain, so 1M actors × ~2-5 ancestor
caches = 2-5M PIDX caches.

**Per-instance:** ~32-64 KB realistic (PIDX cache dominates, ~16 KB; quota +
checksum + metering counters ~1 KB combined).

**Quantification.** 1M `ActorDb` × 64 KB = **~64 GiB across the fleet**. Plus
5M parent caches × 16 KB = 80 GiB. **Fleet total ~144 GiB**. With 50 envoy
pods: ~3 GiB/pod for SQLite caches alone. PITR adds ~10-20% overhead vs
stateless baseline (parent caches + checksum cache). Tractable; not a
regression so much as a confirmation.

### 9. Cold layer file size variance (line 437, 650)

Drain window 32-1024 commits with no byte cap:

- 1024 × 1-page commits = 4 MB layer (single PUT, ~30ms).
- 1024 × 100-page commits = 400 MB layer (multipart, **~10-20s upload**).

Cold-lease TTL is 30s with 10s renewal interval (`stateless:402`). A 20s
multipart upload + post-upload manifest rewrite + FDB cleanup tx can exceed
the lease deadline. Spec at line 411 claims hot/cold compactors run
concurrently because META sub-keys are disjoint, but it doesn't address the
lease-time-budget problem from a slow upload.

**Fix.** Cap drain window by **byte budget**, not txid count. Concrete:
`drain_max_bytes = 64 MB`. Or chunk a single drain window into multiple
layer files and stream PUTs in parallel.

### 10. GC pin computation for fan-out fork tree (line 456)

> "For each child branch C (refcount > 0): pin = min(pin, C.parent_txid)."

Spec doesn't say how children are enumerated. Implies a `BRANCHES/list/*`
prefix scan + per-child KV get. For nested forks (each fork itself forked),
GC walks the entire tree.

**Quantification.** Worst case: 1000 forks × 10 sub-forks each = 11,000
branch records to read. At ~100 bytes/record, 1.1 MB of in-tx reads. **Likely
exceeds FDB 5s tx-age cap**; spec has no multi-tx coordination story for GC.

**Fix.** Maintain `oldest_descendant_parent_txid` as an atomic-min counter on
each branch's `META/manifest`, updated on fork-create and branch-delete. GC
becomes a single-key read instead of tree walk. Trade: extra atomic op on
fork.

## Operational hot spots

### 11. Cold compactor cron has no leader election (line 404)

> "Periodic in-process cron in the cold compactor itself: every
> cold_pass_interval_secs (default 3600), iterate `[BRANCHES]/list/*` for
> actors this pod owns and enqueue."

"Actors this pod owns" is undefined. Two readings:

- **(a) Each pod scans the entire prefix and self-enqueues.** With N pods and
  1M actors, N × 1M-entry scan/hour. ~10s of FDB scan-work per pod-hour.
  Wasteful, and risks thundering herd at top-of-hour.
- **(b) Implied partitioning.** No mechanism described; queue group dedups on
  the receive side but not on the publish/list side.

Compare hot compactor: purely UPS-driven by commits (`stateless:332-345`),
never iterates the full list. Cold's cron is fundamentally different.

**Fix.** Either (a) hash-partition `actor_id mod N_pods` via a shared pod-id
assignment, (b) elect a leader hourly via a UDB-backed lease, or **(c) drop
the cron entirely** — only enqueue actors whose hot compactor publishes a
"crossed cold_compact_delta_threshold" trigger (line 405-406). Prefer (c).

### 12. Cold compactor pass cannot fit in one FDB tx (line 430-432)

Pass step 8 lists "single FDB tx" doing: `META/cold_compact` set, clear up to
1024×N DELTA chunks, delete COMMITS in retention range. This must run AFTER
the S3 PUT (step 4) which can take 100ms-20s.

FDB tx-age cap is 5s. Either (a) the entire pass runs in one tx (impossible —
S3 PUT exceeds), or (b) reads in step 2 use a snapshot, S3 work in steps 3-7
runs OUTSIDE any tx, step 8 opens a fresh write tx with only FDB cleanup.
The spec implies (a); in practice it must be (b). Spec needs to say so
explicitly.

**Fix.** Restructure §Pass procedure to make the FDB tx boundary explicit:
- Snapshot read tx (short-lived) for steps 2-3.
- No tx during steps 4-7 (S3 + UPS work).
- Fresh write tx for step 8, asserting `cold_drained_txid` precondition.

### 13. Cold cron thundering herd at scale (line 404, stateless:332-345)

Hot throttle 500ms/actor; cold 60s/actor. With 1M warm actors:

- Hot: 2M publishes/s steady, ~1% CPU/pod across 100 pods. OK.
- Cold steady: 1M / 60s = 16.7k/s. OK.
- Cold cron at top-of-hour: **N pods × 1M publishes simultaneously**. With
  N = 50, ~50M publishes in <1s. UPS/NATS can saturate.

The throttle is per-actor-per-pod, not cross-pod. Without coordination, all
pods publish for the same actors within the throttle window.

**Fix.** Hash `actor_id` to a 0-3600s offset and stagger the cron, OR adopt
fix (c) from issue 11 (purely UPS-driven cold compaction).

### 14. Hot/cold compactor DELTA-clear temporal race (line 411, 547)

Spec asserts disjoint META sub-keys but both compactors clear DELTA keys.
Disjointness is **temporal**: cold only clears T <= materialized_txid
(already folded by hot). If hot advances `materialized_txid` while cold's
plan-tx is mid-flight using a snapshot of the older value, hot will fold
DELTAs that cold then "drains" → cold writes an empty layer (DELTAs gone) →
wasted S3 PUT, signals corruption in metrics.

**Fix.** Cold's write-phase tx must regular-read `META/compact.materialized_txid`
and abort if it's advanced past the planned drain window. Spec needs to add
this explicitly. Currently relies on snapshot-isolation assumption that
doesn't hold across the S3 PUT boundary.

## Wins (places where the design IS performant)

- **Bookmark format is read-cheap.** `get_current_bookmark` is one cached
  read of head_txid + commit_at_ms. Lex order = chrono order, no decode for
  sort. (lines 78-86.)
- **Fork is metadata-only, O(1).** Two FDB ops (BRANCHES write + atomic-add
  refcount), no data copy. Constant cost regardless of parent size. (line 142.)
- **Lease + sub-key split lets hot/cold run concurrently.** Disjoint META
  sub-key ownership (line 411) is correct in steady state — modulo issue 14's
  temporal-race fix.
- **Image-layer "log >= db size" rule (line 424-427)** matches SRS exactly,
  caps cold-restore download at 2x DB size.

## Recommended fixes (priority order)

1. Fix the "0 RTT change on commit" claim — pipeline COMMITS read or move
   checksum to `/META/head` (issues 1, 2).
2. Add fork warmup or layer prefetch — fork descendants 10-100x slower than
   head reads without it (issue 5).
3. Cap fork chain depth or flatten parent_pidx (issue 3).
4. Hash-partition or UPS-drive the cold compactor (issues 11, 13).
5. Byte-budget the cold drain window (issue 9).
6. Materialize `oldest_descendant_parent_txid` for GC (issue 10).
7. Make cold-pass FDB tx boundaries explicit (issue 12).
8. Bookmark-gap fallback to layer-file binary search (issue 6).
