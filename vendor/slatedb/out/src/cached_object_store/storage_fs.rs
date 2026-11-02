use crate::cached_object_store::stats::CachedObjectStoreStats;
use bytes::Bytes;
use chrono::{DateTime, Utc};
use log::{debug, error, warn};
use lru::LruCache;
use object_store::path::Path;
use object_store::{Attributes, ObjectMeta};
use rand::{distr::Alphanumeric, Rng};
use slatedb_common::clock::SystemClock;
use slatedb_common::DbRand;
use std::collections::{HashMap, HashSet};
use std::fmt::Display;
use std::io::Write;
use std::num::NonZeroUsize;
use std::ops::Range;
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, OnceCell};
use walkdir::WalkDir;

use crate::cached_object_store::storage::{LocalCacheEntry, LocalCacheHead, LocalCacheStorage};
use crate::utils::format_bytes_si;

/// A cached file handle node. Callers that obtain an `Arc<CachedFileHandle>`
/// keep the underlying fd alive even after the entry is evicted from the cache.
#[derive(Debug)]
pub(crate) struct CachedFileHandle {
    file: std::fs::File,
}

impl CachedFileHandle {
    pub(crate) fn file(&self) -> &std::fs::File {
        &self.file
    }
}

/// A cache of open file descriptors, keyed by filesystem path.
///
/// Uses a `Mutex` protecting an `LruCache` for O(1) lookup, promotion, and
/// eviction. Individual file reads use positional I/O (`pread` /
/// `read_exact_at`) which does not touch the file cursor, so multiple threads
/// can read from the same `Arc<CachedFileHandle>` concurrently without any
/// per-file locking.
#[derive(Clone)]
pub(crate) struct FileHandleCache {
    inner: Arc<std::sync::Mutex<LruCache<std::path::PathBuf, Arc<CachedFileHandle>>>>,
}

impl std::fmt::Debug for FileHandleCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let inner = self.inner.lock().expect("lock should not be poisoned");
        f.debug_struct("FileHandleCache")
            .field("len", &inner.len())
            .field("cap", &inner.cap())
            .finish()
    }
}

impl FileHandleCache {
    fn new(max_handles: usize) -> Self {
        Self {
            inner: Arc::new(std::sync::Mutex::new(LruCache::new(
                NonZeroUsize::new(max_handles).expect("max_handles must be > 0"),
            ))),
        }
    }

    /// Look up a cached file handle, or open the file and cache it.
    /// Returns `Ok(None)` if the file does not exist on disk.
    fn get_or_open(
        &self,
        path: &std::path::Path,
    ) -> Result<Option<Arc<CachedFileHandle>>, std::io::Error> {
        let mut cache = self.inner.lock().expect("lock should not be poisoned");
        if let Some(handle) = cache.get(path) {
            if Self::is_valid(handle, path) {
                return Ok(Some(handle.clone()));
            }
            // Stale entry — remove it so we reopen below.
            cache.pop(path);
        }

        let file = match std::fs::File::open(path) {
            Ok(f) => f,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(err) => return Err(err),
        };

        let handle = Arc::new(CachedFileHandle { file });

        cache.push(path.to_path_buf(), handle.clone());
        Ok(Some(handle))
    }

    /// Check whether a cached file descriptor still refers to a live file.
    ///
    /// On Unix an unlinked file keeps its data accessible through open fds,
    /// but `fstat` will report `nlink == 0`. This single in-kernel syscall is
    /// much cheaper than a full `open` and lets us detect deleted or replaced
    /// files without a TOCTOU-prone path `stat`.
    ///
    /// On non-Unix platforms (e.g. Windows), we fall back to checking whether
    /// the path still exists on disk.
    #[cfg(unix)]
    fn is_valid(handle: &CachedFileHandle, _path: &std::path::Path) -> bool {
        use std::os::unix::fs::MetadataExt;
        handle.file().metadata().is_ok_and(|m| m.nlink() > 0)
    }

    #[cfg(not(unix))]
    fn is_valid(_handle: &CachedFileHandle, path: &std::path::Path) -> bool {
        path.exists()
    }

    /// Remove a cached handle, e.g. after eviction or after a write replaces
    /// the file (since the cached fd would still reference the old inode).
    fn invalidate(&self, path: &std::path::Path) {
        let mut cache = self.inner.lock().expect("lock should not be poisoned");
        cache.pop(path);
    }
}

/// Cross-platform positional read. Reads exactly `buf.len()` bytes at `offset`
/// without altering the file cursor, allowing concurrent readers on the same fd.
fn read_exact_at_offset(file: &std::fs::File, buf: &mut [u8], offset: u64) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::FileExt;
        file.read_exact_at(buf, offset)
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::FileExt;
        let mut bytes_read = 0;
        while bytes_read < buf.len() {
            let n = file.seek_read(&mut buf[bytes_read..], offset + bytes_read as u64)?;
            if n == 0 {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "failed to fill whole buffer",
                ));
            }
            bytes_read += n;
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct FsCacheStorage {
    root_folder: std::path::PathBuf,
    evictor: Option<Arc<FsCacheEvictor>>,
    rand: Arc<DbRand>,
    file_handle_cache: FileHandleCache,
}

impl FsCacheStorage {
    pub fn new(
        root_folder: std::path::PathBuf,
        max_cache_size_bytes: Option<usize>,
        scan_interval: Option<Duration>,
        stats: Arc<CachedObjectStoreStats>,
        system_clock: Arc<dyn SystemClock>,
        rand: Arc<DbRand>,
        max_open_file_handles: usize,
    ) -> Self {
        let file_handle_cache = FileHandleCache::new(max_open_file_handles);
        let evictor = max_cache_size_bytes.map(|max_cache_size_bytes| {
            Arc::new(FsCacheEvictor::new(
                root_folder.clone(),
                max_cache_size_bytes,
                scan_interval,
                stats,
                system_clock,
                rand.clone(),
                file_handle_cache.clone(),
            ))
        });

        Self {
            root_folder,
            evictor,
            rand,
            file_handle_cache,
        }
    }

    #[cfg(test)]
    pub(crate) fn file_handle_cache_population(&self) -> usize {
        self.file_handle_cache.inner.lock().unwrap().len()
    }
}

#[async_trait::async_trait]
impl LocalCacheStorage for FsCacheStorage {
    fn entry(&self, location: &Path, part_size: usize) -> Box<dyn LocalCacheEntry> {
        Box::new(FsCacheEntry {
            root_folder: self.root_folder.clone(),
            location: location.clone(),
            evictor: self.evictor.clone(),
            part_size,
            rand: self.rand.clone(),
            file_handle_cache: self.file_handle_cache.clone(),
        })
    }

    async fn start_evictor(&self) {
        if let Some(evictor) = &self.evictor {
            evictor.start().await
        }
    }
}

impl Display for FsCacheStorage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "FsCacheStorage({})", self.root_folder.display())
    }
}

#[derive(Debug)]
pub(crate) struct FsCacheEntry {
    root_folder: std::path::PathBuf,
    location: Path,
    part_size: usize,
    evictor: Option<Arc<FsCacheEvictor>>,
    rand: Arc<DbRand>,
    file_handle_cache: FileHandleCache,
}

impl FsCacheEntry {
    async fn atomic_write(&self, path: std::path::PathBuf, buf: Bytes) -> object_store::Result<()> {
        let tmp_path = path.with_extension(format!("_tmp{}", self.make_rand_suffix()));

        // try triggering evict before writing
        if let Some(evictor) = &self.evictor {
            // If the evictor is backpressured, skip this cache write to avoid
            // stalling foreground PUTs. Cache writes are best-effort.
            if !evictor
                .track_entry_accessed(path.clone(), EntryAccess::Write(buf.len()))
                .await
            {
                return Ok(());
            }
        }

        // Spawn a blocking task and do synchronous I/O rather than use the tokio async apis.
        // Under the hood, on linux systems , tokio itself spawns a blocking task for each call to
        // drive i/o since it hasn't yet adopted the native fully async i/o api (io_uring). Each
        // blocking task adds overhead, so its better to just batch all the calls into a single
        // blocking task.
        // see https://github.com/slatedb/slatedb/pull/1342
        let invalidate_path = path.clone();
        #[allow(clippy::disallowed_methods)]
        tokio::task::spawn_blocking(move || {
            let tmp_path = tmp_path.as_path();
            // ensure the parent folder exists
            if let Some(folder_path) = tmp_path.parent() {
                std::fs::create_dir_all(folder_path).map_err(wrap_io_err)?;
            }

            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(tmp_path)
                .map_err(wrap_io_err)?;
            file.write_all(&buf).map_err(wrap_io_err)?;

            // Note: There is no fsync before the rename. The cache holds copies
            // of durable upstream bytes, so part durability is not required, only
            // correctness. The tmp file plus atomic rename means a reader never
            // observes a partially written part. If a crash leaves a renamed but
            // not yet flushed part that reads back corrupt, block validation
            // fails on read, the retried GET will override the cached entry.
            std::fs::rename(tmp_path, path).map_err(wrap_io_err)
        })
        .await?
        .map_err(wrap_io_err)?;

        // The rename replaced the file at `path`, so any previously cached
        // handle now points to the old (unlinked) inode. Invalidate it so
        // the next read opens the new file.
        self.file_handle_cache.invalidate(&invalidate_path);

        Ok(())
    }

    // every origin file will be split into multiple parts, and all the parts will be saved in the same
    // folder. the part file name is expected to be in the format of `_part{part_size}-{part_number}`,
    // examples: /tmp/mydata.csv/_part1mb-000000001
    pub(crate) fn make_part_path(
        root_folder: std::path::PathBuf,
        location: &Path,
        part_number: usize,
        part_size: usize,
    ) -> std::path::PathBuf {
        // containing the part size in the file name, allows user change the part size on
        // the fly, without the need to invalidate the cache.
        let part_size_name = if part_size.is_multiple_of(1024 * 1024) {
            format!("{}mb", part_size / (1024 * 1024))
        } else {
            format!("{}kb", part_size / 1024)
        };
        let suffix = format!("_part{}-{:09}", part_size_name, part_number);
        let mut path = root_folder.join(location.to_string());
        path.push(suffix);
        path
    }

    fn make_head_path(root_folder: std::path::PathBuf, location: &Path) -> std::path::PathBuf {
        let suffix = "_head".to_string();
        let mut path = root_folder.join(location.to_string());
        path.push(suffix);
        path
    }

    fn make_rand_suffix(&self) -> String {
        let mut rng = self.rand.rng();
        (0..24).map(|_| rng.sample(Alphanumeric) as char).collect()
    }
}

#[async_trait::async_trait]
impl LocalCacheEntry for FsCacheEntry {
    async fn save_part(&self, part_number: usize, buf: Bytes) -> object_store::Result<()> {
        let part_path = Self::make_part_path(
            self.root_folder.clone(),
            &self.location,
            part_number,
            self.part_size,
        );

        self.atomic_write(part_path, buf).await
    }

    async fn read_part(
        &self,
        part_number: usize,
        range_in_part: Range<usize>,
    ) -> object_store::Result<Option<Bytes>> {
        let part_path = Self::make_part_path(
            self.root_folder.clone(),
            &self.location,
            part_number,
            self.part_size,
        );

        // Spawn a blocking task and do synchronous I/O rather than use the tokio async apis.
        // Under the hood, on linux systems , tokio itself spawns a blocking task for each call to
        // drive i/o since it hasn't yet adopted the native fully async i/o api (io_uring). Each
        // blocking task adds overhead, so its better to just batch all the calls into a single
        // blocking task.
        // see https://github.com/slatedb/slatedb/pull/1342
        let file_cache = self.file_handle_cache.clone();
        let this_part_path = part_path.clone();
        #[allow(clippy::disallowed_methods)]
        let result = tokio::task::spawn_blocking(move || {
            let file = match file_cache.get_or_open(&this_part_path) {
                Ok(Some(f)) => f,
                Ok(None) => return Ok(None),
                Err(err) => return Err(wrap_io_err(err)),
            };

            // Use positional I/O (pread) — no seek required, and safe for
            // concurrent readers sharing the same Arc<File>.
            let mut buffer = vec![0; range_in_part.len()];
            read_exact_at_offset(file.file(), &mut buffer, range_in_part.start as u64)
                .map_err(wrap_io_err)?;
            Ok(Some(Bytes::from(buffer)))
        })
        .await
        .map_err(wrap_io_err)??;

        // track the part access for evictor
        if result.is_some() {
            if let Some(evictor) = &self.evictor {
                evictor
                    .track_entry_accessed(part_path, EntryAccess::Read(self.part_size))
                    .await;
            }
        }

        Ok(result)
    }

    #[cfg(test)]
    async fn cached_parts(
        &self,
    ) -> object_store::Result<Vec<crate::cached_object_store::storage::PartID>> {
        let head_path = Self::make_head_path(self.root_folder.clone(), &self.location);
        let directory_path = match head_path.parent() {
            Some(directory_path) => directory_path.to_path_buf(),
            None => return Ok(vec![]),
        };

        #[allow(clippy::disallowed_methods)]
        tokio::task::spawn_blocking(move || {
            let target_prefix = "_part";

            let entries = match std::fs::read_dir(&directory_path) {
                Ok(entries) => entries,
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(vec![]),
                Err(err) => return Err(wrap_io_err(err)),
            };

            let mut part_file_names = vec![];
            for entry in entries {
                let entry = entry.map_err(wrap_io_err)?;
                let metadata = entry.metadata().map_err(wrap_io_err)?;
                if metadata.is_dir() {
                    continue;
                }
                let file_name = entry.file_name();
                let file_name_str = file_name.to_string_lossy();
                if file_name_str.starts_with(target_prefix) {
                    part_file_names.push(file_name_str.to_string());
                }
            }

            // not cached at all
            if part_file_names.is_empty() {
                return Ok(vec![]);
            }

            // sort the paths in alphabetical order
            part_file_names.sort();

            // retrieve the part numbers from the paths
            let mut part_numbers = Vec::with_capacity(part_file_names.len());
            for part_file_name in part_file_names.iter() {
                let part_number = part_file_name
                    .split('-')
                    .next_back()
                    .and_then(|part_number| part_number.parse::<usize>().ok());
                if let Some(part_number) = part_number {
                    part_numbers.push(part_number);
                }
            }

            Ok(part_numbers)
        })
        .await
        .map_err(wrap_io_err)?
    }

    async fn save_head(&self, head: (&ObjectMeta, &Attributes)) -> object_store::Result<()> {
        // if the meta file exists and not corrupted, do nothing
        match self.read_head().await {
            Ok(Some(_)) => return Ok(()),
            Ok(None) => {}
            Err(_) => {
                // TODO: add a warning
            }
        }

        let head: LocalCacheHead = head.into();
        let buf: Bytes = serde_json::to_vec(&head).map_err(wrap_io_err)?.into();

        let meta_path = Self::make_head_path(self.root_folder.clone(), &self.location);

        self.atomic_write(meta_path, buf).await
    }

    async fn read_head(&self) -> object_store::Result<Option<(ObjectMeta, Attributes)>> {
        let head_path = Self::make_head_path(self.root_folder.clone(), &self.location);

        // Spawn a blocking task and do synchronous I/O rather than use the tokio async apis.
        // Under the hood, on linux systems , tokio itself spawns a blocking task for each call to
        // drive i/o since it hasn't yet adopted the native fully async i/o api (io_uring). Each
        // blocking task adds overhead, so its better to just batch all the calls into a single
        // blocking task.
        let file_cache = self.file_handle_cache.clone();
        let this_head_path = head_path.clone();
        #[allow(clippy::disallowed_methods)]
        let result = tokio::task::spawn_blocking(move || {
            let file = match file_cache.get_or_open(&this_head_path) {
                Ok(Some(f)) => f,
                Ok(None) => return Ok(None),
                Err(err) => return Err(wrap_io_err(err)),
            };

            let metadata = file.file().metadata().map_err(wrap_io_err)?;
            let head_size_bytes = metadata.len() as usize;

            // Use positional read from offset 0 to read the entire file.
            let mut buffer = vec![0u8; head_size_bytes];
            read_exact_at_offset(file.file(), &mut buffer, 0).map_err(wrap_io_err)?;

            let content = String::from_utf8(buffer).map_err(|e| {
                wrap_io_err(std::io::Error::new(std::io::ErrorKind::InvalidData, e))
            })?;

            let head: LocalCacheHead = serde_json::from_str(&content).map_err(wrap_io_err)?;
            Ok(Some((head.meta(), head.attributes(), head_size_bytes)))
        })
        .await
        .map_err(wrap_io_err)??;

        if let Some((meta, attributes, head_size_bytes)) = result {
            // track the head access for evictor
            if let Some(evictor) = &self.evictor {
                let head_path = Self::make_head_path(self.root_folder.clone(), &self.location);
                evictor
                    .track_entry_accessed(head_path, EntryAccess::Read(head_size_bytes))
                    .await;
            }
            Ok(Some((meta, attributes)))
        } else {
            Ok(None)
        }
    }

    async fn delete(&self) {
        let Some(path) = Self::make_head_path(self.root_folder.clone(), &self.location)
            .parent()
            .map(std::path::PathBuf::from)
        else {
            error!(
                "failed to obtain cache entry directory for {}",
                self.location
            );
            return;
        };

        if let Some(evictor) = &self.evictor {
            evictor
                .track_entry_accessed(path, EntryAccess::Delete)
                .await;
        } else {
            delete_cache_entry(path, self.file_handle_cache.clone()).await;
        }
    }
}

enum EntryAccess {
    Read(usize),
    Write(usize),
    Delete,
}

type FsCacheEvictorWork = (std::path::PathBuf, EntryAccess);
// Minimum time between aggregated "evictor queue is full" warnings.
const QUEUE_FULL_LOG_INTERVAL_MS: i64 = 30_000;

/// FsCacheEvictor evicts the cache entries when the cache size exceeds the limit. it is expected to
/// run in the background to avoid blocking the caller, and it'll be triggered whenever a new cache entry
/// is added.
#[derive(Debug)]
struct FsCacheEvictor {
    root_folder: std::path::PathBuf,
    max_cache_size_bytes: usize,
    scan_interval: Option<Duration>,
    tx: tokio::sync::mpsc::Sender<FsCacheEvictorWork>,
    rx: Mutex<Option<tokio::sync::mpsc::Receiver<FsCacheEvictorWork>>>,
    started: AtomicBool,
    queue_full_count: AtomicU64,
    last_queue_full_log_ms: AtomicI64,
    background_evict_handle: OnceCell<tokio::task::JoinHandle<()>>,
    background_scan_handle: OnceCell<tokio::task::JoinHandle<()>>,
    stats: Arc<CachedObjectStoreStats>,
    system_clock: Arc<dyn SystemClock>,
    rand: Arc<DbRand>,
    file_handle_cache: FileHandleCache,
}

impl FsCacheEvictor {
    fn new(
        root_folder: std::path::PathBuf,
        max_cache_size_bytes: usize,
        scan_interval: Option<Duration>,
        stats: Arc<CachedObjectStoreStats>,
        system_clock: Arc<dyn SystemClock>,
        rand: Arc<DbRand>,
        file_handle_cache: FileHandleCache,
    ) -> Self {
        let (tx, rx) = tokio::sync::mpsc::channel(100);
        Self {
            root_folder,
            scan_interval,
            max_cache_size_bytes,
            tx,
            rx: Mutex::new(Some(rx)),
            started: AtomicBool::new(false),
            queue_full_count: AtomicU64::new(0),
            last_queue_full_log_ms: AtomicI64::new(i64::MIN),
            background_evict_handle: OnceCell::new(),
            background_scan_handle: OnceCell::new(),
            stats,
            system_clock,
            rand,
            file_handle_cache,
        }
    }

    async fn start(&self) {
        let inner = Arc::new(FsCacheEvictorInner::new(
            self.root_folder.clone(),
            self.max_cache_size_bytes,
            self.stats.clone(),
            self.rand.clone(),
            self.file_handle_cache.clone(),
        ));

        let guard = self.rx.lock();
        let rx = guard.await.take().expect("evictor already started");

        self.started.store(true, Ordering::Release);

        // scan the cache folder (defaults as every 1 hour) to keep the in-memory cache_entries eventually
        // consistent with the cache folder.
        self.background_scan_handle
            .set(tokio::spawn(Self::background_scan(
                inner.clone(),
                self.scan_interval,
                self.system_clock.clone(),
            )))
            .ok();

        // start the background evictor task, it'll be triggered whenever a new cache entry is added
        self.background_evict_handle
            .set(tokio::spawn(Self::background_evict(
                inner,
                rx,
                self.system_clock.clone(),
            )))
            .ok();
    }

    fn started(&self) -> bool {
        self.started.load(Ordering::Acquire)
    }

    async fn background_evict(
        inner: Arc<FsCacheEvictorInner>,
        mut rx: tokio::sync::mpsc::Receiver<FsCacheEvictorWork>,
        system_clock: Arc<dyn SystemClock>,
    ) {
        loop {
            match rx.recv().await {
                Some((path, access)) => match access {
                    EntryAccess::Read(bytes) => {
                        inner
                            .track_entry_accessed(path, bytes, system_clock.now(), false)
                            .await;
                    }
                    EntryAccess::Write(bytes) => {
                        inner
                            .track_entry_accessed(path, bytes, system_clock.now(), true)
                            .await;
                    }
                    EntryAccess::Delete => {
                        inner.delete_entry(path).await;
                    }
                },
                None => return,
            }
        }
    }

    async fn background_scan(
        inner: Arc<FsCacheEvictorInner>,
        scan_interval: Option<Duration>,
        system_clock: Arc<dyn SystemClock>,
    ) {
        inner.clone().scan_entries(true).await;

        if let Some(scan_interval) = scan_interval {
            loop {
                system_clock.clone().sleep(scan_interval).await;
                inner.clone().scan_entries(true).await;
            }
        }
    }

    // Allow send() here because we should never see a closed channel. The evictor owns both the
    // sender and receiver. It doesn't close the channel, and both sender and receiver are dropped
    // when the evictor is dropped.
    #[allow(clippy::disallowed_methods)]
    async fn track_entry_accessed(&self, path: std::path::PathBuf, access: EntryAccess) -> bool {
        if !self.started() {
            return true;
        }

        match self.tx.try_send((path, access)) {
            Ok(()) => true,
            Err(tokio::sync::mpsc::error::TrySendError::Full(_)) => {
                self.queue_full_count.fetch_add(1, Ordering::AcqRel);
                let now_ms = self.system_clock.now().timestamp_millis();
                let last_log_ms = self.last_queue_full_log_ms.load(Ordering::Acquire);
                if now_ms.saturating_sub(last_log_ms) >= QUEUE_FULL_LOG_INTERVAL_MS
                    && self
                        .last_queue_full_log_ms
                        .compare_exchange(last_log_ms, now_ms, Ordering::AcqRel, Ordering::Acquire)
                        .is_ok()
                {
                    let queue_full_count = self.queue_full_count.swap(0, Ordering::AcqRel);
                    warn!(
                        "evictor queue skipped cache write/access event because it was full {} times in the last 30s",
                        queue_full_count,
                    );
                }
                false
            }
            Err(tokio::sync::mpsc::error::TrySendError::Closed(_)) => false,
        }
    }
}

#[derive(Debug, Clone)]
struct CacheEntry {
    access_time: DateTime<Utc>,
    size_bytes: usize,
    key_index: usize,
}

#[derive(Debug, Default)]
struct CacheState {
    entries: HashMap<std::path::PathBuf, CacheEntry>,
    keys: Vec<std::path::PathBuf>,
}

impl CacheState {
    fn remove_entry(&mut self, path: &std::path::PathBuf) -> Option<CacheEntry> {
        let removed = self.entries.remove(path);
        if let Some(removed) = &removed {
            self.keys.swap_remove(removed.key_index);
            if removed.key_index < self.keys.len() {
                let swapped_key = self.keys[removed.key_index].clone();
                if let Some(swapped) = self.entries.get_mut(&swapped_key) {
                    swapped.key_index = removed.key_index;
                }
            }
        }
        removed
    }
}

/// FsCacheEvictorInner manages the cache entries in `CacheState`, and evict the cache entries
/// when the cache size exceeds the limit. it uses a pick-of-2 strategy to approximate LRU, and evict
/// the older file when the cache size exceeds the limit.
///
/// On start up, FsCacheEvictorInner will scan the cache folder to load the cache files into the in-memory
/// trie cache_entries. This loading process is interleaved with the maybe_evict is being called, so the
/// cache entries should be wrapped with Mutex<_>.
#[derive(Debug)]
struct FsCacheEvictorInner {
    root_folder: std::path::PathBuf,
    max_cache_size_bytes: usize,
    track_lock: Mutex<()>,
    cache_state: Mutex<CacheState>,
    cache_size_bytes: AtomicU64,
    stats: Arc<CachedObjectStoreStats>,
    rand: Arc<DbRand>,
    file_handle_cache: FileHandleCache,
}

impl FsCacheEvictorInner {
    fn new(
        root_folder: std::path::PathBuf,
        max_cache_size_bytes: usize,
        stats: Arc<CachedObjectStoreStats>,
        rand: Arc<DbRand>,
        file_handle_cache: FileHandleCache,
    ) -> Self {
        Self {
            root_folder,
            max_cache_size_bytes,
            track_lock: Mutex::new(()),
            cache_state: Mutex::new(CacheState::default()),
            cache_size_bytes: AtomicU64::new(0_u64),
            stats,
            rand,
            file_handle_cache,
        }
    }

    // scan the cache folder, and load the cache entries into memory.
    // this function is only called on start up, and it's expected to run interleavely with
    // maybe_evict is being called.
    async fn scan_entries(self: Arc<Self>, evict: bool) {
        let root_folder = self.root_folder.clone();

        #[allow(clippy::disallowed_methods)]
        let paths = tokio::task::spawn_blocking(move || {
            WalkDir::new(&root_folder)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file())
                .map(|e| e.path().to_path_buf())
                .collect::<Vec<_>>()
        })
        .await
        .unwrap_or_default();

        for path in paths {
            let metadata = match tokio::fs::metadata(&path).await {
                Ok(metadata) => metadata,
                Err(err) => {
                    if err.kind() != std::io::ErrorKind::NotFound {
                        warn!(
                            "evictor failed to get the metadata of the cache file [path={:?}, error={}]",
                            path, err
                        );
                    }

                    continue;
                }
            };
            #[allow(clippy::disallowed_types)]
            let atime = metadata
                .accessed()
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
                .into();
            let bytes = metadata.len() as usize;

            self.track_entry_accessed(path, bytes, atime, evict).await;
        }
    }

    /// track the cache entry access, and evict the cache files when the cache size exceeds the limit if evict is true,
    /// return the bytes of the evicted files. please note that track_entry_accessed might be called concurrently from
    /// the rescanner and evictor tasks, it's expected to be wrapped with a lock to ensure the serial execution.
    async fn track_entry_accessed(
        &self,
        path: std::path::PathBuf,
        bytes: usize,
        accessed_time: DateTime<Utc>,
        evict: bool,
    ) -> usize {
        let _track_guard = self.track_lock.lock().await;

        let entry_count = {
            let mut cache_state = self.cache_state.lock().await;

            match cache_state.entries.get_mut(&path) {
                Some(entry) => {
                    entry.access_time = accessed_time;
                }
                None => {
                    let key_index = cache_state.keys.len();
                    cache_state.keys.push(path.clone());
                    cache_state.entries.insert(
                        path.clone(),
                        CacheEntry {
                            access_time: accessed_time,
                            size_bytes: bytes,
                            key_index,
                        },
                    );
                    self.cache_size_bytes
                        .fetch_add(bytes as u64, Ordering::SeqCst);
                }
            }
            cache_state.entries.len()
        };

        self.stats.object_store_cache_keys.set(entry_count as i64);
        self.stats
            .object_store_cache_bytes
            .set(self.cache_size_bytes.load(Ordering::Relaxed) as i64);

        // if the cache size is still below the limit, do nothing
        if self.cache_size_bytes.load(Ordering::Relaxed) <= self.max_cache_size_bytes as u64 {
            return 0;
        }
        // TODO: check the disk space ratio here, if the disk space is low, also triggers evict.

        // The maximum byte size the cache will take up on disk. If a write would cause the
        // cache to exceed this threshold, entries are evicted using an 2-random strategy until
        // the cache reaches 90% of `max_cache_size_bytes`.
        //
        // It's ok to call evict after inserting the new entry, because we will evict entries with eailer `accessed_time`.
        // This ensures that the newly added entry will not be evicted immediately.
        let evicted_bytes: usize = if evict
            && self.cache_size_bytes.load(Ordering::Relaxed) > self.max_cache_size_bytes as u64
        {
            // We sacrifice floating-point precision error to prevent possible overflow(i.e. self.max_cache_size_bytes * 9 / 10).
            let target_size = ((self.max_cache_size_bytes as f64) * 0.9) as u64;
            self.evict_to_target_size(target_size).await
        } else {
            0
        };

        evicted_bytes
    }

    /// Evict cache entries until the cache size is below the target size.
    /// This method acquires the lock once to pick all eviction targets, then releases the lock
    /// to delete files, and finally acquires the lock again to update the state.
    async fn evict_to_target_size(&self, target_size: u64) -> usize {
        let picked_targets = self.pick_evict_targets(target_size).await;

        if picked_targets.is_empty() {
            if self.cache_size_bytes.load(Ordering::Relaxed) > target_size {
                warn!(
                    "cache_size_bytes still exceeds max_cache_size_bytes but no more entries can be evicted(cache_size_bytes={}, max_cache_size_bytes={})",
                    format_bytes_si(self.cache_size_bytes.load(Ordering::Relaxed)),
                    format_bytes_si(self.max_cache_size_bytes as u64)
                );
            }
            return 0;
        }

        let mut deleted_targets: Vec<(std::path::PathBuf, usize)> =
            Vec::with_capacity(picked_targets.len());
        for (target, target_bytes) in picked_targets {
            // if the file is not found, still try to remove it from the cache_entries, and decrease the cache_size_bytes.
            // this might happen when the file is removed by other processes, or due to a race between the background
            // scan (which collects paths then processes them) and eviction deleting files in between.
            match tokio::fs::remove_file(&target).await {
                Ok(()) => {
                    debug!(
                        "evictor evicted cache file [path={:?}, bytes={}]",
                        target,
                        format_bytes_si(target_bytes as u64)
                    );
                    deleted_targets.push((target.clone(), target_bytes));
                    self.file_handle_cache.invalidate(&target);
                }
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                    // File already gone, still need to clean up state
                    deleted_targets.push((target.clone(), target_bytes));
                    self.file_handle_cache.invalidate(&target);
                }
                Err(err) => {
                    warn!("evictor failed to remove the cache file [error={}]", err);
                    // Skip this file, don't add to deleted_targets
                }
            }
        }

        if deleted_targets.is_empty() {
            return 0;
        }

        let (entry_count, total_evicted_bytes) = {
            let mut cache_state = self.cache_state.lock().await;
            let mut total_bytes: usize = 0;

            for (target, target_bytes) in deleted_targets.iter() {
                if cache_state.remove_entry(target).is_some() {
                    self.cache_size_bytes
                        .fetch_sub(*target_bytes as u64, Ordering::SeqCst);
                    total_bytes += target_bytes;
                }
            }

            (cache_state.entries.len(), total_bytes)
        };

        // Sync the metrics after eviction
        self.stats
            .object_store_cache_evicted_bytes
            .increment(total_evicted_bytes as u64);
        self.stats
            .object_store_cache_evicted_keys
            .increment(deleted_targets.len() as u64);
        self.stats.object_store_cache_keys.set(entry_count as i64);
        self.stats
            .object_store_cache_bytes
            .set(self.cache_size_bytes.load(Ordering::Relaxed) as i64);

        total_evicted_bytes
    }

    async fn delete_entry(&self, path: std::path::PathBuf) {
        let deleted_entries = delete_cache_entry(path, self.file_handle_cache.clone()).await;
        if deleted_entries.is_empty() {
            return;
        }

        let _track_guard = self.track_lock.lock().await;

        // Untrack the deleted entries.
        let entry_count = {
            let mut cache_state = self.cache_state.lock().await;
            for deleted_entry in deleted_entries {
                if let Some(removed) = cache_state.remove_entry(&deleted_entry) {
                    self.cache_size_bytes
                        .fetch_sub(removed.size_bytes as u64, Ordering::SeqCst);
                }
            }
            cache_state.entries.len()
        };

        // Update the stats.
        self.stats.object_store_cache_keys.set(entry_count as i64);
        self.stats
            .object_store_cache_bytes
            .set(self.cache_size_bytes.load(Ordering::Relaxed) as i64);
    }

    /// Pick multiple eviction targets in a single pass using pick-of-2 strategy, which is an approximation
    //  of LRU, it randomized pick two files, compare their last access time, and choose the older one to evict.
    ///
    /// Returns a list of (path, size_bytes) tuples to evict.
    async fn pick_evict_targets(&self, target_size: u64) -> Vec<(std::path::PathBuf, usize)> {
        let cache_state = self.cache_state.lock().await;

        if cache_state.keys.len() < 2 {
            return vec![];
        }

        let mut targets = Vec::new();
        // Track the simulated cache size during eviction but do not modify the actual cache size until
        // after files are deleted.
        let mut simulated_size = self.cache_size_bytes.load(Ordering::Relaxed);
        // Track which indices have been selected for eviction
        let mut picked_indices: HashSet<usize> = HashSet::new();

        let mut rng = self.rand.rng();

        while simulated_size > target_size {
            // Need at least 2 non-evicted entries to pick from
            let available_count = cache_state.keys.len() - picked_indices.len();
            if available_count < 2 {
                break;
            }

            // Pick two random indices that haven't been evicted yet
            let idx0 = match self.pick_random_available_index(
                &mut rng,
                &cache_state.keys,
                &picked_indices,
                None,
            ) {
                Some(idx) => idx,
                None => break,
            };
            let idx1 = match self.pick_random_available_index(
                &mut rng,
                &cache_state.keys,
                &picked_indices,
                Some(idx0),
            ) {
                Some(idx) => idx,
                None => break,
            };

            let path0 = &cache_state.keys[idx0];
            let path1 = &cache_state.keys[idx1];

            let entry0 = match cache_state.entries.get(path0) {
                Some(e) => e,
                None => break,
            };
            let entry1 = match cache_state.entries.get(path1) {
                Some(e) => e,
                None => break,
            };

            let (chosen_idx, chosen_path, chosen_bytes) =
                if entry0.access_time <= entry1.access_time {
                    (idx0, path0.clone(), entry0.size_bytes)
                } else {
                    (idx1, path1.clone(), entry1.size_bytes)
                };

            picked_indices.insert(chosen_idx);
            simulated_size = simulated_size.saturating_sub(chosen_bytes as u64);
            targets.push((chosen_path, chosen_bytes));
        }

        targets
    }

    // Pick a random index from keys that hasn't been chosen yet and optionally excludes
    // a specific index. Returns None if no available index exists.
    fn pick_random_available_index(
        &self,
        rng: &mut impl Rng,
        keys: &[std::path::PathBuf],
        picked: &HashSet<usize>,
        exclude_idx: Option<usize>,
    ) -> Option<usize> {
        let excluded_not_picked = exclude_idx.is_some_and(|idx| !picked.contains(&idx));
        let available_count = keys
            .len()
            .saturating_sub(picked.len())
            .saturating_sub(usize::from(excluded_not_picked));

        if available_count == 0 {
            return None;
        }

        loop {
            let idx = rng.random_range(0..keys.len());
            if !picked.contains(&idx) && Some(idx) != exclude_idx {
                return Some(idx);
            }
        }
    }
}

fn wrap_io_err(err: impl std::error::Error + Send + Sync + 'static) -> object_store::Error {
    object_store::Error::Generic {
        store: "cached_object_store",
        source: Box::new(err),
    }
}

async fn delete_cache_entry(
    path: std::path::PathBuf,
    file_handle_cache: FileHandleCache,
) -> Vec<std::path::PathBuf> {
    // Run deletion as a single blocking task rather than jumping back and
    // forth to the same blocking thread pool tokio uses under the hood.
    #[allow(clippy::disallowed_methods)]
    let result = tokio::task::spawn_blocking(move || {
        let result = std::fs::read_dir(&path);
        match result {
            Ok(read_dir) => {
                let mut deleted_entries = Vec::with_capacity(32);

                // Delete the head and the parts.
                for entry in read_dir {
                    match entry {
                        Ok(entry) => {
                            let entry_path = entry.path();
                            let result = std::fs::remove_file(&entry_path);
                            match result {
                                Ok(()) => {
                                    file_handle_cache.invalidate(&entry_path);
                                    deleted_entries.push(entry_path);
                                }
                                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                                    file_handle_cache.invalidate(&entry_path);
                                    deleted_entries.push(entry_path);
                                }
                                Err(e) => {
                                    file_handle_cache.invalidate(&entry_path);
                                    error!("FS cache failed to delete {entry_path:?}: {e:?}");
                                }
                            }
                        }
                        Err(e) => {
                            error!("FS cache failed to iterate read_dir {path:?}: {e:?}");
                            continue;
                        }
                    }
                }

                // Delete the entry directory itself.
                let result = std::fs::remove_dir(&path);
                if let Err(e) = result {
                    if e.kind() != std::io::ErrorKind::NotFound {
                        error!("FS cache failed to delete {path:?}: {e:?}");
                    }
                }

                deleted_entries
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => vec![],
            Err(e) => {
                error!("FS cache failed to read_dir {path:?}: {e:?}");
                vec![]
            }
        }
    })
    .await;
    result.unwrap_or_else(|e| {
        error!("FS cache failed to join deletion task: {e:?}");
        vec![]
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cached_object_store::stats::{CACHE_BYTES, CACHE_KEYS, EVICTED_BYTES, EVICTED_KEYS};
    use crate::test_utils::gen_rand_bytes;
    use filetime::FileTime;
    use slatedb_common::clock::DefaultSystemClock;
    use slatedb_common::metrics::{lookup_metric, DefaultMetricsRecorder, MetricsRecorderHelper};
    use std::{io::Write, sync::atomic::Ordering, time::SystemTime};

    fn gen_rand_file(
        folder_path: &std::path::Path,
        file_name: &str,
        n: usize,
    ) -> std::path::PathBuf {
        let file_path = folder_path.join(file_name);
        let bytes = gen_rand_bytes(n);
        let mut file = std::fs::File::create(&file_path).unwrap();
        file.write_all(&bytes).unwrap();
        file_path
    }

    #[tokio::test]
    async fn test_evictor() {
        let temp_dir = tempfile::Builder::new()
            .prefix("objstore_cache_test_evictor_")
            .tempdir()
            .unwrap();
        let recorder = MetricsRecorderHelper::noop();

        let evictor = FsCacheEvictorInner::new(
            temp_dir.path().to_path_buf(),
            1024 * 2,
            Arc::new(CachedObjectStoreStats::new(&recorder)),
            Arc::new(DbRand::default()),
            FileHandleCache::new(1000),
        );

        let path0 = gen_rand_file(temp_dir.path(), "file0", 1024);
        let evicted = evictor
            .track_entry_accessed(path0, 1024, DefaultSystemClock::default().now(), true)
            .await;
        assert_eq!(evicted, 0);

        let path1 = gen_rand_file(temp_dir.path(), "file1", 1024);
        let evicted = evictor
            .track_entry_accessed(path1, 1024, DefaultSystemClock::default().now(), true)
            .await;
        assert_eq!(evicted, 0);

        let path2 = gen_rand_file(temp_dir.path(), "file2", 1024);
        let evicted = evictor
            .track_entry_accessed(path2, 1024, DefaultSystemClock::default().now(), true)
            .await;
        assert_eq!(evicted, 2048);

        let file_paths = WalkDir::new(temp_dir.path())
            .into_iter()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().to_string())
            .collect::<Vec<_>>();
        assert_eq!(file_paths.len(), 2); // the folder file "." is also counted
    }

    #[tokio::test]
    async fn test_evictor_track_entry_accessed_backpressure() {
        let temp_dir = tempfile::Builder::new()
            .prefix("objstore_cache_test_evictor_backpressure_")
            .tempdir()
            .unwrap();
        let recorder = MetricsRecorderHelper::noop();

        let evictor = FsCacheEvictor::new(
            temp_dir.path().to_path_buf(),
            1024,
            None,
            Arc::new(CachedObjectStoreStats::new(&recorder)),
            Arc::new(DefaultSystemClock::new()),
            Arc::new(DbRand::default()),
            FileHandleCache::new(1000),
        );

        // Simulate started state without a running receiver so the channel can fill.
        evictor.started.store(true, Ordering::Release);

        for idx in 0..100 {
            let accepted = evictor
                .track_entry_accessed(
                    std::path::PathBuf::from(format!("file{idx}")),
                    EntryAccess::Write(1),
                )
                .await;
            assert!(accepted);
        }

        let accepted = evictor
            .track_entry_accessed(std::path::PathBuf::from("overflow"), EntryAccess::Write(1))
            .await;
        assert!(!accepted);
    }

    #[tokio::test]
    async fn test_evictor_pick() {
        let temp_dir = tempfile::Builder::new()
            .prefix("objstore_cache_test_evictor_")
            .tempdir()
            .unwrap();
        let recorder = MetricsRecorderHelper::noop();
        let evictor = Arc::new(FsCacheEvictorInner::new(
            temp_dir.path().to_path_buf(),
            1024 * 2,
            Arc::new(CachedObjectStoreStats::new(&recorder)),
            Arc::new(DbRand::default()),
            FileHandleCache::new(1000),
        ));

        let path0 = gen_rand_file(temp_dir.path(), "file0", 1024);
        gen_rand_file(temp_dir.path(), "file1", 1025);

        filetime::set_file_atime(&path0, FileTime::from_system_time(SystemTime::UNIX_EPOCH))
            .unwrap();

        evictor.clone().scan_entries(false).await;

        let targets = evictor.pick_evict_targets(1025).await;

        assert_eq!(targets.len(), 1);
        let (target_path, size) = &targets[0];
        assert_eq!(*target_path, path0);
        assert_eq!(*size, 1024);
    }

    #[tokio::test]
    async fn test_evictor_rescan() {
        let temp_dir = tempfile::Builder::new()
            .prefix("objstore_cache_test_evictor_")
            .tempdir()
            .unwrap();
        let recorder = MetricsRecorderHelper::noop();

        let evictor = Arc::new(FsCacheEvictorInner::new(
            temp_dir.path().to_path_buf(),
            1024 * 2,
            Arc::new(CachedObjectStoreStats::new(&recorder)),
            Arc::new(DbRand::default()),
            FileHandleCache::new(1000),
        ));

        gen_rand_file(temp_dir.path(), "file0", 1024);
        gen_rand_file(temp_dir.path(), "file1", 1025);

        // rescan two times, the cache size should be 2049 unchanged
        evictor.clone().scan_entries(false).await;
        let cache_size_bytes = evictor.cache_size_bytes.load(Ordering::SeqCst);
        assert_eq!(cache_size_bytes, 2049);
        evictor.clone().scan_entries(false).await;
        let cache_size_bytes = evictor.cache_size_bytes.load(Ordering::SeqCst);
        assert_eq!(cache_size_bytes, 2049);
    }

    #[rstest::rstest]
    // Basic case: 2 keys, nothing picked, no exclusion
    #[case(&[0, 1], &[], None, &[0, 1])]
    // Basic case: 2 keys, nothing picked, exclude index 0
    #[case(&[0, 1], &[], Some(0), &[1])]
    // Basic case: 2 keys, nothing picked, exclude index 1
    #[case(&[0, 1], &[], Some(1), &[0])]
    // 3 keys, index 0 picked, no exclusion -> can return 1 or 2
    #[case(&[0, 1, 2], &[0], None, &[1, 2])]
    // 3 keys, index 0 picked, exclude index 1 -> must return 2
    #[case(&[0, 1, 2], &[0], Some(1), &[2])]
    // 4 keys, indices 0,1 picked, no exclusion -> can return 2 or 3
    #[case(&[0, 1, 2, 3], &[0, 1], None, &[2, 3])]
    // 4 keys, indices 0,1 picked, exclude 2 -> must return 3
    #[case(&[0, 1, 2, 3], &[0, 1], Some(2), &[3])]
    // Corner case: exclude_idx is already in picked (redundant exclusion) -
    // index 0 is both picked and excluded
    #[case(&[0, 1], &[0], Some(0), &[1])]
    // Corner case: no available index (all picked or excluded)
    #[case(&[0, 1], &[0], Some(1), &[])]
    fn test_pick_random_available_index(
        #[case] key_indices: &[usize],
        #[case] picked_indices: &[usize],
        #[case] exclude_idx: Option<usize>,
        #[case] expected_possible: &[usize],
    ) {
        let temp_dir = tempfile::Builder::new()
            .prefix("objstore_cache_test_pick_")
            .tempdir()
            .unwrap();
        let recorder = MetricsRecorderHelper::noop();
        let evictor = FsCacheEvictorInner::new(
            temp_dir.path().to_path_buf(),
            1024,
            Arc::new(CachedObjectStoreStats::new(&recorder)),
            Arc::new(DbRand::default()),
            FileHandleCache::new(1000),
        );

        let keys: Vec<std::path::PathBuf> = key_indices
            .iter()
            .map(|i| std::path::PathBuf::from(format!("file{}", i)))
            .collect();
        let picked: HashSet<usize> = picked_indices.iter().copied().collect();

        let mut rng = evictor.rand.rng();

        if expected_possible.is_empty() {
            // Should return None when no available index exists
            let result = evictor.pick_random_available_index(&mut rng, &keys, &picked, exclude_idx);
            assert!(
                result.is_none(),
                "pick_random_available_index should return None, got {:?}",
                result
            );
        } else {
            for _ in 0..100 {
                let result =
                    evictor.pick_random_available_index(&mut rng, &keys, &picked, exclude_idx);
                assert!(
                    result.is_some_and(|r| expected_possible.contains(&r)),
                    "pick_random_available_index returned {:?}, expected one of {:?}",
                    result,
                    expected_possible
                );
            }
        }
    }

    #[tokio::test]
    async fn test_should_record_cache_and_eviction_metrics() {
        // given:
        let temp_dir = tempfile::Builder::new()
            .prefix("objstore_cache_test_metrics_")
            .tempdir()
            .unwrap();
        let recorder = Arc::new(DefaultMetricsRecorder::new());
        let helper = MetricsRecorderHelper::new(recorder.clone(), Default::default());

        let evictor = FsCacheEvictorInner::new(
            temp_dir.path().to_path_buf(),
            1024 * 2,
            Arc::new(CachedObjectStoreStats::new(&helper)),
            Arc::new(DbRand::default()),
            FileHandleCache::new(1000),
        );

        // when: add two entries (within capacity)
        let path0 = gen_rand_file(temp_dir.path(), "file0", 1024);
        evictor
            .track_entry_accessed(path0, 1024, DefaultSystemClock::default().now(), true)
            .await;

        let path1 = gen_rand_file(temp_dir.path(), "file1", 1024);
        evictor
            .track_entry_accessed(path1, 1024, DefaultSystemClock::default().now(), true)
            .await;

        // then: cache_keys and cache_bytes are set
        assert_eq!(lookup_metric(&recorder, CACHE_KEYS), Some(2));
        assert_eq!(lookup_metric(&recorder, CACHE_BYTES), Some(2048));

        // when: add a third entry that triggers eviction
        let path2 = gen_rand_file(temp_dir.path(), "file2", 1024);
        evictor
            .track_entry_accessed(path2, 1024, DefaultSystemClock::default().now(), true)
            .await;

        // then: evicted_keys and evicted_bytes are recorded
        let evicted_keys = lookup_metric(&recorder, EVICTED_KEYS).unwrap();
        let evicted_bytes = lookup_metric(&recorder, EVICTED_BYTES).unwrap();
        assert!(
            evicted_keys >= 1,
            "expected evicted_keys >= 1, got {evicted_keys}"
        );
        assert!(
            evicted_bytes >= 1024,
            "expected evicted_bytes >= 1024, got {evicted_bytes}"
        );

        // cache_keys should be updated after eviction
        let keys = lookup_metric(&recorder, CACHE_KEYS).unwrap();
        assert!(keys >= 1, "expected cache_keys >= 1, got {keys}");
    }
}
