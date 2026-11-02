use crate::cached_object_store::policy::{
    CachePutConfig, DefaultGetPolicy, DefaultPutPolicy, GetAction, GetPolicy, HeadAction,
    PutAction, PutPolicy,
};
use crate::cached_object_store::stats::CachedObjectStoreStats;
use crate::cached_object_store::storage_fs::FsCacheStorage;
use crate::cached_object_store::LocalCacheEntry;
use crate::config::ObjectStoreCacheOptions;
use crate::object_store_tag::ObjectStoreCallTag;
use bytes::{Bytes, BytesMut};
use futures::{future::BoxFuture, stream, stream::BoxStream, StreamExt};
use object_store::{path::Path, GetOptions, GetResult, ObjectMeta, ObjectStore, ObjectStoreExt};
use object_store::{
    Attributes, CopyOptions, Extensions, GetRange, GetResultPayload, PutMultipartOptions,
    PutResult, RenameOptions,
};
use object_store::{ListResult, MultipartUpload, PutOptions, PutPayload};
use slatedb_common::clock::{DefaultSystemClock, SystemClock};
use slatedb_common::DbRand;
use std::{ops::Range, sync::Arc};

use crate::single_flight::SingleFlight;

use crate::cached_object_store::storage::{LocalCacheStorage, PartID};
use crate::error::SlateDBError;
use crate::utils::build_concurrent;
use log::warn;

use slatedb_common::metrics::{
    MetricLevel, MetricsRecorder, MetricsRecorderHelper, NoopMetricsRecorder,
};

/// An [`ObjectStore`] wrapper that caches object parts on local disk.
///
/// The cache splits each object into fixed-size parts and stores them under a
/// root folder.
///
/// Reads tagged by SlateDB as compacted SST reads are served from
/// disk when present and admitted on a miss.
///
/// Writes can optionally be admitted via
/// [`CachedObjectStoreBuilder::with_cache_on_flush`] and
/// [`CachedObjectStoreBuilder::with_cache_on_compaction`].
///
/// All other calls (manifests, WAL, listings) pass through to the wrapped store.
///
/// Construct it over the raw backend and pass it to SlateDB as the object
/// store itself:
///
/// ```ignore
/// let cache = CachedObjectStore::builder("/var/slatedb-cache", backend)
///     .build()
///     .await?;
/// let db = Db::builder(path, cache).build().await?;
/// ```
#[derive(Debug, Clone)]
pub struct CachedObjectStore {
    object_store: Arc<dyn ObjectStore>,
    part_size_bytes: usize, // expected to be aligned with mb or kb
    pub(crate) cache_storage: Arc<dyn LocalCacheStorage>,
    get_policy: Arc<dyn GetPolicy>,
    put_policy: Arc<dyn PutPolicy>,
    stats: Arc<CachedObjectStoreStats>,
    // Deduplicates concurrent HEAD requests for the same path after a cache miss.
    head_flights: SingleFlight<Path, (ObjectMeta, Attributes, Extensions)>,
    // Deduplicates concurrent prefetch/GET requests for the same path after a cache miss.
    prefetch_flights:
        SingleFlight<(Path, Option<GetRangeKey>), (ObjectMeta, Attributes, Extensions)>,
    // Deduplicates concurrent fetches of the same part after a cache miss.
    // Keyed on (path, part_id) so multiple readers needing the same part share one fetch.
    part_flights: SingleFlight<(Path, PartID), Bytes>,
}

impl CachedObjectStore {
    pub(crate) fn new(
        object_store: Arc<dyn ObjectStore>,
        cache_storage: Arc<dyn LocalCacheStorage>,
        part_size_bytes: usize,
        cache_put_config: CachePutConfig,
        stats: Arc<CachedObjectStoreStats>,
    ) -> Result<Arc<Self>, SlateDBError> {
        Self::new_with_policies(
            object_store,
            cache_storage,
            part_size_bytes,
            stats,
            Arc::new(DefaultGetPolicy),
            Arc::new(DefaultPutPolicy {
                put: cache_put_config,
            }),
        )
    }

    /// Like [`Self::new`], but with caller supplied read and put policies.
    /// `new` installs [`DefaultGetPolicy`] and [`DefaultPutPolicy`].
    #[allow(unused)]
    pub(crate) fn new_with_policies(
        object_store: Arc<dyn ObjectStore>,
        cache_storage: Arc<dyn LocalCacheStorage>,
        part_size_bytes: usize,
        stats: Arc<CachedObjectStoreStats>,
        get_policy: Arc<dyn GetPolicy>,
        put_policy: Arc<dyn PutPolicy>,
    ) -> Result<Arc<Self>, SlateDBError> {
        if part_size_bytes == 0 || !part_size_bytes.is_multiple_of(1024) {
            return Err(SlateDBError::InvalidCachePartSize);
        }

        Ok(Arc::new(Self {
            object_store,
            part_size_bytes,
            cache_storage,
            get_policy,
            put_policy,
            stats,
            head_flights: SingleFlight::new(),
            prefetch_flights: SingleFlight::new(),
            part_flights: SingleFlight::new(),
        }))
    }

    pub(crate) async fn start_evictor(&self) {
        self.cache_storage.start_evictor().await;
    }

    /// Build a `CachedObjectStore` from `ObjectStoreCacheOptions`, returning `None`
    /// if caching is not configured (i.e. `root_folder` is `None`). When `Some` is
    /// returned the evictor has already been started.
    pub(crate) async fn from_config(
        object_store: Arc<dyn ObjectStore>,
        options: &ObjectStoreCacheOptions,
        recorder: &MetricsRecorderHelper,
        clock: Arc<dyn SystemClock>,
        rand: Arc<DbRand>,
    ) -> Result<Option<Arc<Self>>, SlateDBError> {
        let cache_root_folder = match &options.root_folder {
            None => return Ok(None),
            Some(f) => f,
        };
        let stats = Arc::new(CachedObjectStoreStats::new(recorder));
        let cache_storage = Arc::new(FsCacheStorage::new(
            cache_root_folder.clone(),
            options.max_cache_size_bytes,
            options.scan_interval,
            stats.clone(),
            clock,
            rand,
            options.max_open_file_handles,
        ));
        let cached = Self::new(
            object_store,
            cache_storage,
            options.part_size_bytes,
            CachePutConfig {
                cache_on_flush: options.cache_on_flush,
                cache_on_compaction: options.cache_on_compaction,
            },
            stats,
        )?;
        cached.start_evictor().await;
        Ok(Some(cached))
    }

    /// Returns a builder for a `CachedObjectStore` that caches parts of the
    /// objects in `object_store` under `root_folder` on the local filesystem.
    pub fn builder(
        root_folder: impl Into<std::path::PathBuf>,
        object_store: Arc<dyn ObjectStore>,
    ) -> CachedObjectStoreBuilder {
        CachedObjectStoreBuilder {
            object_store,
            options: ObjectStoreCacheOptions {
                root_folder: Some(root_folder.into()),
                ..ObjectStoreCacheOptions::default()
            },
            metrics_recorder: Arc::new(NoopMetricsRecorder::new()),
            metric_level: MetricLevel::default(),
        }
    }

    /// Loads files into the cache up to a maximum number of bytes.
    ///
    /// Fetches each object's raw bytes from the wrapped store and saves them
    /// as cache parts on disk. Can be used to warm up the cache.
    ///
    /// The `max_bytes` budget is applied in path order: loading stops at the
    /// first file that does not fit, so order paths by priority.
    ///
    /// Fetches are best-effort and failures are logged and skipped.
    pub async fn load_files_to_cache(
        &self,
        file_paths: Vec<Path>,
        max_bytes: usize,
    ) -> Result<(), crate::Error> {
        if file_paths.is_empty() || max_bytes == 0 {
            return Ok(());
        }

        let mut remaining_bytes = max_bytes;
        let mut files_to_load = Vec::with_capacity(file_paths.len());

        // First pass: sequentially get metadata and select files that fit
        // This is done sequentially because the head calls should be very quick compared to files loading
        for path in file_paths {
            match self.object_store.head(&path).await {
                Ok(meta) => {
                    let file_size = meta.size as usize;
                    if remaining_bytes >= file_size {
                        remaining_bytes -= file_size;
                        files_to_load.push(path);
                    } else {
                        // We can't fit this file, so we stop here
                        break;
                    }
                }
                Err(e) => {
                    // If file doesn't exist or can't be accessed, we stop here
                    warn!("Failed to preload all SSTs to cache: {:?}", e);
                    break;
                }
            }
        }

        // Second pass: load the selected files in bounded parallelism and cache them.
        let degree_of_parallelism = 32;
        let _result = build_concurrent(files_to_load.into_iter(), degree_of_parallelism, |path| {
            let this = self.clone();
            async move {
                match this
                    .maybe_prefetch_range(&path, GetOptions::default())
                    .await
                {
                    Ok(_) => Ok(Some(())),
                    Err(e) => {
                        warn!(
                            "Failed to prefetch file into cache [path={}, error={:?}]",
                            path, e
                        );
                        Ok(None) // best-effort: skip errors
                    }
                }
            }
        })
        .await;

        Ok(())
    }

    pub(crate) async fn cached_head(
        &self,
        location: &Path,
        admit_on_miss: bool,
    ) -> object_store::Result<GetResult> {
        let entry = self.cache_storage.entry(location, self.part_size_bytes);
        if let Ok(Some((meta, attributes))) = entry.read_head().await {
            return Ok(head_only_get_result(meta, attributes, Extensions::new()));
        }

        // Cache miss — deduplicate concurrent HEAD requests for the same path.
        let (meta, attributes, extensions) = self
            .head_flights
            .call(location.clone(), || async {
                let result = self
                    .object_store
                    .get_opts(
                        location,
                        GetOptions {
                            range: None,
                            head: true,
                            ..Default::default()
                        },
                    )
                    .await?;
                let meta = result.meta.clone();
                let attributes = result.attributes.clone();
                let extensions = result.extensions.clone();

                if admit_on_miss {
                    self.save_get_result(location, result).await.ok();
                }
                Ok::<_, object_store::Error>((meta, attributes, extensions))
            })
            .await?;
        Ok(head_only_get_result(meta, attributes, extensions))
    }

    pub(crate) async fn cached_get_opts(
        &self,
        location: &Path,
        opts: GetOptions,
        force_refresh: bool,
    ) -> object_store::Result<GetResult> {
        let PrefetchedHead {
            meta,
            attributes,
            extensions,
            head_source,
        } = self.maybe_prefetch_range(location, opts.clone()).await?;

        let get_range = opts.range.clone();
        let range = self.canonicalize_range(get_range, meta.size)?;
        let parts = self.split_range_into_parts(range.clone());

        // Read parts and concatenate them into a single stream. Some parts may not
        // be cached; read_part falls back to the object store for the missing ones.
        let futures = parts
            .into_iter()
            .map(|(part_id, range_in_part)| {
                let this = self.clone();
                let location = location.clone();
                async move {
                    this.stats.object_store_cache_part_access.increment(1);
                    let (bytes, part_source) = this
                        .read_part(&location, part_id, range_in_part, force_refresh)
                        .await?;
                    if head_source == ReadResultSource::Disk
                        && part_source == ReadResultSource::Disk
                    {
                        this.stats.object_store_cache_part_hits.increment(1);
                    }
                    Ok::<Bytes, object_store::Error>(bytes)
                }
            })
            .collect::<Vec<_>>();
        let result_stream = stream::iter(futures).then(|fut| fut).boxed();

        Ok(GetResult {
            meta,
            range,
            attributes,
            payload: GetResultPayload::Stream(result_stream),
            extensions,
        })
    }

    async fn cached_put_opts(
        &self,
        location: &Path,
        payload: PutPayload,
        opts: PutOptions,
    ) -> object_store::Result<PutResult> {
        // The per-call tag decides whether this write is cached.
        let tag = ObjectStoreCallTag::from_extensions(&opts.extensions);
        if self.put_policy.put_action(tag.as_ref()) == PutAction::Skip {
            // Write directly to upstream without caching the payload.
            return self.object_store.put_opts(location, payload, opts).await;
        }

        // Capture the size and attributes before payload/opts are consumed: they
        // go into the head we write below.
        let payload_size = payload.content_length() as u64;
        let attributes = opts.attributes.clone();

        // First, write to the upstream object store.
        let result = self
            .object_store
            .put_opts(location, payload.clone(), opts)
            .await?;

        // Convert PutPayload to stream and save parts to cache.
        let entry = self.cache_storage.entry(location, self.part_size_bytes);
        let stream = stream::iter(payload.into_iter()).map(Ok::<Bytes, object_store::Error>);
        // Save parts, ignoring errors (cache failures must not fail the PUT).
        self.save_parts_stream(entry.as_ref(), stream, 0).await.ok();

        // Make the write visible to reads by writing the head. This is not
        // the actual HEAD response from the upstream store, but a synthesized
        // head with the known size and attributes.
        let meta = build_head(location, payload_size, &result);
        entry.save_head((&meta, &attributes)).await.ok();

        Ok(result)
    }

    // if an object is not cached before, maybe_prefetch_range will try to prefetch the object from the
    // object store and save the parts into the local disk cache. the prefetching is helpful to reduce the
    // number of GET requests to the object store, it'll try to aggregate the parts among the range into a
    // single GET request, and save the related parts into local disks together.
    // when it sends GET requests to the object store, the range is expected to be ALIGNED with the part
    // size.
    async fn maybe_prefetch_range(
        &self,
        location: &Path,
        mut opts: GetOptions,
    ) -> object_store::Result<PrefetchedHead> {
        let entry = self.cache_storage.entry(location, self.part_size_bytes);
        match entry.read_head().await {
            Ok(Some((meta, attrs))) => {
                return Ok(PrefetchedHead {
                    meta,
                    attributes: attrs,
                    extensions: Extensions::new(),
                    head_source: ReadResultSource::Disk,
                })
            }
            Ok(None) => {}
            Err(e) => {
                warn!(
                    "failed to read head from disk cache, will fallback to object store [location={}, error={:?}]",
                    location, e,
                );
            }
        }

        if let Some(range) = &opts.range {
            opts.range = Some(self.align_get_range(range));
        }

        // Cache miss — deduplicate concurrent prefetch requests for the same path.
        // Only one caller performs the fetch+save; others share the metadata result.
        // Parts not covered by the winning caller's range are handled by read_part's
        // own object-store fallback, so correctness is maintained.
        self.prefetch_flights
            .call(
                (location.clone(), opts.range.clone().map(Into::into)),
                || async {
                    let get_result = self.object_store.get_opts(location, opts).await?;
                    let result_meta = get_result.meta.clone();
                    let result_attrs = get_result.attributes.clone();
                    let result_extensions = get_result.extensions.clone();
                    // swallow the error on saving to disk here (the disk might be already full), just fallback
                    // to the object store.
                    // TODO: add a warning log here
                    self.save_get_result(location, get_result).await.ok();
                    Ok((result_meta, result_attrs, result_extensions))
                },
            )
            .await
            .map(|(meta, attributes, extensions)| PrefetchedHead {
                meta,
                attributes,
                extensions,
                head_source: ReadResultSource::Upstream,
            })
    }

    /// save the GetResult to the disk cache, a GetResult may be transformed into multiple part
    /// files and a meta file. please note that the `range` in the GetResult is expected to be
    /// aligned with the part size.
    async fn save_get_result(
        &self,
        cache_location: &Path,
        result: GetResult,
    ) -> object_store::Result<u64> {
        let part_size_bytes_u64 = self.part_size_bytes as u64;
        assert!(result.range.start.is_multiple_of(part_size_bytes_u64));
        assert!(
            result.range.end.is_multiple_of(part_size_bytes_u64)
                || result.range.end == result.meta.size
        );

        let entry = self
            .cache_storage
            .entry(cache_location, self.part_size_bytes);
        let object_size = result.meta.size;

        // Reaching here means the read policy already chose to fill the cache
        // so always save.
        entry.save_head((&result.meta, &result.attributes)).await?;

        let start_part_number = usize::try_from(result.range.start / part_size_bytes_u64)
            .expect("Part number exceeds u32 on a 32-bit system. Try increasing part size.");

        let stream = result.into_stream();

        self.save_parts_stream(entry.as_ref(), stream, start_part_number)
            .await?;

        Ok(object_size)
    }

    /// Save a stream of bytes to cache as parts, starting from the specified part number.
    /// Returns the number of bytes saved.
    /// This method only saves the data parts - the head should be saved separately.
    async fn save_parts_stream<S>(
        &self,
        entry: &dyn LocalCacheEntry,
        mut stream: S,
        start_part_number: usize,
    ) -> object_store::Result<usize>
    where
        S: stream::Stream<Item = Result<Bytes, object_store::Error>> + Unpin,
    {
        let mut buffer = BytesMut::new();
        let mut part_number = start_part_number;
        let mut total_bytes: usize = 0;

        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            total_bytes += chunk.len();
            buffer.extend_from_slice(&chunk);

            while buffer.len() >= self.part_size_bytes {
                let to_write = buffer.split_to(self.part_size_bytes);
                entry.save_part(part_number, to_write.into()).await?;
                part_number += 1;
            }
        }

        // Save any remaining bytes as the last part
        if !buffer.is_empty() {
            entry.save_part(part_number, buffer.into()).await?;
        }

        Ok(total_bytes)
    }

    // split the range into parts, and return the part id and the range inside the part.
    fn split_range_into_parts(&self, range: Range<u64>) -> Vec<(PartID, Range<usize>)> {
        let part_size_bytes_u64 = self.part_size_bytes as u64;
        let range_aligned = self.align_range(&range, self.part_size_bytes);
        let start_part = range_aligned.start / part_size_bytes_u64;
        let end_part = range_aligned.end / part_size_bytes_u64;
        let mut parts: Vec<_> = (start_part..end_part)
            .map(|part_id| {
                (
                    usize::try_from(part_id).expect("Number of parts exceeds usize"),
                    Range {
                        start: 0,
                        end: self.part_size_bytes,
                    },
                )
            })
            .collect();
        if parts.is_empty() {
            return vec![];
        }
        if let Some(first_part) = parts.first_mut() {
            first_part.1.start = usize::try_from(range.start % part_size_bytes_u64)
                .expect("Part size is too large to fit in a usize");
        }
        if let Some(last_part) = parts.last_mut() {
            if !range.end.is_multiple_of(part_size_bytes_u64) {
                last_part.1.end = usize::try_from(range.end % part_size_bytes_u64)
                    .expect("Part size is too large to fit in a usize");
            }
        }
        parts
    }

    /// Get a part from disk if cached, otherwise start a new GET request.
    ///
    /// IO errors reading the disk cache are ignored and fall back to the object
    /// store.
    ///
    /// Returns the bytes plus where they were served from, so the caller can
    /// classify the read as a hit or a miss.
    fn read_part(
        &self,
        location: &Path,
        part_id: PartID,
        range_in_part: Range<usize>,
        force_refresh: bool,
    ) -> BoxFuture<'static, object_store::Result<(Bytes, ReadResultSource)>> {
        let this = self.clone();
        let location = location.clone();
        Box::pin(async move {
            let entry = this.cache_storage.entry(&location, this.part_size_bytes);
            if !force_refresh {
                if let Ok(Some(bytes)) = entry.read_part(part_id, range_in_part.clone()).await {
                    return Ok((bytes, ReadResultSource::Disk));
                }
            }

            // Cache miss, so we need to fetch from the object store.
            // Read Part — deduplicate concurrent fetches of the same part.
            // The SingleFlight fetches the full part and saves it to cache; each
            // caller then copies out their own range_in_part.
            let bytes = this
                .part_flights
                .call((location.clone(), part_id), || async {
                    let part_range = Range {
                        start: (part_id * this.part_size_bytes) as u64,
                        end: ((part_id + 1) * this.part_size_bytes) as u64,
                    };
                    let get_result = this
                        .object_store
                        .get_opts(
                            &location,
                            GetOptions {
                                range: Some(GetRange::Bounded(part_range.clone())),
                                ..Default::default()
                            },
                        )
                        .await?;

                    let meta = get_result.meta.clone();
                    let attrs = get_result.attributes.clone();
                    let bytes = get_result.bytes().await?;

                    // A truncated but successful ranged body is possible and
                    // should be caught here because the rest of the code
                    // assumes the size is correct.
                    //
                    // We return a retryable error before anything is saved or
                    // sliced and the retry layer above will retry.
                    //
                    // We also have to take the min of the part range end and
                    // the object size because the part range may extend beyond
                    // the object size.
                    let expected_len = usize::try_from(
                        meta.size
                            .min(part_range.end)
                            .saturating_sub(part_range.start),
                    )
                    .expect("part length exceeds usize");
                    if bytes.len() != expected_len {
                        return Err(object_store::Error::Generic {
                            store: "cached_object_store",
                            source: format!(
                                "part fetch size check failed: {} bytes read, but expected \
                                 {expected_len} bytes (part range {}..{} truncated at object \
                                 size {})",
                                bytes.len(),
                                part_range.start,
                                part_range.end,
                                meta.size
                            )
                            .into(),
                        });
                    }

                    // Save the head and the part to cache for future accesses.
                    let entry = this.cache_storage.entry(&location, this.part_size_bytes);
                    entry.save_head((&meta, &attrs)).await.ok();
                    entry.save_part(part_id, bytes.clone()).await.ok();

                    Ok::<_, object_store::Error>(bytes)
                })
                .await?;

            Ok((
                Bytes::copy_from_slice(&bytes[range_in_part]),
                ReadResultSource::Upstream,
            ))
        })
    }

    // given the range and object size, return the canonicalized `Range<usize>` with concrete start and
    // end.
    fn canonicalize_range(
        &self,
        range: Option<GetRange>,
        object_size: u64,
    ) -> object_store::Result<Range<u64>> {
        let (start_offset, end_offset) = match range {
            None => (0, object_size),
            Some(range) => match range {
                GetRange::Bounded(range) => {
                    if range.start >= object_size {
                        return Err(object_store::Error::Generic {
                            store: "cached_object_store",
                            source: Box::new(InvalidGetRange::StartTooLarge {
                                requested: range.start,
                                length: object_size,
                            }),
                        });
                    }
                    if range.start >= range.end {
                        return Err(object_store::Error::Generic {
                            store: "cached_object_store",
                            source: Box::new(InvalidGetRange::Inconsistent {
                                start: range.start,
                                end: range.end,
                            }),
                        });
                    }
                    (range.start, range.end.min(object_size))
                }
                GetRange::Offset(offset) => {
                    if offset >= object_size {
                        return Err(object_store::Error::Generic {
                            store: "cached_object_store",
                            source: Box::new(InvalidGetRange::StartTooLarge {
                                requested: offset,
                                length: object_size,
                            }),
                        });
                    }
                    (offset, object_size)
                }
                GetRange::Suffix(suffix) => (object_size.saturating_sub(suffix), object_size),
            },
        };
        Ok(Range {
            start: start_offset,
            end: end_offset,
        })
    }

    fn align_get_range(&self, range: &GetRange) -> GetRange {
        match range {
            GetRange::Bounded(bounded) => {
                let aligned = self.align_range(bounded, self.part_size_bytes);
                GetRange::Bounded(aligned)
            }
            GetRange::Suffix(suffix) => {
                let suffix_aligned = self.align_range(&(0..*suffix), self.part_size_bytes).end;
                GetRange::Suffix(suffix_aligned)
            }
            GetRange::Offset(offset) => {
                let offset_aligned = *offset - *offset % self.part_size_bytes as u64;
                GetRange::Offset(offset_aligned)
            }
        }
    }

    fn align_range(&self, range: &Range<u64>, alignment: usize) -> Range<u64> {
        let alignment = alignment as u64;
        let start_aligned = range.start - range.start % alignment;
        let end_aligned = range.end.div_ceil(alignment) * alignment;
        Range {
            start: start_aligned,
            end: end_aligned,
        }
    }
}

/// Builder for [`CachedObjectStore`]. Created by [`CachedObjectStore::builder`].
pub struct CachedObjectStoreBuilder {
    object_store: Arc<dyn ObjectStore>,
    options: ObjectStoreCacheOptions,
    metrics_recorder: Arc<dyn MetricsRecorder>,
    metric_level: MetricLevel,
}

impl CachedObjectStoreBuilder {
    /// Sets the limit of the cache size in bytes.
    ///
    /// `None` disables eviction and the default is 16gb.
    pub fn with_max_cache_size_bytes(mut self, max_cache_size_bytes: Option<usize>) -> Self {
        self.options.max_cache_size_bytes = max_cache_size_bytes;
        self
    }

    /// Sets the size of each part file. Must be a multiple of 1kb.
    ///
    /// The default is 4mb.
    pub fn with_part_size_bytes(mut self, part_size_bytes: usize) -> Self {
        self.options.part_size_bytes = part_size_bytes;
        self
    }

    /// Sets whether compacted SSTs produced by memtable flushes are admitted
    /// to the cache on write.
    ///
    /// The default is false.
    pub fn with_cache_on_flush(mut self, cache_on_flush: bool) -> Self {
        self.options.cache_on_flush = cache_on_flush;
        self
    }

    /// Sets whether compacted SSTs produced by compaction are admitted to the
    /// cache on write.
    ///
    /// The default is false.
    pub fn with_cache_on_compaction(mut self, cache_on_compaction: bool) -> Self {
        self.options.cache_on_compaction = cache_on_compaction;
        self
    }

    /// Sets the interval at which the cache directory is scanned to rebuild
    /// the evictor's in-memory map.
    ///
    ///  `None` scans only once on startup and the default is 1 hour.
    pub fn with_scan_interval(mut self, scan_interval: Option<std::time::Duration>) -> Self {
        self.options.scan_interval = scan_interval;
        self
    }

    /// Sets the maximum number of open file handles kept by the part file
    /// handle cache.
    ///
    /// The default is 1000.
    pub fn with_max_open_file_handles(mut self, max_open_file_handles: usize) -> Self {
        self.options.max_open_file_handles = max_open_file_handles;
        self
    }

    /// Sets the recorder for the cache's metrics (hit and access counters,
    /// cache size gauges, eviction counters).
    ///
    ///  Defaults to a no-op recorder.
    pub fn with_metrics_recorder(mut self, metrics_recorder: Arc<dyn MetricsRecorder>) -> Self {
        self.metrics_recorder = metrics_recorder;
        self
    }

    /// Sets the metric level for the cache's metrics.
    ///
    /// Defaults to [`MetricLevel::default`].
    pub fn with_metric_level(mut self, metric_level: MetricLevel) -> Self {
        self.metric_level = metric_level;
        self
    }

    /// Builds the `CachedObjectStore` and starts its evictor.
    pub async fn build(self) -> Result<Arc<CachedObjectStore>, crate::Error> {
        let recorder = MetricsRecorderHelper::new(self.metrics_recorder, self.metric_level);
        let cached = CachedObjectStore::from_config(
            self.object_store,
            &self.options,
            &recorder,
            Arc::new(DefaultSystemClock::new()),
            Arc::new(DbRand::default()),
        )
        .await
        .map_err(crate::Error::from)?;
        Ok(cached.expect("builder always sets root_folder"))
    }
}

fn head_only_get_result(
    meta: ObjectMeta,
    attributes: Attributes,
    extensions: Extensions,
) -> GetResult {
    GetResult {
        payload: GetResultPayload::Stream(stream::empty().boxed()),
        range: 0..0,
        meta,
        attributes,
        extensions,
    }
}

/// Builds a synthetic head to save on a write, from the upstream `PutResult`
/// and the known object size.
///
/// The head is the cache entry's commit point: cached parts are not usable
/// until a `read_head` succeeds, so writing it last (after the upstream write
/// completes) publishes the entry.
fn build_head(cache_location: &Path, size: u64, result: &PutResult) -> ObjectMeta {
    ObjectMeta {
        location: cache_location.clone(),
        // `last_modified` is not used by the cache, add a stub instead of
        // executing an actual HEAD request after write. If this ever change,
        // the cache should be updated to use the upstream `last_modified`
        // instead of the stub value here.
        last_modified: chrono::DateTime::<chrono::Utc>::from_timestamp(0, 0)
            .expect("unix epoch is a valid timestamp"),
        size,
        e_tag: result.e_tag.clone(),
        version: result.version.clone(),
    }
}

impl std::fmt::Display for CachedObjectStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "CachedObjectStore({}, {})",
            self.object_store, self.cache_storage
        )
    }
}

#[async_trait::async_trait]
impl ObjectStore for CachedObjectStore {
    async fn get_opts(
        &self,
        location: &Path,
        options: GetOptions,
    ) -> object_store::Result<GetResult> {
        let tag = ObjectStoreCallTag::from_extensions(&options.extensions);

        if options.head {
            return match self.get_policy.head_action(tag.as_ref()) {
                HeadAction::Bypass => self.object_store.get_opts(location, options).await,
                HeadAction::Probe => self.cached_head(location, false).await,
                HeadAction::ReadThrough => self.cached_head(location, true).await,
            };
        }
        match self.get_policy.get_action(tag.as_ref()) {
            GetAction::Bypass => self.object_store.get_opts(location, options).await,
            GetAction::Refetch => self.cached_get_opts(location, options, true).await,
            GetAction::ReadThrough => self.cached_get_opts(location, options, false).await,
        }
    }

    async fn put_opts(
        &self,
        location: &Path,
        payload: PutPayload,
        opts: PutOptions,
    ) -> object_store::Result<PutResult> {
        self.cached_put_opts(location, payload, opts).await
    }

    async fn put_multipart_opts(
        &self,
        location: &Path,
        opts: PutMultipartOptions,
    ) -> object_store::Result<Box<dyn MultipartUpload>> {
        let tag = ObjectStoreCallTag::from_extensions(&opts.extensions);
        let attributes = opts.attributes.clone();

        let inner = self.object_store.put_multipart_opts(location, opts).await?;

        // Wrap the upload to mirror its parts into the cache, unless skipped.
        if self.put_policy.put_action(tag.as_ref()) == PutAction::Skip {
            return Ok(inner);
        }
        Ok(Box::new(CachingMultipartUpload::new(
            inner,
            Arc::clone(&self.cache_storage),
            location.clone(),
            self.part_size_bytes,
            attributes,
        )))
    }

    /// Deletion of the cache entries associated with the object being
    /// deleted is not atomic with respect to the object deletion from
    /// the underlying object store. So for some period of time after
    /// the deletion, cached object parts are still visible in the cache.
    /// But assuming each object ever created by SlateDB is immutable and
    /// has a unique name, this is not a problem.
    ///
    /// If eviction is enabled, deletion of the associated cache entries
    /// happens asynchronously; when the control returns to the caller,
    /// the entries still might be present in the cache. If eviction is
    /// off, the deletion happens synchronously; when the control returns
    /// to the caller, it is guaranteed no entries present in the cache
    /// (assuming no errors happened during the deletion).
    fn delete_stream(
        &self,
        locations: BoxStream<'static, object_store::Result<Path>>,
    ) -> BoxStream<'static, object_store::Result<Path>> {
        let cache_storage = self.cache_storage.clone();
        let part_size_bytes = self.part_size_bytes;

        self.object_store
            .delete_stream(locations)
            .then(move |result| {
                let cache_storage = cache_storage.clone();
                async move {
                    if let Ok(ref location) = result {
                        let entry = cache_storage.entry(location, part_size_bytes);
                        entry.delete().await;
                    }
                    result
                }
            })
            .boxed()
    }

    fn list(&self, prefix: Option<&Path>) -> BoxStream<'static, object_store::Result<ObjectMeta>> {
        self.object_store.list(prefix)
    }

    fn list_with_offset(
        &self,
        prefix: Option<&Path>,
        offset: &Path,
    ) -> BoxStream<'static, object_store::Result<ObjectMeta>> {
        self.object_store.list_with_offset(prefix, offset)
    }

    async fn list_with_delimiter(&self, prefix: Option<&Path>) -> object_store::Result<ListResult> {
        self.object_store.list_with_delimiter(prefix).await
    }

    async fn copy_opts(
        &self,
        from: &Path,
        to: &Path,
        options: CopyOptions,
    ) -> object_store::Result<()> {
        self.object_store.copy_opts(from, to, options).await
    }

    async fn rename_opts(
        &self,
        from: &Path,
        to: &Path,
        options: RenameOptions,
    ) -> object_store::Result<()> {
        self.object_store.rename_opts(from, to, options).await
    }
}

/// A [`MultipartUpload`] that mirrors the uploaded bytes into the local cache as
/// it streams them upstream. Created by [`CachedObjectStore::put_multipart_opts`]
/// when the call policy caches a compacted SST (the path large compacted SSTs
/// take, above BufWriter's multipart threshold).
///
/// The head is the commit point: it is written on `complete` after the
/// upstream upload succeeds, which publishes the parts as a live cache entry.
/// Until then the parts are not usable (a read with no head refetches from
/// upstream).
///
/// Cache writes are best effort: a failed cache write never fails the upload.
///
/// TODO: fix potential part leak: a multipart upload that fails midway
/// (dropped without complete() or abort() leaks its already written cache
/// parts forever when the evictor is disabled. Can happen on a crash or
/// exhuasting retries in one of the uploads.
struct CachingMultipartUpload {
    inner: Box<dyn MultipartUpload>,
    cache_storage: Arc<dyn LocalCacheStorage>,
    cache_location: Path,
    part_size: usize,
    /// In-order bytes observed so far that have not yet filled a cache part.
    buffer: BytesMut,
    /// The next cache part number to write.
    next_part: PartID,
    /// Total bytes teed so far; becomes the committed head's `size`.
    total_len: u64,
    /// Attributes from the upload options, echoed into the committed head.
    attributes: Attributes,
}

impl CachingMultipartUpload {
    fn new(
        inner: Box<dyn MultipartUpload>,
        cache_storage: Arc<dyn LocalCacheStorage>,
        cache_location: Path,
        part_size: usize,
        attributes: Attributes,
    ) -> Self {
        Self {
            inner,
            cache_storage,
            cache_location,
            part_size,
            buffer: BytesMut::new(),
            next_part: 0,
            total_len: 0,
            attributes,
        }
    }
}

impl std::fmt::Debug for CachingMultipartUpload {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CachingMultipartUpload")
            .field("cache_location", &self.cache_location)
            .finish_non_exhaustive()
    }
}

#[async_trait::async_trait]
impl MultipartUpload for CachingMultipartUpload {
    fn put_part(&mut self, data: PutPayload) -> object_store::UploadPart {
        // Write the payload bytes into the cache buffer, then forward the
        // original payload upstream.
        self.total_len += data.content_length() as u64;
        self.buffer.reserve(data.content_length());
        for bytes in &data {
            self.buffer.extend_from_slice(bytes);
        }
        let mut parts = Vec::new();
        while self.buffer.len() >= self.part_size {
            let chunk = self.buffer.split_to(self.part_size).freeze();
            parts.push((self.next_part, chunk));
            self.next_part += 1;
        }

        let inner_fut = self.inner.put_part(data);
        if parts.is_empty() {
            return inner_fut;
        }
        let cache_storage = Arc::clone(&self.cache_storage);
        let cache_location = self.cache_location.clone();
        let part_size = self.part_size;
        Box::pin(async move {
            // Overlap the cache disk writes with the upstream upload.
            let cache_fut = async {
                let entry = cache_storage.entry(&cache_location, part_size);
                for (part_number, chunk) in parts {
                    // Currently, we ignore errors writing to the cache. This
                    // is best-effort: a failed cache write never fails the
                    // upload.
                    entry.save_part(part_number, chunk).await.ok();
                }
            };
            let (result, ()) = futures::future::join(inner_fut, cache_fut).await;
            result
        })
    }

    async fn complete(&mut self) -> object_store::Result<PutResult> {
        let result = self.inner.complete().await?;
        let entry = self
            .cache_storage
            .entry(&self.cache_location, self.part_size);
        // Flush the trailing partial part once the upload is durable upstream.
        if !self.buffer.is_empty() {
            let chunk = std::mem::take(&mut self.buffer).freeze();
            entry.save_part(self.next_part, chunk).await.ok();
            self.next_part += 1;
        }

        // Commit by writing the head last (after the upstream upload succeeded):
        // it publishes the parts as a live cache entry and lets the first read
        // serve from the cache instead of doing an upstream HEAD and re-prefetch.
        let meta = build_head(&self.cache_location, self.total_len, &result);
        entry.save_head((&meta, &self.attributes)).await.ok();
        Ok(result)
    }

    async fn abort(&mut self) -> object_store::Result<()> {
        let result = self.inner.abort().await;
        // The object will never exist upstream, so drop any cached parts.
        self.cache_storage
            .entry(&self.cache_location, self.part_size)
            .delete()
            .await;
        result
    }
}

/// Where a read (of the object head or of a single part) was served from.
#[derive(Clone, Copy, PartialEq, Eq)]
enum ReadResultSource {
    /// Served from the local disk cache.
    Disk,
    /// Fetched from Object Store.
    Upstream,
}

/// The head metadata returned by [`CachedObjectStore::maybe_prefetch_range`],
/// plus where the head was served from (`Disk` = warm, `Upstream` = cold
/// prefetch).
struct PrefetchedHead {
    meta: ObjectMeta,
    attributes: Attributes,
    extensions: Extensions,
    head_source: ReadResultSource,
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum InvalidGetRange {
    #[error("Range start too large, requested: {requested}, length: {length}")]
    StartTooLarge { requested: u64, length: u64 },

    #[error("Range started at {start} and ended at {end}")]
    Inconsistent { start: u64, end: u64 },
}

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
/// A mirror of [`object_store::GetRange`] that implements [`Hash`] and [`Eq`],
/// allowing it to be used as a key in hash-based collections (e.g. `SingleFlight`).
enum GetRangeKey {
    Bounded(Range<u64>),
    Offset(u64),
    Suffix(u64),
}

impl From<GetRange> for GetRangeKey {
    fn from(range: GetRange) -> Self {
        match range {
            GetRange::Bounded(r) => GetRangeKey::Bounded(r),
            GetRange::Offset(o) => GetRangeKey::Offset(o),
            GetRange::Suffix(s) => GetRangeKey::Suffix(s),
        }
    }
}

#[cfg(test)]
mod tests {
    use bytes::Bytes;
    use object_store::{
        path::Path, GetOptions, GetRange, MultipartUpload, ObjectStore, ObjectStoreExt,
        PutMultipartOptions, PutPayload,
    };
    use rand::Rng;
    use rstest::rstest;
    use std::sync::Arc;
    use std::time::Duration;

    use super::{CachedObjectStore, ReadResultSource};
    use crate::cached_object_store::policy::CachePutConfig;
    use crate::cached_object_store::stats::CachedObjectStoreStats;
    use crate::cached_object_store::storage::{LocalCacheStorage, PartID};
    use crate::cached_object_store::storage_fs::FsCacheEntry;
    use crate::cached_object_store::storage_fs::FsCacheStorage;
    use crate::db_state::SstType;
    use crate::instrumented_object_store::{InstrumentedObjectStore, ObjectStoreComponent};
    use crate::object_store_tag::{ObjectStoreCallTag, TableStoreKind};
    use crate::retrying_object_store::RetryingObjectStore;
    use crate::test_utils::{
        gen_rand_bytes, ExtensionMarker, ExtensionObjectStore, FlakyObjectStore, GatedObjectStore,
    };
    use crate::utils::ObjectStoreType;
    use slatedb_common::clock::DefaultSystemClock;
    use slatedb_common::metrics::MetricsRecorderHelper;
    use slatedb_common::DbRand;

    fn new_test_cache_folder() -> std::path::PathBuf {
        let mut rng = rand::rng();
        let dir_name: String = (0..10)
            .map(|_| rng.sample(rand::distr::Alphanumeric) as char)
            .collect();
        let path = format!("/tmp/testcache-{}", dir_name);
        let _ = std::fs::remove_dir_all(&path);
        std::path::PathBuf::from(path)
    }

    fn new_cached_store(object_store: Arc<dyn ObjectStore>) -> Arc<CachedObjectStore> {
        new_cached_store_with_part_size(object_store, 1024)
    }

    fn new_cached_store_with_part_size(
        object_store: Arc<dyn ObjectStore>,
        part_size_bytes: usize,
    ) -> Arc<CachedObjectStore> {
        let test_cache_folder = new_test_cache_folder();
        let recorder = MetricsRecorderHelper::noop();
        let stats = Arc::new(CachedObjectStoreStats::new(&recorder));
        let cache_storage = Arc::new(FsCacheStorage::new(
            test_cache_folder,
            None,
            None,
            stats.clone(),
            Arc::new(DefaultSystemClock::new()),
            Arc::new(DbRand::default()),
            1000,
        ));
        CachedObjectStore::new(
            object_store,
            cache_storage,
            part_size_bytes,
            CachePutConfig::default(),
            stats,
        )
        .unwrap()
    }

    #[tokio::test]
    async fn test_upstream_part_range_does_not_retain_full_part() {
        let part_size = 4 * 1024 * 1024;
        let part = Bytes::from(vec![7_u8; part_size]);
        let range = 1024..5120;
        let source_range_ptr = part[range.clone()].as_ptr();
        let location = Path::from("test");
        let object_store = Arc::new(object_store::memory::InMemory::new());
        object_store
            .put(&location, PutPayload::from_bytes(part))
            .await
            .unwrap();
        let cached_store = new_cached_store_with_part_size(object_store, part_size);

        let (copied, source) = cached_store
            .read_part(&location, 0, range, false)
            .await
            .unwrap();

        assert!(matches!(source, ReadResultSource::Upstream));
        assert_eq!(copied.len(), 4096);
        assert!(copied.iter().all(|byte| *byte == 7));
        assert_ne!(copied.as_ptr(), source_range_ptr);
    }

    #[tokio::test]
    async fn test_save_result_not_aligned() -> object_store::Result<()> {
        let payload = gen_rand_bytes(1024 * 3 + 32);
        let object_store = Arc::new(object_store::memory::InMemory::new());
        let test_cache_folder = new_test_cache_folder();
        let recorder = MetricsRecorderHelper::noop();
        let stats = Arc::new(CachedObjectStoreStats::new(&recorder));
        object_store
            .put(
                &Path::from("/data/testfile1"),
                PutPayload::from_bytes(payload.clone()),
            )
            .await?;
        let location = Path::from("/data/testfile1");
        let get_result = object_store.get(&location).await?;

        let cache_storage = Arc::new(FsCacheStorage::new(
            test_cache_folder.clone(),
            None,
            None,
            stats.clone(),
            Arc::new(DefaultSystemClock::new()),
            Arc::new(DbRand::default()),
            1000,
        ));

        let part_size = 1024;
        let cached_store = CachedObjectStore::new(
            object_store.clone(),
            cache_storage,
            part_size,
            CachePutConfig::default(),
            stats,
        )
        .unwrap();
        let entry = cached_store.cache_storage.entry(&location, 1024);

        let object_size_hint = cached_store.save_get_result(&location, get_result).await?;
        assert_eq!(object_size_hint, 1024 * 3 + 32);

        // assert the cached meta
        let head = entry.read_head().await?;
        assert_eq!(head.unwrap().0.size, 1024 * 3 + 32);

        // assert the parts
        let cached_parts = entry.cached_parts().await?;
        assert_eq!(cached_parts.len(), 4);
        assert_eq!(
            entry.read_part(0, 0..part_size).await?,
            Some(payload.slice(0..1024))
        );
        assert_eq!(
            entry.read_part(1, 0..part_size).await?,
            Some(payload.slice(1024..2048))
        );
        assert_eq!(
            entry.read_part(2, 0..part_size).await?,
            Some(payload.slice(2048..3072))
        );
        // check that the unaligned part was also cached
        assert_eq!(
            entry.read_part(3, 0..32).await?,
            Some(payload.slice(3072..3104))
        );

        // delete part 2, known_cache_size is still known
        let evict_part_path =
            FsCacheEntry::make_part_path(test_cache_folder.clone(), &location, 2, 1024);
        std::fs::remove_file(evict_part_path).unwrap();
        assert_eq!(entry.read_part(2, 0..part_size).await?, None);
        let cached_parts = entry.cached_parts().await?;
        assert_eq!(cached_parts, vec![0, 1, 3]);

        // delete part 3, known_cache_size become None
        let evict_part_path =
            FsCacheEntry::make_part_path(test_cache_folder.clone(), &location, 3, 1024);
        std::fs::remove_file(evict_part_path).unwrap();
        assert_eq!(entry.read_part(3, 0..part_size).await?, None);
        let cached_parts = entry.cached_parts().await?;
        assert_eq!(cached_parts, vec![0, 1]);
        Ok(())
    }

    #[tokio::test]
    async fn test_save_result_aligned() -> object_store::Result<()> {
        let payload = gen_rand_bytes(1024 * 3);
        let object_store = Arc::new(object_store::memory::InMemory::new());
        let test_cache_folder = new_test_cache_folder();
        let recorder = MetricsRecorderHelper::noop();
        let stats = Arc::new(CachedObjectStoreStats::new(&recorder));
        object_store
            .put(
                &Path::from("/data/testfile1"),
                PutPayload::from_bytes(payload.clone()),
            )
            .await?;
        let location = Path::from("/data/testfile1");
        let get_result = object_store.get(&location).await?;
        let part_size = 1024;

        let cache_storage = Arc::new(FsCacheStorage::new(
            test_cache_folder.clone(),
            None,
            None,
            stats.clone(),
            Arc::new(DefaultSystemClock::new()),
            Arc::new(DbRand::default()),
            1000,
        ));

        let cached_store = CachedObjectStore::new(
            object_store,
            cache_storage,
            part_size,
            CachePutConfig::default(),
            stats,
        )
        .unwrap();
        let entry = cached_store.cache_storage.entry(&location, part_size);
        let object_size_hint = cached_store.save_get_result(&location, get_result).await?;
        assert_eq!(object_size_hint, 1024 * 3);
        let cached_parts = entry.cached_parts().await?;
        assert_eq!(cached_parts.len(), 3);
        assert_eq!(
            entry.read_part(0, 0..part_size).await?,
            Some(payload.slice(0..1024))
        );
        assert_eq!(
            entry.read_part(1, 0..part_size).await?,
            Some(payload.slice(1024..2048))
        );
        assert_eq!(
            entry.read_part(2, 0..part_size).await?,
            Some(payload.slice(2048..3072))
        );

        let evict_part_path =
            FsCacheEntry::make_part_path(test_cache_folder.clone(), &location, 2, part_size);
        std::fs::remove_file(evict_part_path).unwrap();
        assert_eq!(entry.read_part(2, 0..part_size).await?, None);

        let cached_parts = entry.cached_parts().await?;
        assert_eq!(cached_parts.len(), 2);
        Ok(())
    }

    #[tokio::test]
    async fn test_cached_get_opts_preserves_extensions_on_cache_miss() {
        let inner: Arc<dyn ObjectStore> = Arc::new(object_store::memory::InMemory::new());
        let location = Path::from("/data/test_extensions_get");
        inner
            .put(
                &location,
                PutPayload::from_bytes(Bytes::from_static(b"hello world")),
            )
            .await
            .unwrap();

        let marking: Arc<dyn ObjectStore> = Arc::new(ExtensionObjectStore::new(inner));
        let cached_store = new_cached_store(marking);
        let result = cached_store
            .cached_get_opts(
                &location,
                GetOptions {
                    range: Some(GetRange::Bounded(0..5)),
                    ..Default::default()
                },
                false,
            )
            .await
            .expect("cache miss should fetch from inner store");

        assert!(result.extensions.get::<ExtensionMarker>().is_some());
        assert_eq!(result.bytes().await.unwrap(), Bytes::from_static(b"hello"));
    }

    #[tokio::test]
    async fn test_cached_head_preserves_extensions_on_cache_miss() {
        let inner: Arc<dyn ObjectStore> = Arc::new(object_store::memory::InMemory::new());
        let location = Path::from("/data/test_extensions_head");
        inner
            .put(
                &location,
                PutPayload::from_bytes(Bytes::from_static(b"hello")),
            )
            .await
            .unwrap();

        let marking: Arc<dyn ObjectStore> = Arc::new(ExtensionObjectStore::new(inner));
        let cached_store = new_cached_store(marking);
        let result = cached_store
            .cached_head(&location, true)
            .await
            .expect("cache miss should fetch head from inner store");

        assert!(result.extensions.get::<ExtensionMarker>().is_some());
    }

    #[test]
    fn test_split_range_into_parts() {
        let object_store = Arc::new(object_store::memory::InMemory::new());
        let test_cache_folder = new_test_cache_folder();
        let recorder = MetricsRecorderHelper::noop();
        let stats = Arc::new(CachedObjectStoreStats::new(&recorder));
        let cache_storage = Arc::new(FsCacheStorage::new(
            test_cache_folder,
            None,
            None,
            stats.clone(),
            Arc::new(DefaultSystemClock::new()),
            Arc::new(DbRand::default()),
            1000,
        ));

        let cached_store = CachedObjectStore::new(
            object_store,
            cache_storage,
            1024,
            CachePutConfig::default(),
            stats,
        )
        .unwrap();

        struct Test {
            input: (Option<GetRange>, usize),
            expect: Vec<(PartID, std::ops::Range<usize>)>,
        }
        let tests = [
            Test {
                input: (None, 1024 * 3),
                expect: vec![(0, 0..1024), (1, 0..1024), (2, 0..1024)],
            },
            Test {
                input: (None, 1024 * 3 + 12),
                expect: vec![(0, 0..1024), (1, 0..1024), (2, 0..1024), (3, 0..12)],
            },
            Test {
                input: (None, 12),
                expect: vec![(0, 0..12)],
            },
            Test {
                input: (Some(GetRange::Bounded(0..1024)), 1024),
                expect: vec![(0, 0..1024)],
            },
            Test {
                input: (Some(GetRange::Bounded(128..1024)), 20000),
                expect: vec![(0, 128..1024)],
            },
            Test {
                input: (Some(GetRange::Bounded(128..1024 + 12)), 20000),
                expect: vec![(0, 128..1024), (1, 0..12)],
            },
            Test {
                input: (Some(GetRange::Bounded(128..1024 * 2 + 12)), 20000),
                expect: vec![(0, 128..1024), (1, 0..1024), (2, 0..12)],
            },
            Test {
                input: (Some(GetRange::Bounded(1024 * 2..1024 * 3 + 12)), 200000),
                expect: vec![(2, 0..1024), (3, 0..12)],
            },
            Test {
                input: (Some(GetRange::Bounded(1024 * 2 - 2..1024 * 3 + 12)), 20000),
                expect: vec![(1, 1022..1024), (2, 0..1024), (3, 0..12)],
            },
            Test {
                input: (Some(GetRange::Suffix(128)), 1024),
                expect: vec![(0, 896..1024)],
            },
            Test {
                input: (Some(GetRange::Suffix(1024 * 2 + 8)), 1024 * 4),
                expect: vec![(1, 1016..1024), (2, 0..1024), (3, 0..1024)],
            },
            Test {
                input: (Some(GetRange::Offset(8)), 1024 * 4),
                expect: vec![(0, 8..1024), (1, 0..1024), (2, 0..1024), (3, 0..1024)],
            },
            Test {
                input: (Some(GetRange::Offset(1024 * 2 + 8)), 1024 * 4),
                expect: vec![(2, 8..1024), (3, 0..1024)],
            },
            Test {
                input: (Some(GetRange::Offset(1024 * 2 + 8)), 1024 * 4 + 2),
                expect: vec![(2, 8..1024), (3, 0..1024), (4, 0..2)],
            },
        ];

        for t in tests.iter() {
            let range = cached_store
                .canonicalize_range(t.input.0.clone(), t.input.1 as u64)
                .unwrap();
            let parts = cached_store.split_range_into_parts(range);
            assert_eq!(parts, t.expect, "input: {:?}", t.input);
        }
    }

    #[test]
    fn test_align_range() {
        let object_store = Arc::new(object_store::memory::InMemory::new());
        let test_cache_folder = new_test_cache_folder();
        let recorder = MetricsRecorderHelper::noop();
        let stats = Arc::new(CachedObjectStoreStats::new(&recorder));
        let cache_storage = Arc::new(FsCacheStorage::new(
            test_cache_folder,
            None,
            None,
            stats.clone(),
            Arc::new(DefaultSystemClock::new()),
            Arc::new(DbRand::default()),
            1000,
        ));
        let cached_store = CachedObjectStore::new(
            object_store,
            cache_storage,
            1024,
            CachePutConfig::default(),
            stats,
        )
        .unwrap();

        let aligned = cached_store.align_range(&(9..1025), 1024);
        assert_eq!(aligned, 0..2048);
        let aligned = cached_store.align_range(&(1024 + 1..2048 + 4), 1024);
        assert_eq!(aligned, 1024..3072);
    }

    #[test]
    fn test_align_get_range() {
        let object_store = Arc::new(object_store::memory::InMemory::new());
        let test_cache_folder = new_test_cache_folder();
        let recorder = MetricsRecorderHelper::noop();
        let stats = Arc::new(CachedObjectStoreStats::new(&recorder));
        let cache_storage = Arc::new(FsCacheStorage::new(
            test_cache_folder,
            None,
            None,
            stats.clone(),
            Arc::new(DefaultSystemClock::new()),
            Arc::new(DbRand::default()),
            1000,
        ));
        let cached_store = CachedObjectStore::new(
            object_store,
            cache_storage,
            1024,
            CachePutConfig::default(),
            stats,
        )
        .unwrap();

        let aligned = cached_store.align_get_range(&GetRange::Bounded(9..1025));
        assert_eq!(aligned, GetRange::Bounded(0..2048));
        let aligned = cached_store.align_get_range(&GetRange::Bounded(9..2048));
        assert_eq!(aligned, GetRange::Bounded(0..2048));
        let aligned = cached_store.align_get_range(&GetRange::Suffix(12));
        assert_eq!(aligned, GetRange::Suffix(1024));
        let aligned = cached_store.align_get_range(&GetRange::Suffix(1024));
        assert_eq!(aligned, GetRange::Suffix(1024));
        let aligned = cached_store.align_get_range(&GetRange::Offset(1024));
        assert_eq!(aligned, GetRange::Offset(1024));
        let aligned = cached_store.align_get_range(&GetRange::Offset(12));
        assert_eq!(aligned, GetRange::Offset(0));
    }

    #[tokio::test]
    async fn test_cached_object_store_impl_object_store() -> object_store::Result<()> {
        let object_store = Arc::new(object_store::memory::InMemory::new());
        let test_cache_folder = new_test_cache_folder();
        let recorder = MetricsRecorderHelper::noop();
        let stats = Arc::new(CachedObjectStoreStats::new(&recorder));
        let cache_storage = Arc::new(FsCacheStorage::new(
            test_cache_folder.clone(),
            None,
            None,
            stats.clone(),
            Arc::new(DefaultSystemClock::new()),
            Arc::new(DbRand::default()),
            1000,
        ));
        let cached_store = CachedObjectStore::new(
            object_store.clone(),
            cache_storage,
            1024,
            CachePutConfig::default(),
            stats,
        )
        .unwrap();

        let test_path = Path::from("/data/testdata1");
        let test_payload = gen_rand_bytes(1024 * 3 + 2);
        object_store
            .put(&test_path, PutPayload::from_bytes(test_payload.clone()))
            .await?;

        // test get entire object
        let test_ranges = vec![
            Some(GetRange::Offset(260817)),
            None,
            Some(GetRange::Bounded(1000..2048)),
            Some(GetRange::Bounded(1000..260817)),
            Some(GetRange::Suffix(10)),
            Some(GetRange::Suffix(260817)),
            Some(GetRange::Offset(1000)),
            Some(GetRange::Offset(0)),
            Some(GetRange::Offset(1028)),
            Some(GetRange::Offset(260817)),
            Some(GetRange::Offset(1024 * 3 + 2)),
            Some(GetRange::Offset(1024 * 3 + 1)),
            #[allow(clippy::reversed_empty_ranges)]
            Some(GetRange::Bounded(2900..2048)),
            Some(GetRange::Bounded(10..10)),
        ];

        // test get a range
        for range in test_ranges.iter() {
            let want = object_store
                .get_opts(
                    &test_path,
                    GetOptions {
                        range: range.clone(),
                        ..Default::default()
                    },
                )
                .await;
            let got = cached_store
                .cached_get_opts(
                    &test_path,
                    GetOptions {
                        range: range.clone(),
                        ..Default::default()
                    },
                    false,
                )
                .await;
            match (want, got) {
                (Ok(want), Ok(got)) => {
                    assert_eq!(want.range, got.range);
                    assert_eq!(want.meta, got.meta);
                    assert_eq!(want.bytes().await?, got.bytes().await?);
                }
                (Err(want), Err(got)) => {
                    if want.to_string().to_lowercase().contains("range") {
                        assert!(got.to_string().to_lowercase().contains("range"));
                    }
                }
                (origin_result, cached_result) => {
                    panic!("expect: {:?}, got: {:?}", origin_result, cached_result);
                }
            }
        }
        Ok(())
    }

    #[tokio::test]
    async fn test_preload_cache() {
        let cache_dir = new_test_cache_folder();
        let recorder = MetricsRecorderHelper::noop();
        let stats = Arc::new(CachedObjectStoreStats::new(&recorder));
        let cache_storage = Arc::new(FsCacheStorage::new(
            cache_dir,
            Some(10 * 1024 * 1024), // 10MB
            None,
            stats.clone(),
            Arc::new(DefaultSystemClock::new()),
            Arc::new(DbRand::default()),
            1000,
        ));

        let object_store = Arc::new(object_store::memory::InMemory::new());

        let cached_store = CachedObjectStore::new(
            object_store.clone(),
            cache_storage,
            1024,
            CachePutConfig::default(),
            stats,
        )
        .unwrap();

        // Create some test files to preload
        let test_paths = vec![
            Path::from("file1.sst"),
            Path::from("file2.sst"),
            Path::from("file3.sst"),
        ];

        let test_data = gen_rand_bytes(2048); // 2KB per file

        // Put test files in object store
        for path in &test_paths {
            object_store
                .put(path, PutPayload::from_bytes(test_data.clone()))
                .await
                .unwrap();
        }

        // Test preloading with max cache size
        cached_store
            .load_files_to_cache(test_paths.clone(), 10 * 1024) // 10KB limit
            .await
            .unwrap();

        // Verify that files are cached by checking if we can read from cache
        for path in &test_paths {
            let entry = cached_store.cache_storage.entry(path, 1024);
            let cached_parts = entry.cached_parts().await.unwrap();
            assert_eq!(cached_parts.len(), 2); // 2KB = 2 parts of 1024 bytes
        }
    }

    #[tokio::test]
    async fn test_preload_cache_above_limit() {
        let cache_dir = new_test_cache_folder();
        let recorder = MetricsRecorderHelper::noop();
        let stats = Arc::new(CachedObjectStoreStats::new(&recorder));
        let cache_storage = Arc::new(FsCacheStorage::new(
            cache_dir,
            Some(10 * 1024 * 1024), // 10MB
            None,
            stats.clone(),
            Arc::new(DefaultSystemClock::new()),
            Arc::new(DbRand::default()),
            1000,
        ));

        let object_store = Arc::new(object_store::memory::InMemory::new());

        let cached_store = CachedObjectStore::new(
            object_store.clone(),
            cache_storage,
            1024,
            CachePutConfig::default(),
            stats,
        )
        .unwrap();

        // Create some test files
        let test_paths = vec![Path::from("file1.sst"), Path::from("file2.sst")];

        let test_data = gen_rand_bytes(2048); // 2KB per file

        // Put test files in object store
        for path in &test_paths {
            object_store
                .put(path, PutPayload::from_bytes(test_data.clone()))
                .await
                .unwrap();
        }

        // Test load_files_to_cache with 0 bytes limit (should load nothing)
        cached_store
            .load_files_to_cache(test_paths.clone(), 0)
            .await
            .unwrap();

        // Verify that files are NOT cached since preloading was disabled
        for path in &test_paths {
            let entry = cached_store.cache_storage.entry(path, 1024);
            let cached_parts = entry.cached_parts().await.unwrap();
            assert_eq!(cached_parts.len(), 0); // No parts should be cached
        }
    }

    /// Helper to build a CachedObjectStore backed by an InstrumentedObjectStore so
    /// we can assert on the number of actual object-store requests made.
    fn build_instrumented_cached_store(
        inner: Arc<dyn ObjectStore>,
    ) -> (
        Arc<slatedb_common::metrics::DefaultMetricsRecorder>,
        Arc<CachedObjectStore>,
    ) {
        use crate::instrumented_object_store::{InstrumentedObjectStore, ObjectStoreComponent};
        use crate::utils::ObjectStoreType;
        use slatedb_common::metrics::test_recorder_helper;

        let (recorder, helper) = test_recorder_helper();
        let instrumented = Arc::new(InstrumentedObjectStore::new(
            inner,
            &helper,
            ObjectStoreComponent::Db,
            ObjectStoreType::Main,
        ));

        let test_cache_folder = new_test_cache_folder();
        let noop_helper = MetricsRecorderHelper::noop();
        let stats = Arc::new(CachedObjectStoreStats::new(&noop_helper));
        let cache_storage = Arc::new(FsCacheStorage::new(
            test_cache_folder,
            None,
            None,
            stats.clone(),
            Arc::new(DefaultSystemClock::new()),
            Arc::new(DbRand::default()),
            1000,
        ));

        let cached_store = CachedObjectStore::new(
            instrumented as Arc<dyn ObjectStore>,
            cache_storage,
            1024,
            CachePutConfig::default(),
            stats,
        )
        .unwrap();

        (recorder, cached_store)
    }

    fn get_request_count(
        recorder: &slatedb_common::metrics::DefaultMetricsRecorder,
        api: &str,
    ) -> i64 {
        use crate::instrumented_object_store::stats::REQUEST_COUNT;
        use slatedb_common::metrics::lookup_metric_with_labels;

        let labels = [
            ("component", "db"),
            ("store_type", "main"),
            ("op", "get"),
            ("api", api),
        ];
        lookup_metric_with_labels(recorder, REQUEST_COUNT, &labels).unwrap_or(0)
    }

    #[tokio::test]
    async fn test_single_flight_deduplicates_concurrent_head_requests() {
        // Set up an object in the backing store.
        let mem: Arc<dyn ObjectStore> = Arc::new(object_store::memory::InMemory::new());
        let path = Path::from("data/test_head_dedup");
        mem.put(&path, PutPayload::from_bytes(gen_rand_bytes(512)))
            .await
            .unwrap();

        // Wrap with a gate-controlled store so we can block callers deterministically.
        let gated = Arc::new(GatedObjectStore::new(mem));
        gated.head_gate.close();
        let (recorder, cached_store) = build_instrumented_cached_store(gated.clone());

        // Launch many concurrent head requests for the same path.
        let mut handles = Vec::new();
        for _ in 0..10 {
            let store = cached_store.clone();
            let p = path.clone();
            handles.push(tokio::spawn(
                async move { store.cached_head(&p, true).await },
            ));
        }

        // Wait until exactly 1 caller arrives at the gate (SingleFlight dedup
        // ensures only one caller reaches the head gate).
        gated.head_gate.wait_for_arrivals(1).await;
        assert_eq!(
            gated.head_gate.arrivals(),
            1,
            "SingleFlight should let only 1 through"
        );

        // Release the gate — success path.
        gated.head_gate.release();

        for handle in handles {
            handle.await.unwrap().unwrap();
        }

        // SingleFlight should collapse them into a single actual HEAD request.
        let count = get_request_count(&recorder, "head");
        assert_eq!(
            count, 1,
            "expected 1 actual object store request, got {count}"
        );
    }

    #[tokio::test]
    async fn test_single_flight_deduplicates_concurrent_get_opts_requests() {
        let mem: Arc<dyn ObjectStore> = Arc::new(object_store::memory::InMemory::new());
        let path = Path::from("data/test_get_dedup");
        let payload = gen_rand_bytes(2048);
        mem.put(&path, PutPayload::from_bytes(payload.clone()))
            .await
            .unwrap();

        let gated = Arc::new(GatedObjectStore::new(mem));
        gated.get_opts_gate.close();
        let (recorder, cached_store) = build_instrumented_cached_store(gated.clone());

        // Launch many concurrent get_opts requests for the same path and range.
        let mut handles = Vec::new();
        for _ in 0..10 {
            let store = cached_store.clone();
            let p = path.clone();
            handles.push(tokio::spawn(async move {
                let opts = GetOptions {
                    range: Some(GetRange::Bounded(0..1024)),
                    ..Default::default()
                };
                let result = store.cached_get_opts(&p, opts, false).await?;
                result.bytes().await
            }));
        }

        // Wait for the single winning caller to arrive at the gate.
        gated.get_opts_gate.wait_for_arrivals(1).await;
        assert_eq!(
            gated.get_opts_gate.arrivals(),
            1,
            "SingleFlight should let only 1 through"
        );

        // Release — success.
        gated.get_opts_gate.release();

        for handle in handles {
            handle.await.unwrap().unwrap();
        }

        // The prefetch SingleFlight should collapse all prefetch GETs into one.
        // Part reads may also be deduplicated. Total GET count should be much less than 10.
        let count = get_request_count(&recorder, "get_range");
        assert!(
            count <= 2,
            "expected at most 2 actual object store requests (prefetch + maybe 1 part), got {count}"
        );
    }

    #[tokio::test]
    async fn test_single_flight_allows_independent_paths() {
        // Requests to different paths should NOT be deduplicated.
        let mem: Arc<dyn ObjectStore> = Arc::new(object_store::memory::InMemory::new());
        let paths: Vec<Path> = (0..5)
            .map(|i| Path::from(format!("data/independent_{}", i)))
            .collect();
        for p in &paths {
            mem.put(p, PutPayload::from_bytes(gen_rand_bytes(512)))
                .await
                .unwrap();
        }

        let gated = Arc::new(GatedObjectStore::new(mem));
        gated.head_gate.close();
        let (recorder, cached_store) = build_instrumented_cached_store(gated.clone());

        let mut handles = Vec::new();
        for p in &paths {
            let store = cached_store.clone();
            let p = p.clone();
            handles.push(tokio::spawn(
                async move { store.cached_head(&p, true).await },
            ));
        }

        // Each distinct path has its own SingleFlight key, so all 5 should arrive.
        gated.head_gate.wait_for_arrivals(5).await;
        assert_eq!(
            gated.head_gate.arrivals(),
            5,
            "different keys should each pass through SingleFlight independently"
        );

        // Release all.
        gated.head_gate.release();

        for handle in handles {
            handle.await.unwrap().unwrap();
        }

        // Each distinct path should result in its own request.
        let count = get_request_count(&recorder, "head");
        assert_eq!(
            count, 5,
            "expected 5 actual object store requests (one per path), got {count}"
        );
    }

    #[tokio::test]
    async fn test_single_flight_different_ranges_are_independent() {
        // Requests with different ranges should be treated as separate flights.
        let mem: Arc<dyn ObjectStore> = Arc::new(object_store::memory::InMemory::new());
        let path = Path::from("data/test_range_independent");
        let payload = gen_rand_bytes(4096);
        mem.put(&path, PutPayload::from_bytes(payload.clone()))
            .await
            .unwrap();

        let gated = Arc::new(GatedObjectStore::new(mem));
        gated.get_opts_gate.close();
        let (recorder, cached_store) = build_instrumented_cached_store(gated.clone());

        let ranges = vec![
            Some(GetRange::Bounded(0..1024)),
            Some(GetRange::Bounded(1024..2048)),
            Some(GetRange::Suffix(512)),
        ];

        let mut handles = Vec::new();
        for range in ranges {
            let store = cached_store.clone();
            let p = path.clone();
            handles.push(tokio::spawn(async move {
                let opts = GetOptions {
                    range,
                    ..Default::default()
                };
                store.cached_get_opts(&p, opts, false).await
            }));
        }

        // Each distinct range maps to a different key, so all 3 should arrive.
        gated.get_opts_gate.wait_for_arrivals(3).await;
        assert_eq!(
            gated.get_opts_gate.arrivals(),
            3,
            "different ranges should each pass through SingleFlight independently"
        );

        // Release all.
        gated.get_opts_gate.release();

        for handle in handles {
            handle.await.unwrap().unwrap();
        }

        // Each distinct range key should trigger its own prefetch request.
        let count = get_request_count(&recorder, "get_range");
        assert!(
            count >= 3,
            "expected at least 3 object store requests (one per distinct range), got {count}"
        );
    }

    #[tokio::test]
    async fn test_single_flight_concurrent_callers_see_gate_failure() {
        // When the gate is configured to fail, all concurrent waiters on the
        // same SingleFlight key should receive an error (not hang forever).
        let mem: Arc<dyn ObjectStore> = Arc::new(object_store::memory::InMemory::new());
        let path = Path::from("data/test_gate_failure");
        mem.put(&path, PutPayload::from_bytes(gen_rand_bytes(512)))
            .await
            .unwrap();

        let gated = Arc::new(GatedObjectStore::new(mem));
        gated.head_gate.close();
        let (_, cached_store) = build_instrumented_cached_store(gated.clone());

        // Launch concurrent head requests.
        let mut handles = Vec::new();
        for _ in 0..5 {
            let store = cached_store.clone();
            let p = path.clone();
            handles.push(tokio::spawn(
                async move { store.cached_head(&p, true).await },
            ));
        }

        // Wait for the single winning caller to arrive at the gate.
        gated.head_gate.wait_for_arrivals(1).await;

        // Inject failure, then release.
        gated.head_gate.set_error(|| object_store::Error::Generic {
            store: "test",
            source: Box::new(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                "injected test failure",
            )),
        });
        gated.head_gate.release();

        // All callers should see an error.
        for handle in handles {
            let result = handle.await.unwrap();
            assert!(result.is_err(), "expected error when gate injects failure");
        }
    }

    #[tokio::test]
    async fn test_single_flight_retries_after_gate_failure() {
        // After a failure, the SingleFlight should not cache the error,
        // allowing the next call to succeed fresh.
        let mem: Arc<dyn ObjectStore> = Arc::new(object_store::memory::InMemory::new());
        let path = Path::from("data/test_retry_after_fail");
        mem.put(&path, PutPayload::from_bytes(gen_rand_bytes(512)))
            .await
            .unwrap();

        let gated = Arc::new(GatedObjectStore::new(mem));
        gated.head_gate.close();
        let (_, cached_store) = build_instrumented_cached_store(gated.clone());

        // First call — configure failure.
        let store = cached_store.clone();
        let p = path.clone();
        let handle = tokio::spawn(async move { store.cached_head(&p, true).await });

        gated.head_gate.wait_for_arrivals(1).await;
        gated.head_gate.set_error(|| object_store::Error::Generic {
            store: "test",
            source: Box::new(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                "injected test failure",
            )),
        });
        gated.head_gate.release();

        let result = handle.await.unwrap();
        assert!(result.is_err(), "first call should fail");

        // Second call — configure success.
        gated.head_gate.clear_error();
        let store = cached_store.clone();
        let p = path.clone();
        let handle = tokio::spawn(async move { store.cached_head(&p, true).await });

        gated.head_gate.wait_for_arrivals(2).await;
        gated.head_gate.release();

        let result = handle.await.unwrap();
        assert!(result.is_ok(), "second call should succeed after retry");
    }

    #[tokio::test]
    async fn test_single_flight_part_fetch_with_get_range_failures() {
        // Validates that when fetching parts fails transiently, the SingleFlight
        // does not permanently cache the failure, and retries succeed.
        let mem: Arc<dyn ObjectStore> = Arc::new(object_store::memory::InMemory::new());
        let path = Path::from("data/test_part_flaky");
        let payload = gen_rand_bytes(4096);
        mem.put(&path, PutPayload::from_bytes(payload.clone()))
            .await
            .unwrap();

        // Use a FlakyObjectStore that fails the first get_range call.
        let flaky = Arc::new(FlakyObjectStore::new(mem, 0).with_get_range_failures(1));
        let (_, cached_store) = build_instrumented_cached_store(flaky.clone());

        // First, prime metadata via a full get (get_opts doesn't use get_range).
        let prime_opts = GetOptions {
            range: None,
            ..Default::default()
        };
        let result = cached_store
            .cached_get_opts(&path, prime_opts, false)
            .await
            .unwrap();
        let _ = result.bytes().await.unwrap();

        // Now try reading — the parts should be cached from the full get above,
        // so even though get_range is flaky, we should succeed from cache.
        let opts = GetOptions {
            range: Some(GetRange::Bounded(0..512)),
            ..Default::default()
        };
        let result = cached_store.cached_get_opts(&path, opts, false).await;
        assert!(result.is_ok());
        let bytes = result.unwrap().bytes().await.unwrap();
        assert_eq!(&bytes[..], &payload[..512]);
    }

    #[tokio::test]
    async fn test_part_fetch_validates_truncated_body_and_retries() {
        let part_size = 1024usize;
        let payload = gen_rand_bytes(part_size * 3);
        let location = Path::from("/data/testfile1");
        let inner: Arc<dyn ObjectStore> = Arc::new(object_store::memory::InMemory::new());
        inner
            .put(&location, PutPayload::from_bytes(payload.clone()))
            .await
            .unwrap();

        let test_cache_folder = new_test_cache_folder();
        let recorder = MetricsRecorderHelper::noop();
        let stats = Arc::new(CachedObjectStoreStats::new(&recorder));
        let cache_storage = Arc::new(FsCacheStorage::new(
            test_cache_folder.clone(),
            None,
            None,
            stats.clone(),
            Arc::new(DefaultSystemClock::new()),
            Arc::new(DbRand::default()),
            1000,
        ));
        let opts = || GetOptions {
            range: Some(GetRange::Bounded(0..(part_size as u64 * 3))),
            extensions: ObjectStoreCallTag::new(TableStoreKind::Main, SstType::Compacted).into(),
            ..Default::default()
        };

        // Prefill the cache through a clean handle, then delete one part file
        // so the read below must fill it from the backend lazily.
        let prefill = CachedObjectStore::new(
            inner.clone(),
            cache_storage.clone(),
            part_size,
            CachePutConfig::default(),
            stats.clone(),
        )
        .unwrap();
        prefill
            .get_opts(&location, opts())
            .await
            .unwrap()
            .bytes()
            .await
            .unwrap();
        let part_path =
            FsCacheEntry::make_part_path(test_cache_folder.clone(), &location, 1, part_size);
        std::fs::remove_file(&part_path).unwrap();

        // The backend truncates the next ranged body to 1 byte but reports
        // success, mimicking a response cut mid-body without a stream error.
        let flaky = Arc::new(FlakyObjectStore::new(inner, 0).with_truncate_get_range_bytes(1, 1));
        let cached = CachedObjectStore::new(
            flaky.clone(),
            cache_storage,
            part_size,
            CachePutConfig::default(),
            stats,
        )
        .unwrap();
        let instrumented = Arc::new(InstrumentedObjectStore::new(
            cached,
            &recorder,
            ObjectStoreComponent::Db,
            ObjectStoreType::Main,
        ));
        let retrying = RetryingObjectStore::new(
            instrumented,
            Arc::new(DbRand::default()),
            Arc::new(DefaultSystemClock::new()),
            None,
        );

        let got = retrying
            .get_opts(&location, opts())
            .await
            .unwrap()
            .bytes()
            .await
            .unwrap();
        assert_eq!(got, payload);
        // The truncated fill plus the successful fill on the reissued read.
        assert_eq!(flaky.get_range_attempts(), 2);
    }

    #[rstest::rstest]
    #[case::no_evictor_cached(false, true)]
    #[case::with_evictor_cached(true, true)]
    #[case::no_evictor_uncached(false, false)]
    #[case::with_evictor_uncached(true, false)]
    #[tokio::test]
    async fn test_delete(#[case] evictor: bool, #[case] cached: bool) {
        const PART_SIZE: usize = 1024;

        let location1 = Path::from("/data/testfile1");
        let location2 = Path::from("/data/testfile2");

        let test_cache_folder = new_test_cache_folder();
        let payload = gen_rand_bytes(PART_SIZE * 3);
        let object_store = Arc::new(object_store::memory::InMemory::new());
        let recorder = MetricsRecorderHelper::noop();
        let stats = Arc::new(CachedObjectStoreStats::new(&recorder));

        object_store
            .put(&location1, PutPayload::from_bytes(payload.clone()))
            .await
            .unwrap();
        object_store
            .put(&location2, PutPayload::from_bytes(payload.clone()))
            .await
            .unwrap();

        let cache_storage = Arc::new(FsCacheStorage::new(
            test_cache_folder.clone(),
            evictor.then_some(1024 * 1024),
            None,
            stats.clone(),
            Arc::new(DefaultSystemClock::new()),
            Arc::new(DbRand::default()),
            1000,
        ));

        let cached_store = CachedObjectStore::new(
            object_store,
            Arc::clone(&cache_storage) as Arc<dyn LocalCacheStorage>,
            PART_SIZE,
            CachePutConfig::default(),
            stats,
        )
        .unwrap();
        cached_store.start_evictor().await;

        // Populate the cache through the normal read path. Untagged reads bypass
        // the cache, so tag these as a cacheable (main, compacted) read.
        let cacheable = || GetOptions {
            extensions: ObjectStoreCallTag::new(TableStoreKind::Main, SstType::Compacted).into(),
            ..GetOptions::default()
        };
        if cached {
            cached_store
                .get_opts(&location1, cacheable())
                .await
                .unwrap()
                .bytes()
                .await
                .unwrap();
        }
        cached_store
            .get_opts(&location2, cacheable())
            .await
            .unwrap()
            .bytes()
            .await
            .unwrap();

        let entry1 = cached_store.cache_storage.entry(&location1, PART_SIZE);
        let parts1 = entry1.cached_parts().await.unwrap();
        if cached {
            assert_eq!(parts1.len(), 3, "{parts1:?}");
            assert_eq!(cache_storage.file_handle_cache_population(), 6);
        } else {
            assert_eq!(parts1.len(), 0, "{parts1:?}");
            assert_eq!(cache_storage.file_handle_cache_population(), 3);
        }

        let entry2 = cached_store.cache_storage.entry(&location2, PART_SIZE);
        let parts2 = entry2.cached_parts().await.unwrap();
        assert_eq!(parts2.len(), 3, "{parts2:?}");

        cached_store.delete(&location1).await.unwrap();
        if evictor {
            // XXX: If evictor is running, deletion is performed asynchronously
            //      from the evictor "thread".
            tokio::time::sleep(Duration::from_secs(3)).await;
        }

        let entry1 = cached_store.cache_storage.entry(&location1, PART_SIZE);
        let parts1 = entry1.cached_parts().await.unwrap();
        assert_eq!(parts1.len(), 0, "{parts1:?}");
        assert_eq!(cache_storage.file_handle_cache_population(), 3);

        let entry2 = cached_store.cache_storage.entry(&location2, PART_SIZE);
        let parts2 = entry2.cached_parts().await.unwrap();
        assert_eq!(parts2.len(), 3, "{parts2:?}");

        // verify repeated delete is idempotent
        cached_store.delete(&location1).await.unwrap();
        let entry1 = cached_store.cache_storage.entry(&location1, PART_SIZE);
        let parts1 = entry1.cached_parts().await.unwrap();
        assert_eq!(parts1.len(), 0, "{parts1:?}");
        assert_eq!(cache_storage.file_handle_cache_population(), 3);
    }

    fn policy_test_store(
        upstream: Arc<dyn ObjectStore>,
        policy: CachePutConfig,
    ) -> Arc<CachedObjectStore> {
        let recorder = MetricsRecorderHelper::noop();
        let stats = Arc::new(CachedObjectStoreStats::new(&recorder));
        let cache_storage = Arc::new(FsCacheStorage::new(
            new_test_cache_folder(),
            None,
            None,
            stats.clone(),
            Arc::new(DefaultSystemClock::new()),
            Arc::new(DbRand::default()),
            1000,
        ));
        CachedObjectStore::new(upstream, cache_storage, 1024, policy, stats).unwrap()
    }

    fn put_opts_tagged(tag: ObjectStoreCallTag) -> object_store::PutOptions {
        object_store::PutOptions {
            extensions: tag.into(),
            ..Default::default()
        }
    }

    fn get_opts_tagged(tag: ObjectStoreCallTag) -> GetOptions {
        GetOptions {
            extensions: tag.into(),
            ..Default::default()
        }
    }

    async fn cached_part_count(store: &CachedObjectStore, location: &Path) -> usize {
        let cache_location = location.clone();
        store
            .cache_storage
            .entry(&cache_location, store.part_size_bytes)
            .cached_parts()
            .await
            .unwrap()
            .len()
    }

    #[rstest]
    // WAL writes are never cached, even with both flags enabled.
    #[case(
        ObjectStoreCallTag::new(TableStoreKind::Main, SstType::Wal),
        CachePutConfig { cache_on_flush: true, cache_on_compaction: true },
        0
    )]
    // Flush writes (main store, compacted) cached only when cache_on_flush is set.
    #[case(
        ObjectStoreCallTag::new(TableStoreKind::Main, SstType::Compacted),
        CachePutConfig { cache_on_flush: true, cache_on_compaction: false },
        2
    )]
    #[case(
        ObjectStoreCallTag::new(TableStoreKind::Main, SstType::Compacted),
        CachePutConfig { cache_on_flush: false, cache_on_compaction: true },
        0
    )]
    // Compaction writes (compactor store, compacted) cached only when
    // cache_on_compaction is set.
    #[case(
        ObjectStoreCallTag::new(TableStoreKind::Compactor, SstType::Compacted),
        CachePutConfig { cache_on_flush: false, cache_on_compaction: true },
        2
    )]
    #[case(
        ObjectStoreCallTag::new(TableStoreKind::Compactor, SstType::Compacted),
        CachePutConfig { cache_on_flush: true, cache_on_compaction: false },
        0
    )]
    #[tokio::test]
    async fn test_put_caching_by_tag(
        #[case] tag: ObjectStoreCallTag,
        #[case] policy: CachePutConfig,
        #[case] expected_parts: usize,
    ) {
        let upstream: Arc<dyn ObjectStore> = Arc::new(object_store::memory::InMemory::new());
        let store = policy_test_store(upstream.clone(), policy);

        let location = Path::from("compacted/01.sst");
        let payload = gen_rand_bytes(2048); // 2 parts of 1024 bytes
        store
            .put_opts(
                &location,
                PutPayload::from_bytes(payload),
                put_opts_tagged(tag),
            )
            .await
            .unwrap();

        assert_eq!(cached_part_count(&store, &location).await, expected_parts);
    }

    #[tokio::test]
    async fn test_untagged_put_is_not_cached() {
        let upstream: Arc<dyn ObjectStore> = Arc::new(object_store::memory::InMemory::new());
        let store = policy_test_store(
            upstream.clone(),
            CachePutConfig {
                cache_on_flush: true,
                cache_on_compaction: true,
            },
        );

        // No tag in the options: coordination I/O (manifest, etc.) is never cached.
        let location = Path::from("manifest/01.manifest");
        let payload = gen_rand_bytes(2048);
        store
            .put_opts(
                &location,
                PutPayload::from_bytes(payload),
                object_store::PutOptions::default(),
            )
            .await
            .unwrap();

        assert_eq!(cached_part_count(&store, &location).await, 0);
    }

    #[tokio::test]
    async fn test_compactor_get_bypasses_cache() {
        let upstream: Arc<dyn ObjectStore> = Arc::new(object_store::memory::InMemory::new());
        let store = policy_test_store(upstream.clone(), CachePutConfig::default());

        let location = Path::from("compacted/01.sst");
        let payload = gen_rand_bytes(2048);
        upstream
            .put(&location, PutPayload::from_bytes(payload.clone()))
            .await
            .unwrap();

        // A compactor read returns the bytes but caches nothing.
        let got = store
            .get_opts(
                &location,
                get_opts_tagged(ObjectStoreCallTag::new(
                    TableStoreKind::Compactor,
                    SstType::Compacted,
                )),
            )
            .await
            .unwrap()
            .bytes()
            .await
            .unwrap();
        assert_eq!(got, payload);
        assert_eq!(cached_part_count(&store, &location).await, 0);

        // A main read of the same object caches it: the bypass is compactor specific.
        store
            .get_opts(
                &location,
                get_opts_tagged(ObjectStoreCallTag::new(
                    TableStoreKind::Main,
                    SstType::Compacted,
                )),
            )
            .await
            .unwrap()
            .bytes()
            .await
            .unwrap();
        assert_eq!(cached_part_count(&store, &location).await, 2);
    }

    #[rstest]
    #[case::no_evictor(None)]
    #[case::with_evictor(Some(64 * 1024 * 1024))]
    #[tokio::test]
    async fn test_retry_get_refetches_stale_part(#[case] max_cache_size_bytes: Option<usize>) {
        let upstream: Arc<dyn ObjectStore> = Arc::new(object_store::memory::InMemory::new());
        let recorder = MetricsRecorderHelper::noop();
        let stats = Arc::new(CachedObjectStoreStats::new(&recorder));
        let cache_storage = Arc::new(FsCacheStorage::new(
            new_test_cache_folder(),
            max_cache_size_bytes,
            None,
            stats.clone(),
            Arc::new(DefaultSystemClock::new()),
            Arc::new(DbRand::default()),
            1000,
        ));
        let store = CachedObjectStore::new(
            upstream.clone(),
            cache_storage,
            1024,
            CachePutConfig::default(),
            stats,
        )
        .unwrap();
        store.start_evictor().await;

        let location = Path::from("compacted/01.sst");
        // 512 bytes is below the 1024 byte part size, so the object is a single
        // part (part 0) and `cached_part_count` is 1.
        let good = gen_rand_bytes(512);
        upstream
            .put(&location, PutPayload::from_bytes(good.clone()))
            .await
            .unwrap();

        // Populate the cache with the correct head and part via a normal read.
        let main_tag = ObjectStoreCallTag::new(TableStoreKind::Main, SstType::Compacted);
        let got = store
            .get_opts(&location, get_opts_tagged(main_tag.clone()))
            .await
            .unwrap()
            .bytes()
            .await
            .unwrap();
        assert_eq!(got, good);
        assert_eq!(cached_part_count(&store, &location).await, 1);

        // Poison part 0 on disk so the cache would otherwise serve corrupt bytes.
        let bad = gen_rand_bytes(512);
        let cache_location = location.clone();
        store
            .cache_storage
            .entry(&cache_location, store.part_size_bytes)
            .save_part(0, bad.clone())
            .await
            .unwrap();

        // A normal read now serves the poisoned bytes (cache hit).
        let served = store
            .get_opts(&location, get_opts_tagged(main_tag.clone()))
            .await
            .unwrap()
            .bytes()
            .await
            .unwrap();
        assert_eq!(served, bad);

        // A reissued (retry) read force-refreshes from upstream, healing the part.
        let retry_tag = ObjectStoreCallTag {
            kind: TableStoreKind::Main,
            sst_type: SstType::Compacted,
            retry: Some(crate::error::RetryReason::CrcMismatch),
            segment: Some(Bytes::new()),
        };
        let refetched = store
            .get_opts(&location, get_opts_tagged(retry_tag))
            .await
            .unwrap()
            .bytes()
            .await
            .unwrap();
        assert_eq!(
            refetched, good,
            "retry should refetch durable upstream bytes"
        );

        // The cache now holds the corrected part: a later normal read serves good bytes.
        let after = store
            .get_opts(&location, get_opts_tagged(main_tag))
            .await
            .unwrap()
            .bytes()
            .await
            .unwrap();
        assert_eq!(after, good);
    }

    #[tokio::test]
    async fn test_compactor_head_reads_without_admitting() {
        let upstream: Arc<dyn ObjectStore> = Arc::new(object_store::memory::InMemory::new());
        let store = policy_test_store(upstream.clone(), CachePutConfig::default());

        let location = Path::from("compacted/01.sst");
        let payload = gen_rand_bytes(512);
        upstream
            .put(&location, PutPayload::from_bytes(payload.clone()))
            .await
            .unwrap();
        let cache_location = location.clone();
        let read_head = || {
            let entry = store
                .cache_storage
                .entry(&cache_location, store.part_size_bytes);
            async move { entry.read_head().await.unwrap() }
        };
        let compactor_head = || GetOptions {
            head: true,
            extensions: ObjectStoreCallTag::new(TableStoreKind::Compactor, SstType::Compacted)
                .into(),
            ..Default::default()
        };

        // On a miss the compactor HEAD serves the metadata but must not admit a
        // head-only entry, which would defeat a later foreground range prefetch.
        let result = store.get_opts(&location, compactor_head()).await.unwrap();
        assert_eq!(result.meta.size, payload.len() as u64);
        assert!(
            read_head().await.is_none(),
            "compactor HEAD must not admit a head-only entry on a miss"
        );

        // But it still serves an already-cached head: populate via a main HEAD,
        // then the compactor HEAD is served from it.
        let main_head = GetOptions {
            head: true,
            extensions: ObjectStoreCallTag::new(TableStoreKind::Main, SstType::Compacted).into(),
            ..Default::default()
        };
        store.get_opts(&location, main_head).await.unwrap();
        assert!(
            read_head().await.is_some(),
            "a main HEAD reads through and populates the cache head"
        );
        let served = store.get_opts(&location, compactor_head()).await.unwrap();
        assert_eq!(
            served.meta.size,
            payload.len() as u64,
            "compactor HEAD should serve the already-cached head"
        );
    }

    #[tokio::test]
    async fn test_wal_read_bypasses_cache() {
        let upstream: Arc<dyn ObjectStore> = Arc::new(object_store::memory::InMemory::new());
        let store = policy_test_store(upstream.clone(), CachePutConfig::default());

        let location = Path::from("wal/00000000000000000001.sst");
        let payload = gen_rand_bytes(2048);
        upstream
            .put(&location, PutPayload::from_bytes(payload.clone()))
            .await
            .unwrap();
        let wal_tag = ObjectStoreCallTag::new(TableStoreKind::Main, SstType::Wal);

        // A WAL data read bypasses: it returns the bytes but caches nothing.
        let got = store
            .get_opts(&location, get_opts_tagged(wal_tag.clone()))
            .await
            .unwrap()
            .bytes()
            .await
            .unwrap();
        assert_eq!(got, payload);
        assert_eq!(cached_part_count(&store, &location).await, 0);

        // A WAL HEAD bypasses too: it does not populate the cache head.
        let wal_head = GetOptions {
            head: true,
            extensions: wal_tag.into(),
            ..Default::default()
        };
        store.get_opts(&location, wal_head).await.unwrap();
        let cache_location = location.clone();
        let head = store
            .cache_storage
            .entry(&cache_location, store.part_size_bytes)
            .read_head()
            .await
            .unwrap();
        assert!(head.is_none(), "WAL HEAD must not populate the cache");
    }

    #[tokio::test]
    async fn test_put_writes_head_and_serves_first_read_from_cache() {
        let upstream: Arc<dyn ObjectStore> = Arc::new(object_store::memory::InMemory::new());
        let store = policy_test_store(
            upstream.clone(),
            CachePutConfig {
                cache_on_flush: true,
                cache_on_compaction: false,
            },
        );

        // A flush write (main store, compacted) is cached and commits a head.
        let location = Path::from("compacted/01.sst");
        let payload = gen_rand_bytes(2048);
        let tag = ObjectStoreCallTag::new(TableStoreKind::Main, SstType::Compacted);
        store
            .put_opts(
                &location,
                PutPayload::from_bytes(payload.clone()),
                put_opts_tagged(tag.clone()),
            )
            .await
            .unwrap();

        // The head is written on the write path, with the right size.
        let cache_location = location.clone();
        let head = store
            .cache_storage
            .entry(&cache_location, store.part_size_bytes)
            .read_head()
            .await
            .unwrap();
        assert_eq!(
            head.expect("head should be written on the put").0.size,
            2048
        );

        // With the head and parts cached, the first read is served entirely from
        // the cache: deleting the object upstream must not affect it. (Without a
        // head, the read would prefetch from the now-missing upstream and fail.)
        upstream.delete(&location).await.unwrap();
        let got = store
            .get_opts(&location, get_opts_tagged(tag))
            .await
            .unwrap()
            .bytes()
            .await
            .unwrap();
        assert_eq!(got, payload);
    }

    fn multipart_opts(tag: ObjectStoreCallTag) -> PutMultipartOptions {
        PutMultipartOptions {
            extensions: tag.into(),
            ..Default::default()
        }
    }

    #[rstest]
    // Uploaded chunks larger than the cache part size: every put_part flushes
    // a full cache part and carries a remainder into the next call.
    #[case(1500, 2, vec![1024, 1024, 952])]
    // Uploaded chunks equal to the cache part size.
    #[case(1024, 2, vec![1024, 1024])]
    // Uploaded chunks smaller than the cache part size.
    #[case(400, 3, vec![1024, 176])]
    // Total upload smaller than the cache part size.
    #[case(400, 2, vec![800])]
    #[tokio::test]
    async fn test_multipart_compacted_upload_is_cached(
        #[case] chunk_size: usize,
        #[case] num_chunks: usize,
        #[case] expected_part_sizes: Vec<usize>,
    ) {
        let upstream: Arc<dyn ObjectStore> = Arc::new(object_store::memory::InMemory::new());
        let store = policy_test_store(
            upstream.clone(),
            CachePutConfig {
                cache_on_flush: false,
                cache_on_compaction: true,
            },
        );

        // A compaction output written as a multipart upload (the path large
        // compacted SSTs take). The tag survives multipart init, so no fallback
        // is involved.
        let location = Path::from("compacted/big.sst");
        let chunks: Vec<Bytes> = (0..num_chunks)
            .map(|_| gen_rand_bytes(chunk_size))
            .collect();
        let tag = ObjectStoreCallTag::new(TableStoreKind::Compactor, SstType::Compacted);
        let mut upload = store
            .put_multipart_opts(&location, multipart_opts(tag))
            .await
            .unwrap();
        for chunk in &chunks {
            upload.put_part(chunk.clone().into()).await.unwrap();
        }
        upload.complete().await.unwrap();

        let cache_location = location.clone();
        let entry = store
            .cache_storage
            .entry(&cache_location, store.part_size_bytes);
        let cached = entry.cached_parts().await.unwrap();
        let expected_part_ids: Vec<PartID> = (0..expected_part_sizes.len()).collect();
        assert_eq!(cached, expected_part_ids);

        // The teed bytes round-trip in order through cache parts of the
        // expected sizes.
        let expected: Vec<u8> = chunks.iter().flat_map(|c| c.to_vec()).collect();
        assert_eq!(expected_part_sizes.iter().sum::<usize>(), expected.len());
        let mut offset = 0;
        for (part_id, part_size) in expected_part_sizes.iter().enumerate() {
            let bytes = entry
                .read_part(part_id, 0..*part_size)
                .await
                .unwrap()
                .unwrap();
            assert_eq!(&bytes[..], &expected[offset..offset + part_size]);
            offset += part_size;
        }
    }

    #[rstest]
    // A compacted multipart upload is not cached when its source is disabled,
    // even if the other source is enabled.
    #[case(
        ObjectStoreCallTag::new(TableStoreKind::Compactor, SstType::Compacted),
        CachePutConfig { cache_on_flush: true, cache_on_compaction: false }
    )]
    // A WAL multipart upload is never cached, even with both flags on.
    #[case(
        ObjectStoreCallTag::new(TableStoreKind::Main, SstType::Wal),
        CachePutConfig { cache_on_flush: true, cache_on_compaction: true }
    )]
    #[tokio::test]
    async fn test_multipart_upload_not_cached(
        #[case] tag: ObjectStoreCallTag,
        #[case] policy: CachePutConfig,
    ) {
        let upstream: Arc<dyn ObjectStore> = Arc::new(object_store::memory::InMemory::new());
        let store = policy_test_store(upstream.clone(), policy);

        let location = Path::from("compacted/big.sst");
        let mut upload = store
            .put_multipart_opts(&location, multipart_opts(tag))
            .await
            .unwrap();
        upload.put_part(gen_rand_bytes(2048).into()).await.unwrap();
        upload.complete().await.unwrap();

        assert_eq!(cached_part_count(&store, &location).await, 0);
    }

    #[tokio::test]
    async fn test_multipart_head_is_the_commit_point() {
        let upstream: Arc<dyn ObjectStore> = Arc::new(object_store::memory::InMemory::new());
        let store = policy_test_store(
            upstream.clone(),
            CachePutConfig {
                cache_on_flush: false,
                cache_on_compaction: true,
            },
        );

        let location = Path::from("compacted/big.sst");
        let cache_location = location.clone();
        let tag = ObjectStoreCallTag::new(TableStoreKind::Compactor, SstType::Compacted);
        let mut upload = store
            .put_multipart_opts(&location, multipart_opts(tag))
            .await
            .unwrap();
        upload.put_part(gen_rand_bytes(2048).into()).await.unwrap();

        // Before complete, parts may be on disk but the entry is not committed:
        // no head, so a read would treat it as a miss and refetch from upstream.
        let head_before = store
            .cache_storage
            .entry(&cache_location, store.part_size_bytes)
            .read_head()
            .await
            .unwrap();
        assert!(
            head_before.is_none(),
            "entry must not be committed before complete"
        );

        upload.complete().await.unwrap();

        // complete writes the head, committing the entry.
        let head_after = store
            .cache_storage
            .entry(&cache_location, store.part_size_bytes)
            .read_head()
            .await
            .unwrap();
        assert_eq!(
            head_after
                .expect("head should be committed on complete")
                .0
                .size,
            2048
        );
    }
}
