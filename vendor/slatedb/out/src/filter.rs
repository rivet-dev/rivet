use std::sync::Arc;

use crate::filter_policy::{Filter, FilterBuilder, FilterQuery, FilterTarget};
use crate::prefix_extractor::{PrefixExtractor, PrefixTarget};
use crate::types::RowEntry;
use crate::utils::clamp_allocated_size_bytes;
use bytes::{Buf, BufMut, Bytes};
use siphasher::sip::SipHasher13;

pub struct BloomFilterBuilder {
    bits_per_key: u32,
    whole_key_filtering: bool,
    prefix_extractor: Option<Arc<dyn PrefixExtractor>>,
    key_hashes: Vec<u64>,
    last_prefix: Option<Vec<u8>>,
}

pub struct BloomFilter {
    num_probes: u16,
    whole_key_filtering: bool,
    prefix_extractor: Option<Arc<dyn PrefixExtractor>>,
    buffer: Bytes,
}

impl BloomFilterBuilder {
    pub(crate) fn new(
        bits_per_key: u32,
        whole_key_filtering: bool,
        prefix_extractor: Option<Arc<dyn PrefixExtractor>>,
    ) -> Self {
        Self {
            bits_per_key,
            whole_key_filtering,
            prefix_extractor,
            key_hashes: Vec::new(),
            last_prefix: None,
        }
    }

    pub(crate) fn add_key(&mut self, key: &Bytes) {
        // Keys must arrive in sorted order (as in SST construction) for the
        // deduplication of prefix hashes to work.
        if let Some(ref extractor) = self.prefix_extractor {
            let target = PrefixTarget::Point(key.clone());
            if let Some(len) = extractor.prefix_len(&target) {
                assert!(
                    len <= key.len(),
                    "PrefixExtractor returned a prefix length ({len}) greater than the key length ({})",
                    key.len()
                );
                let prefix = &key[..len];
                let is_same_prefix = self.last_prefix.as_deref() == Some(prefix);
                if !is_same_prefix {
                    self.key_hashes.push(filter_hash(prefix));
                    self.last_prefix = Some(prefix.to_vec());
                }
            }
        }
        // Add full-key hash if whole_key_filtering is enabled
        if self.whole_key_filtering {
            self.key_hashes.push(filter_hash(key));
        }
    }

    fn filter_size_bytes(num_keys: u32, bits_per_key: u32) -> usize {
        let filter_bits = num_keys * bits_per_key;
        // compute filter bytes rounded up to the number of bytes required to fit the filter
        filter_bits.div_ceil(8) as usize
    }

    fn build_filter(&mut self) -> BloomFilter {
        let num_probes = optimal_num_probes(self.bits_per_key);
        let key_hashes = std::mem::take(&mut self.key_hashes);
        let filter_bytes =
            BloomFilterBuilder::filter_size_bytes(key_hashes.len() as u32, self.bits_per_key);
        let filter_bits = (filter_bytes * 8) as u32;
        let mut buffer = vec![0x00; filter_bytes];
        for k in key_hashes {
            let probes = probes_for_key(k, num_probes, filter_bits);
            for p in probes {
                set_bit(p as usize, &mut buffer)
            }
        }
        BloomFilter {
            num_probes,
            whole_key_filtering: self.whole_key_filtering,
            prefix_extractor: self.prefix_extractor.clone(),
            buffer: Bytes::from(buffer),
        }
    }
}

impl BloomFilter {
    pub fn decode(
        mut buf: &[u8],
        whole_key_filtering: bool,
        prefix_extractor: Option<Arc<dyn PrefixExtractor>>,
    ) -> BloomFilter {
        let num_probes = buf.get_u16();
        BloomFilter {
            num_probes,
            whole_key_filtering,
            prefix_extractor,
            buffer: Bytes::copy_from_slice(buf),
        }
    }

    /// Estimates the size in bytes that [`Filter::encode`] will write for a
    /// bloom filter with `num_keys` entries.
    ///
    /// This is the per-filter payload size only. It does not include composite
    /// block framing (name, data-length prefix) or the per-composite-block CRC32
    /// checksum, which are accounted for at the SST level.
    pub(crate) fn estimate_encoded_size(num_keys: u32, filter_bits_per_key: u32) -> usize {
        let filter_bytes = BloomFilterBuilder::filter_size_bytes(num_keys, filter_bits_per_key);
        let num_probes_size = size_of::<u16>();
        filter_bytes + num_probes_size
    }

    fn filter_bits(&self) -> u32 {
        (self.buffer.len() * 8) as u32
    }

    fn might_contain(&self, hash: u64) -> bool {
        // A filter built with zero extracted hashes has zero bits: nothing can
        // match, and probing it would divide by zero in probes_for_key.
        if self.buffer.is_empty() {
            return false;
        }
        for p in probes_for_key(hash, self.num_probes, self.filter_bits()) {
            if !check_bit(p as usize, &self.buffer) {
                return false;
            }
        }
        true
    }
}

impl FilterBuilder for BloomFilterBuilder {
    fn add_entry(&mut self, entry: &RowEntry) {
        self.add_key(&entry.key);
    }

    fn build(&mut self) -> Arc<dyn Filter> {
        Arc::new(self.build_filter())
    }
}

impl Filter for BloomFilter {
    fn might_match(&self, query: &FilterQuery) -> bool {
        // Full-key hash gives the tightest answer whenever it was stored.
        if let (FilterTarget::Point(key), true) = (&query.target, self.whole_key_filtering) {
            return self.might_contain(filter_hash(key.as_ref()));
        }

        // Otherwise defer to the extractor.
        //   - For `Point` with whole_key_filtering=false: we probe with the
        //     extracted prefix of the queried key. If the prefix is in the
        //     filter, the full key might be present; if not, it cannot be.
        //   - For `Prefix`: the extractor answers whether the scan prefix is
        //     safe to probe. Returning `None` (e.g., the scan prefix is
        //     shorter than the extractor's output, or the extractor can't
        //     make a truncation-safe guarantee) forces us to return `true`.
        let Some(ref extractor) = self.prefix_extractor else {
            return true;
        };
        // Hashes of keys and prefixes only, so a range is unanswerable here.
        let Some(prefix_target) = query.target.as_prefix_target() else {
            return true;
        };
        let Some(n) = extractor.prefix_len(&prefix_target) else {
            return true;
        };
        let bytes = match &prefix_target {
            PrefixTarget::Point(k) => k.as_ref(),
            PrefixTarget::Prefix(p) => p.as_ref(),
        };
        self.might_contain(filter_hash(&bytes[..n]))
    }

    fn encode(&self, writer: &mut dyn BufMut) {
        writer.put_u16(self.num_probes);
        writer.put_slice(&self.buffer);
    }

    fn size(&self) -> usize {
        self.buffer.len()
    }

    fn clamp_allocated_size(&self) -> Arc<dyn Filter> {
        Arc::new(BloomFilter {
            num_probes: self.num_probes,
            whole_key_filtering: self.whole_key_filtering,
            prefix_extractor: self.prefix_extractor.clone(),
            buffer: clamp_allocated_size_bytes(&self.buffer),
        })
    }
}

fn filter_hash(key: &[u8]) -> u64 {
    // sip hash is the default rust hash function, however its only
    // accessible by creating DefaultHasher. Direct use of SipHasher13 in
    // std is deprecated. We don't want to use DefaultHasher because the
    // underlying algorithm could change. Therefore, we use SipHasher13 from
    // the siphasher crate
    let hasher = SipHasher13::new();
    hasher.hash(key)
}

fn probes_for_key(key_hash: u64, num_probes: u16, filter_bits: u32) -> Vec<u32> {
    // implements enhanced double hashing from:
    // https://www.khoury.northeastern.edu/~pete/pub/bloom-filters-verification.pdf
    // as suggested by the author P. Dillinger for RocksDB's legacy filters here:
    // https://github.com/facebook/rocksdb/issues/4120
    let mut probes = vec![0u32; num_probes as usize];
    let filter_bits = filter_bits as u64;
    let mut h = ((key_hash << 32) >> 32) % filter_bits; // lower 32 bits of hash
    let mut delta = (key_hash >> 32) % filter_bits; // higher 32 bits of hash
    for i in 0..num_probes {
        delta = (delta + i as u64) % filter_bits;
        probes[i as usize] = h as u32;
        h = (h + delta) % filter_bits;
    }
    probes
}

fn check_bit(bit: usize, buf: &[u8]) -> bool {
    let byte = bit / 8;
    let bit_in_byte = bit % 8;
    (buf[byte] & (1 << bit_in_byte)) != 0
}

fn set_bit(bit: usize, buf: &mut [u8]) {
    let byte = bit / 8;
    let bit_in_byte = bit % 8;
    buf[byte] |= 1 << bit_in_byte;
}

fn optimal_num_probes(bits_per_key: u32) -> u16 {
    // bits_per_key * ln(2)
    // https://en.wikipedia.org/wiki/Bloom_filter#Optimal_number_of_hash_functions
    (bits_per_key as f32 * 0.69) as u16
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;
    use std::ops::Bound;

    fn point_builder(bits_per_key: u32) -> BloomFilterBuilder {
        BloomFilterBuilder::new(bits_per_key, true, None)
    }

    #[test]
    fn test_set_specified_bit_only() {
        // some hard-coded test cases
        let cases = [
            (
                vec![0xF0u8, 0xABu8, 0x9Cu8],
                vec![0xF8u8, 0xABu8, 0x9Cu8],
                3,
            ),
            (
                vec![0xF0u8, 0xABu8, 0x9Cu8],
                vec![0xF0u8, 0xAFu8, 0x9Cu8],
                10,
            ),
        ];
        for (buf, expected, bit) in cases.iter() {
            let mut updated = buf.clone();
            set_bit(*bit, &mut updated);
            assert_eq!(updated, *expected);
        }
        // more of a property style test
        let nbytes = 4;
        for byte in 0..nbytes {
            for i in 0..8 {
                let mut buf = vec![0u8; nbytes];
                let bit = byte * 8 + i;
                set_bit(bit, &mut buf);
                for unset in 0..nbytes {
                    if unset != byte {
                        assert_eq!(buf[unset], 0)
                    } else {
                        assert_eq!(buf[byte], 1 << i);
                    }
                }
            }
        }
    }

    #[test]
    fn test_set_bits_doesnt_unset_bits() {
        let mut buf = vec![0xFFu8; 3];
        for i in 0..24 {
            set_bit(i, &mut buf);
            assert_eq!(buf, vec![0xFFu8; 3]);
        }
    }

    #[test]
    fn test_check_bits() {
        let num_bytes = 4;
        for i in 0..num_bytes {
            for b in 0..8 {
                let bit = i * 8 + b;
                let mut buf = vec![0u8; num_bytes];
                buf[i] = 1 << b;
                for checked in 0..num_bytes * 8 {
                    let bit_on = check_bit(checked, buf.as_slice());
                    assert_eq!(bit_on, bit == checked);
                }
            }
        }
    }

    #[test]
    fn test_compute_probes() {
        // h1 = 0xDEADBEEF, h2 = 0xDF77EF56
        let hash = 0xDF77EF56DEADBEEFu64;
        let probes = probes_for_key(hash, 7, 1000000);
        assert_eq!(
            probes,
            vec![
                928559, // h1
                107781, // h1 + h2
                287004, // h1 + h2 + h2 + 1
                466229, // h1 + h2 + h2 + 1 + h2 + 1 + 2
                645457, // h1 + h2 + h2 + 1 + h2 + 1 + 2 + h2 + 1 + 2 + 3
                824689, 3926,
            ]
        );
    }

    #[test]
    fn test_filter_effective() {
        let keys_to_test = 100000;
        let key_sz = size_of::<u32>();
        let mut builder = point_builder(10);
        for i in 0..keys_to_test {
            let mut bytes = BytesMut::with_capacity(key_sz);
            bytes.reserve(key_sz);
            bytes.put_u32(i);
            builder.add_key(&bytes.freeze());
        }
        let filter = builder.build_filter();

        // check all entries in filter
        for i in 0..keys_to_test {
            let mut bytes = BytesMut::with_capacity(key_sz);
            bytes.reserve(key_sz);
            bytes.put_u32(i);
            let hash = filter_hash(bytes.freeze().as_ref());
            assert!(filter.might_contain(hash));
        }

        // check false positives
        let mut fp = 0;
        for i in keys_to_test..2 * keys_to_test {
            let mut bytes = BytesMut::with_capacity(key_sz);
            bytes.reserve(key_sz);
            bytes.put_u32(i);
            let hash = filter_hash(bytes.freeze().as_ref());
            if filter.might_contain(hash) {
                fp += 1;
            }
        }

        // observed fp is .0087
        assert!((fp as f32 / keys_to_test as f32) < 0.01);
    }

    #[test]
    fn test_bloom_filter_size() {
        let mut builder = point_builder(10);
        builder.add_key(&Bytes::from_static(b"test_key"));
        let filter = builder.build_filter();

        // The exact size may vary, so we'll check if it's greater than zero
        assert!(
            filter.size() > 0,
            "Bloom filter size should be greater than zero"
        );

        // We can also check if the size matches the buffer length
        assert_eq!(
            filter.size(),
            filter.buffer.len(),
            "Size should match buffer length"
        );
    }

    #[test]
    fn test_should_clamp_allocated_bytes() {
        let mut builder = point_builder(10);
        for i in 0..100 {
            builder.add_key(&Bytes::from(format!("{}", i)));
        }
        let filter = builder.build_filter();
        let original_size = filter.size();
        let mut extended_buf = BytesMut::with_capacity(original_size + 100);
        extended_buf.put(filter.buffer.as_ref());
        extended_buf.put_bytes(0u8, 100);
        let filter = BloomFilter {
            buffer: extended_buf.freeze().slice(0..filter.buffer.len()),
            ..filter
        };

        let clamped = Filter::clamp_allocated_size(&filter);

        assert_eq!(clamped.size(), Filter::size(&filter));
        // Encode both and verify they produce the same bytes
        let mut original_bytes = Vec::new();
        Filter::encode(&filter, &mut original_bytes);
        let mut clamped_bytes = Vec::new();
        clamped.encode(&mut clamped_bytes);
        assert_eq!(original_bytes, clamped_bytes);
    }

    #[test]
    fn test_estimate_encoded_size() {
        // With zero keys the only overhead is the 2-byte num_probes header.
        assert_eq!(BloomFilter::estimate_encoded_size(0, 10), 2);

        // Test with one key
        let bits_per_key = 10;
        let filter_bytes = BloomFilterBuilder::filter_size_bytes(1, bits_per_key);
        let expected_size = filter_bytes + 2; // filter_bytes + num_probes header
        assert_eq!(
            BloomFilter::estimate_encoded_size(1, bits_per_key),
            expected_size
        );

        // Test with multiple keys
        let num_keys = 100;
        let bits_per_key = 10;
        let filter_bytes = BloomFilterBuilder::filter_size_bytes(num_keys, bits_per_key);
        let expected_size = filter_bytes + 2; // filter_bytes + num_probes header
        assert_eq!(
            BloomFilter::estimate_encoded_size(num_keys, bits_per_key),
            expected_size
        );

        // Test with large number of keys
        let num_keys = 100_000_000;
        let bits_per_key = 10;
        let filter_bytes = BloomFilterBuilder::filter_size_bytes(num_keys, bits_per_key);
        let expected_size = filter_bytes + 2; // filter_bytes + num_probes header
        assert_eq!(
            BloomFilter::estimate_encoded_size(num_keys, bits_per_key),
            expected_size
        );
    }

    /// Extracts a fixed 4-byte prefix; shorter targets yield no prefix.
    struct GatedFixed4;

    impl PrefixExtractor for GatedFixed4 {
        fn name(&self) -> &str {
            "gated_fixed_4"
        }

        fn prefix_len(&self, target: &PrefixTarget) -> Option<usize> {
            let bytes = match target {
                PrefixTarget::Point(k) => k.as_ref(),
                PrefixTarget::Prefix(p) => p.as_ref(),
            };
            (bytes.len() >= 4).then_some(4)
        }
    }

    #[test]
    fn test_prefix_only_filter_with_no_extracted_prefixes() {
        // Zero extracted prefixes + whole-key filtering off = zero-bit filter.
        // Probing it must not panic, and a miss is safe: nothing was hashed in.
        let mut builder = BloomFilterBuilder::new(10, false, Some(Arc::new(GatedFixed4)));
        builder.add_key(&Bytes::from_static(b"a"));
        builder.add_key(&Bytes::from_static(b"b"));
        let filter = builder.build_filter();
        assert!(filter.buffer.is_empty());

        assert!(!filter.might_match(&FilterQuery::prefix(Bytes::from_static(b"aaaa"))));
        assert!(!filter.might_match(&FilterQuery::point(Bytes::from_static(b"aaaa_key"))));

        // Queries the extractor rejects never reach the filter and must keep
        // reporting "might match", so the stored short keys stay reachable.
        assert!(filter.might_match(&FilterQuery::prefix(Bytes::from_static(b"a"))));
        assert!(filter.might_match(&FilterQuery::point(Bytes::from_static(b"a"))));
    }

    #[test]
    fn test_bloom_filter_abstains_on_range_queries() {
        let mut builder = BloomFilterBuilder::new(10, false, Some(Arc::new(GatedFixed4)));
        builder.add_key(&Bytes::from_static(b"aaaa_key"));
        let filter = builder.build_filter();

        // The filter has real content: a prefix it never saw is rejected.
        assert!(!filter.might_match(&FilterQuery::prefix(Bytes::from_static(b"bbbb"))));

        // It stores hashes of keys and prefixes, never ranges, so a range
        // query must abstain rather than reject. A wrong `false` here would
        // skip an SST holding rows the scan must see.
        assert!(filter.might_match(&FilterQuery::range(
            Bound::Included(Bytes::from_static(b"bbbb")),
            Bound::Excluded(Bytes::from_static(b"bbbc")),
        )));
    }

    #[test]
    fn test_combined_filter_with_no_extracted_prefixes() {
        // With whole-key filtering on, every key hashes into the filter even
        // when the extractor yields nothing, so the empty-filter guard never fires.
        let mut builder = BloomFilterBuilder::new(10, true, Some(Arc::new(GatedFixed4)));
        builder.add_key(&Bytes::from_static(b"a"));
        builder.add_key(&Bytes::from_static(b"b"));
        let filter = builder.build_filter();
        assert!(!filter.buffer.is_empty());

        // Point lookups take the whole-key path and find the stored keys.
        assert!(filter.might_match(&FilterQuery::point(Bytes::from_static(b"a"))));
        assert!(filter.might_match(&FilterQuery::point(Bytes::from_static(b"b"))));
    }
}
