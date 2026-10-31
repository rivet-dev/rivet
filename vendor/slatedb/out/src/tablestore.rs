use std::collections::VecDeque;
use std::ops::{Range, RangeBounds};
use std::sync::Arc;

use bytes::Bytes;
use fail_parallel::{fail_point, FailPointRegistry};
use futures::{future::join_all, StreamExt};
use log::{debug, warn};
use object_store::buffered::BufWriter;
use object_store::path::Path;
use object_store::{GetOptions, ObjectStore, ObjectStoreExt};
use slatedb_common::object_metadata::IdentifiedObjectMetadata;
use slatedb_common::ObjectMetadata;
use tokio::io::AsyncWriteExt;
use ulid::Ulid;

use crate::block_cache_policy::{should_cache_data_block, BlockCachePolicy};
use crate::bytes_range::BytesRange;
use crate::db_cache::{CacheFetch, CacheLookup, CacheTarget};
use crate::db_cache::{CacheLoader, CachedEntry, CachedKey, DbCache, EncodedCachedFilter};
use crate::db_state::{SsTableHandle, SsTableId, SstType};
use crate::error::SlateDBError;
use crate::filter_policy::NamedFilter;
use crate::flatbuffer_types::SsTableIndexOwned;
use crate::format::block::Block;
use crate::format::sst::{EncodedSsTable, EncodedSsTableBlock, SsTableFormat};
use crate::iter::IterationOrder;
use crate::object_store_tag::ObjectStoreCallTag;
pub(crate) use crate::object_store_tag::TableStoreKind;
use crate::partitioned_keyspace::partitions_covering_range;
use crate::paths::PathResolver;
use crate::reader::{ReadTrace, SstTraceLevel};
use crate::sst_builder::EncodedSsTableBuilder;
#[cfg(test)]
use crate::sst_io::MAX_VALIDATION_RETRIES;
use crate::sst_io::{read_obj, read_with_validation_retry, ReadOnlyObject};
use crate::sst_stats::SstStats;
use crate::types::RowEntry;
use tracing::Instrument;

pub(crate) struct TableStore {
    object_store: Arc<dyn ObjectStore>,
    sst_format: SsTableFormat,
    path_resolver: PathResolver,
    #[allow(dead_code)]
    fp_registry: Arc<FailPointRegistry>,
    /// In-memory cache for data blocks and SST metadata.
    cache: Option<Arc<dyn DbCache>>,
    /// Selects which components to insert into the cache.
    block_cache_policy: BlockCachePolicy,
    /// Which component owns this store. Tagged on compacted-SST calls.
    kind: TableStoreKind,
}

fn record_read_cached(span: &tracing::Span, lookup: CacheLookup) {
    match lookup {
        CacheLookup::Hit => span.record("cached", true),
        CacheLookup::Miss => span.record("cached", false),
    };
}

impl TableStore {
    pub(crate) fn new<P: Into<Path>>(
        object_store: Arc<dyn ObjectStore>,
        sst_format: SsTableFormat,
        root_path: P,
        block_cache: Option<Arc<dyn DbCache>>,
        kind: TableStoreKind,
        block_cache_policy: BlockCachePolicy,
    ) -> Self {
        Self::new_with_fp_registry(
            object_store,
            sst_format,
            PathResolver::from_root(root_path),
            Arc::new(FailPointRegistry::new()),
            block_cache,
            kind,
            block_cache_policy,
        )
    }

    pub(crate) fn new_with_fp_registry(
        object_store: Arc<dyn ObjectStore>,
        sst_format: SsTableFormat,
        path_resolver: PathResolver,
        fp_registry: Arc<FailPointRegistry>,
        cache: Option<Arc<dyn DbCache>>,
        kind: TableStoreKind,
        block_cache_policy: BlockCachePolicy,
    ) -> Self {
        Self {
            object_store,
            sst_format,
            path_resolver,
            fp_registry,
            cache,
            block_cache_policy,
            kind,
        }
    }

    /// Gracefully close the block cache, flushing in-memory entries to disk.
    pub(crate) async fn close_cache(&self) -> Result<(), crate::Error> {
        if let Some(ref cache) = self.cache {
            cache.close().await?;
        }
        Ok(())
    }

    /// Creates a streaming SST writer. `segment` is a hint attached to the
    /// [`ObjectStoreCallTag`] for object-store routing.
    pub(crate) fn table_writer(
        self: &Arc<Self>,
        id: SsTableId,
        segment: Option<Bytes>,
    ) -> EncodedSsTableWriter {
        let object_store = self.object_store.clone();
        let path = self.path(&id);
        EncodedSsTableWriter {
            id,
            builder: self.sst_format.table_builder(),
            writer: tagged_buf_writer(
                object_store,
                path,
                ObjectStoreCallTag::new_with_segment(self.kind, SstType::from(&id), segment),
            ),
            table_store: self.clone(),
            bytes_written: 0,
            shutdown_started: false,
            #[cfg(test)]
            blocks_written: 0,
        }
    }

    pub(crate) fn table_builder(&self) -> EncodedSsTableBuilder {
        self.sst_format.table_builder()
    }

    /// Writes an SST. `segment` is a hint attached to the
    /// [`ObjectStoreCallTag`] for object-store routing.
    #[cfg(test)]
    pub(crate) async fn write_sst(
        &self,
        id: &SsTableId,
        encoded_sst: &EncodedSsTable,
        segment: Option<Bytes>,
    ) -> Result<SsTableHandle, SlateDBError> {
        fail_point!(
            self.fp_registry.clone(),
            "write-compacted-sst-io-error",
            |_| { Err(slatedb_io_error()) }
        );

        let object_store = self.object_store.clone();
        let path = self.path(id);
        write_sst_streaming_in_object_store(
            object_store,
            &path,
            encoded_sst,
            ObjectStoreCallTag::new_with_segment(self.kind, SstType::from(id), segment),
        )
        .await?;

        self.cache_on_sst_write(*id, encoded_sst).await;
        Ok(SsTableHandle::new(
            *id,
            encoded_sst.format_version,
            encoded_sst.info.clone(),
        ))
    }

    /// Targets a write of the SST inserts into the block cache.
    fn targets_to_cache(&self, _id: &SsTableId) -> &[CacheTarget] {
        match self.kind {
            TableStoreKind::Compactor => self.block_cache_policy.compaction_output_targets(),
            TableStoreKind::Main => self.block_cache_policy.flush_targets(),
            // We only cache from the main store (flush) and the compactor
            // (compaction output) right now.
            _ => &[],
        }
    }

    /// Inserts the targets selected by the block cache policy for
    /// `sst_table_id` into the block cache.
    ///
    /// Data blocks come from `unconsumed_blocks`, so a caller that already
    /// streamed blocks out (the streaming writer) only has the blocks it has
    /// not drained yet, and caches the rest itself as it drains them.
    async fn cache_on_sst_write(&self, sst_table_id: SsTableId, encoded_sst: &EncodedSsTable) {
        let Some(cache) = &self.cache else {
            return;
        };
        let targets = self.targets_to_cache(&sst_table_id);
        for block in &encoded_sst.unconsumed_blocks {
            // Blocks without a tracked key span (WAL blocks) are never cached.
            let Some(key_span) = &block.key_span else {
                continue;
            };
            if !should_cache_data_block(targets, key_span) {
                continue;
            }
            cache
                .insert(
                    (sst_table_id, block.offset).into(),
                    CachedEntry::with_block(Arc::clone(&block.block)),
                )
                .await;
        }
        if targets.contains(&CacheTarget::Index) {
            cache
                .insert(
                    (sst_table_id, encoded_sst.info.index_offset).into(),
                    CachedEntry::with_sst_index(Arc::new(encoded_sst.index.clone())),
                )
                .await;
        }
        if targets.contains(&CacheTarget::Filters) && !encoded_sst.filters.is_empty() {
            cache
                .insert(
                    (sst_table_id, encoded_sst.info.filter_offset).into(),
                    CachedEntry::with_filters(encoded_sst.filters.clone()),
                )
                .await;
        }
        if targets.contains(&CacheTarget::Stats) {
            if let Some(stats) = &encoded_sst.stats {
                cache
                    .insert(
                        (sst_table_id, encoded_sst.info.stats_offset).into(),
                        CachedEntry::with_sst_stats(Arc::new(stats.clone())),
                    )
                    .await;
            }
        }
    }

    /// Decodes an `EncodedCachedFilter` slice into a fully-decoded
    /// `Arc<[NamedFilter]>` and overwrites the cache entry under `cache_key`
    /// with the decoded form so subsequent hits bypass the decode step.
    /// Entries whose policy name has no match in the configured policies are
    /// dropped.
    async fn decode_and_refresh_filter(
        &self,
        cache: &Arc<dyn DbCache>,
        cache_key: CachedKey,
        encoded: &[EncodedCachedFilter],
    ) -> Arc<[NamedFilter]> {
        let policies = &self.sst_format.filter_policies;
        let decoded: Arc<[NamedFilter]> = encoded
            .iter()
            .filter_map(|ef| match policies.iter().find(|p| p.name() == ef.name) {
                Some(policy) => Some(NamedFilter {
                    name: ef.name.clone(),
                    filter: policy.decode(&ef.data),
                }),
                None => {
                    warn!(
                        "unknown filter policy '{}' in cached filter, skipping",
                        ef.name
                    );
                    None
                }
            })
            .collect::<Vec<_>>()
            .into();

        cache
            .insert(cache_key, CachedEntry::with_filters(decoded.clone()))
            .await;
        decoded
    }

    /// Delete an SSTable from the object store.
    pub(crate) async fn delete_sst(&self, id: &SsTableId) -> Result<(), SlateDBError> {
        let object_store = self.object_store.clone();
        let path = self.path(id);
        debug!("deleting SST [path={}]", path);
        object_store.delete(&path).await.map_err(SlateDBError::from)
    }

    /// Reads metadata for a specific compacted SST object.
    ///
    /// ## Arguments
    /// - `id`: The SST identifier to fetch metadata for.
    /// - `segment`: A hint attached to the [`ObjectStoreCallTag`] for
    ///   object-store routing.
    ///
    /// ## Returns
    /// - `Ok(ObjectMetadata)` containing the path, last-modified time,
    ///   size in bytes, ETag, and version.
    ///
    /// ## Errors
    /// - Returns [`SlateDBError`] if the underlying object store `head` request
    ///   fails (for example, if the object does not exist or storage access
    ///   fails).
    pub(crate) async fn metadata(
        &self,
        id: &SsTableId,
        segment: Option<Bytes>,
    ) -> Result<ObjectMetadata, SlateDBError> {
        let object_store = self.object_store.clone();
        let path = self.path(id);
        let opts = GetOptions {
            head: true,
            extensions: ObjectStoreCallTag::new_with_segment(self.kind, SstType::from(id), segment)
                .into(),
            ..GetOptions::default()
        };
        Ok(ObjectMetadata::new(
            object_store.get_opts(&path, opts).await?.meta,
        ))
    }

    /// List all SSTables in the compacted directory.
    /// The SSTables are returned in ascending order of their IDs. Ulids within
    /// the same millisecond are sorted based on their random suffix.
    /// # Arguments
    /// * `id_range` - The range of IDs to list
    /// # Returns
    /// A list of SSTables in the compacted directory
    pub(crate) async fn list_compacted_ssts<R: RangeBounds<Ulid>>(
        &self,
        id_range: R,
    ) -> Result<Vec<IdentifiedObjectMetadata<SsTableId>>, SlateDBError> {
        let mut sst_list: Vec<IdentifiedObjectMetadata<SsTableId>> = Vec::new();
        let compacted_path = self.path_resolver.compacted_path();
        let mut files_stream = self.object_store.list(Some(&compacted_path));

        while let Some(file) = files_stream.next().await.transpose()? {
            match self.path_resolver.parse_table_id(&file.location) {
                Ok(Some(table_id)) => {
                    let id = table_id.value();
                    if id_range.contains(&id) {
                        sst_list.push(IdentifiedObjectMetadata::from_object_meta(table_id, file));
                    }
                }
                Err(e) => {
                    warn!(
                        "error while parsing file id [location={}, error={}]",
                        file.location, e
                    );
                }
                _ => {
                    warn!(
                        "unexpected file found in compacted directory [location={}]",
                        file.location
                    );
                }
            }
        }

        sst_list.sort_by_key(|m| m.id.value());
        Ok(sst_list)
    }

    /// Opens an SST. `segment` is a hint attached to the
    /// [`ObjectStoreCallTag`] for object-store routing.
    pub(crate) async fn open_sst(
        &self,
        id: &SsTableId,
        segment: Option<Bytes>,
    ) -> Result<SsTableHandle, SlateDBError> {
        let (info, version) = read_obj!(
            &self.object_store,
            self.path(id),
            ObjectStoreCallTag::new_with_segment(self.kind, SstType::from(id), segment),
            |obj| self.sst_format.read_info_and_version(&obj)
        )
        .await?;
        Ok(SsTableHandle::new(*id, version, info))
    }

    #[cfg(test)]
    /// Reads an SST format version. `segment` is a hint attached to the
    /// [`ObjectStoreCallTag`] for object-store routing.
    pub(crate) async fn read_sst_version(
        &self,
        id: &SsTableId,
        segment: Option<Bytes>,
    ) -> Result<u16, SlateDBError> {
        let (_, version) = read_obj!(
            &self.object_store,
            self.path(id),
            ObjectStoreCallTag::new_with_segment(self.kind, SstType::from(id), segment),
            |obj| self.sst_format.read_info_and_version(&obj)
        )
        .await?;
        Ok(version)
    }

    /// Reads the filters of an SSTable. Every returned entry is decoded.
    ///
    /// ## Arguments
    /// - `handle`: The handle of the SSTable to read the filters from.
    /// - `cache_blocks`: Whether to cache the filters after reading them.
    /// - `segment`: A hint attached to the [`ObjectStoreCallTag`] for
    ///   object-store routing.
    pub(crate) async fn read_filters(
        &self,
        handle: &SsTableHandle,
        cache_blocks: bool,
        segment: Option<Bytes>,
        trace: &ReadTrace,
        sst_level: Option<&SstTraceLevel>,
    ) -> Result<Arc<[NamedFilter]>, SlateDBError> {
        // No filter exists for this SST (either no policies configured, or the
        // SST was built below `min_filter_keys`). Return an empty slice without
        // touching the cache: there is nothing to load, and caching the empty
        // answer would mask a future config change that adds filter policies.
        if self.sst_format.filter_policies.is_empty() || handle.info.filter_len == 0 {
            return Ok(Arc::from([]));
        }
        let span = trace.new_read_filter_span(handle.id, sst_level);
        let read = self.read_filters_inner(handle, cache_blocks, segment, span.clone());
        read.instrument(span).await
    }

    async fn read_filters_inner(
        &self,
        handle: &SsTableHandle,
        cache_blocks: bool,
        segment: Option<Bytes>,
        span: tracing::Span,
    ) -> Result<Arc<[NamedFilter]>, SlateDBError> {
        let cache_key: CachedKey = (handle.id, handle.info.filter_offset).into();
        if let Some(cache) = self.cache_for_reads() {
            // cache_blocks=true: dedup-aware fetch; concurrent callers collapse onto
            // one loader. cache_blocks=false: read-only lookup that won't pollute the
            // cache on miss. Cache errors fall through to a best-effort direct load;
            // we intentionally don't re-insert there — `fetch_X` errors are almost
            // always the smuggled loader error (so the direct retry will also fail),
            // and on the rare foyer-machinery error an insert would likely fail too.
            let fetch = if cache_blocks {
                cache
                    .fetch_filter(
                        cache_key.clone(),
                        self.read_loader(handle, CacheTarget::Filters, segment.clone()),
                    )
                    .await
                    .ok()
            } else {
                cache
                    .get_filter(&cache_key)
                    .await
                    .unwrap_or(None)
                    .map(CacheFetch::hit)
            };
            if let Some(CacheFetch { entry, lookup }) = fetch {
                // Already decoded.
                if let Some(filters) = entry.filters() {
                    record_read_cached(&span, lookup);
                    return Ok(filters);
                }
                // Encoded form from disk-cache deserialize. Decode and overwrite
                // the cache entry with the decoded form.
                if let Some(encoded) = entry.encoded_filters() {
                    record_read_cached(&span, lookup);
                    return Ok(self
                        .decode_and_refresh_filter(cache, cache_key, &encoded)
                        .await);
                }
            }
        }
        record_read_cached(&span, CacheLookup::Miss);
        read_obj!(
            &self.object_store,
            self.path(&handle.id),
            ObjectStoreCallTag::new_with_segment(self.kind, SstType::from(&handle.id), segment),
            |obj| self.sst_format.read_filters(&handle.info, &obj)
        )
        .await
    }

    /// Reads the stats block of an SSTable.
    ///
    /// ## Arguments
    /// - `handle`: The handle of the SSTable to read the stats from.
    /// - `cache_blocks`: Whether to cache the stats after reading them.
    /// - `segment`: A hint attached to the [`ObjectStoreCallTag`] for
    ///   object-store routing.
    pub(crate) async fn read_stats(
        &self,
        handle: &SsTableHandle,
        cache_blocks: bool,
        segment: Option<Bytes>,
    ) -> Result<Option<SstStats>, SlateDBError> {
        if handle.info.stats_len == 0 {
            return Ok(None);
        }
        let cache_key = (handle.id, handle.info.stats_offset).into();
        if let Some(cache) = self.cache_for_reads() {
            // See `read_filters` for the rationale on the fall-through path.
            let entry = if cache_blocks {
                cache
                    .fetch_stats(
                        cache_key,
                        self.read_loader(handle, CacheTarget::Stats, segment.clone()),
                    )
                    .await
                    .ok()
                    .map(|fetch| fetch.entry)
            } else {
                cache.get_stats(&cache_key).await.unwrap_or(None)
            };
            if let Some(stats) = entry.and_then(|e| e.sst_stats()) {
                return Ok(Some(stats.as_ref().clone()));
            }
        }
        read_obj!(
            &self.object_store,
            self.path(&handle.id),
            ObjectStoreCallTag::new_with_segment(self.kind, SstType::from(&handle.id), segment),
            |obj| self.sst_format.read_stats(&handle.info, &obj)
        )
        .await
    }

    /// Reads the index of an SSTable.
    ///
    /// ## Arguments
    /// - `handle`: The handle of the SSTable to read the index from.
    /// - `cache_blocks`: Whether to cache the index blocks after reading them.
    /// - `segment`: A hint attached to the [`ObjectStoreCallTag`] for
    ///   object-store routing.
    /// - `trace`: The read trace that owns the index span, if tracing is enabled.
    /// - `sst_level`: The level that contains the SST, if known.
    pub(crate) async fn read_index(
        &self,
        handle: &SsTableHandle,
        cache_blocks: bool,
        segment: Option<Bytes>,
        trace: &ReadTrace,
        sst_level: Option<&SstTraceLevel>,
    ) -> Result<Arc<SsTableIndexOwned>, SlateDBError> {
        let span = trace.new_read_index_span(handle.id, sst_level);
        let read = self.read_index_inner(handle, cache_blocks, segment, span.clone());
        read.instrument(span).await
    }

    async fn read_index_inner(
        &self,
        handle: &SsTableHandle,
        cache_blocks: bool,
        segment: Option<Bytes>,
        span: tracing::Span,
    ) -> Result<Arc<SsTableIndexOwned>, SlateDBError> {
        let cache_key = (handle.id, handle.info.index_offset).into();
        if let Some(cache) = self.cache_for_reads() {
            // See `read_filters` for the rationale on the fall-through path.
            let fetch = if cache_blocks {
                cache
                    .fetch_index(
                        cache_key,
                        self.read_loader(handle, CacheTarget::Index, segment.clone()),
                    )
                    .await
                    .ok()
            } else {
                cache
                    .get_index(&cache_key)
                    .await
                    .unwrap_or(None)
                    .map(CacheFetch::hit)
            };
            if let Some(CacheFetch { entry, lookup }) = fetch {
                if let Some(index) = entry.sst_index() {
                    record_read_cached(&span, lookup);
                    return Ok(index);
                }
            }
        }
        record_read_cached(&span, CacheLookup::Miss);
        let index = read_obj!(
            &self.object_store,
            self.path(&handle.id),
            ObjectStoreCallTag::new_with_segment(self.kind, SstType::from(&handle.id), segment),
            |obj| self.sst_format.read_index(&handle.info, &obj)
        )
        .await?;
        Ok(Arc::new(index))
    }

    /// Build a [`CacheLoader`] for a section-level cache entry (filter, stats,
    /// or index). Panics on [`CacheTarget::Data`]: data blocks resolve through
    /// [`Self::block_loader`] because they require a block index lookup.
    ///
    /// Builds a fresh boxed closure on every call, even when the cache hits and
    /// the loader is never invoked; revisit if cache-hit allocations show up in
    /// profiles.
    fn read_loader(
        &self,
        handle: &SsTableHandle,
        target: CacheTarget,
        segment: Option<Bytes>,
    ) -> CacheLoader {
        let info = handle.info.clone();
        let object_store = self.object_store.clone();
        let path = self.path(&handle.id);
        let sst_format = self.sst_format.clone();
        let tag =
            ObjectStoreCallTag::new_with_segment(self.kind, SstType::from(&handle.id), segment);
        Box::new(move || {
            Box::pin(async move {
                // Only the stats arm can produce `None` (stats_len > 0 but no
                // stats decoded); filters and index always yield an entry.
                let entry = read_with_validation_retry(tag, async |tag| {
                    let obj = ReadOnlyObject {
                        object_store: object_store.clone(),
                        path: path.clone(),
                        tag,
                    };
                    match target.clone() {
                        CacheTarget::Filters => {
                            let filters = sst_format
                                .read_filters(&info, &obj)
                                .await
                                .map_err(|e| e.with_path(&obj.path))?;
                            Ok(Some(CachedEntry::with_filters(filters)))
                        }
                        CacheTarget::Stats => {
                            let stats = sst_format
                                .read_stats(&info, &obj)
                                .await
                                .map_err(|e| e.with_path(&obj.path))?;
                            Ok(stats.map(|s| CachedEntry::with_sst_stats(Arc::new(s))))
                        }
                        CacheTarget::Index => {
                            let index = sst_format
                                .read_index(&info, &obj)
                                .await
                                .map_err(|e| e.with_path(&obj.path))?;
                            Ok(Some(CachedEntry::with_sst_index(Arc::new(index))))
                        }
                        CacheTarget::Data(_) => {
                            unreachable!("data blocks use block_loader, not read_loader")
                        }
                    }
                })
                .await?;
                entry.ok_or_else(|| {
                    crate::Error::data("stats_len > 0 but read_stats returned no stats".to_string())
                })
            })
        })
    }

    /// Builds a fresh boxed closure on every call, even when the cache hits and
    /// the loader is never invoked; revisit if cache-hit allocations show up in
    /// profiles.
    fn block_loader(
        &self,
        handle: &SsTableHandle,
        index: Arc<SsTableIndexOwned>,
        block_num: usize,
        segment: Option<Bytes>,
    ) -> CacheLoader {
        let info = handle.info.clone();
        let object_store = self.object_store.clone();
        let path = self.path(&handle.id);
        let sst_format = self.sst_format.clone();
        let tag =
            ObjectStoreCallTag::new_with_segment(self.kind, SstType::from(&handle.id), segment);
        Box::new(move || {
            Box::pin(async move {
                let block = read_with_validation_retry(tag, async |tag| {
                    let obj = ReadOnlyObject {
                        object_store: object_store.clone(),
                        path: path.clone(),
                        tag,
                    };
                    sst_format
                        .read_block(&info, &index, block_num, &obj)
                        .await
                        .map_err(|e| e.with_path(&obj.path))
                })
                .await?;
                Ok(CachedEntry::with_block(Arc::new(block)))
            })
        })
    }

    #[allow(dead_code)]
    /// Reads a range of SST blocks. `segment` is a hint attached to the
    /// [`ObjectStoreCallTag`] for object-store routing.
    pub(crate) async fn read_blocks(
        &self,
        handle: &SsTableHandle,
        blocks: Range<usize>,
        segment: Option<Bytes>,
    ) -> Result<VecDeque<Block>, SlateDBError> {
        let object_store = self.object_store.clone();
        let path = self.path(&handle.id);
        read_with_validation_retry(
            ObjectStoreCallTag::new_with_segment(self.kind, SstType::from(&handle.id), segment),
            |tag| {
                let obj = ReadOnlyObject {
                    object_store: object_store.clone(),
                    path: path.clone(),
                    tag,
                };
                let blocks = blocks.clone();
                async move {
                    let index = self
                        .sst_format
                        .read_index(&handle.info, &obj)
                        .await
                        .map_err(|e| e.with_path(&obj.path))?;
                    self.sst_format
                        .read_blocks(&handle.info, &index, blocks, &obj)
                        .await
                        .map_err(|e| e.with_path(&obj.path))
                }
            },
        )
        .await
    }

    /// Returns the smallest contiguous block range, starting at `first_block` in
    /// `order`, whose encoded size is at least `target_bytes`. If the SST boundary
    /// is reached first, all remaining blocks in that direction are returned.
    pub(crate) fn block_range_for_target_bytes(
        &self,
        handle: &SsTableHandle,
        index: &SsTableIndexOwned,
        first_block: usize,
        target_bytes: usize,
        order: IterationOrder,
    ) -> Range<usize> {
        assert!(target_bytes > 0);

        let index = index.borrow();
        let block_meta = index.block_meta();
        let num_blocks = block_meta.len();
        assert!(first_block < num_blocks);

        let target_bytes = u64::try_from(target_bytes).unwrap_or(u64::MAX);
        match order {
            IterationOrder::Ascending => {
                let mut blocks = first_block..first_block + 1;
                loop {
                    let byte_range =
                        self.sst_format
                            .block_range(blocks.clone(), &handle.info, &index);
                    if byte_range.end.saturating_sub(byte_range.start) >= target_bytes
                        || blocks.end == num_blocks
                    {
                        return blocks;
                    }
                    blocks.end += 1;
                }
            }
            IterationOrder::Descending => {
                let mut blocks = first_block..first_block + 1;
                loop {
                    let byte_range =
                        self.sst_format
                            .block_range(blocks.clone(), &handle.info, &index);
                    if byte_range.end.saturating_sub(byte_range.start) >= target_bytes
                        || blocks.start == 0
                    {
                        return blocks;
                    }
                    blocks.start -= 1;
                }
            }
        }
    }

    /// Reads specified blocks from an SSTable using the provided index.
    ///
    /// This function attempts to read blocks from the cache if available
    /// and falls back to reading from storage for uncached blocks
    /// using an async fetch for each contiguous range that blocks are not cached.
    /// It can optionally cache newly read blocks.
    /// `segment` is a hint attached to the [`ObjectStoreCallTag`] for
    /// object-store routing.
    pub(crate) async fn read_blocks_using_index(
        &self,
        handle: &SsTableHandle,
        index: Arc<SsTableIndexOwned>,
        blocks: Range<usize>,
        cache_blocks: bool,
        segment: Option<Bytes>,
    ) -> Result<VecDeque<Arc<Block>>, SlateDBError> {
        // Single-block reads (point-gets via SstIterator::for_key, SstFile::read_block,
        // etc.) take a dedup-aware fast-path: concurrent callers for the same block
        // collapse onto one loader. Multi-block reads fall through to the range-
        // coalesced path below, which issues one object-store GET per contiguous
        // run of uncached blocks. Cache errors fall through to the direct load,
        // which produces the authoritative error if any.
        if cache_blocks && blocks.len() == 1 {
            if let Some(cache) = self.cache_for_reads() {
                let block_num = blocks.start;
                let offset = index.borrow().block_meta().get(block_num).offset();
                let cache_key: CachedKey = (handle.id, offset).into();
                let loader = self.block_loader(handle, index.clone(), block_num, segment.clone());
                if let Ok(fetch) = cache.fetch_block(cache_key, loader).await {
                    if let Some(block) = fetch.entry.block() {
                        let mut result = VecDeque::with_capacity(1);
                        result.push_back(block);
                        return Ok(result);
                    }
                }
            }
        }

        let object_store = self.object_store.clone();
        let path = self.path(&handle.id);
        // Initialize the result vector and a vector to track uncached ranges
        let mut blocks_read = VecDeque::with_capacity(blocks.end - blocks.start);
        let mut uncached_ranges = Vec::new();

        // If block cache is available, try to retrieve cached blocks
        if let Some(cache) = self.cache_for_reads() {
            let index_borrow = index.borrow();
            // Attempt to get all requested blocks from cache concurrently
            let cached_blocks = join_all(blocks.clone().map(|block_num| async move {
                let block_meta = index_borrow.block_meta().get(block_num);
                let offset = block_meta.offset();
                cache
                    .get_block(&(handle.id, offset).into())
                    .await
                    .unwrap_or(None)
                    .and_then(|entry| entry.block())
            }))
            .await;

            let mut last_uncached_start = None;

            // Process cached block results
            for (index, block_result) in cached_blocks.into_iter().enumerate() {
                match block_result {
                    Some(cached_block) => {
                        // If a cached block is found, add it to blocks_read
                        if let Some(start) = last_uncached_start.take() {
                            uncached_ranges.push((blocks.start + start)..(blocks.start + index));
                        }
                        blocks_read.push_back(cached_block);
                    }
                    None => {
                        // If a block is not in cache, mark the start of an uncached range
                        last_uncached_start.get_or_insert(index);
                    }
                }
            }
            // Add the last uncached range if it exists
            if let Some(start) = last_uncached_start {
                uncached_ranges.push((blocks.start + start)..blocks.end);
            }
        } else {
            // If no cache is available, treat all blocks as uncached
            uncached_ranges.push(blocks.clone());
        }
        // Read uncached blocks concurrently
        let uncached_blocks = join_all(uncached_ranges.iter().map(|range| {
            let object_store = &object_store;
            let path = &path;
            let index_ref = &index;
            let segment = segment.clone();
            async move {
                read_with_validation_retry(
                    ObjectStoreCallTag::new_with_segment(
                        self.kind,
                        SstType::from(&handle.id),
                        segment,
                    ),
                    |tag| {
                        let obj = ReadOnlyObject {
                            object_store: object_store.clone(),
                            path: path.clone(),
                            tag,
                        };
                        async move {
                            self.sst_format
                                .read_blocks(&handle.info, index_ref, range.clone(), &obj)
                                .await
                                .map_err(|e| e.with_path(&obj.path))
                        }
                    },
                )
                .await
            }
        }))
        .await;

        // Merge uncached blocks with blocks_read and prepare blocks for caching
        let mut blocks_to_cache = vec![];
        for (range, range_blocks) in uncached_ranges.into_iter().zip(uncached_blocks) {
            let index_borrow = index.borrow();
            for (block_num, block_read) in range.zip(range_blocks?) {
                let block = Arc::new(block_read);
                if cache_blocks {
                    let block_meta = index_borrow.block_meta().get(block_num);
                    let offset = block_meta.offset();
                    blocks_to_cache.push((handle.id, offset, block.clone()));
                }
                blocks_read.insert(block_num - blocks.start, block);
            }
        }

        // Cache the newly read blocks if caching is enabled
        if let Some(cache) = self.cache_for_reads() {
            if !blocks_to_cache.is_empty() {
                join_all(blocks_to_cache.into_iter().map(|(id, offset, block)| {
                    cache.insert((id, offset).into(), CachedEntry::with_block(block))
                }))
                .await;
            }
        }

        Ok(blocks_read)
    }

    #[allow(dead_code)]
    /// Reads one SST block. `segment` is a hint attached to the
    /// [`ObjectStoreCallTag`] for object-store routing.
    pub(crate) async fn read_block(
        &self,
        handle: &SsTableHandle,
        block: usize,
        segment: Option<Bytes>,
    ) -> Result<Block, SlateDBError> {
        read_obj!(
            &self.object_store,
            self.path(&handle.id),
            ObjectStoreCallTag::new_with_segment(self.kind, SstType::from(&handle.id), segment),
            |obj| async {
                let index = self.sst_format.read_index(&handle.info, &obj).await?;
                self.sst_format
                    .read_block(&handle.info, &index, block, &obj)
                    .await
            }
        )
        .await
    }

    fn path(&self, id: &SsTableId) -> Path {
        self.path_resolver.sst_path(id)
    }

    pub(crate) fn estimate_encoded_size_compacted(
        &self,
        num_entries: usize,
        size_entries: usize,
    ) -> usize {
        self.sst_format
            .estimate_encoded_size_compacted(num_entries, size_entries)
    }

    /// Whether any registered filter policy can answer a range query.
    pub(crate) fn any_filter_policy_supports_range_queries(&self) -> bool {
        self.sst_format
            .filter_policies
            .iter()
            .any(|policy| policy.supports_range_queries())
    }

    pub(crate) fn cache(&self) -> Option<&Arc<dyn DbCache>> {
        self.cache.as_ref()
    }

    /// The block cache to probe for read operations, gated based on the table
    /// store kind.
    ///
    /// compactor reads bypass it so compaction input does not pollute the
    /// cache.
    // TODO: revisit this when the read side of BlockCachePolicy is implemented.
    fn cache_for_reads(&self) -> Option<&Arc<dyn DbCache>> {
        if self.kind == TableStoreKind::Compactor {
            None
        } else {
            self.cache.as_ref()
        }
    }

    /// Best-effort removal of the cache entries of an SST named by `targets`.
    ///
    /// A [`CacheTarget::Data`] target removes the blocks whose key span
    /// overlaps its range, which needs the SST index. `segment` is a hint
    /// attached to the [`ObjectStoreCallTag`] of that index read.
    pub(crate) async fn evict_sst_targets_from_cache(
        &self,
        handle: &SsTableHandle,
        targets: &[CacheTarget],
        segment: Option<Bytes>,
    ) {
        let Some(ref cache) = self.cache else {
            return;
        };
        // Drop empty ranges with BytesRange::try_new.
        let data_ranges: Vec<BytesRange> = targets
            .iter()
            .filter_map(|target| match target {
                CacheTarget::Data((start, end)) => BytesRange::try_new(start.clone(), end.clone()),
                _ => None,
            })
            .collect();
        if !data_ranges.is_empty() {
            self.evict_blocks_from_cache(cache, handle, &data_ranges, segment)
                .await;
        }
        if targets.contains(&CacheTarget::Index) {
            cache
                .remove(&(handle.id, handle.info.index_offset).into())
                .await;
        }
        // Only evict filter/stats when those sections exist. Otherwise
        // SsTableInfo's filter_offset collides with index_offset (filter_len
        // == 0) and stats_offset collides with the first data block
        // (stats_offset == 0).
        if targets.contains(&CacheTarget::Filters) && handle.info.filter_len > 0 {
            cache
                .remove(&(handle.id, handle.info.filter_offset).into())
                .await;
        }
        if targets.contains(&CacheTarget::Stats) && handle.info.stats_len > 0 {
            cache
                .remove(&(handle.id, handle.info.stats_offset).into())
                .await;
        }
    }

    /// Removes the data blocks of an SST whose key span overlaps any of
    /// `ranges` from `cache`.
    async fn evict_blocks_from_cache(
        &self,
        cache: &Arc<dyn DbCache>,
        handle: &SsTableHandle,
        ranges: &[BytesRange],
        segment: Option<Bytes>,
    ) {
        // Best effort: if we can't read the index we can't enumerate blocks,
        // so log and skip. Remaining entries will age out under normal pressure.
        let index = match self
            .read_index(handle, false, segment, &ReadTrace::new(None), None)
            .await
        {
            Ok(index) => index,
            Err(e) => {
                warn!(
                    "evict_blocks_from_cache: failed to read index for SST {:?}: {}",
                    handle.id, e
                );
                return;
            }
        };
        let index_borrow = index.borrow();
        let meta = index_borrow.block_meta();
        for range in ranges {
            let blocks = partitions_covering_range(
                &index_borrow,
                range.start_bound().map(|b| b.as_ref()),
                range.end_bound().map(|b| b.as_ref()),
            );
            for block_num in blocks {
                let offset = meta.get(block_num).offset();
                cache.remove(&(handle.id, offset).into()).await;
            }
        }
    }
}

/// Builds a [`BufWriter`] whose upload carries `tag` in its extensions.
fn tagged_buf_writer(
    object_store: Arc<dyn ObjectStore>,
    path: Path,
    tag: ObjectStoreCallTag,
) -> BufWriter {
    // The one sanctioned `BufWriter::new`: it attaches the tag right after, so
    // every SST streaming write carries it. Other sites are steered here by the
    // clippy `disallowed-methods` rule on `BufWriter::new`.
    #[allow(clippy::disallowed_methods)]
    BufWriter::new(object_store, path).with_extensions(tag.into())
}

#[cfg(test)]
async fn write_sst_streaming_in_object_store(
    object_store: Arc<dyn ObjectStore>,
    path: &Path,
    encoded_sst: &EncodedSsTable,
    tag: ObjectStoreCallTag,
) -> Result<(), SlateDBError> {
    let mut writer = tagged_buf_writer(object_store, path.clone(), tag);
    for block in &encoded_sst.unconsumed_blocks {
        writer.put(block.encoded_bytes.clone()).await?;
    }
    writer.put(encoded_sst.footer.clone()).await?;
    writer.shutdown().await?;
    Ok(())
}

pub(crate) struct EncodedSsTableWriter {
    id: SsTableId,
    builder: EncodedSsTableBuilder,
    writer: BufWriter,
    table_store: Arc<TableStore>,
    bytes_written: usize,
    shutdown_started: bool,
    #[cfg(test)]
    blocks_written: usize,
}

impl EncodedSsTableWriter {
    /// Adds an entry to the SSTable and returns the size of the block that was finished if any.
    /// The block size is calculated after applying any compression if enabled.
    /// The block size is None if the builder has not finished compacting a block yet.
    pub(crate) async fn add(&mut self, entry: RowEntry) -> Result<Option<usize>, SlateDBError> {
        let block_size = self.builder.add(entry).await?;
        self.drain_blocks().await?;
        Ok(block_size)
    }

    /// Finish this SST: write the last block, write the footer, close
    /// the upload.
    ///
    /// On success, returns the finished SST handle and the exact
    /// number of bytes written.
    ///
    /// On failure, aborts the upload before returning the error. This
    /// discards any blocks already sent to object storage for this
    /// SST.
    pub(crate) async fn close(mut self) -> Result<(SsTableHandle, u64), SlateDBError> {
        let result = async {
            fail_point!(
                self.table_store.fp_registry.clone(),
                "write-compacted-sst-io-error",
                |_| Err(slatedb_io_error())
            );
            let builder = std::mem::replace(&mut self.builder, self.table_store.table_builder());
            let encoded_sst = builder.build().await?;
            for block in &encoded_sst.unconsumed_blocks {
                self.writer.write_all(block.encoded_bytes.as_ref()).await?;
                self.bytes_written += block.encoded_bytes.len();
            }
            self.writer.write_all(encoded_sst.footer.as_ref()).await?;
            self.bytes_written += encoded_sst.footer.len();
            self.shutdown_started = true;
            self.writer.shutdown().await?;

            // Cache inserts happen after writer shutdown so an SST whose upload
            // fails contributes no metadata entries.
            //
            // Blocks drained while entries were added are cached by `write_block`,
            // so the only data block left for `cache_on_sst_write` is the tail
            // block that `build` finished.
            self.table_store
                .cache_on_sst_write(self.id, &encoded_sst)
                .await;
            Ok((
                SsTableHandle::new(self.id, encoded_sst.format_version, encoded_sst.info),
                self.bytes_written as u64,
            ))
        }
        .await;

        if result.is_err() {
            if let Err(abort_err) = self.abort().await {
                warn!(
                    "failed to abort sst writer after error [id={:?}, error={:?}]",
                    self.id, abort_err
                );
            }
        }
        result
    }

    /// Tell object storage to discard this SST's upload.
    ///
    /// If the upload never grew past the small in-memory buffer,
    /// nothing reached object storage yet, so this is a no-op.
    /// Otherwise this cancels the multipart upload and asks object
    /// storage to delete the parts already stored.
    ///
    /// Once shutdown has started this is a no-op: the underlying
    /// `BufWriter` panics if aborted after shutdown, and a failed
    /// shutdown leaves nothing this can usefully cancel.
    pub(crate) async fn abort(&mut self) -> Result<(), SlateDBError> {
        if self.shutdown_started {
            return Ok(());
        }
        self.writer
            .abort()
            .await
            .map_err(|e| SlateDBError::ObjectStoreError(Arc::new(e)))
    }

    async fn drain_blocks(&mut self) -> Result<(), SlateDBError> {
        while let Some(block) = self.builder.next_block() {
            self.write_block(block).await?;
            #[cfg(test)]
            {
                self.blocks_written += 1;
            }
        }
        Ok(())
    }

    async fn write_block(&mut self, block: EncodedSsTableBlock) -> Result<(), SlateDBError> {
        fail_point!(
            self.table_store.fp_registry.clone(),
            "write-compacted-sst-io-error",
            |_| Err(slatedb_io_error())
        );
        self.writer.write_all(block.encoded_bytes.as_ref()).await?;
        self.bytes_written += block.encoded_bytes.len();
        if let (Some(cache), Some(key_span)) = (&self.table_store.cache, &block.key_span) {
            let targets = self.table_store.targets_to_cache(&self.id);
            if should_cache_data_block(targets, key_span) {
                cache
                    .insert(
                        (self.id, block.offset).into(),
                        CachedEntry::with_block(block.block),
                    )
                    .await;
            }
        }
        Ok(())
    }

    pub(crate) fn is_drained(&self) -> bool {
        self.builder.is_drained()
    }

    pub(crate) fn id(&self) -> SsTableId {
        self.id
    }

    #[cfg(test)]
    pub(crate) fn blocks_written(&self) -> usize {
        self.blocks_written
    }
}

#[allow(dead_code)]
fn slatedb_io_error() -> SlateDBError {
    SlateDBError::from(std::io::Error::other("oops"))
}

#[cfg(test)]
mod tests {
    use crate::config::TracingOptions;
    use crate::reader::{ReadTrace, SstTraceLevel};
    use crate::types::KeyValue;
    use bytes::Bytes;
    use futures::future;
    use futures::StreamExt;
    use object_store::{memory::InMemory, path::Path, ObjectStore, ObjectStoreExt};
    use rstest::rstest;
    use std::collections::{HashMap, VecDeque};
    use std::ops::Range;
    use std::sync::Arc;
    use tracing_subscriber::layer::SubscriberExt;

    use crate::block_cache_policy::BlockCachePolicy;
    use crate::db_cache::test_utils::TestCache;
    use crate::db_cache::CacheTarget;
    use crate::db_cache::SplitCache;
    use crate::db_cache::{CachedKey, DbCache, DbCacheWrapper};
    use crate::db_state::{SsTableHandle, SsTableInfo};
    use crate::error;
    use crate::flatbuffer_types::{
        BlockMeta, BlockMetaArgs, SsTableIndex, SsTableIndexArgs, SsTableIndexOwned,
    };
    use crate::format::block::Block;
    use crate::format::sst::{SsTableFormat, SST_FORMAT_VERSION_LATEST};
    use crate::iter::IterationOrder;
    use crate::manifest::SsTableView;
    use crate::retrying_object_store::RetryingObjectStore;
    use crate::sst_iter::{SstIterator, SstIteratorOptions};
    use crate::tablestore::{TableStore, TableStoreKind};
    use crate::test_utils::{assert_iterator, build_test_sst, FlakyObjectStore, SpanRecorder};
    use crate::types::{RowEntry, ValueDeletable};
    use crate::{block_iterator::BlockIteratorLatest, db_state::SsTableId, iter::RowEntryIterator};
    use slatedb_common::clock::DefaultSystemClock;
    use slatedb_common::DbRand;

    const ROOT: &str = "/root";

    /// Wraps an object store: counts range-bounded `get_opts` calls and pauses the first
    /// one until `release` is notified. Other methods just delegate. Shared by the
    /// concurrent-read dedup tests.
    #[cfg(feature = "foyer")]
    #[derive(Debug)]
    struct PauseFirstReadStore {
        inner: Arc<dyn ObjectStore>,
        get_range_count: std::sync::atomic::AtomicUsize,
        paused: std::sync::atomic::AtomicBool,
        first_read_started: Arc<tokio::sync::Notify>,
        release: Arc<tokio::sync::Notify>,
    }

    #[cfg(feature = "foyer")]
    impl std::fmt::Display for PauseFirstReadStore {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "PauseFirstReadStore({})", self.inner)
        }
    }

    #[cfg(feature = "foyer")]
    #[async_trait::async_trait]
    impl ObjectStore for PauseFirstReadStore {
        async fn put_opts(
            &self,
            location: &Path,
            payload: object_store::PutPayload,
            opts: object_store::PutOptions,
        ) -> object_store::Result<object_store::PutResult> {
            self.inner.put_opts(location, payload, opts).await
        }

        async fn put_multipart_opts(
            &self,
            location: &Path,
            opts: object_store::PutMultipartOptions,
        ) -> object_store::Result<Box<dyn object_store::MultipartUpload>> {
            self.inner.put_multipart_opts(location, opts).await
        }

        async fn get_opts(
            &self,
            location: &Path,
            options: object_store::GetOptions,
        ) -> object_store::Result<object_store::GetResult> {
            use std::sync::atomic::Ordering;
            if options.range.is_some() {
                self.get_range_count.fetch_add(1, Ordering::SeqCst);
                if self.paused.swap(false, Ordering::SeqCst) {
                    self.first_read_started.notify_one();
                    self.release.notified().await;
                }
            }
            self.inner.get_opts(location, options).await
        }

        fn list(
            &self,
            prefix: Option<&Path>,
        ) -> futures::stream::BoxStream<'static, object_store::Result<object_store::ObjectMeta>>
        {
            self.inner.list(prefix)
        }

        async fn list_with_delimiter(
            &self,
            prefix: Option<&Path>,
        ) -> object_store::Result<object_store::ListResult> {
            self.inner.list_with_delimiter(prefix).await
        }

        fn delete_stream(
            &self,
            locations: futures::stream::BoxStream<'static, object_store::Result<Path>>,
        ) -> futures::stream::BoxStream<'static, object_store::Result<Path>> {
            self.inner.delete_stream(locations)
        }

        async fn copy_opts(
            &self,
            from: &Path,
            to: &Path,
            options: object_store::CopyOptions,
        ) -> object_store::Result<()> {
            self.inner.copy_opts(from, to, options).await
        }
    }

    fn make_store() -> Arc<dyn ObjectStore> {
        Arc::new(InMemory::new())
    }

    fn build_index(offsets: &[u64]) -> SsTableIndexOwned {
        let mut builder = flatbuffers::FlatBufferBuilder::new();
        let block_meta = offsets
            .iter()
            .enumerate()
            .map(|(block, offset)| {
                let first_key = builder.create_vector(block.to_string().as_bytes());
                BlockMeta::create(
                    &mut builder,
                    &BlockMetaArgs {
                        offset: *offset,
                        first_key: Some(first_key),
                    },
                )
            })
            .collect::<Vec<_>>();
        let block_meta = builder.create_vector(&block_meta);
        let index = SsTableIndex::create(
            &mut builder,
            &SsTableIndexArgs {
                block_meta: Some(block_meta),
            },
        );
        builder.finish(index, None);
        SsTableIndexOwned::new(Bytes::copy_from_slice(builder.finished_data())).unwrap()
    }

    async fn count_ssts_in(store: &Arc<dyn ObjectStore>) -> usize {
        store
            .list(None)
            .filter(|r| {
                future::ready(
                    r.as_ref()
                        .unwrap()
                        .location
                        .extension()
                        .unwrap()
                        .to_lowercase()
                        == "sst",
                )
            })
            .count()
            .await
    }

    #[tokio::test]
    async fn test_sst_writer_should_write_compacted_sst() {
        // given:
        let main_store = make_store();
        let format = SsTableFormat {
            block_size: 32,
            min_filter_keys: 1,
            ..SsTableFormat::default()
        };
        let ts = Arc::new(TableStore::new(
            main_store.clone(),
            format,
            Path::from(ROOT),
            None,
            TableStoreKind::Main,
            BlockCachePolicy::default(),
        ));
        let id = SsTableId::from(ulid::Ulid::new());

        // when:
        let mut writer = ts.table_writer(id, Some(Bytes::new()));
        writer
            .add(RowEntry::new_value(&[b'a'; 16], &[1u8; 16], 0))
            .await
            .unwrap();
        writer
            .add(RowEntry::new_value(&[b'b'; 16], &[2u8; 16], 0))
            .await
            .unwrap();
        writer
            .add(RowEntry::new_tombstone(&[b'c'; 16], 0))
            .await
            .unwrap();
        writer
            .add(RowEntry::new_value(&[b'd'; 16], &[4u8; 16], 0))
            .await
            .unwrap();
        let (sst, _) = writer.close().await.unwrap();

        let sst_iter_options = SstIteratorOptions {
            eager_spawn: true,
            ..SstIteratorOptions::default()
        };
        // then:
        let mut iter = SstIterator::new_owned_initialized(
            ..,
            SsTableView::identity(sst),
            ts.clone(),
            sst_iter_options,
        )
        .await
        .unwrap()
        .expect("Expected Some(iter) but got None");
        assert_iterator(
            &mut iter,
            vec![
                RowEntry::new_value(&[b'a'; 16], &[1u8; 16], 0),
                RowEntry::new_value(&[b'b'; 16], &[2u8; 16], 0),
                RowEntry::new_tombstone(&[b'c'; 16], 0),
                RowEntry::new_value(&[b'd'; 16], &[4u8; 16], 0),
            ],
        )
        .await;

        assert_eq!(count_ssts_in(&main_store).await, 1);
    }

    #[tokio::test]
    async fn should_use_streaming_upload_for_large_compacted_sst() {
        // given:
        let value_size = 11 * 1024 * 1024;
        let os = Arc::new(
            FlakyObjectStore::new(Arc::new(InMemory::new()), 0)
                .with_single_put_size_limit(1024 * 1024),
        );
        let format = SsTableFormat {
            block_size: 32,
            min_filter_keys: 1,
            ..SsTableFormat::default()
        };
        let ts = Arc::new(TableStore::new(
            os.clone(),
            format,
            Path::from(ROOT),
            None,
            TableStoreKind::Main,
            BlockCachePolicy::default(),
        ));
        let id = SsTableId::from(ulid::Ulid::new());

        let mut builder = ts.table_builder();
        builder
            .add(RowEntry::new(
                Bytes::from_static(b"key"),
                ValueDeletable::Value(Bytes::from(vec![b'x'; value_size])),
                0,
                None,
                None,
            ))
            .await
            .unwrap();
        let sst = builder.build().await.unwrap();

        // when:
        ts.write_sst(&id, &sst, Some(Bytes::new())).await.unwrap();

        // then:
        assert_eq!(os.put_attempts(), 0);
        assert_eq!(os.multipart_attempts(), 1);
        ts.open_sst(&id, Some(Bytes::new())).await.unwrap();
    }

    #[tokio::test]
    async fn close_surfaces_error_instead_of_panicking_when_shutdown_fails() {
        // A small SST flushes as a single put during shutdown. Failing
        // that put must come back as an error the caller can retry, not a
        // panic from aborting a BufWriter that has already shut down.
        let os = Arc::new(FlakyObjectStore::new(Arc::new(InMemory::new()), 1));
        let format = SsTableFormat {
            block_size: 32,
            min_filter_keys: 1,
            ..SsTableFormat::default()
        };
        let ts = Arc::new(TableStore::new(
            os.clone(),
            format,
            Path::from(ROOT),
            None,
            TableStoreKind::Main,
            BlockCachePolicy::default(),
        ));
        let id = SsTableId::from(ulid::Ulid::new());

        let mut writer = ts.table_writer(id, Some(Bytes::new()));
        writer
            .add(RowEntry::new_value(&[b'a'; 16], &[1u8; 16], 0))
            .await
            .unwrap();
        let result = writer.close().await;
        assert!(
            result.is_err(),
            "a failed shutdown must surface as an error, not a panic"
        );

        let mut writer = ts.table_writer(id, Some(Bytes::new()));
        writer
            .add(RowEntry::new_value(&[b'a'; 16], &[1u8; 16], 0))
            .await
            .unwrap();
        writer.close().await.unwrap();
        ts.open_sst(&id, Some(Bytes::new())).await.unwrap();
    }

    #[tokio::test]
    async fn test_checksum_mismatch_error_includes_path() {
        let os = Arc::new(InMemory::new());
        let format = SsTableFormat {
            block_size: 32,
            min_filter_keys: 1,
            ..SsTableFormat::default()
        };
        let ts = Arc::new(TableStore::new(
            os.clone(),
            format,
            Path::from(ROOT),
            None,
            TableStoreKind::Main,
            BlockCachePolicy::default(),
        ));
        let id = SsTableId::from(ulid::Ulid::new());

        let mut writer = ts.table_writer(id, Some(Bytes::new()));
        writer
            .add(RowEntry::new_value(&[b'a'; 16], &[1u8; 16], 0))
            .await
            .unwrap();
        writer.close().await.unwrap();

        let sst_path = ts.path(&id);
        let original = os.get(&sst_path).await.unwrap().bytes().await.unwrap();
        let mut corrupted = original.to_vec();
        corrupted[0] ^= 0x01;
        os.put(&sst_path, corrupted.into()).await.unwrap();

        let handle = ts.open_sst(&id, Some(Bytes::new())).await.unwrap();
        let err = match ts.read_blocks(&handle, 0..1, Some(Bytes::new())).await {
            Err(e) => e,
            Ok(_) => panic!("expected checksum mismatch"),
        };
        assert!(
            matches!(err, error::SlateDBError::ChecksumMismatch { path: Some(ref p) } if p == &sst_path),
            "expected ChecksumMismatch tagged with sst path, got {err:?}"
        );

        let public_msg = error::Error::from(err).to_string();
        let expected = format!("checksum mismatch in {sst_path}");
        assert!(
            public_msg.contains(&expected),
            "public error message {public_msg:?} did not contain {expected:?}"
        );
    }

    #[tokio::test]
    #[cfg(feature = "moka")]
    async fn test_tablestore_sst_and_partial_cache_hits() {
        use crate::db_cache::{moka::MokaCache, SplitCache};

        // Setup
        let os = Arc::new(InMemory::new());
        let format = SsTableFormat {
            block_size: 32,
            ..SsTableFormat::default()
        };

        let recorder = slatedb_common::metrics::MetricsRecorderHelper::noop();
        let block_cache = Arc::new(MokaCache::new());
        let meta_cache = Arc::new(MokaCache::new());

        let split_cache = Arc::new(
            SplitCache::new()
                .with_block_cache(Some(block_cache.clone()))
                .with_meta_cache(Some(meta_cache))
                .build(),
        );

        let wrapper = Arc::new(DbCacheWrapper::new(
            split_cache,
            &recorder,
            Arc::new(DefaultSystemClock::default()),
            1,
        ));
        let ts = Arc::new(TableStore::new(
            os.clone(),
            format,
            Path::from("/root"),
            Some(wrapper.clone()),
            TableStoreKind::Main,
            BlockCachePolicy::default(),
        ));

        // Create and write SST
        let id = SsTableId::from(ulid::Ulid::new());
        let mut writer = ts.table_writer(id, Some(Bytes::new()));
        let mut expected_data = Vec::with_capacity(20);
        for i in 0..20 {
            let key = [i as u8; 16];
            let value = [(i + 1) as u8; 16];
            expected_data.push((
                Vec::from(key.as_slice()),
                ValueDeletable::Value(Bytes::copy_from_slice(&value)),
            ));
            writer
                .add(RowEntry::new_value(key.as_ref(), value.as_ref(), 0))
                .await
                .unwrap();
        }
        let (handle, _) = writer.close().await.unwrap();

        // Read the index
        let index = ts
            .read_index(
                &handle,
                true,
                Some(Bytes::new()),
                &ReadTrace::new(None),
                None,
            )
            .await
            .unwrap();

        // Test 1: SST hit
        let blocks = ts
            .read_blocks_using_index(&handle, index.clone(), 0..20, true, Some(Bytes::new()))
            .await
            .unwrap();

        assert_blocks(&blocks, &expected_data).await;

        // Check that all blocks are now in cache
        for i in 0..20 {
            let offset = index.borrow().block_meta().get(i).offset();
            let cached = wrapper
                .get_block(&(handle.id, offset).into())
                .await
                .unwrap_or(None);
            assert!(
                cached.is_some(),
                "Block with offset {} should be in cache",
                offset
            );
        }

        // Partially clear the cache (remove blocks 5..10 and 15..20)
        for i in 5..10 {
            let offset = index.borrow().block_meta().get(i).offset();
            wrapper.remove(&(handle.id, offset).into()).await;
        }
        for i in 15..20 {
            let offset = index.borrow().block_meta().get(i).offset();
            wrapper.remove(&(handle.id, offset).into()).await;
        }

        // Test 2: Partial cache hit, everything should be returned since missing blocks are returned from sst
        let blocks = ts
            .read_blocks_using_index(&handle, index.clone(), 0..20, true, Some(Bytes::new()))
            .await
            .unwrap();
        assert_blocks(&blocks, &expected_data).await;

        // Check that all blocks are again in cache
        for i in 0..20 {
            let offset = index.borrow().block_meta().get(i).offset();
            let cached = wrapper
                .get_block(&(handle.id, offset).into())
                .await
                .unwrap_or(None);
            assert!(
                cached.is_some(),
                "Block with offset {} should be in cache after partial hit",
                offset
            );
        }

        // Replace SST file with an empty file
        let path = ts.path(&id);
        os.put(&path, Bytes::new().into()).await.unwrap();

        // Test 3: All blocks should be in cache after SST file is emptied
        let blocks = ts
            .read_blocks_using_index(&handle, index.clone(), 0..20, true, Some(Bytes::new()))
            .await
            .unwrap();
        assert_blocks(&blocks, &expected_data).await;

        // Check that all blocks are still in cache
        for i in 0..20 {
            let offset = index.borrow().block_meta().get(i).offset();
            assert!(
                wrapper
                    .get_block(&(handle.id, offset).into())
                    .await
                    .unwrap_or(None)
                    .is_some(),
                "Block with offset {} should be in cache after SST emptying",
                offset
            );
        }

        // Test 4: Verify that reading specific ranges still works after SST file is emptied
        let blocks = ts
            .read_blocks_using_index(&handle, index.clone(), 5..10, true, Some(Bytes::new()))
            .await
            .unwrap();
        assert_blocks(&blocks, &expected_data[5..10]).await;

        let blocks = ts
            .read_blocks_using_index(&handle, index.clone(), 15..20, true, Some(Bytes::new()))
            .await
            .unwrap();
        assert_blocks(&blocks, &expected_data[15..20]).await;
    }

    #[tokio::test]
    async fn test_read_index_honors_cache_blocks() {
        let main_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let format = SsTableFormat {
            min_filter_keys: u32::MAX,
            ..SsTableFormat::default()
        };
        let writer = TableStore::new(
            main_store.clone(),
            format.clone(),
            Path::from(ROOT),
            None,
            TableStoreKind::Main,
            BlockCachePolicy::default(),
        );

        let mut builder = writer.table_builder();
        builder
            .add(RowEntry::new_value(b"key1", b"value1", 0))
            .await
            .unwrap();
        builder
            .add(RowEntry::new_value(b"key2", b"value2", 0))
            .await
            .unwrap();
        let id = SsTableId::from(ulid::Ulid::new());
        let handle = writer
            .write_sst(&id, &builder.build().await.unwrap(), Some(Bytes::new()))
            .await
            .unwrap();

        let meta_cache = Arc::new(TestCache::new());
        let cache = Arc::new(
            SplitCache::new()
                .with_meta_cache(Some(meta_cache.clone()))
                .build(),
        );
        let reader = TableStore::new(
            main_store.clone(),
            format,
            Path::from(ROOT),
            Some(cache),
            TableStoreKind::Main,
            BlockCachePolicy::default(),
        );
        assert_eq!(meta_cache.entry_count(), 0);

        let _ = reader
            .read_index(
                &handle,
                false,
                Some(Bytes::new()),
                &ReadTrace::new(None),
                None,
            )
            .await
            .unwrap();
        assert!(meta_cache
            .get_index(&(handle.id, handle.info.index_offset).into())
            .await
            .unwrap()
            .is_none());

        let _ = reader
            .read_index(
                &handle,
                true,
                Some(Bytes::new()),
                &ReadTrace::new(None),
                None,
            )
            .await
            .unwrap();
        assert!(meta_cache
            .get_index(&(handle.id, handle.info.index_offset).into())
            .await
            .unwrap()
            .is_some());
    }

    #[derive(Clone, Copy, Debug)]
    enum MetadataRead {
        Filters,
        Index,
    }

    #[rstest]
    #[case::cache_enabled_with_caching_hit(true, true, true)]
    #[case::cache_enabled_with_caching_miss(true, true, false)]
    #[case::cache_enabled_without_caching_hit(true, false, true)]
    #[case::cache_enabled_without_caching_miss(true, false, false)]
    #[case::cache_disabled_without_caching(false, false, false)]
    #[test]
    fn test_read_filter_and_index_span_fields(
        #[values(MetadataRead::Filters, MetadataRead::Index)] metadata_read: MetadataRead,
        #[values(SstTraceLevel::SortedRun(7), SstTraceLevel::L0)] sst_level: SstTraceLevel,
        #[case] cache_enabled: bool,
        #[case] cache_blocks: bool,
        #[case] cache_hit: bool,
    ) {
        const TRACE_ID: &str = "metadata-read-trace";

        let span_name = match metadata_read {
            MetadataRead::Filters => "slatedb.read.read_filters",
            MetadataRead::Index => "slatedb.read.read_index",
        };
        let span_recorder = SpanRecorder::for_name(span_name);
        let subscriber = tracing_subscriber::registry().with(span_recorder.clone());
        let id = tracing::subscriber::with_default(subscriber, || {
            tokio_test::block_on(async {
                let main_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
                let format = SsTableFormat {
                    min_filter_keys: 1,
                    ..SsTableFormat::default()
                };
                let writer = TableStore::new(
                    main_store.clone(),
                    format.clone(),
                    Path::from(ROOT),
                    None,
                    TableStoreKind::Main,
                    BlockCachePolicy::default(),
                );
                let mut builder = writer.table_builder();
                builder
                    .add(RowEntry::new_value(b"key1", b"value1", 0))
                    .await
                    .unwrap();
                builder
                    .add(RowEntry::new_value(b"key2", b"value2", 0))
                    .await
                    .unwrap();
                let id = SsTableId::from(ulid::Ulid::new());
                let handle = writer
                    .write_sst(&id, &builder.build().await.unwrap(), Some(Bytes::new()))
                    .await
                    .unwrap();

                let meta_cache = Arc::new(TestCache::new());
                let cache: Option<Arc<dyn DbCache>> = if cache_enabled {
                    Some(Arc::new(
                        SplitCache::new().with_meta_cache(Some(meta_cache)).build(),
                    ))
                } else {
                    None
                };
                let reader = TableStore::new(
                    main_store,
                    format,
                    Path::from(ROOT),
                    cache,
                    TableStoreKind::Main,
                    BlockCachePolicy::default(),
                );

                // Fill the cache before the traced read.
                if cache_hit {
                    match metadata_read {
                        MetadataRead::Filters => {
                            reader
                                .read_filters(
                                    &handle,
                                    true,
                                    Some(Bytes::new()),
                                    &ReadTrace::new(None),
                                    None,
                                )
                                .await
                                .unwrap();
                        }
                        MetadataRead::Index => {
                            reader
                                .read_index(
                                    &handle,
                                    true,
                                    Some(Bytes::new()),
                                    &ReadTrace::new(None),
                                    None,
                                )
                                .await
                                .unwrap();
                        }
                    }
                }

                let trace = ReadTrace::new(Some(TracingOptions::new(TRACE_ID)));
                match metadata_read {
                    MetadataRead::Filters => {
                        let filters = reader
                            .read_filters(
                                &handle,
                                cache_blocks,
                                Some(Bytes::new()),
                                &trace,
                                Some(&sst_level),
                            )
                            .await
                            .unwrap();
                        assert!(!filters.is_empty());
                    }
                    MetadataRead::Index => {
                        let index = reader
                            .read_index(
                                &handle,
                                cache_blocks,
                                Some(Bytes::new()),
                                &trace,
                                Some(&sst_level),
                            )
                            .await
                            .unwrap();
                        assert!(!index.borrow().block_meta().is_empty());
                    }
                }
                id
            })
        });

        let span = span_recorder.only_span();
        assert_eq!(
            span.fields,
            HashMap::from([
                ("trace_id".to_string(), TRACE_ID.to_string()),
                ("sst_id".to_string(), id.value().to_string()),
                (
                    "sst_level".to_string(),
                    match sst_level {
                        SstTraceLevel::SortedRun(level) => format!("sorted_run:{level}"),
                        SstTraceLevel::L0 => "l0".to_string(),
                    }
                ),
                ("cached".to_string(), cache_hit.to_string()),
            ])
        );
    }

    #[tokio::test]
    async fn test_read_filter_honors_cache_blocks() {
        let main_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let format = SsTableFormat {
            min_filter_keys: 1,
            ..SsTableFormat::default()
        };
        let writer = TableStore::new(
            main_store.clone(),
            format.clone(),
            Path::from(ROOT),
            None,
            TableStoreKind::Main,
            BlockCachePolicy::default(),
        );

        let mut builder = writer.table_builder();
        builder
            .add(RowEntry::new_value(b"key1", b"value1", 0))
            .await
            .unwrap();
        builder
            .add(RowEntry::new_value(b"key2", b"value2", 0))
            .await
            .unwrap();
        let id = SsTableId::from(ulid::Ulid::new());
        let handle = writer
            .write_sst(&id, &builder.build().await.unwrap(), Some(Bytes::new()))
            .await
            .unwrap();

        let meta_cache = Arc::new(TestCache::new());
        let cache = Arc::new(
            SplitCache::new()
                .with_meta_cache(Some(meta_cache.clone()))
                .build(),
        );
        let reader = TableStore::new(
            main_store.clone(),
            format,
            Path::from(ROOT),
            Some(cache),
            TableStoreKind::Main,
            BlockCachePolicy::default(),
        );
        assert_eq!(meta_cache.entry_count(), 0);

        let filters = reader
            .read_filters(
                &handle,
                false,
                Some(Bytes::new()),
                &ReadTrace::new(None),
                None,
            )
            .await
            .unwrap();
        assert!(!filters.is_empty());
        assert!(meta_cache
            .get_filter(&(handle.id, handle.info.filter_offset).into())
            .await
            .unwrap()
            .is_none());

        let _ = reader
            .read_filters(
                &handle,
                true,
                Some(Bytes::new()),
                &ReadTrace::new(None),
                None,
            )
            .await
            .unwrap();
        assert!(meta_cache
            .get_filter(&(handle.id, handle.info.filter_offset).into())
            .await
            .unwrap()
            .is_some());
    }

    #[tokio::test]
    async fn test_read_stats_honors_cache_blocks() {
        let main_store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let format = SsTableFormat::default();
        let writer = TableStore::new(
            main_store.clone(),
            format.clone(),
            Path::from(ROOT),
            None,
            TableStoreKind::Main,
            BlockCachePolicy::default(),
        );

        let mut builder = writer.table_builder();
        builder
            .add(RowEntry::new_value(b"key1", b"value1", 0))
            .await
            .unwrap();
        builder
            .add(RowEntry::new_tombstone(b"key2", 0))
            .await
            .unwrap();
        let id = SsTableId::from(ulid::Ulid::new());
        let handle = writer
            .write_sst(&id, &builder.build().await.unwrap(), Some(Bytes::new()))
            .await
            .unwrap();
        assert!(handle.info.stats_len > 0);

        let meta_cache = Arc::new(TestCache::new());
        let cache = Arc::new(
            SplitCache::new()
                .with_meta_cache(Some(meta_cache.clone()))
                .build(),
        );
        let reader = TableStore::new(
            main_store.clone(),
            format,
            Path::from(ROOT),
            Some(cache),
            TableStoreKind::Main,
            BlockCachePolicy::default(),
        );
        assert_eq!(meta_cache.entry_count(), 0);

        let stats = reader
            .read_stats(&handle, false, Some(Bytes::new()))
            .await
            .unwrap();
        assert!(stats.is_some());
        assert!(meta_cache
            .get_stats(&(handle.id, handle.info.stats_offset).into())
            .await
            .unwrap()
            .is_none());

        let _ = reader
            .read_stats(&handle, true, Some(Bytes::new()))
            .await
            .unwrap();
        assert!(meta_cache
            .get_stats(&(handle.id, handle.info.stats_offset).into())
            .await
            .unwrap()
            .is_some());
    }

    #[tokio::test]
    async fn test_write_sst_should_write_cache() {
        let os = Arc::new(InMemory::new());
        let recorder = slatedb_common::metrics::MetricsRecorderHelper::noop();

        let block_cache = Arc::new(TestCache::new());
        let meta_cache = Arc::new(TestCache::new());
        let split_cache = Arc::new(
            SplitCache::new()
                .with_block_cache(Some(block_cache.clone()))
                .with_meta_cache(Some(meta_cache))
                .build(),
        );

        let wrapper = Arc::new(DbCacheWrapper::new(
            split_cache,
            &recorder,
            Arc::new(DefaultSystemClock::default()),
            1,
        ));
        let ts = Arc::new(TableStore::new(
            os.clone(),
            SsTableFormat::default(),
            Path::from("/root"),
            Some(wrapper.clone()),
            TableStoreKind::Main,
            BlockCachePolicy::default(),
        ));
        let id = SsTableId::from(ulid::Ulid::new());
        let sst = build_test_sst(&ts.sst_format, 3).await;
        let sst_bytes = sst.remaining_as_bytes();
        let sst_info = sst.info.clone();

        ts.write_sst(&id, &sst, Some(Bytes::new())).await.unwrap();

        let index = ts
            .sst_format
            .read_index_raw(&sst_info, &sst_bytes)
            .await
            .unwrap();
        let block_metas = index.borrow().block_meta();
        for i in 0..block_metas.len() {
            let block_meta = block_metas.get(i);
            let block = ts
                .sst_format
                .read_block_raw(&sst_info, &index, i, &sst_bytes)
                .await
                .unwrap();
            let cached_block = wrapper
                .get_block(&(id, block_meta.offset()).into())
                .await
                .unwrap();
            assert!(cached_block.is_some());
            assert!(block == *cached_block.unwrap().block().unwrap());
        }
    }

    #[tokio::test]
    async fn test_write_sst_should_not_write_cache() {
        let os = Arc::new(InMemory::new());
        let recorder = slatedb_common::metrics::MetricsRecorderHelper::noop();
        let cache = Arc::new(TestCache::new());
        let wrapper = Arc::new(DbCacheWrapper::new(
            cache.clone(),
            &recorder,
            Arc::new(DefaultSystemClock::default()),
            1,
        ));
        let ts = Arc::new(TableStore::new(
            os.clone(),
            SsTableFormat::default(),
            Path::from("/root"),
            Some(wrapper),
            TableStoreKind::Main,
            BlockCachePolicy::default().with_flush_targets(&[CacheTarget::Filters]),
        ));
        let id = SsTableId::from(ulid::Ulid::new());
        let sst = build_test_sst(&ts.sst_format, 3).await;
        let sst_bytes = sst.remaining_as_bytes();
        let sst_info = sst.info.clone();

        ts.write_sst(&id, &sst, Some(Bytes::new())).await.unwrap();

        let index = ts
            .sst_format
            .read_index_raw(&sst_info, &sst_bytes)
            .await
            .unwrap();
        let block_metas = index.borrow().block_meta();
        for i in 0..block_metas.len() {
            let block_meta = block_metas.get(i);
            let cached_block = cache
                .get_block(&(id, block_meta.offset()).into())
                .await
                .unwrap();
            assert!(cached_block.is_none());
        }
    }

    #[rstest]
    #[case::filters_only(&[CacheTarget::Filters])]
    #[case::index_and_filters(&[CacheTarget::Index, CacheTarget::Filters])]
    #[case::all(&[
        CacheTarget::data::<&[u8], _>(..),
        CacheTarget::Filters,
        CacheTarget::Index,
        CacheTarget::Stats,
    ])]
    #[tokio::test]
    async fn write_sst_should_cache_only_selected_components(#[case] selected: &[CacheTarget]) {
        let cache = Arc::new(TestCache::new());
        let ts = Arc::new(TableStore::new(
            Arc::new(InMemory::new()),
            SsTableFormat::default(),
            Path::from("/root"),
            Some(cache.clone()),
            TableStoreKind::Main,
            BlockCachePolicy::default().with_flush_targets(selected),
        ));
        let id = SsTableId::from(ulid::Ulid::new());
        let sst = build_test_sst(&ts.sst_format, 3).await;
        let data_key: CachedKey = (id, sst.unconsumed_blocks[0].offset).into();
        let index_key: CachedKey = (id, sst.info.index_offset).into();
        let filter_key: CachedKey = (id, sst.info.filter_offset).into();
        let stats_key: CachedKey = (id, sst.info.stats_offset).into();

        ts.write_sst(&id, &sst, Some(Bytes::new())).await.unwrap();

        assert_eq!(
            cache.get_block(&data_key).await.unwrap().is_some(),
            selected.iter().any(|c| matches!(c, CacheTarget::Data(_)))
        );
        assert_eq!(
            cache.get_index(&index_key).await.unwrap().is_some(),
            selected.contains(&CacheTarget::Index)
        );
        assert_eq!(
            cache.get_filter(&filter_key).await.unwrap().is_some(),
            selected.contains(&CacheTarget::Filters)
        );
        assert_eq!(
            cache.get_stats(&stats_key).await.unwrap().is_some(),
            selected.contains(&CacheTarget::Stats)
        );
    }

    #[tokio::test]
    async fn evict_sst_targets_should_remove_only_blocks_in_the_data_range() {
        let cache = Arc::new(TestCache::new());
        let ts = Arc::new(TableStore::new(
            Arc::new(InMemory::new()),
            SsTableFormat::default(),
            Path::from("/root"),
            Some(cache.clone()),
            TableStoreKind::Main,
            BlockCachePolicy::default(),
        ));
        let id = SsTableId::from(ulid::Ulid::new());
        let sst = build_test_sst(&ts.sst_format, 3).await;
        let block_keys: Vec<CachedKey> = sst
            .unconsumed_blocks
            .iter()
            .map(|block| (id, block.offset).into())
            .collect();
        // A range inside the second block. It starts just after the block's
        // first key, because a range that starts on a block boundary also
        // covers the block before it.
        let (first, last) = sst.unconsumed_blocks[1].key_span.clone().unwrap();
        let start = [first.as_ref(), &[0u8]].concat();
        let handle = ts.write_sst(&id, &sst, Some(Bytes::new())).await.unwrap();
        for key in &block_keys {
            assert!(cache.get_block(key).await.unwrap().is_some());
        }

        ts.evict_sst_targets_from_cache(
            &handle,
            &[CacheTarget::data(start.as_slice()..=last.as_ref())],
            Some(Bytes::new()),
        )
        .await;

        let cached: Vec<bool> = future::join_all(
            block_keys
                .iter()
                .map(|key| async { cache.get_block(key).await.unwrap().is_some() }),
        )
        .await;
        let expected: Vec<bool> = (0..block_keys.len()).map(|i| i != 1).collect();
        assert_eq!(cached, expected);
    }

    /// An empty range names no blocks. Without the guard the block spanning
    /// its start would be evicted.
    #[tokio::test]
    async fn evict_sst_targets_should_ignore_an_empty_data_range() {
        let cache = Arc::new(TestCache::new());
        let ts = Arc::new(TableStore::new(
            Arc::new(InMemory::new()),
            SsTableFormat::default(),
            Path::from("/root"),
            Some(cache.clone()),
            TableStoreKind::Main,
            BlockCachePolicy::default(),
        ));
        let id = SsTableId::from(ulid::Ulid::new());
        let sst = build_test_sst(&ts.sst_format, 3).await;
        let block_keys: Vec<CachedKey> = sst
            .unconsumed_blocks
            .iter()
            .map(|block| (id, block.offset).into())
            .collect();
        let (first, _) = sst.unconsumed_blocks[1].key_span.clone().unwrap();
        let start = [first.as_ref(), &[0u8]].concat();
        let handle = ts.write_sst(&id, &sst, Some(Bytes::new())).await.unwrap();

        ts.evict_sst_targets_from_cache(
            &handle,
            &[CacheTarget::data(start.as_slice()..start.as_slice())],
            Some(Bytes::new()),
        )
        .await;

        for key in &block_keys {
            assert!(cache.get_block(key).await.unwrap().is_some());
        }
    }

    #[rstest]
    #[case::nothing(&[])]
    #[case::metadata(&[CacheTarget::Index, CacheTarget::Filters, CacheTarget::Stats])]
    #[case::data_only(&[CacheTarget::data::<&[u8], _>(..)])]
    #[case::all(&[
        CacheTarget::data::<&[u8], _>(..),
        CacheTarget::Filters,
        CacheTarget::Index,
        CacheTarget::Stats,
    ])]
    #[tokio::test]
    async fn evict_sst_targets_should_remove_only_selected_components(
        #[case] selected: &[CacheTarget],
    ) {
        let cache = Arc::new(TestCache::new());
        let all = [
            CacheTarget::data::<&[u8], _>(..),
            CacheTarget::Filters,
            CacheTarget::Index,
            CacheTarget::Stats,
        ];
        let ts = Arc::new(TableStore::new(
            Arc::new(InMemory::new()),
            SsTableFormat::default(),
            Path::from("/root"),
            Some(cache.clone()),
            TableStoreKind::Main,
            BlockCachePolicy::default().with_flush_targets(&all),
        ));
        let id = SsTableId::from(ulid::Ulid::new());
        let sst = build_test_sst(&ts.sst_format, 3).await;
        let data_key: CachedKey = (id, sst.unconsumed_blocks[0].offset).into();
        let index_key: CachedKey = (id, sst.info.index_offset).into();
        let filter_key: CachedKey = (id, sst.info.filter_offset).into();
        let stats_key: CachedKey = (id, sst.info.stats_offset).into();
        let handle = ts.write_sst(&id, &sst, Some(Bytes::new())).await.unwrap();
        assert_eq!(cache.entry_count(), 3 + sst.unconsumed_blocks.len() as u64);

        ts.evict_sst_targets_from_cache(&handle, selected, Some(Bytes::new()))
            .await;

        assert_eq!(
            cache.get_block(&data_key).await.unwrap().is_none(),
            selected.iter().any(|c| matches!(c, CacheTarget::Data(_)))
        );
        assert_eq!(
            cache.get_index(&index_key).await.unwrap().is_none(),
            selected.contains(&CacheTarget::Index)
        );
        assert_eq!(
            cache.get_filter(&filter_key).await.unwrap().is_none(),
            selected.contains(&CacheTarget::Filters)
        );
        assert_eq!(
            cache.get_stats(&stats_key).await.unwrap().is_none(),
            selected.contains(&CacheTarget::Stats)
        );
    }

    #[tokio::test]
    async fn streaming_writer_should_use_block_cache_but_skip_compactor_reads() {
        let os = Arc::new(InMemory::new());
        let cache = Arc::new(TestCache::new());
        let format = SsTableFormat {
            block_size: 32,
            min_filter_keys: 1,
            ..SsTableFormat::default()
        };
        let ts = Arc::new(TableStore::new(
            os.clone(),
            format,
            Path::from("/root"),
            Some(cache.clone()),
            TableStoreKind::Compactor,
            BlockCachePolicy::default().with_compaction_output_targets(&[
                CacheTarget::data::<&[u8], _>(..),
                CacheTarget::Index,
                CacheTarget::Stats,
            ]),
        ));
        let id = SsTableId::from(ulid::Ulid::new());
        let mut writer = ts.table_writer(id, Some(Bytes::new()));
        for i in 0..4 {
            writer
                .add(RowEntry::new_value(&[b'a' + i; 16], &[i; 16], 0))
                .await
                .unwrap();
        }

        let (handle, _) = writer.close().await.unwrap();
        let index_key: CachedKey = (id, handle.info.index_offset).into();
        let filter_key: CachedKey = (id, handle.info.filter_offset).into();
        let stats_key: CachedKey = (id, handle.info.stats_offset).into();
        let index = cache
            .get_index(&index_key)
            .await
            .unwrap()
            .unwrap()
            .sst_index()
            .unwrap();

        assert!(ts.cache().is_some());
        assert!(cache.get_filter(&filter_key).await.unwrap().is_none());
        assert!(cache.get_stats(&stats_key).await.unwrap().is_some());
        for block_meta in index.borrow().block_meta().iter() {
            let data_key: CachedKey = (id, block_meta.offset()).into();
            assert!(cache.get_block(&data_key).await.unwrap().is_some());
        }

        // Delete the SST from the object store and verify that the cache won't
        // be used and reading the index will just return an error.
        os.delete(&ts.path(&id)).await.unwrap();
        assert!(ts
            .read_index(
                &handle,
                false,
                Some(Bytes::new()),
                &ReadTrace::new(None),
                None,
            )
            .await
            .is_err());
    }

    #[tokio::test]
    async fn write_sst_should_cache_only_blocks_in_data_range() {
        let cache = Arc::new(TestCache::new());
        let format = SsTableFormat {
            block_size: 32,
            min_filter_keys: 1,
            ..SsTableFormat::default()
        };
        let ts = Arc::new(TableStore::new(
            Arc::new(InMemory::new()),
            format,
            Path::from("/root"),
            Some(cache.clone()),
            TableStoreKind::Main,
            BlockCachePolicy::default().with_flush_targets(&[CacheTarget::data(
                [b'b'; 16].as_slice()..=[b'c'; 16].as_slice(),
            )]),
        ));
        // single-entry blocks for keys aa.., bb.., cc.., dd..
        let mut builder = ts.table_builder();
        for i in 0..4 {
            builder
                .add(RowEntry::new_value(&[b'a' + i; 16], &[i; 16], 0))
                .await
                .unwrap();
        }
        let sst = builder.build().await.unwrap();
        assert_eq!(sst.unconsumed_blocks.len(), 4);
        let id = SsTableId::from(ulid::Ulid::new());

        ts.write_sst(&id, &sst, Some(Bytes::new())).await.unwrap();

        for (block, expected) in sst.unconsumed_blocks.iter().zip([false, true, true, false]) {
            let data_key: CachedKey = (id, block.offset).into();
            assert_eq!(
                cache.get_block(&data_key).await.unwrap().is_some(),
                expected
            );
        }
    }

    #[tokio::test]
    async fn streaming_writer_should_cache_only_blocks_in_data_range() {
        let cache = Arc::new(TestCache::new());
        let format = SsTableFormat {
            block_size: 32,
            min_filter_keys: 1,
            ..SsTableFormat::default()
        };
        let ts = Arc::new(TableStore::new(
            Arc::new(InMemory::new()),
            format,
            Path::from("/root"),
            Some(cache.clone()),
            TableStoreKind::Compactor,
            BlockCachePolicy::default().with_compaction_output_targets(&[
                CacheTarget::data([b'b'; 16].as_slice()..=[b'c'; 16].as_slice()),
                CacheTarget::Index,
            ]),
        ));
        let id = SsTableId::from(ulid::Ulid::new());
        // single-entry blocks for keys aa.., bb.., cc.., dd..
        let mut writer = ts.table_writer(id, Some(Bytes::new()));
        for i in 0..4 {
            writer
                .add(RowEntry::new_value(&[b'a' + i; 16], &[i; 16], 0))
                .await
                .unwrap();
        }

        let (handle, _) = writer.close().await.unwrap();

        let index_key: CachedKey = (id, handle.info.index_offset).into();
        let index = cache
            .get_index(&index_key)
            .await
            .unwrap()
            .unwrap()
            .sst_index()
            .unwrap();
        let block_metas = index.borrow().block_meta();
        assert_eq!(block_metas.len(), 4);
        for (i, expected) in [false, true, true, false].into_iter().enumerate() {
            let data_key: CachedKey = (id, block_metas.get(i).offset()).into();
            assert_eq!(
                cache.get_block(&data_key).await.unwrap().is_some(),
                expected
            );
        }
    }

    #[tokio::test]
    async fn streaming_writer_should_cache_only_index_and_filters_for_compaction_output() {
        let cache = Arc::new(TestCache::new());
        // The default policy requests only the index and filter
        // compaction-output components; stats stay uncached, and data blocks
        // are streamed out before close so they can never be inserted.
        let ts = Arc::new(TableStore::new(
            Arc::new(InMemory::new()),
            SsTableFormat::default(),
            Path::from("/root"),
            Some(cache.clone()),
            TableStoreKind::Compactor,
            BlockCachePolicy::default(),
        ));
        let id = SsTableId::from(ulid::Ulid::new());
        let mut writer = ts.table_writer(id, Some(Bytes::new()));
        writer
            .add(RowEntry::new_value(b"key", b"value", 0))
            .await
            .unwrap();

        let (handle, _) = writer.close().await.unwrap();

        assert_eq!(cache.entry_count(), 2);
        assert!(cache
            .get_index(&(id, handle.info.index_offset).into())
            .await
            .unwrap()
            .is_some());
        assert!(cache
            .get_filter(&(id, handle.info.filter_offset).into())
            .await
            .unwrap()
            .is_some());
        assert!(cache
            .get_stats(&(id, handle.info.stats_offset).into())
            .await
            .unwrap()
            .is_none());
    }

    #[allow(dead_code)]
    async fn assert_blocks(blocks: &VecDeque<Arc<Block>>, expected: &[(Vec<u8>, ValueDeletable)]) {
        let mut block_iter = blocks.iter();
        let mut expected_iter = expected.iter();

        while let (Some(block), Some(expected_item)) = (block_iter.next(), expected_iter.next()) {
            let mut iter = BlockIteratorLatest::new_ascending(block.clone());
            let kv: KeyValue = iter.next().await.unwrap().unwrap().into();
            assert_eq!(kv.key, expected_item.0);
            assert_eq!(ValueDeletable::Value(kv.value), expected_item.1);
        }

        assert!(block_iter.next().is_none());
        assert!(expected_iter.next().is_none());
    }

    #[tokio::test]
    async fn test_list_compacted_ssts() {
        use ulid::Ulid;

        let main_store = make_store();
        let format = SsTableFormat {
            block_size: 32,
            min_filter_keys: 1,
            ..SsTableFormat::default()
        };
        let ts = Arc::new(TableStore::new(
            main_store.clone(),
            format,
            Path::from(ROOT),
            None,
            TableStoreKind::Main,
            BlockCachePolicy::default(),
        ));

        // Create id1, id2, and i3 as three random UUIDs that have been sorted ascending.
        // Need to do this because the Ulids are sometimes generated in the same millisecond
        // and the random suffix is used to break the tie, which might be out of order.
        let mut ulids = (0..3).map(|_| Ulid::new()).collect::<Vec<Ulid>>();
        ulids.sort();
        let (id1, id2, id3) = (
            SsTableId::from(ulids[0]),
            SsTableId::from(ulids[1]),
            SsTableId::from(ulids[2]),
        );

        let path1 = ts.path(&id1);
        let path2 = ts.path(&id2);
        let path3 = ts.path(&id3);

        main_store.put(&path1, Bytes::new().into()).await.unwrap();
        main_store.put(&path2, Bytes::new().into()).await.unwrap();
        main_store.put(&path3, Bytes::new().into()).await.unwrap();

        let ssts = ts.list_compacted_ssts(..).await.unwrap();
        assert_eq!(ssts.len(), 3);
        assert_eq!(ssts[0].id, id1);
        assert_eq!(ssts[1].id, id2);
        assert_eq!(ssts[2].id, id3);

        let ssts = ts
            .list_compacted_ssts(id2.value()..id3.value())
            .await
            .unwrap();
        assert_eq!(ssts.len(), 1);
        assert_eq!(ssts[0].id, id2);

        let ssts = ts.list_compacted_ssts(id2.value()..).await.unwrap();
        assert_eq!(ssts.len(), 2);
        assert_eq!(ssts[0].id, id2);
        assert_eq!(ssts[1].id, id3);

        let ssts = ts.list_compacted_ssts(..id3.value()).await.unwrap();
        assert_eq!(ssts.len(), 2);
        assert_eq!(ssts[0].id, id1);
        assert_eq!(ssts[1].id, id2);
    }

    #[tokio::test]
    async fn test_retry_write_sst_on_timeout_and_verify_bytes() {
        // Given a flaky store that times out on the first put_opts
        let base: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let flaky = Arc::new(FlakyObjectStore::new(base.clone(), 1));
        let retrying = Arc::new(RetryingObjectStore::new(
            flaky.clone(),
            Arc::new(DbRand::default()),
            Arc::new(DefaultSystemClock::new()),
            None,
        ));

        let format = SsTableFormat {
            block_size: 64,
            min_filter_keys: 1,
            ..SsTableFormat::default()
        };
        let ts = Arc::new(TableStore::new(
            retrying,
            format.clone(),
            Path::from(ROOT),
            None,
            TableStoreKind::Main,
            BlockCachePolicy::default(),
        ));

        // Build an SST and compute expected bytes
        let id = SsTableId::from(ulid::Ulid::new());
        let sst = build_test_sst(&format, 3).await;
        let expected_bytes = sst.remaining_as_bytes();

        // When writing via TableStore (should retry once)
        ts.write_sst(&id, &sst, Some(Bytes::new())).await.unwrap();

        // Then: a retry happened
        assert!(flaky.put_attempts() >= 2);

        // And: the stored file bytes match exactly
        let path = ts.path(&id);
        let actual = base.get(&path).await.unwrap().bytes().await.unwrap();
        assert_eq!(actual, expected_bytes);
    }

    #[tokio::test]
    async fn test_delete_compacted_sst() {
        let main_store = make_store();
        let format = SsTableFormat {
            block_size: 32,
            min_filter_keys: 1,
            ..SsTableFormat::default()
        };
        let ts = Arc::new(TableStore::new(
            main_store.clone(),
            format,
            Path::from(ROOT),
            None,
            TableStoreKind::Main,
            BlockCachePolicy::default(),
        ));

        let id1 = SsTableId::from(ulid::Ulid::new());
        let id2 = SsTableId::from(ulid::Ulid::new());
        let path1 = ts.path(&id1);
        let path2 = ts.path(&id2);
        main_store.put(&path1, Bytes::new().into()).await.unwrap();
        main_store.put(&path2, Bytes::new().into()).await.unwrap();

        let ssts = ts.list_compacted_ssts(..).await.unwrap();
        assert_eq!(ssts.len(), 2);

        ts.delete_sst(&id1).await.unwrap();

        let ssts = ts.list_compacted_ssts(..).await.unwrap();
        assert_eq!(ssts.len(), 1);
        assert_eq!(ssts[0].id, id2);

        assert_eq!(count_ssts_in(&main_store).await, 1);
    }

    #[tokio::test]
    async fn test_metadata_for_compacted_sst() {
        let main_store = make_store();
        let ts = Arc::new(TableStore::new(
            main_store.clone(),
            SsTableFormat::default(),
            Path::from(ROOT),
            None,
            TableStoreKind::Main,
            BlockCachePolicy::default(),
        ));
        let id = SsTableId::from(ulid::Ulid::new());
        let path = ts.path(&id);
        let bytes = Bytes::from_static(b"compacted");
        main_store.put(&path, bytes.clone().into()).await.unwrap();

        let metadata = ts.metadata(&id, Some(Bytes::new())).await.unwrap();
        assert_eq!(metadata.size, bytes.len() as u64);
        assert_eq!(metadata.location, path);
    }

    #[rstest]
    #[case::ascending_one_block(IterationOrder::Ascending, 0, 100, 0..1)]
    #[case::ascending_crosses_boundary(IterationOrder::Ascending, 0, 101, 0..2)]
    #[case::ascending_exact_boundary(IterationOrder::Ascending, 0, 250, 0..2)]
    #[case::ascending_exhausts_sst(IterationOrder::Ascending, 1, 1_000, 1..4)]
    #[case::descending_one_block(IterationOrder::Descending, 3, 200, 3..4)]
    #[case::descending_crosses_boundary(IterationOrder::Descending, 3, 201, 2..4)]
    #[case::descending_exact_boundary(IterationOrder::Descending, 3, 250, 2..4)]
    #[case::descending_exhausts_sst(IterationOrder::Descending, 2, 1_000, 0..3)]
    fn block_range_for_target_bytes_is_minimal(
        #[case] order: IterationOrder,
        #[case] first_block: usize,
        #[case] target_bytes: usize,
        #[case] expected: Range<usize>,
    ) {
        let table_store = TableStore::new(
            make_store(),
            SsTableFormat::default(),
            Path::from(ROOT),
            None,
            TableStoreKind::Main,
            BlockCachePolicy::default(),
        );
        let handle = SsTableHandle::new(
            SsTableId::from(ulid::Ulid::new()),
            SST_FORMAT_VERSION_LATEST,
            SsTableInfo {
                index_offset: 500,
                filter_offset: 500,
                ..SsTableInfo::default()
            },
        );
        // Encoded block sizes are 100, 150, 50, and 200 bytes.
        let index = build_index(&[0, 100, 250, 300]);

        let actual = table_store.block_range_for_target_bytes(
            &handle,
            &index,
            first_block,
            target_bytes,
            order,
        );

        assert_eq!(actual, expected);
    }

    #[rstest]
    #[case::filter_precedes_index(1, 500, 700)]
    #[case::index_follows_data_without_filter(0, 700, 500)]
    fn block_range_for_target_bytes_uses_format_for_last_block_end(
        #[case] filter_len: u64,
        #[case] filter_offset: u64,
        #[case] index_offset: u64,
    ) {
        let table_store = TableStore::new(
            make_store(),
            SsTableFormat::default(),
            Path::from(ROOT),
            None,
            TableStoreKind::Main,
            BlockCachePolicy::default(),
        );
        let handle = SsTableHandle::new(
            SsTableId::from(ulid::Ulid::new()),
            SST_FORMAT_VERSION_LATEST,
            SsTableInfo {
                index_offset,
                filter_offset,
                filter_len,
                ..SsTableInfo::default()
            },
        );
        let index = build_index(&[0, 100, 250, 300]);

        let actual = table_store.block_range_for_target_bytes(
            &handle,
            &index,
            3,
            201,
            IterationOrder::Descending,
        );

        // The last block ends at byte 500 and is 200 bytes, so one preceding
        // block is required to meet a 201-byte target.
        assert_eq!(actual, 2..4);
    }

    /// End-to-end test: concurrent index reads through `TableStore` issue a single
    /// object-store request to load the index, even when the underlying object store
    /// is slow.
    ///
    /// Determinism is anchored at two points:
    ///
    /// 1. The first range-bounded `get_opts` call to the wrapped object store signals
    ///    `first_read_started` and parks on `release`. Once we see that signal we
    ///    know task A's load is in flight inside foyer's spawned loader task.
    /// 2. `tokio::join!(task_b, release_task)` polls members in source order. Task
    ///    B's first poll synchronously joins foyer's in-flight fetch (no second
    ///    object-store call); the release fires only after B has registered as a
    ///    waiter.
    #[cfg(feature = "foyer")]
    #[tokio::test]
    async fn dedups_concurrent_reads_through_object_store() {
        use crate::db_cache::foyer::FoyerCache;
        use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
        use tokio::sync::Notify;

        // given: an SST written through a plain in-memory store
        let inner: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let format = SsTableFormat::default();

        let writer = TableStore::new(
            inner.clone(),
            format.clone(),
            Path::from(ROOT),
            None,
            TableStoreKind::Main,
            BlockCachePolicy::default(),
        );
        let mut builder = writer.table_builder();
        builder
            .add(RowEntry::new_value(b"k1", b"v1", 0))
            .await
            .unwrap();
        builder
            .add(RowEntry::new_value(b"k2", b"v2", 0))
            .await
            .unwrap();
        let id = SsTableId::from(ulid::Ulid::new());
        let handle = writer
            .write_sst(&id, &builder.build().await.unwrap(), Some(Bytes::new()))
            .await
            .unwrap();

        // given: the same store wrapped so the first range read pauses, behind a
        // real FoyerCache that supports dedup
        let first_read_started = Arc::new(Notify::new());
        let release = Arc::new(Notify::new());
        let counting = Arc::new(PauseFirstReadStore {
            inner: inner.clone(),
            get_range_count: AtomicUsize::new(0),
            paused: AtomicBool::new(true),
            first_read_started: first_read_started.clone(),
            release: release.clone(),
        });
        let counting_store: Arc<dyn ObjectStore> = counting.clone();
        let cache: Arc<dyn DbCache> = Arc::new(FoyerCache::new());
        let reader = Arc::new(TableStore::new(
            counting_store,
            format,
            Path::from(ROOT),
            Some(cache),
            TableStoreKind::Main,
            BlockCachePolicy::default(),
        ));

        // when: task A starts reading the index; its loader will pause inside the
        // wrapped object store
        let handle_a = tokio::spawn({
            let reader = reader.clone();
            let handle = handle.clone();
            async move {
                reader
                    .read_index(
                        &handle,
                        true,
                        Some(Bytes::new()),
                        &ReadTrace::new(None),
                        None,
                    )
                    .await
            }
        });

        // wait until A's read has reached the object store and is paused
        first_read_started.notified().await;
        assert_eq!(
            counting.get_range_count.load(Ordering::SeqCst),
            1,
            "exactly one read should have hit the store so far"
        );

        // when: task B races A for the same index. join! polls B first, so B's
        // fetch_index reaches foyer's dedup map before release_task fires.
        let task_b = {
            let reader = reader.clone();
            let handle = handle.clone();
            async move {
                reader
                    .read_index(
                        &handle,
                        true,
                        Some(Bytes::new()),
                        &ReadTrace::new(None),
                        None,
                    )
                    .await
            }
        };
        let release_task = {
            let release = release.clone();
            async move {
                tokio::task::yield_now().await;
                release.notify_one();
            }
        };
        let (b_result, _) = tokio::join!(task_b, release_task);

        // then: both callers got an index, and exactly one object-store read happened
        let a_result = handle_a.await.expect("task A panicked");
        assert!(a_result.is_ok(), "task A failed: {:?}", a_result.err());
        assert!(b_result.is_ok(), "task B failed: {:?}", b_result.err());
        assert_eq!(
            counting.get_range_count.load(Ordering::SeqCst),
            1,
            "concurrent index reads must dedup into a single object-store read"
        );
    }

    /// End-to-end test: concurrent single-block reads through `TableStore` issue a
    /// single object-store request, even when the underlying object store is slow.
    /// Same determinism strategy as the index test above. Exercises the fast-path
    /// in `read_blocks_using_index` that routes 1-block reads (e.g. point-gets via
    /// `SstIterator::for_key`) through `cache.fetch_block`.
    #[cfg(feature = "foyer")]
    #[tokio::test]
    async fn dedups_concurrent_block_reads_through_object_store() {
        use crate::db_cache::foyer::FoyerCache;
        use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
        use tokio::sync::Notify;

        // given: an SST written through a plain in-memory store
        let inner: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let format = SsTableFormat::default();

        let writer = TableStore::new(
            inner.clone(),
            format.clone(),
            Path::from(ROOT),
            None,
            TableStoreKind::Main,
            BlockCachePolicy::default(),
        );
        let mut builder = writer.table_builder();
        builder
            .add(RowEntry::new_value(b"k1", b"v1", 0))
            .await
            .unwrap();
        builder
            .add(RowEntry::new_value(b"k2", b"v2", 0))
            .await
            .unwrap();
        let id = SsTableId::from(ulid::Ulid::new());
        let handle = writer
            .write_sst(&id, &builder.build().await.unwrap(), Some(Bytes::new()))
            .await
            .unwrap();

        // given: pre-load the index via the unwrapped writer so the wrapped store
        // sees only block reads (the fast-path takes `index` as an argument, so no
        // extra index read happens inside the race).
        let index = writer
            .read_index(
                &handle,
                false,
                Some(Bytes::new()),
                &ReadTrace::new(None),
                None,
            )
            .await
            .unwrap();

        // given: the same store wrapped so the first range read pauses, behind a
        // real FoyerCache that supports dedup
        let first_read_started = Arc::new(Notify::new());
        let release = Arc::new(Notify::new());
        let counting = Arc::new(PauseFirstReadStore {
            inner: inner.clone(),
            get_range_count: AtomicUsize::new(0),
            paused: AtomicBool::new(true),
            first_read_started: first_read_started.clone(),
            release: release.clone(),
        });
        let counting_store: Arc<dyn ObjectStore> = counting.clone();
        let cache: Arc<dyn DbCache> = Arc::new(FoyerCache::new());
        let reader = Arc::new(TableStore::new(
            counting_store,
            format,
            Path::from(ROOT),
            Some(cache),
            TableStoreKind::Main,
            BlockCachePolicy::default(),
        ));

        // when: task A starts a single-block read; its loader will pause inside the
        // wrapped object store
        let handle_a = tokio::spawn({
            let reader = reader.clone();
            let handle = handle.clone();
            let index = index.clone();
            async move {
                reader
                    .read_blocks_using_index(&handle, index, 0..1, true, Some(Bytes::new()))
                    .await
            }
        });

        // wait until A's read has reached the object store and is paused
        first_read_started.notified().await;
        assert_eq!(
            counting.get_range_count.load(Ordering::SeqCst),
            1,
            "exactly one read should have hit the store so far"
        );

        // when: task B races A for the same block. join! polls B first, so B's
        // fetch_block reaches foyer's dedup map before release_task fires.
        let task_b = {
            let reader = reader.clone();
            let handle = handle.clone();
            let index = index.clone();
            async move {
                reader
                    .read_blocks_using_index(&handle, index, 0..1, true, Some(Bytes::new()))
                    .await
            }
        };
        let release_task = {
            let release = release.clone();
            async move {
                tokio::task::yield_now().await;
                release.notify_one();
            }
        };
        let (b_result, _) = tokio::join!(task_b, release_task);

        // then: both callers got the block, and exactly one object-store read happened
        let a_result = handle_a.await.expect("task A panicked");
        assert!(a_result.is_ok(), "task A failed: {:?}", a_result.err());
        assert!(b_result.is_ok(), "task B failed: {:?}", b_result.err());
        assert_eq!(a_result.as_ref().unwrap().len(), 1);
        assert_eq!(b_result.as_ref().unwrap().len(), 1);
        assert_eq!(
            counting.get_range_count.load(Ordering::SeqCst),
            1,
            "concurrent single-block reads must dedup into a single object-store read"
        );
    }

    mod kind_tags {
        use super::super::{
            read_with_validation_retry, ObjectStoreCallTag, TableStoreKind, MAX_VALIDATION_RETRIES,
        };
        use super::{Path, ROOT};
        use crate::block_cache_policy::BlockCachePolicy;
        use crate::db_state::{SsTableId, SstType};
        use crate::error::{RetryReason, SlateDBError};
        use crate::format::sst::SsTableFormat;
        use crate::reader::ReadTrace;
        use crate::tablestore::TableStore;
        use crate::test_utils::{build_test_sst, RecordingObjectStore};
        use bytes::Bytes;
        use object_store::memory::InMemory;
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::{Arc, Mutex};

        fn format() -> SsTableFormat {
            SsTableFormat {
                block_size: 32,
                min_filter_keys: 1,
                ..SsTableFormat::default()
            }
        }

        fn recording_store(kind: TableStoreKind) -> (Arc<RecordingObjectStore>, Arc<TableStore>) {
            let recording = Arc::new(RecordingObjectStore::new(Arc::new(InMemory::new())));
            let ts = Arc::new(TableStore::new(
                recording.clone(),
                format(),
                Path::from(ROOT),
                None,
                kind,
                BlockCachePolicy::default(),
            ));
            (recording, ts)
        }

        // Compacted-SST range reads carry the source kind and Compacted type.
        #[tokio::test]
        async fn compacted_reads_carry_source_and_type() {
            let (recording, ts) = recording_store(TableStoreKind::Reader);
            let encoded = build_test_sst(&format(), 4).await;
            let id = SsTableId::from(ulid::Ulid::new());
            let segment = Bytes::from_static(b"segment/");
            let handle = ts
                .write_sst(&id, &encoded, Some(segment.clone()))
                .await
                .unwrap();

            recording.clear();
            ts.read_index(
                &handle,
                false,
                Some(segment.clone()),
                &ReadTrace::new(None),
                None,
            )
            .await
            .unwrap();

            let kinds = recording.get_kinds(false);
            assert!(!kinds.is_empty(), "expected at least one range read");
            assert!(
                kinds.iter().all(|k| *k == Some(TableStoreKind::Reader)),
                "compacted reads should carry the source kind, got {kinds:?}"
            );
            assert!(
                recording
                    .get_sst_types(false)
                    .iter()
                    .all(|t| *t == Some(SstType::Compacted)),
                "compacted reads should carry the Compacted type"
            );
            assert!(
                recording.get_retries(false).iter().all(|r| r.is_none()),
                "a successful read should carry no retry reason"
            );
            assert!(
                recording
                    .get_segments(false)
                    .iter()
                    .all(|actual| actual.as_ref() == Some(&segment)),
                "compacted reads should carry the segment hint"
            );
        }

        // A compacted-SST metadata HEAD read carries the source kind and type.
        #[tokio::test]
        async fn compacted_metadata_head_carries_source_and_type() {
            let (recording, ts) = recording_store(TableStoreKind::Compactor);
            let encoded = build_test_sst(&format(), 1).await;
            let id = SsTableId::from(ulid::Ulid::new());
            let segment = Bytes::from_static(b"segment/");
            ts.write_sst(&id, &encoded, Some(segment.clone()))
                .await
                .unwrap();

            recording.clear();
            ts.metadata(&id, Some(segment.clone())).await.unwrap();

            assert_eq!(
                recording.get_kinds(true),
                vec![Some(TableStoreKind::Compactor)],
                "the metadata HEAD read should carry the source kind"
            );
            assert_eq!(
                recording.get_sst_types(true),
                vec![Some(SstType::Compacted)],
                "the metadata HEAD read should carry the Compacted type"
            );
            assert_eq!(recording.get_segments(true), vec![Some(segment)]);
        }

        // A recoverable validation failure on a compacted read reissues it once
        // with the same source/type and a RetryReason describing the failure.
        #[tokio::test]
        async fn validation_failure_reissues_with_retry_reason() {
            let observed: Arc<Mutex<Vec<ObjectStoreCallTag>>> = Arc::new(Mutex::new(Vec::new()));
            let attempts = AtomicUsize::new(0);
            let obs = observed.clone();
            let segment = Bytes::from_static(b"segment/");
            let result: Result<u8, SlateDBError> = read_with_validation_retry(
                ObjectStoreCallTag::new_with_segment(
                    TableStoreKind::Compactor,
                    SstType::Compacted,
                    Some(segment.clone()),
                ),
                |tag| {
                    obs.lock().unwrap().push(tag);
                    let n = attempts.fetch_add(1, Ordering::SeqCst);
                    async move {
                        if n == 0 {
                            Err(SlateDBError::ChecksumMismatch { path: None })
                        } else {
                            Ok(42u8)
                        }
                    }
                },
            )
            .await;

            assert_eq!(result.unwrap(), 42);
            assert_eq!(MAX_VALIDATION_RETRIES, 1);
            let calls = observed.lock().unwrap().clone();
            assert_eq!(
                calls,
                vec![
                    ObjectStoreCallTag {
                        kind: TableStoreKind::Compactor,
                        sst_type: SstType::Compacted,
                        retry: None,
                        segment: Some(segment.clone()),
                    },
                    ObjectStoreCallTag {
                        kind: TableStoreKind::Compactor,
                        sst_type: SstType::Compacted,
                        retry: Some(RetryReason::CrcMismatch),
                        segment: Some(segment),
                    },
                ],
                "the reissue must carry the same source/type and the retry reason"
            );
        }

        #[tokio::test]
        async fn compacted_writes_carry_source_and_compacted_type() {
            let (recording, ts) = recording_store(TableStoreKind::Compactor);
            let encoded = build_test_sst(&format(), 4).await;
            let id = SsTableId::from(ulid::Ulid::new());
            let segment = Bytes::from_static(b"segment/");
            ts.write_sst(&id, &encoded, Some(segment.clone()))
                .await
                .unwrap();

            let kinds = recording.write_kinds();
            let sst_types = recording.write_sst_types();
            assert!(!kinds.is_empty(), "expected at least one write");
            assert!(
                kinds.iter().all(|k| *k == Some(TableStoreKind::Compactor)),
                "compacted writes should carry the source kind, got {kinds:?}"
            );
            assert!(
                sst_types.iter().all(|t| *t == Some(SstType::Compacted)),
                "compacted writes should carry the Compacted type, got {sst_types:?}"
            );
            assert!(
                recording
                    .write_segments()
                    .iter()
                    .all(|actual| actual.as_ref() == Some(&segment)),
                "compacted writes should carry the segment hint"
            );
        }
    }
}
