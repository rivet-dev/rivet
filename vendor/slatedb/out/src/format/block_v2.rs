#![allow(dead_code)]
use crate::error::SlateDBError;
use crate::format::block::{Block, SIZEOF_U16};
use crate::format::row_codec_v2::{SstRowCodecV2, SstRowEntryV2};
use crate::types::RowEntry;
use bytes::Bytes;

const DEFAULT_RESTART_INTERVAL: usize = 16;

/// Block V2 encoding uses prefix compression and restart points.
///
/// # Prefix Compression Algorithm
///
/// Entries are stored with delta encoding relative to the **previous key**:
/// - Each entry stores `shared_bytes` (bytes in common with the previous key) and `key_suffix`
///   (the remaining bytes that differ)
/// - This achieves good compression for sorted keys with common prefixes (e.g., "user:1000", "user:1001")
///
/// # Restart Points
///
/// To enable efficient seeking, the block is divided into "restart regions":
/// - Every N entries (default: 16), a "restart point" is created
/// - At restart points, entries store the full key (`shared_bytes = 0`)
/// - The trailer stores byte offsets to each restart point
///
/// # Seeking Algorithm
///
/// To find a key:
/// 1. Binary search the restart point offsets to find the region containing the target
/// 2. Seek to that restart point and decode the full key
/// 3. Linear scan forward, reconstructing keys via prefix compression, until key >= target
///
/// # Block Layout
///
/// ```text
/// +--------+--------+-----+--------+------------+------------+-----+--------------+
/// | entry0 | entry1 | ... | entryN | restart[0] | restart[1] | ... | num_restarts |
/// +--------+--------+-----+--------+------------+------------+-----+--------------+
///                                  |<------ u16 each ------->|     |<--- u16 --->|
/// ```
///
/// - `entry_i`: encoded using `SstRowCodecV2` with prefix compression
/// - `restart[i]`: byte offset (u16) to the i-th restart point entry
/// - `num_restarts`: count of restart points (u16)
///
/// # Unified Block Type
///
/// BlockBuilderV2 produces the standard `Block` type. The `offsets` field in `Block` contains
/// restart point offsets (not per-entry offsets like BlockBuilder V0). Use `BlockIteratorV2`
/// to iterate over blocks built with `BlockBuilderV2`.
/// Compute the length of the common prefix between two byte slices.
fn compute_prefix(lhs: &[u8], rhs: &[u8]) -> usize {
    compute_prefix_chunks::<128>(lhs, rhs)
}

/// Optimized common prefix computation using chunked comparison.
///
/// This algorithm improves performance over naive byte-by-byte comparison by:
/// 1. First comparing N-byte chunks at a time (default N=128)
/// 2. Once a differing chunk is found, falling back to byte-by-byte comparison
///
/// For keys with long common prefixes, this significantly reduces the number of
/// comparisons needed. The chunk size of 128 bytes is chosen to balance between
/// the overhead of chunk iteration and the benefits of bulk comparison.
fn compute_prefix_chunks<const N: usize>(lhs: &[u8], rhs: &[u8]) -> usize {
    // Compare N-byte chunks until we find one that differs
    let off = std::iter::zip(lhs.chunks_exact(N), rhs.chunks_exact(N))
        .take_while(|(a, b)| a == b)
        .count()
        * N;
    // Then compare remaining bytes one at a time
    off + std::iter::zip(&lhs[off..], &rhs[off..])
        .take_while(|(a, b)| a == b)
        .count()
}

/// Builder for BlockV2 with restart points.
pub(super) struct BlockBuilderV2 {
    data: Vec<u8>,
    restarts: Vec<u16>,
    block_size: usize,
    restart_interval: usize,
    counter: usize,
    // NOTE: BlockBuilderV1 needs the first key to compute the prefix
    // but BlockBuilderV2 needs the most recent key to compute the shared
    // prefix. If you're comparing the two this field is only used internally
    // so it does not affect the "Block API" at all
    last_key: Bytes,
}

impl BlockBuilderV2 {
    /// Create a new builder with default restart interval (16).
    pub(crate) fn new(block_size: usize) -> Self {
        Self {
            data: Vec::new(),
            restarts: Vec::new(),
            block_size,
            restart_interval: DEFAULT_RESTART_INTERVAL,
            counter: 0,
            last_key: Bytes::new(),
        }
    }

    /// Create a new builder with a custom restart interval.
    #[cfg(test)]
    pub(crate) fn new_with_restart_interval(block_size: usize, restart_interval: usize) -> Self {
        Self {
            data: Vec::new(),
            restarts: Vec::new(),
            block_size,
            restart_interval,
            counter: 0,
            last_key: Bytes::new(),
        }
    }

    /// Calculate the current size of the block being built.
    fn current_size(&self) -> usize {
        self.data.len()
            + self.restarts.len() * SIZEOF_U16 // restart offsets
            + SIZEOF_U16 // num_restarts
    }

    /// Calculate the encoded size of an entry.
    fn entry_encoded_size(&self, entry: &RowEntry) -> usize {
        let shared_bytes = if self.is_restart_point() {
            0
        } else {
            compute_prefix(&self.last_key, &entry.key) as u32
        };
        let key_suffix = &entry.key[shared_bytes as usize..];

        let temp_entry = SstRowEntryV2::new(
            shared_bytes,
            Bytes::copy_from_slice(key_suffix),
            entry.seq,
            entry.value.clone(),
            entry.create_ts,
            entry.expire_ts,
        );
        temp_entry.encoded_size()
    }

    /// Check if we're at a restart point.
    fn is_restart_point(&self) -> bool {
        self.counter.is_multiple_of(self.restart_interval)
    }

    /// Checks if the entry would fit in the current block without consuming it.
    /// Empty blocks always return true (they accept entries that exceed block_size).
    pub(crate) fn would_fit(&self, entry: &RowEntry) -> bool {
        if self.is_empty() {
            return true;
        }

        let entry_size = self.entry_encoded_size(entry);
        let restart_overhead = if self.is_restart_point() {
            SIZEOF_U16 // new restart offset
        } else {
            0
        };

        self.current_size() + entry_size + restart_overhead <= self.block_size
    }

    /// Add an entry to the block. Returns Ok(true) if the entry was added.
    pub(crate) fn add(&mut self, entry: RowEntry) -> Result<bool, SlateDBError> {
        if entry.key.is_empty() {
            return Err(SlateDBError::EmptyKey);
        }

        if !self.would_fit(&entry) {
            return Ok(false);
        }

        // Preallocate the data buffer on the first entry, sized to the larger
        // of the block target and the first entry's encoded size. The block's
        // final `data` length is bounded above by `max(block_size,
        // first_entry_encoded_size)` because:
        //   - If the first entry is smaller than `block_size`, `would_fit`
        //     caps subsequent entries at `block_size`.
        //   - If the first entry is >= `block_size` (oversized-entry path,
        //     e.g., large filesystem chunk values), the block accepts it on
        //     the empty-block rule, and any subsequent entry is rejected
        //     because `self.current_size() > block_size` already.
        // Reserving upfront avoids the ~log2(final_size) reallocs that
        // geometric Vec growth would otherwise perform per block.
        if self.data.is_empty() {
            let first_entry_size = self.entry_encoded_size(&entry);
            self.data.reserve(first_entry_size.max(self.block_size));
        }

        let shared_bytes = if self.is_restart_point() {
            // Record restart offset
            assert!(
                self.data.len() <= u16::MAX as usize,
                "Block data too large for u16 offset: {} bytes",
                self.data.len()
            );
            self.restarts.push(self.data.len() as u16);
            0 // Full key at restart point
        } else {
            compute_prefix(&self.last_key, &entry.key) as u32
        };

        let key_suffix = Bytes::copy_from_slice(&entry.key[shared_bytes as usize..]);

        let sst_row = SstRowEntryV2::new(
            shared_bytes,
            key_suffix,
            entry.seq,
            entry.value,
            entry.create_ts,
            entry.expire_ts,
        );

        let codec = SstRowCodecV2::new();
        codec.encode(&mut self.data, &sst_row);

        self.last_key = entry.key;
        self.counter += 1;

        Ok(true)
    }

    /// Check if the block is empty.
    pub(crate) fn is_empty(&self) -> bool {
        self.counter == 0
    }

    /// Build the final block.
    pub(crate) fn build(self) -> Result<Block, SlateDBError> {
        if self.is_empty() {
            return Err(SlateDBError::EmptyBlock);
        }
        Ok(Block {
            data: Bytes::from(self.data),
            offsets: self.restarts,
        })
    }

    #[cfg(test)]
    pub(crate) fn add_value(
        &mut self,
        key: &[u8],
        value: &[u8],
        ts: Option<i64>,
        expire_ts: Option<i64>,
    ) -> bool {
        let entry = RowEntry::new(
            key.to_vec().into(),
            crate::types::ValueDeletable::Value(Bytes::copy_from_slice(value)),
            0,
            ts,
            expire_ts,
        );
        self.add(entry).unwrap_or(false)
    }

    #[cfg(test)]
    pub(crate) fn restart_interval(&self) -> usize {
        self.restart_interval
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ValueDeletable;
    use rstest::rstest;

    fn make_entry(key: &[u8], value: &[u8], seq: u64) -> RowEntry {
        RowEntry::new(
            Bytes::copy_from_slice(key),
            ValueDeletable::Value(Bytes::copy_from_slice(value)),
            seq,
            None,
            None,
        )
    }

    #[test]
    fn should_build_single_entry_block() {
        // given: a builder with one entry
        let mut builder = BlockBuilderV2::new(4096);

        // when: adding one entry
        let added = builder.add(make_entry(b"key1", b"value1", 1));

        // then: entry is added and block builds successfully
        assert!(added.unwrap());
        let block = builder.build().expect("build failed");

        // Block should have one restart point at offset 0
        assert_eq!(block.offsets.len(), 1);
        assert_eq!(block.offsets[0], 0);
    }

    #[test]
    fn should_build_block_with_restart_points() {
        // given: a builder with restart_interval=4
        let mut builder = BlockBuilderV2::new_with_restart_interval(4096, 4);

        // when: adding 10 entries
        for i in 0..10 {
            let key = format!("key_{:03}", i);
            let value = format!("value_{}", i);
            assert!(builder
                .add(make_entry(key.as_bytes(), value.as_bytes(), i as u64))
                .unwrap());
        }

        // then: block has correct number of restart points (0, 4, 8)
        let block = builder.build().expect("build failed");
        assert_eq!(block.offsets.len(), 3); // entries 0, 4, 8 are restart points
    }

    #[test]
    fn should_encode_decode_round_trip() {
        // given: a block with multiple entries
        let mut builder = BlockBuilderV2::new_with_restart_interval(4096, 4);
        for i in 0..8 {
            let key = format!("key_{:03}", i);
            let value = format!("value_{}", i);
            let _ = builder.add(make_entry(key.as_bytes(), value.as_bytes(), i as u64));
        }
        let block = builder.build().expect("build failed");

        // when: encoding and decoding
        let encoded = block.encode();
        let decoded = Block::decode(encoded);

        // then: blocks are equal
        assert_eq!(block.data, decoded.data);
        assert_eq!(block.offsets, decoded.offsets);
    }

    #[test]
    fn should_reject_entry_exceeding_block_size() {
        // given: a small block that already has entries
        let mut builder = BlockBuilderV2::new(100);
        let _ = builder.add(make_entry(b"key1", b"value1", 1));

        // when: trying to add a large entry
        let large_value = vec![b'x'; 200];
        let would_fit = builder.would_fit(&make_entry(b"key2", &large_value, 2));

        // then: entry doesn't fit
        assert!(!would_fit);
    }

    #[test]
    fn should_accept_first_entry_exceeding_block_size() {
        // given: an empty builder with small block size
        let builder = BlockBuilderV2::new(10);

        // when: checking if a large entry fits in an empty block
        let large_value = vec![b'x'; 200];
        let would_fit = builder.would_fit(&make_entry(b"key1", &large_value, 1));

        // then: entry fits (empty blocks always accept)
        assert!(would_fit);
    }

    #[test]
    fn should_compute_correct_restart_offsets() {
        // given: a builder with restart_interval=2
        let mut builder = BlockBuilderV2::new_with_restart_interval(4096, 2);

        // when: adding entries
        let _ = builder.add(make_entry(b"aaa", b"v1", 1)); // restart 0 at offset 0
        let _ = builder.add(make_entry(b"aab", b"v2", 2)); // not a restart
        let _ = builder.add(make_entry(b"bbb", b"v3", 3)); // restart 1
        let _ = builder.add(make_entry(b"bbc", b"v4", 4)); // not a restart
        let _ = builder.add(make_entry(b"ccc", b"v5", 5)); // restart 2

        // then: restarts are at correct offsets
        let block = builder.build().expect("build failed");
        assert_eq!(block.offsets.len(), 3);

        // First restart should be at offset 0
        assert_eq!(block.offsets[0], 0);

        // Other restarts should be at non-zero offsets
        assert!(block.offsets[1] > 0);
        assert!(block.offsets[2] > block.offsets[1]);
    }

    #[test]
    fn should_store_full_key_at_restart_points() {
        // given: entries with shared prefixes and restart_interval=2
        let mut builder = BlockBuilderV2::new_with_restart_interval(4096, 2);

        let _ = builder.add(make_entry(b"prefix_aaa", b"v1", 1)); // restart, shared=0
        let _ = builder.add(make_entry(b"prefix_aab", b"v2", 2)); // shared with prev
        let _ = builder.add(make_entry(b"prefix_bbb", b"v3", 3)); // restart, shared=0

        let block = builder.build().expect("build failed");

        // when: decoding entries at restart points
        let codec = SstRowCodecV2::new();

        // First restart point
        let mut data: &[u8] = &block.data[block.offsets[0] as usize..];
        let entry0 = codec.decode(&mut data).expect("decode failed");

        // Second restart point
        let mut data: &[u8] = &block.data[block.offsets[1] as usize..];
        let entry2 = codec.decode(&mut data).expect("decode failed");

        // then: restart point entries have shared_bytes=0
        assert_eq!(entry0.shared_bytes, 0);
        assert_eq!(entry0.key_suffix.as_ref(), b"prefix_aaa");

        assert_eq!(entry2.shared_bytes, 0);
        assert_eq!(entry2.key_suffix.as_ref(), b"prefix_bbb");
    }

    #[test]
    fn should_use_prefix_compression_between_restarts() {
        // given: entries with shared prefixes
        let mut builder = BlockBuilderV2::new_with_restart_interval(4096, 16);

        let _ = builder.add(make_entry(b"prefix_aaa", b"v1", 1)); // restart, shared=0
        let _ = builder.add(make_entry(b"prefix_aab", b"v2", 2)); // shares "prefix_aa" (9 bytes)

        let block = builder.build().expect("build failed");

        // when: decoding the second entry
        let codec = SstRowCodecV2::new();

        // Skip first entry
        let restart_offset = block.offsets[0] as usize;
        let mut data: &[u8] = &block.data[restart_offset..];
        let first_entry = codec.decode(&mut data).expect("decode failed");

        // Now data points to second entry
        let second_entry = codec.decode(&mut data).expect("decode failed");

        // then: second entry uses prefix compression
        assert_eq!(first_entry.shared_bytes, 0);
        assert_eq!(second_entry.shared_bytes, 9); // "prefix_aa" shared
        assert_eq!(second_entry.key_suffix.as_ref(), b"b"); // only "b" differs
    }

    #[test]
    fn should_return_correct_block_size() {
        // given: a block with entries
        let mut builder = BlockBuilderV2::new_with_restart_interval(4096, 4);
        for i in 0..5 {
            let key = format!("key_{:03}", i);
            let value = format!("val_{}", i);
            let _ = builder.add(make_entry(key.as_bytes(), value.as_bytes(), i as u64));
        }
        let block = builder.build().expect("build failed");

        // when: encoding the block
        let encoded = block.encode();

        // then: size matches encoded length
        assert_eq!(block.size(), encoded.len());
    }

    #[test]
    fn should_build_empty_returns_error() {
        // given: an empty builder
        let builder = BlockBuilderV2::new(4096);

        // when: trying to build
        let result = builder.build();

        // then: error is returned
        assert!(matches!(result, Err(SlateDBError::EmptyBlock)));
    }

    #[test]
    fn should_handle_tombstones() {
        // given: entries including tombstones
        let mut builder = BlockBuilderV2::new(4096);

        let _ = builder.add(RowEntry::new(
            Bytes::from("key1"),
            ValueDeletable::Value(Bytes::from("value1")),
            1,
            None,
            None,
        ));
        let _ = builder.add(RowEntry::new(
            Bytes::from("key2"),
            ValueDeletable::Tombstone,
            2,
            Some(100),
            None,
        ));

        // when: building the block
        let block = builder.build().expect("build failed");

        // then: block builds successfully
        assert!(!block.data.is_empty());
    }

    #[rstest]
    #[case(1)]
    #[case(2)]
    #[case(4)]
    #[case(16)]
    #[case(32)]
    fn should_handle_various_restart_intervals(#[case] interval: usize) {
        // given: a builder with specified restart interval
        let mut builder = BlockBuilderV2::new_with_restart_interval(8192, interval);
        let num_entries: usize = 50;

        // when: adding many entries
        for i in 0..num_entries {
            let key = format!("key_{:05}", i);
            let value = format!("value_{}", i);
            let _ = builder.add(make_entry(key.as_bytes(), value.as_bytes(), i as u64));
        }

        let block = builder.build().expect("build failed");

        // then: correct number of restart points
        let expected_restarts = num_entries.div_ceil(interval);
        assert_eq!(block.offsets.len(), expected_restarts);
    }

    #[test]
    fn should_add_value_helper_works() {
        // given: a builder
        let mut builder = BlockBuilderV2::new(4096);

        // when: using add_value helper
        assert!(builder.add_value(b"key1", b"value1", None, None));
        assert!(builder.add_value(b"key2", b"value2", None, None));

        // then: block builds successfully
        let block = builder.build().expect("build failed");
        assert!(!block.data.is_empty());
    }

    #[test]
    fn should_handle_large_key_requiring_u32_length() {
        // given: a key larger than 64KB (requires more than u16 for length)
        // Using 3MB key to ensure varint encoding handles large values correctly
        let large_key_size = 3 * 1024 * 1024; // 3MB
        let large_key = vec![b'k'; large_key_size];
        let value = b"small_value";

        // Use a large block size to accommodate the key
        let mut builder = BlockBuilderV2::new(4 * 1024 * 1024); // 4MB block

        // when: adding entry with large key
        let entry = RowEntry::new(
            Bytes::from(large_key.clone()),
            ValueDeletable::Value(Bytes::copy_from_slice(value)),
            1,
            None,
            None,
        );
        let added = builder.add(entry);

        // then: entry is added successfully
        assert!(added.unwrap());

        let block = builder.build().expect("build failed");

        // Verify the block can be encoded and decoded
        let encoded = block.encode();
        let decoded = Block::decode(encoded);

        assert_eq!(block.data, decoded.data);
        assert_eq!(block.offsets, decoded.offsets);

        // Verify we can read the entry back with correct key length
        let codec = SstRowCodecV2::new();
        let mut data: &[u8] = &decoded.data[decoded.offsets[0] as usize..];
        let entry = codec.decode(&mut data).expect("decode failed");

        assert_eq!(entry.shared_bytes, 0);
        assert_eq!(entry.key_suffix.len(), large_key_size);
        assert_eq!(entry.key_suffix.as_ref(), large_key.as_slice());
    }

    #[test]
    fn should_handle_large_key_with_prefix_compression() {
        // given: two large keys with a shared prefix
        let prefix_size = 1024 * 1024; // 1MB shared prefix
        let suffix_size = 100;

        let mut key1 = vec![b'p'; prefix_size];
        key1.extend(vec![b'a'; suffix_size]);

        let mut key2 = vec![b'p'; prefix_size];
        key2.extend(vec![b'b'; suffix_size]);

        let mut builder = BlockBuilderV2::new(4 * 1024 * 1024);

        // when: adding both entries
        let _ = builder.add(RowEntry::new(
            Bytes::from(key1.clone()),
            ValueDeletable::Value(Bytes::from("v1")),
            1,
            None,
            None,
        ));
        let _ = builder.add(RowEntry::new(
            Bytes::from(key2.clone()),
            ValueDeletable::Value(Bytes::from("v2")),
            2,
            None,
            None,
        ));

        let block = builder.build().expect("build failed");

        // then: second entry uses prefix compression with large shared_bytes
        let codec = SstRowCodecV2::new();

        // Skip first entry
        let mut data: &[u8] = &block.data[block.offsets[0] as usize..];
        let first_entry = codec.decode(&mut data).expect("decode failed");
        assert_eq!(first_entry.shared_bytes, 0);
        assert_eq!(first_entry.key_suffix.len(), key1.len());

        // Second entry should have large shared_bytes (the 1MB prefix)
        let second_entry = codec.decode(&mut data).expect("decode failed");
        assert_eq!(second_entry.shared_bytes, prefix_size as u32);
        assert_eq!(second_entry.key_suffix.len(), suffix_size);
    }
}

/// Comparison tests between BlockBuilder (V1) and BlockBuilderV2 block sizes.
/// Run with `cargo test -p slatedb block_size_comparison -- --nocapture` to see output.
///
/// Results (as of initial implementation):
/// ```text
/// Scenario                                 |       Entries |            V1 Size |            V2 Size |         Difference
/// ------------------------------------------------------------------------------------------------------------------------
/// Sequential keys (key0001..key0100)       |    100 entries | V1:     2600 bytes | V2:     1869 bytes | diff:   +731 (+28.1%)
/// Sequential keys, 100-byte values         |    100 entries | V1:    12100 bytes | V2:    11369 bytes | diff:   +731 (+6.0%)
/// Long common prefix (90% shared)          |    100 entries | V1:     2638 bytes | V2:     2135 bytes | diff:   +503 (+19.1%)
/// Random keys (no common prefix)           |    100 entries | V1:     4322 bytes | V2:     3541 bytes | diff:   +781 (+18.1%)
/// Few entries (10 sequential)              |     10 entries | V1:      259 bytes | V2:      191 bytes | diff:    +68 (+26.3%)
/// Many small entries (500)                 |    500 entries | V1:    11398 bytes | V2:     7249 bytes | diff:  +4149 (+36.4%)
/// Tombstones (100 sequential keys)         |    100 entries | V1:     1700 bytes | V2:     1369 bytes | diff:   +331 (+19.5%)
/// Mixed: 50% values, 50% tombstones        |    100 entries | V1:     2150 bytes | V2:     1619 bytes | diff:   +531 (+24.7%)
/// Varying value sizes (1-500 bytes)        |    100 entries | V1:    26950 bytes | V2:    26293 bytes | diff:   +657 (+2.4%)
/// Large values (1KB each)                  |     50 entries | V1:    52249 bytes | V2:    51939 bytes | diff:   +310 (+0.6%)
/// Short keys (1-3 chars)                   |    100 entries | V1:     2499 bytes | V2:     1816 bytes | diff:   +683 (+27.3%)
/// UUID-like keys                           |    100 entries | V1:     5717 bytes | V2:     4963 bytes | diff:   +754 (+13.2%)
/// Hierarchical paths (/a/b/c/...)          |    100 entries | V1:     3942 bytes | V2:     3181 bytes | diff:   +761 (+19.3%)
/// With create timestamps                   |    100 entries | V1:     3400 bytes | V2:     2669 bytes | diff:   +731 (+21.5%)
/// With create and expire timestamps        |    100 entries | V1:     4200 bytes | V2:     3469 bytes | diff:   +731 (+17.4%)
///
/// Total across all scenarios: V1=136124 bytes, V2=123944 bytes, diff=+12180 (+8.9%)
/// ```
///
/// Positive diff means V1 is larger (V2 saves space).
#[cfg(test)]
mod block_size_comparison {
    use super::*;
    use crate::format::sst::BlockBuilder;
    use crate::types::ValueDeletable;

    const BLOCK_SIZE: usize = 64 * 1024; // 64KB blocks

    struct ComparisonResult {
        scenario: &'static str,
        entry_count: usize,
        v1_size: usize,
        v2_size: usize,
    }

    impl ComparisonResult {
        fn print(&self) {
            let diff = self.v1_size as i64 - self.v2_size as i64;
            let pct = if self.v1_size > 0 {
                (diff as f64 / self.v1_size as f64) * 100.0
            } else {
                0.0
            };
            println!(
                "{:<40} | {:>6} entries | V1: {:>8} bytes | V2: {:>8} bytes | diff: {:>+6} ({:>+.1}%)",
                self.scenario, self.entry_count, self.v1_size, self.v2_size, diff, pct
            );
        }
    }

    fn build_v1_block(entries: &[RowEntry]) -> usize {
        let mut builder = BlockBuilder::new_v1(BLOCK_SIZE);
        for entry in entries {
            let _ = builder.add(entry.clone());
        }
        builder.build().map(|b| b.size()).unwrap_or(0)
    }

    fn build_v2_block(entries: &[RowEntry]) -> usize {
        let mut builder = BlockBuilderV2::new(BLOCK_SIZE);
        for entry in entries {
            let _ = builder.add(entry.clone());
        }
        builder.build().map(|b| b.size()).unwrap_or(0)
    }

    fn compare(scenario: &'static str, entries: Vec<RowEntry>) -> ComparisonResult {
        let entry_count = entries.len();
        let v1_size = build_v1_block(&entries);
        let v2_size = build_v2_block(&entries);
        ComparisonResult {
            scenario,
            entry_count,
            v1_size,
            v2_size,
        }
    }

    fn make_entry(key: &[u8], value: &[u8], seq: u64) -> RowEntry {
        RowEntry::new(
            Bytes::copy_from_slice(key),
            ValueDeletable::Value(Bytes::copy_from_slice(value)),
            seq,
            None,
            None,
        )
    }

    fn make_tombstone(key: &[u8], seq: u64) -> RowEntry {
        RowEntry::new(
            Bytes::copy_from_slice(key),
            ValueDeletable::Tombstone,
            seq,
            None,
            None,
        )
    }

    #[test]
    fn should_compare_block_sizes() {
        println!("\n=== Block Size Comparison: V1 vs V2 ===\n");
        println!(
            "{:<40} | {:>13} | {:>18} | {:>18} | {:>18}",
            "Scenario", "Entries", "V1 Size", "V2 Size", "Difference"
        );
        println!("{}", "-".repeat(120));

        let results = vec![
            // Scenario 1: Sequential numeric keys (good prefix sharing for V2)
            compare("Sequential keys (key0001..key0100)", {
                (1..=100)
                    .map(|i| make_entry(format!("key{:04}", i).as_bytes(), b"value", i))
                    .collect()
            }),
            // Scenario 2: Sequential keys with longer values
            compare("Sequential keys, 100-byte values", {
                let value = vec![b'v'; 100];
                (1..=100)
                    .map(|i| make_entry(format!("key{:04}", i).as_bytes(), &value, i))
                    .collect()
            }),
            // Scenario 3: Keys with long common prefix (best case for V2)
            compare("Long common prefix (90% shared)", {
                let prefix = "com.example.application.module.submodule.";
                (1..=100)
                    .map(|i| make_entry(format!("{}{:04}", prefix, i).as_bytes(), b"value", i))
                    .collect()
            }),
            // Scenario 4: Random keys with no common prefix (worst case for V2)
            compare("Random keys (no common prefix)", {
                // Use deterministic "random" keys
                (1..=100)
                    .map(|i| {
                        let key = format!("{:08x}{:08x}", i * 2654435761, i * 1597334677);
                        make_entry(key.as_bytes(), b"value", i)
                    })
                    .collect()
            }),
            // Scenario 5: Small number of entries
            compare("Few entries (10 sequential)", {
                (1..=10)
                    .map(|i| make_entry(format!("key{:04}", i).as_bytes(), b"value", i))
                    .collect()
            }),
            // Scenario 6: Many small entries
            compare("Many small entries (500)", {
                (1..=500)
                    .map(|i| make_entry(format!("k{:04}", i).as_bytes(), b"v", i))
                    .collect()
            }),
            // Scenario 7: Tombstones only
            compare("Tombstones (100 sequential keys)", {
                (1..=100)
                    .map(|i| make_tombstone(format!("key{:04}", i).as_bytes(), i))
                    .collect()
            }),
            // Scenario 8: Mixed values and tombstones
            compare("Mixed: 50% values, 50% tombstones", {
                (1..=100)
                    .map(|i| {
                        if i % 2 == 0 {
                            make_entry(format!("key{:04}", i).as_bytes(), b"value", i)
                        } else {
                            make_tombstone(format!("key{:04}", i).as_bytes(), i)
                        }
                    })
                    .collect()
            }),
            // Scenario 9: Varying value sizes
            compare("Varying value sizes (1-500 bytes)", {
                (1..=100)
                    .map(|i| {
                        let value = vec![b'v'; (i as usize * 5) % 500 + 1];
                        make_entry(format!("key{:04}", i).as_bytes(), &value, i)
                    })
                    .collect()
            }),
            // Scenario 10: Large values (1KB each)
            compare("Large values (1KB each)", {
                let value = vec![b'v'; 1024];
                (1..=50)
                    .map(|i| make_entry(format!("key{:04}", i).as_bytes(), &value, i))
                    .collect()
            }),
            // Scenario 11: Very short keys
            compare("Short keys (1-3 chars)", {
                (1..=100)
                    .map(|i| {
                        let key = format!("{}", (b'a' + (i % 26) as u8) as char);
                        make_entry(key.as_bytes(), b"value", i)
                    })
                    .collect()
            }),
            // Scenario 12: UUID-like keys
            compare("UUID-like keys", {
                (1..=100)
                    .map(|i| {
                        let key = format!(
                            "{:08x}-{:04x}-{:04x}-{:04x}-{:012x}",
                            i * 12345,
                            i * 67,
                            i * 89,
                            i * 101,
                            i * 112131
                        );
                        make_entry(key.as_bytes(), b"value", i)
                    })
                    .collect()
            }),
            // Scenario 13: Hierarchical paths
            compare("Hierarchical paths (/a/b/c/...)", {
                (1..=100)
                    .map(|i| {
                        let depth = (i % 5) + 1;
                        let path: String = (0..depth)
                            .map(|d| format!("/level{}", d))
                            .collect::<String>()
                            + &format!("/item{:04}", i);
                        make_entry(path.as_bytes(), b"value", i)
                    })
                    .collect()
            }),
            // Scenario 14: Entries with timestamps
            compare("With create timestamps", {
                (1..=100)
                    .map(|i| {
                        RowEntry::new(
                            Bytes::from(format!("key{:04}", i)),
                            ValueDeletable::Value(Bytes::from("value")),
                            i,
                            Some(1700000000000 + i as i64),
                            None,
                        )
                    })
                    .collect()
            }),
            // Scenario 15: Entries with both timestamps
            compare("With create and expire timestamps", {
                (1..=100)
                    .map(|i| {
                        RowEntry::new(
                            Bytes::from(format!("key{:04}", i)),
                            ValueDeletable::Value(Bytes::from("value")),
                            i,
                            Some(1700000000000 + i as i64),
                            Some(1800000000000 + i as i64),
                        )
                    })
                    .collect()
            }),
        ];

        for result in &results {
            result.print();
        }

        println!("\n=== Summary ===");
        let total_v1: usize = results.iter().map(|r| r.v1_size).sum();
        let total_v2: usize = results.iter().map(|r| r.v2_size).sum();
        let total_diff = total_v1 as i64 - total_v2 as i64;
        let total_pct = (total_diff as f64 / total_v1 as f64) * 100.0;
        println!(
            "Total across all scenarios: V1={} bytes, V2={} bytes, diff={:+} ({:+.1}%)\n",
            total_v1, total_v2, total_diff, total_pct
        );
    }
}
