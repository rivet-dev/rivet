use async_trait::async_trait;
use log::warn;
use slatedb_common::{IdentifiedObjectMetadata, ObjectMetadata};
use std::collections::HashSet;
use std::sync::Arc;

/// Filters garbage-collection deletion candidates.
///
/// Implementations receive the complete set of objects that SlateDB has already
/// determined are eligible for deletion. The returned set is the subset that may
/// actually be deleted. Objects returned by the filter that were not in the input
/// are ignored.
#[async_trait]
pub trait GcFilter: Send + Sync {
    /// Returns the subset of `candidates` that may be deleted.
    async fn filter(&self, candidates: HashSet<ObjectMetadata>) -> HashSet<ObjectMetadata>;
}

pub(crate) async fn retain_allowed_by_gc_filter<Id>(
    gc_filter: &Option<Arc<dyn GcFilter>>,
    candidates: Vec<IdentifiedObjectMetadata<Id>>,
) -> Vec<IdentifiedObjectMetadata<Id>> {
    let Some(gc_filter) = gc_filter else {
        return candidates;
    };

    let candidate_metadata = candidates
        .iter()
        .map(|candidate| candidate.metadata.clone())
        .collect::<HashSet<_>>();
    let allowed_metadata = gc_filter.filter(candidate_metadata.clone()).await;
    let mut sanitized_allowed_metadata = HashSet::with_capacity(allowed_metadata.len());
    for metadata in allowed_metadata {
        if candidate_metadata.contains(&metadata) {
            sanitized_allowed_metadata.insert(metadata);
        } else {
            warn!(
                "GC filter returned non-candidate object; dropping [location={}, size={}]",
                metadata.location, metadata.size
            );
        }
    }

    candidates
        .into_iter()
        .filter(|candidate| sanitized_allowed_metadata.contains(&candidate.metadata))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{DateTime, Utc};
    use object_store::path::Path;

    struct StaticGcFilter {
        allowed: HashSet<ObjectMetadata>,
    }

    #[async_trait]
    impl GcFilter for StaticGcFilter {
        async fn filter(&self, _candidates: HashSet<ObjectMetadata>) -> HashSet<ObjectMetadata> {
            self.allowed.clone()
        }
    }

    fn metadata(location: &str) -> ObjectMetadata {
        ObjectMetadata {
            location: Path::from(location),
            last_modified: DateTime::<Utc>::UNIX_EPOCH,
            size: 1,
            e_tag: None,
            version: None,
        }
    }

    #[tokio::test]
    async fn test_retain_allowed_by_gc_filter_without_filter_keeps_all_candidates() {
        let metadata_a = metadata("a");
        let metadata_b = metadata("b");
        let candidates = vec![
            IdentifiedObjectMetadata::new(1, metadata_a.clone()),
            IdentifiedObjectMetadata::new(2, metadata_b.clone()),
        ];

        let retained = retain_allowed_by_gc_filter(&None, candidates).await;

        assert_eq!(
            retained,
            vec![
                IdentifiedObjectMetadata::new(1, metadata_a),
                IdentifiedObjectMetadata::new(2, metadata_b),
            ]
        );
    }

    #[tokio::test]
    async fn test_retain_allowed_by_gc_filter_keeps_only_allowed_candidates() {
        let metadata_a = metadata("a");
        let metadata_b = metadata("b");
        let candidates = vec![
            IdentifiedObjectMetadata::new(1, metadata_a),
            IdentifiedObjectMetadata::new(2, metadata_b.clone()),
        ];
        let gc_filter = Some(Arc::new(StaticGcFilter {
            allowed: HashSet::from([metadata_b.clone()]),
        }) as Arc<dyn GcFilter>);

        let retained = retain_allowed_by_gc_filter(&gc_filter, candidates).await;

        assert_eq!(retained, vec![IdentifiedObjectMetadata::new(2, metadata_b)]);
    }

    #[tokio::test]
    async fn test_retain_allowed_by_gc_filter_drops_non_candidate_returns() {
        let metadata_a = metadata("a");
        let metadata_b = metadata("b");
        let injected = metadata("injected");
        let candidates = vec![
            IdentifiedObjectMetadata::new(1, metadata_a.clone()),
            IdentifiedObjectMetadata::new(2, metadata_b),
        ];
        let gc_filter = Some(Arc::new(StaticGcFilter {
            allowed: HashSet::from([metadata_a.clone(), injected]),
        }) as Arc<dyn GcFilter>);

        let retained = retain_allowed_by_gc_filter(&gc_filter, candidates).await;

        assert_eq!(retained, vec![IdentifiedObjectMetadata::new(1, metadata_a)]);
    }
}
