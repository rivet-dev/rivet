use std::{
    cmp::{max, min, Ordering},
    hash::{Hash, Hasher},
    ops::{Bound, RangeBounds},
};

use serde::{ser::SerializeStruct, Serialize, Serializer};

#[derive(Debug, Eq)]
pub(crate) struct StartBound<T: Ord> {
    inner: Bound<T>,
}

impl<T: Ord + Clone> Clone for StartBound<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<T: Ord + Clone> StartBound<&T> {
    pub(crate) fn cloned(&self) -> StartBound<T> {
        StartBound {
            inner: self.inner.cloned(),
        }
    }
}

impl<T: Ord + Serialize> Serialize for StartBound<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.inner.serialize(serializer)
    }
}

impl<T: Ord + Hash> Hash for StartBound<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.inner.hash(state)
    }
}

impl<T: Ord> From<Bound<T>> for StartBound<T> {
    fn from(bound: Bound<T>) -> Self {
        Self { inner: bound }
    }
}

impl<T: Ord> From<StartBound<T>> for Bound<T> {
    fn from(bound: StartBound<T>) -> Self {
        bound.inner
    }
}

impl<T: Ord> PartialEq for StartBound<T> {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl<T: Ord> PartialOrd for StartBound<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<T: Ord> Ord for StartBound<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        cmp_bound(&self.inner, &other.inner, true)
    }
}

#[derive(Debug, Eq)]
pub(crate) struct EndBound<T: Ord> {
    inner: Bound<T>,
}

impl<T: Ord + Clone> Clone for EndBound<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<T: Ord + Clone> EndBound<&T> {
    pub(crate) fn cloned(&self) -> EndBound<T> {
        EndBound {
            inner: self.inner.cloned(),
        }
    }
}

impl<T: Ord + Serialize> Serialize for EndBound<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.inner.serialize(serializer)
    }
}

impl<T: Ord + Hash> Hash for EndBound<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.inner.hash(state)
    }
}

impl<T: Ord> From<Bound<T>> for EndBound<T> {
    fn from(bound: Bound<T>) -> Self {
        Self { inner: bound }
    }
}

impl<T: Ord> From<EndBound<T>> for Bound<T> {
    fn from(bound: EndBound<T>) -> Self {
        bound.inner
    }
}

impl<T: Ord> PartialEq for EndBound<T> {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl<T: Ord> PartialOrd for EndBound<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<T: Ord> Ord for EndBound<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        cmp_bound(&self.inner, &other.inner, false)
    }
}

fn cmp_bound<T: Ord>(a: &Bound<T>, b: &Bound<T>, start: bool) -> Ordering {
    match (a, b) {
        (Bound::Included(a), Bound::Included(b)) | (Bound::Excluded(a), Bound::Excluded(b)) => {
            a.cmp(b)
        }
        (Bound::Included(a), Bound::Excluded(b)) => match a.cmp(b) {
            Ordering::Equal => {
                if start {
                    Ordering::Less
                } else {
                    Ordering::Greater
                }
            }
            other => other,
        },
        (Bound::Excluded(a), Bound::Included(b)) => match a.cmp(b) {
            Ordering::Equal => {
                if start {
                    Ordering::Greater
                } else {
                    Ordering::Less
                }
            }
            other => other,
        },
        (Bound::Unbounded, Bound::Unbounded) => Ordering::Equal,
        (Bound::Unbounded, _) => {
            if start {
                Ordering::Less
            } else {
                Ordering::Greater
            }
        }
        (_, Bound::Unbounded) => {
            if start {
                Ordering::Greater
            } else {
                Ordering::Less
            }
        }
    }
}

#[derive(Debug, Eq, PartialEq, Ord, PartialOrd)]
pub(crate) struct ComparableRange<T: Ord> {
    start: StartBound<T>,
    end: EndBound<T>,
}

impl<T: Ord> ComparableRange<T> {
    pub(crate) fn new(start: Bound<T>, end: Bound<T>) -> Self {
        Self {
            start: StartBound::from(start),
            end: EndBound::from(end),
        }
    }
}

impl<T: Ord + Serialize> Serialize for ComparableRange<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_struct("ComparableRange", 2)?;
        seq.serialize_field("start", &self.start)?;
        seq.serialize_field("end", &self.end)?;
        seq.end()
    }
}

impl<T: Ord + Hash> Hash for ComparableRange<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.start.hash(state);
        self.end.hash(state);
    }
}

impl<T: Ord + Clone> ComparableRange<T> {
    #[cfg(test)]
    pub(crate) fn from_range<R: RangeBounds<T>>(range: R) -> Self {
        Self::new(range.start_bound().cloned(), range.end_bound().cloned())
    }

    pub(crate) fn intersect(&self, other: &Self) -> Option<Self> {
        let max_start = max(&self.start, &other.start);
        let min_end = min(&self.end, &other.end);
        let intersection = Self {
            start: max_start.clone(),
            end: min_end.clone(),
        };
        if intersection.non_empty() {
            Some(intersection)
        } else {
            None
        }
    }

    #[allow(dead_code)]
    pub(crate) fn union(&self, other: &Self) -> Option<Self> {
        // Sort the ranges to make the function commutative
        let (first, second) = if self < other {
            (self, other)
        } else {
            (other, self)
        };
        // If the ranges are not intersecting and they are not adjacent, no union is possible
        if first.intersect(second).is_none() && !first.are_adjacent(second) {
            return None;
        }
        // Take the minimum of start bounds and maximum of end bounds
        Some(Self {
            start: min(&first.start, &other.start).clone(),
            end: max(&first.end, &second.end).clone(),
        })
    }

    fn are_adjacent(&self, other: &Self) -> bool {
        match (&self.end.inner, &other.start.inner) {
            (Bound::Included(a), Bound::Excluded(b)) => a == b,
            (Bound::Excluded(a), Bound::Included(b)) => a == b,
            _ => false,
        }
    }

    pub(crate) fn non_empty(&self) -> bool {
        match (&self.start.inner, &self.end.inner) {
            (Bound::Included(a), Bound::Included(b)) => a <= b,
            (Bound::Included(a), Bound::Excluded(b)) => a < b,
            (Bound::Excluded(a), Bound::Excluded(b)) => a < b,
            (Bound::Excluded(a), Bound::Included(b)) => a < b,
            (Bound::Unbounded, _) => true,
            (_, Bound::Unbounded) => true,
        }
    }

    pub(crate) fn comparable_start_bound(&self) -> StartBound<&T> {
        StartBound {
            inner: self.start_bound(),
        }
    }

    pub(crate) fn comparable_end_bound(&self) -> EndBound<&T> {
        EndBound {
            inner: self.end_bound(),
        }
    }
}

impl<T: Ord + Clone> Clone for ComparableRange<T> {
    fn clone(&self) -> Self {
        Self::new(self.start.inner.clone(), self.end.inner.clone())
    }
}

impl<T: Ord> RangeBounds<T> for ComparableRange<T> {
    fn start_bound(&self) -> Bound<&T> {
        self.start.inner.as_ref()
    }

    fn end_bound(&self) -> Bound<&T> {
        self.end.inner.as_ref()
    }
}

#[cfg(test)]
pub(crate) mod tests {

    use std::{
        cmp::Ordering,
        ops::{Bound, RangeBounds},
    };

    use rand::seq::SliceRandom;
    use rstest::rstest;

    use crate::comparable_range::{ComparableRange, EndBound, StartBound};

    struct TestCase(Bound<u32>, Bound<u32>, Ordering);

    #[rstest]
    #[case(TestCase(Bound::Included(1), Bound::Included(1), Ordering::Equal))]
    // [1, 100) vs. (1, 100) => (1 <= n < 100) vs. (1 < n < 100)
    #[case(TestCase(Bound::Included(1), Bound::Excluded(1), Ordering::Less))]
    // (1, 100) vs. [1, 100) =>  (1 < n < 100) vs. (1 <= n < 100)
    #[case(TestCase(Bound::Excluded(1), Bound::Included(1), Ordering::Greater))]
    // For start bound, unbounded represents -Inf
    #[case(TestCase(Bound::Unbounded, Bound::Included(1), Ordering::Less))]
    #[case(TestCase(Bound::Unbounded, Bound::Excluded(1), Ordering::Less))]
    #[case(TestCase(Bound::Included(1), Bound::Unbounded, Ordering::Greater))]
    #[case(TestCase(Bound::Excluded(1), Bound::Unbounded, Ordering::Greater))]
    fn test_start_bound_cmp(#[case] test_case: TestCase) {
        let lhs = StartBound::from(test_case.0);
        let rhs = StartBound::from(test_case.1);
        assert_eq!(lhs.cmp(&rhs), test_case.2);
    }

    #[rstest]
    #[case(TestCase(Bound::Included(100), Bound::Included(100), Ordering::Equal))]
    // (1, 100] vs. (1, 100) => (1 < n <= 100) vs. (1 < n < 100)
    #[case(TestCase(Bound::Included(100), Bound::Excluded(100), Ordering::Greater))]
    // (1, 100) vs. (1, 100] =>  (1 < n < 100) vs. (1 < n <= 100)
    #[case(TestCase(Bound::Excluded(100), Bound::Included(100), Ordering::Less))]
    // For end bound, unbounded represents +Inf
    #[case(TestCase(Bound::Unbounded, Bound::Included(1), Ordering::Greater))]
    #[case(TestCase(Bound::Unbounded, Bound::Excluded(1), Ordering::Greater))]
    #[case(TestCase(Bound::Included(1), Bound::Unbounded, Ordering::Less))]
    #[case(TestCase(Bound::Excluded(1), Bound::Unbounded, Ordering::Less))]
    fn test_end_bound_cmp(#[case] test_case: TestCase) {
        let lhs = EndBound::from(test_case.0);
        let rhs = EndBound::from(test_case.1);
        assert_eq!(lhs.cmp(&rhs), test_case.2);
    }

    #[test]
    fn test_range() {
        let ranges = vec![
            ComparableRange::from_range(..10),
            ComparableRange::from_range(..1000),
            ComparableRange::from_range(..),
            ComparableRange::from_range(1..5),
            ComparableRange::from_range(1..10),
            ComparableRange::from_range(1..),
            ComparableRange::from_range(2..3),
            ComparableRange::from_range(2..),
            ComparableRange::from_range(100..123),
        ];
        let mut shuffled_ranges = ranges.clone();
        // Shuffle the ranges to ensure the order is random
        shuffled_ranges.shuffle(&mut rand::rng());
        // Sort the ranges to ensure the order is deterministic
        shuffled_ranges.sort();

        assert_eq!(shuffled_ranges, ranges);
    }

    struct TwoRangeOperation<T: Ord + Clone> {
        first: ComparableRange<T>,
        second: ComparableRange<T>,
        result: Option<ComparableRange<T>>,
    }

    impl<T: Ord + Clone> TwoRangeOperation<T> {
        fn some<R1, R2, RI>(first: R1, second: R2, intersection: RI) -> Self
        where
            R1: RangeBounds<T>,
            R2: RangeBounds<T>,
            RI: RangeBounds<T>,
        {
            Self {
                first: ComparableRange::from_range(first),
                second: ComparableRange::from_range(second),
                result: Some(ComparableRange::from_range(intersection)),
            }
        }

        fn none<R1, R2>(first: R1, second: R2) -> Self
        where
            R1: RangeBounds<T>,
            R2: RangeBounds<T>,
        {
            Self {
                first: ComparableRange::from_range(first),
                second: ComparableRange::from_range(second),
                result: None,
            }
        }
    }

    #[rstest]
    #[case(TwoRangeOperation::some(0..10, 0..10, 0..10))]
    #[case(TwoRangeOperation::some(0..10, 1..10, 1..10))]
    #[case(TwoRangeOperation::some(0..10, 0..9, 0..9))]
    #[case(TwoRangeOperation::some(0..10, 0..=9, 0..=9))]
    #[case(TwoRangeOperation::some(..=1337, 10..15, 10..15))]
    #[allow(clippy::reversed_empty_ranges)]
    #[case(TwoRangeOperation::none(50..40, 10..60))]
    fn test_intersection(#[case] test_case: TwoRangeOperation<u32>) {
        for (first, second) in [
            (&test_case.first, &test_case.second),
            (&test_case.second, &test_case.first),
        ] {
            let intersection = first.intersect(second);
            assert_eq!(intersection, test_case.result);
        }
    }

    #[rstest]
    #[case(TwoRangeOperation::some(0..10, 10..100, 0..100))]
    #[case(TwoRangeOperation::some(0..=10, 10..100, 0..100))]
    #[case(TwoRangeOperation::none(0..10, 11..100))]
    #[case(TwoRangeOperation::some(..100, 5..=100, ..=100))]
    #[case(TwoRangeOperation::some(..100, 5.., ..))]
    #[case(TwoRangeOperation::some(0..=10, (Bound::Excluded(10), Bound::Included(100)), 0..=100))]
    #[allow(clippy::reversed_empty_ranges)]
    #[case::empty_range(TwoRangeOperation::none(5..0, 0..10))]
    #[case::empty_range(TwoRangeOperation::none((Bound::Excluded(5), Bound::Excluded(5)), 0..10))]
    fn test_union(#[case] test_case: TwoRangeOperation<u32>) {
        for (first, second) in [
            (&test_case.first, &test_case.second),
            (&test_case.second, &test_case.first),
        ] {
            let union = first.union(second);
            assert_eq!(union, test_case.result);
        }
    }

    #[test]
    fn test_is_non_empty() {
        struct TestCase(Bound<i32>, Bound<i32>, bool);
        let cases = vec![
            TestCase(Bound::Included(1), Bound::Included(1), true),
            TestCase(Bound::Included(1), Bound::Excluded(1), false),
            TestCase(Bound::Excluded(1), Bound::Included(1), false),
            TestCase(Bound::Excluded(1), Bound::Excluded(1), false),
            TestCase(Bound::Excluded(1), Bound::Excluded(2), true),
            TestCase(Bound::Excluded(2), Bound::Excluded(1), false),
            TestCase(Bound::Unbounded, Bound::Included(1), true),
            TestCase(Bound::Unbounded, Bound::Excluded(1), true),
            TestCase(Bound::Included(1), Bound::Unbounded, true),
            TestCase(Bound::Excluded(1), Bound::Unbounded, true),
            TestCase(Bound::Unbounded, Bound::Unbounded, true),
        ];
        for case in cases {
            assert_eq!(ComparableRange::new(case.0, case.1).non_empty(), case.2);
        }
    }
}
