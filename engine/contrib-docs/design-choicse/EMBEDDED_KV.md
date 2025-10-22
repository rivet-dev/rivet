# Embedded KV

## Goals

- Embedded (no need for multi-process)
- Similarity to FDB in transacations
- Supports high write throughput

## Decision

Chose RocksDB because:

- It's stable & boring
- Supports high write throughput

Nice to haves:

- Better compression on disk

## Evaluation

### RocksDB

Uses LSM trees. Requires backgound compaction. Has a lot of overhead to work like a "real" database.

Pros:

- Supports a WAL for logging & buffering writes

Cons:

- Significantly more complicated
- Overhead of background threads managing compaction
- Does not provide native MVCC + conflict ranges, we must re-implement this ourselves

### LMDB

Uses B+ tree for storage. mmaps database.

**The critical downside of LMDB is that performance suffers under write-heavy workoads from (a) single writer locks and (b) uses mmap instead of buffering writes.**

Pros:

- Simple implementation
- Very little overhead

Cons:

- No compaction
- Single writer lock
- Transaction commits each have an fsync, while RocksDB can optimize with a WAL + batching writes
- Uses pages with mmap which is slower than using a WAL + batching writes
- Does not work well with small writes since it's copied at the page level (Rivet is almost all small writes)

### sled, redb, fjall

All three differe significantly, but the primary reason for disqualifying these is maturity. Sled claims to be "beta" and not mature yet, while redb and fjall claim to have a stable disk format.

**This is not a place we need to innovate, so it makes more sense to go with an established embedded KV.**

Pros:

- Rust-native
	- Less potential issues with building & linking
	- Theoretically faster compilation than RocksDB

### SQLite

This would be the simplest & most straightforward option in terms of support. FDB's pre-7.0 storage engine is based on the SQLite's B-tree implementation.

**SQLite provides overhead we don't need when options like RocksDB or LMDB exist.**

Pros:

- Widely supported, no issues with building it
- We already have a deep experience with SQLite

## Resources

- https://github.com/marvin-j97/rust-storage-bench
