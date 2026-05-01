# SQLite Storage Structure

This is the key-format reference for the branchable Depot layer. Update it whenever FDB or S3 layout changes.

## Identity Model

Depot has two external ids:

- `NamespaceId`: the namespace branch id. There is no separate namespace pointer id.
- `DatabaseId`: the database branch id. There is no separate database pointer id.

Both branch records are immutable once written. Forks allocate a new id and write a parent pointer to the source branch plus the fork versionstamp. Engine-layer rollback is implemented outside this crate by forking a database and changing the engine's database-to-database mapping.

Namespace database membership is stored in `NSCAT`. Namespace forks do not copy catalog entries. Reads walk namespace parents and accept inherited entries only when the entry versionstamp is at or before the walking branch's `parent_versionstamp`.

## FDB Prefixes

All Depot keys live under the crate-owned `[0x02]` prefix. The next byte is the partition.

| Partition | Prefix | Owner | Purpose |
|---|---|---|---|
| `NSCAT` | `[0x02][0x10]` | create/fork/delete database paths | Namespace catalog membership markers. |
| `BRANCHES` | `[0x02][0x20]` | branch operations, GC | Immutable database branch records plus mutable counters. |
| `NSBRANCH` | `[0x02][0x21]` | namespace operations, GC | Immutable namespace branch records plus mutable counters and tombstones. |
| `BR` | `[0x02][0x30]` | conveyer and compactors | Per-database hot data, metadata, and leases. |
| `CTR` | `[0x02][0x40]` | quota, eviction access touch | Global counters and eviction index. |
| `BOOKMARK` | `[0x02][0x50]` | bookmark APIs, cold compactor | Bookmark records and pinned bookmark state. |
| `CMPC` | `[0x02][0x60]` | compactor dispatch | Cold/eviction work queue and global leases. |

## Namespace Catalog

```text
NSCAT/{namespace_id_uuid_be:16}/{database_id_uuid_be:16}
  -> 16-byte FDB versionstamp via SetVersionstampedValue
```

The value is the database membership versionstamp. Parent walks use it as the AS-OF cap for `fork_namespace`. Database tombstones on the namespace branch hide matching inherited catalog entries.

## Branch Records And Pins

```text
BRANCHES/list/{database_id_uuid_be:16}
  -> DatabaseBranchRecord (vbare-versioned)
BRANCHES/list/{database_id_uuid_be:16}/refcount
  -> i64 LE atomic-add
BRANCHES/list/{database_id_uuid_be:16}/desc_pin
  -> 16-byte versionstamp atomic-min
BRANCHES/list/{database_id_uuid_be:16}/bk_pin
  -> 16-byte versionstamp atomic-min

NSBRANCH/list/{namespace_id_uuid_be:16}
  -> NamespaceBranchRecord (vbare-versioned)
NSBRANCH/list/{namespace_id_uuid_be:16}/refcount
  -> i64 LE atomic-add
NSBRANCH/list/{namespace_id_uuid_be:16}/desc_pin
  -> 16-byte versionstamp atomic-min
NSBRANCH/list/{namespace_id_uuid_be:16}/bk_pin
  -> 16-byte versionstamp atomic-min
NSBRANCH/list/{namespace_id_uuid_be:16}/database_tombstones/{database_id_uuid_be:16}
  -> empty
```

Pins are raw fixed-width bytes, not vbare records. GC computes the branch pin as the minimum of the live refcount root floor, descendant pin, and bookmark pin.

## Per-Database Hot Data

```text
BR/{database_id_be:16}/META/head
  -> DBHead (vbare-versioned, commit-owned)
BR/{database_id_be:16}/META/head_at_fork
  -> DBHead (vbare-versioned, fork snapshot until first local commit)
BR/{database_id_be:16}/META/compact
  -> CompactState (vbare-versioned, hot-compactor-owned)
BR/{database_id_be:16}/META/cold_compact
  -> ColdState (vbare-versioned, cold-compactor-owned)
BR/{database_id_be:16}/META/quota
  -> i64 LE atomic
BR/{database_id_be:16}/META/compactor_lease
  -> Lease (vbare-versioned, hot compactor)
BR/{database_id_be:16}/META/cold_lease
  -> Lease (vbare-versioned, cold compactor)
BR/{database_id_be:16}/COMMITS/{txid_be:8}
  -> CommitRow (vbare-versioned)
BR/{database_id_be:16}/VTX/{versionstamp_be:16}
  -> u64 BE txid
BR/{database_id_be:16}/PIDX/{pgno_be:4}
  -> u64 BE owner_txid
BR/{database_id_be:16}/DELTA/{txid_be:8}/{chunk_be:4}
  -> LTX chunk blob
BR/{database_id_be:16}/SHARD/{shard_id_be:4}/{as_of_txid_be:8}
  -> LTX shard blob
```

`COMMITS` stores commit metadata, including wall-clock time, captured versionstamp, size in pages, and post-apply checksum. `VTX` maps a versionstamp back to txid for bookmark resolution and GC. `PIDX` maps a page number to the DELTA txid that currently owns it.

`SHARD` is versioned by `as_of_txid`. Reads choose the largest `as_of_txid <= read_txid`. Hot compaction writes new SHARD versions and does not overwrite older ones.

## Branch Manifest Subkeys

`BranchManifest` is exposed as one logical struct but stored as owner-specific keys to avoid read-modify-write conflicts.

```text
BR/{database_id}/META/manifest/cold_drained_txid
  -> u64 BE, cold-compactor-owned
BR/{database_id}/META/manifest/last_hot_pass_txid
  -> u64 BE, hot-compactor-owned
BR/{database_id}/META/manifest/last_access_ts_ms
  -> i64 LE, database access touch
BR/{database_id}/META/manifest/last_access_bucket
  -> i64 LE, database access touch
```

Eviction regular-reads `last_hot_pass_txid` as its hot-compactor OCC fence. Cold compaction advances `cold_drained_txid` after uploaded layers are durable.

## Global Keys

```text
CTR/quota_global
  -> i64 LE
CTR/eviction_index/{last_access_bucket_be:8}/{database_id}
  -> empty

BOOKMARK/{database_id}/{bookmark_str}
  -> BookmarkRecord (vbare-versioned)
BOOKMARK/{database_id}/{bookmark_str}/pinned
  -> PinnedBookmarkRecord (vbare-versioned)

CMPC/enqueue/{ts_ms_be:8}/{database_id}/{kind:1}
  -> empty
CMPC/lease_global/{kind:1}
  -> Lease
```

`kind` is `0x00` for cold compaction and `0x01` for eviction. The eviction index bucket is `floor(last_access_ts_ms / ACCESS_TOUCH_THROTTLE_MS)` and is re-keyed only when the bucket advances.

## S3 Layout

All objects are under a deployment-configured root. Persisted S3 records carry `schema_version: u32` in their vbare payloads.

```text
{root}/
  ns/{namespace_id_uuid_hex:32}/
    branch_record.bare
    catalog/
      {ns_versionstamp_hex:32}.bare
  db/{database_id_uuid_hex:32}/
    branch_record.bare
    image/{as_of_txid_high_bytes_hex:8}/
      {shard_id_be_hex:8}-{as_of_txid_be_hex:16}.ltx
    pin/
      {versionstamp_hex:32}.ltx
    delta/
      {min_txid_be_hex:16}-{max_txid_be_hex:16}.ltx
    cold_manifest/
      index.bare
      chunks/
        {pass_versionstamp_hex:32}.bare
  catalog_snapshot/
    {pass_versionstamp_hex:32}.bare
  pending/
    {uuid}.marker
```

Image files contain exactly one shard. Filenames omit checksums so retries overwrite the same object key; integrity is carried by LTX trailers and manifest chunk entries.

`cold_manifest/index.bare` is the small mutable index. It points at immutable chunk files, each containing the layer entries and bookmark index entries written by one cold pass. Readers fetch the index and only the chunks needed for the requested versionstamp.

`catalog_snapshot` is the disaster-recovery catalog: observed namespace branch records, database branch records, and NSCAT membership entries at a cold-pass versionstamp.
