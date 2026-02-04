# Schema for long-running spans (tradeoffs and rationale)

This document explains the design choices for handling long-running spans in `@rivetkit/traces` and the tradeoffs between index-per-span and chunk-only storage.

## Options considered

### Option A: Per-span index (one KV key per span)
- Store a separate KV entry per span (`span_id -> start/snapshot pointer`).
- Reads can hydrate a long-running span with a single lookup.
- Writes are append-only; index updates happen on start and snapshot.

**Pros**
- Fast reads for long-running spans (O(1) base lookup per span).
- No backward scans required.
- Minimal chunk size inflation.

**Cons**
- Adds 1 KV key per span.
- Requires additional KV writes on start and snapshot.

### Option B: Chunk-only (chosen)
- Only chunk keys exist in KV.
- Each chunk includes an `activeSpans` snapshot (spanId -> start/snapshot pointers).
- On read, we look up the previous chunk to find spans that started before the range.

**Pros**
- Fewer KV keys (only chunks).
- Simpler KV footprint.

**Cons**
- Duplicated active span metadata in each chunk.
- Requires reverse list / previous-chunk lookup.
- If no previous chunk exists, we can only reconstruct spans that start within the range.

## Why chunk-only is acceptable here
- Active spans are expected to be **small in number**.
- We only write a chunk when there are records, so duplication happens only during active write periods.
- Reverse lookup of the previous chunk is cheap if the driver supports `listRange(..., reverse: true, limit: 1)`.

## Active span snapshot overhead (rough estimate)
An `ActiveSpanRef` entry contains:
- `spanId` (8 bytes)
- `startKey` (prefix + bucket + chunk + record index) ≈ 20 bytes
- `latestSnapshotKey` (optional) ≈ 20 bytes

Rough size per active span: **~32–96 bytes** depending on whether the snapshot pointer is present and VBare overhead.

Example:
- 200 active spans × 64 bytes ≈ **12.8 KB per chunk**
- 1,000 active spans × 64 bytes ≈ **64 KB per chunk**

This is acceptable relative to the default chunk size target (512 KiB).

## Reverse scan requirement
To hydrate spans that began before the read range, we need the **previous chunk**:
- Use `listRange(startKey, endKey, { reverse: true, limit: 1 })` to find the latest chunk before `startMs`.
- If the driver cannot support reverse range, we would need a separate “latest chunk” pointer (meta key) or accept expensive forward scans.

## Snapshots vs start pointer
Long-running spans can accumulate many deltas. We store periodic `SpanSnapshot` records and update the `latestSnapshotKey` in `activeSpans`:
- Reduces hydration cost for spans that have been active for months.
- Keeps reads bounded without rewriting old chunks.

## Active span cap (depth-based dropping)
To avoid unbounded in-memory growth, we cap the number of active spans:
- `maxActiveSpans` is enforced by **dropping the deepest spans first** (keep shallower spans).
- Depth is calculated by parent links among active spans (root = 0).
- Tie-breaker: drop the most recently started spans.
- Dropped spans stop emitting events/updates/end records.

## Summary
Chunk-only storage trades off a small amount of duplicate metadata per chunk for a simpler KV footprint and avoids per-span index keys. With the reverse list requirement and periodic snapshots, long-running spans remain efficient to read and write under heavy load.
