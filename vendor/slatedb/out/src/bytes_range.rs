use bytes::Bytes;
use serde::Serialize;
use std::ops::Bound::{Excluded, Included, Unbounded};
use std::ops::{
    Bound, Range, RangeBounds, RangeFrom, RangeFull, RangeInclusive, RangeTo, RangeToInclusive,
};

use crate::comparable_range::{ComparableRange, EndBound, StartBound};

/// Concrete struct representing a range of Bytes. Gets around much of
/// the cumbersome work associated with the generic trait RangeBounds<Bytes>
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub(crate) struct BytesRange {
    inner: ComparableRange<Bytes>,
}

pub trait ByteRangeBounds {
    fn start_bound(&self) -> Bound<&[u8]>;
    fn end_bound(&self) -> Bound<&[u8]>;
}

fn bound_as_bytes<K: AsRef<[u8]>>(bound: Bound<&K>) -> Bound<&[u8]> {
    match bound {
        Included(k) => Included(k.as_ref()),
        Excluded(k) => Excluded(k.as_ref()),
        Unbounded => Unbounded,
    }
}

impl<K: AsRef<[u8]>> ByteRangeBounds for Range<K> {
    fn start_bound(&self) -> Bound<&[u8]> {
        Included(self.start.as_ref())
    }

    fn end_bound(&self) -> Bound<&[u8]> {
        Excluded(self.end.as_ref())
    }
}

impl<K: AsRef<[u8]>> ByteRangeBounds for RangeInclusive<K> {
    fn start_bound(&self) -> Bound<&[u8]> {
        bound_as_bytes(RangeBounds::start_bound(self))
    }

    fn end_bound(&self) -> Bound<&[u8]> {
        bound_as_bytes(RangeBounds::end_bound(self))
    }
}

impl<K: AsRef<[u8]>> ByteRangeBounds for RangeFrom<K> {
    fn start_bound(&self) -> Bound<&[u8]> {
        Included(self.start.as_ref())
    }

    fn end_bound(&self) -> Bound<&[u8]> {
        Unbounded
    }
}

impl<K: AsRef<[u8]>> ByteRangeBounds for RangeTo<K> {
    fn start_bound(&self) -> Bound<&[u8]> {
        Unbounded
    }

    fn end_bound(&self) -> Bound<&[u8]> {
        Excluded(self.end.as_ref())
    }
}

impl<K: AsRef<[u8]>> ByteRangeBounds for RangeToInclusive<K> {
    fn start_bound(&self) -> Bound<&[u8]> {
        Unbounded
    }

    fn end_bound(&self) -> Bound<&[u8]> {
        Included(self.end.as_ref())
    }
}

impl ByteRangeBounds for RangeFull {
    fn start_bound(&self) -> Bound<&[u8]> {
        Unbounded
    }

    fn end_bound(&self) -> Bound<&[u8]> {
        Unbounded
    }
}

impl ByteRangeBounds for BytesRange {
    fn start_bound(&self) -> Bound<&[u8]> {
        RangeBounds::start_bound(self).map(|b| b.as_ref())
    }

    fn end_bound(&self) -> Bound<&[u8]> {
        RangeBounds::end_bound(self).map(|b| b.as_ref())
    }
}

impl<T: AsRef<[u8]>> ByteRangeBounds for (Bound<T>, Bound<T>) {
    fn start_bound(&self) -> Bound<&[u8]> {
        match &self.0 {
            Included(v) => Included(v.as_ref()),
            Excluded(v) => Excluded(v.as_ref()),
            Unbounded => Unbounded,
        }
    }

    fn end_bound(&self) -> Bound<&[u8]> {
        match &self.1 {
            Included(v) => Included(v.as_ref()),
            Excluded(v) => Excluded(v.as_ref()),
            Unbounded => Unbounded,
        }
    }
}

impl Serialize for BytesRange {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.inner.serialize(serializer)
    }
}

#[cfg(test)]
// Checks for the annoying case when we have ("prefix", "prefix\0").
// When both bounds are excluded, the range is empty even though
// "prefix" < "prefix\0".
pub(crate) fn is_prefix_increment(prefix: &[u8], b: &[u8]) -> bool {
    if !b.starts_with(prefix.as_ref()) {
        return false;
    }

    b[prefix.len()..] == [u8::MIN]
}

impl RangeBounds<Bytes> for BytesRange {
    fn start_bound(&self) -> Bound<&Bytes> {
        self.inner.start_bound()
    }

    fn end_bound(&self) -> Bound<&Bytes> {
        self.inner.end_bound()
    }
}

impl BytesRange {
    pub(crate) fn new(start_bound: Bound<Bytes>, end_bound: Bound<Bytes>) -> Self {
        let inner = ComparableRange::new(start_bound, end_bound);
        assert!(inner.non_empty(), "Range must be non-empty");
        Self { inner }
    }

    /// Build a `BytesRange` without panicking on empty ranges. Returns `None`
    /// when the bounds describe an empty key interval.
    pub(crate) fn try_new(start_bound: Bound<Bytes>, end_bound: Bound<Bytes>) -> Option<Self> {
        let inner = ComparableRange::new(start_bound, end_bound);
        inner.non_empty().then_some(Self { inner })
    }

    pub(crate) fn unbounded() -> Self {
        Self::new(Unbounded, Unbounded)
    }

    pub(crate) fn new_empty() -> Self {
        Self {
            inner: ComparableRange::new(
                Excluded(Bytes::copy_from_slice(&[0_u8])),
                Excluded(Bytes::copy_from_slice(&[0_u8])),
            ),
        }
    }

    pub(crate) fn from<T: RangeBounds<Bytes>>(range: T) -> Self {
        Self::new(range.start_bound().cloned(), range.end_bound().cloned())
    }

    pub(crate) fn from_slice<'a, T>(range: T) -> Self
    where
        T: RangeBounds<&'a [u8]>,
    {
        Self::new(
            range.start_bound().map(|b| Bytes::copy_from_slice(b)),
            range.end_bound().map(|b| Bytes::copy_from_slice(b)),
        )
    }

    /// Build a half-open range `[prefix, prefix+1)` covering all keys that start with
    /// `prefix`.
    ///
    /// - Empty prefix returns `(..)` so callers can scan the entire keyspace.
    /// - If the prefix is all `0xff`, the end bound is unbounded rather than overflowing.
    pub(crate) fn from_prefix(prefix: &[u8]) -> Self {
        if prefix.is_empty() {
            return Self::new(Unbounded, Unbounded);
        }

        let start = Bytes::copy_from_slice(prefix);
        let end = Self::increment_prefix(prefix)
            .map(Excluded)
            .unwrap_or(Unbounded);
        Self::new(Included(start), end)
    }

    /// Build the range of keys that start with `prefix`, restricted to
    /// `subrange`. Subrange bounds are key *suffixes*: a bound `s` denotes
    /// the full key `prefix ++ s`. Because every key in the range shares the
    /// prefix, ordering suffixes is the same as ordering full keys.
    ///
    /// - An unbounded subrange start means "from the first key with this
    ///   prefix" (the bare prefix itself).
    /// - An unbounded subrange end means "to the end of the prefix's
    ///   keyspace" (exclusive at the incremented prefix, or unbounded when
    ///   the prefix is empty or all `0xff`).
    /// - `from_prefix_and_subrange(prefix, ..)` is equivalent to
    ///   [`Self::from_prefix`]; an empty prefix degenerates to a plain range
    ///   over the subrange bounds.
    pub(crate) fn from_prefix_and_subrange(prefix: &[u8], subrange: impl ByteRangeBounds) -> Self {
        let concat = |suffix: &[u8]| {
            let mut key = Vec::with_capacity(prefix.len() + suffix.len());
            key.extend_from_slice(prefix);
            key.extend_from_slice(suffix);
            Bytes::from(key)
        };
        let start = match subrange.start_bound() {
            Included(s) => Included(concat(s)),
            Excluded(s) => Excluded(concat(s)),
            Unbounded if prefix.is_empty() => Unbounded,
            Unbounded => Included(Bytes::copy_from_slice(prefix)),
        };
        let end = match subrange.end_bound() {
            Included(e) => Included(concat(e)),
            Excluded(e) => Excluded(concat(e)),
            Unbounded => Self::increment_prefix(prefix)
                .map(Excluded)
                .unwrap_or(Unbounded),
        };
        Self::new(start, end)
    }

    /// Compute the smallest byte string that is lexicographically greater than any key
    /// starting with `prefix`. Returns `None` when `prefix` is all `0xff`.
    fn increment_prefix(prefix: &[u8]) -> Option<Bytes> {
        let mut upper_bound = prefix.to_vec();

        for i in (0..upper_bound.len()).rev() {
            if upper_bound[i] != u8::MAX {
                upper_bound[i] += 1;
                upper_bound.truncate(i + 1);
                return Some(Bytes::from(upper_bound));
            }
        }

        None
    }

    #[cfg(test)]
    pub(crate) fn from_ref<K, T>(range: T) -> Self
    where
        K: AsRef<[u8]>,
        T: RangeBounds<K>,
    {
        let start = range
            .start_bound()
            .map(|b| Bytes::copy_from_slice(b.as_ref()));
        let end = range
            .end_bound()
            .map(|b| Bytes::copy_from_slice(b.as_ref()));
        Self::new(start, end)
    }

    pub(crate) fn intersect(&self, other: &Self) -> Option<Self> {
        self.inner
            .intersect(&other.inner)
            .map(|inner| Self { inner })
    }

    pub(crate) fn is_start_bound_included_or_unbounded(&self) -> bool {
        !matches!(ByteRangeBounds::start_bound(self), Excluded(_))
    }

    #[cfg(test)]
    pub(crate) fn non_empty(&self) -> bool {
        self.inner.non_empty()
    }

    #[cfg(test)]
    pub(crate) fn empty(&self) -> bool {
        !self.inner.non_empty()
    }

    pub(crate) fn comparable_start_bound(&self) -> StartBound<&Bytes> {
        self.inner.comparable_start_bound()
    }

    pub(crate) fn comparable_end_bound(&self) -> EndBound<&Bytes> {
        self.inner.comparable_end_bound()
    }

    pub(crate) fn as_point(&self) -> Option<&Bytes> {
        match (RangeBounds::start_bound(self), RangeBounds::end_bound(self)) {
            (Included(start), Included(end)) if start == end => Some(start),
            _ => None,
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use crate::bytes_range::BytesRange;
    use crate::proptest_util::arbitrary;
    use crate::proptest_util::sample;

    use bytes::Bytes;
    use proptest::{prop_assert, prop_assert_eq, proptest};
    use std::ops::Bound::{Excluded, Included, Unbounded};
    use std::ops::RangeBounds;

    #[test]
    fn test_arbitrary_range() {
        proptest!(|(range in arbitrary::nonempty_range(10))| {
            assert!(range.non_empty());
        });
    }

    #[should_panic(expected = "Range must be non-empty")]
    #[test]
    fn test_arbitrary_empty_range() {
        proptest!(|(range in arbitrary::empty_range(10))| {
            assert!(range.empty());
        });
    }

    #[test]
    fn test_from_prefix_and_subrange_full_subrange_equals_from_prefix() {
        proptest!(|(prefix in arbitrary::bytes(8))| {
            prop_assert_eq!(
                BytesRange::from_prefix_and_subrange(&prefix, ..),
                BytesRange::from_prefix(&prefix)
            );
        });
    }

    #[test]
    fn test_byte_range_bounds_for_common_shapes() {
        let full = BytesRange::from_prefix_and_subrange(b"p", ..);
        assert_eq!(full.start_bound(), Included(&Bytes::from_static(b"p")));
        assert_eq!(full.end_bound(), Excluded(&Bytes::from_static(b"q")));

        let range = BytesRange::from_prefix_and_subrange(b"", b"a".to_vec()..=b"b".to_vec());
        assert_eq!(range.start_bound(), Included(&Bytes::from_static(b"a")));
        assert_eq!(range.end_bound(), Included(&Bytes::from_static(b"b")));

        let tuple =
            BytesRange::from_prefix_and_subrange(b"ab", (Excluded(&b"x"[..]), Included(&b"y"[..])));
        assert_eq!(
            tuple,
            BytesRange::from_prefix_and_subrange(b"ab", (Excluded(&b"x"[..]), Included(&b"y"[..])))
        );
    }

    #[test]
    fn test_from_prefix_and_subrange_bounded_both_ends() {
        let range = BytesRange::from_prefix_and_subrange(b"user1:", &b"0005"[..]..&b"0042"[..]);
        let start = Bytes::from("user1:0005");
        let end = Bytes::from("user1:0042");
        assert_eq!(range.start_bound(), Included(&start));
        assert_eq!(range.end_bound(), Excluded(&end));
    }

    #[test]
    fn test_from_prefix_and_subrange_inclusive_end() {
        let range = BytesRange::from_prefix_and_subrange(b"ab", ..=&b"x"[..]);
        let start = Bytes::from("ab");
        let end = Bytes::from("abx");
        assert_eq!(range.start_bound(), Included(&start));
        assert_eq!(range.end_bound(), Included(&end));
    }

    #[test]
    fn test_from_prefix_and_subrange_from_suffix_to_end_of_prefix() {
        // The cursor shape: everything with the prefix from a suffix onward.
        let range = BytesRange::from_prefix_and_subrange(b"ab", &b"x"[..]..);
        let start = Bytes::from("abx");
        let end = Bytes::from("ac");
        assert_eq!(range.start_bound(), Included(&start));
        assert_eq!(range.end_bound(), Excluded(&end));
    }

    #[test]
    fn test_from_prefix_and_subrange_excluded_start() {
        let range = BytesRange::from_prefix_and_subrange(b"ab", (Excluded(&b"x"[..]), Unbounded));
        let start = Bytes::from("abx");
        assert_eq!(range.start_bound(), Excluded(&start));
    }

    #[test]
    fn test_from_prefix_and_subrange_all_ff_prefix_unbounded_end() {
        let prefix = vec![0xff, 0xff];
        let range = BytesRange::from_prefix_and_subrange(&prefix, &b"a"[..]..);
        let start = Bytes::from(vec![0xff, 0xff, b'a']);
        assert_eq!(range.start_bound(), Included(&start));
        assert_eq!(range.end_bound(), Unbounded);
    }

    #[test]
    fn test_from_prefix_and_subrange_empty_prefix_is_plain_range() {
        let range = BytesRange::from_prefix_and_subrange(b"", &b"a"[..]..&b"b"[..]);
        assert_eq!(range, BytesRange::from_slice(&b"a"[..]..&b"b"[..]));
        let unbounded = BytesRange::from_prefix_and_subrange(b"", ..);
        assert_eq!(unbounded, BytesRange::unbounded());
    }

    #[test]
    #[should_panic(expected = "Range must be non-empty")]
    fn test_from_prefix_and_subrange_rejects_reversed_bounds() {
        let _ = BytesRange::from_prefix_and_subrange(b"ab", b"z".to_vec()..b"a".to_vec());
    }

    #[test]
    fn test_from_prefix_and_subrange_keys_start_with_prefix() {
        // Safety property for filter consultation: every key in the composed
        // range starts with the prefix, so probing prefix filters with it
        // can never cause a false-negative SST skip.
        proptest!(|(
            prefix in arbitrary::nonempty_bytes(6),
            suffix in arbitrary::bytes(6),
            mut rng in arbitrary::rng()
        )| {
            let range = BytesRange::from_prefix_and_subrange(&prefix, suffix.as_ref()..);
            let key = sample::bytes_in_range(&mut rng, &range);
            prop_assert!(
                key.starts_with(prefix.as_ref()),
                "key {:?} in {:?} does not start with prefix {:?}",
                key, range, prefix,
            );
        });
    }

    #[test]
    fn test_from_prefix_builds_half_open_range() {
        let range = BytesRange::from_prefix(b"ab");
        let start = Bytes::from("ab");
        let end = Bytes::from("ac");
        assert_eq!(range.start_bound(), Included(&start));
        assert_eq!(range.end_bound(), Excluded(&end));
    }

    #[test]
    fn test_from_prefix_handles_all_ff_prefix() {
        let prefix = vec![0xff, 0xff];
        let range = BytesRange::from_prefix(&prefix);
        let start = Bytes::from(prefix);
        assert_eq!(range.start_bound(), Included(&start));
        assert_eq!(range.end_bound(), Unbounded);
    }

    #[test]
    fn test_from_prefix_allows_empty_prefix() {
        let range = BytesRange::from_prefix(b"");
        assert_eq!(range.start_bound(), Unbounded);
        assert_eq!(range.end_bound(), Unbounded);
    }

    #[test]
    fn test_from_prefix_contains_all_prefixed_keys() {
        proptest!(|(
            prefix in arbitrary::nonempty_bytes(8),
            suffix in arbitrary::bytes(8)
        )| {
            let range = BytesRange::from_prefix(&prefix);
            let mut combined = prefix.to_vec();
            combined.extend_from_slice(&suffix);
            let key = Bytes::from(combined);
            prop_assert!(range.contains(&key));
        });
    }

    #[test]
    fn test_from_prefix_unbounded_for_empty_prefix() {
        proptest!(|(key in arbitrary::bytes(12))| {
            let range = BytesRange::from_prefix(b"");
            prop_assert!(range.contains(&key));
        });
    }

    #[test]
    fn test_from_prefix_filters_keys_by_prefix() {
        proptest!(|(
            prefix in arbitrary::nonempty_bytes(8),
            suffix in arbitrary::bytes(8)
        )| {
            let range = BytesRange::from_prefix(&prefix);

            // Any key starting with the prefix should be contained.
            let mut prefixed = prefix.to_vec();
            prefixed.extend_from_slice(&suffix);
            let prefixed = Bytes::from(prefixed);
            prop_assert!(range.contains(&prefixed));

            // A key with a different first byte should be rejected.
            let mut non_prefixed = prefix.to_vec();
            non_prefixed[0] = non_prefixed[0].wrapping_add(1);
            let non_prefixed = Bytes::from(non_prefixed);
            prop_assert!(!range.contains(&non_prefixed));
        });
    }

    #[test]
    fn test_intersection_of_empty_range_and_nonempty_range_is_empty() {
        proptest!(|(
            non_empty_range in arbitrary::nonempty_range(10),
        )| {
            assert!(BytesRange::new_empty().intersect(&non_empty_range).is_none())
        });
    }

    #[test]
    fn test_intersection_of_non_empty_and_unbounded_range_is_nonempty() {
        proptest!(|(non_empty_range in arbitrary::nonempty_range(10))| {
            let unbounded_range = BytesRange::new(Unbounded, Unbounded);
            assert!(non_empty_range.intersect(&unbounded_range).is_some());
        });
    }

    #[test]
    fn test_contains_with_value_in_range() {
        proptest!(|(range in arbitrary::nonempty_range(10), mut rng in arbitrary::rng())| {
            let sample = sample::bytes_in_range(&mut rng, &range);
            assert!(range.contains(&sample), "Expected value {sample:?} is not in range {range:?}");
        });
    }

    #[test]
    #[should_panic(expected = "Range must be non-empty")]
    fn test_contains_with_empty_range() {
        proptest!(|(range in arbitrary::empty_range(10), sample in arbitrary::nonempty_bytes(10))| {
            assert!(!range.contains(&sample), "Expected value {sample:?} to not be in empty range {range:?}");
        });
    }

    #[test]
    fn test_nonempty_intersection_of_intersecting_ranges() {
        proptest!(|(
            (non_empty_1, non_empty_2) in arbitrary::nonempty_intersecting_ranges(10),
        )| {
            assert!(non_empty_1.intersect(&non_empty_2).is_some())
        });
    }

    #[test]
    fn test_new_with_unbounded_range_is_valid() {
        BytesRange::new(Unbounded, Unbounded);
    }

    #[test]
    #[should_panic(expected = "Range must be non-empty")]
    fn test_new_with_start_larger_than_end_panics() {
        let start = Included(Bytes::from("z"));
        let end = Included(Bytes::from("a"));
        BytesRange::new(start, end);
    }

    #[test]
    fn test_empty_included_start_bound_is_valid_and_contains_all_keys() {
        let range = BytesRange::new(Included(Bytes::new()), Unbounded);
        assert!(range.contains(&Bytes::new())); // b"" <= b"" holds for Included
        assert!(range.contains(&Bytes::from("a")));
        assert!(range.contains(&Bytes::from("z")));
    }

    #[test]
    fn test_empty_excluded_start_bound_is_valid_and_contains_all_keys() {
        let range = BytesRange::new(Excluded(Bytes::new()), Unbounded);
        assert!(!range.contains(&Bytes::new())); // b"" < b"" is false for Excluded
        assert!(range.contains(&Bytes::from("a")));
        assert!(range.contains(&Bytes::from("z")));
    }
}
