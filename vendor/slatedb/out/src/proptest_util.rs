#[cfg(test)]
pub(crate) mod runner {
    use crate::proptest_util::rng;
    use proptest::test_runner::{Config, TestRunner};

    pub(crate) fn new(source_file: &'static str, rng_seed: Option<[u8; 32]>) -> TestRunner {
        let rng = rng::new_test_rng(rng_seed);
        let mut config = proptest::test_runner::contextualize_config(Config::default());
        config.source_file = Some(source_file);
        TestRunner::new_with_rng(config, rng)
    }
}

/// A convenient place to put strategies for generating common value types.
/// See tests below for example usage.
#[cfg(test)]
pub(crate) mod arbitrary {
    use crate::bytes_range;
    use crate::bytes_range::BytesRange;
    use crate::iter::IterationOrder;
    use crate::proptest_util::{rng, sample};
    use bytes::{BufMut, Bytes, BytesMut};
    use proptest::arbitrary::any;
    use proptest::collection::vec;
    use proptest::prelude::{Just, Strategy};
    use proptest::prop_oneof;
    use proptest::test_runner::TestRng;
    use std::ops::Bound::{Excluded, Included, Unbounded};

    pub(crate) fn iteration_order() -> impl Strategy<Value = IterationOrder> {
        prop_oneof![
            Just(IterationOrder::Ascending),
            Just(IterationOrder::Descending),
        ]
    }

    pub(crate) fn bytes(size: usize) -> impl Strategy<Value = Bytes> {
        vec(any::<u8>(), 0..size).prop_map(Bytes::from)
    }

    pub(crate) fn nonempty_bytes(size: usize) -> impl Strategy<Value = Bytes> {
        vec(any::<u8>(), 1..size)
            .prop_filter("Filter out [0; 1]", |v| v != &[0; 1])
            .prop_map(Bytes::from)
    }

    /// Get a deterministic RNG which has a seed derived from proptest,
    /// which means all values generated from the RNG can be reproduced
    /// using the proptest seed.
    pub(crate) fn rng() -> impl Strategy<Value = TestRng> {
        vec(any::<u8>(), 32).prop_map(|seed| {
            let mut s: [u8; 32] = Default::default();
            s.clone_from_slice(&seed);
            rng::new_test_rng(Some(s))
        })
    }

    fn nonempty_bounded_range(size: usize) -> impl Strategy<Value = BytesRange> {
        (nonempty_bytes(size), nonempty_bytes(size))
            .prop_filter_map("Filter non-empty ranges", nonempty_range_filter)
            .prop_flat_map(|(start, end)| {
                prop_oneof![
                    Just(BytesRange::new(
                        Included(start.clone()),
                        Included(end.clone())
                    )),
                    Just(BytesRange::new(
                        Excluded(start.clone()),
                        Excluded(end.clone())
                    )),
                    Just(BytesRange::new(
                        Included(start.clone()),
                        Excluded(end.clone())
                    )),
                    Just(BytesRange::new(Excluded(start), Included(end))),
                ]
            })
    }

    fn nonempty_partial_bounded_range(size: usize) -> impl Strategy<Value = BytesRange> {
        nonempty_bytes(size).prop_flat_map(|a| {
            prop_oneof![
                Just(BytesRange::new(Unbounded, Included(a.clone()))),
                Just(BytesRange::new(Unbounded, Excluded(a.clone()))),
                Just(BytesRange::new(Included(a.clone()), Unbounded)),
                Just(BytesRange::new(Excluded(a), Unbounded)),
            ]
        })
    }

    fn extend_bytes(a: &Bytes, value: u8) -> Bytes {
        let mut b = BytesMut::from(a.as_ref());
        b.put_u8(value);
        b.freeze()
    }

    fn arbitrary_single_point_bounded_range(size: usize) -> impl Strategy<Value = BytesRange> {
        nonempty_bytes(size).prop_flat_map(|a| {
            let a_extended = extend_bytes(&a, u8::MIN);
            prop_oneof![
                Just(BytesRange::new(Included(a.clone()), Included(a.clone()))),
                Just(BytesRange::new(
                    Included(a.clone()),
                    Excluded(a_extended.clone())
                )),
                Just(BytesRange::new(Excluded(a), Included(a_extended))),
            ]
        })
    }

    fn min_and_max(a: Bytes, b: Bytes) -> (Bytes, Bytes) {
        if a < b {
            (a, b)
        } else {
            (b, a)
        }
    }

    fn nonempty_range_filter(tuple: (Bytes, Bytes)) -> Option<(Bytes, Bytes)> {
        let (a, b) = tuple;
        if a == b {
            None
        } else {
            let (start, end) = min_and_max(a, b);
            if bytes_range::is_prefix_increment(&start, &end) {
                None
            } else {
                Some((start, end))
            }
        }
    }

    pub(crate) fn empty_range(size: usize) -> impl Strategy<Value = BytesRange> {
        nonempty_bytes(size).prop_flat_map(|a| {
            prop_oneof![
                Just(BytesRange::new(Excluded(a.clone()), Excluded(a.clone()))),
                Just(BytesRange::new(Included(a.clone()), Excluded(a.clone()))),
                Just(BytesRange::new(Excluded(a.clone()), Included(a))),
            ]
        })
    }

    pub(crate) fn nonempty_range(size: usize) -> impl Strategy<Value = BytesRange> {
        prop_oneof![
            Just(BytesRange::new(Unbounded, Unbounded)),
            arbitrary_single_point_bounded_range(size),
            nonempty_partial_bounded_range(size),
            nonempty_bounded_range(size),
        ]
    }

    pub(crate) fn nonempty_intersecting_ranges(
        size: usize,
    ) -> impl Strategy<Value = (BytesRange, BytesRange)> {
        (rng(), nonempty_range(size)).prop_flat_map(|(mut rng, range1)| {
            let start = sample::bytes_in_range(&mut rng, &range1);
            let end_range = BytesRange::new(Included(start.clone()), Unbounded);
            let end = sample::bytes_in_range(&mut rng, &end_range);
            prop_oneof![
                (
                    Just(range1.clone()),
                    Just(BytesRange::new(
                        Included(start.clone()),
                        Included(end.clone())
                    ))
                ),
                (
                    Just(range1.clone()),
                    Just(BytesRange::new(Unbounded, Included(end)))
                ),
                (
                    Just(range1.clone()),
                    Just(BytesRange::new(Included(start), Unbounded))
                ),
                (
                    Just(range1.clone()),
                    Just(BytesRange::new(Unbounded, Unbounded))
                ),
            ]
        })
    }
}

#[cfg(test)]
pub(crate) mod rng {
    use proptest::test_runner::{RngAlgorithm, TestRng};
    use rand::Rng;

    pub(crate) fn new_test_rng(seed: Option<[u8; 32]>) -> TestRng {
        let seed = seed.unwrap_or_else(|| {
            let mut thread_rng = rand::rng();
            let random_bytes: [u8; 32] = thread_rng.random();
            random_bytes
        });
        TestRng::from_seed(RngAlgorithm::ChaCha, &seed)
    }
}

/// Sometimes it is more convenient to generate individual values. This can be
/// useful when we need a sample value which is derived from arbitrary inputs
/// or when the generation of the sample is expensive. For example, it
/// would be too expensive to treat a table of key-value pairs as an arbitrary
/// input into a property test. Instead, we might generate a single arbitrary
/// table sample and use it to seed the database. Then we can run property tests
/// against the database. We nevertheless want the sample table to be deterministically
/// generated so that the test failure can be easily reproduced.
#[cfg(test)]
pub(crate) mod sample {
    use crate::bytes_range::BytesRange;
    use crate::{bytes_range, test_utils};
    use bytes::{BufMut, Bytes, BytesMut};
    use proptest::test_runner::TestRng;
    use rand::distr::uniform::SampleRange;
    use rand::prelude::SliceRandom;
    use rand::Rng;
    use std::cmp::max;
    use std::collections::BTreeMap;
    use std::ops::Bound::{Excluded, Included, Unbounded};
    use std::ops::{Bound, RangeBounds};

    pub(crate) fn bytes<T: SampleRange<usize>>(rng: &mut TestRng, len_range: T) -> Bytes {
        let len = rng.random_range(len_range);
        let mut v: Vec<u8> = Vec::with_capacity(len);
        for _ in 0..len {
            v.push(rng.random());
        }
        v.into()
    }

    pub(crate) fn record(rng: &mut TestRng, max_bytes_len: usize) -> (Bytes, Bytes) {
        let key = bytes(rng, 1..max_bytes_len);
        let value = bytes(rng, 0..max_bytes_len);
        (key, value)
    }

    pub(crate) fn table(
        rng: &mut TestRng,
        num_records: usize,
        max_bytes_len: usize,
    ) -> BTreeMap<Bytes, Bytes> {
        let mut table = BTreeMap::new();
        while table.len() < num_records {
            let (key, value) = record(rng, max_bytes_len);
            table.insert(key, value);
        }
        table
    }

    fn bytes_between(rng: &mut TestRng, start: &Bytes, end: &Bytes) -> Bytes {
        assert_eq!(start.len(), end.len());
        let mut res = BytesMut::new();
        let mut start_bound_satisfied = false;
        let mut end_bound_satisfied = false;
        let min_len = rng.random_range(0..=start.len());

        for i in 0..start.len() {
            let next_val_lb = if !start_bound_satisfied {
                start[i]
            } else {
                u8::MIN
            };
            let next_val_ub = if !end_bound_satisfied {
                end[i]
            } else {
                u8::MAX
            };

            let next_val = if next_val_lb == next_val_ub {
                next_val_lb
            } else {
                let next_bound_range_diff = next_val_ub - next_val_lb;
                next_val_lb + (rng.random::<u8>() % next_bound_range_diff)
            };
            res.put_u8(next_val);

            if next_val > next_val_lb {
                start_bound_satisfied = true;
            }

            if next_val < next_val_ub {
                end_bound_satisfied = true;
            }

            if start_bound_satisfied && end_bound_satisfied && res.len() >= min_len {
                break;
            }
        }

        let res = res.freeze();
        assert!(res >= start && res <= end);
        res
    }

    fn increment_lex(b: &Bytes) -> Bytes {
        let mut res = BytesMut::from(b.as_ref());
        for i in (0..res.len()).rev() {
            if res[i] < u8::MAX {
                res[i] += 1;
                return res.freeze();
            } else {
                res[i] = u8::MIN;
            }
        }
        panic!("Overflow when incrementing bytes {b:?}")
    }

    fn decrement_lex(b: &Bytes) -> Bytes {
        let mut res = BytesMut::from(b.as_ref());
        for i in (0..res.len()).rev() {
            if res[i] > u8::MIN {
                res[i] -= 1;
                return res.freeze();
            } else {
                res[i] = u8::MAX;
            }
        }
        panic!("Overflow when decrementing bytes {b:?}")
    }

    /// Get a random byte array which has [`u8::MIN`] at each index and is
    /// between the min and max bounds (which are assumed to also be arrays
    /// of [`u8::MIN`]).
    fn minvalue_bytes(
        rng: &mut TestRng,
        min_bound: Bound<usize>,
        max_bound: Bound<usize>,
    ) -> Bytes {
        let min_len = match min_bound {
            Unbounded => 0,
            Included(len) => len,
            Excluded(len) => len + 1,
        };

        let max_len = match max_bound {
            Unbounded => min_len + 10,
            Included(len) => len,
            Excluded(len) => len - 1,
        };

        let len = rng.random_range(min_len..=max_len);
        Bytes::from(vec![u8::MIN; len])
    }

    fn bound_len(bound: Bound<&Bytes>) -> usize {
        match bound {
            Unbounded => 0,
            Included(b) | Excluded(b) => b.len(),
        }
    }

    fn padded_bytes(b: &[u8], value: u8, len: usize) -> Bytes {
        let mut padded = BytesMut::from(b);
        while padded.len() < len {
            padded.put_u8(value);
        }
        padded.freeze()
    }

    fn inclusive_end_bound(range: &BytesRange, len: usize) -> Bytes {
        match range.end_bound() {
            Unbounded => padded_bytes(&Bytes::new(), u8::MAX, len),
            Included(b) | Excluded(b) => {
                let b = padded_bytes(b, u8::MIN, len);
                decrement_lex(&b)
            }
        }
    }

    fn inclusive_start_bound(range: &BytesRange, len: usize) -> Bytes {
        match range.start_bound() {
            Unbounded => padded_bytes(&Bytes::new(), u8::MIN, len),
            Included(b) => padded_bytes(b, u8::MIN, len),
            Excluded(b) => {
                let padded = padded_bytes(b, u8::MIN, len);
                if b.len() < len {
                    padded
                } else {
                    increment_lex(&padded)
                }
            }
        }
    }

    fn can_decrement_without_truncation(bytes: &Bytes) -> bool {
        bytes.iter().any(|b| *b > u8::MIN)
    }

    pub(crate) fn bytes_in_range(rng: &mut TestRng, range: &BytesRange) -> Bytes {
        assert!(
            range.non_empty(),
            "Cannot choose an arbitrary value from an empty range"
        );

        if let Some(end) = test_utils::bound_as_option(range.end_bound()) {
            if !can_decrement_without_truncation(end) {
                let min_len = match range.start_bound() {
                    // If start bound is unbounded, minvalue_bytes uses 0 as the length,
                    // and random_range(0 ..) is inclusive. This can lead to an empty length
                    // result, which will trigger a failure in the assertion below (is_empty).
                    // Prevent this by using Excluded(0) as the min len so we get a non-empty
                    // result no matter what.
                    Unbounded => Excluded(0),
                    Included(b) => Included(b.len()),
                    Excluded(b) => Excluded(b.len()),
                };
                let max_len = range.end_bound().map(|b| b.len());
                let result = minvalue_bytes(rng, min_len, max_len);
                assert!(
                    !result.is_empty(),
                    "calculated empty bytes for range {range:?}"
                );
                return result;
            } else if range.start_bound() == Included(end) {
                return end.clone();
            } else {
                let start = test_utils::bound_as_option(range.start_bound())
                    .cloned()
                    .unwrap_or_default();
                if bytes_range::is_prefix_increment(&start, end) {
                    let mut tmp = Vec::new();
                    if matches!(range.start_bound(), Included(_) | Unbounded) {
                        tmp.push(start);
                    }
                    if matches!(range.end_bound(), Included(_)) {
                        tmp.push(end.clone());
                    }
                    tmp.shuffle(rng);
                    return tmp.pop().unwrap();
                }
            }
        }

        let len = max(bound_len(range.start_bound()), bound_len(range.end_bound())) + 3;
        let start_bound = inclusive_start_bound(range, len);
        let end_bound = inclusive_end_bound(range, len);
        bytes_between(rng, &start_bound, &end_bound)
    }
}

#[cfg(test)]
mod tests {
    use std::ops::Bound;

    use crate::bytes_range::BytesRange;
    use crate::proptest_util::{arbitrary, sample};
    use bytes::Bytes;
    use proptest::proptest;
    use proptest::test_runner::{RngAlgorithm, TestRng};

    #[test]
    fn test_arbitrary_bytes() {
        proptest!(|(bytes in arbitrary::bytes(10))| {
            assert!(bytes.len() <= 10);
        });

        proptest!(|(bytes in arbitrary::nonempty_bytes(10))| {
            assert!(!bytes.is_empty());
            assert!(bytes.len() <= 10);
        });
    }

    #[test]
    fn test_sample_table() {
        let mut rng = TestRng::deterministic_rng(RngAlgorithm::ChaCha);
        let num_records = 50;
        let table = sample::table(&mut rng, num_records, 10);
        assert_eq!(table.len(), num_records);

        for (key, value) in table {
            assert!(!key.is_empty());
            assert!(key.len() <= 10);
            assert!(value.len() <= 10);
        }
    }

    #[test]
    fn test_bytes_in_range_with_multiple_bytes() {
        let mut rng = TestRng::deterministic_rng(RngAlgorithm::ChaCha);
        let range = BytesRange::new(
            Bound::Unbounded,
            Bound::Included(Bytes::from_static(&[0; 2])),
        );
        sample::bytes_in_range(&mut rng, &range);
    }
}
