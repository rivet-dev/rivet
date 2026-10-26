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

//! An object store wrapper handling a constant path prefix
use bytes::Bytes;
use futures_util::{StreamExt, TryStreamExt, stream::BoxStream};
use std::ops::Range;

use crate::multipart::{MultipartStore, PartId};
use crate::path::Path;
#[cfg(feature = "cloud-base")]
use crate::signer::Signer;
use crate::{
    CopyOptions, GetOptions, GetResult, ListResult, MultipartId, MultipartUpload, ObjectMeta,
    ObjectStore, PutMultipartOptions, PutOptions, PutPayload, PutResult, RenameOptions, Result,
};

/// Store wrapper that applies a constant prefix to all paths handled by the store.
#[derive(Debug, Clone)]
pub struct PrefixStore<T> {
    prefix: Path,
    inner: T,
}

impl<T> std::fmt::Display for PrefixStore<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PrefixObjectStore({})", self.prefix.as_ref())
    }
}

impl<T> PrefixStore<T> {
    /// Create a new instance of [`PrefixStore`]
    pub fn new(store: T, prefix: impl Into<Path>) -> Self {
        Self {
            prefix: prefix.into(),
            inner: store,
        }
    }

    /// Create the full path from a path relative to prefix
    fn full_path(&self, location: &Path) -> Path {
        full_path(&self.prefix, location)
    }

    /// Strip the constant prefix from a given path
    fn strip_prefix(&self, path: Path) -> Path {
        strip_prefix(&self.prefix, path)
    }

    /// Strip the constant prefix from a given ObjectMeta
    fn strip_meta(&self, meta: ObjectMeta) -> ObjectMeta {
        strip_meta(&self.prefix, meta)
    }
}

// Note: This is a relative hack to move these functions to pure functions so they don't rely
// on the `self` lifetime.

/// Create the full path from a path relative to prefix
fn full_path(prefix: &Path, path: &Path) -> Path {
    prefix.parts().chain(path.parts()).collect()
}

/// Strip the constant prefix from a given path
fn strip_prefix(prefix: &Path, path: Path) -> Path {
    // Note cannot use match because of borrow checker
    if let Some(suffix) = path.prefix_match(prefix) {
        return suffix.collect();
    }
    path
}

/// Strip the constant prefix from a given ObjectMeta
fn strip_meta(prefix: &Path, meta: ObjectMeta) -> ObjectMeta {
    let ObjectMeta {
        last_modified,
        size,
        location,
        e_tag,
        version,
    } = meta;
    ObjectMeta {
        last_modified,
        size,
        location: strip_prefix(prefix, location),
        e_tag,
        version,
    }
}

#[async_trait::async_trait]
#[deny(clippy::missing_trait_methods)]
impl<T: ObjectStore> ObjectStore for PrefixStore<T> {
    async fn put_opts(
        &self,
        location: &Path,
        payload: PutPayload,
        opts: PutOptions,
    ) -> Result<PutResult> {
        let full_path = self.full_path(location);
        self.inner.put_opts(&full_path, payload, opts).await
    }

    async fn put_multipart_opts(
        &self,
        location: &Path,
        opts: PutMultipartOptions,
    ) -> Result<Box<dyn MultipartUpload>> {
        let full_path = self.full_path(location);
        self.inner.put_multipart_opts(&full_path, opts).await
    }

    async fn get_opts(&self, location: &Path, options: GetOptions) -> Result<GetResult> {
        let full_path = self.full_path(location);
        let mut result = self.inner.get_opts(&full_path, options).await?;
        result.meta = self.strip_meta(result.meta);
        Ok(result)
    }

    async fn get_ranges(&self, location: &Path, ranges: &[Range<u64>]) -> Result<Vec<Bytes>> {
        let full_path = self.full_path(location);
        self.inner.get_ranges(&full_path, ranges).await
    }

    async fn delete_if_match(&self, location: &Path, etag: &str) -> Result<()> {
        self.inner.delete_if_match(&self.full_path(location), etag).await
    }

    fn delete_stream(
        &self,
        locations: BoxStream<'static, Result<Path>>,
    ) -> BoxStream<'static, Result<Path>> {
        let prefix = self.prefix.clone();
        let locations = locations
            .map(move |location| location.map(|loc| full_path(&prefix, &loc)))
            .boxed();
        let prefix = self.prefix.clone();
        self.inner
            .delete_stream(locations)
            .map(move |location| location.map(|loc| strip_prefix(&prefix, loc)))
            .boxed()
    }

    fn list(&self, prefix: Option<&Path>) -> BoxStream<'static, Result<ObjectMeta>> {
        let prefix = self.full_path(prefix.unwrap_or(&Path::default()));
        let s = self.inner.list(Some(&prefix));
        let slf_prefix = self.prefix.clone();
        s.map_ok(move |meta| strip_meta(&slf_prefix, meta)).boxed()
    }

    fn list_with_offset(
        &self,
        prefix: Option<&Path>,
        offset: &Path,
    ) -> BoxStream<'static, Result<ObjectMeta>> {
        let offset = self.full_path(offset);
        let prefix = self.full_path(prefix.unwrap_or(&Path::default()));
        let s = self.inner.list_with_offset(Some(&prefix), &offset);
        let slf_prefix = self.prefix.clone();
        s.map_ok(move |meta| strip_meta(&slf_prefix, meta)).boxed()
    }

    async fn list_with_delimiter(&self, prefix: Option<&Path>) -> Result<ListResult> {
        let prefix = self.full_path(prefix.unwrap_or(&Path::default()));
        self.inner
            .list_with_delimiter(Some(&prefix))
            .await
            .map(|lst| ListResult {
                common_prefixes: lst
                    .common_prefixes
                    .into_iter()
                    .map(|p| self.strip_prefix(p))
                    .collect(),
                objects: lst
                    .objects
                    .into_iter()
                    .map(|meta| self.strip_meta(meta))
                    .collect(),
                extensions: lst.extensions,
            })
    }

    async fn copy_opts(&self, from: &Path, to: &Path, options: CopyOptions) -> Result<()> {
        let full_from = self.full_path(from);
        let full_to = self.full_path(to);
        self.inner.copy_opts(&full_from, &full_to, options).await
    }

    async fn rename_opts(&self, from: &Path, to: &Path, options: RenameOptions) -> Result<()> {
        let full_from = self.full_path(from);
        let full_to = self.full_path(to);
        self.inner.rename_opts(&full_from, &full_to, options).await
    }
}

#[async_trait::async_trait]
impl<T: MultipartStore> MultipartStore for PrefixStore<T> {
    async fn create_multipart(&self, path: &Path) -> Result<MultipartId> {
        let full_path = self.full_path(path);
        self.inner.create_multipart(&full_path).await
    }

    async fn create_multipart_opts(
        &self,
        path: &Path,
        opts: PutMultipartOptions,
    ) -> Result<MultipartId> {
        let full_path = self.full_path(path);
        self.inner.create_multipart_opts(&full_path, opts).await
    }

    async fn put_part(
        &self,
        path: &Path,
        id: &MultipartId,
        part_idx: usize,
        data: PutPayload,
    ) -> Result<PartId> {
        let full_path = self.full_path(path);
        self.inner.put_part(&full_path, id, part_idx, data).await
    }

    async fn complete_multipart(
        &self,
        path: &Path,
        id: &MultipartId,
        parts: Vec<PartId>,
    ) -> Result<PutResult> {
        let full_path = self.full_path(path);
        self.inner.complete_multipart(&full_path, id, parts).await
    }

    async fn abort_multipart(&self, path: &Path, id: &MultipartId) -> Result<()> {
        let full_path = self.full_path(path);
        self.inner.abort_multipart(&full_path, id).await
    }
}

#[cfg(feature = "cloud-base")]
#[async_trait::async_trait]
impl<T: Signer> Signer for PrefixStore<T> {
    async fn signed_url(
        &self,
        method: http::Method,
        path: &Path,
        expires_in: std::time::Duration,
    ) -> Result<url::Url> {
        self.inner
            .signed_url(method, &self.full_path(path), expires_in)
            .await
    }

    async fn signed_url_opts(
        &self,
        method: http::Method,
        path: &Path,
        expires_in: std::time::Duration,
        options: &crate::signer::SignedUrlOptions,
    ) -> Result<url::Url> {
        self.inner
            .signed_url_opts(method, &self.full_path(path), expires_in, options)
            .await
    }

    async fn signed_urls(
        &self,
        method: http::Method,
        paths: &[Path],
        expires_in: std::time::Duration,
    ) -> Result<Vec<url::Url>> {
        self.inner
            .signed_urls(
                method,
                &paths.iter().map(|p| self.full_path(p)).collect::<Vec<_>>(),
                expires_in,
            )
            .await
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
    use std::slice;

    use super::*;
    use crate::local::LocalFileSystem;
    use crate::memory::InMemory;
    use crate::{ObjectStoreExt, integration::*};

    use tempfile::TempDir;

    #[tokio::test]
    async fn prefix_test() {
        let root = TempDir::new().unwrap();
        let inner = LocalFileSystem::new_with_prefix(root.path()).unwrap();
        let integration = PrefixStore::new(inner, "prefix");

        put_get_delete_list(&integration).await;
        get_opts(&integration).await;
        list_uses_directories_correctly(&integration).await;
        list_with_delimiter(&integration).await;
        rename_and_copy(&integration).await;
        copy_if_not_exists(&integration).await;
        stream_get(&integration).await;
    }

    #[tokio::test]
    async fn prefix_test_applies_prefix() {
        let tmpdir = TempDir::new().unwrap();
        let local = LocalFileSystem::new_with_prefix(tmpdir.path()).unwrap();

        let location = Path::from("prefix/test_file.json");
        let data = Bytes::from("arbitrary data");

        local.put(&location, data.clone().into()).await.unwrap();

        let prefix = PrefixStore::new(local, "prefix");
        let location_prefix = Path::from("test_file.json");

        let content_list = flatten_list_stream(&prefix, None).await.unwrap();
        assert_eq!(content_list, slice::from_ref(&location_prefix));

        let root = Path::from("/");
        let content_list = flatten_list_stream(&prefix, Some(&root)).await.unwrap();
        assert_eq!(content_list, slice::from_ref(&location_prefix));

        let read_data = prefix
            .get(&location_prefix)
            .await
            .unwrap()
            .bytes()
            .await
            .unwrap();
        assert_eq!(&*read_data, data);

        let target_prefix = Path::from("/test_written.json");
        prefix
            .put(&target_prefix, data.clone().into())
            .await
            .unwrap();

        prefix.delete(&location_prefix).await.unwrap();

        let local = LocalFileSystem::new_with_prefix(tmpdir.path()).unwrap();

        let err = local.get(&location).await.unwrap_err();
        assert!(matches!(err, crate::Error::NotFound { .. }), "{}", err);

        let location = Path::from("prefix/test_written.json");
        let read_data = local.get(&location).await.unwrap().bytes().await.unwrap();
        assert_eq!(&*read_data, data)
    }

    // Regression test for
    // https://github.com/apache/arrow-rs-object-store/issues/664:
    // head/get returned an ObjectMeta whose location still contained the
    // prefix, so round-tripping the returned location back into the store would
    // double-prefix it.
    #[tokio::test]
    async fn prefix_head_get_strip_prefix() {
        let store = PrefixStore::new(InMemory::new(), "prefix");
        let path = Path::from("test_file");
        store.put(&path, "data".into()).await.unwrap();

        let head = store.head(&path).await.unwrap();
        assert_eq!(head.location, path);
        let get = store.get(&path).await.unwrap();
        assert_eq!(get.meta.location, path);

        // The returned location must round-trip back into the same store.
        store.get(&head.location).await.unwrap();
    }

    #[test]
    fn strip_meta_preserves_version_and_etag() {
        let prefix = Path::from("prefix");
        let meta = ObjectMeta {
            location: Path::from("prefix/foo"),
            last_modified: chrono::DateTime::from_timestamp(1_700_000_000, 0).unwrap(),
            size: 42,
            e_tag: Some("etag-value".to_string()),
            version: Some("version-value".to_string()),
        };

        let ObjectMeta {
            location,
            last_modified,
            size,
            e_tag,
            version,
        } = strip_meta(&prefix, meta.clone());

        assert_eq!(location, Path::from("foo"));
        assert_eq!(last_modified, meta.last_modified);
        assert_eq!(size, meta.size);
        assert_eq!(e_tag, meta.e_tag);
        assert_eq!(version, meta.version);
    }

    #[tokio::test]
    async fn prefix_multipart() {
        let store = PrefixStore::new(InMemory::new(), "prefix");

        multipart(&store, &store).await;
        multipart_with_opts(&store, &store).await;
        multipart_put_part_out_of_order(&store, &store).await;
        multipart_out_of_order(&store).await;
        multipart_race_condition(&store, true).await;
    }

    #[cfg(feature = "cloud-base")]
    #[tokio::test]
    async fn signer() {
        #[derive(Debug)]
        struct Foo;

        #[async_trait::async_trait]
        impl Signer for Foo {
            async fn signed_url(
                &self,
                method: http::Method,
                path: &Path,
                _expires_in: std::time::Duration,
            ) -> Result<url::Url> {
                Ok(url::Url::parse(&format!("ex:{path}?method={method}")).unwrap())
            }
        }

        assert_eq!(
            PrefixStore::new(Foo, "prefix")
                .signed_url(
                    http::Method::GET,
                    &"foo".into(),
                    std::time::Duration::from_secs(1)
                )
                .await
                .unwrap(),
            url::Url::parse("ex:prefix/foo?method=GET").unwrap()
        );
        assert_eq!(
            PrefixStore::new(Foo, "prefix")
                .signed_urls(
                    http::Method::GET,
                    &["foo".into()],
                    std::time::Duration::from_secs(1)
                )
                .await
                .unwrap(),
            vec![url::Url::parse("ex:prefix/foo?method=GET").unwrap()]
        );
    }
}
