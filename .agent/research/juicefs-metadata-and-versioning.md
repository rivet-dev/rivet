# JuiceFS Metadata Schema and Versioning Research

## Overview

JuiceFS does NOT support file versioning or filesystem-level snapshots. It offers clone (metadata-only copy-on-write fork) and a trash/recycle bin for deleted files. This document covers the exact internal metadata schema and how it compares to our planned VFS design.

## Metadata Schema

JuiceFS supports three metadata backends (Redis, SQL via xorm, TKV) with the same logical schema. Below is the SQL representation.

### Core Tables

**`node`** (inode table, 19 fields):
```
Inode         uint64   PK
Type          uint8    -- 1=file, 2=dir, 3=symlink, 4=FIFO, 5=blockdev, 6=chardev, 7=socket
Flags         uint8    -- FlagImmutable, FlagAppend, FlagSkipTrash
Mode          uint16   -- Unix permission bits
Uid           uint32
Gid           uint32
Atime         int64    -- microseconds
Mtime         int64    -- microseconds
Ctime         int64    -- microseconds
Atimensec     int16    -- sub-microsecond nanosecond remainder
Mtimensec     int16
Ctimensec     int16
Nlink         uint32
Length        uint64   -- file size in bytes
Rdev          uint32   -- device number (for device nodes)
Parent        Ino      -- parent inode (0 for hardlinked files)
AccessACLId   uint32   -- FK to acl table
DefaultACLId  uint32   -- FK to acl table (directories only)
Tier          uint8    -- storage tier ID
```

**`edge`** (directory entries):
```
Id      int64   PK (bigserial)
Parent  Ino     UNIQUE(edge)
Name    []byte  UNIQUE(edge), varbinary(255)
Inode   Ino     INDEX
Type    uint8
```

**`chunk`** (file chunk-to-slices mapping):
```
Id      int64   PK (bigserial)
Inode   Ino     UNIQUE(chunk)
Indx    uint32  UNIQUE(chunk)   -- chunk index (file offset / 64MB)
Slices  []byte  blob            -- packed array of 24-byte slice records
```

**`sliceRef`** (table name: `chunk_ref`, reference counting):
```
Id     uint64  PK  (chunkid / slice id)
Size   uint32
Refs   int     INDEX
```

**`symlink`**:
```
Inode   Ino     PK
Target  []byte  varbinary(4096)
```

### The 24-Byte Slice Record

Each slice within a chunk's `Slices` blob is packed as:
```
pos   uint32  -- offset within the chunk (0 to 64MB)
id    uint64  -- globally unique slice ID
size  uint32  -- total size of the object in object storage
off   uint32  -- offset within that object where this slice's data starts
len   uint32  -- length of data this slice covers
```

Slices are appended in write order. Newer slices override older ones at the same byte positions.

### Supporting Tables

- **`xattr`**: Extended attributes (inode, name, value)
- **`acl`**: POSIX ACL rules (owner, group, mask, other, named users/groups)
- **`flock`**: BSD-style file locks
- **`plock`**: POSIX range locks
- **`session2`**: Client sessions (sid, expire, info JSON)
- **`sustained`**: Open file handles preventing deletion (sid, inode)
- **`delfile`**: Files pending deletion (unlinked but still open)
- **`delslices`**: Delayed slice deletion queue (for trash)
- **`dirStats`**: Per-directory usage statistics
- **`dirQuota`**: Per-directory quotas
- **`setting`**: Key-value config (volume format JSON)
- **`counter`**: Named counters (nextInode, nextChunk, usedSpace, totalInodes)

### Redis Key Schema

```
i{inode}                -> binary Attr
d{inode}                -> hash { name -> packed(inode, type) }
p{inode}                -> hash { parent_ino -> count }
c{inode}_{indx}         -> list of 24-byte packed Slice records
s{inode}                -> target string
x{inode}                -> hash { name -> value }
lockf{inode}            -> hash { {sid}_{owner} -> ltype }
lockp{inode}            -> hash { {sid}_{owner} -> packed Plock }
sessions                -> sorted set { sid -> heartbeat }
session{sid}            -> set [ inode ]
delfiles                -> sorted set { {inode}:{length} -> seconds }
sliceRef                -> hash { k{sliceId}_{size} -> refcount }
```

## Slice Lifecycle

### Write Path

1. Allocate new slice ID from `nextChunk` counter.
2. Write data to object storage keyed by slice ID.
3. Append 24-byte slice record to the chunk's `Slices` blob.
4. Create `sliceRef` entry with `refs=1`.
5. Update inode `Length`, `Mtime`, `Ctime`.

### Read Path (Resolving Overlaps)

`buildSlice()` uses an interval tree approach:
1. Process slices in write order (oldest first).
2. Each new slice cuts/splits any existing slices that overlap.
3. Later writes always win at any byte position.
4. Final in-order traversal yields non-overlapping resolved slice list.
5. Gaps (regions with `id == 0`) are zeros/holes.

### Compaction

Triggered when a chunk accumulates many slices (every 100th slice, forced at 350+, also on read if 5+ slices).

1. Read all slices for the chunk.
2. Skip leading large contiguous slices (no need to rewrite).
3. Build resolved slice view, trim leading/trailing zeros.
4. Read resolved data, write as a single new object.
5. Atomic compare-and-swap: replace compacted slices with one new slice record.
6. Decrement refs on old slices (or queue to `delslices` if trash enabled).

Constants: `maxCompactSlices = 1000`, `maxSlices = 2500`, `ChunkSize = 64MB`.

## What JuiceFS Has Instead of Versioning

### Trash / Recycle Bin

- Controlled by `TrashDays` setting.
- Deleted files moved to `.trash/` (reserved inode `0x7FFFFFFF10000000`).
- Sub-directories per hour: `.trash/2024-01-15-14/`.
- Entries named `{parent_ino}-{file_ino}-{original_name}`.
- Background job cleans entries older than `TrashDays`.
- Files with `FlagSkipTrash` bypass trash.

### Clone (metadata-only COW fork)

`juicefs clone SRC DST`:
- Creates new inodes for all entries in source tree.
- Copies chunk slice arrays verbatim to new inodes.
- Increments `sliceRef.Refs` for every referenced slice.
- Redirect-on-write: subsequent writes to either copy create new slices; unmodified regions share data blocks.
- Fast regardless of data size (metadata-only operation).
- NOT a reversible snapshot. It is a one-time fork.

## Comparison: JuiceFS vs Our Planned Design

| Aspect | JuiceFS | Our Design |
|--------|---------|------------|
| Versioning | None. Trash + clone only. | Native per-file versioning via `inode_versions` table. |
| Snapshots | No filesystem snapshots. Clone is a one-time fork. | Point-in-time snapshots by recording `{ino -> version}` mappings. Instant, metadata-only. |
| Slice model | Packed 24-byte records in a blob column. Overlap resolution via interval tree. | Similar concept but our "slices" only needed for chunked-mode large files. Small files use inline SQLite or single S3 objects. |
| Metadata engines | Redis, PostgreSQL, MySQL, SQLite, TiKV, etcd | SQLite primary. Interface allows Redis, Postgres, etc. |
| Block store | Any S3-compatible object storage | Same. Plus inline SQLite for tiny files. |
| File size tiers | All files use chunk/slice/block model | Three tiers: inline SQLite (<64KB), single S3 object (64KB-8MB), chunked (>8MB) |
| Small file optimization | None. Even 1-byte files get a slice + S3 object. | Inline in SQLite. Zero S3 round-trips for tiny files. |
| Reference counting | `sliceRef` table tracks refs per slice. Clone increments refs. Compaction decrements. | Version-based. Old versions kept until GC. Simpler model since we don't need clone/COW. |

### Key Differences

1. **JuiceFS is designed for shared multi-client POSIX workloads.** It needs session tracking, distributed locks, sustained inodes, and compaction under concurrent access. We are single-client (one VM per filesystem instance), so we can skip all of that complexity.

2. **JuiceFS uses the slice model for ALL files.** Every byte written creates a slice record and an S3 object, even for a 10-byte config file. Our tiered approach avoids S3 round-trips for small files entirely.

3. **JuiceFS has no versioning because it wasn't designed for it.** The slice model technically contains historical data (old slices exist until compaction), but there's no way to query "what did this file look like 5 minutes ago." Our `inode_versions` table makes this a first-class operation.

4. **Our versioning is cheaper than JuiceFS clone.** Clone duplicates the entire metadata tree. Our versioning just increments a version number and keeps the old S3 key/inline content around. Rolling back = updating `current_version` on the inode.
