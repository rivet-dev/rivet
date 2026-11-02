// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.  See the NOTICE file
// distributed with this work for additional information
// regarding copyright ownership.  The ASF licenses this file
// to you under the Apache License, Version 2.0 (the
// "License"); you may not use this file except in compliance
// with the License.  You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing,
// software distributed under the License is distributed on an
// "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied.  See the License for the
// specific language governing permissions and limitations
// under the License.

//! An object store implementation for a local filesystem
use std::fs::{File, Metadata, OpenOptions, metadata, symlink_metadata};
use std::io::{ErrorKind, Read, Seek, SeekFrom, Write};
use std::ops::Range;
#[cfg(target_family = "unix")]
use std::os::unix::fs::FileExt;
#[cfg(target_family = "windows")]
use std::os::windows::fs::FileExt;
use std::sync::Arc;
use std::time::SystemTime;
use std::{collections::BTreeSet, io};
use std::{collections::VecDeque, path::PathBuf};

use async_trait::async_trait;
use bytes::Bytes;
use chrono::{DateTime, Utc};
use futures_util::{FutureExt, TryStreamExt};
use futures_util::{StreamExt, stream::BoxStream};
use parking_lot::Mutex;
use url::Url;
use walkdir::{DirEntry, WalkDir};

use crate::{
    Attributes, GetOptions, GetResult, GetResultPayload, ListResult, MultipartUpload, ObjectMeta,
    ObjectStore, PutMode, PutMultipartOptions, PutOptions, PutPayload, PutResult, Result,
    UploadPart, maybe_spawn_blocking,
    path::{Path, absolute_path_to_url},
    util::{InvalidGetRange, merge_ranges},
};
use crate::{CopyMode, CopyOptions, RenameOptions, RenameTargetMode};

/// A specialized `Error` for filesystem object store-related errors
#[derive(Debug, thiserror::Error)]
pub(crate) enum Error {
    #[error("Unable to walk dir: {}", source)]
    UnableToWalkDir { source: walkdir::Error },

    #[error("Unable to access metadata for {}: {}", path, source)]
    Metadata {
        source: Box<dyn std::error::Error + Send + Sync + 'static>,
        path: String,
    },

    #[error("Unable to copy data to file: {}", source)]
    UnableToCopyDataToFile { source: io::Error },

    #[error("Unable to rename file: {}", source)]
    UnableToRenameFile { source: io::Error },

    #[error("Unable to create dir {}: {}", path.display(), source)]
    UnableToCreateDir { source: io::Error, path: PathBuf },

    #[error("Unable to create file {}: {}", path.display(), source)]
    UnableToCreateFile { source: io::Error, path: PathBuf },

    #[error("Unable to delete file {}: {}", path.display(), source)]
    UnableToDeleteFile { source: io::Error, path: PathBuf },

    #[error("Unable to open file {}: {}", path.display(), source)]
    UnableToOpenFile { source: io::Error, path: PathBuf },

    #[error("Unable to read data from file {}: {}", path.display(), source)]
    UnableToReadBytes { source: io::Error, path: PathBuf },

    #[error("Out of range of file {}, expected: {}, actual: {}", path.display(), expected, actual)]
    OutOfRange {
        path: PathBuf,
        expected: u64,
        actual: u64,
    },

    #[error("Requested range was invalid")]
    InvalidRange { source: InvalidGetRange },

    #[error("Unable to copy file from {} to {}: {}", from.display(), to.display(), source)]
    UnableToCopyFile {
        from: PathBuf,
        to: PathBuf,
        source: io::Error,
    },

    #[error("NotFound")]
    NotFound { path: PathBuf, source: io::Error },

    #[error("Error seeking file {}: {}", path.display(), source)]
    Seek { source: io::Error, path: PathBuf },

    #[error("Unable to convert URL \"{}\" to filesystem path", url)]
    InvalidUrl { url: Url },

    #[error("AlreadyExists")]
    AlreadyExists { path: String, source: io::Error },

    #[error("Unable to canonicalize filesystem root: {}", path.display())]
    UnableToCanonicalize { path: PathBuf, source: io::Error },

    #[error("Filenames containing trailing '/#\\d+/' are not supported: {}", path)]
    InvalidPath { path: String },

    #[error("Unable to sync data to disk for {}: {}", path.display(), source)]
    UnableToSyncFile { source: io::Error, path: PathBuf },

    #[error("Upload aborted")]
    Aborted,
}

impl From<Error> for super::Error {
    fn from(source: Error) -> Self {
        match source {
            Error::NotFound { path, source } => Self::NotFound {
                path: path.to_string_lossy().to_string(),
                source: source.into(),
            },
            Error::AlreadyExists { path, source } => Self::AlreadyExists {
                path,
                source: source.into(),
            },
            _ => Self::Generic {
                store: "LocalFileSystem",
                source: Box::new(source),
            },
        }
    }
}

/// Explicitly close a file, checking for errors that would be silently ignored by Rust's `File::drop()`.
///
/// On network filesystems (e.g. NFS), `close()` can fail and indicate data loss.
fn close_file(file: File) -> std::result::Result<(), io::Error> {
    #[cfg(target_family = "unix")]
    {
        use std::os::fd::IntoRawFd;

        close_fd(file.into_raw_fd())
    }
    #[cfg(target_family = "windows")]
    {
        use std::os::windows::io::IntoRawHandle;

        close_handle(file.into_raw_handle())
    }
    #[cfg(not(any(target_family = "unix", target_family = "windows")))]
    {
        drop(file);
        Ok(())
    }
}

#[cfg(target_family = "unix")]
fn close_fd(fd: std::os::fd::RawFd) -> std::result::Result<(), io::Error> {
    nix::unistd::close(fd).map_err(|e| e.into())
}

#[cfg(target_family = "windows")]
fn close_handle(handle: std::os::windows::io::RawHandle) -> std::result::Result<(), io::Error> {
    // SAFETY: `handle` must be an owned handle, except when testing invalid handle errors.
    match unsafe { windows_sys::Win32::Foundation::CloseHandle(handle) } {
        0 => Err(io::Error::last_os_error()),
        _ => Ok(()),
    }
}

/// Opaque ownership retained through a local blocking filesystem operation.
pub type LocalOperationGuard = Arc<dyn std::fmt::Debug + Send + Sync>;

/// Local filesystem storage providing an [`ObjectStore`] interface to files on
/// local disk. Can optionally be created with a directory prefix
///
/// # Path Semantics
///
/// This implementation follows the [file URI] scheme outlined in [RFC 3986]. In
/// particular paths are delimited by `/`
///
/// [file URI]: https://en.wikipedia.org/wiki/File_URI_scheme
/// [RFC 3986]: https://www.rfc-editor.org/rfc/rfc3986
///
/// # Path Semantics
///
/// [`LocalFileSystem`] will expose the path semantics of the underlying filesystem, which may
/// have additional restrictions beyond those enforced by [`Path`].
///
/// For example:
///
/// * Windows forbids certain filenames, e.g. `COM0`,
/// * Windows forbids folders with trailing `.`
/// * Windows forbids certain ASCII characters, e.g. `<` or `|`
/// * OS X forbids filenames containing `:`
/// * Leading `-` are discouraged on Unix systems where they may be interpreted as CLI flags
/// * Filesystems may have restrictions on the maximum path or path segment length
/// * Filesystem support for non-ASCII characters is inconsistent
///
/// Additionally some filesystems, such as NTFS, are case-insensitive, whilst others like
/// FAT don't preserve case at all. Further some filesystems support non-unicode character
/// sequences, such as unpaired UTF-16 surrogates, and [`LocalFileSystem`] will error on
/// encountering such sequences.
///
/// Finally, filenames matching the regex `/.*#\d+/`, e.g. `foo.parquet#123`, are not supported
/// by [`LocalFileSystem`] as they are used to provide atomic writes. Such files will be ignored
/// for listing operations, and attempting to address such a file will error.
///
/// # Tokio Compatibility
///
/// Tokio discourages performing blocking IO on a tokio worker thread, however,
/// no major operating systems have stable async file APIs. Therefore if called from
/// a tokio context, this will use [`tokio::runtime::Handle::spawn_blocking`] to dispatch
/// IO to a blocking thread pool, much like `tokio::fs` does under-the-hood.
///
/// If not called from a tokio context, this will perform IO on the current thread with
/// no additional complexity or overheads
///
/// # Symlinks
///
/// [`LocalFileSystem`] will follow symlinks as normal, however, it is worth noting:
///
/// * Broken symlinks will be silently ignored by listing operations
/// * No effort is made to prevent breaking symlinks when deleting files
/// * Symlinks that resolve to paths outside the root **will** be followed
/// * Mutating a file through one or more symlinks will mutate the underlying file
/// * Deleting a path that resolves to a symlink will only delete the symlink
///
/// # Cross-Filesystem Copy
///
/// [`LocalFileSystem::copy_opts`] is implemented using [`std::fs::hard_link`], and therefore
/// does not support copying across filesystem boundaries.
///
#[derive(Clone, Debug)]
pub struct LocalFileSystem {
    config: Arc<Config>,
    // if you want to delete empty directories when deleting files
    automatic_cleanup: bool,
    // if true, fsync written files and their parent directories after writes
    fsync: bool,
    operation_guard: Option<LocalOperationGuard>,
}

#[derive(Debug)]
struct Config {
    root: Url,
}

impl std::fmt::Display for LocalFileSystem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "LocalFileSystem({})", self.config.root)
    }
}

impl Default for LocalFileSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl LocalFileSystem {
    /// Create new filesystem storage with no prefix
    pub fn new() -> Self {
        Self {
            config: Arc::new(Config {
                root: Url::parse("file:///").unwrap(),
            }),
            automatic_cleanup: false,
            fsync: false,
            operation_guard: None,
        }
    }

    /// Create new filesystem storage with `prefix` applied to all paths
    ///
    /// Returns an error if the path does not exist
    ///
    pub fn new_with_prefix(prefix: impl AsRef<std::path::Path>) -> Result<Self> {
        let path = std::fs::canonicalize(&prefix).map_err(|source| {
            let path = prefix.as_ref().into();
            Error::UnableToCanonicalize { source, path }
        })?;

        Ok(Self {
            config: Arc::new(Config {
                root: absolute_path_to_url(path)?,
            }),
            automatic_cleanup: false,
            fsync: false,
            operation_guard: None,
        })
    }

    /// Retain external ownership until blocking mutations finish, even if their caller cancels.
    pub fn with_operation_guard(mut self, guard: LocalOperationGuard) -> Self {
        self.operation_guard = Some(guard);
        self
    }

    /// Create a concrete upload whose publication can retain a separate operation guard.
    pub fn create_multipart(
        &self,
        location: &Path,
        opts: PutMultipartOptions,
    ) -> Result<LocalUpload> {
        if !opts.attributes.is_empty() {
            return Err(crate::Error::NotImplemented {
                operation: "`put_multipart_opts` with `opts.attributes` specified".into(),
                implementer: self.to_string(),
            });
        }
        let dest = self.path_to_filesystem(location)?;
        let (file, src) = new_staged_upload(&dest, self.fsync)?;
        let mut upload = LocalUpload::new(src, dest, file, self.fsync);
        upload.lifetime_guard = self.operation_guard.clone();
        Ok(upload)
    }

    /// Return an absolute filesystem path of the given file location
    pub fn path_to_filesystem(&self, location: &Path) -> Result<PathBuf> {
        self.config.path_to_filesystem(location)
    }

    /// Enable automatic cleanup of empty directories when deleting files
    pub fn with_automatic_cleanup(mut self, automatic_cleanup: bool) -> Self {
        self.automatic_cleanup = automatic_cleanup;
        self
    }

    /// Enable `fsync` after writes for durability
    ///
    /// When enabled, [`LocalFileSystem`] calls [`File::sync_all`] on written files and fsyncs
    /// the affected parent directories before a write operation
    /// ([`put_opts`](ObjectStore::put_opts), [`copy_opts`](ObjectStore::copy_opts),
    /// [`rename_opts`](ObjectStore::rename_opts), and multipart upload completion) returns
    /// success. This guarantees that both the file contents and the directory entries pointing
    /// to them are durable on stable storage, matching the implicit durability contract of
    /// remote object stores such as S3 or GCS.
    ///
    /// This trades write throughput for durability and is **disabled by default**.
    ///
    /// Note that directory fsync is only performed on Unix; on other platforms (e.g. Windows)
    /// it is a no-op, as directories cannot be portably opened and synced.
    pub fn with_fsync(mut self, fsync: bool) -> Self {
        self.fsync = fsync;
        self
    }
}

impl Config {
    /// Return an absolute filesystem path of the given location
    fn prefix_to_filesystem(&self, location: &Path) -> Result<PathBuf> {
        let mut url = self.root.clone();
        url.path_segments_mut()
            .expect("url path")
            // technically not necessary as Path ignores empty segments
            // but avoids creating paths with "//" which look odd in error messages.
            .pop_if_empty()
            .extend(location.parts());

        url.to_file_path()
            .map_err(|_| Error::InvalidUrl { url }.into())
    }

    /// Return an absolute filesystem path of the given file location
    fn path_to_filesystem(&self, location: &Path) -> Result<PathBuf> {
        if !is_valid_file_path(location) {
            let path = location.as_ref().into();
            let error = Error::InvalidPath { path };
            return Err(error.into());
        }

        let path = self.prefix_to_filesystem(location)?;

        #[cfg(target_os = "windows")]
        let path = {
            let path = path.to_string_lossy();

            // Assume the first char is the drive letter and the next is a colon.
            let mut out = String::new();
            let drive = &path[..2]; // The drive letter and colon (e.g., "C:")
            let filepath = &path[2..].replace(':', "%3A"); // Replace subsequent colons
            out.push_str(drive);
            out.push_str(filepath);
            PathBuf::from(out)
        };

        Ok(path)
    }

    /// Resolves the provided absolute filesystem path to a [`Path`] prefix
    fn filesystem_to_path(&self, location: &std::path::Path) -> Result<Path> {
        Ok(Path::from_absolute_path_with_base(
            location,
            Some(&self.root),
        )?)
    }
}

fn is_valid_file_path(path: &Path) -> bool {
    match path.filename() {
        Some(p) => match p.split_once('#') {
            Some((_, suffix)) if !suffix.is_empty() => {
                // Valid if contains non-digits
                !suffix.as_bytes().iter().all(|x| x.is_ascii_digit())
            }
            _ => true,
        },
        None => false,
    }
}

#[async_trait]
impl ObjectStore for LocalFileSystem {
    async fn put_opts(
        &self,
        location: &Path,
        payload: PutPayload,
        opts: PutOptions,
    ) -> Result<PutResult> {
        if matches!(opts.mode, PutMode::Update(_)) {
            return Err(crate::Error::NotImplemented {
                operation: "`put_opts` with mode `PutMode::Update`".into(),
                implementer: self.to_string(),
            });
        }

        if !opts.attributes.is_empty() {
            return Err(crate::Error::NotImplemented {
                operation: "`put_opts` with `opts.attributes` specified".into(),
                implementer: self.to_string(),
            });
        }

        let path = self.path_to_filesystem(location)?;
        let fsync = self.fsync;
        let operation_guard = self.operation_guard.clone();
        maybe_spawn_blocking(move || {
            let _operation_guard = operation_guard;
            let (mut file, staging_path) = new_staged_upload(&path, fsync)?;
            let mut e_tag = None;

            let err = match payload.iter().try_for_each(|x| file.write_all(x)) {
                Ok(_) => {
                    let metadata = file.metadata().map_err(|e| Error::Metadata {
                        source: e.into(),
                        path: path.to_string_lossy().to_string(),
                    })?;
                    e_tag = Some(get_etag(&metadata));
                    // Atomically publish the staged file. When fsync is enabled the publish
                    // helpers flush the file's contents and the destination's parent directory to
                    // disk first, so a successful return is durable; the fsync calls are bundled
                    // into the helpers so a file-system modification can never be left unsynced.
                    match opts.mode {
                        PutMode::Overwrite => {
                            finish_staged_rename(file, &staging_path, &path, fsync).err()
                        }
                        PutMode::Create => {
                            finish_staged_hard_link(file, &staging_path, &path, fsync).err()
                        }
                        PutMode::Update(_) => unreachable!(),
                    }
                }
                Err(source) => Some(Error::UnableToCopyDataToFile { source }.into()),
            };

            if let Some(err) = err {
                let _ = std::fs::remove_file(&staging_path); // Attempt to cleanup
                return Err(err);
            }

            Ok(PutResult {
                e_tag,
                version: None,
                extensions: Default::default(),
            })
        })
        .await
    }

    async fn put_multipart_opts(
        &self,
        location: &Path,
        opts: PutMultipartOptions,
    ) -> Result<Box<dyn MultipartUpload>> {
        Ok(Box::new(self.create_multipart(location, opts)?))
    }

    async fn get_opts(&self, location: &Path, options: GetOptions) -> Result<GetResult> {
        let location = location.clone();
        let path = self.path_to_filesystem(&location)?;
        maybe_spawn_blocking(move || {
            let file = open_file(&path)?;
            let metadata = open_metadata(&file, &path)?;
            let meta = convert_metadata(metadata, location);
            options.check_preconditions(&meta)?;

            let range = match options.range {
                Some(r) => r
                    .as_range(meta.size)
                    .map_err(|source| Error::InvalidRange { source })?,
                None => 0..meta.size,
            };

            Ok(GetResult {
                payload: GetResultPayload::File(file, path),
                attributes: Attributes::default(),
                range,
                meta,
                extensions: Default::default(),
            })
        })
        .await
    }

    async fn get_ranges(&self, location: &Path, ranges: &[Range<u64>]) -> Result<Vec<Bytes>> {
        let path = self.path_to_filesystem(location)?;
        let ranges = ranges.to_vec();
        maybe_spawn_blocking(move || {
            // We do not read the metadata here, but error in `read_range` if necessary
            let mut file = File::open(&path).map_err(|e| map_open_error(e, &path))?;

            // Coalesce only contiguous/overlapping ranges (gap 0), so we never
            // read bytes that weren't requested while still collapsing adjacent
            // ranges into fewer reads.
            let fetch_ranges = merge_ranges(&ranges, 0);

            // Fast path: nothing coalesced, read each range directly in order.
            if fetch_ranges.len() == ranges.len() {
                return ranges
                    .iter()
                    .map(|r| read_range(&mut file, &path, r.clone()))
                    .collect();
            }

            let fetched = fetch_ranges
                .iter()
                .map(|r| read_range(&mut file, &path, r.clone()))
                .collect::<Result<Vec<_>>>()?;

            Ok(ranges
                .iter()
                .map(|range| {
                    let idx = fetch_ranges.partition_point(|v| v.start <= range.start) - 1;
                    let fetch_range = &fetch_ranges[idx];
                    let fetch_bytes = &fetched[idx];

                    let start = (range.start - fetch_range.start) as usize;
                    let end = (range.end - fetch_range.start) as usize;
                    fetch_bytes.slice(start..end.min(fetch_bytes.len()))
                })
                .collect())
        })
        .await
    }

    fn delete_stream(
        &self,
        locations: BoxStream<'static, Result<Path>>,
    ) -> BoxStream<'static, Result<Path>> {
        let config = Arc::clone(&self.config);
        let automatic_cleanup = self.automatic_cleanup;
        let operation_guard = self.operation_guard.clone();
        locations
            .map(move |location| {
                let config = Arc::clone(&config);
                let operation_guard = operation_guard.clone();
                maybe_spawn_blocking(move || {
                    let _operation_guard = operation_guard;
                    let location = location?;
                    // `with_fsync` does not apply to standalone deletes; only create-mode rename
                    // fsyncs its internal source removal as part of the durable copy-and-delete.
                    Self::delete_location(config, automatic_cleanup, &location, false)?;
                    Ok(location)
                })
            })
            .buffered(10)
            .boxed()
    }

    fn list(&self, prefix: Option<&Path>) -> BoxStream<'static, Result<ObjectMeta>> {
        Self::list_with_maybe_offset(Arc::clone(&self.config), prefix, None)
    }

    fn list_with_offset(
        &self,
        prefix: Option<&Path>,
        offset: &Path,
    ) -> BoxStream<'static, Result<ObjectMeta>> {
        Self::list_with_maybe_offset(Arc::clone(&self.config), prefix, Some(offset))
    }

    async fn list_with_delimiter(&self, prefix: Option<&Path>) -> Result<ListResult> {
        let config = Arc::clone(&self.config);

        let prefix = prefix.cloned().unwrap_or_default();
        let resolved_prefix = config.prefix_to_filesystem(&prefix)?;

        maybe_spawn_blocking(move || {
            let walkdir = WalkDir::new(&resolved_prefix)
                .min_depth(1)
                .max_depth(1)
                .follow_links(true);

            let mut common_prefixes = BTreeSet::new();
            let mut objects = Vec::new();

            for entry_res in walkdir.into_iter().map(convert_walkdir_result) {
                if let Some(entry) = entry_res? {
                    let is_directory = entry.file_type().is_dir();
                    let entry_location = config.filesystem_to_path(entry.path())?;
                    if !is_directory && !is_valid_file_path(&entry_location) {
                        continue;
                    }

                    let mut parts = match entry_location.prefix_match(&prefix) {
                        Some(parts) => parts,
                        None => continue,
                    };

                    let common_prefix = match parts.next() {
                        Some(p) => p,
                        None => continue,
                    };

                    drop(parts);

                    if is_directory {
                        common_prefixes.insert(prefix.clone().join(common_prefix));
                    } else if let Some(metadata) = convert_entry(entry, entry_location)? {
                        objects.push(metadata);
                    }
                }
            }

            Ok(ListResult {
                common_prefixes: common_prefixes.into_iter().collect(),
                objects,
                extensions: Default::default(),
            })
        })
        .await
    }

    async fn copy_opts(&self, from: &Path, to: &Path, options: CopyOptions) -> Result<()> {
        let CopyOptions {
            mode,
            extensions: _,
        } = options;

        let from = self.path_to_filesystem(from)?;
        let to = self.path_to_filesystem(to)?;
        let fsync = self.fsync;
        let operation_guard = self.operation_guard.clone();

        match mode {
            CopyMode::Overwrite => {
                let mut id = 0;
                // In order to make this atomic we:
                //
                // - hard link to a hidden temporary file
                // - atomically rename this temporary file into place
                //
                // This is necessary because hard_link returns an error if the destination already exists
                maybe_spawn_blocking(move || {
                    let _operation_guard = operation_guard;
                    loop {
                        let staged = staged_upload_path(&to, &id.to_string());
                        // Stage via a temporary hard link; the source is already durable so the
                        // staging link itself needs no fsync (the publish rename below fsyncs the
                        // shared parent directory).
                        match std::fs::hard_link(&from, &staged) {
                            // `rename` bundles in the fsync of `to`'s parent directory.
                            Ok(_) => match rename(&staged, &to, fsync) {
                                Ok(_) => return Ok(()),
                                Err(source) => {
                                    let _ = std::fs::remove_file(&staged); // Attempt to clean up
                                    return Err(Error::UnableToCopyFile { from, to, source }.into());
                                }
                            },
                            Err(source) => match source.kind() {
                                ErrorKind::AlreadyExists => id += 1,
                                ErrorKind::NotFound => match from.exists() {
                                    true => create_parent_dirs(&to, source, fsync)?,
                                    false => {
                                        return Err(Error::NotFound { path: from, source }.into());
                                    }
                                },
                                _ => {
                                    return Err(Error::UnableToCopyFile { from, to, source }.into());
                                }
                            },
                        }
                    }
                })
                .await
            }
            CopyMode::Create => {
                maybe_spawn_blocking(move || {
                    let _operation_guard = operation_guard;
                    loop {
                        // The source is an existing object that is already durable, so no file
                        // sync is needed; `hard_link` bundles in the fsync of `to`'s parent dir.
                        match hard_link(&from, &to, fsync) {
                            Ok(_) => return Ok(()),
                            Err(source) => match source.kind() {
                                ErrorKind::AlreadyExists => {
                                    return Err(Error::AlreadyExists {
                                        path: to.to_str().unwrap().to_string(),
                                        source,
                                    }
                                    .into());
                                }
                                ErrorKind::NotFound => match from.exists() {
                                    true => create_parent_dirs(&to, source, fsync)?,
                                    false => {
                                        return Err(Error::NotFound { path: from, source }.into());
                                    }
                                },
                                _ => {
                                    return Err(Error::UnableToCopyFile { from, to, source }.into());
                                }
                            },
                        }
                    }
                })
                .await
            }
        }
    }

    async fn rename_opts(&self, from: &Path, to: &Path, options: RenameOptions) -> Result<()> {
        let RenameOptions {
            target_mode,
            extensions,
        } = options;

        match target_mode {
            // optimized implementation
            RenameTargetMode::Overwrite => {
                let from = self.path_to_filesystem(from)?;
                let to = self.path_to_filesystem(to)?;
                let fsync = self.fsync;
                let operation_guard = self.operation_guard.clone();
                maybe_spawn_blocking(move || {
                    let _operation_guard = operation_guard;
                    loop {
                        // Unlike multipart `complete`, there is no freshly written file to
                        // `sync_all` here: `from` is an existing, already-durable object and a
                        // rename only mutates directory entries. `rename` bundles in the fsync of
                        // both affected directories (destination, and source if it differs).
                        match rename(&from, &to, fsync) {
                            Ok(_) => return Ok(()),
                            Err(source) => match source.kind() {
                                ErrorKind::NotFound => match from.exists() {
                                    true => create_parent_dirs(&to, source, fsync)?,
                                    false => {
                                        return Err(Error::NotFound { path: from, source }.into());
                                    }
                                },
                                _ => {
                                    return Err(Error::UnableToCopyFile { from, to, source }.into());
                                }
                            },
                        }
                    }
                })
                .await
            }
            // fall-back to copy & delete
            RenameTargetMode::Create => {
                self.copy_opts(
                    from,
                    to,
                    CopyOptions {
                        mode: CopyMode::Create,
                        extensions,
                    },
                )
                .await?;
                let config = Arc::clone(&self.config);
                let automatic_cleanup = self.automatic_cleanup;
                let fsync = self.fsync;
                let from = from.clone();
                let operation_guard = self.operation_guard.clone();
                maybe_spawn_blocking(move || {
                    let _operation_guard = operation_guard;
                    Self::delete_location(config, automatic_cleanup, &from, fsync)
                })
                .await?;
                Ok(())
            }
        }
    }
}

impl LocalFileSystem {
    fn delete_location(
        config: Arc<Config>,
        automatic_cleanup: bool,
        location: &Path,
        fsync: bool,
    ) -> Result<()> {
        let path = config.path_to_filesystem(location)?;
        if let Err(e) = std::fs::remove_file(&path) {
            Err(match e.kind() {
                ErrorKind::NotFound => Error::NotFound { path, source: e }.into(),
                _ => Error::UnableToDeleteFile { path, source: e }.into(),
            })
        } else {
            if fsync {
                fsync_parent_dir(&path).map_err(|source| Error::UnableToSyncFile {
                    source,
                    path: path.clone(),
                })?;
            }

            if !automatic_cleanup {
                return Ok(());
            }

            let root = &config.root;
            let root = root
                .to_file_path()
                .map_err(|_| Error::InvalidUrl { url: root.clone() })?;

            // here we will try to traverse up and delete an empty dir if possible until we reach the root or get an error
            let mut parent = path.parent();

            while let Some(loc) = parent {
                if loc != root && std::fs::remove_dir(loc).is_ok() {
                    parent = loc.parent();
                } else {
                    break;
                }
            }

            Ok(())
        }
    }

    fn list_with_maybe_offset(
        config: Arc<Config>,
        prefix: Option<&Path>,
        maybe_offset: Option<&Path>,
    ) -> BoxStream<'static, Result<ObjectMeta>> {
        let root_path = match prefix {
            Some(prefix) => match config.prefix_to_filesystem(prefix) {
                Ok(path) => path,
                Err(e) => return futures_util::future::ready(Err(e)).into_stream().boxed(),
            },
            None => config.root.to_file_path().unwrap(),
        };

        let walkdir = WalkDir::new(root_path)
            // Don't include the root directory itself
            .min_depth(1)
            .follow_links(true);

        let maybe_offset = maybe_offset.cloned();

        let s = walkdir.into_iter().flat_map(move |result_dir_entry| {
            // Apply offset filter before proceeding, to reduce statx file system calls
            // This matters for NFS mounts
            if let (Some(offset), Ok(entry)) = (maybe_offset.as_ref(), result_dir_entry.as_ref()) {
                let location = config.filesystem_to_path(entry.path());
                match location {
                    Ok(path) if path <= *offset => return None,
                    Err(e) => return Some(Err(e)),
                    _ => {}
                }
            }

            let entry = match convert_walkdir_result(result_dir_entry).transpose()? {
                Ok(entry) => entry,
                Err(e) => return Some(Err(e)),
            };

            if !entry.path().is_file() {
                return None;
            }

            match config.filesystem_to_path(entry.path()) {
                Ok(path) => match is_valid_file_path(&path) {
                    true => convert_entry(entry, path).transpose(),
                    false => None,
                },
                Err(e) => Some(Err(e)),
            }
        });

        // If no tokio context, return iterator directly as no
        // need to perform chunked spawn_blocking reads
        if tokio::runtime::Handle::try_current().is_err() {
            return futures_util::stream::iter(s).boxed();
        }

        // Otherwise list in batches of CHUNK_SIZE
        const CHUNK_SIZE: usize = 1024;

        let buffer = VecDeque::with_capacity(CHUNK_SIZE);
        futures_util::stream::try_unfold((s, buffer), |(mut s, mut buffer)| async move {
            if buffer.is_empty() {
                (s, buffer) = tokio::task::spawn_blocking(move || {
                    for _ in 0..CHUNK_SIZE {
                        match s.next() {
                            Some(r) => buffer.push_back(r),
                            None => break,
                        }
                    }
                    (s, buffer)
                })
                .await?;
            }

            match buffer.pop_front() {
                Some(Err(e)) => Err(e),
                Some(Ok(meta)) => Ok(Some((meta, (s, buffer)))),
                None => Ok(None),
            }
        })
        .boxed()
    }
}

/// Creates the parent directories of `path` or returns an error based on `source` if no parent
///
/// When `fsync` is true, every directory created here is fsynced, up to and including the first
/// pre-existing ancestor (whose entry list also changed), so the new directory entries are durable.
fn create_parent_dirs(path: &std::path::Path, source: io::Error, fsync: bool) -> Result<()> {
    let parent = path.parent().ok_or_else(|| {
        let path = path.to_path_buf();
        Error::UnableToCreateFile { path, source }
    })?;

    // Record the deepest already-existing ancestor *before* creating any directories, so that
    // afterwards we know exactly which directories are new and need to be fsynced.
    let first_existing = fsync.then(|| {
        let mut dir = parent;
        while !dir.exists() {
            match dir.parent() {
                Some(p) => dir = p,
                None => break,
            }
        }
        dir.to_path_buf()
    });

    std::fs::create_dir_all(parent).map_err(|source| {
        let path = parent.into();
        Error::UnableToCreateDir { source, path }
    })?;

    if let Some(first_existing) = first_existing {
        // Walk from `parent` up to `first_existing`, fsyncing each directory whose entries changed.
        let mut dir = parent;
        loop {
            fsync_dir(dir).map_err(|source| Error::UnableToSyncFile {
                source,
                path: dir.into(),
            })?;
            if dir == first_existing {
                break;
            }
            dir = match dir.parent() {
                Some(p) => p,
                None => break,
            };
        }
    }
    Ok(())
}

/// Renames `from` to `to`, then — when `fsync` is enabled — fsyncs the destination's parent
/// directory (and the source's too, if it differs) so the moved directory entries are durable.
///
/// The directory fsync is bundled in deliberately: every durable rename goes through here, so the
/// post-rename fsync can never be forgotten at an individual call site.
fn rename(from: &std::path::Path, to: &std::path::Path, fsync: bool) -> io::Result<()> {
    std::fs::rename(from, to)?;
    if fsync {
        fsync_parent_dir(to)?;
        // A cross-directory move also removes an entry from the source directory.
        if from.parent() != to.parent() {
            fsync_parent_dir(from)?;
        }
    }
    Ok(())
}

/// Hard-links `original` to `link`, then — when `fsync` is enabled — fsyncs `link`'s parent
/// directory so the new directory entry is durable.
///
/// As with [`rename`], the directory fsync is bundled in so it cannot be forgotten at a call site.
fn hard_link(original: &std::path::Path, link: &std::path::Path, fsync: bool) -> io::Result<()> {
    std::fs::hard_link(original, link)?;
    if fsync {
        fsync_parent_dir(link)?;
    }
    Ok(())
}

/// Durably publishes the freshly-written staging file `file` (located at `src`) to `dest` via a
/// rename.
///
/// When `fsync` is enabled, the file's contents are flushed before — and `dest`'s parent
/// directory after — the rename, so a successful return is durable. The file is always closed
/// before the rename (checking for close errors): required for NFS error detection and for some
/// FUSE filesystems (e.g. Blobfuse) that only commit the data on close.
fn finish_staged_rename(
    file: File,
    src: &std::path::Path,
    dest: &std::path::Path,
    fsync: bool,
) -> Result<()> {
    sync_and_close(file, src, fsync)?;
    rename(src, dest, fsync).map_err(|source| Error::UnableToRenameFile { source })?;
    Ok(())
}

/// Like [`finish_staged_rename`] but publishes via a hard link (`PutMode::Create` semantics): the
/// staging file is linked to `dest` and then removed. Returns [`Error::AlreadyExists`] if `dest`
/// already exists.
fn finish_staged_hard_link(
    file: File,
    src: &std::path::Path,
    dest: &std::path::Path,
    fsync: bool,
) -> Result<()> {
    sync_and_close(file, src, fsync)?;
    match hard_link(src, dest, fsync) {
        Ok(()) => {
            let _ = std::fs::remove_file(src); // Attempt to cleanup
            Ok(())
        }
        Err(source) => match source.kind() {
            ErrorKind::AlreadyExists => Err(Error::AlreadyExists {
                path: dest.to_str().unwrap().to_string(),
                source,
            }
            .into()),
            _ => Err(Error::UnableToRenameFile { source }.into()),
        },
    }
}

/// Flushes the freshly-written `file`'s contents to disk (when `fsync` is enabled) and then
/// closes it, checking for close errors that dropping the [`File`] would silently ignore.
fn sync_and_close(file: File, path: &std::path::Path, fsync: bool) -> Result<()> {
    if fsync {
        file.sync_all().map_err(|source| Error::UnableToSyncFile {
            source,
            path: path.into(),
        })?;
    }
    close_file(file).map_err(|source| Error::UnableToCopyDataToFile { source })?;
    Ok(())
}

/// Fsyncs the parent directory of `path` so a change to its directory entries (e.g. one just made
/// by a rename or hard link) is durable. A no-op when `path` has no parent.
fn fsync_parent_dir(path: &std::path::Path) -> io::Result<()> {
    match path.parent() {
        Some(parent) => fsync_dir(parent),
        None => Ok(()),
    }
}

/// Fsyncs `dir_path` so that changes to its directory entries are durable.
///
/// This is only meaningful on Unix; on other platforms (e.g. Windows) directories cannot be
/// portably opened as a [`File`] and synced, so this is a no-op.
fn fsync_dir(dir_path: &std::path::Path) -> io::Result<()> {
    #[cfg(target_family = "unix")]
    {
        File::open(dir_path)?.sync_all()
    }
    #[cfg(not(target_family = "unix"))]
    {
        let _ = dir_path;
        Ok(())
    }
}

/// Generates a unique file path `{base}#{suffix}`, returning the opened `File` and `path`
///
/// Creates any directories if necessary, fsyncing them when `fsync` is enabled
fn new_staged_upload(base: &std::path::Path, fsync: bool) -> Result<(File, PathBuf)> {
    let mut multipart_id = 1;
    loop {
        let suffix = multipart_id.to_string();
        let path = staged_upload_path(base, &suffix);
        let mut options = OpenOptions::new();
        match options.read(true).write(true).create_new(true).open(&path) {
            Ok(f) => return Ok((f, path)),
            Err(source) => match source.kind() {
                ErrorKind::AlreadyExists => multipart_id += 1,
                ErrorKind::NotFound => create_parent_dirs(&path, source, fsync)?,
                _ => return Err(Error::UnableToOpenFile { source, path }.into()),
            },
        }
    }
}

/// Returns the unique upload for the given path and suffix
fn staged_upload_path(dest: &std::path::Path, suffix: &str) -> PathBuf {
    let mut staging_path = dest.as_os_str().to_owned();
    staging_path.push("#");
    staging_path.push(suffix);
    staging_path.into()
}

/// A local multipart upload with ownership retained by its blocking operations.
#[derive(Debug)]
pub struct LocalUpload {
    /// The upload state
    state: Arc<UploadState>,
    /// The location of the temporary file
    src: Option<PathBuf>,
    /// The next offset to write into the file
    offset: u64,
    /// Whether to fsync the file and its parent directory on completion
    fsync: bool,
    lifetime_guard: Option<LocalOperationGuard>,
}

#[derive(Debug)]
struct UploadState {
    dest: PathBuf,
    file: Mutex<Option<File>>,
}

impl LocalUpload {
    pub(crate) fn new(src: PathBuf, dest: PathBuf, file: File, fsync: bool) -> Self {
        Self {
            state: Arc::new(UploadState {
                dest,
                file: Mutex::new(Some(file)),
            }),
            src: Some(src),
            offset: 0,
            fsync,
            lifetime_guard: None,
        }
    }
}

impl LocalUpload {
    /// Write one part while retaining ownership through the actual file write.
    pub fn put_part_with_guard(
        &mut self,
        data: PutPayload,
        guard: Option<LocalOperationGuard>,
    ) -> UploadPart {
        let offset = self.offset;
        self.offset += data.content_length() as u64;

        let s = Arc::clone(&self.state);
        let lifetime_guard = self.lifetime_guard.clone();
        maybe_spawn_blocking(move || {
            let _ownership = (lifetime_guard, guard);
            let mut guard = s.file.lock();
            let file = guard.as_mut().ok_or(Error::Aborted)?;
            file.seek(SeekFrom::Start(offset)).map_err(|source| {
                let path = s.dest.clone();
                Error::Seek { source, path }
            })?;

            data.iter()
                .try_for_each(|x| file.write_all(x))
                .map_err(|source| Error::UnableToCopyDataToFile { source })?;

            Ok(())
        })
        .boxed()
    }

    /// Publish while retaining ownership through the final rename and fsync.
    pub async fn complete_with_guard(
        &mut self,
        guard: Option<LocalOperationGuard>,
    ) -> Result<PutResult> {
        let src = self.src.take().ok_or(Error::Aborted)?;
        let s = Arc::clone(&self.state);
        let fsync = self.fsync;
        let lifetime_guard = self.lifetime_guard.clone();
        maybe_spawn_blocking(move || {
            let _ownership = (lifetime_guard, guard);
            // Ensure no inflight writes
            let mut guard = s.file.lock();
            let file = guard.take().ok_or(Error::Aborted)?;

            let metadata = file.metadata().map_err(|e| Error::Metadata {
                source: e.into(),
                path: src.to_string_lossy().to_string(),
            })?;

            // Durably publish the freshly-written staging file: flush its contents, close it, then
            // rename it into place and fsync the destination's parent directory (the fsync calls
            // are bundled into the helper and only run when fsync is enabled).
            finish_staged_rename(file, &src, &s.dest, fsync)?;

            Ok(PutResult {
                e_tag: Some(get_etag(&metadata)),
                version: None,
                extensions: Default::default(),
            })
        })
        .await
    }

    /// Remove the staging file while retaining ownership through the actual unlink.
    pub async fn abort_with_guard(
        &mut self,
        guard: Option<LocalOperationGuard>,
    ) -> Result<()> {
        let src = self.src.take().ok_or(Error::Aborted)?;
        let lifetime_guard = self.lifetime_guard.clone();
        maybe_spawn_blocking(move || {
            let _ownership = (lifetime_guard, guard);
            std::fs::remove_file(&src)
                .map_err(|source| Error::UnableToDeleteFile { source, path: src })?;
            Ok(())
        })
        .await
    }
}

#[async_trait]
impl MultipartUpload for LocalUpload {
    fn put_part(&mut self, data: PutPayload) -> UploadPart {
        self.put_part_with_guard(data, None)
    }
    async fn complete(&mut self) -> Result<PutResult> {
        self.complete_with_guard(None).await
    }
    async fn abort(&mut self) -> Result<()> {
        self.abort_with_guard(None).await
    }
}

impl Drop for LocalUpload {
    fn drop(&mut self) {
        if let Some(src) = self.src.take() {
            // Try to clean up intermediate file ignoring any error
            match tokio::runtime::Handle::try_current() {
                Ok(r) => {
                    let guard = self.lifetime_guard.clone();
                    drop(r.spawn_blocking(move || {
                        let _guard = guard;
                        std::fs::remove_file(src)
                    }));
                }
                Err(_) => drop(std::fs::remove_file(src)),
            };
        }
    }
}

pub(crate) fn chunked_stream(
    mut file: File,
    path: PathBuf,
    range: Range<u64>,
    chunk_size: usize,
) -> BoxStream<'static, Result<Bytes, super::Error>> {
    futures_util::stream::once(async move {
        let requested = range.end - range.start;

        let (file, path) = maybe_spawn_blocking(move || {
            file.seek(SeekFrom::Start(range.start as _))
                .map_err(|err| map_seek_error(err, &file, &path, range.start))?;
            Ok((file, path))
        })
        .await?;

        let stream = futures_util::stream::try_unfold(
            (file, path, requested),
            move |(mut file, path, remaining)| {
                maybe_spawn_blocking(move || {
                    if remaining == 0 {
                        return Ok(None);
                    }

                    let to_read = remaining.min(chunk_size as u64);
                    let cap = usize::try_from(to_read).map_err(|_e| Error::InvalidRange {
                        source: InvalidGetRange::TooLarge {
                            requested: to_read,
                            max: usize::MAX as u64,
                        },
                    })?;
                    let mut buffer = Vec::with_capacity(cap);
                    let read = (&mut file)
                        .take(to_read)
                        .read_to_end(&mut buffer)
                        .map_err(|e| Error::UnableToReadBytes {
                            source: e,
                            path: path.clone(),
                        })?;

                    Ok(Some((buffer.into(), (file, path, remaining - read as u64))))
                })
            },
        );
        Ok::<_, super::Error>(stream)
    })
    .try_flatten()
    .boxed()
}

pub(crate) fn read_range(
    file: &mut File,
    path: &std::path::Path,
    range: Range<u64>,
) -> Result<Bytes> {
    let requested = range.end - range.start;

    let mut buf = Vec::with_capacity(requested as usize);

    #[cfg(any(target_family = "unix", target_family = "windows"))]
    {
        buf.resize(requested as usize, 0_u8);

        let mut buf_slice = &mut buf[..];
        let mut offset = range.start;

        while !buf_slice.is_empty() {
            #[cfg(target_family = "unix")]
            let read_result = file.read_at(buf_slice, offset);

            #[cfg(target_family = "windows")]
            let read_result = file.seek_read(buf_slice, offset);

            match read_result {
                Ok(0) => break,
                Ok(n) => {
                    let tmp = buf_slice;
                    buf_slice = &mut tmp[n..];
                    offset += n as u64;
                }
                // This error is recoverable
                Err(e) if e.kind() == ErrorKind::Interrupted => {}
                Err(source) => {
                    let error = Error::UnableToReadBytes {
                        source,
                        path: path.into(),
                    };

                    return Err(error.into());
                }
            }
        }

        // If we reached EOF before filling the buffer
        if !buf_slice.is_empty() {
            let metadata = open_metadata(file, path)?;
            let file_len = metadata.len();

            // If none of the range is satisfiable we should error, e.g. if the start offset is beyond the
            // extents of the file, or if its at the end of the file and wants to read a non-empty range.
            // if range.start > file_len || (range.start == file_len && !range.is_empty()) {
            if range.start >= file_len {
                return Err(Error::InvalidRange {
                    source: InvalidGetRange::StartTooLarge {
                        requested: range.start,
                        length: file_len,
                    },
                }
                .into());
            }

            let expected = range.end.min(file_len) - range.start;

            let error = Error::OutOfRange {
                path: path.into(),
                expected,
                actual: offset - range.start,
            };

            return Err(error.into());
        }
    }
    #[cfg(all(not(windows), not(unix)))]
    {
        file.seek(SeekFrom::Start(range.start))
            .map_err(|err| map_seek_error(err, file, path, range.start))?;

        let read = file.take(requested).read_to_end(&mut buf).map_err(|err| {
            // try to read metadata to give a better error in case of directory
            if let Err(e) = open_metadata(file, path) {
                return e;
            }
            Error::UnableToReadBytes {
                source: err,
                path: path.to_path_buf(),
            }
        })? as u64;

        if read != requested {
            let metadata = open_metadata(file, path)?;
            let file_len = metadata.len();

            if range.start >= file_len {
                return Err(Error::InvalidRange {
                    source: InvalidGetRange::StartTooLarge {
                        requested: range.start,
                        length: file_len,
                    },
                }
                .into());
            }

            let expected = range.end.min(file_len) - range.start;
            if read != expected {
                return Err(Error::OutOfRange {
                    path: path.to_path_buf(),
                    expected,
                    actual: read,
                }
                .into());
            }
        }
    }

    Ok(buf.into())
}

fn open_file(path: &std::path::Path) -> Result<File, Error> {
    File::open(path).map_err(|e| map_open_error(e, path))
}

fn open_metadata(file: &File, path: &std::path::Path) -> Result<Metadata, Error> {
    let metadata = file.metadata().map_err(|e| map_open_error(e, path))?;
    if metadata.is_dir() {
        Err(Error::NotFound {
            path: PathBuf::from(path),
            source: io::Error::new(ErrorKind::NotFound, "is directory"),
        })
    } else {
        Ok(metadata)
    }
}

/// Translates errors from opening a file into a more specific [`Error`] when possible
fn map_open_error(source: io::Error, path: &std::path::Path) -> Error {
    let path = PathBuf::from(path);
    match source.kind() {
        ErrorKind::NotFound => Error::NotFound { path, source },
        _ => Error::UnableToOpenFile { path, source },
    }
}

/// Translates errors from attempting to a file into a more specific [`Error`] when possible
fn map_seek_error(source: io::Error, file: &File, path: &std::path::Path, requested: u64) -> Error {
    // if we can't seek, check if start is out of bounds to give
    // a better error. Don't read metadata before to avoid
    // an extra syscall in the common case
    let m = match open_metadata(file, path) {
        Err(e) => return e,
        Ok(m) => m,
    };
    if requested >= m.len() {
        return Error::InvalidRange {
            source: InvalidGetRange::StartTooLarge {
                requested,
                length: m.len(),
            },
        };
    }
    Error::Seek {
        source,
        path: PathBuf::from(path),
    }
}

fn convert_entry(entry: DirEntry, location: Path) -> Result<Option<ObjectMeta>> {
    match entry.metadata() {
        Ok(metadata) => Ok(Some(convert_metadata(metadata, location))),
        Err(e) => {
            if let Some(io_err) = e.io_error() {
                if io_err.kind() == ErrorKind::NotFound {
                    return Ok(None);
                }
            }
            Err(Error::Metadata {
                source: e.into(),
                path: location.to_string(),
            })?
        }
    }
}

fn last_modified(metadata: &Metadata) -> DateTime<Utc> {
    metadata
        .modified()
        .expect("Modified file time should be supported on this platform")
        .into()
}

fn get_etag(metadata: &Metadata) -> String {
    let inode = get_inode(metadata);
    let size = metadata.len();
    let mtime = metadata
        .modified()
        .ok()
        .and_then(|mtime| mtime.duration_since(SystemTime::UNIX_EPOCH).ok())
        .unwrap_or_default()
        .as_micros();

    // Use an ETag scheme based on that used by many popular HTTP servers
    // <https://httpd.apache.org/docs/2.2/mod/core.html#fileetag>
    // <https://stackoverflow.com/questions/47512043/how-etags-are-generated-and-configured>
    format!("\"{inode:x}-{mtime:x}-{size:x}\"")
}

fn convert_metadata(metadata: Metadata, location: Path) -> ObjectMeta {
    let last_modified = last_modified(&metadata);

    ObjectMeta {
        location,
        last_modified,
        size: metadata.len(),
        e_tag: Some(get_etag(&metadata)),
        version: None,
    }
}

#[cfg(unix)]
/// We include the inode when available to yield an ETag more resistant to collisions
/// and as used by popular web servers such as [Apache](https://httpd.apache.org/docs/2.2/mod/core.html#fileetag)
fn get_inode(metadata: &Metadata) -> u64 {
    std::os::unix::fs::MetadataExt::ino(metadata)
}

#[cfg(not(unix))]
/// On platforms where an inode isn't available, fallback to just relying on size and mtime
fn get_inode(_metadata: &Metadata) -> u64 {
    0
}

/// Convert walkdir results and converts not-found errors into `None`.
/// Convert broken symlinks to `None`.
fn convert_walkdir_result(
    res: std::result::Result<DirEntry, walkdir::Error>,
) -> Result<Option<DirEntry>> {
    match res {
        Ok(entry) => {
            // To check for broken symlink: call symlink_metadata() - it does not traverse symlinks);
            // if ok: check if entry is symlink; and try to read it by calling metadata().
            match symlink_metadata(entry.path()) {
                Ok(attr) => {
                    if attr.is_symlink() {
                        let target_metadata = metadata(entry.path());
                        match target_metadata {
                            Ok(_) => {
                                // symlink is valid
                                Ok(Some(entry))
                            }
                            Err(_) => {
                                // this is a broken symlink, return None
                                Ok(None)
                            }
                        }
                    } else {
                        Ok(Some(entry))
                    }
                }
                Err(_) => Ok(None),
            }
        }

        Err(walkdir_err) => match walkdir_err.io_error() {
            Some(io_err) => match io_err.kind() {
                ErrorKind::NotFound => Ok(None),
                _ => Err(Error::UnableToWalkDir {
                    source: walkdir_err,
                }
                .into()),
            },
            None => Err(Error::UnableToWalkDir {
                source: walkdir_err,
            }
            .into()),
        },
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use futures_util::TryStreamExt;
    use tempfile::TempDir;

    #[cfg(target_family = "unix")]
    use std::os::unix::fs::PermissionsExt;

    #[cfg(target_family = "unix")]
    use tempfile::NamedTempFile;

    use crate::{ObjectStoreExt, integration::*};

    use super::*;

    #[tokio::test]
    #[cfg(target_family = "unix")]
    async fn file_test() {
        let root = TempDir::new().unwrap();
        let integration = LocalFileSystem::new_with_prefix(root.path()).unwrap();

        put_get_delete_list(&integration).await;
        list_with_offset_exclusivity(&integration).await;
        get_opts(&integration).await;
        list_uses_directories_correctly(&integration).await;
        list_with_delimiter(&integration).await;
        rename_and_copy(&integration).await;
        copy_if_not_exists(&integration).await;
        copy_rename_nonexistent_object(&integration).await;
        stream_get(&integration).await;
        put_opts(&integration, false).await;
    }

    #[tokio::test]
    #[cfg(target_family = "unix")]
    async fn file_test_fsync() {
        // Run the full integration suite with fsync enabled to ensure the durability code
        // paths (file sync + directory fsync on put/copy/rename/multipart, including recursive
        // directory creation) behave identically to the default.
        let root = TempDir::new().unwrap();
        let integration = LocalFileSystem::new_with_prefix(root.path())
            .unwrap()
            .with_fsync(true);

        put_get_delete_list(&integration).await;
        list_with_offset_exclusivity(&integration).await;
        get_opts(&integration).await;
        list_uses_directories_correctly(&integration).await;
        list_with_delimiter(&integration).await;
        rename_and_copy(&integration).await;
        copy_if_not_exists(&integration).await;
        copy_rename_nonexistent_object(&integration).await;
        stream_get(&integration).await;
        put_opts(&integration, false).await;
    }

    #[tokio::test]
    async fn fsync_creates_nested_dirs() {
        // Exercises the recursive directory fsync in `create_parent_dirs`: every directory
        // component is newly created, so each must be synced up to the pre-existing root.
        let root = TempDir::new().unwrap();
        let integration = LocalFileSystem::new_with_prefix(root.path())
            .unwrap()
            .with_fsync(true);

        let data = Bytes::from("arbitrary data");

        // `put` (overwrite) into a deeply nested, non-existent directory tree
        let location = Path::from("a/b/c/d/put_file");
        integration
            .put(&location, data.clone().into())
            .await
            .unwrap();
        let read = integration
            .get(&location)
            .await
            .unwrap()
            .bytes()
            .await
            .unwrap();
        assert_eq!(read, data);

        // multipart upload into another nested tree
        let location = Path::from("e/f/g/multipart_file");
        let mut upload = integration.put_multipart(&location).await.unwrap();
        upload.put_part(data.clone().into()).await.unwrap();
        upload.complete().await.unwrap();
        let read = integration
            .get(&location)
            .await
            .unwrap()
            .bytes()
            .await
            .unwrap();
        assert_eq!(read, data);
    }

    #[tokio::test]
    #[cfg(target_family = "unix")]
    async fn fsync_rename_if_not_exists_propagates_source_delete_sync_error() {
        let root = TempDir::new().unwrap();
        let integration = LocalFileSystem::new_with_prefix(root.path())
            .unwrap()
            .with_fsync(true);

        let source = Path::from("source_dir/source_file");
        let dest = Path::from("dest_dir/dest_file");
        integration.put(&source, "data".into()).await.unwrap();

        let source_dir = root.path().join("source_dir");
        let original_permissions = fs::metadata(&source_dir).unwrap().permissions();
        // Allow unlinking the file from the directory, but prevent opening the
        // directory for fsync. Without the source-parent fsync, rename succeeds;
        // with it, the sync error is propagated after the source is deleted.
        fs::set_permissions(&source_dir, fs::Permissions::from_mode(0o300)).unwrap();

        let result = integration.rename_if_not_exists(&source, &dest).await;
        fs::set_permissions(&source_dir, original_permissions).unwrap();

        match result {
            Err(crate::Error::Generic { source, .. }) => {
                assert!(source.to_string().contains("Unable to sync data to disk"));
            }
            _ => panic!("expected source parent fsync to fail"),
        }

        let read = integration.get(&dest).await.unwrap().bytes().await.unwrap();
        assert_eq!(read, Bytes::from("data"));
        assert!(matches!(
            integration.get(&source).await.unwrap_err(),
            crate::Error::NotFound { .. }
        ));
    }

    #[test]
    #[cfg(target_family = "unix")]
    fn test_non_tokio() {
        let root = TempDir::new().unwrap();
        let integration = LocalFileSystem::new_with_prefix(root.path()).unwrap();
        futures_executor::block_on(async move {
            put_get_delete_list(&integration).await;
            list_uses_directories_correctly(&integration).await;
            list_with_delimiter(&integration).await;

            // Can't use stream_get test as WriteMultipart uses a tokio JoinSet
            let p = Path::from("manual_upload");
            let mut upload = integration.put_multipart(&p).await.unwrap();
            upload.put_part("123".into()).await.unwrap();
            upload.put_part("45678".into()).await.unwrap();
            let r = upload.complete().await.unwrap();

            let get = integration.get(&p).await.unwrap();
            assert_eq!(get.meta.e_tag.as_ref().unwrap(), r.e_tag.as_ref().unwrap());
            let actual = get.bytes().await.unwrap();
            assert_eq!(actual.as_ref(), b"12345678");
        });
    }

    #[tokio::test]
    async fn creates_dir_if_not_present() {
        let root = TempDir::new().unwrap();
        let integration = LocalFileSystem::new_with_prefix(root.path()).unwrap();

        let location = Path::from("nested/file/test_file");

        let data = Bytes::from("arbitrary data");

        integration
            .put(&location, data.clone().into())
            .await
            .unwrap();

        let read_data = integration
            .get(&location)
            .await
            .unwrap()
            .bytes()
            .await
            .unwrap();
        assert_eq!(&*read_data, data);
    }

    #[tokio::test]
    async fn unknown_length() {
        let root = TempDir::new().unwrap();
        let integration = LocalFileSystem::new_with_prefix(root.path()).unwrap();

        let location = Path::from("some_file");

        let data = Bytes::from("arbitrary data");

        integration
            .put(&location, data.clone().into())
            .await
            .unwrap();

        let read_data = integration
            .get(&location)
            .await
            .unwrap()
            .bytes()
            .await
            .unwrap();
        assert_eq!(&*read_data, data);
    }

    #[tokio::test]
    async fn range_request_start_beyond_end_of_file() {
        let root = TempDir::new().unwrap();
        let integration = LocalFileSystem::new_with_prefix(root.path()).unwrap();

        let location = Path::from("some_file");

        let data = Bytes::from("arbitrary data");

        integration
            .put(&location, data.clone().into())
            .await
            .unwrap();

        integration
            .get_range(&location, 100..200)
            .await
            .expect_err("Should error with start range beyond end of file");
    }

    #[tokio::test]
    async fn range_request_beyond_end_of_file() {
        let root = TempDir::new().unwrap();
        let integration = LocalFileSystem::new_with_prefix(root.path()).unwrap();

        let location = Path::from("some_file");

        let data = Bytes::from("arbitrary data");

        integration
            .put(&location, data.clone().into())
            .await
            .unwrap();

        let read_data = integration.get_range(&location, 0..100).await.unwrap();
        assert_eq!(&*read_data, data);
    }

    #[tokio::test]
    async fn get_ranges_coalesces() {
        let root = TempDir::new().unwrap();
        let integration = LocalFileSystem::new_with_prefix(root.path()).unwrap();
        let location = Path::from("some_file");

        // Position-dependent content so a misrouted slice is detectable.
        let data: Bytes = (0..4096u32)
            .map(|i| (i % 251) as u8)
            .collect::<Vec<_>>()
            .into();
        integration
            .put(&location, data.clone().into())
            .await
            .unwrap();

        let cases: Vec<Vec<Range<u64>>> = vec![
            vec![0..16, 16..32, 32..48],                    // adjacent -> coalesced
            vec![0..16, 64..80, 1000..1016],                // gapped -> not coalesced
            vec![100..120, 0..50, 40..60, 0..50, 110..130], // out-of-order, overlapping, duplicate
            vec![],                                         // empty
        ];

        for ranges in cases {
            let got = integration.get_ranges(&location, &ranges).await.unwrap();
            assert_eq!(got.len(), ranges.len());
            for (range, bytes) in ranges.iter().zip(got) {
                assert_eq!(
                    bytes,
                    data.slice(range.start as usize..range.end as usize),
                    "mismatch for range {range:?}"
                );
            }
        }
    }

    #[tokio::test]
    #[cfg(target_family = "unix")]
    // Fails on github actions runner (which runs the tests as root)
    #[ignore]
    async fn bubble_up_io_errors() {
        use std::{fs::set_permissions, os::unix::prelude::PermissionsExt};

        let root = TempDir::new().unwrap();

        // make non-readable
        let metadata = root.path().metadata().unwrap();
        let mut permissions = metadata.permissions();
        permissions.set_mode(0o000);
        set_permissions(root.path(), permissions).unwrap();

        let store = LocalFileSystem::new_with_prefix(root.path()).unwrap();

        let mut stream = store.list(None);
        let mut any_err = false;
        while let Some(res) = stream.next().await {
            if res.is_err() {
                any_err = true;
            }
        }
        assert!(any_err);

        // `list_with_delimiter
        assert!(store.list_with_delimiter(None).await.is_err());
    }

    const NON_EXISTENT_NAME: &str = "nonexistentname";

    #[tokio::test]
    async fn get_nonexistent_location() {
        let root = TempDir::new().unwrap();
        let integration = LocalFileSystem::new_with_prefix(root.path()).unwrap();

        let location = Path::from(NON_EXISTENT_NAME);

        let err = get_nonexistent_object(&integration, Some(location))
            .await
            .unwrap_err();
        if let crate::Error::NotFound { path, source } = err {
            let source_variant = source.downcast_ref::<std::io::Error>();
            assert!(
                matches!(source_variant, Some(std::io::Error { .. }),),
                "got: {source_variant:?}"
            );
            assert!(path.ends_with(NON_EXISTENT_NAME), "{}", path);
        } else {
            panic!("unexpected error type: {err:?}");
        }
    }

    #[tokio::test]
    async fn root() {
        let integration = LocalFileSystem::new();

        let canonical = std::path::Path::new("Cargo.toml").canonicalize().unwrap();
        let url = Url::from_directory_path(&canonical).unwrap();
        let path = Path::parse(url.path()).unwrap();

        let roundtrip = integration.path_to_filesystem(&path).unwrap();

        // Needed as on Windows canonicalize returns extended length path syntax
        // C:\Users\circleci -> \\?\C:\Users\circleci
        let roundtrip = roundtrip.canonicalize().unwrap();

        assert_eq!(roundtrip, canonical);

        integration.head(&path).await.unwrap();
    }

    #[tokio::test]
    #[cfg(target_family = "windows")]
    async fn test_list_root() {
        let fs = LocalFileSystem::new();
        let r = fs.list_with_delimiter(None).await.unwrap_err().to_string();

        assert!(
            r.contains("Unable to convert URL \"file:///\" to filesystem path"),
            "{}",
            r
        );
    }

    #[tokio::test]
    #[cfg(target_os = "linux")]
    async fn test_list_root() {
        let fs = LocalFileSystem::new();
        fs.list_with_delimiter(None).await.unwrap();
    }

    #[cfg(target_family = "unix")]
    async fn check_list(integration: &LocalFileSystem, prefix: Option<&Path>, expected: &[&str]) {
        let result: Vec<_> = integration.list(prefix).try_collect().await.unwrap();

        let mut strings: Vec<_> = result.iter().map(|x| x.location.as_ref()).collect();
        strings.sort_unstable();
        assert_eq!(&strings, expected)
    }

    #[tokio::test]
    #[cfg(target_family = "unix")]
    async fn test_symlink() {
        let root = TempDir::new().unwrap();
        let integration = LocalFileSystem::new_with_prefix(root.path()).unwrap();

        let subdir = root.path().join("a");
        std::fs::create_dir(&subdir).unwrap();
        let file = subdir.join("file.parquet");
        std::fs::write(file, "test").unwrap();

        check_list(&integration, None, &["a/file.parquet"]).await;
        integration
            .head(&Path::from("a/file.parquet"))
            .await
            .unwrap();

        // Follow out of tree symlink
        let other = NamedTempFile::new().unwrap();
        std::os::unix::fs::symlink(other.path(), root.path().join("test.parquet")).unwrap();

        // Should return test.parquet even though out of tree
        check_list(&integration, None, &["a/file.parquet", "test.parquet"]).await;

        // Can fetch test.parquet
        integration.head(&Path::from("test.parquet")).await.unwrap();

        // Follow in tree symlink
        std::os::unix::fs::symlink(&subdir, root.path().join("b")).unwrap();
        check_list(
            &integration,
            None,
            &["a/file.parquet", "b/file.parquet", "test.parquet"],
        )
        .await;
        check_list(&integration, Some(&Path::from("b")), &["b/file.parquet"]).await;

        // Can fetch through symlink
        integration
            .head(&Path::from("b/file.parquet"))
            .await
            .unwrap();

        // Ignore broken symlink
        std::os::unix::fs::symlink(root.path().join("foo.parquet"), root.path().join("c")).unwrap();

        check_list(
            &integration,
            None,
            &["a/file.parquet", "b/file.parquet", "test.parquet"],
        )
        .await;

        let mut r = integration.list_with_delimiter(None).await.unwrap();
        r.common_prefixes.sort_unstable();
        assert_eq!(r.common_prefixes.len(), 2);
        assert_eq!(r.common_prefixes[0].as_ref(), "a");
        assert_eq!(r.common_prefixes[1].as_ref(), "b");
        assert_eq!(r.objects.len(), 1);
        assert_eq!(r.objects[0].location.as_ref(), "test.parquet");

        let r = integration
            .list_with_delimiter(Some(&Path::from("a")))
            .await
            .unwrap();
        assert_eq!(r.common_prefixes.len(), 0);
        assert_eq!(r.objects.len(), 1);
        assert_eq!(r.objects[0].location.as_ref(), "a/file.parquet");

        // Deleting a symlink doesn't delete the source file
        integration
            .delete(&Path::from("test.parquet"))
            .await
            .unwrap();
        assert!(other.path().exists());

        check_list(&integration, None, &["a/file.parquet", "b/file.parquet"]).await;

        // Deleting through a symlink deletes both files
        integration
            .delete(&Path::from("b/file.parquet"))
            .await
            .unwrap();

        check_list(&integration, None, &[]).await;

        // Adding a file through a symlink creates in both paths
        integration
            .put(&Path::from("b/file.parquet"), vec![0, 1, 2].into())
            .await
            .unwrap();

        check_list(&integration, None, &["a/file.parquet", "b/file.parquet"]).await;
    }

    #[tokio::test]
    async fn invalid_path() {
        let root = TempDir::new().unwrap();
        let root = root.path().join("🙀");
        std::fs::create_dir(root.clone()).unwrap();

        // Invalid paths supported above root of store
        let integration = LocalFileSystem::new_with_prefix(root.clone()).unwrap();

        let directory = Path::from("directory");
        let object = directory.clone().join("child.txt");
        let data = Bytes::from("arbitrary");
        integration.put(&object, data.clone().into()).await.unwrap();
        integration.head(&object).await.unwrap();
        let result = integration.get(&object).await.unwrap();
        assert_eq!(result.bytes().await.unwrap(), data);

        flatten_list_stream(&integration, None).await.unwrap();
        flatten_list_stream(&integration, Some(&directory))
            .await
            .unwrap();

        let result = integration
            .list_with_delimiter(Some(&directory))
            .await
            .unwrap();
        assert_eq!(result.objects.len(), 1);
        assert!(result.common_prefixes.is_empty());
        assert_eq!(result.objects[0].location, object);

        let emoji = root.join("💀");
        std::fs::write(emoji, "foo").unwrap();

        // Can list illegal file
        let mut paths = flatten_list_stream(&integration, None).await.unwrap();
        paths.sort_unstable();

        assert_eq!(
            paths,
            vec![
                Path::parse("directory/child.txt").unwrap(),
                Path::parse("💀").unwrap()
            ]
        );
    }

    #[tokio::test]
    async fn list_hides_incomplete_uploads() {
        let root = TempDir::new().unwrap();
        let integration = LocalFileSystem::new_with_prefix(root.path()).unwrap();
        let location = Path::from("some_file");

        let data = PutPayload::from("arbitrary data");
        let mut u1 = integration.put_multipart(&location).await.unwrap();
        u1.put_part(data.clone()).await.unwrap();

        let mut u2 = integration.put_multipart(&location).await.unwrap();
        u2.put_part(data).await.unwrap();

        let list = flatten_list_stream(&integration, None).await.unwrap();
        assert_eq!(list.len(), 0);

        assert_eq!(
            integration
                .list_with_delimiter(None)
                .await
                .unwrap()
                .objects
                .len(),
            0
        );
    }

    #[tokio::test]
    async fn test_path_with_offset() {
        let root = TempDir::new().unwrap();
        let integration = LocalFileSystem::new_with_prefix(root.path()).unwrap();

        let root_path = root.path();
        for i in 0..5 {
            let filename = format!("test{i}.parquet");
            let file = root_path.join(filename);
            std::fs::write(file, "test").unwrap();
        }
        let filter_str = "test";
        let filter = String::from(filter_str);
        let offset_str = filter + "1";
        let offset = Path::from(offset_str.clone());

        // Use list_with_offset to retrieve files
        let res = integration.list_with_offset(None, &offset);
        let offset_paths: Vec<_> = res.map_ok(|x| x.location).try_collect().await.unwrap();
        let mut offset_files: Vec<_> = offset_paths
            .iter()
            .map(|x| String::from(x.filename().unwrap()))
            .collect();

        // Check result with direct filesystem read
        let files = fs::read_dir(root_path).unwrap();
        let filtered_files = files
            .filter_map(Result::ok)
            .filter_map(|d| {
                d.file_name().to_str().and_then(|f| {
                    if f.contains(filter_str) {
                        Some(String::from(f))
                    } else {
                        None
                    }
                })
            })
            .collect::<Vec<_>>();

        let mut expected_offset_files: Vec<_> = filtered_files
            .iter()
            .filter(|s| **s > offset_str)
            .cloned()
            .collect();

        fn do_vecs_match<T: PartialEq>(a: &[T], b: &[T]) -> bool {
            let matching = a.iter().zip(b.iter()).filter(|&(a, b)| a == b).count();
            matching == a.len() && matching == b.len()
        }

        offset_files.sort();
        expected_offset_files.sort();

        // println!("Expected Offset Files: {:?}", expected_offset_files);
        // println!("Actual Offset Files: {:?}", offset_files);

        assert_eq!(offset_files.len(), expected_offset_files.len());
        assert!(do_vecs_match(&expected_offset_files, &offset_files));
    }

    #[tokio::test]
    async fn filesystem_filename_with_percent() {
        let temp_dir = TempDir::new().unwrap();
        let integration = LocalFileSystem::new_with_prefix(temp_dir.path()).unwrap();
        let filename = "L%3ABC.parquet";

        std::fs::write(temp_dir.path().join(filename), "foo").unwrap();

        let res: Vec<_> = integration.list(None).try_collect().await.unwrap();
        assert_eq!(res.len(), 1);
        assert_eq!(res[0].location.as_ref(), filename);

        let res = integration.list_with_delimiter(None).await.unwrap();
        assert_eq!(res.objects.len(), 1);
        assert_eq!(res.objects[0].location.as_ref(), filename);
    }

    #[tokio::test]
    async fn relative_paths() {
        LocalFileSystem::new_with_prefix(".").unwrap();
        LocalFileSystem::new_with_prefix("..").unwrap();
        LocalFileSystem::new_with_prefix("../..").unwrap();

        let integration = LocalFileSystem::new();
        let path = Path::from_filesystem_path(".").unwrap();
        integration.list_with_delimiter(Some(&path)).await.unwrap();
    }

    #[test]
    fn test_valid_path() {
        let cases = [
            ("foo#123/test.txt", true),
            ("foo#123/test#23.txt", true),
            ("foo#123/test#34", false),
            ("foo😁/test#34", false),
            ("foo/test#😁34", true),
        ];

        for (case, expected) in cases {
            let path = Path::parse(case).unwrap();
            assert_eq!(is_valid_file_path(&path), expected);
        }
    }

    #[tokio::test]
    async fn test_intermediate_files() {
        let root = TempDir::new().unwrap();
        let integration = LocalFileSystem::new_with_prefix(root.path()).unwrap();

        let a = Path::parse("foo#123/test.txt").unwrap();
        integration.put(&a, "test".into()).await.unwrap();

        let list = flatten_list_stream(&integration, None).await.unwrap();
        assert_eq!(list, vec![a.clone()]);

        std::fs::write(root.path().join("bar#123"), "test").unwrap();

        // Should ignore file
        let list = flatten_list_stream(&integration, None).await.unwrap();
        assert_eq!(list, vec![a.clone()]);

        let b = Path::parse("bar#123").unwrap();
        let err = integration.get(&b).await.unwrap_err().to_string();
        assert_eq!(
            err,
            "Generic LocalFileSystem error: Filenames containing trailing '/#\\d+/' are not supported: bar#123"
        );

        let c = Path::parse("foo#123.txt").unwrap();
        integration.put(&c, "test".into()).await.unwrap();

        let mut list = flatten_list_stream(&integration, None).await.unwrap();
        list.sort_unstable();
        assert_eq!(list, vec![c, a]);
    }

    #[tokio::test]
    #[cfg(target_os = "windows")]
    async fn filesystem_filename_with_colon() {
        let root = TempDir::new().unwrap();
        let integration = LocalFileSystem::new_with_prefix(root.path()).unwrap();
        let path = Path::parse("file%3Aname.parquet").unwrap();
        let location = Path::parse("file:name.parquet").unwrap();

        integration.put(&location, "test".into()).await.unwrap();
        let list = flatten_list_stream(&integration, None).await.unwrap();
        assert_eq!(list, vec![path.clone()]);

        let result = integration
            .get(&location)
            .await
            .unwrap()
            .bytes()
            .await
            .unwrap();
        assert_eq!(result, Bytes::from("test"));
    }

    #[tokio::test]
    async fn delete_dirs_automatically() {
        let root = TempDir::new().unwrap();
        let integration = LocalFileSystem::new_with_prefix(root.path())
            .unwrap()
            .with_automatic_cleanup(true);
        let location = Path::from("nested/file/test_file");
        let data = Bytes::from("arbitrary data");

        integration
            .put(&location, data.clone().into())
            .await
            .unwrap();

        let read_data = integration
            .get(&location)
            .await
            .unwrap()
            .bytes()
            .await
            .unwrap();

        assert_eq!(&*read_data, data);
        assert!(fs::read_dir(root.path()).unwrap().count() > 0);
        integration.delete(&location).await.unwrap();
        assert!(fs::read_dir(root.path()).unwrap().count() == 0);
    }

    #[test]
    #[cfg(target_family = "unix")]
    fn test_close_file_detects_error_unix() {
        let err = super::close_fd(-1).unwrap_err();
        assert_eq!(err.raw_os_error(), Some(nix::libc::EBADF), "got: {err:?}");
    }

    #[test]
    #[cfg(target_family = "windows")]
    fn test_close_file_detects_error_windows() {
        let err = super::close_handle(std::ptr::null_mut()).unwrap_err();
        assert_eq!(
            err.raw_os_error(),
            Some(windows_sys::Win32::Foundation::ERROR_INVALID_HANDLE as i32),
            "got: {err:?}"
        );
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod not_wasm_tests {
    use std::time::Duration;
    use tempfile::TempDir;

    use crate::local::LocalFileSystem;
    use crate::{ObjectStoreExt, Path, PutPayload};

    #[tokio::test]
    async fn test_cleanup_intermediate_files() {
        let root = TempDir::new().unwrap();
        let integration = LocalFileSystem::new_with_prefix(root.path()).unwrap();

        let location = Path::from("some_file");
        let data = PutPayload::from_static(b"hello");
        let mut upload = integration.put_multipart(&location).await.unwrap();
        upload.put_part(data).await.unwrap();

        let file_count = std::fs::read_dir(root.path()).unwrap().count();
        assert_eq!(file_count, 1);
        drop(upload);

        for _ in 0..100 {
            tokio::time::sleep(Duration::from_millis(1)).await;
            let file_count = std::fs::read_dir(root.path()).unwrap().count();
            if file_count == 0 {
                return;
            }
        }
        panic!("Failed to cleanup file in 100ms")
    }
}

#[cfg(target_family = "unix")]
#[cfg(test)]
mod unix_test {
    use std::fs::OpenOptions;

    use nix::sys::stat;
    use nix::unistd;
    use tempfile::TempDir;

    use crate::local::LocalFileSystem;
    use crate::{ObjectStoreExt, Path};

    #[tokio::test]
    async fn test_fifo() {
        let filename = "some_file";
        let root = TempDir::new().unwrap();
        let integration = LocalFileSystem::new_with_prefix(root.path()).unwrap();
        let path = root.path().join(filename);
        unistd::mkfifo(&path, stat::Mode::S_IRWXU).unwrap();

        // Need to open read and write side in parallel
        let spawned =
            tokio::task::spawn_blocking(|| OpenOptions::new().write(true).open(path).unwrap());

        let location = Path::from(filename);
        integration.head(&location).await.unwrap();
        integration.get(&location).await.unwrap();

        spawned.await.unwrap();
    }
}
