//! Per-call cache routing policy for the bundled object store cache.

use crate::db_state::SstType;
use crate::object_store_tag::{ObjectStoreCallTag, TableStoreKind};

/// Decides how a GET or HEAD is routed, from the call tag.
pub(crate) trait GetPolicy: Send + Sync + 'static + std::fmt::Debug {
    fn get_action(&self, tag: Option<&ObjectStoreCallTag>) -> GetAction;
    fn head_action(&self, tag: Option<&ObjectStoreCallTag>) -> HeadAction;
}

/// The built-in get policy.
#[derive(Debug, Clone, Default)]
pub(crate) struct DefaultGetPolicy;

impl GetPolicy for DefaultGetPolicy {
    fn get_action(&self, tag: Option<&ObjectStoreCallTag>) -> GetAction {
        // Untagged reads (manifest, WAL existence probes, other coordination I/O)
        // are never cached.
        let Some(tag) = tag else {
            return GetAction::Bypass;
        };

        // Compactor reads are one shot large scans that are not worth caching
        // and GC doesn't issue any reads that would benefit from caching.
        // Both bypass the cache. WAL reads are mainly during startup so they
        // bypass the cache.
        if matches!(tag.kind, TableStoreKind::Compactor | TableStoreKind::GC)
            || tag.sst_type == SstType::Wal
        {
            GetAction::Bypass
        } else if tag.retry.is_some() {
            // A reissued, non-bypassed read refetches.
            GetAction::Refetch
        } else {
            GetAction::ReadThrough
        }
    }

    fn head_action(&self, tag: Option<&ObjectStoreCallTag>) -> HeadAction {
        match tag {
            None => HeadAction::Bypass,
            Some(t) if t.sst_type == SstType::Wal => HeadAction::Bypass,
            // GC never serves reads, so a cached head is useless to it.
            Some(t) if t.kind == TableStoreKind::GC => HeadAction::Bypass,
            // A compactor HEAD may serve a cached head but must not create a
            // head-only entry, which would defeat a later foreground range prefetch.
            Some(t) if t.kind == TableStoreKind::Compactor => HeadAction::Probe,
            // Main and reader HEADs read through the cache.
            Some(_) => HeadAction::ReadThrough,
        }
    }
}

/// Decides whether a write source PUT is cached in the on-disk object cache.
pub(crate) trait PutPolicy: Send + Sync + 'static + std::fmt::Debug {
    /// Decide whether a write source PUT or multipart upload is cached.
    fn put_action(&self, tag: Option<&ObjectStoreCallTag>) -> PutAction;
}

/// The built-in put policy, configured by [`CachePutConfig`].
#[derive(Debug, Clone)]
pub(crate) struct DefaultPutPolicy {
    pub(crate) put: CachePutConfig,
}

impl PutPolicy for DefaultPutPolicy {
    fn put_action(&self, tag: Option<&ObjectStoreCallTag>) -> PutAction {
        let Some(tag) = tag else {
            // Untagged writes (manifest, compaction state) are never cached.
            return PutAction::Skip;
        };
        match tag.sst_type {
            SstType::Wal => PutAction::Skip,
            // Each compacted SST write source has its own config gate: the
            // main store writes on flush, the compactor on compaction. Other
            // sources never write compacted SSTs and skip the cache.
            SstType::Compacted => match tag.kind {
                TableStoreKind::Main if self.put.cache_on_flush => PutAction::Cache,
                TableStoreKind::Compactor if self.put.cache_on_compaction => PutAction::Cache,
                _ => PutAction::Skip,
            },
        }
    }
}

/// What the cache should do for a GET, decided from the call tag.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum GetAction {
    /// Skip the cache entirely: no lookup, nothing cached.
    ///
    /// Used for things like compaction input scans and for WAL reads.
    Bypass,
    /// Drop any cached entry for the path, refetches from upstream and
    /// re-caches.
    ///
    /// Used when a read is reissued after a validation failure, so a corrupt
    /// local part is replaced rather than served again.
    Refetch,
    /// Read through the cache: serve from cache, or on a miss fetch from
    /// upstream and admit the parts subject to the entry admission policy.
    ReadThrough,
}

/// What the cache should do for a HEAD request, decided from the call tag.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum HeadAction {
    /// Read the head straight from upstream, with no cache lookup or save.
    /// Used for non compacted SST reads (WAL, manifest, etc.)
    Bypass,
    /// Serve a cached head if present. on a miss fetch from upstream but do not
    /// save it.
    Probe,
    /// Read the head through the cache: serve a saved head, or fetch from
    /// upstream and save it on a miss.
    ReadThrough,
}

/// What the cache should do for a PUT, decided from the call tag and config.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PutAction {
    /// Write through to upstream and also save the payload to the local cache.
    Cache,
    /// Write through to upstream only; leave the local cache untouched.
    Skip,
}

/// Which compacted SST write sources are cached.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct CachePutConfig {
    /// Cache compacted SSTs written by a memtable flush.
    ///
    /// Default is false.
    pub(crate) cache_on_flush: bool,
    /// Cache compacted SSTs written by compaction.
    ///
    /// Default is false.
    pub(crate) cache_on_compaction: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::RetryReason;
    use rstest::rstest;

    fn tag(
        kind: TableStoreKind,
        sst_type: SstType,
        retry: Option<RetryReason>,
    ) -> ObjectStoreCallTag {
        let mut tag = ObjectStoreCallTag::new(kind, sst_type);
        tag.retry = retry;
        tag
    }

    #[rstest]
    // Compactor and GC reads bypass regardless of sst type or retry.
    #[case(
        Some(tag(TableStoreKind::Compactor, SstType::Compacted, None)),
        GetAction::Bypass
    )]
    #[case(
        Some(tag(
            TableStoreKind::Compactor,
            SstType::Compacted,
            Some(RetryReason::CrcMismatch)
        )),
        GetAction::Bypass
    )]
    #[case(
        Some(tag(TableStoreKind::GC, SstType::Compacted, None)),
        GetAction::Bypass
    )]
    // WAL reads bypass for any source, even when reissued (WAL is never cached).
    #[case(Some(tag(TableStoreKind::Main, SstType::Wal, None)), GetAction::Bypass)]
    #[case(
        Some(tag(TableStoreKind::Reader, SstType::Wal, None)),
        GetAction::Bypass
    )]
    #[case(
        Some(tag(
            TableStoreKind::Main,
            SstType::Wal,
            Some(RetryReason::BlockDecodeError)
        )),
        GetAction::Bypass
    )]
    // A reissued, non-bypassed read refetches.
    #[case(
        Some(tag(
            TableStoreKind::Main,
            SstType::Compacted,
            Some(RetryReason::CrcMismatch)
        )),
        GetAction::Refetch
    )]
    #[case(
        Some(tag(
            TableStoreKind::Reader,
            SstType::Compacted,
            Some(RetryReason::BlockDecodeError)
        )),
        GetAction::Refetch
    )]
    // Main and reader reads take the normal read-through path.
    #[case(
        Some(tag(TableStoreKind::Main, SstType::Compacted, None)),
        GetAction::ReadThrough
    )]
    #[case(
        Some(tag(TableStoreKind::Reader, SstType::Compacted, None)),
        GetAction::ReadThrough
    )]
    // Untagged reads (coordination I/O) bypass.
    #[case(None, GetAction::Bypass)]
    fn test_get_action(#[case] tag: Option<ObjectStoreCallTag>, #[case] expected: GetAction) {
        assert_eq!(DefaultGetPolicy.get_action(tag.as_ref()), expected);
    }

    #[rstest]
    // Untagged HEADs bypass.
    #[case(None, HeadAction::Bypass)]
    // WAL HEADs bypass for any source, even when reissued.
    #[case(
        Some(tag(TableStoreKind::Main, SstType::Wal, None)),
        HeadAction::Bypass
    )]
    #[case(
        Some(tag(TableStoreKind::Reader, SstType::Wal, None)),
        HeadAction::Bypass
    )]
    #[case(
        Some(tag(
            TableStoreKind::Main,
            SstType::Wal,
            Some(RetryReason::BlockDecodeError)
        )),
        HeadAction::Bypass
    )]
    #[case(
        Some(tag(TableStoreKind::Compactor, SstType::Compacted, None)),
        HeadAction::Probe
    )]
    #[case(
        Some(tag(
            TableStoreKind::Compactor,
            SstType::Compacted,
            Some(RetryReason::CrcMismatch)
        )),
        HeadAction::Probe
    )]
    #[case(
        Some(tag(TableStoreKind::Main, SstType::Compacted, None)),
        HeadAction::ReadThrough
    )]
    #[case(
        Some(tag(TableStoreKind::Reader, SstType::Compacted, None)),
        HeadAction::ReadThrough
    )]
    #[case(
        Some(tag(TableStoreKind::GC, SstType::Compacted, None)),
        HeadAction::Bypass
    )]
    // Retry does not change the HEAD decision: a head has no refetch path.
    #[case(
        Some(tag(
            TableStoreKind::Main,
            SstType::Compacted,
            Some(RetryReason::CrcMismatch)
        )),
        HeadAction::ReadThrough
    )]
    fn test_head_action(#[case] tag: Option<ObjectStoreCallTag>, #[case] expected: HeadAction) {
        assert_eq!(DefaultGetPolicy.head_action(tag.as_ref()), expected);
    }

    #[rstest]
    // WAL writes are never cached, even with both flags on.
    #[case(
        Some(tag(TableStoreKind::Main, SstType::Wal, None)),
        CachePutConfig { cache_on_flush: true, cache_on_compaction: true },
        PutAction::Skip
    )]
    // Untagged writes (manifest, compaction state) are never cached.
    #[case(
        None,
        CachePutConfig { cache_on_flush: true, cache_on_compaction: true },
        PutAction::Skip
    )]
    // Flush writes (main store, compacted) gated by cache_on_flush.
    #[case(
        Some(tag(TableStoreKind::Main, SstType::Compacted, None)),
        CachePutConfig { cache_on_flush: true, cache_on_compaction: false },
        PutAction::Cache
    )]
    #[case(
        Some(tag(TableStoreKind::Main, SstType::Compacted, None)),
        CachePutConfig { cache_on_flush: false, cache_on_compaction: true },
        PutAction::Skip
    )]
    // Compaction writes (compactor store, compacted) gated by cache_on_compaction.
    #[case(
        Some(tag(TableStoreKind::Compactor, SstType::Compacted, None)),
        CachePutConfig { cache_on_flush: false, cache_on_compaction: true },
        PutAction::Cache
    )]
    #[case(
        Some(tag(TableStoreKind::Compactor, SstType::Compacted, None)),
        CachePutConfig { cache_on_flush: true, cache_on_compaction: false },
        PutAction::Skip
    )]
    // Reader/GC never write compacted SSTs, but if they did the policy is Skip.
    #[case(
        Some(tag(TableStoreKind::Reader, SstType::Compacted, None)),
        CachePutConfig { cache_on_flush: true, cache_on_compaction: true },
        PutAction::Skip
    )]
    #[case(
        Some(tag(TableStoreKind::GC, SstType::Compacted, None)),
        CachePutConfig { cache_on_flush: true, cache_on_compaction: true },
        PutAction::Skip
    )]
    fn test_put_action(
        #[case] tag: Option<ObjectStoreCallTag>,
        #[case] policy: CachePutConfig,
        #[case] expected: PutAction,
    ) {
        assert_eq!(
            DefaultPutPolicy { put: policy }.put_action(tag.as_ref()),
            expected
        );
    }

    #[test]
    fn test_default_put_policy_caches_nothing() {
        let policy = CachePutConfig::default();
        assert!(!policy.cache_on_flush);
        assert!(!policy.cache_on_compaction);
        for kind in [
            TableStoreKind::Main,
            TableStoreKind::Compactor,
            TableStoreKind::Reader,
            TableStoreKind::GC,
        ] {
            assert_eq!(
                DefaultPutPolicy { put: policy }.put_action(Some(&tag(
                    kind,
                    SstType::Compacted,
                    None
                ))),
                PutAction::Skip
            );
        }
    }
}
