# LiteFS — distributed SQLite + LTX format

Research on Fly.io's LiteFS, the LTX file format, the replication wire protocol,
LiteFS Cloud / LFSC, and what it does and does not provide for PITR / forking.
Compiled to inform Rivet's per-actor SQLite PITR + forking design (VFS-based,
UDB hot path, S3 cold storage, LTX V3 already in use).

## Sources

- [superfly/litefs ARCHITECTURE.md](https://github.com/superfly/litefs/blob/main/docs/ARCHITECTURE.md) — top-level design notes (FUSE, leases, HTTP, rolling checksum).
- [superfly/litefs README.md](https://github.com/superfly/litefs/blob/main/README.md) — feature scope.
- [superfly/litefs `db.go`](https://github.com/superfly/litefs/blob/main/db.go) — per-DB on-disk layout, page checksum vector, HWM.
- [superfly/litefs `litefs.go` stream frame types](https://github.com/superfly/litefs/blob/main/litefs.go) — wire frames: LTX/Ready/End/DropDB/Handoff/HWM/Heartbeat.
- [superfly/litefs `http/server.go`](https://github.com/superfly/litefs/blob/main/http/server.go) — `/stream`, `/export`, `/import`, `/promote`, `/handoff`, `/halt`, `/tx`, `/events`.
- [superfly/litefs `lfsc/backup_client.go`](https://github.com/superfly/litefs/blob/main/lfsc/backup_client.go) — LFSC client: `/pos`, `/db/tx`, `/db/snapshot`, `Litefs-Hwm` header.
- [superfly/ltx README.md](https://github.com/superfly/ltx/blob/main/README.md) — LTX byte layout (legacy v1 fields).
- [superfly/ltx `ltx.go`](https://github.com/superfly/ltx/blob/main/ltx.go) — current V3 header (100B), trailer (16B), filename format `<min>-<max>.ltx`.
- [Fly blog: Introducing LiteFS](https://fly.io/blog/introducing-litefs/) — design rationale, journal-deletion interception.
- [Fly blog: LiteFS Cloud](https://fly.io/blog/litefs-cloud/) — managed PITR, L1/L2/L3 compaction, 5-min granularity.
- [Fly docs: How LiteFS Works](https://fly.io/docs/litefs/how-it-works/) — rollback vs WAL capture.
- [Fly docs: Tracking LiteFS replication position](https://fly.io/docs/litefs/position/) — TXID + checksum pair.
- [Fly docs: LiteFS event stream](https://fly.io/docs/litefs/events/) — `/events` newline-JSON.
- [stephen/litefs-backup](https://github.com/stephen/litefs-backup) — open-source LFSC alternative; same L1/L2/L3 model.

## Architecture overview

LiteFS interposes a FUSE filesystem between the application and SQLite's
on-disk files. It is "a passthrough file system which intercepts these API
calls and copies out the page sets for each transaction"
([how-it-works](https://fly.io/docs/litefs/how-it-works/)).

Per-database directory layout on each node (from `db.go`):

- `database` — the real SQLite file (`db.DatabasePath()`).
- `journal` — rollback journal (`db.JournalPath()`).
- `wal`, `shm` — WAL + shared-memory (`db.WALPath()`, `db.SHMPath()`).
- `ltx/` — directory of LTX transaction files named `<minTXID>-<maxTXID>.ltx`
  (`db.LTXDir()`, `db.LTXPath(min, max)`).

Topology is single-writer primary + async read replicas:

- Primary election uses **Consul sessions / leases**, not Raft. From
  ARCHITECTURE.md: "it cannot use a distributed consensus algorithm that
  requires strong membership such as Raft. Instead, it delegates leader
  election to Consul sessions and uses a time-based lease system."
- Replicas connect to the primary over HTTP/2 (`/stream`) and receive LTX
  frames. The primary intercepts journal deletion (rollback) or WAL writes
  to materialize each transaction as an LTX file.
- Clients see a normal SQLite database file under the FUSE mount; on
  replicas the mount denies writes and exposes a `.primary` virtual file
  that names the current leader.

Trust model: LiteFS assumes intra-cluster trust and HTTP/2 plaintext (h2c)
on the replication socket. The wire protocol is not designed to be exposed
to untrusted peers.

## LTX format (LiteFS variant)

LTX v3 (current, `Version = 3` in `ltx.go`). Big-endian throughout. Magic is
ASCII `"LTX1"`. Sizes from `ltx.go` constants: `HeaderSize = 100`,
`PageHeaderSize = 6`, `TrailerSize = 16`.

### Header (100 bytes)

| Off | Size | Field                                                |
|-----|------|------------------------------------------------------|
| 0   | 4    | Magic `"LTX1"`                                       |
| 4   | 4    | Flags (`HeaderFlagNoChecksum = 1<<1` is the only one)|
| 8   | 4    | Page size (bytes; power of two 512..65536)           |
| 12  | 4    | `Commit` — DB size after txn, in pages               |
| 16  | 8    | `MinTXID`                                            |
| 24  | 8    | `MaxTXID`                                            |
| 32  | 8    | Timestamp (ms since epoch)                           |
| 40  | 8    | `PreApplyChecksum` (CRC-ISO-64)                      |
| 48  | 8    | `WALOffset` (origin WAL offset; 0 if journal)        |
| 56  | 8    | `WALSize`                                            |
| 64  | 4    | `WALSalt1`                                           |
| 68  | 4    | `WALSalt2`                                           |
| 72  | 8    | `NodeID` (origin node)                               |
| 80  | 20   | reserved (zero)                                      |

(README.md still documents the older v1 header. The Go source is the
authoritative current layout; there is also a `Database ID` field in v1
that is gone in v3 — `db.go` notes the `chksums.pages` vector replaces it.)

Snapshots: `IsSnapshot()` is `MinTXID == 1`; snapshots must contain every
live page and must have `PreApplyChecksum == 0`.

### Page block

Repeated frames of `PageHeader (6 bytes) || page data`:

| Off | Size | Field            |
|-----|------|------------------|
| 0   | 4    | Page number      |
| 4   | 2    | Flags (`PageHeaderFlagSize = 1<<0` ⇒ data is LZ4 block-format with a leading 4-byte size) |

Pages are written sorted by page number. Sorted order is the property that
makes LTX files merge-compactable into larger TXID-range files in O(N) and
makes a snapshot indistinguishable from a TXID-range LTX whose `MinTXID=1`.

### Trailer (16 bytes)

| Off | Size | Field                                       |
|-----|------|---------------------------------------------|
| 0   | 8    | `PostApplyChecksum` (rolling CRC after txn) |
| 8   | 8    | `FileChecksum` (CRC-ISO-64 over file)       |

### Checksum strategy

Two distinct checksums:

1. **File checksum** — CRC-ISO-64 over the entire LTX file bytes (header
   + page block + first 8 bytes of trailer). Integrity of one LTX file.
2. **Rolling DB checksum** (`PreApplyChecksum`, `PostApplyChecksum`) — a
   CRC-ISO-64 over `(pgno, page bytes)`, XOR-folded across every page in
   the database. From ARCHITECTURE.md: "When a page is written, LiteFS
   will compute the CRC64 of the page number and the page data and XOR
   them into the rolling checksum. It will also compute this same page
   checksum for the old page data and XOR that value out of the rolling
   checksum." XOR is associative, so the value can be re-derived from a
   raw DB file and used as a content-addressable fingerprint of the DB
   state at any TXID.

`ChecksumFlag = 1 << 63` is OR'd into checksums so a zero value reliably
means "absent".

### Position

`ltx.Pos = (TXID, PostApplyChecksum)` is the unit of replication state
everywhere — wire frames, HTTP responses, durability HWM. Two replicas
holding the same `(TXID, checksum)` are byte-identical. Two replicas with
the same TXID but different checksum are split-brained and one of them
must resnapshot.

## Replication protocol

### LTX streaming

Replica → primary connection is a single HTTP/2 POST to `/stream`
(rejected if `r.ProtoMajor < 2`, see `http/server.go`). Body is a position
map `name → Pos` describing where each replica DB currently sits. The
response is a long-lived stream of typed frames defined in `litefs.go`:

```
StreamFrameTypeLTX       = 1   // header + chunked LTX bytes
StreamFrameTypeReady     = 2   // initial catch-up complete
StreamFrameTypeEnd       = 3   // stream closing cleanly
StreamFrameTypeDropDB    = 4   // database removed
StreamFrameTypeHandoff   = 5   // please become primary; carries lease ID
StreamFrameTypeHWM       = 6   // updated backup high-water mark per DB
StreamFrameTypeHeartbeat = 7   // liveness, carries unix-ms timestamp
```

Each frame is `uint32 type` followed by a self-describing body. LTX frames
embed an `LTXStreamFrame{Name}` and then an LTX file streamed via a
length-prefixed chunked reader (`internal/chunk/chunk.go`).

### TXID sequence

`TXID uint64`, autoincrementing per database, formatted as 16 hex chars on
disk and on the wire (`fmt.Sprintf("%016x", ...)`). The primary owns
allocation; replicas do not allocate TXIDs unless they receive a halt
lock and forward a transaction back via `POST /tx`.

LTX filenames encode an inclusive range `MinTXID-MaxTXID`. Single-txn
files are `00..N-00..N.ltx`; compactions yield `00..A-00..B.ltx` with
`A < B`. Filenames are the catch-up index — replicas list the directory
and pick the file whose `MinTXID = client.TXID + 1`.

### Catch-up after disconnect

The primary's `streamLTX()` (`http/server.go` ~L686):

1. Look up the next LTX file `(clientPos.TXID + 1)`.
2. Validate `PreApplyChecksum == clientPos.PostApplyChecksum`. Mismatch
   ⇒ `streamLTXSnapshot()` (full re-export of the live DB as a single
   `MinTXID=1` LTX).
3. Validate `clientPos.TXID <= dbPos.TXID`; if the client is *ahead* of
   the primary, the primary clears the client position and snapshots.
4. Stream the LTX file as one frame; on success emit an `HWMStreamFrame`
   with the latest backup HWM so the replica knows what's safe to GC.

The replica's `processLTXStreamFrame()` validates `PreApplyChecksum`
against its own `Pos()` before applying. Either side may force a
resnapshot; the file is self-describing.

### Conflict / split-brain handling

The rolling checksum is the conflict detector. From ARCHITECTURE.md:
"When the old primary node connects to the new primary node, it will see
that its checksum is different even though its transaction ID could be
the same. At this point, it will resnapshot the database from the new
primary to ensure consistency." There is no merge — the new primary's
state always wins, and divergent transactions on the demoted primary
are silently dropped.

Async replication ⇒ "a window of time where transactions are only
durable on the primary node" (ARCHITECTURE.md). LiteFS does not provide
strong-consistency writes; downstream backups (LFSC) close the gap by
acking via `Litefs-Hwm`.

## LiteFS Cloud (LFSC)

LFSC is the managed PITR service. Open-source equivalent:
[stephen/litefs-backup](https://github.com/stephen/litefs-backup).

### Wire protocol (from `lfsc/backup_client.go`)

- `GET  /pos?cluster=<id>` → JSON `name → Pos` map of what LFSC has.
- `POST /db/tx?cluster=<id>&db=<name>` → upload one LTX file.
  - Response header `Litefs-Hwm: <16-hex TXID>` is the new durable HWM.
  - On gap: `EPOSMISMATCH` JSON error with the LFSC-side `Pos`; the LiteFS
    primary re-uploads from there or re-snapshots.
- `GET  /db/snapshot?cluster=<id>&db=<name>` → full DB snapshot (used
  when the primary is far enough behind LFSC that gap-fill is impractical).
- Auth via `Authorization` header; cluster scoped via `Litefs-Cluster-Id`.
- `fly-force-instance-id` header pins routing to the LFSC instance that
  owns the cluster (Fly-proxy hint, not protocol).

### S3 layout (multi-level compaction)

From [LiteFS Cloud blog](https://fly.io/blog/litefs-cloud/): "LiteFS Cloud
uses a hierarchical compaction system based on Lite Transaction Files":

- **L1** — fine-grained windows (~5 min as observed by users; 10 s in
  the open-source `litefs-backup` deployment per its README).
- **L2** — hourly aggregations.
- **L3** — daily full snapshots (the restore "anchor"; every PITR replays
  from the most recent L3 snapshot forward).

A restore at time `T` resolves to one L3 snapshot at `T_d ≤ T`, then the
L2 segments covering `(T_d, T_h]`, then the L1 segments covering
`(T_h, T]`. The hierarchy bounds restore I/O independent of retention
window length.

### PITR / retention

- "the equivalent of a snapshot every five minutes (8760 snapshots per
  month!)" ([LiteFS Cloud blog](https://fly.io/blog/litefs-cloud/)).
- Retention: 30 days.
- Restore granularity: 5 minutes.
- Restore latency: "a couple of seconds (or less)".

### Durability handshake

The `Litefs-Hwm` header is propagated back through `HWMStreamFrame`
(`StreamFrameTypeHWM = 6`) on the replication stream. From the LiteFS
Cloud blog: transaction IDs "are propagated back down to the nodes of
the LiteFS cluster so LiteFS Cloud can ensure that the transaction file
is not removed from any node until it is safely persisted in object
storage". This is how LiteFS prevents local LTX GC from outrunning S3
durability.

## Forking

**Limited.** LiteFS itself has no in-protocol fork operation. The only
LiteFS-native cloning primitives are:

- `GET /export?name=<db>` — primary streams a full snapshot; consumer
  pipes it through `POST /import?name=<db>` to a different cluster.
  Equivalent to `litefs export` / `litefs import` CLI commands.
- `streamLTXSnapshot()` — internal, used to bootstrap replicas.

LFSC adds higher-level cluster cloning:
"you can clone your LiteFS Cloud cluster to a new cluster, which you
could use for a staging environment (or on-demand test environments
for your CI pipelines) with real data" ([LiteFS Cloud blog](https://fly.io/blog/litefs-cloud/)).

Mechanism (inferred — not documented at byte level): take the latest L3
snapshot and replay L2 + L1 up to the chosen point, then materialize
the resulting DB into a new cluster's S3 prefix and let new LiteFS
nodes bootstrap from `GET /db/snapshot`. There is no copy-on-write at
the page level — fork = full replay + new TXID lineage.

There is no "fork at TXID T producing a writable child that diverges
from parent" primitive: every cluster has its own monotone TXID space.

## Page index

This is the most informative finding for Rivet's design.

**LiteFS does not maintain a `pgno → latest_txid` index.** It does not
need to: it keeps the live SQLite database file on disk in the per-DB
directory (`<path>/database`). Reads on the FUSE mount are passthrough
to that real file. A page lookup is just a `pread` at offset
`(pgno-1) * page_size`. The LTX directory is the *write log*, not the
read path.

What LiteFS *does* keep in memory per DB (`db.go` ~L37–L52):

```go
pageN     atomic.Uint32           // db size in pages
chksums struct {
    pages  []ltx.Checksum         // per-page CRC64
    blocks []ltx.Checksum         // aggregated, ChecksumBlockSize-sized buckets
}
```

That's the *content fingerprint* per page, not a TXID map. Two roles:

1. Recompute the rolling DB checksum quickly when a single page changes
   (XOR out the old `pages[pgno]`, XOR in the new one, update `blocks[]`).
2. Cheap divergence detection between two DB instances at the same
   nominal TXID.

LTX files on disk are kept in `ltx/` ordered by filename. To replay
forward from `Pos P`, LiteFS lists the directory, picks the file whose
`MinTXID = P.TXID + 1`, validates `PreApplyChecksum == P.Checksum`, and
applies it (sorted page block ⇒ in-place page writes to `database`).

Implication for Rivet: LiteFS solves the "current page" problem by not
having that problem — the canonical state is always the materialized
SQLite file, and LTX is a derived journal. Rivet's UDB hot path /
chunked-cold-storage design is the *opposite* model (no canonical
materialization; pages live as the youngest delta). The LiteFS rolling
checksum + sorted page block + LTX filename indexing patterns transfer;
the read-path passthrough does not.

## Direct quotes / code references

- LTX V3 constants — `ltx.go`:
  ```go
  const ( Magic = "LTX1"; Version = 3 )
  const ( HeaderSize = 100; PageHeaderSize = 6; TrailerSize = 16 )
  const ChecksumFlag Checksum = 1 << 63
  ```
- Per-DB on-disk layout — `db.go`:
  ```go
  func (db *DB) LTXDir() string  { return filepath.Join(db.path, "ltx") }
  func (db *DB) DatabasePath() string { return filepath.Join(db.path, "database") }
  func (db *DB) JournalPath() string  { return filepath.Join(db.path, "journal") }
  func (db *DB) WALPath() string      { return filepath.Join(db.path, "wal") }
  ```
- Stream frame catalogue — `litefs.go`:
  ```go
  StreamFrameTypeLTX, Ready, End, DropDB, Handoff, HWM, Heartbeat = 1..7
  ```
- Resnapshot trigger — `http/server.go` `streamLTX`:
  ```
  // Invalidate client position if the TXID matches but the checksum does not.
  if clientPos.TXID == dbPos.TXID && clientPos.PostApplyChecksum != dbPos.PostApplyChecksum {
      // …clear, fall through to streamLTXSnapshot
  }
  ```
- LFSC durability handshake — `lfsc/backup_client.go`:
  ```go
  // POST /db/tx
  hwmStr := resp.Header.Get("Litefs-Hwm")
  hwm, err = ltx.ParseTXID(hwmStr)
  // EPOSMISMATCH → ltx.NewPosMismatchError(e.Pos)
  ```
- Rolling checksum mechanic — ARCHITECTURE.md:
  > "When a page is written, LiteFS will compute the CRC64 of the page
  > number and the page data and XOR them into the rolling checksum. It
  > will also compute this same page checksum for the old page data and
  > XOR that value out of the rolling checksum."

## Open questions

1. **L2/L3 page-index format inside LFSC.** Public docs only show `/db/tx`
   ingest and `/db/snapshot` egress. The internal index that resolves
   "T → which L3 + L2 + L1 chain" is not documented. `litefs-backup`
   source is the closest open reference.
2. **Compaction page-collision rules.** When LiteFS rolls many TXID-range
   LTX files into one, duplicate page numbers from later TXIDs must
   override earlier ones. The Go `ltx` library has compaction helpers
   (`Compactor`, see `compactor.go`) — worth a follow-up read for how
   the sorted page block + last-write-wins is implemented.
3. **Forking semantics in LFSC clone.** "Clone cluster" in the blog
   post is described product-side, not at the byte level. Whether the
   clone re-derives a fresh TXID lineage (recommended) or carries over
   the parent's TXIDs (risk: future split-brain confusion) is not
   public. For Rivet, fork should always reset TXID lineage and emit a
   new fork ancestry record.
4. **WAL2 / Begin-Concurrent.** ARCHITECTURE.md mentions "It will
   possibly support `wal2` in the future." No public progress as of
   this research.
5. **Client read-path latency on FUSE.** Every page read traverses
   FUSE, which costs ~5–20 µs per syscall on Linux. SQLite's page
   cache absorbs most of this. Whether LiteFS adds a userspace
   `pages[]` cache as well is not documented; `db.go` only caches
   *checksums*, not page bytes.
