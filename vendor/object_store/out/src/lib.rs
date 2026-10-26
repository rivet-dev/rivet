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

#![cfg_attr(docsrs, feature(doc_cfg))]
#![deny(rustdoc::broken_intra_doc_links, rustdoc::bare_urls, rust_2018_idioms)]
#![warn(
    missing_copy_implementations,
    missing_debug_implementations,
    missing_docs,
    clippy::explicit_iter_loop,
    clippy::future_not_send,
    clippy::use_self,
    clippy::clone_on_ref_ptr,
    unreachable_pub
)]

//! # object_store
//!
//! This crate provides a uniform API for interacting with object
//! storage services and local files via the [`ObjectStore`]
//! trait.
//!
//! Using this crate, the same binary and code can run in multiple
//! clouds and local test environments, via a simple runtime
//! configuration change.
//!
//! # Highlights
//!
//! 1. A high-performance async API focused on providing a consistent interface
//!    mirroring that of object stores such as [S3]
//!
//! 2. Production quality, leading this crate to be used in large
//!    scale production systems, such as [crates.io] and [InfluxDB IOx]
//!
//! 3. Support for advanced functionality, including atomic, conditional reads
//!    and writes, vectored IO, bulk deletion, and more...
//!
//! 4. Stable and predictable governance via the [Apache Arrow] project
//!
//! 5. Small dependency footprint, depending on only a small number of common crates
//!
//! Originally developed by [InfluxData] and subsequently donated
//! to [Apache Arrow].
//!
//! [Apache Arrow]: https://arrow.apache.org/
//! [InfluxData]: https://www.influxdata.com/
//! [crates.io]: https://github.com/rust-lang/crates.io
//! [ACID]: https://en.wikipedia.org/wiki/ACID
//! [S3]: https://aws.amazon.com/s3/
//!
//! # APIs
//!
//! * [`ObjectStore`]: Core object store API
//! * [`ObjectStoreExt`]: (*New in 0.13.0*) Extension trait with additional convenience methods
//!
//! # Available [`ObjectStore`] Implementations
//!
//! By default, this crate provides the following implementations:
//!
//! * Memory: [`InMemory`](memory::InMemory)
//!
//! Feature flags are used to enable support for other implementations:
//!
#![cfg_attr(
    feature = "fs",
    doc = "* Local filesystem: [`LocalFileSystem`](local::LocalFileSystem)"
)]
#![cfg_attr(
    feature = "gcp-base",
    doc = "* [`gcp`]: [Google Cloud Storage](https://cloud.google.com/storage/) support. See [`GoogleCloudStorageBuilder`](gcp::GoogleCloudStorageBuilder)"
)]
#![cfg_attr(
    feature = "aws-base",
    doc = "* [`aws`]: [Amazon S3](https://aws.amazon.com/s3/). See [`AmazonS3Builder`](aws::AmazonS3Builder)"
)]
#![cfg_attr(
    feature = "azure-base",
    doc = "* [`azure`]: [Azure Blob Storage](https://azure.microsoft.com/en-gb/services/storage/blobs/). See [`MicrosoftAzureBuilder`](azure::MicrosoftAzureBuilder)"
)]
#![cfg_attr(
    feature = "http-base",
    doc = "* [`http`]: [HTTP/WebDAV Storage](https://datatracker.ietf.org/doc/html/rfc2518). See [`HttpBuilder`](http::HttpBuilder)"
)]
//!
//! See [Feature Flags](#feature-flags) for the full set of flags.
//!
//! # Why not a Filesystem Interface?
//!
//! The [`ObjectStore`] interface is designed to mirror the APIs
//! of object stores and *not* filesystems, and thus has stateless APIs instead
//! of cursor based interfaces such as [`Read`] or [`Seek`] available in filesystems.
//!
//! This design provides the following advantages:
//!
//! * All operations are atomic, and readers cannot observe partial and/or failed writes
//! * Methods map directly to object store APIs, providing both efficiency and predictability
//! * Abstracts away filesystem and operating system specific quirks, ensuring portability
//! * Allows for functionality not native to filesystems, such as operation preconditions
//!   and atomic multipart uploads
//!
//! This crate does provide [`BufReader`] and [`BufWriter`] adapters
//! which provide a more filesystem-like API for working with the
//! [`ObjectStore`] trait, however, they should be used with care
//!
//! [`BufReader`]: buffered::BufReader
//! [`BufWriter`]: buffered::BufWriter
//! [`Read`]: std::io::Read
//! [`Seek`]: std::io::Seek
//!
//! # Adapters
//!
//! [`ObjectStore`] instances can be composed with various adapters
//! which add additional functionality:
//!
//! * Rate Throttling: [`ThrottleConfig`](throttle::ThrottleConfig)
//! * Concurrent Request Limit: [`LimitStore`](limit::LimitStore)
//!
//! # Configuration System
//!
//! This crate provides a configuration system inspired by the APIs exposed by [fsspec],
//! [PyArrow FileSystem], and [Hadoop FileSystem], allowing creating a [`DynObjectStore`]
//! from a URL and an optional list of key value pairs. This provides a flexible interface
//! to support a wide variety of user-defined store configurations, with minimal additional
//! application complexity.
//!
//! ```no_run,ignore-wasm32
//! # #[cfg(feature = "aws")] {
//! # use url::Url;
//! # use object_store::{parse_url, parse_url_opts};
//! # use object_store::aws::{AmazonS3, AmazonS3Builder};
//! #
//! #
//! // Can manually create a specific store variant using the appropriate builder
//! let store: AmazonS3 = AmazonS3Builder::from_env()
//!     .with_bucket_name("my-bucket").build().unwrap();
//!
//! // Alternatively can create an ObjectStore from an S3 URL
//! let url = Url::parse("s3://bucket/path").unwrap();
//! let (store, path) = parse_url(&url).unwrap();
//! assert_eq!(path.as_ref(), "path");
//!
//! // Potentially with additional options
//! let (store, path) = parse_url_opts(&url, vec![("aws_access_key_id", "...")]).unwrap();
//!
//! // Or with URLs that encode the bucket name in the URL path
//! let url = Url::parse("https://ACCOUNT_ID.r2.cloudflarestorage.com/bucket/path").unwrap();
//! let (store, path) = parse_url(&url).unwrap();
//! assert_eq!(path.as_ref(), "path");
//! # }
//! ```
//!
//! [PyArrow FileSystem]: https://arrow.apache.org/docs/python/generated/pyarrow.fs.FileSystem.html#pyarrow.fs.FileSystem.from_uri
//! [fsspec]: https://filesystem-spec.readthedocs.io/en/latest/api.html#fsspec.filesystem
//! [Hadoop FileSystem]: https://hadoop.apache.org/docs/r3.0.0/api/org/apache/hadoop/fs/FileSystem.html#get-java.net.URI-org.apache.hadoop.conf.Configuration-
//!
//! # List objects
//!
//! Use the [`ObjectStore::list`] method to iterate over objects in
//! remote storage or files in the local filesystem:
//!
//! ```ignore-wasm32
//! # use object_store::local::LocalFileSystem;
//! # use std::sync::Arc;
//! # use object_store::{path::Path, ObjectStore};
//! # use futures_util::stream::StreamExt;
//! # // use LocalFileSystem for example
//! # fn get_object_store() -> Arc<dyn ObjectStore> {
//! #   Arc::new(LocalFileSystem::new())
//! # }
//! #
//! # async fn example() {
//! #
//! // create an ObjectStore
//! let object_store: Arc<dyn ObjectStore> = get_object_store();
//!
//! // Recursively list all files below the 'data' path.
//! // 1. On AWS S3 this would be the 'data/' prefix
//! // 2. On a local filesystem, this would be the 'data' directory
//! let prefix = Path::from("data");
//!
//! // Get an `async` stream of Metadata objects:
//! let mut list_stream = object_store.list(Some(&prefix));
//!
//! // Print a line about each object
//! while let Some(meta) = list_stream.next().await.transpose().unwrap() {
//!     println!("Name: {}, size: {}", meta.location, meta.size);
//! }
//! # }
//! ```
//!
//! Which will print out something like the following:
//!
//! ```text
//! Name: data/file01.parquet, size: 112832
//! Name: data/file02.parquet, size: 143119
//! Name: data/child/file03.parquet, size: 100
//! ...
//! ```
//!
//! # Fetch objects
//!
//! Use the [`ObjectStoreExt::get`] / [`ObjectStore::get_opts`] method to fetch the data bytes
//! from remote storage or files in the local filesystem as a stream.
//!
//! ```ignore-wasm32
//! # use futures_util::TryStreamExt;
//! # use object_store::local::LocalFileSystem;
//! # use std::sync::Arc;
//! #  use bytes::Bytes;
//! # use object_store::{path::Path, ObjectStore, ObjectStoreExt, GetResult};
//! # fn get_object_store() -> Arc<dyn ObjectStore> {
//! #   Arc::new(LocalFileSystem::new())
//! # }
//! #
//! # async fn example() {
//! #
//! // Create an ObjectStore
//! let object_store: Arc<dyn ObjectStore> = get_object_store();
//!
//! // Retrieve a specific file
//! let path = Path::from("data/file01.parquet");
//!
//! // Fetch just the file metadata
//! let meta = object_store.head(&path).await.unwrap();
//! println!("{meta:?}");
//!
//! // Fetch the object including metadata
//! let result: GetResult = object_store.get(&path).await.unwrap();
//! assert_eq!(result.meta, meta);
//!
//! // Buffer the entire object in memory
//! let object: Bytes = result.bytes().await.unwrap();
//! assert_eq!(object.len() as u64, meta.size);
//!
//! // Alternatively stream the bytes from object storage
//! let stream = object_store.get(&path).await.unwrap().into_stream();
//!
//! // Count the '0's using `try_fold` from `TryStreamExt` trait
//! let num_zeros = stream
//!     .try_fold(0, |acc, bytes| async move {
//!         Ok(acc + bytes.iter().filter(|b| **b == 0).count())
//!     }).await.unwrap();
//!
//! println!("Num zeros in {} is {}", path, num_zeros);
//! # }
//! ```
//!
//! # Put Object
//!
//! Use the [`ObjectStoreExt::put`] method to atomically write data.
//!
//! To upload large objects without buffering them entirely in [`PutPayload`],
//! see [Multipart Upload](#multipart-upload) below.
//!
//! ```ignore-wasm32
//! # use object_store::local::LocalFileSystem;
//! # use object_store::{ObjectStore, ObjectStoreExt, PutPayload};
//! # use std::sync::Arc;
//! # use object_store::path::Path;
//! # fn get_object_store() -> Arc<dyn ObjectStore> {
//! #   Arc::new(LocalFileSystem::new())
//! # }
//! # async fn put() {
//! #
//! let object_store: Arc<dyn ObjectStore> = get_object_store();
//! let path = Path::from("data/file1");
//! let payload = PutPayload::from_static(b"hello");
//! object_store.put(&path, payload).await.unwrap();
//! # }
//! ```
//!
//! # Multipart Upload
//!
//! Use the [`ObjectStoreExt::put_multipart`] / [`ObjectStore::put_multipart_opts`] method to atomically write a large
//! amount of data in multiple parts, without buffering the entire object in memory.
//! [`WriteMultipart`] uploads fixed size parts in parallel as data is written.
//!
//! [`BufWriter`](buffered::BufWriter) provides an [`AsyncWrite`]
//! interface that automatically picks between a single and multipart upload
//! based on the amount of data written.
//!
//! [`AsyncWrite`]: tokio::io::AsyncWrite
//! [`BufWriter`]: buffered::BufWriter
//!
//! ```ignore-wasm32
//! # use object_store::local::LocalFileSystem;
//! # use object_store::{ObjectStore, ObjectStoreExt, WriteMultipart};
//! # use std::sync::Arc;
//! # use bytes::Bytes;
//! # use tokio::io::AsyncWriteExt;
//! # use object_store::path::Path;
//! # fn get_object_store() -> Arc<dyn ObjectStore> {
//! #   Arc::new(LocalFileSystem::new())
//! # }
//! # async fn multi_upload() {
//! #
//! let object_store: Arc<dyn ObjectStore> = get_object_store();
//! let path = Path::from("data/large_file");
//! let upload =  object_store.put_multipart(&path).await.unwrap();
//! let mut write = WriteMultipart::new(upload);
//! write.write(b"hello");
//! write.finish().await.unwrap();
//! # }
//! ```
//!
//! # Vectored Read
//!
//! A common pattern, especially when reading structured datasets, is to need to fetch
//! multiple, potentially non-contiguous, ranges of a particular object.
//!
//! [`ObjectStore::get_ranges`] provides an efficient way to perform such vectored IO, and will
//! automatically coalesce adjacent ranges into an appropriate number of parallel requests.
//!
//! ```ignore-wasm32
//! # use object_store::local::LocalFileSystem;
//! # use object_store::ObjectStore;
//! # use std::sync::Arc;
//! # use bytes::Bytes;
//! # use tokio::io::AsyncWriteExt;
//! # use object_store::path::Path;
//! # fn get_object_store() -> Arc<dyn ObjectStore> {
//! #   Arc::new(LocalFileSystem::new())
//! # }
//! # async fn multi_upload() {
//! #
//! let object_store: Arc<dyn ObjectStore> = get_object_store();
//! let path = Path::from("data/large_file");
//! let ranges = object_store.get_ranges(&path, &[90..100, 400..600, 0..10]).await.unwrap();
//! assert_eq!(ranges.len(), 3);
//! assert_eq!(ranges[0].len(), 10);
//! # }
//! ```
//!
//! To retrieve ranges from a versioned object, use [`ObjectStore::get_opts`] by specifying the range in the [`GetOptions`].
//!
//! ```ignore-wasm32
//! # use object_store::local::LocalFileSystem;
//! # use object_store::ObjectStore;
//! # use object_store::GetOptions;
//! # use std::sync::Arc;
//! # use bytes::Bytes;
//! # use tokio::io::AsyncWriteExt;
//! # use object_store::path::Path;
//! # fn get_object_store() -> Arc<dyn ObjectStore> {
//! #   Arc::new(LocalFileSystem::new())
//! # }
//! # async fn get_range_with_options() {
//! #
//! let object_store: Arc<dyn ObjectStore> = get_object_store();
//! let path = Path::from("data/large_file");
//! let ranges = vec![90..100, 400..600, 0..10];
//! for range in ranges {
//!     let opts = GetOptions::default().with_range(Some(range));
//!     let data = object_store.get_opts(&path, opts).await.unwrap();
//!     // Do something with the data
//! }
//! # }
//! ``````
//!
//! # Vectored Write
//!
//! When writing data it is often the case that the size of the output is not known ahead of time.
//!
//! A common approach to handling this is to bump-allocate a `Vec`, whereby the underlying
//! allocation is repeatedly reallocated, each time doubling the capacity. The performance of
//! this is suboptimal as reallocating memory will often involve copying it to a new location.
//!
//! Fortunately, as [`PutPayload`] does not require memory regions to be contiguous, it is
//! possible to instead allocate memory in chunks and avoid bump allocating. [`PutPayloadMut`]
//! encapsulates this approach
//!
//! ```ignore-wasm32
//! # use object_store::local::LocalFileSystem;
//! # use object_store::{ObjectStore, ObjectStoreExt, PutPayloadMut};
//! # use std::sync::Arc;
//! # use bytes::Bytes;
//! # use tokio::io::AsyncWriteExt;
//! # use object_store::path::Path;
//! # fn get_object_store() -> Arc<dyn ObjectStore> {
//! #   Arc::new(LocalFileSystem::new())
//! # }
//! # async fn multi_upload() {
//! #
//! let object_store: Arc<dyn ObjectStore> = get_object_store();
//! let path = Path::from("data/large_file");
//! let mut buffer = PutPayloadMut::new().with_block_size(8192);
//! for _ in 0..22 {
//!     buffer.extend_from_slice(&[0; 1024]);
//! }
//! let payload = buffer.freeze();
//!
//! // Payload consists of 3 separate 8KB allocations
//! assert_eq!(payload.as_ref().len(), 3);
//! assert_eq!(payload.as_ref()[0].len(), 8192);
//! assert_eq!(payload.as_ref()[1].len(), 8192);
//! assert_eq!(payload.as_ref()[2].len(), 6144);
//!
//! object_store.put(&path, payload).await.unwrap();
//! # }
//! ```
//!
//! # Conditional Fetch
//!
//! More complex object retrieval can be supported by [`ObjectStore::get_opts`].
//!
//! For example, efficiently refreshing a cache without re-fetching the entire object
//! data if the object hasn't been modified.
//!
//! ```
//! # use std::collections::btree_map::Entry;
//! # use std::collections::HashMap;
//! # use object_store::{GetOptions, GetResult, ObjectStore, ObjectStoreExt, Result, Error};
//! # use std::sync::Arc;
//! # use std::time::{Duration, Instant};
//! # use bytes::Bytes;
//! # use tokio::io::AsyncWriteExt;
//! # use object_store::path::Path;
//! struct CacheEntry {
//!     /// Data returned by last request
//!     data: Bytes,
//!     /// ETag identifying the object returned by the server
//!     e_tag: String,
//!     /// Instant of last refresh
//!     refreshed_at: Instant,
//! }
//!
//! /// Example cache that checks entries after 10 seconds for a new version
//! struct Cache {
//!     entries: HashMap<Path, CacheEntry>,
//!     store: Arc<dyn ObjectStore>,
//! }
//!
//! impl Cache {
//!     pub async fn get(&mut self, path: &Path) -> Result<Bytes> {
//!         Ok(match self.entries.get_mut(path) {
//!             Some(e) => match e.refreshed_at.elapsed() < Duration::from_secs(10) {
//!                 true => e.data.clone(), // Return cached data
//!                 false => { // Check if remote version has changed
//!                     let opts = GetOptions::new().with_if_none_match(Some(e.e_tag.clone()));
//!                     match self.store.get_opts(&path, opts).await {
//!                         Ok(d) => e.data = d.bytes().await?,
//!                         Err(Error::NotModified { .. }) => {} // Data has not changed
//!                         Err(e) => return Err(e),
//!                     };
//!                     e.refreshed_at = Instant::now();
//!                     e.data.clone()
//!                 }
//!             },
//!             None => { // Not cached, fetch data
//!                 let get = self.store.get(&path).await?;
//!                 let e_tag = get.meta.e_tag.clone();
//!                 let data = get.bytes().await?;
//!                 if let Some(e_tag) = e_tag {
//!                     let entry = CacheEntry {
//!                         e_tag,
//!                         data: data.clone(),
//!                         refreshed_at: Instant::now(),
//!                     };
//!                     self.entries.insert(path.clone(), entry);
//!                 }
//!                 data
//!             }
//!         })
//!     }
//! }
//! ```
//!
//! # Conditional Put
//!
//! The default behaviour when writing data is to upsert any existing object at the given path,
//! overwriting any previous value. More complex behaviours can be achieved using [`PutMode`], and
//! can be used to build [Optimistic Concurrency Control] based transactions. This facilitates
//! building metadata catalogs, such as [Apache Iceberg] or [Delta Lake], directly on top of object
//! storage, without relying on a separate DBMS.
//!
//! ```
//! # use object_store::{Error, ObjectStore, ObjectStoreExt, PutMode, UpdateVersion};
//! # use std::sync::Arc;
//! # use bytes::Bytes;
//! # use tokio::io::AsyncWriteExt;
//! # use object_store::memory::InMemory;
//! # use object_store::path::Path;
//! # fn get_object_store() -> Arc<dyn ObjectStore> {
//! #   Arc::new(InMemory::new())
//! # }
//! # fn do_update(b: Bytes) -> Bytes {b}
//! # async fn conditional_put() {
//! let store = get_object_store();
//! let path = Path::from("test");
//!
//! // Perform a conditional update on path
//! loop {
//!     // Perform get request
//!     let r = store.get(&path).await.unwrap();
//!
//!     // Save version information fetched
//!     let version = UpdateVersion {
//!         e_tag: r.meta.e_tag.clone(),
//!         version: r.meta.version.clone(),
//!     };
//!
//!     // Compute new version of object contents
//!     let new = do_update(r.bytes().await.unwrap());
//!
//!     // Attempt to commit transaction
//!     match store.put_opts(&path, new.into(), PutMode::Update(version).into()).await {
//!         Ok(_) => break, // Successfully committed
//!         Err(Error::Precondition { .. }) => continue, // Object has changed, try again
//!         Err(e) => panic!("{e}")
//!     }
//! }
//! # }
//! ```
//!
//! [Optimistic Concurrency Control]: https://en.wikipedia.org/wiki/Optimistic_concurrency_control
//! [Apache Iceberg]: https://iceberg.apache.org/
//! [Delta Lake]: https://delta.io/
//!
//! # Feature Flags
//!
//! The feature set is layered so that you can pick an object store
//! implementation, its HTTP transport, and its cryptography provider
//! independently:
//!
//! * `cloud-base` shared cloud implementation (XML/JSON parsing,
//!   credentials, retry, etc.) and intentionally does *not* depend on
//!   `reqwest` or a cryptography provider.
//! * `reqwest` enables the built-in [`reqwest`]-based [`HttpConnector`].
//! * `aws-lc-rs` and `ring` each provide a bundled [`client::CryptoProvider`].
//! * `<provider>-base` (`aws-base`, `azure-base`, `gcp-base`, `http-base`)
//!   adds the implementation specific logic on top of `cloud-base` without pulling in
//!   `reqwest` or a cryptography provider.
//! * `<provider>` (`aws`, `azure`, `gcp`, `http`) is the batteries-included
//!   feature for `<provider>-base` + `reqwest` (with `rustls`) + the default
//!   `aws-lc-rs` cryptography provider, and is the typical choice.
//!
//! ## Implementation specific features
//!
//! | Feature | Enables | Notes |
//! | --- | --- | --- |
//! | `aws` | `aws-base` + `reqwest` + `aws-lc-rs` | Amazon S3 with the built-in HTTP transport. |
//! | `azure` | `azure-base` + `reqwest` + `aws-lc-rs` | Azure Blob Storage with the built-in HTTP transport. |
//! | `gcp` | `gcp-base` + `reqwest` + `aws-lc-rs` | Google Cloud Storage with the built-in HTTP transport. |
//! | `http` | `http-base` + `reqwest` + `aws-lc-rs` | HTTP/WebDAV with the built-in HTTP transport. |
//! | `aws-base` |  | S3 without `reqwest` or crypto; supply your own [`HttpConnector`] and [`client::CryptoProvider`]. |
//! | `azure-base` |  | Azure without `reqwest` or crypto; supply your own [`HttpConnector`] and [`client::CryptoProvider`]. |
//! | `gcp-base` |  | GCS without `reqwest` or crypto; supply your own [`HttpConnector`] and [`client::CryptoProvider`]. |
//! | `http-base` |  | HTTP/WebDAV without `reqwest`; supply your own [`HttpConnector`]. |
//!
//! ## Transport and crypto features
//!
//! | Feature | Description |
//! | --- | --- |
//! | `reqwest` | Enables the [`reqwest`]-based [`HttpConnector`]. Enabled automatically by `aws`, `azure`, `gcp`, and `http`. |
//! | `aws-lc-rs` | Bundled [`aws-lc-rs`]-based [`client::CryptoProvider`]. The default for the batteries-included provider features. |
//! | `ring` | Bundled [`ring`]-based [`client::CryptoProvider`], e.g. for WASM targets. |
//! | `cloud-base` | Shared cloud-provider implementation. Enabled automatically by `*-base` features; usually not enabled directly. |
//!
//! ## Other features
//!
//! | Feature | Description |
//! | --- | --- |
//! | `fs` *(default)* | Local filesystem store via [`LocalFileSystem`](local::LocalFileSystem). |
//! | `tokio` | Enables Tokio-based utilities such as [`BufReader`](buffered::BufReader) and [`BufWriter`](buffered::BufWriter). Pulled in automatically by `fs` and the `*-base` features. |
//! | `integration` | Exposes the [`integration`] module, a reusable test suite for verifying custom [`ObjectStore`] implementations. Not API-stable. |
//!
//! ## Selecting a `reqwest` TLS backend
//!
//! `reqwest` needs a TLS backend to compile, so whenever you enable the `reqwest` feature directly
//! you must also enable one of `reqwest`'s TLS features:
//!
//! | reqwest feature | TLS stack | Notes |
//! | --- | --- | --- |
//! | `reqwest/rustls` | [rustls] with [`aws-lc-rs`] | enables `aws-lc-rs`. This is what `aws`/`azure`/`gcp`/`http` enable. |
//! | `reqwest/native-tls` | the platform's native TLS (OpenSSL / SChannel / Secure Transport) | enables neither `rustls` nor `aws-lc-rs`. |
//! | `reqwest/rustls-no-provider` | [rustls] with no bundled provider | enables neither provider; you must install one at runtime, e.g. `rustls::crypto::ring::default_provider().install_default()`. |
//!
//! ## Feature examples
//!
//! S3 implementation only; user provides the HTTP connector and crypto provider:
//! ```toml
//! object_store = { default-features = false, features = ["aws-base"] }
//! ```
//!
//! S3 implementation + `reqwest` + `aws-lc-rs` signing (equivalent to the `aws` feature):
//! ```toml
//! object_store = { default-features = false, features = ["aws-base", "reqwest", "reqwest/rustls", "aws-lc-rs"] }
//! ```
//!
//! S3 implementation + `reqwest` with native TLS + `ring` signing (no `aws-lc-rs` in the dependency tree):
//! ```toml
//! object_store = { default-features = false, features = ["aws-base", "reqwest", "reqwest/native-tls", "ring"] }
//! ```
//!
//! [rustls]: https://crates.io/crates/rustls/
//!
//! # Cryptography
//!
//! Request signing (e.g. AWS SigV4 or GCP service-account signing) requires a
//! [`client::CryptoProvider`]. The `aws`, `gcp`, and `azure` features
//! use [`aws-lc-rs`], matching `reqwest`'s default so that applications do not end up with
//! two crypto stacks.
//!
//! If you wish to use [`ring`] (e.g. to support WASM targets), use the
//! `*-base` feature flags, e.g. `aws-base`, and then enable the `ring` feature.
//!
//! When enabling the `aws-lc-rs` feature without the built-in `reqwest`
//! transport (e.g. `aws-base` + `aws-lc-rs` with a custom [`HttpConnector`]),
//! you must also select an [`aws-lc-rs`] backend, as `object_store` does not
//! pick one for you.
//! For example enable `aws-lc-rs/aws-lc-sys` (or `fips` / `non-fips`).
//!
//! If both `ring` and `aws-lc-rs` are enabled, `aws-lc-rs` is used by default.
//!
//! You can also implement a custom [`client::CryptoProvider`] to use your own cryptographic library.
//!
//! This signing provider is independent of the TLS crypto provider used by the
//! built-in `reqwest` transport — see
//! [Selecting a `reqwest` TLS backend](#selecting-a-reqwest-tls-backend). The
//! only combination that needs the provider registered manually (e.g.
//! `rustls::crypto::ring::default_provider().install_default()` in your `main`)
//! is `reqwest/rustls-no-provider`; `reqwest/rustls` and `reqwest/native-tls`
//! configure their TLS stack automatically.
//!
//! [`aws-lc-rs`]: https://crates.io/crates/aws-lc-rs/
//! [`ring`]: https://crates.io/crates/ring/
//!
//! # TLS Certificates
//!
//! Stores that use HTTPS/TLS (this is true for most cloud stores) can choose how certificates are validated.
//!
//! By default [`rustls-platform-verifier`] is used to verify certificates using the system's certificate
//! facilities. Alternatively, this functionality can be disabled using
//! [`ClientOptions::with_no_system_certificates`] and certificates manually registered using
//! [`ClientOptions::with_root_certificate`].
//!
//! These could be a custom CA chain, or alternatively an alternative trust store, e.g. [`webpki-roots`].
//!
//! ```ignore-wasm32
//! # #[cfg(feature = "aws")] {
//! use object_store::{ClientOptions, Certificate};
//!
//! let mut options = ClientOptions::default().with_no_system_certificates(true);
//! for root_cert in webpki_root_certs::TLS_SERVER_ROOT_CERTS {
//!     options = options.with_root_certificate(Certificate::from_der(root_cert.as_ref()).unwrap());
//! }
//! # }
//! ```
//!
//! [CA]: https://en.wikipedia.org/wiki/Certificate_authority
//! [`rustls-platform-verifier`]: https://crates.io/crates/rustls-platform-verifier/
//! [`webpki-roots`]: https://crates.io/crates/webpki-roots
//!
//! # Customizing HTTP Clients
//!
//! Many [`ObjectStore`] implementations permit customization of the HTTP client via
//! the [`HttpConnector`] trait and utilities in the [`client`] module.
//! Examples include injecting custom HTTP headers or using an alternate
//! tokio Runtime for I/O requests. To replace `reqwest` entirely (rather than
//! tweak the bundled transport) see [Disabling `reqwest`](#disabling-reqwest).
//!
//! [`HttpConnector`]: client::HttpConnector
//!
//! # Disabling `reqwest`
//!
//! The `aws`, `azure`, `gcp`, and `http` features each bundle a
//! [`reqwest`]-based HTTP transport, which is the right choice for most
//! applications. If you would rather supply your own HTTP client — for example
//! to share an existing client, to target a platform where `reqwest` does not
//! compile (such as `wasm32-wasip1`), or to keep `reqwest` out of your
//! dependency tree — use the matching `*-base` feature and provide an
//! [`HttpConnector`](client::HttpConnector) at builder time.
//!
//! Remember to disable the default features so that `fs` (and its transitive
//! dependencies) is not pulled in:
//!
//! ```toml
//! [dependencies]
//! object_store = { version = "0.13", default-features = false, features = ["aws-base"] }
//! ```
//!
//! ```ignore
//! use object_store::aws::AmazonS3Builder;
//!
//! let store = AmazonS3Builder::from_env()
//!     // `my_connector` is your own `impl HttpConnector`
//!     .with_http_connector(my_connector)
//!     .build()?;
//! ```
//!
//! See [Feature Flags](#feature-flags) above for the full set of flags.
//!
//! [`reqwest`]: https://crates.io/crates/reqwest

#[cfg(feature = "aws-base")]
pub mod aws;
#[cfg(feature = "azure-base")]
pub mod azure;
#[cfg(feature = "tokio")]
pub mod buffered;
#[cfg(not(target_arch = "wasm32"))]
pub mod chunked;
pub mod delimited;
#[cfg(feature = "gcp-base")]
pub mod gcp;
#[cfg(feature = "http-base")]
pub mod http;
#[cfg(feature = "tokio")]
pub mod limit;
#[cfg(all(feature = "fs", not(target_arch = "wasm32")))]
pub mod local;
pub mod memory;
pub mod path;
pub mod prefix;
pub mod registry;
#[cfg(any(feature = "aws-base", feature = "azure-base", feature = "gcp-base"))]
pub mod retry;
#[cfg(feature = "cloud-base")]
pub mod signer;
#[cfg(feature = "tokio")]
pub mod throttle;

#[cfg(feature = "cloud-base")]
pub mod client;

#[cfg(feature = "cloud-base")]
pub use client::{
    ClientConfigKey, ClientOptions, CredentialProvider, StaticCredentialProvider,
    backoff::BackoffConfig, retry::RetryConfig,
};

#[cfg(all(
    feature = "cloud-base",
    feature = "reqwest",
    not(target_arch = "wasm32")
))]
pub use client::Certificate;

#[cfg(feature = "cloud-base")]
mod config;

mod tags;

pub use tags::TagSet;

pub mod list;
pub mod multipart;
mod parse;
mod payload;
mod upload;
mod util;

mod attributes;

#[cfg(any(feature = "integration", test))]
pub mod integration;

pub use attributes::*;

pub use parse::{ObjectStoreScheme, parse_url, parse_url_opts};
pub use payload::*;
pub use upload::*;
pub use util::{GetRange, OBJECT_STORE_COALESCE_DEFAULT, coalesce_ranges, collect_bytes};

// Re-export HTTP types used in public API
pub use ::http::{Extensions, HeaderMap, HeaderValue};

use crate::path::Path;
#[cfg(all(feature = "fs", not(target_arch = "wasm32")))]
use crate::util::maybe_spawn_blocking;
use async_trait::async_trait;
use bytes::Bytes;
use chrono::{DateTime, Utc};
use futures_util::{StreamExt, TryStreamExt, stream::BoxStream};
use std::fmt::{Debug, Formatter};
use std::ops::Range;
use std::sync::Arc;

/// An alias for a dynamically dispatched object store implementation.
pub type DynObjectStore = dyn ObjectStore;

/// Id type for multipart uploads.
pub type MultipartId = String;

/// Universal API for object store services.
///
/// See the [module-level documentation](crate) for a high level overview and
/// examples. See [`ObjectStoreExt`] for additional convenience methods.
///
/// # Contract
/// This trait is a contract between object store _implementations_
/// (e.g. providers, wrappers) and the `object_store` crate itself. It is
/// intended to be the minimum API required for an object store.
///
/// The [`ObjectStoreExt`] acts as an API/contract between `object_store` and
/// the store _users_ and provides additional methods that may be simpler to use
/// but overlap in functionality with [`ObjectStore`].
///
/// # Clone
/// If a store implements [`Clone`], that will only clone the handle to the underlying data. It will NOT clone/fork the
/// actual key-value data. Hence, the cloned instance and the original instance share the same state.
///
/// # Minimal Default Implementations
/// There are only a few default implementations for methods in this trait by
/// design. This was different from versions prior to `0.13.0`, which had many
/// more default implementations. Default implementations are convenient for
/// users, but error-prone for implementors as they require keeping the
/// convenience APIs correctly in sync.
///
/// As of version 0.13.0, most methods on [`ObjectStore`] must be implemented, and
/// the convenience methods have been moved to the [`ObjectStoreExt`] trait as
/// described above. See [#385] for more details.
///
/// [#385]: https://github.com/apache/arrow-rs-object-store/issues/385
///
/// # Wrappers
/// If you wrap an [`ObjectStore`] -- e.g. to add observability -- you SHOULD
/// implement all trait methods. This ensures that defaults implementations
/// that are overwritten by the wrapped store are also used by the wrapper.
/// For example:
///
/// ```ignore
/// struct MyStore {
///     ...
/// }
///
/// #[async_trait]
/// impl ObjectStore for MyStore {
///     // implement custom ranges handling
///     async fn get_ranges(
///         &self,
///         location: &Path,
///         ranges: &[Range<u64>],
///     ) -> Result<Vec<Bytes>> {
///         ...
///     }
///
///     ...
/// }
///
/// struct Wrapper {
///     inner: Arc<dyn ObjectStore>,
/// }
///
/// #[async_trait]
/// #[deny(clippy::missing_trait_methods)]
/// impl ObjectStore for Wrapper {
///     // If we would not implement this method,
///     // we would get the trait default and not
///     // use the actual implementation of `inner`.
///     async fn get_ranges(
///         &self,
///         location: &Path,
///         ranges: &[Range<u64>],
///     ) -> Result<Vec<Bytes>> {
///         ...
///     }
///
///     ...
/// }
/// ```
///
/// To automatically detect this issue, use
/// [`#[deny(clippy::missing_trait_methods)]`](https://rust-lang.github.io/rust-clippy/master/index.html#missing_trait_methods).
///
/// # Upgrade Guide for 0.13.0
///
/// Upgrading to object_store 0.13.0 from an earlier version typically involves:
///
/// 1. Add a `use` for [`ObjectStoreExt`] to solve the error
///
/// ```text
/// error[E0599]: no method named `put` found for reference `&dyn object_store::ObjectStore` in the current scope
///    --> datafusion/datasource/src/url.rs:993:14
/// ```
///
/// 2. Remove any (now) redundant implementations (such as `ObjectStore::put`) from any
///   `ObjectStore` implementations to resolve the error
///
/// ```text
/// error[E0407]: method `put` is not a member of trait `ObjectStore`
///     --> datafusion/datasource/src/url.rs:1103:9
///      |
/// ```
///
/// 3. Convert `ObjectStore::delete` to [`ObjectStore::delete_stream`] (see documentation
///    on that method for details and examples)
///
/// 4. Combine `ObjectStore::copy` and `ObjectStore::copy_if_not_exists` implementations into
///    [`ObjectStore::copy_opts`] (see documentation on that method for details and examples)
///
/// 5. Update `object_store::Error::NotImplemented` to include the name of the missing method
///
/// For example, change instances of
/// ```text
/// object_store::Error::NotImplemented
/// ```
/// to
/// ```
/// object_store::Error::NotImplemented {
///    operation: "put".to_string(),
///    implementer: "RequestCountingObjectStore".to_string(),
///  };
/// ```
///
#[async_trait]
pub trait ObjectStore: std::fmt::Display + Send + Sync + Debug + 'static {
    /// Save the provided `payload` to `location` with the given options
    ///
    /// The operation is guaranteed to be atomic, it will either successfully
    /// write the entirety of `payload` to `location`, or fail. No clients
    /// should be able to observe a partially written object
    ///
    /// To upload large objects without buffering them entirely in memory, use
    /// the multipart API: [`ObjectStore::put_multipart_opts`]
    async fn put_opts(
        &self,
        location: &Path,
        payload: PutPayload,
        opts: PutOptions,
    ) -> Result<PutResult>;

    /// Perform a multipart upload with options
    ///
    /// Client should prefer [`ObjectStore::put_opts`] for small payloads, as streaming uploads
    /// typically require multiple separate requests. See [`MultipartUpload`] for more information
    ///
    /// See also [`BufWriter`](buffered::BufWriter) for an interface that
    /// automatically picks between a single and multipart upload based on the
    /// amount of data written.
    ///
    /// For more advanced multipart uploads see [`MultipartStore`](multipart::MultipartStore)
    ///
    /// [`BufWriter`]: buffered::BufWriter
    async fn put_multipart_opts(
        &self,
        location: &Path,
        opts: PutMultipartOptions,
    ) -> Result<Box<dyn MultipartUpload>>;

    /// Perform a get request with options
    ///
    /// ## Example
    ///
    /// This example uses a basic local filesystem object store to get an object with a specific etag.
    /// On the local filesystem, supplying an invalid etag will error.
    /// Versioned object stores will return the specified object version, if it exists.
    ///
    /// ```ignore-wasm32
    /// # use object_store::local::LocalFileSystem;
    /// # use tempfile::tempdir;
    /// # use object_store::{path::Path, ObjectStore, ObjectStoreExt, GetOptions};
    /// async fn get_opts_example() {
    ///     let tmp = tempdir().unwrap();
    ///     let store = LocalFileSystem::new_with_prefix(tmp.path()).unwrap();
    ///     let location = Path::from("example.txt");
    ///     let content = b"Hello, Object Store!";
    ///
    ///     // Put the object into the store
    ///     store
    ///         .put(&location, content.as_ref().into())
    ///         .await
    ///         .expect("Failed to put object");
    ///
    ///     // Get the object from the store to figure out the right etag
    ///     let result: object_store::GetResult = store.get(&location).await.expect("Failed to get object");
    ///
    ///     let etag = result.meta.e_tag.expect("ETag should be present");
    ///
    ///     // Get the object from the store with range and etag
    ///     let bytes = store
    ///         .get_opts(
    ///             &location,
    ///             GetOptions::new()
    ///                 .with_if_match(Some(etag.clone())),
    ///         )
    ///         .await
    ///         .expect("Failed to get object with range and etag")
    ///         .bytes()
    ///         .await
    ///         .expect("Failed to read bytes");
    ///
    ///     println!(
    ///         "Retrieved with ETag {}: {}",
    ///         etag,
    ///         String::from_utf8_lossy(&bytes)
    ///     );
    ///
    ///     // Show that if the etag does not match, we get an error
    ///     let wrong_etag = "wrong-etag".to_string();
    ///     match store
    ///         .get_opts(
    ///             &location,
    ///             GetOptions::new().with_if_match(Some(wrong_etag))
    ///         )
    ///         .await
    ///     {
    ///         Ok(_) => println!("Unexpectedly succeeded with wrong ETag"),
    ///         Err(e) => println!("On a non-versioned object store, getting an invalid ETag ('wrong-etag') results in an error as expected: {}", e),
    ///     }
    /// }
    /// ```
    ///
    /// To retrieve a range of bytes from a versioned object, specify the range in the [`GetOptions`] supplied to this method.
    ///
    /// ```ignore-wasm32
    /// # use object_store::local::LocalFileSystem;
    /// # use tempfile::tempdir;
    /// # use object_store::{path::Path, ObjectStore, ObjectStoreExt, GetOptions};
    /// async fn get_opts_range_example() {
    ///     let tmp = tempdir().unwrap();
    ///     let store = LocalFileSystem::new_with_prefix(tmp.path()).unwrap();
    ///     let location = Path::from("example.txt");
    ///     let content = b"Hello, Object Store!";
    ///
    ///     // Put the object into the store
    ///     store
    ///         .put(&location, content.as_ref().into())
    ///         .await
    ///         .expect("Failed to put object");
    ///
    ///     // Get the object from the store to figure out the right etag
    ///     let result: object_store::GetResult = store.get(&location).await.expect("Failed to get object");
    ///
    ///     let etag = result.meta.e_tag.expect("ETag should be present");
    ///
    ///     // Get the object from the store with range and etag
    ///     let bytes = store
    ///         .get_opts(
    ///             &location,
    ///             GetOptions::new()
    ///                 .with_range(Some(0..5))
    ///                 .with_if_match(Some(etag.clone())),
    ///         )
    ///         .await
    ///         .expect("Failed to get object with range and etag")
    ///         .bytes()
    ///         .await
    ///         .expect("Failed to read bytes");
    ///
    ///     println!(
    ///         "Retrieved range [0-5] with ETag {}: {}",
    ///         etag,
    ///         String::from_utf8_lossy(&bytes)
    ///     );
    ///
    ///     // Show that if the etag does not match, we get an error
    ///     let wrong_etag = "wrong-etag".to_string();
    ///     match store
    ///         .get_opts(
    ///             &location,
    ///             GetOptions::new().with_range(Some(0..5)).with_if_match(Some(wrong_etag))
    ///         )
    ///         .await
    ///     {
    ///         Ok(_) => println!("Unexpectedly succeeded with wrong ETag"),
    ///         Err(e) => println!("On a non-versioned object store, getting an invalid ETag ('wrong-etag') results in an error as expected: {}", e),
    ///     }
    /// }
    /// ```
    async fn get_opts(&self, location: &Path, options: GetOptions) -> Result<GetResult>;

    /// Return the bytes that are stored at the specified location
    /// in the given byte ranges
    async fn get_ranges(&self, location: &Path, ranges: &[Range<u64>]) -> Result<Vec<Bytes>> {
        coalesce_ranges(
            ranges,
            |range| self.get_range(location, range),
            OBJECT_STORE_COALESCE_DEFAULT,
        )
        .await
    }

    /// Atomically delete the object only if its ETag matches `etag`.
    ///
    /// A changed object returns [`Error::Precondition`]. Missing objects may return
    /// success or [`Error::NotFound`]. Implementations must fail closed if unsupported;
    /// checking an ETag with GET followed by an unconditional DELETE is not safe.
    async fn delete_if_match(&self, _location: &Path, _etag: &str) -> Result<()> {
        Err(Error::NotImplemented {
            operation: "delete_if_match".into(),
            implementer: self.to_string(),
        })
    }

    /// Delete all the objects at the specified locations
    ///
    /// When supported, this method will use bulk operations that delete more
    /// than one object per a request. Otherwise, the implementation may call
    /// the single object delete method for each location.
    ///
    /// # Bulk Delete Support
    ///
    /// The following backends support native bulk delete operations:
    ///
    /// - **AWS (S3)**: Uses the native [DeleteObjects] API with batches of up to 1000 objects
    /// - **Azure**: Uses the native [Blob Batch] API with batches of up to 256 objects
    ///
    /// The following backends use concurrent individual delete operations:
    ///
    /// - **GCP**: Performs individual delete requests with up to 10 concurrent operations
    /// - **HTTP**: Performs individual delete requests with up to 10 concurrent operations
    /// - **Local**: Performs individual file deletions with up to 10 concurrent operations
    /// - **Memory**: Performs individual in-memory deletions sequentially
    ///
    /// [DeleteObjects]: https://docs.aws.amazon.com/AmazonS3/latest/API/API_DeleteObjects.html
    /// [Blob Batch]: https://learn.microsoft.com/en-us/rest/api/storageservices/blob-batch
    ///
    /// The returned stream yields the results of the delete operations in the
    /// same order as the input locations. However, some errors will be from
    /// an overall call to a bulk delete operation, and not from a specific
    /// location.
    ///
    /// If the object did not exist, the result may be an error or a success,
    /// depending on the behavior of the underlying store. For example, local
    /// filesystems, GCP, and Azure return an error, while S3 and in-memory will
    /// return Ok. If it is an error, it will be [`Error::NotFound`].
    ///
    /// ```ignore-wasm32
    /// # use futures_util::{StreamExt, TryStreamExt};
    /// # use object_store::local::LocalFileSystem;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let root = tempfile::TempDir::new().unwrap();
    /// # let store = LocalFileSystem::new_with_prefix(root.path()).unwrap();
    /// # use object_store::{ObjectStore, ObjectStoreExt, ObjectMeta};
    /// # use object_store::path::Path;
    /// # use futures_util::{StreamExt, TryStreamExt};
    /// #
    /// // Create two objects
    /// store.put(&Path::from("foo"), "foo".into()).await?;
    /// store.put(&Path::from("bar"), "bar".into()).await?;
    ///
    /// // List object
    /// let locations = store.list(None).map_ok(|m| m.location).boxed();
    ///
    /// // Delete them
    /// store.delete_stream(locations).try_collect::<Vec<Path>>().await?;
    /// # Ok(())
    /// # }
    /// # let rt = tokio::runtime::Builder::new_current_thread().build().unwrap();
    /// # rt.block_on(example()).unwrap();
    /// ```
    ///
    /// Note: Before version 0.13, `delete_stream` has a default implementation
    /// that deletes each object with up to 10 concurrent requests. This default
    /// behavior has been removed, and each implementation must now provide its
    /// own `delete_stream` implementation explicitly. The following example
    /// shows how to implement `delete_stream` to get the previous default
    /// behavior.
    ///
    /// ```
    /// # use async_trait::async_trait;
    /// # use futures_util::stream::{BoxStream, StreamExt};
    /// # use object_store::path::Path;
    /// # use object_store::{
    /// #     CopyOptions, GetOptions, GetResult, ListResult, MultipartUpload, ObjectMeta, ObjectStore,
    /// #     PutMultipartOptions, PutOptions, PutPayload, PutResult, Result,
    /// # };
    /// # use std::fmt;
    /// # use std::fmt::Debug;
    /// # use std::sync::Arc;
    /// #
    /// # struct ExampleClient;
    /// #
    /// # impl ExampleClient {
    /// #     async fn delete(&self, _path: &Path) -> Result<()> {
    /// #         Ok(())
    /// #     }
    /// # }
    /// #
    /// # struct ExampleStore {
    /// #     client: Arc<ExampleClient>,
    /// # }
    /// #
    /// # impl Debug for ExampleStore {
    /// #     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    /// #         write!(f, "ExampleStore")
    /// #     }
    /// # }
    /// #
    /// # impl fmt::Display for ExampleStore {
    /// #     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    /// #         write!(f, "ExampleStore")
    /// #     }
    /// # }
    /// #
    /// # #[async_trait]
    /// # impl ObjectStore for ExampleStore {
    /// #     async fn put_opts(&self, _: &Path, _: PutPayload, _: PutOptions) -> Result<PutResult> {
    /// #         todo!()
    /// #     }
    /// #
    /// #     async fn put_multipart_opts(
    /// #         &self,
    /// #         _: &Path,
    /// #         _: PutMultipartOptions,
    /// #     ) -> Result<Box<dyn MultipartUpload>> {
    /// #         todo!()
    /// #     }
    /// #
    /// #     async fn get_opts(&self, _: &Path, _: GetOptions) -> Result<GetResult> {
    /// #         todo!()
    /// #     }
    /// #
    /// fn delete_stream(
    ///     &self,
    ///     locations: BoxStream<'static, Result<Path>>,
    /// ) -> BoxStream<'static, Result<Path>> {
    ///     let client = Arc::clone(&self.client);
    ///     locations
    ///         .map(move |location| {
    ///             let client = Arc::clone(&client);
    ///             async move {
    ///                 let location = location?;
    ///                 client.delete(&location).await?;
    ///                 Ok(location)
    ///             }
    ///         })
    ///         .buffered(10)
    ///         .boxed()
    /// }
    /// #
    /// #     fn list(&self, _: Option<&Path>) -> BoxStream<'static, Result<ObjectMeta>> {
    /// #         todo!()
    /// #     }
    /// #
    /// #     async fn list_with_delimiter(&self, _: Option<&Path>) -> Result<ListResult> {
    /// #         todo!()
    /// #     }
    /// #
    /// #     async fn copy_opts(&self, _: &Path, _: &Path, _: CopyOptions) -> Result<()> {
    /// #         todo!()
    /// #     }
    /// # }
    /// #
    /// # async fn example() {
    /// #     let store = ExampleStore { client: Arc::new(ExampleClient) };
    /// #     let paths = futures_util::stream::iter(vec![Ok(Path::from("foo")), Ok(Path::from("bar"))]).boxed();
    /// #     let results = store.delete_stream(paths).collect::<Vec<_>>().await;
    /// #     assert_eq!(results.len(), 2);
    /// #     assert_eq!(results[0].as_ref().unwrap(), &Path::from("foo"));
    /// #     assert_eq!(results[1].as_ref().unwrap(), &Path::from("bar"));
    /// # }
    /// #
    /// # let rt = tokio::runtime::Builder::new_current_thread().build().unwrap();
    /// # rt.block_on(example());
    /// ```
    fn delete_stream(
        &self,
        locations: BoxStream<'static, Result<Path>>,
    ) -> BoxStream<'static, Result<Path>>;

    /// List all the objects with the given prefix.
    ///
    /// Prefixes are evaluated on a path segment basis, i.e. `foo/bar` is a prefix of `foo/bar/x` but not of
    /// `foo/bar_baz/x`. List is recursive, i.e. `foo/bar/more/x` will be included.
    ///
    /// Note: the order of returned [`ObjectMeta`] is not guaranteed
    ///
    /// For more advanced listing see [`PaginatedListStore`](list::PaginatedListStore)
    fn list(&self, prefix: Option<&Path>) -> BoxStream<'static, Result<ObjectMeta>>;

    /// List all the objects with the given prefix and a location greater than `offset`
    ///
    /// Some stores, such as S3 and GCS, may be able to push `offset` down to reduce
    /// the number of network requests required.
    ///
    /// This returns an exclusive offset, i.e. objects at exactly `offset` will not be included.
    ///
    /// Note: the order of returned [`ObjectMeta`] is not guaranteed
    ///
    /// For more advanced listing see [`PaginatedListStore`](list::PaginatedListStore)
    fn list_with_offset(
        &self,
        prefix: Option<&Path>,
        offset: &Path,
    ) -> BoxStream<'static, Result<ObjectMeta>> {
        let offset = offset.clone();
        self.list(prefix)
            .try_filter(move |f| futures_util::future::ready(f.location > offset))
            .boxed()
    }

    /// List objects with the given prefix and an implementation specific
    /// delimiter. Returns common prefixes (directories) in addition to object
    /// metadata.
    ///
    /// Prefixes are evaluated on a path segment basis, i.e. `foo/bar` is a prefix of `foo/bar/x` but not of
    /// `foo/bar_baz/x`. List is not recursive, i.e. `foo/bar/more/x` will not be included.
    async fn list_with_delimiter(&self, prefix: Option<&Path>) -> Result<ListResult>;

    /// Copy an object from one path to another in the same object store.
    async fn copy_opts(&self, from: &Path, to: &Path, options: CopyOptions) -> Result<()>;

    /// Move an object from one path to another in the same object store.
    ///
    /// By default, this is implemented as a copy and then delete source. It may not
    /// check when deleting source that it was the same object that was originally copied.
    async fn rename_opts(&self, from: &Path, to: &Path, options: RenameOptions) -> Result<()> {
        let RenameOptions {
            target_mode,
            extensions,
        } = options;
        let copy_mode = match target_mode {
            RenameTargetMode::Overwrite => CopyMode::Overwrite,
            RenameTargetMode::Create => CopyMode::Create,
        };
        let copy_options = CopyOptions {
            mode: copy_mode,
            extensions,
        };
        self.copy_opts(from, to, copy_options).await?;
        self.delete(from).await?;
        Ok(())
    }
}

macro_rules! as_ref_impl {
    ($type:ty) => {
        #[async_trait]
        #[deny(clippy::missing_trait_methods)]
        impl<T: ObjectStore + ?Sized> ObjectStore for $type {
            async fn put_opts(
                &self,
                location: &Path,
                payload: PutPayload,
                opts: PutOptions,
            ) -> Result<PutResult> {
                self.as_ref().put_opts(location, payload, opts).await
            }

            async fn put_multipart_opts(
                &self,
                location: &Path,
                opts: PutMultipartOptions,
            ) -> Result<Box<dyn MultipartUpload>> {
                self.as_ref().put_multipart_opts(location, opts).await
            }

            async fn get_opts(&self, location: &Path, options: GetOptions) -> Result<GetResult> {
                self.as_ref().get_opts(location, options).await
            }

            async fn get_ranges(
                &self,
                location: &Path,
                ranges: &[Range<u64>],
            ) -> Result<Vec<Bytes>> {
                self.as_ref().get_ranges(location, ranges).await
            }

            async fn delete_if_match(&self, location: &Path, etag: &str) -> Result<()> {
                self.as_ref().delete_if_match(location, etag).await
            }

            fn delete_stream(
                &self,
                locations: BoxStream<'static, Result<Path>>,
            ) -> BoxStream<'static, Result<Path>> {
                self.as_ref().delete_stream(locations)
            }

            fn list(&self, prefix: Option<&Path>) -> BoxStream<'static, Result<ObjectMeta>> {
                self.as_ref().list(prefix)
            }

            fn list_with_offset(
                &self,
                prefix: Option<&Path>,
                offset: &Path,
            ) -> BoxStream<'static, Result<ObjectMeta>> {
                self.as_ref().list_with_offset(prefix, offset)
            }

            async fn list_with_delimiter(&self, prefix: Option<&Path>) -> Result<ListResult> {
                self.as_ref().list_with_delimiter(prefix).await
            }

            async fn copy_opts(&self, from: &Path, to: &Path, options: CopyOptions) -> Result<()> {
                self.as_ref().copy_opts(from, to, options).await
            }

            async fn rename_opts(
                &self,
                from: &Path,
                to: &Path,
                options: RenameOptions,
            ) -> Result<()> {
                self.as_ref().rename_opts(from, to, options).await
            }
        }
    };
}

as_ref_impl!(Arc<T>);
as_ref_impl!(Box<T>);

/// Extension trait for [`ObjectStore`] with convenience functions.
///
/// See the [module-level documentation](crate) for a high level overview and
/// examples. See "contract" section within the [`ObjectStore`] documentation
/// for more reasoning.
///
/// # Implementation
/// You MUST NOT implement this trait yourself. It is automatically implemented for all [`ObjectStore`] implementations.
pub trait ObjectStoreExt: ObjectStore {
    /// Save the provided bytes to the specified location
    ///
    /// The operation is guaranteed to be atomic, it will either successfully
    /// write the entirety of `payload` to `location`, or fail. No clients
    /// should be able to observe a partially written object
    ///
    /// Note the entire `payload` is buffered in memory. To upload large objects
    /// without buffering them entirely in memory, use the multipart API:
    /// [`ObjectStoreExt::put_multipart`], [`WriteMultipart`], or
    /// [`BufWriter`](buffered::BufWriter)
    fn put(&self, location: &Path, payload: PutPayload) -> impl Future<Output = Result<PutResult>>;

    /// Perform a multipart upload
    ///
    /// Client should prefer [`ObjectStoreExt::put`] for small payloads, as streaming uploads
    /// typically require multiple separate requests. See [`MultipartUpload`] for more information
    ///
    /// For more advanced multipart uploads see [`MultipartStore`](multipart::MultipartStore)
    fn put_multipart(
        &self,
        location: &Path,
    ) -> impl Future<Output = Result<Box<dyn MultipartUpload>>>;

    /// Return the bytes that are stored at the specified location.
    ///
    /// ## Example
    ///
    /// This example uses a basic local filesystem object store to get an object.
    ///
    /// ```ignore-wasm32
    /// # use object_store::local::LocalFileSystem;
    /// # use tempfile::tempdir;
    /// # use object_store::{path::Path, ObjectStore, ObjectStoreExt};
    /// async fn get_example() {
    ///     let tmp = tempdir().unwrap();
    ///     let store = LocalFileSystem::new_with_prefix(tmp.path()).unwrap();
    ///     let location = Path::from("example.txt");
    ///     let content = b"Hello, Object Store!";
    ///
    ///     // Put the object into the store
    ///     store
    ///         .put(&location, content.as_ref().into())
    ///         .await
    ///         .expect("Failed to put object");
    ///
    ///     // Get the object from the store
    ///     let get_result = store.get(&location).await.expect("Failed to get object");
    ///     let bytes = get_result.bytes().await.expect("Failed to read bytes");
    ///     println!("Retrieved content: {}", String::from_utf8_lossy(&bytes));
    /// }
    /// ```
    fn get(&self, location: &Path) -> impl Future<Output = Result<GetResult>>;

    /// Return the bytes that are stored at the specified location
    /// in the given byte range.
    ///
    /// See [`GetRange::Bounded`] for more details on how `range` gets interpreted.
    ///
    /// To retrieve a range of bytes from a versioned object, use [`ObjectStore::get_opts`] by specifying the range in the [`GetOptions`].
    ///
    /// ## Examples
    ///
    /// This example uses a basic local filesystem object store to get a byte range from an object.
    ///
    /// ```ignore-wasm32
    /// # use object_store::local::LocalFileSystem;
    /// # use tempfile::tempdir;
    /// # use object_store::{path::Path, ObjectStore, ObjectStoreExt};
    /// async fn get_range_example() {
    ///     let tmp = tempdir().unwrap();
    ///     let store = LocalFileSystem::new_with_prefix(tmp.path()).unwrap();
    ///     let location = Path::from("example.txt");
    ///     let content = b"Hello, Object Store!";
    ///
    ///     // Put the object into the store
    ///     store
    ///         .put(&location, content.as_ref().into())
    ///         .await
    ///         .expect("Failed to put object");
    ///
    ///     // Get the object from the store
    ///     let bytes = store
    ///         .get_range(&location, 0..5)
    ///         .await
    ///         .expect("Failed to get object");
    ///     println!("Retrieved range [0-5]: {}", String::from_utf8_lossy(&bytes));
    /// }
    /// ```
    fn get_range(&self, location: &Path, range: Range<u64>) -> impl Future<Output = Result<Bytes>>;

    /// Return the metadata for the specified location
    fn head(&self, location: &Path) -> impl Future<Output = Result<ObjectMeta>>;

    /// Delete the object at the specified location.
    fn delete(&self, location: &Path) -> impl Future<Output = Result<()>>;

    /// Copy an object from one path to another in the same object store.
    ///
    /// If there exists an object at the destination, it will be overwritten.
    fn copy(&self, from: &Path, to: &Path) -> impl Future<Output = Result<()>>;

    /// Copy an object from one path to another, only if destination is empty.
    ///
    /// Will return an error if the destination already has an object.
    ///
    /// Performs an atomic operation if the underlying object storage supports it.
    /// If atomic operations are not supported by the underlying object storage (like S3)
    /// it will return an error.
    fn copy_if_not_exists(&self, from: &Path, to: &Path) -> impl Future<Output = Result<()>>;

    /// Move an object from one path to another in the same object store.
    ///
    /// By default, this is implemented as a copy and then delete source. It may not
    /// check when deleting source that it was the same object that was originally copied.
    ///
    /// If there exists an object at the destination, it will be overwritten.
    fn rename(&self, from: &Path, to: &Path) -> impl Future<Output = Result<()>>;

    /// Move an object from one path to another in the same object store.
    ///
    /// Will return an error if the destination already has an object.
    fn rename_if_not_exists(&self, from: &Path, to: &Path) -> impl Future<Output = Result<()>>;
}

impl<T> ObjectStoreExt for T
where
    T: ObjectStore + ?Sized,
{
    async fn put(&self, location: &Path, payload: PutPayload) -> Result<PutResult> {
        self.put_opts(location, payload, PutOptions::default())
            .await
    }

    async fn put_multipart(&self, location: &Path) -> Result<Box<dyn MultipartUpload>> {
        self.put_multipart_opts(location, PutMultipartOptions::default())
            .await
    }

    async fn get(&self, location: &Path) -> Result<GetResult> {
        self.get_opts(location, GetOptions::default()).await
    }

    async fn get_range(&self, location: &Path, range: Range<u64>) -> Result<Bytes> {
        let options = GetOptions::new().with_range(Some(range));
        self.get_opts(location, options).await?.bytes().await
    }

    async fn head(&self, location: &Path) -> Result<ObjectMeta> {
        let options = GetOptions::new().with_head(true);
        Ok(self.get_opts(location, options).await?.meta)
    }

    async fn delete(&self, location: &Path) -> Result<()> {
        let location = location.clone();
        let mut stream =
            self.delete_stream(futures_util::stream::once(async move { Ok(location) }).boxed());
        let _path = stream.try_next().await?.ok_or_else(|| Error::Generic {
            store: "ext",
            source: "`delete_stream` with one location should yield once but didn't".into(),
        })?;
        if stream.next().await.is_some() {
            Err(Error::Generic {
                store: "ext",
                source: "`delete_stream` with one location expected to yield exactly once, but yielded more than once".into(),
            })
        } else {
            Ok(())
        }
    }

    async fn copy(&self, from: &Path, to: &Path) -> Result<()> {
        let options = CopyOptions::new().with_mode(CopyMode::Overwrite);
        self.copy_opts(from, to, options).await
    }

    async fn copy_if_not_exists(&self, from: &Path, to: &Path) -> Result<()> {
        let options = CopyOptions::new().with_mode(CopyMode::Create);
        self.copy_opts(from, to, options).await
    }

    async fn rename(&self, from: &Path, to: &Path) -> Result<()> {
        let options = RenameOptions::new().with_target_mode(RenameTargetMode::Overwrite);
        self.rename_opts(from, to, options).await
    }

    async fn rename_if_not_exists(&self, from: &Path, to: &Path) -> Result<()> {
        let options = RenameOptions::new().with_target_mode(RenameTargetMode::Create);
        self.rename_opts(from, to, options).await
    }
}

/// Result of a list call that includes objects, prefixes (directories) and a
/// token for the next set of results. Individual result sets may be limited to
/// 1,000 objects based on the underlying object storage's limitations.
#[derive(Debug)]
pub struct ListResult {
    /// Prefixes that are common (like directories)
    pub common_prefixes: Vec<Path>,
    /// Object metadata for the listing
    pub objects: Vec<ObjectMeta>,
    /// Implementation-specific extensions. Intended for use by [`ObjectStore`] implementations
    /// that need to return context-specific information (like cache status) from trait methods.
    ///
    /// HTTP-backed stores in this crate populate this with the extensions of the HTTP
    /// response, allowing custom HTTP middleware to propagate information to callers.
    /// Where a result is assembled from multiple paginated requests, the extensions of
    /// each response are merged, with those of later responses taking precedence.
    pub extensions: Extensions,
}

/// The metadata that describes an object.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectMeta {
    /// The full path to the object
    pub location: Path,
    /// The last modified time
    pub last_modified: DateTime<Utc>,
    /// The size in bytes of the object.
    ///
    /// Note this is not `usize` as `object_store` supports 32-bit architectures such as WASM
    pub size: u64,
    /// The unique identifier for the object
    ///
    /// <https://datatracker.ietf.org/doc/html/rfc9110#name-etag>
    pub e_tag: Option<String>,
    /// A version indicator for this object
    pub version: Option<String>,
}

/// Options for a get request, such as range
#[derive(Debug, Default, Clone)]
pub struct GetOptions {
    /// Request will succeed if the `ObjectMeta::e_tag` matches
    /// otherwise returning [`Error::Precondition`]
    ///
    /// See <https://datatracker.ietf.org/doc/html/rfc9110#name-if-match>
    ///
    /// Examples:
    ///
    /// ```text
    /// If-Match: "xyzzy"
    /// If-Match: "xyzzy", "r2d2xxxx", "c3piozzzz"
    /// If-Match: *
    /// ```
    pub if_match: Option<String>,
    /// Request will succeed if the `ObjectMeta::e_tag` does not match
    /// otherwise returning [`Error::NotModified`]
    ///
    /// See <https://datatracker.ietf.org/doc/html/rfc9110#section-13.1.2>
    ///
    /// Examples:
    ///
    /// ```text
    /// If-None-Match: "xyzzy"
    /// If-None-Match: "xyzzy", "r2d2xxxx", "c3piozzzz"
    /// If-None-Match: *
    /// ```
    pub if_none_match: Option<String>,
    /// Request will succeed if the object has been modified since
    ///
    /// <https://datatracker.ietf.org/doc/html/rfc9110#section-13.1.3>
    pub if_modified_since: Option<DateTime<Utc>>,
    /// Request will succeed if the object has not been modified since
    /// otherwise returning [`Error::Precondition`]
    ///
    /// Some stores, such as S3, will only return `NotModified` for exact
    /// timestamp matches, instead of for any timestamp greater than or equal.
    ///
    /// <https://datatracker.ietf.org/doc/html/rfc9110#section-13.1.4>
    pub if_unmodified_since: Option<DateTime<Utc>>,
    /// Request transfer of only the specified range of bytes
    /// otherwise returning [`Error::NotModified`]
    ///
    /// <https://datatracker.ietf.org/doc/html/rfc9110#name-range>
    pub range: Option<GetRange>,
    /// Request a particular object version
    pub version: Option<String>,
    /// Request transfer of no content
    ///
    /// <https://datatracker.ietf.org/doc/html/rfc9110#name-head>
    pub head: bool,
    /// Implementation-specific extensions. Intended for use by [`ObjectStore`] implementations
    /// that need to pass context-specific information (like tracing spans) via trait methods.
    ///
    /// These extensions are ignored entirely by backends offered through this crate.
    pub extensions: Extensions,
}

impl GetOptions {
    /// Returns an error if the modification conditions on this request are not satisfied
    ///
    /// <https://datatracker.ietf.org/doc/html/rfc7232#section-6>
    pub fn check_preconditions(&self, meta: &ObjectMeta) -> Result<()> {
        // The use of the invalid etag "*" means no ETag is equivalent to never matching
        let etag = meta.e_tag.as_deref().unwrap_or("*");
        let last_modified = meta.last_modified;

        if let Some(m) = &self.if_match {
            if m != "*" && m.split(',').map(str::trim).all(|x| x != etag) {
                return Err(Error::Precondition {
                    path: meta.location.to_string(),
                    source: format!("{etag} does not match {m}").into(),
                });
            }
        } else if let Some(date) = self.if_unmodified_since {
            if last_modified > date {
                return Err(Error::Precondition {
                    path: meta.location.to_string(),
                    source: format!("{date} < {last_modified}").into(),
                });
            }
        }

        if let Some(m) = &self.if_none_match {
            if m == "*" || m.split(',').map(str::trim).any(|x| x == etag) {
                return Err(Error::NotModified {
                    path: meta.location.to_string(),
                    source: format!("{etag} matches {m}").into(),
                });
            }
        } else if let Some(date) = self.if_modified_since {
            if last_modified <= date {
                return Err(Error::NotModified {
                    path: meta.location.to_string(),
                    source: format!("{date} >= {last_modified}").into(),
                });
            }
        }
        Ok(())
    }

    /// Create a new [`GetOptions`]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the `if_match` condition.
    ///
    /// See [`GetOptions::if_match`]
    #[must_use]
    pub fn with_if_match(mut self, etag: Option<impl Into<String>>) -> Self {
        self.if_match = etag.map(Into::into);
        self
    }

    /// Sets the `if_none_match` condition.
    ///
    /// See [`GetOptions::if_none_match`]
    #[must_use]
    pub fn with_if_none_match(mut self, etag: Option<impl Into<String>>) -> Self {
        self.if_none_match = etag.map(Into::into);
        self
    }

    /// Sets the `if_modified_since` condition.
    ///
    /// See [`GetOptions::if_modified_since`]
    #[must_use]
    pub fn with_if_modified_since(mut self, dt: Option<impl Into<DateTime<Utc>>>) -> Self {
        self.if_modified_since = dt.map(Into::into);
        self
    }

    /// Sets the `if_unmodified_since` condition.
    ///
    /// See [`GetOptions::if_unmodified_since`]
    #[must_use]
    pub fn with_if_unmodified_since(mut self, dt: Option<impl Into<DateTime<Utc>>>) -> Self {
        self.if_unmodified_since = dt.map(Into::into);
        self
    }

    /// Sets the `range` condition.
    ///
    /// See [`GetOptions::range`]
    #[must_use]
    pub fn with_range(mut self, range: Option<impl Into<GetRange>>) -> Self {
        self.range = range.map(Into::into);
        self
    }

    /// Sets the `version` condition.
    ///
    /// See [`GetOptions::version`]
    #[must_use]
    pub fn with_version(mut self, version: Option<impl Into<String>>) -> Self {
        self.version = version.map(Into::into);
        self
    }

    /// Sets the `head` condition.
    ///
    /// See [`GetOptions::head`]
    #[must_use]
    pub fn with_head(mut self, head: impl Into<bool>) -> Self {
        self.head = head.into();
        self
    }

    /// Sets the `extensions` condition.
    ///
    /// See [`GetOptions::extensions`]
    #[must_use]
    pub fn with_extensions(mut self, extensions: Extensions) -> Self {
        self.extensions = extensions;
        self
    }
}

/// Result for a get request
#[derive(Debug)]
pub struct GetResult {
    /// The [`GetResultPayload`]
    pub payload: GetResultPayload,
    /// The [`ObjectMeta`] for this object
    pub meta: ObjectMeta,
    /// The range of bytes returned by this request
    ///
    /// Note this is not `usize` as `object_store` supports 32-bit architectures such as WASM
    pub range: Range<u64>,
    /// Additional object attributes
    pub attributes: Attributes,
    /// Implementation-specific extensions. Intended for use by [`ObjectStore`] implementations
    /// that need to return context-specific information (like cache status) from trait methods.
    ///
    /// HTTP-backed stores in this crate populate this with the extensions of the HTTP
    /// response, allowing custom HTTP middleware to propagate information to callers.
    pub extensions: Extensions,
}

/// The kind of a [`GetResult`]
///
/// This special cases the case of a local file, as some systems may
/// be able to optimise the case of a file already present on local disk
pub enum GetResultPayload {
    /// The file, path
    #[cfg(all(feature = "fs", not(target_arch = "wasm32")))]
    File(std::fs::File, std::path::PathBuf),
    /// An opaque stream of bytes
    Stream(BoxStream<'static, Result<Bytes>>),
}

impl Debug for GetResultPayload {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            #[cfg(all(feature = "fs", not(target_arch = "wasm32")))]
            Self::File(_, _) => write!(f, "GetResultPayload(File)"),
            Self::Stream(_) => write!(f, "GetResultPayload(Stream)"),
        }
    }
}

impl GetResult {
    /// Collects the data into a [`Bytes`]
    pub async fn bytes(self) -> Result<Bytes> {
        let len = self.range.end - self.range.start;
        match self.payload {
            #[cfg(all(feature = "fs", not(target_arch = "wasm32")))]
            GetResultPayload::File(mut file, path) => {
                maybe_spawn_blocking(move || {
                    use crate::local::read_range;

                    let buffer = read_range(&mut file, &path, self.range)?;

                    Ok(buffer)
                })
                .await
            }
            GetResultPayload::Stream(s) => collect_bytes(s, Some(len)).await,
        }
    }

    /// Converts this into a byte stream
    ///
    /// If the `self.kind` is [`GetResultPayload::File`] will perform chunked reads of the file,
    /// otherwise will return the [`GetResultPayload::Stream`].
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
    pub fn into_stream(self) -> BoxStream<'static, Result<Bytes>> {
        match self.payload {
            #[cfg(all(feature = "fs", not(target_arch = "wasm32")))]
            GetResultPayload::File(file, path) => {
                const CHUNK_SIZE: usize = 8 * 1024;
                local::chunked_stream(file, path, self.range, CHUNK_SIZE)
            }
            GetResultPayload::Stream(s) => s,
        }
    }
}

/// Configure preconditions for the put operation
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum PutMode {
    /// Perform an atomic write operation, overwriting any object present at the provided path
    #[default]
    Overwrite,
    /// Perform an atomic write operation, returning [`Error::AlreadyExists`] if an
    /// object already exists at the provided path
    Create,
    /// Perform an atomic write operation if the current version of the object matches the
    /// provided [`UpdateVersion`], returning [`Error::Precondition`] otherwise
    Update(UpdateVersion),
}

/// Uniquely identifies a version of an object to update
///
/// Stores will use differing combinations of `e_tag` and `version` to provide conditional
/// updates, and it is therefore recommended applications preserve both
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateVersion {
    /// The unique identifier for the newly created object
    ///
    /// <https://datatracker.ietf.org/doc/html/rfc9110#name-etag>
    pub e_tag: Option<String>,
    /// A version indicator for the newly created object
    pub version: Option<String>,
}

impl From<PutResult> for UpdateVersion {
    fn from(value: PutResult) -> Self {
        Self {
            e_tag: value.e_tag,
            version: value.version,
        }
    }
}

/// Options for a put request
#[derive(Debug, Clone, Default)]
pub struct PutOptions {
    /// Configure the [`PutMode`] for this operation
    pub mode: PutMode,
    /// Provide a [`TagSet`] for this object
    ///
    /// Implementations that don't support object tagging should ignore this
    pub tags: TagSet,
    /// Provide a set of [`Attributes`]
    ///
    /// Implementations that don't support an attribute should return an error
    pub attributes: Attributes,
    /// Implementation-specific extensions. Intended for use by [`ObjectStore`] implementations
    /// that need to pass context-specific information (like tracing spans) via trait methods.
    ///
    /// These extensions are ignored entirely by backends offered through this crate.
    ///
    /// They are also excluded from [`PartialEq`] and [`Eq`].
    pub extensions: Extensions,
}

impl PartialEq<Self> for PutOptions {
    fn eq(&self, other: &Self) -> bool {
        let Self {
            mode,
            tags,
            attributes,
            extensions: _,
        } = self;
        let Self {
            mode: other_mode,
            tags: other_tags,
            attributes: other_attributes,
            extensions: _,
        } = other;
        (mode == other_mode) && (tags == other_tags) && (attributes == other_attributes)
    }
}

impl Eq for PutOptions {}

impl From<PutMode> for PutOptions {
    fn from(mode: PutMode) -> Self {
        Self {
            mode,
            ..Default::default()
        }
    }
}

impl From<TagSet> for PutOptions {
    fn from(tags: TagSet) -> Self {
        Self {
            tags,
            ..Default::default()
        }
    }
}

impl From<Attributes> for PutOptions {
    fn from(attributes: Attributes) -> Self {
        Self {
            attributes,
            ..Default::default()
        }
    }
}

// See <https://github.com/apache/arrow-rs-object-store/issues/339>.
#[doc(hidden)]
#[deprecated(note = "Use PutMultipartOptions", since = "0.12.3")]
pub type PutMultipartOpts = PutMultipartOptions;

/// Options for [`ObjectStore::put_multipart_opts`]
#[derive(Debug, Clone, Default)]
pub struct PutMultipartOptions {
    /// Provide a [`TagSet`] for this object
    ///
    /// Implementations that don't support object tagging should ignore this
    pub tags: TagSet,
    /// Provide a set of [`Attributes`]
    ///
    /// Implementations that don't support an attribute should return an error
    pub attributes: Attributes,
    /// Implementation-specific extensions. Intended for use by [`ObjectStore`] implementations
    /// that need to pass context-specific information (like tracing spans) via trait methods.
    ///
    /// Cloud backends offered through this crate use extensions installed by methods such as
    /// `PutMultipartOptions::with_retry_policy`. Other extensions are ignored.
    ///
    /// They are also excluded from [`PartialEq`] and [`Eq`].
    pub extensions: Extensions,
}

impl PartialEq<Self> for PutMultipartOptions {
    fn eq(&self, other: &Self) -> bool {
        let Self {
            tags,
            attributes,
            extensions: _,
        } = self;
        let Self {
            tags: other_tags,
            attributes: other_attributes,
            extensions: _,
        } = other;
        (tags == other_tags) && (attributes == other_attributes)
    }
}

impl Eq for PutMultipartOptions {}

impl From<TagSet> for PutMultipartOptions {
    fn from(tags: TagSet) -> Self {
        Self {
            tags,
            ..Default::default()
        }
    }
}

impl From<Attributes> for PutMultipartOptions {
    fn from(attributes: Attributes) -> Self {
        Self {
            attributes,
            ..Default::default()
        }
    }
}

/// Result for a put request
#[derive(Debug, Clone)]
pub struct PutResult {
    /// The unique identifier for the newly created object
    ///
    /// <https://datatracker.ietf.org/doc/html/rfc9110#name-etag>
    pub e_tag: Option<String>,
    /// A version indicator for the newly created object
    pub version: Option<String>,
    /// Implementation-specific extensions. Intended for use by [`ObjectStore`] implementations
    /// that need to return context-specific information (like cache status) from trait methods.
    ///
    /// HTTP-backed stores in this crate populate this with the extensions of the HTTP
    /// response, allowing custom HTTP middleware to propagate information to callers.
    ///
    /// These extensions are excluded from [`PartialEq`] and [`Eq`].
    pub extensions: Extensions,
}

impl PartialEq<Self> for PutResult {
    fn eq(&self, other: &Self) -> bool {
        let Self {
            e_tag,
            version,
            extensions: _,
        } = self;
        let Self {
            e_tag: other_e_tag,
            version: other_version,
            extensions: _,
        } = other;
        (e_tag == other_e_tag) && (version == other_version)
    }
}

impl Eq for PutResult {}

/// Configure preconditions for the copy operation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CopyMode {
    /// Perform an atomic write operation, overwriting any object present at the provided path
    #[default]
    Overwrite,
    /// Perform an atomic write operation, returning [`Error::AlreadyExists`] if an
    /// object already exists at the provided path
    Create,
}

/// Options for a copy request
#[derive(Debug, Clone, Default)]
pub struct CopyOptions {
    /// Configure the [`CopyMode`] for this operation
    pub mode: CopyMode,
    /// Implementation-specific extensions. Intended for use by [`ObjectStore`] implementations
    /// that need to pass context-specific information (like tracing spans) via trait methods.
    ///
    /// These extensions are ignored entirely by backends offered through this crate.
    ///
    /// They are also excluded from [`PartialEq`] and [`Eq`].
    pub extensions: Extensions,
}

impl CopyOptions {
    /// Create a new [`CopyOptions`]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the `mode.
    ///
    /// See [`CopyOptions::mode`].
    #[must_use]
    pub fn with_mode(mut self, mode: CopyMode) -> Self {
        self.mode = mode;
        self
    }

    /// Sets the `extensions`.
    ///
    /// See [`CopyOptions::extensions`].
    #[must_use]
    pub fn with_extensions(mut self, extensions: Extensions) -> Self {
        self.extensions = extensions;
        self
    }
}

impl PartialEq<Self> for CopyOptions {
    fn eq(&self, other: &Self) -> bool {
        let Self {
            mode,
            extensions: _,
        } = self;
        let Self {
            mode: mode_other,
            extensions: _,
        } = other;

        mode == mode_other
    }
}

impl Eq for CopyOptions {}

/// Configure preconditions for the target of rename operation.
///
/// Note though that the source location may or not be deleted at the same time in an atomic operation. There is
/// currently NO flag to control the atomicity of "delete source at the same time as creating the target".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RenameTargetMode {
    /// Perform a write operation on the target, overwriting any object present at the provided path.
    #[default]
    Overwrite,
    /// Perform an atomic write operation of the target, returning [`Error::AlreadyExists`] if an
    /// object already exists at the provided path.
    Create,
}

/// Options for a rename request
#[derive(Debug, Clone, Default)]
pub struct RenameOptions {
    /// Configure the [`RenameTargetMode`] for this operation
    pub target_mode: RenameTargetMode,
    /// Implementation-specific extensions. Intended for use by [`ObjectStore`] implementations
    /// that need to pass context-specific information (like tracing spans) via trait methods.
    ///
    /// These extensions are ignored entirely by backends offered through this crate.
    ///
    /// They are also excluded from [`PartialEq`] and [`Eq`].
    pub extensions: Extensions,
}

impl RenameOptions {
    /// Create a new [`RenameOptions`]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the `target_mode=.
    ///
    /// See [`RenameOptions::target_mode`].
    #[must_use]
    pub fn with_target_mode(mut self, target_mode: RenameTargetMode) -> Self {
        self.target_mode = target_mode;
        self
    }

    /// Sets the `extensions`.
    ///
    /// See [`RenameOptions::extensions`].
    #[must_use]
    pub fn with_extensions(mut self, extensions: Extensions) -> Self {
        self.extensions = extensions;
        self
    }
}

impl PartialEq<Self> for RenameOptions {
    fn eq(&self, other: &Self) -> bool {
        let Self {
            target_mode,
            extensions: _,
        } = self;
        let Self {
            target_mode: target_mode_other,
            extensions: _,
        } = other;

        target_mode == target_mode_other
    }
}

impl Eq for RenameOptions {}

/// A specialized `Result` for object store-related errors
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// A specialized `Error` for object store-related errors
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// A fallback error type when no variant matches
    #[error("Generic {} error: {}", store, source)]
    Generic {
        /// The store this error originated from
        store: &'static str,
        /// The wrapped error
        source: Box<dyn std::error::Error + Send + Sync + 'static>,
    },

    /// Error when the object is not found at given location
    #[error("Object at location {} not found: {}", path, source)]
    NotFound {
        /// The path to file
        path: String,
        /// The wrapped error
        source: Box<dyn std::error::Error + Send + Sync + 'static>,
    },

    /// Error for invalid path
    #[error("Encountered object with invalid path: {}", source)]
    InvalidPath {
        /// The wrapped error
        #[from]
        source: path::Error,
    },

    /// Error when `tokio::spawn` failed
    #[cfg(feature = "tokio")]
    #[error("Error joining spawned task: {}", source)]
    JoinError {
        /// The wrapped error
        #[from]
        source: tokio::task::JoinError,
    },

    /// Error when the attempted operation is not supported
    #[error("Operation not supported: {}", source)]
    NotSupported {
        /// The wrapped error
        source: Box<dyn std::error::Error + Send + Sync + 'static>,
    },

    /// Error when the object already exists
    #[error("Object at location {} already exists: {}", path, source)]
    AlreadyExists {
        /// The path to the
        path: String,
        /// The wrapped error
        source: Box<dyn std::error::Error + Send + Sync + 'static>,
    },

    /// Error when the required conditions failed for the operation
    #[error("Request precondition failure for path {}: {}", path, source)]
    Precondition {
        /// The path to the file
        path: String,
        /// The wrapped error
        source: Box<dyn std::error::Error + Send + Sync + 'static>,
    },

    /// Error when the object at the location isn't modified
    #[error("Object at location {} not modified: {}", path, source)]
    NotModified {
        /// The path to the file
        path: String,
        /// The wrapped error
        source: Box<dyn std::error::Error + Send + Sync + 'static>,
    },

    /// Error when an operation is not implemented
    #[error("Operation {operation} not yet implemented by {implementer}.")]
    NotImplemented {
        /// What isn't implemented. Should include at least the method
        /// name that was called; could also include other relevant
        /// subcontexts.
        operation: String,

        /// Which driver this is that hasn't implemented this operation,
        /// to aid debugging in contexts that may be using multiple implementations.
        implementer: String,
    },

    /// Error when the used credentials don't have enough permission
    /// to perform the requested operation
    #[error(
        "The operation lacked the necessary privileges to complete for path {}: {}",
        path,
        source
    )]
    PermissionDenied {
        /// The path to the file
        path: String,
        /// The wrapped error
        source: Box<dyn std::error::Error + Send + Sync + 'static>,
    },

    /// Error when the used credentials lack valid authentication
    #[error(
        "The operation lacked valid authentication credentials for path {}: {}",
        path,
        source
    )]
    Unauthenticated {
        /// The path to the file
        path: String,
        /// The wrapped error
        source: Box<dyn std::error::Error + Send + Sync + 'static>,
    },

    /// Error when a configuration key is invalid for the store used
    #[error("Configuration key: '{}' is not valid for store '{}'.", key, store)]
    UnknownConfigurationKey {
        /// The object store used
        store: &'static str,
        /// The configuration key used
        key: String,
    },
}

impl From<Error> for std::io::Error {
    fn from(e: Error) -> Self {
        let kind = match &e {
            Error::NotFound { .. } => std::io::ErrorKind::NotFound,
            _ => std::io::ErrorKind::Other,
        };
        Self::new(kind, e)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use chrono::TimeZone;

    macro_rules! maybe_skip_integration {
        () => {
            if std::env::var("TEST_INTEGRATION").is_err() {
                eprintln!("Skipping integration test - set TEST_INTEGRATION");
                return;
            }
        };
    }
    pub(crate) use maybe_skip_integration;

    /// Skip a test that asserts the *storage backend* rejects an invalid presigned request
    /// (tampered signature, expired URL, unmet signed header) unless
    /// `TEST_S3_SIGNATURE_ENFORCEMENT` is set.
    ///
    /// These tests require a backend that actually validates SigV4 — real S3 or MinIO. The
    /// LocalStack emulator used by the default integration suite does not validate presigned
    /// signatures or expiry and would return success, so they are gated separately. See
    /// CONTRIBUTING.md for the MinIO/S3 setup.
    macro_rules! maybe_skip_signature_enforcement {
        () => {
            if std::env::var("TEST_S3_SIGNATURE_ENFORCEMENT").is_err() {
                eprintln!(
                    "Skipping signature-enforcement test - set TEST_S3_SIGNATURE_ENFORCEMENT \
                     and point at a backend that validates SigV4 (real S3 or MinIO)"
                );
                return;
            }
        };
    }
    pub(crate) use maybe_skip_signature_enforcement;

    /// Test that the returned stream does not borrow the lifetime of Path
    fn list_store<'a>(
        store: &'a dyn ObjectStore,
        path_str: &str,
    ) -> BoxStream<'a, Result<ObjectMeta>> {
        let path = Path::from(path_str);
        store.list(Some(&path))
    }

    #[cfg(any(feature = "azure-base", feature = "aws-base"))]
    pub(crate) async fn signing<T>(integration: &T)
    where
        T: ObjectStore + signer::Signer,
    {
        use ::http::Method;
        use std::time::Duration;

        let data = Bytes::from("hello world");
        let path = Path::from("file.txt");
        integration.put(&path, data.clone().into()).await.unwrap();

        let signed = integration
            .signed_url(Method::GET, &path, Duration::from_secs(60))
            .await
            .unwrap();

        let resp = reqwest::get(signed).await.unwrap();
        let loaded = resp.bytes().await.unwrap();

        assert_eq!(data, loaded);
    }

    #[cfg(any(feature = "aws-base", feature = "azure-base"))]
    pub(crate) async fn tagging<F, Fut>(storage: Arc<dyn ObjectStore>, validate: bool, get_tags: F)
    where
        F: Fn(Path) -> Fut + Send + Sync,
        Fut: std::future::Future<Output = Result<client::HttpResponse>> + Send,
    {
        use bytes::Buf;
        use serde::Deserialize;
        use tokio::io::AsyncWriteExt;

        use crate::buffered::BufWriter;

        #[derive(Deserialize)]
        struct Tagging {
            #[serde(rename = "TagSet")]
            list: TagList,
        }

        #[derive(Debug, Deserialize)]
        struct TagList {
            #[serde(rename = "Tag")]
            tags: Vec<Tag>,
        }

        #[derive(Debug, Deserialize, Eq, PartialEq)]
        #[serde(rename_all = "PascalCase")]
        struct Tag {
            key: String,
            value: String,
        }

        let tags = vec![
            Tag {
                key: "foo.com=bar/s".to_string(),
                value: "bananas/foo.com-_".to_string(),
            },
            Tag {
                key: "namespace/key.foo".to_string(),
                value: "value with a space".to_string(),
            },
        ];
        let mut tag_set = TagSet::default();
        for t in &tags {
            tag_set.push(&t.key, &t.value)
        }

        let path = Path::from("tag_test");
        storage
            .put_opts(&path, "test".into(), tag_set.clone().into())
            .await
            .unwrap();

        let multi_path = Path::from("tag_test_multi");
        let mut write = storage
            .put_multipart_opts(&multi_path, tag_set.clone().into())
            .await
            .unwrap();

        write.put_part("foo".into()).await.unwrap();
        write.complete().await.unwrap();

        let buf_path = Path::from("tag_test_buf");
        let mut buf = BufWriter::new(storage, buf_path.clone()).with_tags(tag_set);
        buf.write_all(b"foo").await.unwrap();
        buf.shutdown().await.unwrap();

        // Write should always succeed, but certain configurations may simply ignore tags
        if !validate {
            return;
        }

        for path in [path, multi_path, buf_path] {
            let resp = get_tags(path.clone()).await.unwrap();
            let body = resp.into_body().bytes().await.unwrap();

            let mut resp: Tagging = quick_xml::de::from_reader(body.reader()).unwrap();
            resp.list.tags.sort_by(|a, b| a.key.cmp(&b.key));
            assert_eq!(resp.list.tags, tags);
        }
    }

    #[tokio::test]
    async fn test_list_lifetimes() {
        let store = memory::InMemory::new();
        let mut stream = list_store(&store, "path");
        assert!(stream.next().await.is_none());
    }

    #[test]
    fn test_preconditions() {
        let mut meta = ObjectMeta {
            location: Path::from("test"),
            last_modified: Utc.timestamp_nanos(100),
            size: 100,
            e_tag: Some("\"123\"".to_string()),
            version: None,
        };

        let mut options = GetOptions::default();
        options.check_preconditions(&meta).unwrap();

        options.if_modified_since = Some(Utc.timestamp_nanos(50));
        options.check_preconditions(&meta).unwrap();

        options.if_modified_since = Some(Utc.timestamp_nanos(100));
        options.check_preconditions(&meta).unwrap_err();

        options.if_modified_since = Some(Utc.timestamp_nanos(101));
        options.check_preconditions(&meta).unwrap_err();

        options = GetOptions::default();

        options.if_unmodified_since = Some(Utc.timestamp_nanos(50));
        options.check_preconditions(&meta).unwrap_err();

        options.if_unmodified_since = Some(Utc.timestamp_nanos(100));
        options.check_preconditions(&meta).unwrap();

        options.if_unmodified_since = Some(Utc.timestamp_nanos(101));
        options.check_preconditions(&meta).unwrap();

        options = GetOptions::default();

        options.if_match = Some("\"123\"".to_string());
        options.check_preconditions(&meta).unwrap();

        options.if_match = Some("\"123\",\"354\"".to_string());
        options.check_preconditions(&meta).unwrap();

        options.if_match = Some("\"354\", \"123\"".to_string());
        options.check_preconditions(&meta).unwrap();

        options.if_match = Some("\"354\"".to_string());
        options.check_preconditions(&meta).unwrap_err();

        options.if_match = Some("*".to_string());
        options.check_preconditions(&meta).unwrap();

        // If-Match takes precedence
        options.if_unmodified_since = Some(Utc.timestamp_nanos(200));
        options.check_preconditions(&meta).unwrap();

        options = GetOptions::default();

        options.if_none_match = Some("\"123\"".to_string());
        options.check_preconditions(&meta).unwrap_err();

        options.if_none_match = Some("*".to_string());
        options.check_preconditions(&meta).unwrap_err();

        options.if_none_match = Some("\"1232\"".to_string());
        options.check_preconditions(&meta).unwrap();

        options.if_none_match = Some("\"23\", \"123\"".to_string());
        options.check_preconditions(&meta).unwrap_err();

        // If-None-Match takes precedence
        options.if_modified_since = Some(Utc.timestamp_nanos(10));
        options.check_preconditions(&meta).unwrap_err();

        // Check missing ETag
        meta.e_tag = None;
        options = GetOptions::default();

        options.if_none_match = Some("*".to_string()); // Fails if any file exists
        options.check_preconditions(&meta).unwrap_err();

        options = GetOptions::default();
        options.if_match = Some("*".to_string()); // Passes if file exists
        options.check_preconditions(&meta).unwrap();
    }

    #[test]
    #[cfg(feature = "http-base")]
    fn test_reexported_types() {
        // Test HeaderMap
        let mut headers = HeaderMap::new();
        headers.insert("content-type", HeaderValue::from_static("text/plain"));
        assert_eq!(headers.len(), 1);

        // Test HeaderValue
        let value = HeaderValue::from_static("test-value");
        assert_eq!(value.as_bytes(), b"test-value");

        // Test Extensions
        let mut extensions = Extensions::new();
        extensions.insert("test-key");
        assert!(extensions.get::<&str>().is_some());
    }

    #[test]
    fn test_get_options_builder() {
        let dt = Utc::now();
        let extensions = Extensions::new();

        let options = GetOptions::new();

        // assert defaults
        assert_eq!(options.if_match, None);
        assert_eq!(options.if_none_match, None);
        assert_eq!(options.if_modified_since, None);
        assert_eq!(options.if_unmodified_since, None);
        assert_eq!(options.range, None);
        assert_eq!(options.version, None);
        assert!(!options.head);
        assert!(options.extensions.get::<&str>().is_none());

        let options = options
            .with_if_match(Some("etag-match"))
            .with_if_none_match(Some("etag-none-match"))
            .with_if_modified_since(Some(dt))
            .with_if_unmodified_since(Some(dt))
            .with_range(Some(0..100))
            .with_version(Some("version-1"))
            .with_head(true)
            .with_extensions(extensions.clone());

        assert_eq!(options.if_match, Some("etag-match".to_string()));
        assert_eq!(options.if_none_match, Some("etag-none-match".to_string()));
        assert_eq!(options.if_modified_since, Some(dt));
        assert_eq!(options.if_unmodified_since, Some(dt));
        assert_eq!(options.range, Some(GetRange::Bounded(0..100)));
        assert_eq!(options.version, Some("version-1".to_string()));
        assert!(options.head);
        assert_eq!(options.extensions.get::<&str>(), extensions.get::<&str>());
    }

    fn takes_generic_object_store<T: ObjectStore>(store: T) {
        // This function is just to ensure that the trait bounds are satisfied
        let _ = store;
    }
    #[test]
    fn test_dyn_impl() {
        let store: Arc<dyn ObjectStore> = Arc::new(memory::InMemory::new());
        takes_generic_object_store(store);
        let store: Box<dyn ObjectStore> = Box::new(memory::InMemory::new());
        takes_generic_object_store(store);
    }
    #[test]
    fn test_generic_impl() {
        let store = Arc::new(memory::InMemory::new());
        takes_generic_object_store(store);
        let store = Box::new(memory::InMemory::new());
        takes_generic_object_store(store);
    }
}
