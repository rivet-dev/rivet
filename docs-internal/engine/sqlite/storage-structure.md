# SQLite Storage Structure

This is the key-format reference for the branchable Depot layer. Update it whenever FDB or S3 layout changes.

## Identity Model

Depot has two external ids:

- `BucketId`: the bucket branch id. There is no separate bucket pointer id.
- `DatabaseId`: the database branch id. There is no separate database pointer id.

Branch records are append-only for ancestry fields. Database branch records also carry mutable lifecycle state and a monotonic lifecycle generation used to reject stale workflow compaction work. Forks allocate a new id and write a parent pointer to the source branch plus the fork versionstamp. Engine-layer rollback is implemented outside this crate by forking a database and changing the engine's database-to-database mapping.

Bucket database membership is stored in `BUCKET_CATALOG`. Bucket forks do not copy catalog entries. Reads walk bucket parents and accept inherited entries only when the entry versionstamp is at or before the walking branch's `parent_versionstamp`.

## FDB Prefixes

All Depot keys live under the crate-owned `[0x02]` prefix. The next byte is the partition.

| Partition | Prefix | Owner | Purpose |
|---|---|---|---|
| `DBPTR` | `[0x02][0x10]` | database pointer APIs | Current and historical database pointer rows by bucket branch and database name. |
| `BUCKET_PTR` | `[0x02][0x11]` | bucket pointer APIs | Current bucket branch pointer rows. |
| `BUCKET_CATALOG` | `[0x02][0x12]` | create/fork/delete database paths | Bucket catalog membership markers. |
| `BRANCHES` | `[0x02][0x20]` | branch operations, GC, workflow compaction | Database branch records plus mutable counters and lifecycle state. |
| `BUCKET_BRANCH` | `[0x02][0x21]` | bucket operations, GC | Immutable bucket branch records plus mutable counters and tombstones. |
| `BR` | `[0x02][0x30]` | conveyer and workflow compaction | Per-database hot data, metadata, workflow state, and staged output. |
| `CTR` | `[0x02][0x40]` | quota | Global counters. |
| `RESTORE_POINT` | `[0x02][0x50]` | restore point APIs | RestorePoint records and restore point state. |
| `DB_PIN` | `[0x02][0x70]` | workflow compaction | Unified database history pins. |
| `BUCKET_FORK_PIN` | `[0x02][0x71]` | bucket fork, workflow compaction | Unresolved bucket fork retention facts. |
| `BUCKET_CHILD` | `[0x02][0x72]` | bucket fork, workflow compaction | Bucket child edges for bounded proof walks. |
| `BUCKET_CATALOG_BY_DB` | `[0x02][0x73]` | bucket catalog, workflow compaction | Reverse bucket membership index by database branch. |
| `BUCKET_PROOF_EPOCH` | `[0x02][0x74]` | bucket fork/catalog mutation | Bucket-tree proof invalidation epoch. |
| `SQLITE_CMP_DIRTY` | `[0x02][0x75]` | conveyer, workflow compaction | Coalesced compaction wake marker by database branch. |

## Pointers And Bucket Catalog

```text
DBPTR/{bucket_branch_id_uuid_be:16}/{database_name}/cur
  -> DatabaseBranchId (vbare-versioned)
DBPTR/{bucket_branch_id_uuid_be:16}/{database_name}/history/{ts_ms_be:8}{nonce_be:4}
  -> DatabaseBranchId (vbare-versioned)

BUCKET_PTR/{bucket_id_uuid_be:16}/cur
  -> BucketBranchId (vbare-versioned)
```

Database pointer resolution walks bucket parents when a current bucket branch does not contain a local database pointer. Pointer history supports restore/rollback bookkeeping without changing the branch-local page layout.

```text
BUCKET_CATALOG/{bucket_id_uuid_be:16}/{database_id_uuid_be:16}
  -> 16-byte FDB versionstamp via SetVersionstampedValue
```

The value is the database membership versionstamp. Parent walks use it as the AS-OF cap for `fork_bucket`. Database tombstones on the bucket branch hide matching inherited catalog entries.

## Branch Records And Pins

```text
BRANCHES/list/{database_id_uuid_be:16}
  -> DatabaseBranchRecord (vbare-versioned)
BRANCHES/list/{database_id_uuid_be:16}/refcount
  -> i64 LE atomic-add
BRANCHES/list/{database_id_uuid_be:16}/desc_pin
  -> 16-byte versionstamp atomic-min
BRANCHES/list/{database_id_uuid_be:16}/restore_point_pin
  -> 16-byte versionstamp atomic-min

BUCKET_BRANCH/list/{bucket_id_uuid_be:16}
  -> BucketBranchRecord (vbare-versioned)
BUCKET_BRANCH/list/{bucket_id_uuid_be:16}/refcount
  -> i64 LE atomic-add
BUCKET_BRANCH/list/{bucket_id_uuid_be:16}/desc_pin
  -> 16-byte versionstamp atomic-min
BUCKET_BRANCH/list/{bucket_id_uuid_be:16}/restore_point_pin
  -> 16-byte versionstamp atomic-min
BUCKET_BRANCH/list/{bucket_id_uuid_be:16}/database_tombstones/{database_id_uuid_be:16}
  -> empty
```

Pins are raw fixed-width bytes, not vbare records. GC computes the branch pin as the minimum of the live refcount root floor, descendant pin, and restore point pin.

## Per-Database Hot Data

```text
BR/{database_id_be:16}/META/head
  -> DBHead (vbare-versioned, commit-owned)
BR/{database_id_be:16}/META/head_at_fork
  -> DBHead (vbare-versioned, fork snapshot until first local commit)
BR/{database_id_be:16}/META/quota
  -> i64 LE atomic
BR/{database_id_be:16}/COMMITS/{txid_be:8}
  -> CommitRow (vbare-versioned)
BR/{database_id_be:16}/VTX/{versionstamp_be:16}
  -> u64 BE txid
BR/{database_id_be:16}/PIDX/{pgno_be:4}
  -> u64 BE owner_txid
BR/{database_id_be:16}/DELTA/{txid_be:8}/{chunk_be:4}
  -> LTX chunk blob
BR/{database_id_be:16}/STAGE/commit/{stage_id:16}/meta
  -> CommitStageMeta (vbare-versioned)
BR/{database_id_be:16}/STAGE/commit/{stage_id:16}/pages/{batch_be:4}
  -> DirtyPageBatch (vbare-versioned)
BR/{database_id_be:16}/STAGE/commit/{stage_id:16}/complete
  -> CommitStageComplete (vbare-versioned)
BR/{database_id_be:16}/STAGE/commit/{stage_id:16}/finalized
  -> CommitStageFinalized (vbare-versioned)
BR/{database_id_be:16}/DELTA_OBJ/{object_id:16}/chunk/{chunk_be:4}
  -> Large LTX object chunk blob
BR/{database_id_be:16}/DELTA_OBJ/{object_id:16}/meta
  -> DeltaObjectMeta (vbare-versioned)
BR/{database_id_be:16}/DELTA_OBJ_REF/{object_id:16}
  -> u64 BE committed_txid
BR/{database_id_be:16}/DELTA_MANIFEST/{txid_be:8}
  -> DeltaManifest (vbare-versioned)
BR/{database_id_be:16}/DELTA_PAGEIDX/{txid_be:8}/{pgno_be:4}
  -> DeltaPageIndexEntry (vbare-versioned)
BR/{database_id_be:16}/SHARD/{shard_id_be:4}/{as_of_txid_be:8}
  -> LTX shard blob
BR/{database_id_be:16}/PITR_INTERVAL/{bucket_start_ms_be:8}
  -> PitrIntervalCoverage (vbare-versioned)
```

`COMMITS` stores commit metadata, including wall-clock time, captured versionstamp, size in pages, and post-apply checksum. `VTX` maps a versionstamp back to txid for restore point resolution and GC. `PIDX` maps a page number to the delta txid that currently owns it. Small commits store that delta as legacy `DELTA/{txid}/{chunk}` rows. Large commits store immutable bytes in `DELTA_OBJ`, publish `DELTA_MANIFEST`, and use `DELTA_PAGEIDX` so sparse reads can fetch only the chunks covering one page frame.

`SHARD` is versioned by `as_of_txid`. Reads choose the largest `as_of_txid <= read_txid`. Hot compaction writes new SHARD versions and does not overwrite older ones. When cold storage is enabled, the same FDB SHARD rows are also the shard cache for manager-published `CMP/cold_shard` refs; read-through fill can repopulate an evicted row without advancing manifest watermarks.

## Workflow Compaction Metadata

```text
BR/{database_id_be:16}/CMP/root
  -> CompactionRoot (vbare-versioned)
BR/{database_id_be:16}/CMP/cold_shard/{shard_id_be:4}/{as_of_txid_be:8}
  -> ColdShardRef (vbare-versioned)
BR/{database_id_be:16}/CMP/retired_cold_object/{object_key_hash:32}
  -> RetiredColdObject (vbare-versioned)
BR/{database_id_be:16}/CMP/stage/{job_id}/hot_shard/{shard_id_be:4}/{as_of_txid_be:8}/{chunk_be:4}
  -> staged LTX shard blob
```

The DB manager owns published `CMP` metadata. Staged hot shard keys are not reader-visible until the manager validates the active job and copies them to `SHARD`; the same install transaction advances `CMP/root` and compare-and-clears matching PIDX rows.

`CMP/cold_shard` refs point to workflow cold objects and are the durable proof that an evictable FDB `SHARD` has cold coverage. `CMP/retired_cold_object` records fence the S3 grace-delete lifecycle so an object key is not republished after retirement.

## Branch Manifest Subkeys

`BranchManifest` is exposed as one logical struct but stored as owner-specific keys to avoid read-modify-write conflicts.

```text
BR/{database_id}/META/manifest/last_access_ts_ms
  -> i64 LE, database access touch
BR/{database_id}/META/manifest/last_access_bucket
  -> i64 LE, database access touch
```

Workflow compaction publishes live manifest state under `BR/{database_id}/CMP/root` and related `CMP/*` keys. Access-touch manifest subkeys are cache-policy inputs only; they are not deletion authority by themselves.

## Global Keys

```text
CTR/quota_global
  -> i64 LE
RESTORE_POINT/{database_id}/{restore_point_str}
  -> RestorePointRecord (vbare-versioned)

DB_PIN/{database_id_be:16}/{pin_id}
  -> DbHistoryPin (vbare-versioned)
BUCKET_FORK_PIN/{source_bucket_id_be:16}/{fork_versionstamp_be:16}/{target_bucket_id_be:16}
  -> BucketForkFact (vbare-versioned)
BUCKET_CHILD/{source_bucket_id_be:16}/{fork_versionstamp_be:16}/{target_bucket_id_be:16}
  -> BucketForkFact (vbare-versioned)
BUCKET_CATALOG_BY_DB/{database_id_be:16}/{bucket_id_be:16}
  -> BucketCatalogDbFact (vbare-versioned)
BUCKET_PROOF_EPOCH/{root_bucket_id_be:16}
  -> proof invalidation epoch (atomic i64 LE)
SQLITE_CMP_DIRTY/{database_id_be:16}
  -> SqliteCmpDirty (vbare-versioned)
```

## S3 Layout

All objects are under a deployment-configured root. Workflow cold objects are immutable LTX shard objects. The object generation id is the cold job id, so retrying a job reuses the same object key.

```text
{root}/
  db/{database_id_uuid_hex:32}/
    shard/{shard_id_be_hex:8}/
      {as_of_txid_be_hex:16}-{object_generation_id}-{content_hash_hex}.ltx
```

Each object contains exactly one shard. The content hash is also stored in the matching `ColdShardRef`, and the manager/reclaimer compare that identity before publishing, evicting, retiring, or deleting cold-backed data.
