use std::ops::Bound;
use std::sync::Arc;

use bytes::{BufMut, Bytes};

use crate::filter::{BloomFilter, BloomFilterBuilder};
use crate::prefix_extractor::{PrefixExtractor, PrefixTarget};
use crate::types::RowEntry;

/// A named, configurable filter policy.
///
/// Each policy produces a filter per SST during construction and can decode
/// previously written filters during reads. The engine stores the policy's
/// [`name`](FilterPolicy::name) alongside filter data so readers can match
/// stored filters to the correct policy.
pub trait FilterPolicy: Send + Sync {
    /// An identifier for this policy. Stored per-SST so the reader knows
    /// whether it can decode and use the filter block.
    ///
    /// The name should encode anything that affects **compatibility** —
    /// i.e., anything that would make a filter unreadable or produce wrong
    /// results if mismatched. For the built-in bloom filter:
    /// - `bits_per_key` is not included — changing it affects quality (FP
    ///   rate) but not the encoding format, so any reader can decode any
    ///   bloom filter regardless of bits_per_key.
    /// - `prefix_extractor` name is included — it changes which hashes are
    ///   stored, so querying with a different extractor produces false
    ///   negatives.
    fn name(&self) -> &str;

    /// Creates a new builder for constructing a filter.
    fn builder(&self) -> Box<dyn FilterBuilder>;

    /// Decodes a previously encoded filter.
    ///
    /// The engine matches the policy name stored in the composite filter
    /// block against `self.name()` before calling this to ensure the policy
    /// can deserialize the data.
    fn decode(&self, data: &[u8]) -> Arc<dyn Filter>;

    /// Estimates the encoded size in bytes for a filter with `num_keys` keys.
    ///
    /// This is a hint used by the SST builder to reserve buffer space before
    /// the filter is built. It does not need to be exact.
    fn estimate_size(&self, num_keys: usize) -> usize;

    /// Whether filters from this policy can answer [`FilterTarget::Range`]
    /// queries.
    ///
    /// A plain range scan reads an SST's filters only when some registered
    /// policy says yes here. Defaults to `false`, so a policy that indexes
    /// keys alone, such as a bloom filter, costs a range scan nothing.
    fn supports_range_queries(&self) -> bool {
        false
    }
}

/// Accumulator for entries during SST construction that produces a [`Filter`].
pub trait FilterBuilder: Send {
    /// Feeds an SST entry to the filter being built.
    ///
    /// The builder receives the full `RowEntry`so that filter implementations
    /// are not limited to key-only hashing. A bloom filter will typically hash
    /// only the key (or a prefix of it), but other filters (e.g. a min/max
    /// timestamp filter) can inspect whichever fields they need.
    fn add_entry(&mut self, entry: &RowEntry);

    /// Finalizes and returns the completed filter.
    fn build(&mut self) -> Arc<dyn Filter>;
}

/// A read-only filter that answers membership queries.
pub trait Filter: Send + Sync {
    /// Returns `true` if the filter cannot rule out the query.
    /// A return value of `false` guarantees no matching key exists.
    ///
    /// Filters that cannot answer a particular query kind should return
    /// `true` to avoid false negatives.
    fn might_match(&self, query: &FilterQuery) -> bool;

    /// Serializes the filter into the provided buffer.
    fn encode(&self, writer: &mut dyn BufMut);

    /// Returns the size of the filter's data in bytes.
    fn size(&self) -> usize;

    /// Returns a copy with over-allocated memory reclaimed.
    ///
    /// Used by the block cache to reduce memory waste from `Bytes` slices
    /// that reference larger buffers.
    fn clamp_allocated_size(&self) -> Arc<dyn Filter>;
}

/// A membership query passed to [`Filter::might_match`].
#[derive(Clone, Debug)]
pub struct FilterQuery {
    /// The target of this query (a specific key, a prefix, or a range).
    pub target: FilterTarget,
    /// Opaque, caller-supplied context forwarded from
    /// [`crate::config::ScanOptions::filter_context`] or
    /// [`crate::config::ReadOptions::filter_context`].
    ///
    /// Custom filter policies read this to parametrize their evaluation
    /// (e.g., a min/max version filter expects two `u64`s encoded in the
    /// `Inline` variant). Built-in filters (including `BloomFilterPolicy`)
    /// ignore this field.
    pub context: Option<FilterContext>,
}

/// The target of a filter query.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FilterTarget {
    /// Tests whether a specific key might exist in the SST.
    Point(Bytes),
    /// Tests whether any key with the given prefix might exist in the SST.
    Prefix(Bytes),
    /// Tests whether any key within the given bounds might exist in the SST.
    Range {
        lower: Bound<Bytes>,
        upper: Bound<Bytes>,
    },
}

impl FilterTarget {
    /// The equivalent [`PrefixTarget`], or `None` for a target a
    /// [`PrefixExtractor`] cannot describe.
    pub fn as_prefix_target(&self) -> Option<PrefixTarget> {
        match self {
            Self::Point(key) => Some(PrefixTarget::Point(key.clone())),
            Self::Prefix(prefix) => Some(PrefixTarget::Prefix(prefix.clone())),
            Self::Range { .. } => None,
        }
    }
}

impl FilterQuery {
    /// Creates a point-lookup query for the given key.
    pub fn point(key: Bytes) -> Self {
        Self {
            target: FilterTarget::Point(key),
            context: None,
        }
    }

    /// Creates a prefix-scan query for the given prefix.
    pub fn prefix(prefix: Bytes) -> Self {
        Self {
            target: FilterTarget::Prefix(prefix),
            context: None,
        }
    }

    /// Creates a range-scan query over the given bounds.
    pub fn range(lower: Bound<Bytes>, upper: Bound<Bytes>) -> Self {
        Self {
            target: FilterTarget::Range { lower, upper },
            context: None,
        }
    }

    /// Attaches caller-supplied context to this query.
    pub fn with_context(mut self, context: Option<FilterContext>) -> Self {
        self.context = context;
        self
    }
}

/// Caller-supplied opaque context forwarded to custom filter policies at
/// query time.
///
/// Carries raw bytes that a custom filter policy knows how to decode.
/// Built-in policies ignore this entirely.
///
/// Marked `#[non_exhaustive]` so new variants can be added (e.g., typed
/// variants) as concrete user use cases emerge, without it being a breaking
/// change.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum FilterContext {
    /// Inline 64-byte payload with no heap allocation. Suitable for
    /// pairs of `u64`s, `u128`s, and other fixed-layout small structs.
    Inline([u8; 64]),
    /// Variable-length, heap-allocated payload. Used when the encoded
    /// context is too large for [`FilterContext::Inline`] or when the
    /// length is dynamic, and across FFI boundaries that cannot represent
    /// fixed-size arrays.
    Bytes(Bytes),
}

// ---------------------------------------------------------------------------
// NamedFilter — a filter paired with its policy name
// ---------------------------------------------------------------------------

/// A filter paired with the name of the policy that produced it.
#[derive(Clone)]
pub(crate) struct NamedFilter {
    pub(crate) name: String,
    pub(crate) filter: Arc<dyn Filter>,
}

impl std::fmt::Debug for NamedFilter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "NamedFilter({})", self.name)
    }
}

// ---------------------------------------------------------------------------
// Built-in implementation: BloomFilterPolicy
// ---------------------------------------------------------------------------

/// A filter policy backed by the existing bloom filter implementation.
///
/// Supports full-key filtering (point lookups) and optional prefix filtering
/// when a [`PrefixExtractor`] is configured.
pub struct BloomFilterPolicy {
    bits_per_key: u32,
    whole_key_filtering: bool,
    prefix_extractor: Option<Arc<dyn PrefixExtractor>>,
    name: String,
}

// Manual Debug impl because `dyn PrefixExtractor` doesn't implement Debug,
// so #[derive(Debug)] won't compile. We surface the extractor's name instead.
impl std::fmt::Debug for BloomFilterPolicy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BloomFilterPolicy")
            .field("bits_per_key", &self.bits_per_key)
            .field("whole_key_filtering", &self.whole_key_filtering)
            .field("name", &self.name)
            .field(
                "prefix_extractor",
                &self.prefix_extractor.as_ref().map(|e| e.name()),
            )
            .finish()
    }
}

impl BloomFilterPolicy {
    /// The canonical name for bloom filter policies.
    pub const NAME: &'static str = "_bf";

    /// Creates a new bloom filter policy with the given bits per key.
    ///
    /// By default, whole-key filtering is enabled and no prefix extractor
    /// is configured.
    pub fn new(bits_per_key: u32) -> Self {
        let whole_key_filtering = true;
        let prefix_extractor: Option<Arc<dyn PrefixExtractor>> = None;
        Self {
            bits_per_key,
            name: Self::compose_name(whole_key_filtering, prefix_extractor.as_deref()),
            whole_key_filtering,
            prefix_extractor,
        }
    }

    /// Configures a prefix extractor for prefix-based bloom filtering.
    ///
    /// When set, the bloom filter hashes both extracted prefixes and (if
    /// `whole_key_filtering` is enabled) full keys. Prefix scans can then
    /// probe the filter to skip SSTs that contain no matching prefixes.
    ///
    /// The extractor's name is included in the policy name to ensure that
    /// filters built with different extractors are not mismatched.
    pub fn with_prefix_extractor(mut self, extractor: Arc<dyn PrefixExtractor>) -> Self {
        self.prefix_extractor = Some(extractor);
        self.name = Self::compose_name(self.whole_key_filtering, self.prefix_extractor.as_deref());
        self
    }

    /// Controls whether full keys are hashed into the bloom filter.
    ///
    /// When `true` (the default), point lookups can use the filter. Set
    /// to `false` if only prefix scans are needed and you want to reduce
    /// filter size.
    pub fn with_whole_key_filtering(mut self, enabled: bool) -> Self {
        self.whole_key_filtering = enabled;
        self.name = Self::compose_name(self.whole_key_filtering, self.prefix_extractor.as_deref());
        self
    }

    fn compose_name(
        whole_key_filtering: bool,
        prefix_extractor: Option<&dyn PrefixExtractor>,
    ) -> String {
        let mut name = Self::NAME.to_string();
        if let Some(extractor) = prefix_extractor {
            name.push_str(":p=");
            name.push_str(extractor.name());
        }
        if !whole_key_filtering {
            name.push_str(":wh=0");
        }
        name
    }

    /// Returns the bits per key setting.
    pub fn bits_per_key(&self) -> u32 {
        self.bits_per_key
    }
}

impl FilterPolicy for BloomFilterPolicy {
    fn name(&self) -> &str {
        &self.name
    }

    fn builder(&self) -> Box<dyn FilterBuilder> {
        Box::new(BloomFilterBuilder::new(
            self.bits_per_key,
            self.whole_key_filtering,
            self.prefix_extractor.clone(),
        ))
    }

    fn decode(&self, data: &[u8]) -> Arc<dyn Filter> {
        Arc::new(BloomFilter::decode(
            data,
            self.whole_key_filtering,
            self.prefix_extractor.clone(),
        ))
    }

    fn estimate_size(&self, num_keys: usize) -> usize {
        let num_keys = u32::try_from(num_keys).expect("num_keys should fit in u32");
        BloomFilter::estimate_encoded_size(num_keys, self.bits_per_key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ValueDeletable;

    fn make_entry(key: &[u8]) -> RowEntry {
        RowEntry::new(
            Bytes::copy_from_slice(key),
            ValueDeletable::Value(Bytes::from_static(b"val")),
            0,
            None,
            None,
        )
    }

    #[test]
    fn test_bloom_filter_policy_round_trip() {
        let policy = BloomFilterPolicy::new(10);
        assert_eq!(policy.name(), BloomFilterPolicy::NAME);

        // Build a filter via the policy
        let mut builder = policy.builder();
        for i in 0u32..1000 {
            builder.add_entry(&make_entry(&i.to_be_bytes()));
        }
        let filter = builder.build();

        // Encode and decode
        let mut buf = Vec::new();
        filter.encode(&mut buf);
        let decoded = policy.decode(&buf);

        // Point queries: all inserted keys should match
        for i in 0u32..1000 {
            let query = FilterQuery::point(Bytes::copy_from_slice(&i.to_be_bytes()));
            assert!(decoded.might_match(&query), "false negative for key {}", i);
        }

        // Prefix queries: should return true (inapplicable, no prefix extractor)
        let prefix_query = FilterQuery::prefix(Bytes::from_static(b"anything"));
        assert!(decoded.might_match(&prefix_query));
    }

    #[test]
    fn test_bloom_filter_policy_estimate_size() {
        let policy = BloomFilterPolicy::new(10);
        let size = policy.estimate_size(1000);
        assert!(size > 0);
    }

    #[test]
    fn test_bloom_filter_policy_false_positives() {
        let policy = BloomFilterPolicy::new(10);
        let mut builder = policy.builder();
        let num_keys = 100_000u32;
        for i in 0..num_keys {
            builder.add_entry(&make_entry(&i.to_be_bytes()));
        }
        let filter = builder.build();

        // Check false positive rate on keys NOT in the filter
        let mut fp = 0u32;
        for i in num_keys..2 * num_keys {
            let query = FilterQuery::point(Bytes::copy_from_slice(&i.to_be_bytes()));
            if filter.might_match(&query) {
                fp += 1;
            }
        }
        let fpr = fp as f64 / num_keys as f64;
        assert!(fpr < 0.01, "false positive rate too high: {}", fpr);
    }

    #[test]
    fn test_clamp_allocated_size() {
        let policy = BloomFilterPolicy::new(10);
        let mut builder = policy.builder();
        for i in 0u32..100 {
            builder.add_entry(&make_entry(&i.to_be_bytes()));
        }
        let filter = builder.build();
        let clamped = filter.clamp_allocated_size();
        assert_eq!(filter.size(), clamped.size());
    }

    /// A fixed-length prefix extractor for testing.
    struct FixedPrefixExtractor {
        len: usize,
        name: String,
    }

    impl FixedPrefixExtractor {
        fn new(len: usize) -> Self {
            Self {
                len,
                name: format!("fixed{}", len),
            }
        }
    }

    impl PrefixExtractor for FixedPrefixExtractor {
        fn name(&self) -> &str {
            &self.name
        }

        fn prefix_len(&self, target: &PrefixTarget) -> Option<usize> {
            // A fixed-length extractor is truncation-safe: the first `len`
            // bytes fully determine the extracted prefix, so both the
            // `Point` and `Prefix` variants return the same answer. We
            // only require the input to be at least `len` bytes.
            let input = match target {
                PrefixTarget::Point(k) => k.as_ref(),
                PrefixTarget::Prefix(p) => p.as_ref(),
            };
            (input.len() >= self.len).then_some(self.len)
        }
    }

    #[test]
    fn test_prefix_bloom_filter_policy_name() {
        let extractor = Arc::new(FixedPrefixExtractor::new(3));
        let policy = BloomFilterPolicy::new(10).with_prefix_extractor(extractor);
        assert_eq!(policy.name(), "_bf:p=fixed3");
    }

    #[test]
    fn test_prefix_bloom_filter_round_trip() {
        let extractor = Arc::new(FixedPrefixExtractor::new(3));
        let policy = BloomFilterPolicy::new(10).with_prefix_extractor(extractor);

        // Build a filter with keys sharing prefixes "aaa" and "bbb"
        let mut builder = policy.builder();
        for i in 0u32..100 {
            let key = format!("aaa{:04}", i);
            builder.add_entry(&make_entry(key.as_bytes()));
        }
        for i in 0u32..100 {
            let key = format!("bbb{:04}", i);
            builder.add_entry(&make_entry(key.as_bytes()));
        }
        let filter = builder.build();

        // Encode and decode
        let mut buf = Vec::new();
        filter.encode(&mut buf);
        let decoded = policy.decode(&buf);

        // Point queries should still work
        let query = FilterQuery::point(Bytes::from_static(b"aaa0050"));
        assert!(decoded.might_match(&query));

        // Prefix query for "aaa" should match
        let query = FilterQuery::prefix(Bytes::from_static(b"aaa"));
        assert!(decoded.might_match(&query));

        // Prefix query for "bbb" should match
        let query = FilterQuery::prefix(Bytes::from_static(b"bbb"));
        assert!(decoded.might_match(&query));

        // Prefix query for "ccc" (not inserted) should have low FP rate
        // (can't guarantee false for a single query, but it shouldn't be true
        // for ALL non-existent prefixes)
        let mut fp = 0;
        for c in b'c'..=b'z' {
            let prefix = vec![c, c, c];
            let query = FilterQuery::prefix(Bytes::from(prefix));
            if decoded.might_match(&query) {
                fp += 1;
            }
        }
        // With 10 bits per key and only ~2 prefix hashes, FP rate should be very low
        assert!(
            fp < 10,
            "too many false positives for prefix queries: {}",
            fp
        );
    }

    #[test]
    fn test_prefix_bloom_out_of_domain_returns_true() {
        let extractor = Arc::new(FixedPrefixExtractor::new(3));
        let policy = BloomFilterPolicy::new(10).with_prefix_extractor(extractor);

        let mut builder = policy.builder();
        builder.add_entry(&make_entry(b"aaa0001"));
        let filter = builder.build();

        let mut buf = Vec::new();
        filter.encode(&mut buf);
        let decoded = policy.decode(&buf);

        // Prefix shorter than extractor length is out-of-domain, must return true
        let query = FilterQuery::prefix(Bytes::from_static(b"aa"));
        assert!(decoded.might_match(&query));
    }

    #[test]
    fn test_no_prefix_extractor_returns_true_for_prefix_queries() {
        let policy = BloomFilterPolicy::new(10);

        let mut builder = policy.builder();
        builder.add_entry(&make_entry(b"aaa0001"));
        let filter = builder.build();

        let mut buf = Vec::new();
        filter.encode(&mut buf);
        let decoded = policy.decode(&buf);

        // Without a prefix extractor, prefix queries should always return true
        let query = FilterQuery::prefix(Bytes::from_static(b"aaa"));
        assert!(decoded.might_match(&query));
    }

    #[test]
    fn test_whole_key_filtering_disabled() {
        let extractor = Arc::new(FixedPrefixExtractor::new(3));
        let policy = BloomFilterPolicy::new(10)
            .with_prefix_extractor(extractor)
            .with_whole_key_filtering(false);

        let mut builder = policy.builder();
        for i in 0u32..1000 {
            let key = format!("aaa{:04}", i);
            builder.add_entry(&make_entry(key.as_bytes()));
        }
        let filter = builder.build();

        // Encode and decode so the decoded filter goes through BloomFilterPolicy::decode,
        // which is where the whole_key_filtering flag must be propagated correctly.
        let mut buf = Vec::new();
        filter.encode(&mut buf);
        let decoded = policy.decode(&buf);

        // Prefix query should still work
        let query = FilterQuery::prefix(Bytes::from_static(b"aaa"));
        assert!(decoded.might_match(&query));

        // With whole_key_filtering=false but an extractor configured, point
        // queries probe the filter using the extracted prefix of the key.
        // All stored keys begin with "aaa", so the prefix hash is present and
        // a point query for "aaa0001" must report "might match".
        let query = FilterQuery::point(Bytes::from_static(b"aaa0001"));
        assert!(decoded.might_match(&query));
    }

    /// With `whole_key_filtering=false` and a prefix extractor configured, a
    /// point lookup whose extracted prefix was never stored in the filter
    /// must be rejected. This is the `GroupId ‖ Suffix` workload: stored
    /// prefixes are group ids, and point lookups reuse those hashes.
    ///
    /// Filter size scales with the number of *stored* hashes (deduplicated
    /// prefix hashes here), so the test uses many distinct group ids to
    /// keep the filter non-trivially sized.
    #[test]
    fn test_point_lookup_via_extracted_prefix_rejects_absent_prefix() {
        let extractor = Arc::new(FixedPrefixExtractor::new(3));
        let policy = BloomFilterPolicy::new(10)
            .with_prefix_extractor(extractor)
            .with_whole_key_filtering(false);

        // 1000 distinct 3-byte group ids in the "A.." space, each with one
        // suffix. The builder stores 1000 distinct prefix hashes after dedup.
        let mut builder = policy.builder();
        for gid in 0u32..1000 {
            builder.add_entry(&make_entry(format!("A{:03}_row", gid).as_bytes()));
        }
        let filter = builder.build();

        // Every stored group id must probe as "might match".
        for gid in 0u32..1000 {
            let q = FilterQuery::point(Bytes::from(format!("A{:03}_row", gid)));
            assert!(filter.might_match(&q), "gid {} missing from filter", gid);
        }

        // Point lookups whose extracted 3-byte prefix lives in a disjoint
        // namespace ("B..") must be rejected at bloom-filter FP rate.
        let mut fp = 0;
        let mut total = 0;
        for gid in 0u32..1000 {
            let q = FilterQuery::point(Bytes::from(format!("B{:03}_row", gid)));
            total += 1;
            if filter.might_match(&q) {
                fp += 1;
            }
        }
        let fpr = fp as f64 / total as f64;
        assert!(
            fpr < 0.02,
            "point-via-prefix FPR should be low: got {} of {} ({})",
            fp,
            total,
            fpr,
        );
    }

    /// A scan prefix longer than the extractor's output is still safe to
    /// probe — the extractor can return a shorter length and the filter is
    /// probed with the truncated bytes. Stored key `"aaa0001"` hashes prefix
    /// `"aaa"`; a scan for `"aaa0"` must truncate to `"aaa"` and match.
    #[test]
    fn test_prefix_scan_truncates_over_length_scan_prefix() {
        let extractor = Arc::new(FixedPrefixExtractor::new(3));
        let policy = BloomFilterPolicy::new(10).with_prefix_extractor(extractor);

        let mut builder = policy.builder();
        builder.add_entry(&make_entry(b"aaa0001"));
        let filter = builder.build();

        // Over-length scan prefixes whose first 3 bytes match a stored prefix
        // must still "might match" after truncation.
        for scan in ["aaa", "aaa0", "aaa1234"] {
            let query = FilterQuery::prefix(Bytes::copy_from_slice(scan.as_bytes()));
            assert!(
                filter.might_match(&query),
                "scan_prefix({scan:?}) should match after truncation",
            );
        }
    }

    /// A selective extractor indexes only a subset of keys — e.g., `user:*`
    /// but not `post:*`. During build, keys outside the domain return `None`
    /// from `prefix_len(Point(_))` and their prefix is never hashed; during
    /// reads, a prefix scan outside the domain returns `None` as well and
    /// the filter reports "might match" (cannot rule out). Returning `Some`
    /// for an out-of-domain `Prefix` would violate the trait's truncation
    /// invariant and produce false negatives for stored keys in that namespace.
    #[test]
    fn test_selective_extractor_indexes_only_matching_keys() {
        struct UserOnlyExtractor;
        impl PrefixExtractor for UserOnlyExtractor {
            fn name(&self) -> &str {
                "user-only"
            }
            fn prefix_len(&self, target: &PrefixTarget) -> Option<usize> {
                let input = match target {
                    PrefixTarget::Point(k) => k.as_ref(),
                    PrefixTarget::Prefix(p) => p.as_ref(),
                };
                (input.len() >= 5 && input.starts_with(b"user:")).then_some(5)
            }
        }

        let policy = BloomFilterPolicy::new(10).with_prefix_extractor(Arc::new(UserOnlyExtractor));

        let mut builder = policy.builder();
        for i in 0u32..10 {
            builder.add_entry(&make_entry(format!("user:u{:02}", i).as_bytes()));
        }
        for i in 0u32..10 {
            builder.add_entry(&make_entry(format!("post:p{:02}", i).as_bytes()));
        }
        let filter = builder.build();

        // Prefix scan on the indexed namespace finds the stored prefix.
        let q = FilterQuery::prefix(Bytes::from_static(b"user:"));
        assert!(filter.might_match(&q));

        // Prefix scan on a non-indexed namespace — extractor returns `None`,
        // so the filter returns true (inapplicable). This is crucial: the
        // `post:` prefix was never hashed during build, so returning
        // `Some(5)` here would let the filter flag the SST as absent and
        // skip real `post:` data. The `None` return keeps us correct.
        let q = FilterQuery::prefix(Bytes::from_static(b"post:"));
        assert!(filter.might_match(&q));
        let q = FilterQuery::prefix(Bytes::from_static(b"session:"));
        assert!(filter.might_match(&q));

        // Whole-key filtering is still on (default), so every stored key
        // round-trips on point lookup regardless of namespace.
        for i in 0u32..10 {
            let q = FilterQuery::point(Bytes::from(format!("user:u{:02}", i)));
            assert!(filter.might_match(&q));
            let q = FilterQuery::point(Bytes::from(format!("post:p{:02}", i)));
            assert!(filter.might_match(&q));
        }
    }

    /// `add_key` asserts that an extractor respects the "returned length must
    /// be ≤ input length" contract. A misbehaving extractor trips the panic.
    #[test]
    #[should_panic(expected = "PrefixExtractor returned a prefix length")]
    fn test_add_key_panics_on_prefix_len_longer_than_key() {
        struct TooLongExtractor;
        impl PrefixExtractor for TooLongExtractor {
            fn name(&self) -> &str {
                "too-long"
            }
            fn prefix_len(&self, _target: &PrefixTarget) -> Option<usize> {
                Some(usize::MAX)
            }
        }

        let policy = BloomFilterPolicy::new(10).with_prefix_extractor(Arc::new(TooLongExtractor));
        let mut builder = policy.builder();
        builder.add_entry(&make_entry(b"short"));
    }

    /// A "last-delimiter" extractor depends on bytes beyond any proper
    /// prefix, so it cannot make a truncation-safe promise for `Prefix`
    /// inputs. The trait contract says such an extractor must return `None`
    /// for `PrefixTarget::Prefix`; the bloom filter then conservatively
    /// returns `true` for prefix scans. `Point` inputs still work.
    #[test]
    fn test_last_delimiter_extractor_skips_prefix_scan() {
        struct LastColonExtractor;
        impl PrefixExtractor for LastColonExtractor {
            fn name(&self) -> &str {
                "last-colon"
            }
            fn prefix_len(&self, target: &PrefixTarget) -> Option<usize> {
                match target {
                    // For a complete key, extract up to and including the last ':'.
                    PrefixTarget::Point(k) => {
                        k.as_ref().iter().rposition(|&b| b == b':').map(|i| i + 1)
                    }
                    // For a scan prefix, the final ':' could land anywhere in
                    // an unseen extension, so no truncation is safe.
                    PrefixTarget::Prefix(_) => None,
                }
            }
        }

        let policy = BloomFilterPolicy::new(10)
            .with_prefix_extractor(Arc::new(LastColonExtractor))
            .with_whole_key_filtering(false);

        let mut builder = policy.builder();
        builder.add_entry(&make_entry(b"ns1:item:abc"));
        builder.add_entry(&make_entry(b"ns1:item:xyz"));
        let filter = builder.build();

        // Point lookup: extractor returns Some(9) for "ns1:item:", and that
        // hash was stored during build. Must match.
        let q = FilterQuery::point(Bytes::from_static(b"ns1:item:abc"));
        assert!(filter.might_match(&q));

        // Prefix scan: extractor returns None, so the filter returns true
        // rather than risk a false negative.
        let q = FilterQuery::prefix(Bytes::from_static(b"ns1:item:"));
        assert!(filter.might_match(&q));
        let q = FilterQuery::prefix(Bytes::from_static(b"ns1:"));
        assert!(filter.might_match(&q));
    }

    /// Degenerate configuration: `whole_key_filtering=false` with no prefix
    /// extractor configured disables all filtering. `might_match` must
    /// unconditionally return `true` — there are no stored hashes to probe.
    #[test]
    fn test_no_filtering_configured_returns_true() {
        let policy = BloomFilterPolicy::new(10).with_whole_key_filtering(false);
        let mut builder = policy.builder();
        builder.add_entry(&make_entry(b"anything"));
        let filter = builder.build();

        assert!(filter.might_match(&FilterQuery::point(Bytes::from_static(b"anything"))));
        assert!(filter.might_match(&FilterQuery::point(Bytes::from_static(b"missing"))));
        assert!(filter.might_match(&FilterQuery::prefix(Bytes::from_static(b"any"))));
    }

    /// The `last_prefix` dedup in `BloomFilterBuilder` only fires for
    /// *consecutive* same-prefix keys — which is the sorted-input case SST
    /// construction provides. Non-consecutive duplicates re-hash, inflating
    /// the stored hash count and thus the filter size. This test pins that
    /// invariant: a future change to full set-based dedup would shrink the
    /// interleaved filter below the sorted one and fail the assertion.
    ///
    /// Uses `whole_key_filtering=false` so filter size depends *only* on
    /// unique prefix-hash dedup behavior, not on whole-key hashes.
    #[test]
    fn test_dedup_only_fires_on_consecutive_same_prefix() {
        let make_filter = |order: &[&[u8]]| {
            let policy = BloomFilterPolicy::new(10)
                .with_prefix_extractor(Arc::new(FixedPrefixExtractor::new(3)))
                .with_whole_key_filtering(false);
            let mut builder = policy.builder();
            for key in order {
                builder.add_entry(&make_entry(key));
            }
            builder.build()
        };

        let sorted = make_filter(&[b"aaa1", b"aaa2", b"bbb1", b"bbb2"]);
        let interleaved = make_filter(&[b"aaa1", b"bbb1", b"aaa2", b"bbb2"]);

        // Sorted: dedup keeps 2 prefix hashes ("aaa", "bbb").
        // Interleaved: dedup resets at each prefix flip, 4 stored hashes.
        // Filter bytes track hash count * bits_per_key, so interleaved is
        // strictly larger.
        assert!(
            interleaved.size() > sorted.size(),
            "expected interleaved filter to be larger than sorted (sorted={}, interleaved={}); \
             consecutive-only dedup invariant may have regressed",
            sorted.size(),
            interleaved.size(),
        );
    }

    /// `prefix_len` returning `Some(0)` (empty prefix) must not panic, and
    /// the filter must still report "might match" for the same empty prefix
    /// on read. The empty hash is degenerate but legal.
    #[test]
    fn test_prefix_len_zero_is_legal() {
        struct EmptyPrefixExtractor;
        impl PrefixExtractor for EmptyPrefixExtractor {
            fn name(&self) -> &str {
                "empty"
            }
            fn prefix_len(&self, _target: &PrefixTarget) -> Option<usize> {
                Some(0)
            }
        }

        let policy = BloomFilterPolicy::new(10)
            .with_prefix_extractor(Arc::new(EmptyPrefixExtractor))
            .with_whole_key_filtering(false);
        let mut builder = policy.builder();
        // Add 100 distinct entries — every one extracts the empty prefix.
        // The `last_prefix` dedup should collapse them to a single stored
        // hash. Filter size is derived from that hash count; at 10
        // bits/key with one hash, the payload is 10 bits → 2 bytes.
        for i in 0u32..100 {
            builder.add_entry(&make_entry(format!("entry{i}").as_bytes()));
        }
        let filter = builder.build();
        assert_eq!(
            filter.size(),
            2,
            "expected dedup to collapse the empty prefix to one stored hash",
        );

        // Every probe collapses to hash("") and matches. Degenerate but
        // not a panic or out-of-bounds slice.
        assert!(filter.might_match(&FilterQuery::point(Bytes::from_static(b"any"))));
        assert!(filter.might_match(&FilterQuery::prefix(Bytes::from_static(b"any"))));
    }

    /// `prefix_len` returning `Some(key.len())` means the extracted prefix is
    /// the full input. With `whole_key_filtering=true`, the prefix hash and
    /// whole-key hash are identical — not wasted bits exactly, but
    /// duplicates set the same probes. Must not panic and must round-trip.
    #[test]
    fn test_prefix_len_equals_full_key_length() {
        struct FullKeyExtractor;
        impl PrefixExtractor for FullKeyExtractor {
            fn name(&self) -> &str {
                "full-key"
            }
            fn prefix_len(&self, target: &PrefixTarget) -> Option<usize> {
                let input = match target {
                    PrefixTarget::Point(k) => k.as_ref(),
                    PrefixTarget::Prefix(p) => p.as_ref(),
                };
                Some(input.len())
            }
        }

        let policy = BloomFilterPolicy::new(10).with_prefix_extractor(Arc::new(FullKeyExtractor));
        let mut builder = policy.builder();
        builder.add_entry(&make_entry(b"key1"));
        builder.add_entry(&make_entry(b"key2"));
        let filter = builder.build();

        // Point lookup on stored key matches via whole-key hash.
        assert!(filter.might_match(&FilterQuery::point(Bytes::from_static(b"key1"))));
        // "Prefix" query whose length equals a stored key is probed with
        // hash of the full candidate — stored keys match, others don't
        // (modulo bloom FP).
        assert!(filter.might_match(&FilterQuery::prefix(Bytes::from_static(b"key1"))));
    }

    // ---------- Property tests ----------

    use proptest::collection::vec;
    use proptest::prelude::{any, proptest};

    proptest! {
        /// Every key inserted into a bloom filter under the default policy
        /// must be reported by `might_match(Point(..))`. This is the
        /// defining "no false negatives" invariant of bloom filters. The
        /// behavioral tests above spot-check it; this property explores
        /// many more shapes (empty keys, single-byte keys, long keys,
        /// non-UTF8 bytes, duplicates, unsorted input).
        #[test]
        fn prop_no_false_negatives_default_policy(
            keys in vec(vec(any::<u8>(), 0..32), 1..64),
        ) {
            let policy = BloomFilterPolicy::new(10);
            let mut builder = policy.builder();
            for key in &keys {
                builder.add_entry(&make_entry(key));
            }
            let filter = builder.build();
            for key in &keys {
                let q = FilterQuery::point(Bytes::copy_from_slice(key));
                assert!(
                    filter.might_match(&q),
                    "false negative for {:?}",
                    key,
                );
            }
        }

        /// With a `FixedPrefixExtractor(3)` configured, every stored key
        /// round-trips on point lookup AND every key's extracted prefix
        /// round-trips on prefix lookup. Keys are constrained to ≥ 3
        /// bytes so the extractor always extracts.
        #[test]
        fn prop_no_false_negatives_with_prefix_extractor(
            keys in vec(vec(any::<u8>(), 3..32), 1..64),
        ) {
            let policy = BloomFilterPolicy::new(10)
                .with_prefix_extractor(Arc::new(FixedPrefixExtractor::new(3)));
            let mut builder = policy.builder();
            for key in &keys {
                builder.add_entry(&make_entry(key));
            }
            let filter = builder.build();

            for key in &keys {
                // Full-key hash stored (default whole_key_filtering=true).
                let q = FilterQuery::point(Bytes::copy_from_slice(key));
                assert!(
                    filter.might_match(&q),
                    "point false negative for {:?}",
                    key,
                );

                // Extracted prefix hash stored — prefix query matches.
                let q = FilterQuery::prefix(Bytes::copy_from_slice(&key[..3]));
                assert!(
                    filter.might_match(&q),
                    "prefix false negative for {:?}",
                    &key[..3],
                );
            }
        }

        /// With `whole_key_filtering=false`, no full-key hashes are stored.
        /// A point lookup is resolved by probing the filter with the
        /// *extracted prefix* of the queried key. That means: for any key
        /// that *was* added to the filter, its point lookup must still
        /// match — because its prefix hash is there.
        #[test]
        fn prop_no_false_negatives_with_whole_key_filtering_disabled(
            keys in vec(vec(any::<u8>(), 3..32), 1..64),
        ) {
            let policy = BloomFilterPolicy::new(10)
                .with_prefix_extractor(Arc::new(FixedPrefixExtractor::new(3)))
                .with_whole_key_filtering(false);
            let mut builder = policy.builder();
            for key in &keys {
                builder.add_entry(&make_entry(key));
            }
            let filter = builder.build();

            for key in &keys {
                // Point lookup probes with the key's extracted prefix, which
                // was stored during build.
                let q = FilterQuery::point(Bytes::copy_from_slice(key));
                assert!(
                    filter.might_match(&q),
                    "point-via-prefix false negative for {:?}",
                    key,
                );

                // The extracted prefix itself also round-trips.
                let q = FilterQuery::prefix(Bytes::copy_from_slice(&key[..3]));
                assert!(
                    filter.might_match(&q),
                    "prefix false negative for {:?}",
                    &key[..3],
                );
            }
        }
    }

    #[test]
    fn test_filter_query_default_context_is_none() {
        let q = FilterQuery::point(Bytes::from_static(b"k"));
        assert!(q.context.is_none());
        let q = FilterQuery::prefix(Bytes::from_static(b"p"));
        assert!(q.context.is_none());
    }

    #[test]
    fn test_filter_query_with_inline_context() {
        let mut payload = [0u8; 64];
        payload[..8].copy_from_slice(&42u64.to_be_bytes());
        payload[8..16].copy_from_slice(&100u64.to_be_bytes());
        let ctx = FilterContext::Inline(payload);

        let query = FilterQuery::point(Bytes::from_static(b"k")).with_context(Some(ctx));
        match query.context {
            Some(FilterContext::Inline(buf)) => {
                assert_eq!(u64::from_be_bytes(buf[..8].try_into().unwrap()), 42);
                assert_eq!(u64::from_be_bytes(buf[8..16].try_into().unwrap()), 100);
            }
            other => panic!("expected Inline variant, got {:?}", other),
        }
    }
}
