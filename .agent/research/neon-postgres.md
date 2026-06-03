# Neon Postgres — Pageserver + branching + PITR

Research compiled for designing a SQLite PITR + forking system on top of FoundationDB-stored DELTA/SHARD blobs. Neon is Postgres-on-S3, but the page-level COW layer model maps cleanly onto a per-actor SQLite world with a `txid: u64` substituted for `LSN`.

## Sources

- https://github.com/neondatabase/neon/blob/main/docs/pageserver-storage.md
- https://github.com/neondatabase/neon/blob/main/docs/glossary.md
- https://github.com/neondatabase/neon/blob/main/docs/walservice.md
- https://github.com/neondatabase/neon/blob/main/docs/safekeeper-protocol.md
- https://github.com/neondatabase/neon/blob/main/docs/settings.md
- https://github.com/neondatabase/neon/blob/main/pageserver/src/tenant/layer_map.rs
- https://neon.com/blog/get-page-at-lsn  (Heikki Linnakangas, deep dive into the storage engine)
- https://neon.com/blog/architecture-decisions-in-neon
- https://neon.com/blog/pitr-deep-dive
- https://neon.com/blog/point-in-time-recovery-in-postgres
- https://neon.com/blog/announcing-point-in-time-restore
- https://neon.com/blog/announcing-neon-snapshots-a-smoother-path-to-recovery
- https://neon.com/blog/recent-storage-performance-improvements-at-neon
- https://neon.com/blog/how-we-scale-an-open-source-multi-tenant-storage-engine-for-postgres-written-rust
- https://neon.com/docs/introduction/architecture-overview
- https://neon.com/docs/introduction/restore-window
- https://neon.com/docs/manage/branches
- https://neon.com/docs/guides/branching-neon-api
- https://api-docs.neon.tech/reference/createprojectbranch
- https://jack-vanlightly.com/analyses/2023/11/15/neon-serverless-postgresql-asds-chapter-3
- https://db.cs.cmu.edu/events/databases-2022-neon-serverless-postgresql-heikki-linnakangas/  (CMU talk)

## Architecture overview

Three services, fully decoupled, talking over WAL stream + S3.

- **Compute node**: stock Postgres binaries, stateless. No local durable storage. Streams WAL out to safekeepers; reads pages on miss via `GetPage@LSN` RPC to a pageserver. Local NVMe + shared buffers are caches only. NeonVM (QEMU/KVM, K8s-scheduled) wraps the Postgres process for autoscaling and live migration.
- **Safekeeper**: Paxos-replicated WAL durability tier. Cluster of three nodes per tenant. Postgres is the proposer, safekeepers are acceptors, pageservers are learners. A WAL record is committed once a quorum (majority of three) has fsynced it. WAL is held until pageservers confirm ingestion + S3 upload, then GC'd.
- **Pageserver**: stateless cache + materializer. Ingests WAL from safekeepers, organizes it into immutable layer files, uploads to S3, evicts locally. Serves `GetPage@LSN(key, lsn)` to compute. "Page servers function as just a cache of what's stored in the object storage."
- **Object storage (S3)**: single durable copy of all layer files. Tenants are sharded across pageservers by block-number stripe (default 256 MiB stripe).

Multi-tenancy: tenant -> timelines -> branches. A timeline is the internal name; users see "branch". A timeline has zero or one `ancestor_timeline` + `ancestor_lsn` pair.

## Layer files (delta + image)

Two file types, both immutable, both addressed in 2D `(key range, LSN range)` space. Filenames encode the bounds:

```
ImageLayer:  <start_key>-<end_key>__<lsn>
DeltaLayer:  <start_key>-<end_key>__<start_lsn>-<end_lsn>
```

- **Image layer**: snapshot of all keys in a key range at a single LSN. Keys absent from the layer are known not to exist at that LSN.
- **Delta layer**: collection of WAL records (or full page images) for a key range across an LSN range. Keys not modified in the LSN range have no entry. `start_lsn` inclusive, `end_lsn` exclusive.

LSN ordering rule: image layers are searched before delta layers, on-disk before in-memory. Within delta layers the search walks backwards in LSN until an image (or full-page WAL record) is hit.

**Levels** (L0/L1) are not stored on disk. They are inferred from key range:

- **L0**: covers the entire keyspace, narrow LSN range. These are the freshly-flushed delta layers from in-memory.
- **L1**: covers a partial key range, wide LSN range. Produced by compacting L0s.

**In-memory + frozen layers**: writes accumulate in an `open_layer` (in-memory). When `checkpoint_distance` bytes are buffered (`~256 MiB`-ish open layer), it is "frozen" (still in RAM, no longer accepting writes) and queued for flush. On flush, it becomes an L0 delta on local disk (and uploads to S3). The `LayerMap` exposes `open_layer`, `frozen_layers: VecDeque`, `historic` (a persistent BST keyed by key, versioned by LSN), and `l0_delta_layers`.

**Compaction**:
- L0 -> L1: when N L0s pile up (default `image_creation_threshold = 3`, target file `compaction_target_size = 128 MiB`), they are merged and re-sharded into L1 deltas that each cover a narrow key range and a wide LSN range.
- Image layer generation: pageservers periodically materialize a fresh image layer for hot key ranges by replaying WAL on top of the last image, controlled by `image_creation_threshold`.
- L0 compaction is given priority and back-pressures ingestion when L0 count grows too fast (recent perf improvements: max L0 count fell from ~500 to <30 under heavy ingest, p99 read amp -50%).

## Page reconstruction

`GetPage@LSN(key, request_lsn)`:

1. Look up `request_lsn` in the `LayerMap`'s BTreeMap-of-versions to get the right BST snapshot, then point-query that BST by key.
2. Walk layers older-to-newer-no, actually start at `request_lsn` and walk **backwards in LSN** collecting WAL records for the key.
3. Stop when an image of the page is found (image layer, or a full-page-image WAL record).
4. Hand the base image + collected WAL records to a pooled WAL-redo Postgres process; it replays them and returns the materialized page.

The selection priority when multiple layers cover the same `(key, lsn)` point: image layers > delta layers > in-memory. If the timeline has an `ancestor_timeline`, and the requested LSN <= `ancestor_lsn`, the search continues in the ancestor timeline's layer map.

## PITR

- **Granularity**: LSN. Sub-millisecond. WAL records are the unit. Timestamp restores are translated to an LSN by the control plane, then proceed identically.
- **Mechanism**: PITR is *not* a full restore; it is **branch-at-LSN**. The control plane creates a new timeline whose `ancestor_timeline` = the source and `ancestor_lsn` = target LSN. Compute attaches to the new branch. No data copy. Reconstruction of any page below `ancestor_lsn` is served from the parent's layer files. "Restore completes in less than a second, regardless of how much data is stored."
- **Time-Travel Assist**: query the proposed restore point before committing.
- **Retention** (`pitr_interval`, default 7 days; user-facing "restore window"):
  - Free: 6 h, 1 GB cap
  - Launch: up to 7 days
  - Scale: up to 30 days
  - Set per-project via `history_retention_seconds`. Storage billed at $0.20/GB-month for the WAL/delta history above the live size.
- **GC horizon** (`gc_horizon`, byte-denominated, separate from `pitr_interval`): the larger of the two thresholds wins. Layers older than both are eligible for deletion. Default `gc_horizon = 64 MB` (a floor below which nothing is GC'd even after `pitr_interval` expires).

## Branching

- **Mechanism**: A branch is a new **timeline** with `(ancestor_timeline_id, ancestor_lsn)` set. Pure metadata operation, O(1), independent of database size. The user-facing `parent_lsn` / `parent_timestamp` on the API response are the materialized values of these.
- **COW**: at the **layer-file level**, not the page level. The child timeline writes its own new L0/L1 layer files; reads below `ancestor_lsn` fall through to the parent's layer map. No layer file is ever rewritten or copied at branch time.
- **Cost model**:
  - Branch creation: $0, zero bytes.
  - Storage billed only for **delta** between parent and child. "A child branch that has no data changes compared to its parent and is still within the restore window does not incur additional database storage costs."
  - Compute is billed independently (a branch only costs compute when it has an attached endpoint).
- **Divergence**: child writes go into child-timeline layers only. Parent is unaffected. To collapse divergence, the API supports **restore** (drop child writes, snap back to parent state, ~1 second).
- **Parent retention pinning**: a child branch holds a "lock" on parent layer files at `ancestor_lsn`. As the child ages past the restore window, those pinned layers become billable storage on the project. GC will not delete a layer that any descendant timeline still needs at its `ancestor_lsn`. (Historical bug: issue #707 "GC removes image layers still needed by delta layers" — they had to teach GC about layer dependencies.)

**Snapshots** (early-access, distinct from branches): a snapshot is a *labelled, retained* `(timeline_id, lsn)` reference that is intended to outlive the restore window. Restoring a snapshot creates a new branch (`main_from_snapshot_2025-04-14`). Snapshots are the bookmark / long-retention escape hatch for things that would otherwise be GC'd at the end of `pitr_interval`. API/CLI access "planned"; today they're a console feature.

## Storage tiering

- **Local pageserver disk**: hot cache. Layer files are LRU-evicted at ~128–256 MB granularity once uploaded to S3. Cold reads pull layer files back from S3 transparently.
- **S3 (object storage)**: single source of truth for all layer files. Immutable. "Files are written sequentially and never modified in place. ... [this] makes it straightforward to support branching."
- **Garbage collection**:
  - WAL on safekeepers: trimmed once pageservers ack ingest *and* S3 upload completes.
  - Layer files on pageservers + S3: deleted when (a) older than `pitr_interval` AND (b) older than `gc_horizon` AND (c) no descendant timeline pins them. GC is per-tenant, runs in background, and respects the layer-dependency graph (image layer can't be deleted while a still-live delta layer depends on it).
  - Compaction may emit a new image layer that supersedes a long delta chain; that's what eventually allows old deltas to be GC'd.

## API surface

- **Branch creation**: `POST /api/v2/projects/{project_id}/branches` with optional `branch.parent_id`, `branch.parent_lsn`, `branch.parent_timestamp`. Response includes resolved `parent_lsn` + `parent_timestamp`. Optional `endpoints[]` to spin up compute. Branch creation without endpoints is metadata-only and "instant".
- **PITR / restore-in-place**: a separate restore endpoint that drops divergent writes on a branch and resnaps it to its parent (or to a chosen LSN/timestamp) in ~1 s. Internally this is "create new branch at LSN, swap pointer, retire old timeline".
- **CLI**: `neon_local timeline branch` (internal); `neonctl branches create --parent-lsn ... --parent-timestamp ...` (user).
- **Snapshots**: console UI today; programmatic API + scheduled snapshots on the roadmap.
- **Settings**: `pitr_interval`, `gc_horizon`, `image_creation_threshold` (default 3), `compaction_target_size` (128 MB), `checkpoint_distance` (open-layer flush threshold), `compaction_period` (1 s).

## Direct relevance to a SQLite-on-FDB PITR design

- **The 2D `(key, LSN)` layer-file model transplants almost verbatim.** Substitute `txid: u64` for LSN and `(actor_id, page_no)` (or whatever your SQLite VFS shard key is) for Postgres `(rel, blockno)`. Your existing DELTA/SHARD on FDB is already the L0-delta + image-layer pair; you are missing only (a) the per-actor `LayerMap` indexing those by `(page-range, txid-range)` and (b) the COW pointer `(ancestor_actor_id, ancestor_txid)` on a forked actor. Branch creation becomes "write one row in FDB". Reads below `ancestor_txid` traverse the parent's DELTA/SHARD chain.
- **PITR == branch-at-txid.** Do not design PITR as "rewind in place". Make it `fork(actor_id, at_txid) -> new_actor_id` and let the user swap the routing pointer. This is what gives Neon "instant restore regardless of size", and it works exactly the same on FDB because no data moves. Bookmarks are then "named (actor_id, txid) tuples retained beyond the 30-day window", which maps to Neon's Snapshots.
- **GC horizon must be a graph predicate, not a scalar.** A delta is deletable iff `(txid < now - retention) AND no descendant fork pins it AND a covering image SHARD exists`. Single-scalar GC will silently break forks the moment one diverges (see Neon issue #707). Track parent->child timeline edges and refcount per `ancestor_txid`.
- **The safekeeper tier is mostly not your problem** because per-actor SQLite already has a single-writer invariant (the Pegboard exclusivity rule from CLAUDE.md). You don't need Paxos quorum on the WAL — the actor *is* the proposer and FDB is the durable acceptor in one hop. What you do still need from the safekeeper design: a fence between "txid committed to FDB" and "delta visible to readers/forks", plus an explicit ack from the materializer (your equivalent of pageserver) before a delta becomes eligible for cold-tier offload.
- **Compaction policy is the load-bearing detail nobody warns you about.** The Neon team's recent perf work was almost entirely about L0 backpressure and image-layer cadence. For a per-actor SQLite world, an `image_creation_threshold = 3` analog (write a SHARD after N DELTAs accumulated for a page range) plus `compaction_target_size ~ 128 MB` (or much smaller — actor pages are tiny) plus ingest backpressure when L0 count exceeds a watermark are the parameters that decide whether reads stay <ms or fall off a cliff. Also: separate background L1 compaction from foreground L0 flush so a hot actor doesn't starve.
- **What does *not* transplant**: the Postgres WAL-redo helper (you don't have WAL records, you have full pages in DELTAs — your reconstruction is a simple page-level replay, no separate redo process needed); the per-tenant pageserver+local-disk tier (FDB *is* your hot cache, S3 is your cold cache, no third tier needed in v1); the Paxos safekeeper cluster (single-writer SQLite with FDB durability removes that need entirely).

## Direct quotes / code refs

- `pageserver/src/tenant/layer_map.rs`: "The persistent BST maintains a map of which layer file 'covers' each key. It has only one dimension, the key." `LayerMap { open_layer, frozen_layers: VecDeque<...>, historic: BufferedHistoricLayerCoverage, l0_delta_layers: Vec<...> }`. Search priority: image > delta > in-memory; pick most recent layer with `lsn <= end_lsn`.
- `docs/pageserver-storage.md`: "ImageLayer represents a snapshot of all keys in a particular range, at one particular LSN. Any keys that are not present in the ImageLayer are known not to exist at that LSN." "DeltaLayer represents a collection of WAL records or page images in a range of LSNs, for a range of keys." "start_lsn is inclusive. end_lsn is exclusive."
- `docs/settings.md`: `pitr_interval` default 7 days, `image_creation_threshold` default 3, `compaction_target_size` default 128 MB, `compaction_period` default 1 s, `gc_horizon` floor (default 64 MB).
- `docs/walservice.md`: "WAL record is durable when the majority of safekeepers have received and stored the WAL to local disk." Pageserver is a learner; safekeeper GC waits for pageserver ack + S3 upload.
- `docs/glossary.md`: "Timeline accepts page changes and serves get_page_at_lsn() and get_rel_size() requests. The term 'timeline' is used internally in the system, but to users they are exposed as 'branches'."
- Neon blog (architecture-decisions): "Files are written sequentially and never modified in place. ... made it straightforward to support branching."
- Neon blog (pitr-deep-dive): "Neon creates a new branch of the database at a specific LSN, instantly making the restored state available. ... No data is truly 'copied'; the branch simply references existing storage layers up to that point."
- Neon docs (restore-window): "WAL records that exceed your restore window are automatically removed and stop contributing to your project's storage costs."
- API: `POST /api/v2/projects/{project_id}/branches` with `branch.parent_lsn` / `branch.parent_timestamp`. Response carries resolved `parent_lsn` (e.g. `"0/1FA22C0"`) + `parent_timestamp`.
- GitHub issue #707: "Garbage collection removes image layers that are still needed by delta layers" — historical reminder that GC must be dependency-aware, not just LSN-thresholded.

## Open questions

1. **Snapshot durability semantics**: Neon's Snapshots are still early-access; the public material does not say whether snapshot bytes are *copied* to a separate retained tier or whether they are layer-file pins that override `pitr_interval` GC. The latter is cheaper and is almost certainly the implementation, but worth confirming via the open-source repo's `tenant/timeline.rs` before copying the design.
2. **Cross-shard branch coherence**: With sharded pageservers (256 MiB stripes), branching has to be atomic across all shards of a tenant at the same LSN. The docs mention sharding briefly; the consistency protocol for "branch at LSN X across N shards" is not described publicly. Less relevant for a per-actor SQLite world (one actor = one shard) but worth understanding if we ever shard a single actor's data.
3. **Image layer cadence under sparse writes**: `image_creation_threshold = 3` triggers on L0 count, not on time. A cold key range that gets 3 deltas over a year still pays the read-amplification cost until a write hits it. Neon's solution (if any) wasn't found in the public material; might be a periodic full-keyspace image sweep gated on age.
4. **Parent-pin storage accounting**: when a child branch ages out of the restore window, exactly which layer-file bytes are billed? The blog says "minimum of accumulated changes vs. underlying storage footprint", which suggests a per-layer dedup-aware accounting that we'd want to mirror.
5. **Snapshot vs branch UX collision**: Neon now has three overlapping primitives — branches, PITR-as-branch, and snapshots. They don't fully describe the decision tree. For our system, "bookmark" + "fork" is probably enough; we should not adopt all three.
