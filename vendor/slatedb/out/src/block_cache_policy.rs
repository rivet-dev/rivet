//! Block cache policy for controlling which SST components are cached.

use std::ops::Bound;

use bytes::Bytes;

use crate::bytes_range::BytesRange;
use crate::db_cache::CacheTarget;

/// Whether any [`CacheTarget::Data`] range in `targets` overlaps a data
/// block whose first and last key are `key_span`.
pub(crate) fn should_cache_data_block(targets: &[CacheTarget], key_span: &(Bytes, Bytes)) -> bool {
    targets.iter().any(|target| {
        let CacheTarget::Data(range) = target else {
            return false;
        };
        let (first_key, last_key) = key_span;
        let Some(range) = BytesRange::try_new(range.0.clone(), range.1.clone()) else {
            return false;
        };
        let span = BytesRange::new(
            Bound::Included(first_key.clone()),
            Bound::Included(last_key.clone()),
        );
        range.intersect(&span).is_some()
    })
}

/// Controls block-cache insertion for memtable flush and compaction output.
///
// TODO: add control over when reads go through the block cache, e.g. for
// probing during compaction.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BlockCachePolicy {
    flush_targets: Vec<CacheTarget>,
    compaction_output_targets: Vec<CacheTarget>,
}

impl BlockCachePolicy {
    /// Sets the targets requested for insertion after a memtable flush.
    /// An empty slice disables insertion.
    pub fn with_flush_targets(mut self, targets: &[CacheTarget]) -> Self {
        self.flush_targets = targets.to_vec();
        self
    }

    /// Sets the targets requested for insertion as compaction output is
    /// written. An empty slice disables insertion.
    pub fn with_compaction_output_targets(mut self, targets: &[CacheTarget]) -> Self {
        self.compaction_output_targets = targets.to_vec();
        self
    }

    pub(crate) fn flush_targets(&self) -> &[CacheTarget] {
        &self.flush_targets
    }

    pub(crate) fn compaction_output_targets(&self) -> &[CacheTarget] {
        &self.compaction_output_targets
    }
}

/// The default policy inserts data, index, and filter blocks after a memtable
/// flush, and inserts index and filter blocks as compaction output is written.
impl Default for BlockCachePolicy {
    fn default() -> Self {
        Self {
            flush_targets: vec![
                CacheTarget::data::<&[u8], _>(..),
                CacheTarget::Index,
                CacheTarget::Filters,
            ],
            compaction_output_targets: vec![CacheTarget::Index, CacheTarget::Filters],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_policy_caches_on_flush_and_metadata_on_compaction_output() {
        let policy = BlockCachePolicy::default();

        assert_eq!(
            policy.flush_targets(),
            &[
                CacheTarget::data::<&[u8], _>(..),
                CacheTarget::Index,
                CacheTarget::Filters
            ]
        );
        assert_eq!(
            policy.compaction_output_targets(),
            &[CacheTarget::Index, CacheTarget::Filters]
        );
    }

    #[test]
    fn should_cache_data_block_by_key_span_overlap() {
        let targets = [CacheTarget::data(b"c".as_slice()..b"f".as_slice())];
        let span = |first: &[u8], last: &[u8]| {
            (Bytes::copy_from_slice(first), Bytes::copy_from_slice(last))
        };

        assert!(should_cache_data_block(&targets, &span(b"a", b"c")));
        assert!(should_cache_data_block(&targets, &span(b"d", b"e")));
        assert!(should_cache_data_block(&targets, &span(b"e", b"z")));
        // end bound is exclusive
        assert!(!should_cache_data_block(&targets, &span(b"f", b"z")));
        assert!(!should_cache_data_block(&targets, &span(b"a", b"b")));

        assert!(!should_cache_data_block(
            &[CacheTarget::Index],
            &span(b"d", b"e")
        ));
        assert!(should_cache_data_block(
            &[CacheTarget::data::<&[u8], _>(..)],
            &span(b"a", b"b")
        ));
    }

    #[test]
    fn setters_replace_targets() {
        let policy = BlockCachePolicy::default()
            .with_flush_targets(&[CacheTarget::Stats])
            .with_compaction_output_targets(&[CacheTarget::Index, CacheTarget::Filters]);

        assert_eq!(policy.flush_targets, &[CacheTarget::Stats]);
        assert_eq!(
            policy.compaction_output_targets,
            &[CacheTarget::Index, CacheTarget::Filters]
        );
    }
}
