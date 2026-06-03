# Turso / libSQL — Bottomless storage + PITR + branching

Research notes pulled from official Turso docs, blog posts, and the
`tursodatabase/libsql` source (`bottomless/` extension and `libsql-server`).

## Sources

- [docs.turso.tech — Point-in-Time Recovery](https://docs.turso.tech/features/point-in-time-recovery) — User-facing PITR docs (CLI, API, retention, granularity gap).
- [docs.turso.tech — Branching](https://docs.turso.tech/features/branching) — User-facing branching docs (CLI, API, limitations).
- [Turso blog — How does Turso Cloud keep your data durable and safe?](https://turso.tech/blog/how-does-the-turso-cloud-keep-your-data-durable-and-safe) — Most informative public source on internal segments/generations + S3 Express + 90-day PITR.
- [Turso blog — Track Database Branching with Turso Cloud](https://turso.tech/blog/track-database-branching-with-turso-cloud) — Confirms branching is metadata-only; introduces parent metadata API.
- [Medium / Glauber Costa — Turso now supports Database Branching and Point-in-Time Restore](https://medium.com/chiselstrike/turso-now-supports-database-branching-and-point-in-time-restore-eaadb8c4dce5) — Original announcement; describes PITR/branching as "metadata-only" with shared generations.
- [Turso blog — Introducing Databases Anywhere with Turso Sync](https://turso.tech/blog/introducing-databases-anywhere-with-turso-sync) — New sync model in the Turso DB rewrite (push logical, pull physical).
- [GitHub — `tursodatabase/libsql` `bottomless/` directory](https://github.com/tursodatabase/libsql/tree/main/bottomless) — Source of the bottomless extension.
- [`bottomless/README.md`](https://github.com/tursodatabase/libsql/blob/main/bottomless/README.md) — Build/config + bottomless-cli reference.
- [`bottomless/src/replicator.rs`](https://github.com/tursodatabase/libsql/blob/main/bottomless/src/replicator.rs) — Authoritative S3 layout, generation rotation, restore code path.
- [`libsql-server/README.md`](https://github.com/tursodatabase/libsql/blob/main/libsql-server/README.md) — `--enable-bottomless-replication`, env var surface.
- [HN: discussion on bottomless restore behavior](https://news.ycombinator.com/item?id=34519898) — Maintainers describing intended bottomless behavior.
- [Substack — libSQL: Diving Into a Database Engineering Epic](https://compileralchemy.substack.com/p/libsql-diving-into-a-database-engineering) — Third-party deep dive on libSQL internals.
- [Discussion #738 — Replication to S3](https://github.com/tursodatabase/libsql/discussions/738) — Operator-level configuration questions.

Note: the public Turso docs only describe the user-facing PITR/branching surface. The implementation details below come almost entirely from reading `bottomless/src/replicator.rs` directly. The newer "Turso DB" codebase under `tursodatabase/turso` (rewrite, not `libsql`) is moving toward a different sync/storage design (Turso Sync), but PITR + branching today are powered by the bottomless+sqld stack.

## libSQL server / sqld architecture

- libSQL is an open-source fork of SQLite maintained by Turso. `sqld` (now `libsql-server`) is the server that wraps embedded libSQL with HTTP/WS protocols, replication, and bottomless backup.
- Bottomless is a **Virtual WAL** plugin built into libSQL (libSQL's "virtual WAL" is a libSQL-only extension on top of SQLite's WAL design). This gives bottomless full visibility into every WAL frame at write time without having to tail `*-wal` from another process. Source: `bottomless/README.md` ("This project implements a virtual write-ahead log (WAL) which continuously backs up the data to S3-compatible storage and is able to restore it later").
- Enable with `--enable-bottomless-replication` on `libsql-server`, or load `bottomless.so` in the libSQL shell with `.open file:test.db?wal=bottomless`. WAL journal mode is required (`PRAGMA journal_mode=wal;`).
- Configuration is read from env vars (`Options::from_env`, replicator.rs:153–225):
  - `LIBSQL_BOTTOMLESS_DATABASE_ID` (db identity in S3)
  - `LIBSQL_BOTTOMLESS_ENDPOINT` (S3 endpoint, defaults to `http://localhost:9000`)
  - `LIBSQL_BOTTOMLESS_BUCKET` (default `bottomless`)
  - `LIBSQL_BOTTOMLESS_AWS_*` credentials (`ACCESS_KEY_ID`, `SECRET_ACCESS_KEY`, `SESSION_TOKEN`, `DEFAULT_REGION`)
  - `LIBSQL_BOTTOMLESS_BATCH_INTERVAL_SECS` (default 15s — max wall time between S3 frame batch uploads)
  - `LIBSQL_BOTTOMLESS_BATCH_MAX_FRAMES` (default 10000 — max frames per batched S3 object)
  - `LIBSQL_BOTTOMLESS_S3_PARALLEL_MAX` (default 32 — concurrent S3 uploads)
  - `LIBSQL_BOTTOMLESS_COMPRESSION` (default `zstd`; also `gzip`, `none`)
  - `LIBSQL_BOTTOMLESS_VERIFY_CRC` (default true — verify each frame's checksum during restore)
  - `LIBSQL_BOTTOMLESS_SKIP_SNAPSHOT`, `LIBSQL_BOTTOMLESS_SKIP_SHUTDOWN_UPLOAD` (test/operator knobs)
  - `LIBSQL_BOTTOMLESS_ENCRYPTION_CIPHER` / `LIBSQL_BOTTOMLESS_ENCRYPTION_KEY` (envelope encryption of frames)
- Server cluster topology (separate from bottomless) uses logical replication: a primary `sqld` ships frames to read replicas over gRPC; embedded replicas are clients of that same protocol.

## Bottomless replication

### Frame format

- A frame is exactly one SQLite WAL frame: a 24-byte WAL frame header (page number, "size after" commit marker, salts, CRCs) followed by `page_size` bytes (commonly 4096; bottomless test uses 64 KiB).
- Bottomless reuses the WAL CRC chain (`checksum: (u32, u32)` rolling pair) for integrity. The `.meta` object captures the initial (page-1) checksum so the chain can be verified on restore (`store_metadata` writes `page_size:u32 || crc1:u32 || crc2:u32`, big-endian, replicator.rs:1917–1940).
- During restore, frames are decoded into `libsql_replication::frame::FrameHeader { frame_no, checksum, page_no, size_after }` and replayed via `SqliteInjector::inject_frame` against a fresh DB file (replicator.rs:1672–1689).
- Frames inside an S3 batch object are compressed end-to-end (one stream per object — gzip or zstd — not per-frame), see `BatchReader` consumption.

### S3 layout (verbatim from `Options` doc-comment)

```
/// Bucket directory name where all S3 objects are backed up. General schema is:
/// - `{db-name}-{uuid-v7}` subdirectories:
///   - `.meta` file with database page size and initial WAL checksum.
///   - Series of files `{first-frame-no}-{last-frame-no}.{compression-kind}` containing
///     the batches of frames from which the restore will be made.
```
(replicator.rs:106–112)

Concrete object keys observed in the source:

- `{db-name}-{generation-uuid}/.meta` — 12 bytes: `page_size:u32 || crc_lo:u32 || crc_hi:u32` (big-endian).
- `{db-name}-{generation-uuid}/db.{gz|zstd|raw}` — full compressed (or raw) main DB file snapshot. Suffix is the compression kind. Written via `snapshot_main_db_file` (replicator.rs:1095).
- `{db-name}-{generation-uuid}/.changecounter` — 4-byte SQLite change counter, used as an "is local newer than remote?" tiebreaker (replicator.rs:1106–1115). The code comments warn this is unreliable in WAL mode and the WAL checksum is the real source of truth.
- `{db-name}-{generation-uuid}/.dep` — 16 bytes containing the parent generation UUID. Establishes the generation lineage (replicator.rs:804). Crucial for cross-generation restore.
- `{db-name}-{generation-uuid}/{first}-{last}-{timestamp}.{gz|zstd|raw}` — batched WAL frame ranges. Parsing in `parse_frame_range` (replicator.rs:1283–1299): frame numbers are `u32`, timestamp is unix seconds (`u64`).
- `{db-name}.tombstone` — 8-byte big-endian unix timestamp marking the entire DB as deleted (`delete_all`, replicator.rs:1965+).

S3 client uses `force_path_style(true)` (replicator.rs:140), so MinIO and S3-compatible stores all work.

### Generation rotation

- Each generation is a UUID v7 (timestamp-prefixed; `uuid_unstable` cargo cfg required to install `bottomless-cli`). Lexicographic sort order = creation order, which is critical because `parse_frame_range` and `restore_wal` rely on `ListObjects` returning frames in order.
- A new generation is created on:
  - boot (every server start triggers a new generation; old WAL pages from the previous generation are flushed first via `maybe_replicate_wal`, replicator.rs:1024–1056),
  - successful restore that applied WAL frames (action `RestoreAction::SnapshotMainDbFile`, replicator.rs:1290–1295), and
  - explicit cluster events.
- A generation links to its predecessor via the `.dep` object (`store_dependency`, replicator.rs:801–828). This is best-effort and async because it would otherwise add latency to checkpoint.
- Generation lookup on restore (`full_restore`, replicator.rs:1359–1402) walks the `.dep` chain backward up to `MAX_RESTORE_STACK_DEPTH = 100` generations until it finds a generation that has a `db.{gz|zstd|raw}` snapshot, then replays WAL forward through every generation on the stack.

### Compaction / batching

- Frames are buffered locally and flushed to S3 by `WalCopier::flush` either when (a) `LIBSQL_BOTTOMLESS_BATCH_MAX_FRAMES` (default 10 000) frames have accumulated, or (b) `LIBSQL_BOTTOMLESS_BATCH_INTERVAL_SECS` (default 15 s) elapses since the last flush (replicator.rs:382–428). Each flush produces one S3 object covering a contiguous frame range.
- On a SQLite WAL checkpoint, bottomless takes a fresh `db.{compression}` snapshot of the main DB file and uploads it as the snapshot for the *current* generation (replicator.rs:1075–1170). Frame-range objects within that generation are not deleted; the snapshot is just an additional optimization so a future restore can short-circuit the WAL replay.
- `bottomless-cli rm --older-than <date>` deletes generations older than a date (`bottomless/README.md`). There is no automatic GC of old generations in the published OSS bottomless code; deletion is operator-driven (or, in Turso Cloud, driven by the plan's retention window).
- The 15-second default `BATCH_INTERVAL_SECS` is exactly the "up to 15 second gap immediately preceding the timestamp" warning that appears in the public PITR docs. The acknowledged data is durable in S3 only after the next batch flush.

## Point-in-time recovery

### Granularity

- **Per-frame in S3, but per-batch in time.** Every WAL frame is preserved (no truncation between snapshots), so any successfully-uploaded frame can be replayed. The wall-clock granularity is bounded by the batch flush cadence: the docs explicitly note "there may be a gap of up to 15 seconds in the data immediately preceding the timestamp" — matching the default `LIBSQL_BOTTOMLESS_BATCH_INTERVAL_SECS=15` in code.
- The Turso Cloud blog claims tighter durability ("write latency of single-digit milliseconds" using **S3 Express**), but the granularity gap remains because PITR resolves to a frame batch, not an individual frame timestamp.
- Restore selects the largest frame batch whose embedded timestamp `<= target_utc_time` (replicator.rs:1644–1659):

```rust
if let Some(threshold) = utc_time.as_ref() {
    match DateTime::from_timestamp(timestamp as i64, 0).map(|t| t.naive_utc()) {
        Some(timestamp) => {
            if &timestamp > threshold {
                tracing::info!("Frame batch {} has timestamp more recent than expected {}. Stopping recovery.", key, timestamp);
                break 'restore_wal;
            }
        }
        ...
```

### Mechanism

1. Find generation: `choose_generation(generation, timestamp)` picks the newest generation whose UUID v7 timestamp is `<=` target.
2. Walk `.dep` chain backward to the first generation with a `db.*` snapshot.
3. Apply that snapshot to a temp file (`data.tmp`), then walk the stack forward, replaying every batched frame range whose first frame number is exactly `last_injected_frame_no + 1`. Out-of-order or missing frames abort the restore.
4. When walking the final (target) generation, stop the moment a batch's timestamp exceeds the target.
5. Atomically rename `data.tmp` to the live DB path and remove `*-wal`, `*-shm`.

### Retention

| Plan      | PITR window |
|-----------|-------------|
| Free      | 24 hours    |
| Developer | 10 days     |
| Scaler    | 30 days     |
| Pro       | 90 days     |
| Enterprise| custom      |

Source: <https://docs.turso.tech/features/point-in-time-recovery>. Enforcement is by Turso Cloud's lifecycle/retention policy on the S3 bucket; the OSS bottomless code itself has no built-in retention enforcer.

### CLI / API surface

User-facing (Turso Cloud):

```bash
turso db create my-new-database \
    --from-db my-existing-database \
    --timestamp 2024-01-01T00:00:00Z
```

```http
POST /v1/organizations/{orgSlug}/databases
{
  "name": "my-new-database",
  "seed": { "type": "database", "name": "my-existing-database",
            "timestamp": "2024-01-01T00:00:00Z" }
}
```

PITR always **creates a new database** — it never overwrites the existing one — and it produces a new connection string and new auth tokens. Consumes one slot of your DB quota.

Operator-facing (OSS `bottomless-cli`):

```
ls       List available generations
restore  Restore the database
rm       Remove given generation from remote storage
```

## Branching / forking

- Supported on Turso Cloud since the same announcement that introduced PITR.
- Mechanism (per Turso blog and `track-database-branching-with-turso-cloud`): **metadata-only, copy-on-write at the generation/segment level.** Quoting the docs: "The process of branching is metadata-only where generations belonging to a database become shared between databases, no data copying occurs, making branching instantaneous."
- This maps cleanly onto bottomless: a branch is a new database identity that points its initial generation's `.dep` (or a control-plane equivalent) at the parent's most recent generation. Both DBs read from the same shared upstream segments until either side writes — at which point each side's writes go into its own new generation.
- The Turso Cloud blog explicitly discusses 128 KiB **segments** as the unit of sharing: "Rather than storing complete database files, Turso splits databases into 128kB segments organized into collections called 'generations.' New database creation references existing segments without duplication." This sounds like a layer above the raw bottomless WAL-frame batches; the public OSS bottomless code does not implement segment-level dedup. It is plausible that Turso Cloud's production stack runs a generation/segment indirection on top of bottomless.
- Branching from a point-in-time uses the same `--timestamp` flag: a branch is "PITR + new identity," confirming the underlying mechanism is identical.

CLI:

```bash
turso db create my-branch --from-db my-existing-database
turso db create my-branch --from-db my-existing-database --timestamp <iso8601>
turso db show mydb --branches
```

API:

- `POST /v1/organizations/{org}/databases` with seed `{ type: "database", name: <parent>, timestamp?: <iso8601> }`.
- `GET /v1/organizations/{org}/databases?parent=<name>` returns child branches.
- Branched DBs are listed with a `parent: { id, name, branched_at }` field on `/databases`.

Limitations (per docs):

- A branch is a separate DB; **no automatic schema sync**. Merging back is manual.
- Branch needs its own auth token (or a shared group token).
- Counts toward the org's DB quota; manual cleanup required.

## Embedded replicas / sync

- Embedded replicas use a **logical-replication frame stream** between client and primary, separate from bottomless. Client connects with `(generation_uuid, last_frame_no)` and the primary streams missing frames forward. Frame numbers reset at generation boundaries.
- Same `Frame { frame_no, checksum, page_no, size_after, page_bytes }` shape used by `libsql_replication::injector::SqliteInjector` to apply incoming frames into the local DB.
- Sync is client-driven: the application calls `client.sync()` (or sets `syncInterval`) to pull. There is no server push.
- The newer **Turso Sync** model (in `tursodatabase/turso`, the rewrite) replaces this single `sync()` with explicit `push()` / `pull()`. Pull is still physical pages (server is source of truth, clients converge byte-for-byte). Push is *logical* mutations — apps run conflict resolution before applying.
- Server restart with a dirty WAL regenerates the replication log so newly-synced replicas pick up any extra frames.

## Storage tiering (local WAL vs S3 vs S3 Express)

- The primary's local WAL is the hot path. Reads and writes go through SQLite as normal.
- Bottomless intercepts WAL frame writes at the virtual-WAL layer and ships batches to S3 asynchronously. The acknowledgment back to the user only depends on local fsync, **not on S3** in the OSS bottomless codepath. Turso Cloud upgrades this: writes are acknowledged only after they hit **S3 Express One Zone** (single-digit-millisecond write latency, durability matching standard S3).
- Bottomless aggressively triggers SQLite checkpoints — Turso Cloud states "after every 1 MB of writes" — to keep the local WAL small. Each checkpoint becomes a snapshot upload opportunity.
- Cold tier: standard S3. Compute nodes can run with a small local cache (the blog uses 1 GB as an example) and load segments on-demand from S3 to satisfy reads. This is what makes hosting millions of mostly-cold actor DBs feasible.

## Direct quotes / code references

From `bottomless/src/replicator.rs` line 106–112:
> Bucket directory name where all S3 objects are backed up. General schema is:
> - `{db-name}-{uuid-v7}` subdirectories:
>   - `.meta` file with database page size and initial WAL checksum.
>   - Series of files `{first-frame-no}-{last-frame-no}.{compression-kind}` containing
>     the batches of frames from which the restore will be made.

From `bottomless/src/replicator.rs` ~line 1283 (`parse_frame_range`):
```rust
let first_frame_no = frame_suffix[0..last_frame_delim].parse::<u32>().ok()?;
let last_frame_no = frame_suffix[(last_frame_delim + 1)..timestamp_delim].parse::<u32>().ok()?;
let timestamp = frame_suffix[(timestamp_delim + 1)..compression_delim].parse::<u64>().ok()?;
```
Frame range objects are `{first}-{last}-{unix_seconds}.{ext}`.

From `bottomless/src/replicator.rs` ~line 805 (`store_dependency`):
```rust
let key = format!("{}-{}/.dep", self.db_name, curr);
let request = self.client.put_object().bucket(&self.bucket).key(key)
    .body(ByteStream::from(Bytes::copy_from_slice(prev.into_bytes().as_slice())));
```
A 16-byte parent UUID written under `.dep` per generation — the literal pointer that makes restore stack walking and (presumably) cloud-side branching possible.

From `bottomless/src/replicator.rs` ~line 1917 (`store_metadata`):
```rust
let key = format!("{}-{}/.meta", self.db_name, generation);
let mut body = Vec::with_capacity(12);
body.extend_from_slice(page_size.to_be_bytes().as_slice());
body.extend_from_slice(checksum.0.to_be_bytes().as_slice());
body.extend_from_slice(checksum.1.to_be_bytes().as_slice());
```
The `.meta` object is a fixed 12 bytes big-endian: `page_size || crc_lo || crc_hi`.

From `bottomless/src/replicator.rs` line 41:
```rust
const MAX_RESTORE_STACK_DEPTH: usize = 100;
```
A restore can chain through at most 100 generations (without hitting a snapshot) before bailing.

From Turso Cloud durability blog:
> Rather than storing complete database files, Turso splits databases into 128kB segments organized into collections called 'generations.' [...] New database creation references existing segments without duplication.

> Database branching is "a metadata-only operation, and no data copying occurs," making it instantaneous. Branched databases share generations and WAL fragments until divergence occurs through new writes.

From Turso PITR docs:
> Backup creation happens automatically at each COMMIT. [...] there may be a gap of up to 15 seconds in the data immediately preceding the timestamp due to Turso's periodic checkpoint timing.

From `bottomless/README.md`:
> All page writes committed to the database end up being asynchronously replicated to S3-compatible storage. On boot, if the main database file is empty, it will be restored with data coming from the remote storage.

## Open questions / not publicly documented

- **Segment vs frame-batch layer.** The 128 KiB "segment" abstraction described in the Turso Cloud blog does **not** appear in the OSS `bottomless` code, which uses variable-size compressed frame batches keyed by `{first}-{last}-{timestamp}`. Either Turso Cloud added a segment indirection on top of bottomless, or it's a future direction. Worth probing the `tursodatabase/turso` rewrite repo if we want a model closer to what they ship in production.
- **Branch point storage layout.** Public docs say generations are *shared* between branches, but the OSS bottomless `.dep` pointer is single-parent only and uses a single `{db-name}-...` prefix. The cloud control plane must layer on a database-id → generation-list mapping that allows N children to point at the same parent generation. Not in OSS.
- **Branch merge semantics.** Docs explicitly say merge is manual. There is no documented mechanism for replaying frames from one branch onto another — would require either a logical CDC stream (Turso Sync's logical-push direction) or an application-level diff.
- **PITR sub-15-second resolution.** S3 Express makes individual frames durable in milliseconds, but the public docs still warn 15 s gap. Unclear whether Turso Cloud has a smaller batch interval in production or is conservatively documenting the OSS default.
- **Compaction / segment GC.** OSS bottomless has no automatic GC; the cloud must track per-generation reference counts (because branches share generations) before a retention sweep is allowed to delete a generation. This logic is not in the OSS repo.
- **Encryption-at-rest semantics across branches.** Bottomless supports per-DB envelope encryption keys (`LIBSQL_BOTTOMLESS_ENCRYPTION_KEY`). If two branches share a parent generation, they must share the parent's key as well; key rotation across a generation boundary is undocumented.
- **Embedded replica handling of generation rotation.** Replicas track `(generation, frame_no)`. When the primary rotates generations, replicas must somehow learn the new generation id. Not detailed in public docs; presumably included in the gRPC handshake.
- **Turso Sync (rewrite) on-disk format.** The `tursodatabase/turso` rewrite has its own storage layer (different from libSQL's bottomless). The new push/pull model and conflict-resolution APIs suggest a different on-disk representation that is not yet covered in any of the linked docs.
