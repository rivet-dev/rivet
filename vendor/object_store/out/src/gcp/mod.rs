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

//! An object store implementation for Google Cloud Storage
//!
//! See the [Feature Flags](crate#feature-flags) section for the difference
//! between the `gcp` and `gcp-base` features.
//!
//! ## Multipart uploads
//!
//! [Multipart uploads](https://cloud.google.com/storage/docs/multipart-uploads)
//! can be initiated with the [`ObjectStore::put_multipart_opts`] method. If neither
//! [`MultipartUpload::complete`] nor [`MultipartUpload::abort`] is invoked, you may
//! have parts uploaded to GCS but not used, that you will be charged for. It is recommended
//! you configure a [lifecycle rule] to abort incomplete multipart uploads after a certain
//! period of time to avoid being charged for storing partial uploads.
//!
//! ## Using HTTP/2
//!
//! Google Cloud Storage supports both HTTP/2 and HTTP/1. HTTP/1 is used by default
//! because it allows much higher throughput in our benchmarks (see
//! [#5194](https://github.com/apache/arrow-rs/issues/5194)). HTTP/2 can be
//! enabled by setting [crate::ClientConfigKey::Http1Only] to false.
//!
//! [lifecycle rule]: https://cloud.google.com/storage/docs/lifecycle#abort-mpu
use std::sync::Arc;
use std::time::Duration;

use crate::CopyOptions;
use crate::client::{CredentialProvider, crypto_provider};
use crate::gcp::credential::GCSAuthorizer;
use crate::retry::{MultipartRetry, RetryPolicy};
use crate::signer::{SignedUrlOptions, Signer};
use crate::util::validate_signed_url_extras;
use crate::{
    GetOptions, GetResult, ListResult, MultipartId, MultipartUpload, ObjectMeta, ObjectStore,
    PutMultipartOptions, PutOptions, PutPayload, PutResult, Result, UploadPart, multipart::PartId,
    path::Path,
};
use async_trait::async_trait;
use client::GoogleCloudStorageClient;
use futures_util::stream::{BoxStream, StreamExt};
use http::Method;
use url::Url;

use crate::client::get::GetClientExt;
use crate::client::list::{ListClient, ListClientExt};
use crate::client::parts::Parts;
use crate::list::{PaginatedListOptions, PaginatedListResult, PaginatedListStore};
use crate::multipart::MultipartStore;
pub use builder::{GoogleCloudStorageBuilder, GoogleConfigKey};
pub use credential::{GcpCredential, GcpSigningCredential, ServiceAccountKey};

mod builder;
mod client;
mod credential;

const STORE: &str = "GCS";

/// [`CredentialProvider`] for [`GoogleCloudStorage`]
pub type GcpCredentialProvider = Arc<dyn CredentialProvider<Credential = GcpCredential>>;

/// [`GcpSigningCredential`] for [`GoogleCloudStorage`]
pub type GcpSigningCredentialProvider =
    Arc<dyn CredentialProvider<Credential = GcpSigningCredential>>;

/// Interface for [Google Cloud Storage](https://cloud.google.com/storage/).
#[derive(Debug, Clone)]
pub struct GoogleCloudStorage {
    client: Arc<GoogleCloudStorageClient>,
    generation_etags: bool,
}

impl std::fmt::Display for GoogleCloudStorage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "GoogleCloudStorage({})",
            self.client.config().bucket_name
        )
    }
}

impl GoogleCloudStorage {
    /// Expose generations through ETag fields for clients retaining a single CAS token.
    /// Conditional reads are checked against the returned generation; list results without
    /// generations have no ETag. Native version fields and multipart part IDs are unchanged.
    pub fn with_generation_etags(mut self) -> Self {
        self.generation_etags = true;
        self
    }

    fn metadata(&self, mut meta: ObjectMeta) -> ObjectMeta {
        if self.generation_etags {
            meta.e_tag = meta.version.clone();
        }
        meta
    }

    fn put_result(&self, mut result: PutResult) -> PutResult {
        if self.generation_etags {
            result.e_tag = result.version.clone();
        }
        result
    }

    /// Returns the [`GcpCredentialProvider`] used by [`GoogleCloudStorage`]
    pub fn credentials(&self) -> &GcpCredentialProvider {
        &self.client.config().credentials
    }

    /// Returns the [`GcpSigningCredentialProvider`] used by [`GoogleCloudStorage`]
    pub fn signing_credentials(&self) -> &GcpSigningCredentialProvider {
        &self.client.config().signing_credentials
    }
}

#[derive(Debug)]
struct GCSMultipartUpload {
    store: GoogleCloudStorage,
    state: Arc<UploadState>,
    part_idx: usize,
}

#[derive(Debug)]
struct UploadState {
    client: Arc<GoogleCloudStorageClient>,
    path: Path,
    multipart_id: MultipartId,
    parts: Parts,
    retry_policy: Option<Arc<dyn RetryPolicy>>,
}

#[async_trait]
impl MultipartUpload for GCSMultipartUpload {
    fn put_part(&mut self, payload: PutPayload) -> UploadPart {
        let idx = self.part_idx;
        self.part_idx += 1;
        let state = Arc::clone(&self.state);
        Box::pin(async move {
            let mut retry = MultipartRetry::new(state.retry_policy.clone());
            loop {
                match state
                    .client
                    .put_part(&state.path, &state.multipart_id, idx, payload.clone())
                    .await
                {
                    Ok(part) => {
                        state.parts.put(idx, part);
                        return Ok(());
                    }
                    Err(error) if retry.should_retry(&error).await => continue,
                    Err(error) => return Err(error),
                }
            }
        })
    }

    async fn complete(&mut self) -> Result<PutResult> {
        let parts = self.state.parts.finish(self.part_idx)?;

        let result = self.state
            .client
            .multipart_complete(&self.state.path, &self.state.multipart_id, parts)
            .await?;
        Ok(self.store.put_result(result))
    }

    async fn abort(&mut self) -> Result<()> {
        self.state
            .client
            .multipart_cleanup(&self.state.path, &self.state.multipart_id)
            .await
    }
}

#[async_trait]
impl ObjectStore for GoogleCloudStorage {
    async fn put_opts(
        &self,
        location: &Path,
        payload: PutPayload,
        mut opts: PutOptions,
    ) -> Result<PutResult> {
        if self.generation_etags {
            if let crate::PutMode::Update(version) = &mut opts.mode {
                version.version = version.version.take().or_else(|| version.e_tag.take());
            }
        }
        let result = self.client.put(location, payload, opts).await?;
        Ok(self.put_result(result))
    }

    async fn put_multipart_opts(
        &self,
        location: &Path,
        opts: PutMultipartOptions,
    ) -> Result<Box<dyn MultipartUpload>> {
        let retry_policy = opts.retry_policy();
        let upload_id = self.client.multipart_initiate(location, opts).await?;

        Ok(Box::new(GCSMultipartUpload {
            store: self.clone(),
            part_idx: 0,
            state: Arc::new(UploadState {
                client: Arc::clone(&self.client),
                path: location.clone(),
                multipart_id: upload_id.clone(),
                parts: Default::default(),
                retry_policy,
            }),
        }))
    }

    async fn get_opts(&self, location: &Path, options: GetOptions) -> Result<GetResult> {
        if !self.generation_etags {
            return self.client.get_opts(location, options).await;
        }
        // Check the metadata from this very response, not a separate HEAD (which races).
        let mut request = options.clone();
        request.if_match = None;
        request.if_none_match = None;
        request.if_modified_since = None;
        request.if_unmodified_since = None;
        let mut result = self.client.get_opts(location, request).await?;
        result.meta = self.metadata(result.meta);
        options.check_preconditions(&result.meta)?;
        Ok(result)
    }

    async fn delete_if_match(&self, location: &Path, etag: &str) -> Result<()> {
        if !self.generation_etags {
            return Err(crate::Error::NotSupported {
                source: "conditional delete requires generation ETags".into(),
            });
        }
        self.client.delete_if_generation_match(location, etag).await
    }

    fn delete_stream(
        &self,
        locations: BoxStream<'static, Result<Path>>,
    ) -> BoxStream<'static, Result<Path>> {
        let client = Arc::clone(&self.client);
        locations
            .map(move |location| {
                let client = Arc::clone(&client);
                async move {
                    let location = location?;
                    client.delete_request(&location).await?;
                    Ok(location)
                }
            })
            .buffered(10)
            .boxed()
    }

    fn list(&self, prefix: Option<&Path>) -> BoxStream<'static, Result<ObjectMeta>> {
        let store = self.clone();
        self.client.list(prefix).map(move |meta| meta.map(|meta| store.metadata(meta))).boxed()
    }

    fn list_with_offset(
        &self,
        prefix: Option<&Path>,
        offset: &Path,
    ) -> BoxStream<'static, Result<ObjectMeta>> {
        let store = self.clone();
        self.client.list_with_offset(prefix, offset).map(move |meta| meta.map(|meta| store.metadata(meta))).boxed()
    }

    async fn list_with_delimiter(&self, prefix: Option<&Path>) -> Result<ListResult> {
        let mut result = self.client.list_with_delimiter(prefix).await?;
        result.objects = result.objects.into_iter().map(|meta| self.metadata(meta)).collect();
        Ok(result)
    }

    async fn copy_opts(&self, from: &Path, to: &Path, options: CopyOptions) -> Result<()> {
        let CopyOptions {
            mode,
            extensions: _,
        } = options;

        self.client.copy_request(from, to, mode).await
    }
}

#[async_trait]
impl MultipartStore for GoogleCloudStorage {
    /// Create a new multipart upload, returning its [`MultipartId`].
    ///
    /// This is the low-level [`MultipartStore`] API, which gives direct control
    /// over individual parts. See [`ObjectStoreExt::put_multipart`] for a
    /// higher-level API that handles parts automatically.
    ///
    /// # Example
    ///
    /// Create a multipart upload, upload two parts, and finalize it by calling
    /// [`complete_multipart`]:
    ///
    /// ```no_run
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # use object_store::{gcp::GoogleCloudStorageBuilder, multipart::MultipartStore, path::Path, PutPayload};
    /// #
    /// let gcs = GoogleCloudStorageBuilder::new()
    ///     .with_bucket_name("my-bucket")
    ///     .with_service_account_path("/path/to/service-account.json")
    ///     .build()?;
    ///
    /// let path = Path::from("data/large_file");
    ///
    /// // Start the upload, obtaining an id used to reference it in later calls
    /// let id = gcs.create_multipart(&path).await?;
    ///
    /// // Upload the individual parts. Every part except the last must be at least
    /// // 5 MiB.
    /// let part0 = gcs
    ///     .put_part(&path, &id, 0, PutPayload::from(vec![0; 5 * 1024 * 1024]))
    ///     .await?;
    /// let part1 = gcs
    ///     .put_part(&path, &id, 1, PutPayload::from("the final part"))
    ///     .await?;
    ///
    /// // Finalize the upload. The parts must be provided in `part_idx` order.
    /// gcs.complete_multipart(&path, &id, vec![part0, part1]).await?;
    /// #     Ok(())
    /// # }
    /// ```
    ///
    /// Every part except the last must be at least 5 MiB. Parts may be uploaded
    /// concurrently and in any order, provided each is given the correct
    /// `part_idx`. See [Cloud Storage multipart upload limits] for the full set of
    /// size and count constraints.
    ///
    /// To discard an upload instead of completing it, call
    /// [`abort_multipart`] with the same `id`.
    ///
    /// [`ObjectStoreExt::put_multipart`]: crate::ObjectStoreExt::put_multipart
    /// [`complete_multipart`]: MultipartStore::complete_multipart
    /// [`abort_multipart`]: MultipartStore::abort_multipart
    /// [Cloud Storage multipart upload limits]: https://docs.cloud.google.com/storage/docs/xml-api/post-object-complete
    async fn create_multipart(&self, path: &Path) -> Result<MultipartId> {
        self.client
            .multipart_initiate(path, PutMultipartOptions::default())
            .await
    }

    async fn create_multipart_opts(
        &self,
        path: &Path,
        opts: PutMultipartOptions,
    ) -> Result<MultipartId> {
        self.client.multipart_initiate(path, opts).await
    }

    async fn put_part(
        &self,
        path: &Path,
        id: &MultipartId,
        part_idx: usize,
        payload: PutPayload,
    ) -> Result<PartId> {
        self.client.put_part(path, id, part_idx, payload).await
    }

    async fn complete_multipart(
        &self,
        path: &Path,
        id: &MultipartId,
        parts: Vec<PartId>,
    ) -> Result<PutResult> {
        let result = self.client.multipart_complete(path, id, parts).await?;
        Ok(self.put_result(result))
    }

    async fn abort_multipart(&self, path: &Path, id: &MultipartId) -> Result<()> {
        self.client.multipart_cleanup(path, id).await
    }
}

#[async_trait]
impl Signer for GoogleCloudStorage {
    async fn signed_url(&self, method: Method, path: &Path, expires_in: Duration) -> Result<Url> {
        self.signed_url_opts(method, path, expires_in, &SignedUrlOptions::default())
            .await
    }

    /// Create a signed URL, additionally folding the query parameters and headers in `options`
    /// into the signature.
    ///
    /// `extra_query` lets callers sign query parameters the recipient must send, and
    /// `signed_headers` binds request headers (e.g. `content-type`) to the signature; the
    /// recipient must send exactly these headers and values.
    async fn signed_url_opts(
        &self,
        method: Method,
        path: &Path,
        expires_in: Duration,
        options: &SignedUrlOptions,
    ) -> Result<Url> {
        if expires_in.as_secs() > 604800 {
            return Err(crate::Error::Generic {
                store: STORE,
                source: "Expiration Time can't be longer than 604800 seconds (7 days).".into(),
            });
        }

        // Validate the caller-provided extras up front, rejecting reserved query parameters and
        // headers controlled by the signer.
        validate_signed_url_extras(
            STORE,
            &options.extra_query,
            &options.signed_headers,
            "x-goog-",
        )?;

        let config = self.client.config();
        let path_url = config.path_url(path);
        let mut url = Url::parse(&path_url).map_err(|e| crate::Error::Generic {
            store: STORE,
            source: format!("Unable to parse url {path_url}: {e}").into(),
        })?;

        let signing_credentials = self.signing_credentials().get_credential().await?;
        let authorizer = GCSAuthorizer::new(signing_credentials);

        let crypto = crypto_provider(self.client.config().crypto.as_deref())?;
        authorizer
            .sign_with(
                crypto,
                method,
                &mut url,
                &options.extra_query,
                &options.signed_headers,
                expires_in,
                &self.client,
            )
            .await?;

        Ok(url)
    }
}

#[async_trait]
impl PaginatedListStore for GoogleCloudStorage {
    async fn list_paginated(
        &self,
        prefix: Option<&str>,
        opts: PaginatedListOptions,
    ) -> Result<PaginatedListResult> {
        let mut result = self.client.list_request(prefix, opts).await?;
        result.result.objects = result.result.objects.into_iter().map(|meta| self.metadata(meta)).collect();
        Ok(result)
    }
}

#[cfg(test)]
mod test {
    use credential::DEFAULT_GCS_BASE_URL;

    use crate::ObjectStoreExt;
    use crate::integration::*;
    use crate::tests::*;

    use super::*;

    const NON_EXISTENT_NAME: &str = "nonexistentname";

    #[tokio::test]
    async fn gcs_test() {
        maybe_skip_integration!();
        // tag the extensions of every HTTP response with a marker,
        // allowing response_extensions to verify their propagation
        let integration = GoogleCloudStorageBuilder::from_env()
            .with_http_connector(MarkerHttpConnector::default())
            .build()
            .unwrap();

        put_get_delete_list(&integration).await;
        list_with_offset_exclusivity(&integration).await;
        list_uses_directories_correctly(&integration).await;
        list_with_delimiter(&integration).await;
        rename_and_copy(&integration).await;
        if integration.client.config().base_url == DEFAULT_GCS_BASE_URL {
            // Fake GCS server doesn't currently honor ifGenerationMatch
            // https://github.com/fsouza/fake-gcs-server/issues/994
            copy_if_not_exists(&integration).await;
            // Fake GCS server does not yet implement XML Multipart uploads
            // https://github.com/fsouza/fake-gcs-server/issues/852
            stream_get(&integration).await;
            multipart(&integration, &integration).await;
            multipart_with_opts(&integration, &integration).await;
            multipart_put_part_out_of_order(&integration, &integration).await;
            multipart_race_condition(&integration, true).await;
            multipart_out_of_order(&integration).await;
            list_paginated(&integration, &integration).await;
            // Fake GCS server doesn't currently honor preconditions
            get_opts(&integration).await;
            put_opts(&integration, true).await;
            // Fake GCS server doesn't currently support attributes
            put_get_attributes(&integration).await;
        }

        // Fake GCS server does not yet implement XML Multipart uploads
        let test_multipart = integration.client.config().base_url == DEFAULT_GCS_BASE_URL;
        response_extensions(&integration, test_multipart).await;
    }

    #[cfg(feature = "reqwest")]
    #[tokio::test]
    #[ignore]
    async fn gcs_test_sign() {
        maybe_skip_integration!();
        let integration = GoogleCloudStorageBuilder::from_env().build().unwrap();

        let client = reqwest::Client::new();

        let path = Path::from("test_sign");
        let url = integration
            .signed_url(Method::PUT, &path, Duration::from_secs(3600))
            .await
            .unwrap();
        println!("PUT {url}");

        let resp = client.put(url).body("data").send().await.unwrap();
        resp.error_for_status().unwrap();

        let url = integration
            .signed_url(Method::GET, &path, Duration::from_secs(3600))
            .await
            .unwrap();
        println!("GET {url}");

        let resp = client.get(url).send().await.unwrap();
        let resp = resp.error_for_status().unwrap();
        let data = resp.bytes().await.unwrap();
        assert_eq!(data.as_ref(), b"data");
    }

    #[tokio::test]
    async fn gcs_test_get_nonexistent_location() {
        maybe_skip_integration!();
        let integration = GoogleCloudStorageBuilder::from_env().build().unwrap();

        let location = Path::from_iter([NON_EXISTENT_NAME]);

        let err = integration.get(&location).await.unwrap_err();

        assert!(
            matches!(err, crate::Error::NotFound { .. }),
            "unexpected error type: {err}"
        );
    }

    #[tokio::test]
    async fn gcs_test_get_nonexistent_bucket() {
        maybe_skip_integration!();
        let config = GoogleCloudStorageBuilder::from_env();
        let integration = config.with_bucket_name(NON_EXISTENT_NAME).build().unwrap();

        let location = Path::from_iter([NON_EXISTENT_NAME]);

        let err = get_nonexistent_object(&integration, Some(location))
            .await
            .unwrap_err();

        assert!(
            matches!(err, crate::Error::NotFound { .. }),
            "unexpected error type: {err}"
        );
    }

    #[tokio::test]
    async fn gcs_test_delete_nonexistent_location() {
        maybe_skip_integration!();
        let integration = GoogleCloudStorageBuilder::from_env().build().unwrap();

        let location = Path::from_iter([NON_EXISTENT_NAME]);

        let err = integration.delete(&location).await.unwrap_err();
        assert!(
            matches!(err, crate::Error::NotFound { .. }),
            "unexpected error type: {err}"
        );
    }

    #[tokio::test]
    async fn gcs_test_delete_nonexistent_bucket() {
        maybe_skip_integration!();
        let config = GoogleCloudStorageBuilder::from_env();
        let integration = config.with_bucket_name(NON_EXISTENT_NAME).build().unwrap();

        let location = Path::from_iter([NON_EXISTENT_NAME]);

        let err = integration.delete(&location).await.unwrap_err();
        assert!(
            matches!(err, crate::Error::NotFound { .. }),
            "unexpected error type: {err}"
        );
    }

    #[tokio::test]
    async fn gcs_test_put_nonexistent_bucket() {
        maybe_skip_integration!();
        let config = GoogleCloudStorageBuilder::from_env();
        let integration = config.with_bucket_name(NON_EXISTENT_NAME).build().unwrap();

        let location = Path::from_iter([NON_EXISTENT_NAME]);
        let data = PutPayload::from("arbitrary data");

        let err = integration
            .put(&location, data)
            .await
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("Server returned non-2xx status code: 404 Not Found"),
            "{}",
            err
        )
    }
}
