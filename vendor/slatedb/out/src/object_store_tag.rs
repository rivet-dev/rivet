//! The per-call tag SlateDB attaches to the object store calls it issues for
//! an SST (through its internal TableStore component).
//!
//! This module is the contract between SlateDB and a caching
//! [`ObjectStore`](object_store::ObjectStore) wrapper, whether the bundled
//! [`CachedObjectStore`](crate::cached_object_store::CachedObjectStore) or an
//! external implementation passed to
//! [`Db::builder`](crate::Db::builder) as the object store.
//!
//! SlateDB inserts an [`ObjectStoreCallTag`] into the
//! [`object_store::Extensions`] of every `GetOptions`, `PutOptions`, and
//! `PutMultipartOptions` the TableStore builds for an SST. A wrapper reads it
//! back with one lookup:
//!
//! ```ignore
//! if let Some(tag) = ObjectStoreCallTag::from_extensions(&options.extensions) {
//!     // classify by tag.kind, tag.sst_type, tag.retry
//! }
//! ```
//!
//! Manifest reads and writes, compaction state, garbage collector listings,
//! and other coordination I/O carry no tag.
//!
//! When decoding a read fails with a recoverable validation error, SlateDB
//! reissues the read once with `retry` set. A caching wrapper must not serve
//! the same locally cached bytes for a retry-tagged read: drop the cached
//! entry for the path and refetch from the wrapped store, otherwise the
//! caller keeps receiving the corrupt bytes and the read fails permanently.

use bytes::Bytes;
use object_store::Extensions;

pub use crate::db_state::SstType;
pub use crate::error::RetryReason;

/// Identifies the component whose TableStore issued an object store call (the
/// call source). Tagged on every SST read and write alongside the
/// [`SstType`].
///
/// A caching wrapper combines it with the call type (get vs put)
/// to decide admission.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TableStoreKind {
    /// The primary database store: foreground reads and memtable flush writes.
    Main,
    /// A read-only store.
    Reader,
    /// The compactor store: compaction-input reads, compaction-output writes.
    Compactor,
    /// The garbage collector store.
    GC,
}

/// The tag carried on every TableStore SST object store call via
/// [`object_store::Extensions`].
///
/// An `ObjectStore` wrapper (such as an object store cache) reads the tag to
/// decide the action for the call.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObjectStoreCallTag {
    /// The source of the call, to distinguish main store, compactor, etc.
    pub kind: TableStoreKind,
    /// The kind of SST the call is targeting (WAL vs compacted).
    pub sst_type: SstType,
    /// The reason for retry if this call is reissued after a validation failure
    /// on a read.
    pub retry: Option<RetryReason>,
    /// Optional segment prefix used by object-store wrappers for routing.
    /// An empty prefix identifies the root segment; `None` means the segment is
    /// unknown or not applicable.
    pub segment: Option<Bytes>,
}

impl ObjectStoreCallTag {
    /// A tag with no retry reason: the common case (a read sets the retry reason
    /// itself on a reissue).
    ///
    /// Compacted SSTs default to the root segment. WAL SSTs default to `None`.
    pub fn new(kind: TableStoreKind, sst_type: SstType) -> Self {
        let segment = match sst_type {
            SstType::Compacted => Some(Bytes::new()),
            SstType::Wal => None,
        };
        Self::new_with_segment(kind, sst_type, segment)
    }

    /// A tag with an explicit segment hint and no retry reason.
    ///
    /// An empty prefix identifies the root segment. Unlike [`Self::new`],
    /// passing `None` leaves the segment unknown or not applicable for any SST type.
    pub fn new_with_segment(
        kind: TableStoreKind,
        sst_type: SstType,
        segment: Option<Bytes>,
    ) -> Self {
        Self {
            kind,
            sst_type,
            retry: None,
            segment,
        }
    }

    /// Reads the tag back from an extensions map, if present.
    pub fn from_extensions(extensions: &Extensions) -> Option<Self> {
        extensions.get::<Self>().cloned()
    }
}

impl From<ObjectStoreCallTag> for Extensions {
    fn from(tag: ObjectStoreCallTag) -> Self {
        let mut extensions = Extensions::new();
        extensions.insert(tag);
        extensions
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case(SstType::Compacted, Some(Bytes::new()))]
    #[case(SstType::Wal, None)]
    fn test_new_defaults(#[case] sst_type: SstType, #[case] segment: Option<Bytes>) {
        assert_eq!(
            ObjectStoreCallTag::new(TableStoreKind::Main, sst_type),
            ObjectStoreCallTag {
                kind: TableStoreKind::Main,
                sst_type,
                retry: None,
                segment,
            }
        );
    }

    #[rstest]
    #[case(Some(Bytes::from_static(b"segment/")))]
    #[case(Some(Bytes::new()))]
    #[case(None)]
    fn test_new_with_segment(#[case] segment: Option<Bytes>) {
        assert_eq!(
            ObjectStoreCallTag::new_with_segment(
                TableStoreKind::Compactor,
                SstType::Compacted,
                segment.clone(),
            ),
            ObjectStoreCallTag {
                kind: TableStoreKind::Compactor,
                sst_type: SstType::Compacted,
                retry: None,
                segment,
            }
        );
    }
}
