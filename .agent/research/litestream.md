# Litestream — WAL streaming to S3 + PITR

Research notes on Litestream's architecture as of v0.5.x (the LTX-based generation, not the legacy v0.3.x WAL-segment generation system). Focus is on what the Rivet engine should borrow when building continuous PITR + bookmarks + forking on top of the LTX V3 format we already use internally.

## Sources

- `https://litestream.io/how-it-works/` — high-level pipeline (shadow WAL → snapshot → S3), retention defaults.
- `https://litestream.io/reference/config/` — sync interval, snapshot interval, level intervals, S3 multipart options.
- `https://litestream.io/reference/restore/` — restore flag surface (`-timestamp`, `-txid`, `-f`, `-parallelism`).
- `https://fly.io/blog/litestream-v050-is-here/` — design rationale: LTX, dropped generations, three compaction tiers.
- `https://fly.io/blog/litestream-revamped/` — earlier rewrite announcement; positions Litestream as a sidecar.
- `https://github.com/superfly/ltx/blob/main/README.md` — canonical byte-level header/trailer table for LTX.
- `github.com/superfly/ltx` (`ltx.go`, `encoder.go`, `decoder.go`, `compactor.go`) — actual encoding, page index, LZ4 block compression, checksum semantics.
- `github.com/benbjohnson/litestream` (`store.go`, `db.go`, `replica.go`, `replica_client.go`, `compactor.go`, `compaction_level.go`, `litestream.go`, `v3.go`, `s3/replica_client.go`, `cmd/litestream/restore.go`) — runtime: monitor, sync, compaction levels, S3 wiring, restore, legacy v0.3.x compat.
- `https://deepwiki.com/benbjohnson/litestream/2.3-data-structures` — index pointer to LTX-format wiki page (mostly metadata; the in-tree files were authoritative).

## Architecture overview

Litestream is an out-of-process **sidecar** that opens the SQLite database read-only and streams WAL frames to a remote replica. There is no in-process integration with the application; the application keeps using stock SQLite.

The pipeline:

1. **Long-running read transaction.** On `DB.Open`, Litestream opens a `*sql.Tx` and holds it (`db.go: rtx *sql.Tx`). This blocks SQLite's automatic checkpoint, so the WAL only advances when Litestream chooses.
2. **Monitor loop.** Every `MonitorInterval` (default `1s`, `db.go:DefaultMonitorInterval`) Litestream observes the WAL.
3. **Sync.** `Replica.Sync` (`replica.go`) reads new committed frames and writes them out as LTX files at level 0. The `SyncInterval` (default `1s`, `replica.go:DefaultSyncInterval`) controls how often these L0 files are flushed to the replica client.
4. **Checkpoint.** Litestream owns checkpointing. `db.go` documents a 3-tier strategy: PASSIVE at `MinCheckpointPageN` (~1000 pages, ~4 MiB), PASSIVE at `CheckpointInterval` (1 minute), and TRUNCATE at `TruncatePageN` (~121,359 pages ≈ 500 MiB). The RESTART mode was removed in production after issue #724 (indefinite write blocking).
5. **Compaction monitors.** `Store.Open` spawns a goroutine per non-zero compaction level plus one for the snapshot level (`store.go: monitorCompactionLevel`). Each monitor runs `Compactor.Compact(dstLevel)` on its level's interval boundary.
6. **Retention enforcer.** `monitorL0Retention` periodically deletes L0 files older than `L0Retention` (default 5 minutes) once they have been promoted to L1.
7. **Heartbeat / validation monitors.** Optional periodic health pings and integrity checks.

The legacy v0.3.x design used a "shadow WAL" — a directory of cloned WAL files (`00000000.wal`, `00000001.wal`, …) — that Litestream maintained next to the main DB. v0.5.x replaced that with direct LTX emission: an LTX file *is* the on-disk and on-S3 unit. The shadow WAL still exists in the v3.go compatibility layer for restoring from old backups but is not produced for new writes. Citation: `litestream/v3.go` defines `GenerationsDirV3 = "generations"`, `SnapshotsDirV3 = "snapshots"`, `WALDirV3 = "wal"` and a `ReplicaClientV3` interface for reading legacy data only.

## LTX file format

The format is identified by `Magic = "LTX1"`. The current version is `Version = 3` (`ltx.go: const Version = 3`). All multi-byte integers are big-endian.

### High-level layout

```
+------------------+   100 bytes
| Header           |
+------------------+
| Page block       |   N * (PageHeader + size-prefix + LZ4-block-data)
| ...              |
| Empty PageHeader |   6 bytes (zero pgno = end marker)
+------------------+
| Page index       |   varint stream + 8-byte size suffix
+------------------+
| Trailer          |   16 bytes
+------------------+
```

Constants from `ltx.go`:

- `HeaderSize = 100`
- `PageHeaderSize = 6`
- `TrailerSize = 16`
- `ChecksumSize = 8`
- `TrailerChecksumOffset = TrailerSize - ChecksumSize` (= 8)
- `ChecksumFlag Checksum = 1 << 63` (top bit forced on so a checksum is never literally zero)

### Header (100 bytes, big-endian)

From `ltx.go: Header.MarshalBinary` and the `superfly/ltx` README:

| Offset | Size | Field                | Notes                                                      |
| ------ | ---- | -------------------- | ---------------------------------------------------------- |
| 0      | 4    | Magic                | ASCII `"LTX1"`                                             |
| 4      | 4    | Flags                | `HeaderFlagNoChecksum = 1 << 1` is the only defined flag   |
| 8      | 4    | PageSize             | 512–65536, must be a power of 2                            |
| 12     | 4    | Commit               | DB size in pages after this transaction                    |
| 16     | 8    | MinTXID              | low TXID covered by this file                              |
| 24     | 8    | MaxTXID              | high TXID covered by this file                             |
| 32     | 8    | Timestamp            | milliseconds since unix epoch                              |
| 40     | 8    | PreApplyChecksum     | rolling CRC-ISO-64 of DB *before* applying this file       |
| 48     | 8    | WALOffset            | original WAL byte offset; 0 if from journal/compaction     |
| 56     | 8    | WALSize              | original WAL byte size                                     |
| 64     | 4    | WALSalt1             | WAL salt-1 (0 if compaction)                               |
| 68     | 4    | WALSalt2             | WAL salt-2                                                 |
| 72     | 8    | NodeID               | originating node id; 0 if unset                            |
| 80     | 20   | (reserved tail)      | header is 100 bytes; remaining bytes are zero              |

(Note: the public README's offset table is the *v1/v2* shape and lists `Reserved 48` after a `Database ID` field at offset 16. The actual code in `ltx.go: Header.MarshalBinary` uses the V3 layout shown above — `MinTXID` starts at offset 16, no separate `Database ID` field, and the LiteFS-style WAL offset/salt/node id fields occupy bytes 48–80.)

The header is included in the file checksum.

#### Header validation rules

`Header.Validate` enforces (`ltx.go`):

- `Version == 3`.
- `PageSize` is a valid SQLite page size (`IsValidPageSize`).
- `MinTXID > 0`, `MaxTXID > 0`, `MinTXID <= MaxTXID`.
- `WALOffset >= 0`, `WALSize >= 0`; salt requires offset; size requires offset.
- **Snapshot rule**: if `MinTXID == 1` (`IsSnapshot`), `PreApplyChecksum` must be 0; the file must include all DB pages.
- **Non-snapshot rule**: if checksums are tracked, `PreApplyChecksum != 0` and the high bit (`ChecksumFlag`) must be set.

### Page block

For each page:

```
+----------------------+
| PageHeader (6 B)     |   pgno:u32 BE, flags:u16 BE
+----------------------+
| Compressed-size (4 B)|   only when flags & PageHeaderFlagSize (= 1<<0)
+----------------------+
| LZ4-block data       |   length = compressed size
+----------------------+
```

Termination: a zero `PageHeader` (pgno = 0) marks end-of-page-block.

`PageHeaderFlagSize = 1 << 0`. From `encoder.go`: every newly written page sets this flag and uses **LZ4 block** compression (`lz4.Compressor.CompressBlock`). The legacy "old format" (no size prefix; LZ4 *frame* per page) is still readable by the decoder for backward compatibility but is never produced. The decoder picks based on the per-page `Flags & PageHeaderFlagSize`.

Page-ordering rules (encoder):

- Snapshot files: pages must be sequential starting at 1, skipping the lock page (`LockPgno(pageSize) = PENDING_BYTE / pageSize + 1`, `PENDING_BYTE = 0x40000000`).
- Non-snapshot files: pages must be strictly increasing by pgno (gaps allowed).

### Page index (between page block and trailer)

After the empty PageHeader sentinel, the encoder writes the page index (`encoder.go: encodePageIndex`):

```
loop over pgnos in ascending order:
    varint(pgno)        // uvarint, 1–10 bytes
    varint(offset)      // byte offset of the page frame from start of file
    varint(size)        // size of the compressed page block including header
varint(0)               // end marker
uint64-BE(index_size)   // size in bytes of the varint stream including the 0
```

Then the 16-byte trailer follows. Restore reads use this index for random-access page fetch: `replica_client.go: FetchPageIndex` reads the last `DefaultEstimatedPageIndexSize = 32 * 1024` bytes of the file with a single S3 ranged GET; if the index is bigger it issues a second GET. The 8-byte size suffix at `len-TrailerSize-8` is what tells the reader where the index begins. `decoder.go: DecodePageIndex` consumes the varint stream.

### Trailer (16 bytes)

| Offset | Size | Field             |
| ------ | ---- | ----------------- |
| 0      | 8    | PostApplyChecksum |
| 8      | 8    | FileChecksum      |

- `PostApplyChecksum`: rolling CRC-ISO-64 of the database state *after* this LTX file is applied. With `HeaderFlagNoChecksum` set this is 0; otherwise it has `ChecksumFlag` (bit 63) forced on.
- `FileChecksum`: CRC-ISO-64 over header + page block + page index + the first 8 bytes of the trailer (`encoder.go: enc.writeToHash(b1[:TrailerChecksumOffset])` then writes `ChecksumFlag | hash.Sum64()` into the file-checksum slot).

### Compression

LZ4 **block** format, one block per page (`encoder.go`: `lz4.Compressor.CompressBlock`, `compressBuf = lz4.CompressBlockBound(pageSize)`). No frame footer. The 4-byte BE compressed size precedes each block. There is no whole-file compression.

### Concrete byte-offset cheat sheet

For a non-snapshot LTX file with one 4096-byte page that compresses to `C` bytes:

```
0x00  4   "LTX1"
0x04  4   flags
0x08  4   page_size = 4096
0x0C  4   commit
0x10  8   minTXID
0x18  8   maxTXID
0x20  8   timestamp_ms
0x28  8   preApplyChecksum
0x30  8   walOffset
0x38  8   walSize
0x40  4   walSalt1
0x44  4   walSalt2
0x48  8   nodeID
0x50  20  (zero pad to 100)

0x64  4   pgno (BE)
0x68  2   flags = 0x0001
0x6A  4   compressedSize = C
0x6E  C   lz4 block

0x6E+C  4   pgno = 0    (PageHeader BE u32)
0x72+C  2   flags = 0   (PageHeader BE u16)
            -> empty PageHeader = end of page block

(page index varints)
... index_size_u64_BE ...
postApplyChecksum (8)
fileChecksum (8)
```

## Generations

In v0.3.x a **generation** was a 16-character random hex string created on first replication and any time WAL continuity broke. Each generation owned its own snapshot directory and its own monotonic 8-character hex WAL index. Restore had to pick a generation, find its newest snapshot, then replay WAL segments. `litestream/v3.go` still contains the parser (`generationRegexV3 = ^[0-9a-f]{16}$`) and `ParseSnapshotFilenameV3` for `{index:08x}.snapshot.lz4` / `{index:08x}_{offset:08x}.wal.lz4` so v0.5.x can still restore old backups.

In **v0.5.x generations are gone**. Quoting the v0.5.0 announcement: "Upon detecting a break in WAL file continuity, the system re-snapshots with the next LTX file instead, establishing monotonically incrementing transaction IDs." The replication position is now `Pos{TXID, PostApplyChecksum}` (`ltx.go: Pos`) — a single global TXID space per database. Continuity breaks insert a fresh snapshot LTX file rather than starting a parallel namespace.

This is the single most important Rivet-relevant simplification: PITR collapses from "find the right generation, then the right WAL index, then the right offset" to "find the LTX file whose `[MinTXID, MaxTXID]` covers the target TXID."

## Compaction levels

Defined in `litestream/compaction_level.go`. The default ladder is:

```go
DefaultCompactionLevels = CompactionLevels{
    {Level: 0, Interval: 0},
    {Level: 1, Interval: 30 * time.Second},
    {Level: 2, Interval: 5 * time.Minute},
    {Level: 3, Interval: time.Hour},
}
const SnapshotLevel = 9
```

- **L0** — raw per-sync output. One LTX file per call to `Replica.Sync` (so by default one per second of writes). Files are named `<minTXID>-<maxTXID>.ltx`; for L0 these are typically single-tx files (`<txid>-<txid>.ltx`). L0 has no compaction interval; instead it is *retained* for `DefaultL0Retention = 5 * time.Minute` and pruned by `monitorL0Retention` every `DefaultL0RetentionCheckInterval = 15 * time.Second`. The 15s value is documented in `store.go` as deliberately shorter than the L1 cadence so VFS read replicas can observe new files before they vanish.
- **L1** — compacts L0 files on aligned 30-second windows. `CompactionLevel.PrevCompactionAt(now) = now.Truncate(Interval)` so L1 boundaries land at `:00, :30, :00, :30…` UTC. The compactor reads every L0 file with `MinTXID > prevL1.MaxTXID` and merges them into one L1 LTX file via `ltx.NewCompactor` (`compactor.go`). The compactor preserves min/max TXID and the post-apply checksum from the last input.
- **L2** — compacts L1 on 5-minute aligned boundaries.
- **L3** — compacts L2 on hourly boundaries.
- **Snapshot level (level 9)** — `SnapshotLevel = 9`. Full-DB snapshot LTX files (`MinTXID == 1`). Generated by the snapshot monitor on the configured snapshot interval (default 24h). Snapshots are how retention finally cuts the long tail of older LTX files: anything older than the oldest retained snapshot can be dropped.

The level number is a hard tier index, not an LSM-style level-size heuristic. There is no size-based promotion. `Compactor.Compact(dstLevel)` always pulls from `dstLevel - 1`. There is no "compact L0 → L2" shortcut; promotions are strictly one level at a time.

`Compactor.HeaderFlags = ltx.HeaderFlagNoChecksum` is set when compacting (see `litestream/compactor.go`), so compacted files carry no rolling DB checksum (only the file checksum). Snapshots, in contrast, retain checksums.

### Retention defaults

From `store.go`:

```
DefaultSnapshotInterval         = 24 * time.Hour
DefaultSnapshotRetention        = 24 * time.Hour
DefaultRetention                = 24 * time.Hour
DefaultRetentionCheckInterval   = 1  * time.Hour
DefaultL0Retention              = 5  * time.Minute
DefaultL0RetentionCheckInterval = 15 * time.Second
```

Retention is enforced by:

- `Compactor.EnforceSnapshotRetention(retention)` — deletes snapshot-level files older than `now - retention` but always keeps the newest snapshot. Returns `minSnapshotTXID`.
- `Compactor.EnforceRetentionByTXID(level, txID)` — for non-snapshot levels, drops files with `MaxTXID < txID`. Cascaded from the snapshot floor.
- `RetentionEnabled` flag — if false, Litestream skips remote deletes and lets the cloud lifecycle policy handle it; local files are still cleaned up.

`VerifyCompaction` flag (off by default) re-iterates the destination level after each compaction and asserts `prev.MaxTXID + 1 == next.MinTXID` (`Compactor.VerifyLevelConsistency`). Non-contiguous TXID ranges = gap or overlap and increment a Prometheus error counter.

## Point-in-time recovery

### Granularity

The advertised PITR precision is the L0 sync cadence, which defaults to **1 second** (`replica.go: DefaultSyncInterval`). Internally the unit is a TXID, and `-txid` lets the operator restore at single-transaction granularity, but only TXIDs that actually correspond to an L0 boundary are addressable: the smallest unit Litestream uploads is one LTX file, and each file has a `[MinTXID, MaxTXID]` range. Restoring "between" the min and max of a multi-TX file is not supported — it applies whole files.

In practice, with the default config, the granularity is:

- ≤1s for the most recent ~5 minutes (L0 still present).
- 30s for the most recent ~hour (L1).
- 5m for the recent day (L2).
- 1h beyond that (L3).
- After 24h the snapshot is the floor.

### Retention

`DefaultRetention = 24 * time.Hour`. Configurable via the `retention` block. Litestream guarantees at least one snapshot survives retention enforcement (`EnforceSnapshotRetention` strips the newest from the `deleted` list).

### Restore mechanics

`Replica.Restore` (`replica.go`) and `cmd/litestream/restore.go`:

1. **Pick a target TXID.** `CalcRestoreTarget` consults `TimeBounds` (LTX) and optionally `TimeBoundsV3` (legacy). If `-timestamp` is set, the target is the LTX file whose `Timestamp` ≤ requested time. If `-txid` is set, that is the target directly. If both are unset, target is the latest available `MaxTXID`.
2. **Find the snapshot.** Iterate `SnapshotLevel` LTX files; pick the newest snapshot whose `MaxTXID <= target`.
3. **Apply the snapshot.** Decode the snapshot LTX file as a SQLite database (`ltx.Decoder.DecodeDatabaseTo`) writing pages 1..Commit (substituting an empty page for the lock page).
4. **Replay incrementals.** Walk levels from highest down to L0, applying every LTX file in `(snapshot.MaxTXID, target]`. The walker prefers the highest level available for any given TXID range so most of the replay is large compacted files; only the tail is L0. `-parallelism` (default 8) controls concurrent downloads.
5. **Sidecar TXID file.** A `<output>-txid` file records the last applied TXID so follow-mode (`-f`) can resume after a crash. If the sidecar is missing on restart, the user must delete the DB and re-restore.
6. **Optional integrity check.** `-integrity-check {none|quick|full}` runs `PRAGMA quick_check` / `PRAGMA integrity_check` after restore.

### Command surface

`litestream restore [flags] DB_PATH | REPLICA_URL`:

- `-o PATH` — output path (required when restoring from a URL).
- `-timestamp ISO8601` — PITR target time.
- `-txid HEX` — PITR target transaction (16-char hex, inclusive).
- `-f` / `-follow-interval DURATION` — follow mode for read replicas (default poll 1s).
- `-parallelism N` — concurrent LTX downloads (default 8).
- `-if-db-not-exists` — succeed silently if DB already exists.
- `-if-replica-exists` — exit 0 if no backups found.
- `-integrity-check {none,quick,full}`.
- `-config PATH` / `-no-expand-env`.

`-timestamp` and `-txid` are mutually exclusive. Follow mode is incompatible with both.

There is no first-class "fork" or "bookmark" command. Forking has to be done by restoring to a new path with a target TXID and then pointing a fresh replica at it. Bookmarks would be application-level metadata.

## S3 object layout

### Key structure

`litestream/litestream.go`:

```go
func LTXDir(root string) string                            { return path.Join(root, "ltx") }
func LTXLevelDir(root string, level int) string            { return path.Join(LTXDir(root), strconv.Itoa(level)) }
func LTXFilePath(root string, level int, min, max ltx.TXID) string {
    return path.Join(LTXLevelDir(root, level), ltx.FormatFilename(min, max))
}
// ltx.FormatFilename: fmt.Sprintf("%s-%s.ltx", minTXID, maxTXID)  -> "0000000000000001-0000000000000005.ltx"
```

So an S3 replica configured as `s3://my-bucket/db1` produces keys:

```
db1/ltx/0/0000000000000007-0000000000000007.ltx     # L0 single-tx
db1/ltx/0/0000000000000008-0000000000000008.ltx
db1/ltx/1/0000000000000007-0000000000000010.ltx     # L1, 30s window
db1/ltx/2/0000000000000001-0000000000000050.ltx     # L2, 5m window
db1/ltx/3/0000000000000001-0000000000000200.ltx     # L3, 1h window
db1/ltx/9/0000000000000001-000000000000ffff.ltx     # snapshot (MinTXID=1)
```

(In v0.3.x it was instead `db1/generations/<16-hex>/snapshots/<8-hex>.snapshot.lz4` and `db1/generations/<16-hex>/wal/<8-hex>_<8-hex>.wal.lz4`.)

### Object types

The v0.5.x layout has exactly **one** object type: an LTX file. Snapshots vs incrementals are distinguished by the level prefix (`9` = snapshot, `0..3` = incremental tiers) and by `Header.IsSnapshot()` (`MinTXID == 1`). There are no separate index/manifest objects on S3 — the per-file page index lives inside the LTX file itself, fetched via a tail range read.

S3 `HEAD` is used to read the `litestream-timestamp` user metadata key (`MetadataKeyTimestamp = "litestream-timestamp"`) for accurate timestamp-based restore. The fast path uses `LastModified` from `ListObjectsV2`. `DefaultMetadataConcurrency = 50` HEAD requests in parallel during timestamp-based restore (S3 docs say 5500+ HEAD/s per prefix).

### Multipart uploads

`s3/replica_client.go`:

- Default `PartSize = 5 MiB` (S3 minimum).
- Default `Concurrency = 5` parts in flight.
- Cloudflare R2 special-cased to `DefaultR2Concurrency = 2` (R2 has stricter concurrent multipart limits).
- `RequireContentMD5 = true` by default (set per-provider; some compatible providers disable it).
- `MaxKeys = 1000` per bulk delete batch.

L0 files are typically tiny (one transaction's worth of pages), so they tend to be single PUTs. Compacted higher-level files routinely cross the multipart threshold.

### Server-side encryption

Both SSE-C (works with any S3-compatible) and SSE-KMS (AWS-only) are supported in `s3/replica_client.go`. SSE-C uses customer-supplied 256-bit AES key + base64 MD5.

## Position / replication semantics

### Position triple (well, pair)

```go
// ltx.go
type Pos struct {
    TXID              TXID     // uint64, formatted as 16 lowercase hex
    PostApplyChecksum Checksum // uint64, top bit forced (ChecksumFlag)
}
```

`Pos.String() = "<16-hex-txid>/<16-hex-checksum>"` (33 chars including the `/`). `ParsePos` requires that exact length.

This replaced the v0.3.x triple `(generation:16hex, index:8hex, offset:16hex)` formatted as `gen/00000003:0000000000001234`. The single global TXID space means there is no need to scope by generation when comparing positions.

LTX files carry both an "incoming" and "outgoing" position:

- `FileInfo.PreApplyPos() = Pos{MinTXID-1, PreApplyChecksum}`
- `FileInfo.PostApplyPos() = Pos{MaxTXID, PostApplyChecksum}`

### Crash recovery

- **In-process recovery.** `Replica.Sync` clears `r.pos` on any error so the next sync rediscovers it from the remote (`calcPos`). `calcPos` walks the highest-level files first to find the newest contiguous TXID range it can reach.
- **DB-side recovery.** `DB` caches `lastSyncedWALOffset` so a checkpoint that truncates the WAL does not look like rollback (issue #927). `syncedToWALEnd` similarly suppresses spurious "full snapshot needed" decisions after a clean checkpoint.
- **Restore-side recovery.** Follow mode writes a sidecar `<db>-txid` file every time it applies an LTX file. On restart it reads the sidecar; if the earliest available snapshot has `MaxTXID > sidecar.txid`, retention has already pruned past us and the recovery is unrecoverable (operator must delete the DB and full-restore).
- **Auto-recover.** `Replica.AutoRecoverEnabled` (off by default) lets a sync that hits an LTX integrity error reset its local position and re-sync from scratch. Off by default because it can mask data loss.

### Continuity invariants

- Files at any single level must have contiguous TXID ranges: `prev.MaxTXID + 1 == next.MinTXID`. Verified by `Compactor.VerifyLevelConsistency` when `VerifyCompaction` is on. A gap is a hard error and increments a Prometheus counter.
- Across levels, the higher level's files must be supersets of the corresponding lower-level ranges (because they were produced from those L0/L1 inputs). Restore relies on this when picking the highest level available for any given TXID range.
- A new snapshot is required (and re-bootstraps the chain at the new MaxTXID) when WAL continuity breaks, e.g. if a checkpoint races and the WAL salts change.

## Direct quotes / code references

```go
// ltx.go
const (
    Magic           = "LTX1"
    Version         = 3
    HeaderSize      = 100
    PageHeaderSize  = 6
    TrailerSize     = 16
    ChecksumSize    = 8
)
const ChecksumFlag Checksum = 1 << 63
```

```go
// litestream/store.go
const (
    DefaultSnapshotInterval         = 24 * time.Hour
    DefaultSnapshotRetention        = 24 * time.Hour
    DefaultRetention                = 24 * time.Hour
    DefaultRetentionCheckInterval   = 1  * time.Hour
    DefaultL0Retention              = 5  * time.Minute
    DefaultL0RetentionCheckInterval = 15 * time.Second
)
```

```go
// litestream/compaction_level.go
const SnapshotLevel = 9
var DefaultCompactionLevels = CompactionLevels{
    {Level: 0, Interval: 0},
    {Level: 1, Interval: 30 * time.Second},
    {Level: 2, Interval: 5  * time.Minute},
    {Level: 3, Interval: time.Hour},
}
```

```go
// litestream/db.go (defaults)
DefaultMonitorInterval    = 1  * time.Second
DefaultCheckpointInterval = 1  * time.Minute
DefaultBusyTimeout        = 1  * time.Second
DefaultMinCheckpointPageN = 1000              // ~4 MiB at 4 KiB pages
DefaultTruncatePageN      = 121359            // ~500 MiB
```

```go
// litestream/replica.go
DefaultSyncInterval = 1 * time.Second
```

> "Litestream performs this compaction itself. It doesn't rely on SQLite to process the WAL file." — Fly blog, v0.5.0 announcement.

> "L0 retention check interval should be more frequent than the L1 compaction interval so that VFS read replicas have time to observe new files." — `store.go` doc comment.

> "The RESTART checkpoint mode was permanently removed due to production issues with indefinite write blocking (issue #724). All checkpoints now use either PASSIVE (non-blocking) or TRUNCATE (emergency only) modes." — `db.go` doc comment.

> README header table differs from runtime — the README at `superfly/ltx` still documents the v1/v2 header (Database ID at offset 16, 48 bytes reserved). The actual V3 layout written by `Header.MarshalBinary` puts MinTXID at offset 16 and uses the trailing space for WAL offset/size/salts/NodeID. **Use `ltx.go` as the source of truth, not the README.**

## Open questions

- **README vs source drift on header layout.** The `superfly/ltx` README still lists the V1/V2 header. We use `ltx.go`'s V3 marshalling in our internal SHARD/DELTA blobs. Worth confirming we wrote the same offsets we read.
- **No client-side encryption.** Litestream relies on SSE-C / SSE-KMS at the bucket. For Rivet's multi-tenant cold storage we may want envelope encryption per-actor (AES-GCM with a per-actor DEK wrapped by a tenant KEK) before PUT, since SSE-C still gives the bucket operator plaintext access during request signing.
- **No bookmarks/forking primitives.** Litestream's restore writes a fresh DB and a TXID sidecar; there's no concept of a named "checkpoint" or a fork that branches the TXID line. For our 30-day retention + named bookmarks + actor forking we'll need a metadata layer above LTX that maps `(actor_id, bookmark_name) → TXID` and supports diverging TXID streams (probably by giving the fork its own actor_id-prefixed key namespace and a parent-pointer to the source file at the fork TXID).
- **Per-file vs multi-file index.** Each LTX has its own page index; restore reads N tail-ranges (N = number of LTX files in the replay window). For our long retention window we may want a periodic "manifest" object that stitches indices together so PITR doesn't issue thousands of HEAD+GET per restore.
- **Compaction is interval-aligned, not size-aligned.** A quiet 5-minute window produces a tiny L1 file; a busy one produces a big one. For our object-storage costs we may want to combine "every 30s" with a soft "or 8 MiB, whichever is first" trigger.
- **Single replica per DB in v0.5.x.** Multi-replica was dropped. We almost certainly want at least primary + DR, so we'll need our own fan-out at upload time (or run two LTX writers in parallel from the same authoritative L0 stream).
- **Lock page handling on snapshots.** LTX writes a synthetic empty page for SQLite's lock page (`PENDING_BYTE / pageSize + 1`). Worth double-checking that our reader does the same when materializing snapshots into a fresh DB file, otherwise integrity check will fail.
- **CRC-ISO-64.** Cheap and fine for opportunistic detection but not collision-resistant. For long-tail cold storage we likely want SHA-256 over each LTX's logical content as an external manifest column (independent of the file checksum).
- **Heartbeat / leaser.** Litestream has a `HeartbeatClient` and an S3-based leaser (`s3/leaser.go`) to prevent two writers from racing on the same DB. We need an equivalent fence at the actor exclusivity boundary; pegboard-envoy's lost-timeout already gives us most of this, but the S3 lease pattern may still be useful for the snapshot/compaction path which can run from any node.
