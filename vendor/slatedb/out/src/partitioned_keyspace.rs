use std::ops::{Bound, Range};

/// Represents a set of keys that are partitioned into multiple partitions, where each
/// partition stores some range of keys and the keys in a given partition are greater than
/// or equal to all keys from the previous partitions. The space is a multiset, so a given key
/// can be present in multiple contiguous partitions. Each partition specifies a min key that
/// is less than or equal to all its keys, and greater than or equal to all keys in the
/// previous partition.
pub(crate) trait RangePartitionedKeySpace {
    fn partitions(&self) -> usize;

    fn partition_first_key(&self, partition: usize) -> &[u8];
}

// equivalent to https://doc.rust-lang.org/std/primitive.slice.html#method.partition_point
pub(crate) fn partition_point<T: RangePartitionedKeySpace, P: Fn(&[u8]) -> bool>(
    keyspace: &T,
    pred: P,
) -> usize {
    if keyspace.partitions() == 0 {
        return 0;
    }
    let mut low = 0;
    let mut high = keyspace.partitions() - 1;
    let mut part_point = 0;
    while low <= high {
        let mid = low + (high - low) / 2;
        let mid_part_first_key = keyspace.partition_first_key(mid);
        if pred(mid_part_first_key) {
            low = mid + 1;
            part_point = mid + 1;
        } else if mid > low {
            high = mid - 1;
        } else {
            break;
        }
    }
    part_point
}

/// Returns the first partition that could include keys that are greater than or equal to
/// the specified key.
pub(crate) fn first_partition_including_or_after_key<T: RangePartitionedKeySpace>(
    keyspace: &T,
    key: &[u8],
) -> usize {
    // If the whole keyspace is larger than the key, then just return the first partition
    first_partition_including_key(keyspace, key).unwrap_or(0)
}

/// Returns the first partition that could include the given key. Returns None if all partitions
/// contain keys that are strictly greater than the specified key.
pub(crate) fn first_partition_including_key<T: RangePartitionedKeySpace>(
    keyspace: &T,
    key: &[u8],
) -> Option<usize> {
    let part_point = partition_point(keyspace, |first_key| first_key < key);
    if part_point > 0 {
        // Some partition after the first has first_key >= key, so return the previous partition
        return Some(part_point - 1);
    }
    // If the first partition starts with key, then return it. Otherwise, the key is not present
    // in the keyspace because the first partition's first_key is strictly greater.
    if keyspace.partition_first_key(0) == key {
        return Some(0);
    }
    None
}

/// Returns the last partition that could include the given key. Returns None if all partitions
/// contain keys that are strictly greater than the given key.
pub(crate) fn last_partition_including_key<T: RangePartitionedKeySpace>(
    keyspace: &T,
    key: &[u8],
) -> Option<usize> {
    let part_point = partition_point(keyspace, |first_key| first_key <= key);
    if part_point == 0 {
        // If the partition point is 0, that means the first partition's first_key is strictly
        // greater than the key, so no partitions include the key.
        return None;
    }
    Some(part_point - 1)
}

/// Returns the half-open range of partitions that overlap `[start, end)`. Used for mapping
/// key ranges to SST block ranges (range scans, cache warming).
pub(crate) fn partitions_covering_range<T: RangePartitionedKeySpace>(
    keyspace: &T,
    start: Bound<&[u8]>,
    end: Bound<&[u8]>,
) -> Range<usize> {
    let start_idx = match start {
        Bound::Included(k) | Bound::Excluded(k) => {
            first_partition_including_or_after_key(keyspace, k)
        }
        Bound::Unbounded => 0,
    };
    let end_idx_exclusive = match end {
        Bound::Included(k) => last_partition_including_key(keyspace, k)
            .map(|p| p + 1)
            .unwrap_or(start_idx),
        Bound::Excluded(k) => match last_partition_including_key(keyspace, k) {
            None => start_idx,
            Some(p) if k == keyspace.partition_first_key(p) => p,
            Some(p) => p + 1,
        },
        Bound::Unbounded => keyspace.partitions(),
    };
    start_idx..end_idx_exclusive
}

#[cfg(test)]
mod tests {
    use crate::partitioned_keyspace::{
        first_partition_including_key, last_partition_including_key, partition_point,
        RangePartitionedKeySpace,
    };
    use rstest::rstest;

    #[test]
    fn test_partition_point() {
        for len in 0..20 {
            let mut keyspace = TestKeyspace {
                partitions: vec![b"x"; len],
            };
            assert_eq!(0, partition_point(&keyspace, |k| k.is_empty()));
            for i in 0..len {
                keyspace.partitions[i] = b"";
                let partition_point = partition_point(&keyspace, |k| k.is_empty());
                assert_eq!(i + 1, partition_point);
            }
        }
    }

    struct FirstPartitionIncludingKeyTestCase {
        first_keys: Vec<&'static [u8]>,
    }

    #[rstest]
    #[case::one_partition(FirstPartitionIncludingKeyTestCase{
        first_keys: vec![b"aaaa"]
    })]
    #[case::two_partitions(FirstPartitionIncludingKeyTestCase{
        first_keys: vec![b"aaaa", b"bbbb"]
    })]
    #[case::three_partitions(FirstPartitionIncludingKeyTestCase{
        first_keys: vec![b"aaaa", b"bbbb", b"cccccc"]
    })]
    #[case::five_partitions(FirstPartitionIncludingKeyTestCase{
        first_keys: vec![b"aaaa", b"bbbb", b"cccccccc", b"ddd", b"eeeee"]
    })]
    #[case::five_partitions_same_first_key(FirstPartitionIncludingKeyTestCase{
        first_keys: vec![b"aaaa", b"bbbb", b"bbbb", b"bbbb", b"bbbb"]
    })]
    #[case::all_same_first_key(FirstPartitionIncludingKeyTestCase{
        first_keys: vec![b"bbbb", b"bbbb", b"bbbb"]
    })]
    fn test_first_partition_including_key(#[case] case: FirstPartitionIncludingKeyTestCase) {
        let first_keys = &case.first_keys;
        let keyspace = TestKeyspace {
            partitions: case.first_keys.clone(),
        };

        // do a search where the key is earlier than the first key
        let found_partition = first_partition_including_key(&keyspace, b"\x00\x00\x00");
        assert_eq!(None, found_partition);

        let mut prev_key = None;
        let mut expected_found_partition = 0;
        for i in 0..first_keys.len() {
            // do a search where the key matches this partition's first key
            let key = first_keys[i];
            let found_partition = first_partition_including_key(&keyspace, key);
            expected_found_partition = match prev_key {
                Some(prev_key) if prev_key == key => expected_found_partition,
                _ => {
                    if i == 0 {
                        0
                    } else {
                        i - 1
                    }
                }
            };
            assert_eq!(Some(expected_found_partition), found_partition);
            prev_key = Some(key);

            // if this is the last partition, or the next partition is not the same key, do a search
            // where the key is between this partition and the next one
            if i == first_keys.len() - 1 || key != first_keys[i + 1] {
                // we could do something more robust here, but its fine since the test cases are
                // static.
                let key = [key, b"bla"].concat();
                let found_partition = first_partition_including_key(&keyspace, &key);
                assert_eq!(Some(i), found_partition);
            }
        }
    }

    struct LastPartitionIncludingKeyTestCase {
        first_keys: Vec<&'static [u8]>,
    }

    #[rstest]
    #[case::one_partition(LastPartitionIncludingKeyTestCase{
    first_keys: vec![b"aaaa"]
    })]
    #[case::two_partitions(LastPartitionIncludingKeyTestCase{
    first_keys: vec![b"aaaa", b"bbbb"]
    })]
    #[case::three_partitions(LastPartitionIncludingKeyTestCase{
    first_keys: vec![b"aaaa", b"bbbb", b"cccccc"]
    })]
    #[case::five_partitions(LastPartitionIncludingKeyTestCase{
    first_keys: vec![b"aaaa", b"bbbb", b"cccccccc", b"ddd", b"eeeee"]
    })]
    #[case::five_partitions_same_first_key(LastPartitionIncludingKeyTestCase{
    first_keys: vec![b"aaaa", b"bbbb", b"bbbb", b"bbbb", b"bbbb"]
    })]
    #[case::all_same_first_key(LastPartitionIncludingKeyTestCase{
    first_keys: vec![b"bbbb", b"bbbb", b"bbbb"]
    })]
    fn test_last_partition_including_key(#[case] case: LastPartitionIncludingKeyTestCase) {
        let first_keys = &case.first_keys;
        let keyspace = TestKeyspace {
            partitions: case.first_keys.clone(),
        };

        // do a search where the key is earlier than the first key
        let found_partition = last_partition_including_key(&keyspace, b"\x00\x00\x00");
        assert_eq!(None, found_partition);

        for i in 0..first_keys.len() {
            // do a search where the key matches this partition's first key
            let key = first_keys[i];
            let found_partition = last_partition_including_key(&keyspace, key);
            let mut expected_found_partition = i;
            for p in i..first_keys.len() {
                if keyspace.partition_first_key(p) == key {
                    expected_found_partition = p;
                }
            }
            assert_eq!(Some(expected_found_partition), found_partition);

            // if this is the last partition, or the next partition is not the same key, do a search
            // where the key is between this partition and the next one
            if i == first_keys.len() - 1 || key != first_keys[i + 1] {
                let key = [key, b"bla"].concat();
                let found_partition = last_partition_including_key(&keyspace, &key);
                assert_eq!(Some(i), found_partition);
            }
        }
    }

    struct TestKeyspace<'a> {
        partitions: Vec<&'a [u8]>,
    }

    impl RangePartitionedKeySpace for TestKeyspace<'_> {
        fn partitions(&self) -> usize {
            self.partitions.len()
        }

        fn partition_first_key(&self, partition: usize) -> &[u8] {
            self.partitions[partition]
        }
    }
}
