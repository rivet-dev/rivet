# LTX V3 Format: Research and Recommendation

## What V3 Changed from V1

The Go `superfly/ltx` repo evolved V1 to V3 across several PRs. Key changes:

- **Page header: 4 bytes to 6 bytes.** V1 had only a `uint32` page number. V3 adds a `uint16` flags field (`PageHeaderFlagSize = 1 << 0`). The flag signals that a 4-byte compressed-size prefix follows the header.
- **LZ4 frame to LZ4 block compression.** V1 wrapped each page in a full LZ4 frame (header + blocks + endmark + content checksum = 8 extra bytes per page). V3 uses raw LZ4 block compression with an explicit 4-byte size prefix. This eliminates per-page framing overhead and simplifies seeking.
- **Page index (new).** After the last page, V3 writes a varint-encoded index mapping `pgno -> (offset, size)`, terminated by a zero pgno sentinel, followed by a big-endian `uint64` total index size. This enables random-access reads without scanning all pages.
- **Header expanded to 100 bytes.** V1 used the first 48 bytes. V3 adds WALOffset (8B), WALSize (8B), WALSalt1 (4B), WALSalt2 (4B), NodeID (8B), and reserves remaining bytes as zero.
- **`HeaderFlagNoChecksum` (bit 1).** When set, pre-apply and post-apply checksums must be zero. This lets producers skip the rolling checksum entirely, which is exactly what we want.
- **Decoder backward compatibility.** The V3 decoder checks `PageHeaderFlagSize` per-page. If the flag is absent, it falls back to V1-style LZ4 frame reads. This means V3 decoders can read V1 files.

## Go Source Line Counts

| File | Lines |
|------|-------|
| `ltx.go` (types, header, trailer, page header) | ~570 |
| `encoder.go` | 299 |
| `decoder.go` | 406 |
| `checksum.go` | ~130 |
| `encoder_test.go` | 286 |
| `decoder_test.go` | 386 |
| `ltx_test.go` | 834 |

Core encoder+decoder logic: ~700 lines of Go. With header/trailer marshaling: ~1,300 lines total.

## Existing `litetx` Rust Crate (V1)

| File | Lines |
|------|-------|
| `ltx.rs` (header, trailer, page header, CRC) | 330 |
| `encoder.rs` | 455 |
| `decoder.rs` | 280 |
| `types.rs` | 331 |
| `lib.rs` | 13 |

Total: ~1,400 lines of Rust implementing V1.

Key differences from V3:
- Page header is 4 bytes (no flags field).
- Uses `lz4_flex::frame::FrameEncoder` / `FrameDecoder` (LZ4 frame format, not block).
- No page index.
- No `NoChecksum` flag.
- No WAL fields or NodeID in header.
- Header flags use bit 0 (`COMPRESS_LZ4`) rather than V3's bit 1 (`NoChecksum`). V3 removed the compress flag entirely because compression is always-on at the block level.

Reusable from ltx-rs: CRC64 ISO digest setup, `Checksum`/`TXID`/`PageNum`/`PageSize` newtypes, `CrcDigestWrite`/`CrcDigestRead` wrappers, and the test structure.

## Options and Effort Estimates

### (A) Fork `litetx` and upgrade to V3

- Expand page header from 4 to 6 bytes, add flags field.
- Replace `lz4_flex::frame` with `lz4_flex::block` (`compress` / `decompress`).
- Add 4-byte size prefix after page header.
- Add page index encoding/decoding (varint encode/decode, sorted pgno map).
- Expand header to 100 bytes with WAL fields and NodeID.
- Add `NoChecksum` header flag.
- Update all tests.

Effort: **2-3 days.** The crate structure is solid, but the frame-to-block LZ4 change touches the core write/read paths. The page index is ~80 lines of new code. Risk: the crate's lifetime-heavy `Encoder<'a, W>` / `Decoder<'a, R>` design makes the CRC digest flow awkward. V3's approach of hashing uncompressed data (not the wire bytes) requires rethinking the `CrcDigestWrite` wrapper.

### (B) Write a fresh V3 encoder/decoder from scratch

Port the Go V3 `encoder.go` and `decoder.go` directly. Reuse the newtypes and CRC setup from `litetx`.

Estimated size: ~500-600 lines of Rust for encoder+decoder, ~200 lines for header/trailer/page-header types, ~300 lines for tests. Total ~1,000-1,100 lines.

Effort: **2-3 days.** The Go code is straightforward imperative code that maps cleanly to Rust. The tricky parts are:
1. LZ4 block API: `lz4_flex::block::compress` / `decompress` are simple, but buffer sizing needs care (`lz4_flex::block::get_maximum_output_size`).
2. Varint encoding for the page index: use the `integer-encoding` crate or hand-roll (5 lines).
3. CRC64 ISO: the `crc` crate supports this directly (`crc::CRC_64_GO_ISO`), already used by `litetx`.
4. The checksum hashes uncompressed page data, not the compressed wire bytes. This is simpler than V1's approach.

### (C) Use V1 format as-is

We could use V1 since we do not interop with external Litestream tooling.

Effort: **0 days.**

Trade-offs: No page index means no random-access reads (must scan sequentially). LZ4 frame overhead adds ~8 bytes per page. No `NoChecksum` flag means we must compute rolling checksums we do not use. If we ever want to interop with Fly.io's LTX tooling (litefs, litestream), we would need to upgrade later anyway.

### (D) Thin V3 wrapper over `litetx` types

Keep the `litetx` newtypes (`Checksum`, `TXID`, `PageNum`, `PageSize`, `Pos`) and CRC digest helpers. Write new V3 encoder/decoder as a separate module that does not use the V1 encoder/decoder at all.

Effort: **2 days.** Same as (B) but with less boilerplate for types.

## Recommendation: (D) Thin V3 wrapper reusing `litetx` types

Rationale:
1. The `litetx` newtypes and CRC setup are correct and well-tested. No reason to rewrite them.
2. The V1 encoder/decoder cannot be incrementally upgraded. The LZ4 format change and page index are fundamental enough that the encoder/decoder need full rewrites.
3. Writing V3 from scratch against the Go reference is faster than trying to understand and modify the V1 Rust code's lifetime patterns.
4. The `NoChecksum` flag is valuable for us. We do not track rolling checksums, so skipping them simplifies our code.
5. The page index enables future random-access reads if we need them for partial page fetches.

## Phase Plan

1. **Port types and constants** (0.5 day). Add V3 constants to existing types: `HeaderSize=100`, `PageHeaderSize=6`, `PageHeaderFlagSize`, `HeaderFlagNoChecksum`. Extend `Header` struct with WAL fields and NodeID.

2. **Port encoder** (0.5 day). Translate Go `encoder.go` to Rust. Use `lz4_flex::block::compress` for per-page compression. Implement page index encoding with varint. Hash uncompressed page data for the file checksum.

3. **Port decoder** (0.5 day). Translate Go `decoder.go` to Rust. Support both V1 (frame) and V3 (block) page formats by checking `PageHeaderFlagSize`. Implement page index decoding. Slurp remaining bytes for index + trailer like Go does.

4. **Port Go tests** (0.5 day). Translate `encoder_test.go` and `decoder_test.go`. Add round-trip tests (encode then decode). Add cross-version test: encode with V1 frame format, decode with V3 decoder.

5. **Integration** (0.5 day). Wire into the existing SQLite VFS shard writer. Replace any V1 LTX calls with V3. Set `HeaderFlagNoChecksum` since we do not use rolling checksums.

Total: **~2.5 days.**
