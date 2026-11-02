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

//! An object store implementation for S3
//!
//! See the [Feature Flags](crate#feature-flags) section for the difference
//! between the `aws` and `aws-base` features.
//!
//! ## Multipart uploads
//!
//! Multipart uploads can be initiated with the [`ObjectStore::put_multipart_opts`] method.
//!
//! If the writer fails for any reason, you may have parts uploaded to AWS but not
//! used that you will be charged for. [`MultipartUpload::abort`] may be invoked to drop
//! these unneeded parts, however, it is recommended that you consider implementing
//! [automatic cleanup] of unused parts that are older than some threshold.
//!
//! [automatic cleanup]: https://aws.amazon.com/blogs/aws/s3-lifecycle-management-update-support-for-multipart-uploads-and-delete-markers/

use async_trait::async_trait;
use futures_util::stream::BoxStream;
use futures_util::{StreamExt, TryStreamExt};
use http::header::{HeaderName, IF_MATCH, IF_NONE_MATCH};
use http::{Method, StatusCode};
use std::{sync::Arc, time::Duration};
use url::Url;

use crate::aws::client::{CompleteMultipartMode, PutPartPayload, RequestError, S3Client};
use crate::client::CredentialProvider;
use crate::client::get::GetClientExt;
use crate::client::list::{ListClient, ListClientExt};
use crate::multipart::{MultipartStore, PartId};
use crate::retry::{MultipartRetry, RetryPolicy};
use crate::signer::{SignedUrlOptions, Signer};
use crate::util::{STRICT_ENCODE_SET, validate_signed_url_extras};
use crate::{
    CopyMode, CopyOptions, Error, GetOptions, GetResult, ListResult, MultipartId, MultipartUpload,
    ObjectMeta, ObjectStore, Path, PutMode, PutMultipartOptions, PutOptions, PutPayload, PutResult,
    Result, UploadPart,
};

static TAGS_HEADER: HeaderName = HeaderName::from_static("x-amz-tagging");
static COPY_SOURCE_HEADER: HeaderName = HeaderName::from_static("x-amz-copy-source");

mod builder;
mod checksum;
mod client;
mod credential;
mod precondition;

#[cfg(all(feature = "reqwest", not(target_arch = "wasm32")))]
mod resolve;

pub use builder::{AmazonS3Builder, AmazonS3ConfigKey};
pub use checksum::Checksum;
pub use precondition::{S3ConditionalPut, S3CopyIfNotExists};

#[cfg(all(feature = "reqwest", not(target_arch = "wasm32")))]
pub use resolve::resolve_bucket_region;

/// This struct is used to maintain the URI path encoding
const STRICT_PATH_ENCODE_SET: percent_encoding::AsciiSet = STRICT_ENCODE_SET.remove(b'/');

const STORE: &str = "S3";

/// [`CredentialProvider`] for [`AmazonS3`]
pub type AwsCredentialProvider = Arc<dyn CredentialProvider<Credential = AwsCredential>>;
use crate::client::parts::Parts;
use crate::list::{PaginatedListOptions, PaginatedListResult, PaginatedListStore};
pub use credential::{AwsAuthorizer, AwsCredential};

/// Interface for [Amazon S3](https://aws.amazon.com/s3/).
#[derive(Debug, Clone)]
pub struct AmazonS3 {
    client: Arc<S3Client>,
}

impl std::fmt::Display for AmazonS3 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "AmazonS3({})", self.client.config.bucket)
    }
}

impl AmazonS3 {
    /// Returns the [`AwsCredentialProvider`] used by [`AmazonS3`]
    pub fn credentials(&self) -> &AwsCredentialProvider {
        &self.client.config.credentials
    }

    /// Create a full URL to the resource specified by `path` with this instance's configuration.
    fn path_url(&self, path: &Path) -> String {
        self.client.config.path_url(path)
    }
}

#[async_trait]
impl Signer for AmazonS3 {
    /// Create a URL containing the relevant [AWS SigV4] query parameters that authorize a request
    /// via `method` to the resource at `path` valid for the duration specified in `expires_in`.
    ///
    /// [AWS SigV4]: https://docs.aws.amazon.com/IAM/latest/UserGuide/create-signed-request.html
    ///
    /// # Example
    ///
    /// This example returns a URL that will enable a user to upload a file to
    /// "some-folder/some-file.txt" in the next hour.
    ///
    /// ```
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # use object_store::{aws::AmazonS3Builder, path::Path, signer::Signer};
    /// # use http::Method;
    /// # use std::time::Duration;
    /// #
    /// let region = "us-east-1";
    /// let s3 = AmazonS3Builder::new()
    ///     .with_region(region)
    ///     .with_bucket_name("my-bucket")
    ///     .with_access_key_id("my-access-key-id")
    ///     .with_secret_access_key("my-secret-access-key")
    ///     .build()?;
    ///
    /// let url = s3.signed_url(
    ///     Method::PUT,
    ///     &Path::from("some-folder/some-file.txt"),
    ///     Duration::from_secs(60 * 60)
    /// ).await?;
    /// #     Ok(())
    /// # }
    /// ```
    async fn signed_url(&self, method: Method, path: &Path, expires_in: Duration) -> Result<Url> {
        self.signed_url_opts(method, path, expires_in, &SignedUrlOptions::default())
            .await
    }

    /// Create a signed URL, additionally folding the query parameters and headers in `options`
    /// into the SigV4 signature.
    ///
    /// `extra_query` lets callers sign query parameters that the recipient must send, such as
    /// `partNumber` and `uploadId` for a multipart `UploadPart`, or a `versionId`. `signed_headers`
    /// binds specific request headers (e.g. `x-amz-checksum-sha256`, `content-type`, SSE headers)
    /// to the signature; the recipient must send exactly these headers and values.
    ///
    /// # Example
    ///
    /// ```
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # use object_store::{aws::AmazonS3Builder, path::Path, signer::{Signer, SignedUrlOptions}};
    /// # use http::Method;
    /// # use std::time::Duration;
    /// #
    /// let s3 = AmazonS3Builder::new()
    ///     .with_region("us-east-1")
    ///     .with_bucket_name("my-bucket")
    ///     .with_access_key_id("my-access-key-id")
    ///     .with_secret_access_key("my-secret-access-key")
    ///     .build()?;
    ///
    /// // Presign a multipart UploadPart request.
    /// let options = SignedUrlOptions::default()
    ///     .with_query([("partNumber", "1"), ("uploadId", "abc123")]);
    /// let url = s3.signed_url_opts(
    ///     Method::PUT,
    ///     &Path::from("some-folder/some-file.txt"),
    ///     Duration::from_secs(60 * 60),
    ///     &options,
    /// ).await?;
    /// #     Ok(())
    /// # }
    /// ```
    ///
    /// See [`AmazonS3::create_multipart`] for a complete end-to-end example that presigns every
    /// part of a multipart upload so a credential-less client can upload them.
    ///
    /// [`AmazonS3::create_multipart`]: crate::aws::AmazonS3::create_multipart
    async fn signed_url_opts(
        &self,
        method: Method,
        path: &Path,
        expires_in: Duration,
        options: &SignedUrlOptions,
    ) -> Result<Url> {
        // Validate the caller-provided extras up front, rejecting reserved query parameters and
        // headers controlled by the signer.
        validate_signed_url_extras(
            STORE,
            &options.extra_query,
            &options.signed_headers,
            "x-amz-",
        )?;

        let crypto = self.client.config.crypto()?;
        let credential = self.credentials().get_credential().await?;
        let authorizer = AwsAuthorizer::new(&credential, "s3", &self.client.config.region)
            .with_request_payer(self.client.config.request_payer)
            .with_crypto(crypto);

        let path_url = self.path_url(path);
        let mut url = path_url.parse().map_err(|e| Error::Generic {
            store: STORE,
            source: format!("Unable to parse url {path_url}: {e}").into(),
        })?;

        authorizer.sign_with(
            method,
            &mut url,
            &options.extra_query,
            &options.signed_headers,
            expires_in,
        )?;

        Ok(url)
    }
}

#[async_trait]
impl ObjectStore for AmazonS3 {
    async fn put_opts(
        &self,
        location: &Path,
        payload: PutPayload,
        opts: PutOptions,
    ) -> Result<PutResult> {
        let PutOptions {
            mode,
            tags,
            attributes,
            extensions,
        } = opts;

        let request = self
            .client
            .request(Method::PUT, location)
            .with_payload(payload)?
            .with_attributes(attributes)
            .with_tags(tags)
            .with_extensions(extensions)
            .with_encryption_headers();

        match (mode, &self.client.config.conditional_put) {
            (PutMode::Overwrite, _) => request.idempotent(true).do_put().await,
            (PutMode::Create, S3ConditionalPut::Disabled) => Err(Error::NotImplemented {
                operation:
                    "`put_opts` with mode `PutMode::Create` when conditional put is disabled".into(),
                implementer: self.to_string(),
            }),
            (PutMode::Create, S3ConditionalPut::ETagMatch) => {
                match request.header(&IF_NONE_MATCH, "*").do_put().await {
                    // Technically If-None-Match should return NotModified but some stores,
                    // such as R2, instead return PreconditionFailed
                    // https://developers.cloudflare.com/r2/api/s3/extensions/#conditional-operations-in-putobject
                    Err(e @ Error::NotModified { .. } | e @ Error::Precondition { .. }) => {
                        Err(Error::AlreadyExists {
                            path: location.to_string(),
                            source: Box::new(e),
                        })
                    }
                    r => r,
                }
            }
            (PutMode::Update(v), put) => {
                let etag = v.e_tag.ok_or_else(|| Error::Generic {
                    store: STORE,
                    source: "ETag required for conditional put".to_string().into(),
                })?;
                match put {
                    S3ConditionalPut::ETagMatch => {
                        match request
                            .header(&IF_MATCH, etag.as_str())
                            // Real S3 will occasionally report 409 Conflict
                            // if there are concurrent `If-Match` requests
                            // in flight, so we need to be prepared to retry
                            // 409 responses.
                            .retry_on_conflict(true)
                            .do_put()
                            .await
                        {
                            // Real S3 reports NotFound rather than PreconditionFailed when the
                            // object doesn't exist. Convert to PreconditionFailed for
                            // consistency with R2. This also matches what the HTTP spec
                            // says the behavior should be.
                            Err(Error::NotFound { path, source }) => {
                                Err(Error::Precondition { path, source })
                            }
                            r => r,
                        }
                    }
                    S3ConditionalPut::Disabled => Err(Error::NotImplemented {
                        operation:
                            "`put_opts` with mode `PutMode::Update` when conditional put is disabled"
                                .into(),
                        implementer: self.to_string(),
                    }),
                }
            }
        }
    }

    async fn put_multipart_opts(
        &self,
        location: &Path,
        opts: PutMultipartOptions,
    ) -> Result<Box<dyn MultipartUpload>> {
        let retry_policy = opts.retry_policy();
        let upload_id = self.client.create_multipart(location, opts).await?;

        Ok(Box::new(S3MultiPartUpload {
            part_idx: 0,
            state: Arc::new(UploadState {
                client: Arc::clone(&self.client),
                location: location.clone(),
                upload_id: upload_id.clone(),
                parts: Default::default(),
                retry_policy,
            }),
        }))
    }

    async fn get_opts(&self, location: &Path, options: GetOptions) -> Result<GetResult> {
        self.client.get_opts(location, options).await
    }

    async fn delete_if_match(&self, location: &Path, etag: &str) -> Result<()> {
        self.client.delete_if_match(location, etag).await
    }

    fn delete_stream(
        &self,
        locations: BoxStream<'static, Result<Path>>,
    ) -> BoxStream<'static, Result<Path>> {
        let client = Arc::clone(&self.client);

        // Some S3-compatible providers do not implement
        // the bulk `DeleteObjects` API (`POST /?delete`). When bulk delete is
        // disabled, fall back to parallel single-object `DELETE /key` requests,
        // which are part of the core S3 API supported by every provider.
        if client.config.disable_bulk_delete {
            return locations
                .map(move |location| {
                    let client = Arc::clone(&client);
                    async move {
                        let location = location?;
                        client.delete_request(&location).await?;
                        Ok(location)
                    }
                })
                .buffered(20)
                .boxed();
        }

        locations
            .try_chunks(1_000)
            .map(move |locations| {
                let client = Arc::clone(&client);
                async move {
                    // Early return the error. We ignore the paths that have already been
                    // collected into the chunk.
                    let locations = locations.map_err(|e| e.1)?;
                    client
                        .bulk_delete_request(locations)
                        .await
                        .map(futures_util::stream::iter)
                }
            })
            .buffered(20)
            .try_flatten()
            .boxed()
    }

    fn list(&self, prefix: Option<&Path>) -> BoxStream<'static, Result<ObjectMeta>> {
        self.client.list(prefix)
    }

    fn list_with_offset(
        &self,
        prefix: Option<&Path>,
        offset: &Path,
    ) -> BoxStream<'static, Result<ObjectMeta>> {
        if self.client.config.is_s3_express() {
            let offset = offset.clone();
            // S3 Express does not support start-after
            return self
                .client
                .list(prefix)
                .try_filter(move |f| futures_util::future::ready(f.location > offset))
                .boxed();
        }

        self.client.list_with_offset(prefix, offset)
    }

    async fn list_with_delimiter(&self, prefix: Option<&Path>) -> Result<ListResult> {
        self.client.list_with_delimiter(prefix).await
    }

    async fn copy_opts(&self, from: &Path, to: &Path, options: CopyOptions) -> Result<()> {
        let CopyOptions {
            mode,
            extensions: _,
        } = options;

        match mode {
            CopyMode::Overwrite => {
                self.client
                    .copy_request(from, to)
                    .idempotent(true)
                    .send()
                    .await?;
                Ok(())
            }
            CopyMode::Create => {
                let (k, v, status) = match &self.client.config.copy_if_not_exists {
                    Some(S3CopyIfNotExists::Header(k, v)) => {
                        (k, v, StatusCode::PRECONDITION_FAILED)
                    }
                    Some(S3CopyIfNotExists::HeaderWithStatus(k, v, status)) => (k, v, *status),
                    Some(S3CopyIfNotExists::Multipart) => {
                        let upload_id = self
                            .client
                            .create_multipart(to, PutMultipartOptions::default())
                            .await?;

                        let res = async {
                            let part_id = self
                                .client
                                .put_part(to, &upload_id, 0, PutPartPayload::Copy(from))
                                .await?;
                            match self
                                .client
                                .complete_multipart(
                                    to,
                                    &upload_id,
                                    vec![part_id],
                                    CompleteMultipartMode::Create,
                                )
                                .await
                            {
                                Err(e @ Error::Precondition { .. }) => Err(Error::AlreadyExists {
                                    path: to.to_string(),
                                    source: Box::new(e),
                                }),
                                Ok(_) => Ok(()),
                                Err(e) => Err(e),
                            }
                        }
                        .await;

                        // If the multipart upload failed, make a best effort attempt to
                        // clean it up. It's the caller's responsibility to add a
                        // lifecycle rule if guaranteed cleanup is required, as we
                        // cannot protect against an ill-timed process crash.
                        if res.is_err() {
                            let _ = self.client.abort_multipart(to, &upload_id).await;
                        }

                        return res;
                    }
                    None => {
                        return Err(Error::NotSupported {
                            source: "S3 does not support copy-if-not-exists".to_string().into(),
                        });
                    }
                };

                let req = self.client.copy_request(from, to);
                match req.header(k, v).send().await {
                    Err(RequestError::Retry { source, path })
                        if source.status() == Some(status) =>
                    {
                        Err(Error::AlreadyExists {
                            source: Box::new(source),
                            path,
                        })
                    }
                    Err(e) => Err(e.into()),
                    Ok(_) => Ok(()),
                }
            }
        }
    }
}

#[derive(Debug)]
struct S3MultiPartUpload {
    part_idx: usize,
    state: Arc<UploadState>,
}

#[derive(Debug)]
struct UploadState {
    parts: Parts,
    location: Path,
    upload_id: String,
    client: Arc<S3Client>,
    retry_policy: Option<Arc<dyn RetryPolicy>>,
}

#[async_trait]
impl MultipartUpload for S3MultiPartUpload {
    fn put_part(&mut self, data: PutPayload) -> UploadPart {
        let idx = self.part_idx;
        self.part_idx += 1;
        let state = Arc::clone(&self.state);
        Box::pin(async move {
            let mut retry = MultipartRetry::new(state.retry_policy.clone());
            loop {
                match state
                    .client
                    .put_part(
                        &state.location,
                        &state.upload_id,
                        idx,
                        PutPartPayload::Part(data.clone()),
                    )
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

        self.state
            .client
            .complete_multipart(
                &self.state.location,
                &self.state.upload_id,
                parts,
                CompleteMultipartMode::Overwrite,
            )
            .await
    }

    async fn abort(&mut self) -> Result<()> {
        self.state
            .client
            .request(Method::DELETE, &self.state.location)
            .query(&[("uploadId", &self.state.upload_id)])
            .idempotent(true)
            .send()
            .await?;

        Ok(())
    }
}

#[async_trait]
impl MultipartStore for AmazonS3 {
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
    /// # use object_store::{aws::AmazonS3Builder, multipart::MultipartStore, path::Path, PutPayload};
    /// #
    /// let s3 = AmazonS3Builder::new()
    ///     .with_region("us-east-1")
    ///     .with_bucket_name("my-bucket")
    ///     .with_access_key_id("my-access-key-id")
    ///     .with_secret_access_key("my-secret-access-key")
    ///     .build()?;
    ///
    /// let path = Path::from("data/large_file");
    ///
    /// // Start the upload, obtaining an id used to reference it in later calls
    /// let id = s3.create_multipart(&path).await?;
    ///
    /// // Upload the individual parts. Every part except the last must be at least
    /// // 5 MiB.
    /// let part0 = s3
    ///     .put_part(&path, &id, 0, PutPayload::from(vec![0; 5 * 1024 * 1024]))
    ///     .await?;
    /// let part1 = s3
    ///     .put_part(&path, &id, 1, PutPayload::from("the final part"))
    ///     .await?;
    ///
    /// // Finalize the upload. The parts must be provided in `part_idx` order.
    /// s3.complete_multipart(&path, &id, vec![part0, part1]).await?;
    /// #     Ok(())
    /// # }
    /// ```
    ///
    /// Every part except the last must be at least 5 MiB. Parts may be uploaded
    /// concurrently and in any order, provided each is given the correct
    /// `part_idx`. See [Amazon S3 multipart upload limits] for the full set of
    /// size and count constraints.
    ///
    /// To discard an upload instead of completing it, call
    /// [`abort_multipart`] with the same `id`.
    ///
    /// # Example: presigned multipart upload
    ///
    /// The same upload can be driven by a client that holds no credentials. The credential holder
    /// starts the upload and presigns an `UploadPart` URL for each part with
    /// [`Signer::signed_url_opts`]; the client `PUT`s its bytes to that URL and returns the `ETag`
    /// from the response; the credential holder then completes the upload with those ETags. This
    /// is how you hand out per-part upload URLs without sharing credentials.
    ///
    /// ```no_run
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # use object_store::{aws::AmazonS3Builder, multipart::{MultipartStore, PartId}, path::Path};
    /// # use object_store::signer::{Signer, SignedUrlOptions};
    /// # use http::Method;
    /// # use std::time::Duration;
    /// #
    /// let s3 = AmazonS3Builder::new()
    ///     .with_region("us-east-1")
    ///     .with_bucket_name("my-bucket")
    ///     .with_access_key_id("my-access-key-id")
    ///     .with_secret_access_key("my-secret-access-key")
    ///     .build()?;
    ///
    /// let path = Path::from("data/large_file");
    ///
    /// // Credential holder: start the upload.
    /// let id = s3.create_multipart(&path).await?;
    ///
    /// // Credential holder: presign an `UploadPart` URL for each part. `partNumber` and `uploadId`
    /// // must be signed query parameters, or S3 rejects the request.
    /// let mut part_urls = Vec::new();
    /// for part_number in 1..=3 {
    ///     let options = SignedUrlOptions::default().with_query([
    ///         ("partNumber", part_number.to_string()),
    ///         ("uploadId", id.clone()),
    ///     ]);
    ///     let url = s3
    ///         .signed_url_opts(Method::PUT, &path, Duration::from_secs(60 * 60), &options)
    ///         .await?;
    ///     part_urls.push(url);
    /// }
    ///
    /// // Client (no credentials): `PUT` each part's bytes to its URL, read the `ETag` response
    /// // header, and return the ETags to the credential holder in part order, e.g. with `reqwest`:
    /// //
    /// //     let resp = reqwest::Client::new().put(url).body(bytes).send().await?;
    /// //     let etag = resp.headers()[http::header::ETAG].to_str()?.to_string();
    /// let etags: Vec<String> = // returned by the client
    /// #     vec![];
    ///
    /// // Credential holder: complete the upload with the collected ETags.
    /// let parts = etags.into_iter().map(|content_id| PartId { content_id }).collect();
    /// s3.complete_multipart(&path, &id, parts).await?;
    /// #     Ok(())
    /// # }
    /// ```
    ///
    /// [`ObjectStoreExt::put_multipart`]: crate::ObjectStoreExt::put_multipart
    /// [`complete_multipart`]: MultipartStore::complete_multipart
    /// [`abort_multipart`]: MultipartStore::abort_multipart
    /// [`Signer::signed_url_opts`]: crate::signer::Signer::signed_url_opts
    /// [Amazon S3 multipart upload limits]: https://docs.aws.amazon.com/AmazonS3/latest/userguide/qfacts.html
    async fn create_multipart(&self, path: &Path) -> Result<MultipartId> {
        self.client
            .create_multipart(path, PutMultipartOptions::default())
            .await
    }

    async fn create_multipart_opts(
        &self,
        path: &Path,
        opts: PutMultipartOptions,
    ) -> Result<MultipartId> {
        self.client.create_multipart(path, opts).await
    }

    async fn put_part(
        &self,
        path: &Path,
        id: &MultipartId,
        part_idx: usize,
        data: PutPayload,
    ) -> Result<PartId> {
        self.client
            .put_part(path, id, part_idx, PutPartPayload::Part(data))
            .await
    }

    async fn complete_multipart(
        &self,
        path: &Path,
        id: &MultipartId,
        parts: Vec<PartId>,
    ) -> Result<PutResult> {
        self.client
            .complete_multipart(path, id, parts, CompleteMultipartMode::Overwrite)
            .await
    }

    async fn abort_multipart(&self, path: &Path, id: &MultipartId) -> Result<()> {
        self.client
            .request(Method::DELETE, path)
            .query(&[("uploadId", id)])
            .send()
            .await?;
        Ok(())
    }
}

#[async_trait]
impl PaginatedListStore for AmazonS3 {
    async fn list_paginated(
        &self,
        prefix: Option<&str>,
        opts: PaginatedListOptions,
    ) -> Result<PaginatedListResult> {
        self.client.list_request(prefix, opts).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ClientOptions;
    use crate::ObjectStoreExt;
    #[cfg(feature = "reqwest")]
    use crate::client::SpawnedReqwestConnector;
    use crate::client::get::GetClient;
    #[cfg(feature = "reqwest")]
    use crate::client::mock_server::MockServer;
    use crate::client::retry::RetryContext;
    use crate::integration::*;
    use crate::tests::*;
    use base64::Engine;
    use base64::prelude::BASE64_STANDARD;
    use http::HeaderMap;
    use http::HeaderValue;
    #[cfg(feature = "reqwest")]
    use http::Response;
    #[cfg(feature = "reqwest")]
    use http::header::ETAG;
    #[cfg(feature = "reqwest")]
    use std::sync::atomic::{AtomicUsize, Ordering};

    const NON_EXISTENT_NAME: &str = "nonexistentname";

    /// Presigned-URL tests run against a dedicated bucket so their concurrent writes cannot
    /// contaminate the shared `test-bucket`, whose exact contents `s3_test` asserts.
    ///
    /// Set `OBJECT_STORE_SIGNING_BUCKET` to run them against a bucket in your own account
    /// (e.g. real S3) without editing this source; it defaults to `test-bucket-for-signing`.
    const DEFAULT_SIGNING_BUCKET: &str = "test-bucket-for-signing";

    fn signing_bucket() -> String {
        std::env::var("OBJECT_STORE_SIGNING_BUCKET")
            .unwrap_or_else(|_| DEFAULT_SIGNING_BUCKET.to_string())
    }

    /// Builds the signing store and preflights write access with an authenticated `PutObject`
    /// probe that fails fast, with an actionable message, if the credentials cannot write to the
    /// bucket. Without this a permissions problem surfaces as an opaque `403 AccessDenied` panic
    /// deep inside whichever presign assertion happens to run first (which is exactly what made a
    /// reviewer's real-S3 run hard to diagnose).
    async fn signing_store() -> AmazonS3 {
        let store = AmazonS3Builder::from_env()
            .with_bucket_name(signing_bucket())
            .build()
            .unwrap();

        let bucket = signing_bucket();
        let probe = Path::from(".object_store_presign_preflight");
        if let Err(source) = store.put(&probe, PutPayload::from_static(b"ok")).await {
            panic!(
                "presign test preflight failed: the configured credentials cannot write to \
                 bucket `{bucket}`.\n\
                 Verify the bucket exists, is in your configured region, and that the credentials \
                 (from the AWS_* env vars read by `from_env`) can access it. The tests exercise \
                 s3:PutObject, s3:GetObject, and s3:DeleteObject; this preflight checks writes. \
                 Point at a different bucket with OBJECT_STORE_SIGNING_BUCKET.\n\
                 Underlying error: {source}"
            );
        }
        // Best-effort cleanup; the probe object is harmless if it lingers.
        let _ = store.delete(&probe).await;

        store
    }

    #[cfg(feature = "reqwest")]
    #[derive(Debug, Default)]
    struct RetryOnce(AtomicUsize);

    #[cfg(feature = "reqwest")]
    #[async_trait]
    impl RetryPolicy for RetryOnce {
        async fn retry(&self, _: crate::retry::RetryContext) -> bool {
            self.0.fetch_add(1, Ordering::SeqCst) == 0
        }
    }

    #[cfg(feature = "reqwest")]
    #[tokio::test]
    async fn retries_multipart_part_operation() {
        let mock = MockServer::new().await;
        mock.push(
            Response::builder()
                .status(StatusCode::OK)
                .body(
                    "<InitiateMultipartUploadResult><UploadId>upload-id</UploadId></InitiateMultipartUploadResult>"
                        .to_string(),
                )
                .unwrap(),
        );
        mock.push(
            Response::builder()
                .status(StatusCode::SERVICE_UNAVAILABLE)
                .body(String::new())
                .unwrap(),
        );
        mock.push(
            Response::builder()
                .status(StatusCode::OK)
                .header(ETAG, "etag")
                .body(String::new())
                .unwrap(),
        );

        let store = AmazonS3Builder::new()
            .with_endpoint(mock.url())
            .with_bucket_name("test-bucket")
            .with_region("us-east-1")
            .with_allow_http(true)
            .with_skip_signature(true)
            .with_retry(crate::RetryConfig {
                max_retries: 0,
                ..Default::default()
            })
            .build()
            .unwrap();
        let policy = Arc::new(RetryOnce::default());
        let opts = PutMultipartOptions::default()
            .with_retry_policy(Arc::clone(&policy) as Arc<dyn RetryPolicy>);

        let mut upload = store
            .put_multipart_opts(&Path::from("multipart"), opts)
            .await
            .unwrap();
        upload.put_part(PutPayload::from("data")).await.unwrap();

        assert_eq!(policy.0.load(Ordering::SeqCst), 1);
        mock.shutdown().await;
    }

    #[tokio::test]
    async fn write_multipart_file_with_signature() {
        maybe_skip_integration!();

        let bucket = "test-bucket-for-checksum";
        for checksum in [Checksum::SHA256, Checksum::CRC64NVME] {
            let store = AmazonS3Builder::from_env()
                .with_bucket_name(bucket)
                .with_checksum_algorithm(checksum)
                .build()
                .unwrap();

            let str = "test.bin";
            let path = Path::parse(str).unwrap();
            let opts = PutMultipartOptions::default();
            let mut upload = store.put_multipart_opts(&path, opts).await.unwrap();

            upload
                .put_part(PutPayload::from(vec![0u8; 10_000_000]))
                .await
                .unwrap();
            upload
                .put_part(PutPayload::from(vec![0u8; 5_000_000]))
                .await
                .unwrap();

            let res = upload.complete().await.unwrap();
            assert!(res.e_tag.is_some(), "Should have valid etag");

            store.delete(&path).await.unwrap();
        }
    }

    #[tokio::test]
    async fn signed_url_multipart_upload() {
        maybe_skip_integration!();

        // Exercises presigning a multipart `UploadPart` request: the `partNumber` and `uploadId`
        // query parameters must be folded into the signature, and the resulting URL must be
        // usable by a client that has no access to the credentials.
        let integration = signing_store().await;

        let path = Path::from("test_signed_multipart_upload.bin");
        let _ = integration.delete(&path).await;

        let upload_id = integration.create_multipart(&path).await.unwrap();

        let part = vec![42u8; 1024];
        let options = SignedUrlOptions::default()
            .with_query([("partNumber", "1"), ("uploadId", upload_id.as_str())]);
        let url = integration
            .signed_url_opts(Method::PUT, &path, Duration::from_secs(300), &options)
            .await
            .unwrap();

        // Upload the part using only the presigned URL, as a credential-less client would.
        let resp = reqwest::Client::new()
            .put(url)
            .body(part.clone())
            .send()
            .await
            .unwrap();
        assert!(
            resp.status().is_success(),
            "UploadPart via presigned URL failed: {resp:?}"
        );
        let etag = resp
            .headers()
            .get(http::header::ETAG)
            .expect("ETag in UploadPart response")
            .to_str()
            .unwrap()
            .to_string();

        integration
            .complete_multipart(&path, &upload_id, vec![PartId { content_id: etag }])
            .await
            .unwrap();

        let got = integration.get(&path).await.unwrap().bytes().await.unwrap();
        assert_eq!(got.as_ref(), part.as_slice());

        integration.delete(&path).await.unwrap();
    }

    #[tokio::test]
    async fn signed_url_with_signed_checksum_header_is_enforced() {
        maybe_skip_integration!();
        maybe_skip_signature_enforcement!();

        // Presign a PUT that binds `x-amz-checksum-sha256` to a fixed value. This is the
        // storage-enforced-checksum path: the server must accept a body matching the signed
        // checksum and reject one that does not, and the header is part of the signature so it
        // cannot be omitted.
        let integration = signing_store().await;

        let path = Path::from("test_signed_checksum.bin");
        let _ = integration.delete(&path).await;

        let body = b"hello world".to_vec();
        // base64(sha256(b"hello world")), computed independently.
        const CHECKSUM: &str = "uU0nuZNNPgilLlLX2n2r+sSE7+N6U4DukIj3rOLvzek=";

        let options = SignedUrlOptions::default().with_signed_header(
            HeaderName::from_static("x-amz-checksum-sha256"),
            HeaderValue::from_static(CHECKSUM),
        );
        let url = integration
            .signed_url_opts(Method::PUT, &path, Duration::from_secs(300), &options)
            .await
            .unwrap();

        let client = reqwest::Client::new();

        // 1. Matching body + checksum header is accepted.
        let ok = client
            .put(url.clone())
            .header("x-amz-checksum-sha256", CHECKSUM)
            .body(body.clone())
            .send()
            .await
            .unwrap();
        assert!(
            ok.status().is_success(),
            "matching checksum rejected: {ok:?}"
        );

        // 2. A body that does not match the signed checksum is rejected (server enforces it).
        let bad_body = client
            .put(url.clone())
            .header("x-amz-checksum-sha256", CHECKSUM)
            .body(b"tampered content".to_vec())
            .send()
            .await
            .unwrap();
        assert!(
            !bad_body.status().is_success(),
            "mismatched body was NOT rejected: {bad_body:?}"
        );

        // 3. Omitting the signed header is rejected (the header is bound to the signature).
        let missing_header = client.put(url).body(body.clone()).send().await.unwrap();
        assert!(
            !missing_header.status().is_success(),
            "missing signed header was NOT rejected: {missing_header:?}"
        );

        let got = integration.get(&path).await.unwrap().bytes().await.unwrap();
        assert_eq!(got.as_ref(), body.as_slice());

        integration.delete(&path).await.unwrap();
    }

    #[tokio::test]
    async fn signed_url_with_signed_content_type_is_enforced() {
        maybe_skip_integration!();
        maybe_skip_signature_enforcement!();

        // Presign a PUT binding a `content-type` (with internal whitespace). The recipient must
        // send exactly this value; a different one breaks the signature.
        let integration = signing_store().await;

        let path = Path::from("test_signed_content_type.bin");
        let _ = integration.delete(&path).await;

        let body = b"some body".to_vec();
        const CONTENT_TYPE_VALUE: &str = "text/plain; charset=utf-8";

        let options = SignedUrlOptions::default().with_signed_header(
            http::header::CONTENT_TYPE,
            HeaderValue::from_static(CONTENT_TYPE_VALUE),
        );
        let url = integration
            .signed_url_opts(Method::PUT, &path, Duration::from_secs(300), &options)
            .await
            .unwrap();

        let client = reqwest::Client::new();

        // Matching content-type is accepted.
        let ok = client
            .put(url.clone())
            .header(http::header::CONTENT_TYPE, CONTENT_TYPE_VALUE)
            .body(body.clone())
            .send()
            .await
            .unwrap();
        assert!(
            ok.status().is_success(),
            "matching content-type rejected: {ok:?}"
        );

        // A different content-type is rejected (the header is bound to the signature).
        let wrong = client
            .put(url)
            .header(http::header::CONTENT_TYPE, "application/octet-stream")
            .body(body)
            .send()
            .await
            .unwrap();
        assert!(
            !wrong.status().is_success(),
            "mismatched content-type was NOT rejected: {wrong:?}"
        );

        integration.delete(&path).await.unwrap();
    }

    #[tokio::test]
    async fn signed_url_query_value_with_space() {
        maybe_skip_integration!();

        // A signed query value containing a space must be `%20`-encoded so the URL bytes match
        // the canonical query string; a real server rejects the signature otherwise. Uses a GET
        // with a `response-content-disposition` override, whose value contains spaces.
        let integration = signing_store().await;

        let path = Path::from("test_signed_query_space.bin");
        let body = b"contents".to_vec();
        integration.put(&path, body.clone().into()).await.unwrap();

        let options = SignedUrlOptions::default().with_query([(
            "response-content-disposition",
            "attachment; filename=\"a b.txt\"",
        )]);
        let url = integration
            .signed_url_opts(Method::GET, &path, Duration::from_secs(300), &options)
            .await
            .unwrap();

        let resp = reqwest::Client::new().get(url).send().await.unwrap();
        assert!(
            resp.status().is_success(),
            "GET with space-containing signed query rejected: {resp:?}"
        );
        // The server honoured the override, echoing it back in the response.
        let disposition = resp
            .headers()
            .get(http::header::CONTENT_DISPOSITION)
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default()
            .to_string();
        let got = resp.bytes().await.unwrap();
        assert_eq!(got.as_ref(), body.as_slice());
        assert!(
            disposition.contains("a b.txt"),
            "unexpected content-disposition: {disposition:?}"
        );

        integration.delete(&path).await.unwrap();
    }

    #[tokio::test]
    async fn signed_url_baseline_roundtrip_without_options() {
        maybe_skip_integration!();

        // Regression: the no-options path (now routed through `signed_url_opts`) still produces a
        // working presigned PUT and GET.
        let integration = signing_store().await;
        let path = Path::from("test_signed_baseline.bin");
        let _ = integration.delete(&path).await;
        let body = b"baseline body".to_vec();
        let client = reqwest::Client::new();

        let put_url = integration
            .signed_url(Method::PUT, &path, Duration::from_secs(300))
            .await
            .unwrap();
        let put = client.put(put_url).body(body.clone()).send().await.unwrap();
        assert!(put.status().is_success(), "baseline PUT failed: {put:?}");

        let get_url = integration
            .signed_url(Method::GET, &path, Duration::from_secs(300))
            .await
            .unwrap();
        let got = client.get(get_url).send().await.unwrap();
        assert!(got.status().is_success(), "baseline GET failed: {got:?}");
        assert_eq!(got.bytes().await.unwrap().as_ref(), body.as_slice());

        integration.delete(&path).await.unwrap();
    }

    #[tokio::test]
    async fn signed_url_multipart_multiple_parts_roundtrip() {
        maybe_skip_integration!();

        // Full multipart round trip with three parts (exercising S3's 5 MiB minimum-part rule)
        // and query parameters supplied in non-alphabetical order, proving the signer sorts the
        // canonical query string rather than signing in call order.
        let integration = signing_store().await;
        let path = Path::from("test_signed_multipart_large.bin");
        let _ = integration.delete(&path).await;

        const MIB: usize = 1024 * 1024;
        let parts_data = [vec![1u8; 6 * MIB], vec![2u8; 6 * MIB], vec![3u8; MIB]];

        let upload_id = integration.create_multipart(&path).await.unwrap();
        let client = reqwest::Client::new();
        let mut part_ids = Vec::new();

        for (idx, data) in parts_data.iter().enumerate() {
            let part_number = (idx + 1).to_string();
            // `uploadId` is supplied before `partNumber` — i.e. not in canonical (sorted) order.
            let options = SignedUrlOptions::default().with_query([
                ("uploadId", upload_id.as_str()),
                ("partNumber", part_number.as_str()),
            ]);
            let url = integration
                .signed_url_opts(Method::PUT, &path, Duration::from_secs(600), &options)
                .await
                .unwrap();
            let resp = client.put(url).body(data.clone()).send().await.unwrap();
            assert!(
                resp.status().is_success(),
                "UploadPart {part_number} failed: {resp:?}"
            );
            let etag = resp
                .headers()
                .get(http::header::ETAG)
                .expect("ETag")
                .to_str()
                .unwrap()
                .to_string();
            part_ids.push(PartId { content_id: etag });
        }

        integration
            .complete_multipart(&path, &upload_id, part_ids)
            .await
            .unwrap();

        let got = integration.get(&path).await.unwrap().bytes().await.unwrap();
        let expected: Vec<u8> = parts_data.concat();
        assert_eq!(got.len(), expected.len(), "assembled length mismatch");
        assert_eq!(
            got.as_ref(),
            expected.as_slice(),
            "assembled bytes mismatch"
        );

        integration.delete(&path).await.unwrap();
    }

    #[tokio::test]
    async fn signed_url_tampering_is_rejected() {
        maybe_skip_integration!();
        maybe_skip_signature_enforcement!();

        // Proves the signed query parameters and headers are actually bound to the signature:
        // mutating them must produce SignatureDoesNotMatch, while an extra *unsigned* header is
        // accepted (we don't over-constrain).
        let integration = signing_store().await;
        let path = Path::from("test_signed_tamper.bin");
        let _ = integration.delete(&path).await;
        let client = reqwest::Client::new();

        // --- query parameter tampering, in a multipart context ---
        let upload_id = integration.create_multipart(&path).await.unwrap();
        let options = SignedUrlOptions::default()
            .with_query([("partNumber", "1"), ("uploadId", upload_id.as_str())]);
        let url = integration
            .signed_url_opts(Method::PUT, &path, Duration::from_secs(300), &options)
            .await
            .unwrap();
        let url_str = url.as_str();

        // Mutating partNumber breaks the signature.
        let tampered_part = Url::parse(&url_str.replace("partNumber=1", "partNumber=2")).unwrap();
        let resp = client
            .put(tampered_part)
            .body(vec![0u8; 16])
            .send()
            .await
            .unwrap();
        assert_eq!(
            resp.status().as_u16(),
            403,
            "tampered partNumber was accepted: {resp:?}"
        );

        // Mutating uploadId breaks the signature.
        let bogus = format!("{}XXXX", upload_id);
        let tampered_id = Url::parse(&url_str.replace(upload_id.as_str(), bogus.as_str())).unwrap();
        let resp = client
            .put(tampered_id)
            .body(vec![0u8; 16])
            .send()
            .await
            .unwrap();
        assert_eq!(
            resp.status().as_u16(),
            403,
            "tampered uploadId was accepted: {resp:?}"
        );

        // An extra *unsigned* header is fine — only signed headers are bound. This also actually
        // uploads the part, leaving the multipart upload to be reaped by teardown.
        let resp = client
            .put(url)
            .header("x-custom-unsigned", "anything")
            .body(vec![0u8; 16])
            .send()
            .await
            .unwrap();
        assert!(
            resp.status().is_success(),
            "extra unsigned header was rejected: {resp:?}"
        );

        // --- signed header value tampering ---
        const CHECKSUM: &str = "uU0nuZNNPgilLlLX2n2r+sSE7+N6U4DukIj3rOLvzek=";
        let options = SignedUrlOptions::default().with_signed_header(
            HeaderName::from_static("x-amz-checksum-sha256"),
            HeaderValue::from_static(CHECKSUM),
        );
        let url = integration
            .signed_url_opts(Method::PUT, &path, Duration::from_secs(300), &options)
            .await
            .unwrap();
        // Sending a *different* (well-formed) value than was signed breaks the signature itself,
        // i.e. 403 rather than a checksum (400) error.
        let resp = client
            .put(url)
            .header(
                "x-amz-checksum-sha256",
                "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=",
            )
            .body(b"hello world".to_vec())
            .send()
            .await
            .unwrap();
        assert_eq!(
            resp.status().as_u16(),
            403,
            "tampered signed header value was accepted: {resp:?}"
        );

        integration.abort_multipart(&path, &upload_id).await.ok();
        let _ = integration.delete(&path).await;
    }

    #[tokio::test]
    async fn signed_url_special_character_key() {
        maybe_skip_integration!();

        // Proves path percent-encoding and signed query parameters coexist: a key containing a
        // space, `=`, and a non-ASCII character is presigned for a multipart part and lands at the
        // correct key. This is the exact path/query conflation class of bug we fixed.
        let integration = signing_store().await;
        let path = Path::from("test signed/a b=c/π.bin");
        let _ = integration.delete(&path).await;

        let upload_id = integration.create_multipart(&path).await.unwrap();
        let body = vec![7u8; 2048];
        let options = SignedUrlOptions::default()
            .with_query([("partNumber", "1"), ("uploadId", upload_id.as_str())]);
        let url = integration
            .signed_url_opts(Method::PUT, &path, Duration::from_secs(300), &options)
            .await
            .unwrap();
        let resp = reqwest::Client::new()
            .put(url)
            .body(body.clone())
            .send()
            .await
            .unwrap();
        assert!(
            resp.status().is_success(),
            "special-key UploadPart failed: {resp:?}"
        );
        let etag = resp
            .headers()
            .get(http::header::ETAG)
            .expect("ETag")
            .to_str()
            .unwrap()
            .to_string();
        integration
            .complete_multipart(&path, &upload_id, vec![PartId { content_id: etag }])
            .await
            .unwrap();

        // Read back through the normal (credentialed) path to confirm the object landed at the
        // intended key.
        let got = integration.get(&path).await.unwrap().bytes().await.unwrap();
        assert_eq!(got.as_ref(), body.as_slice());

        integration.delete(&path).await.unwrap();
    }

    #[tokio::test]
    async fn signed_url_expires() {
        maybe_skip_integration!();
        maybe_skip_signature_enforcement!();

        // A short-lived signed URL is rejected after it expires, confirming the TTL is part of the
        // signed policy.
        let integration = signing_store().await;
        let path = Path::from("test_signed_expiry.bin");
        let _ = integration.delete(&path).await;

        let url = integration
            .signed_url(Method::PUT, &path, Duration::from_secs(1))
            .await
            .unwrap();
        std::thread::sleep(Duration::from_secs(3));
        let resp = reqwest::Client::new()
            .put(url)
            .body(b"too late".to_vec())
            .send()
            .await
            .unwrap();
        let status = resp.status();
        // Clean up before asserting: if the PUT was wrongly accepted, the object exists and would
        // otherwise leak past the panic into the shared bucket.
        let _ = integration.delete(&path).await;
        assert_eq!(
            status.as_u16(),
            403,
            "expired signed URL was accepted: {resp:?}"
        );
    }

    #[tokio::test]
    async fn signed_url_conditional_create_blocks_overwrite() {
        maybe_skip_integration!();

        // A presigned PUT that signs `If-None-Match: *` succeeds when the object is absent and is
        // rejected (412) on replay once the object exists — the leaked-URL replay guard.
        let integration = signing_store().await;
        let path = Path::from("test_signed_conditional.bin");
        let _ = integration.delete(&path).await;
        let client = reqwest::Client::new();

        let options = SignedUrlOptions::default()
            .with_signed_header(IF_NONE_MATCH, HeaderValue::from_static("*"));
        let url = integration
            .signed_url_opts(Method::PUT, &path, Duration::from_secs(300), &options)
            .await
            .unwrap();

        let first = client
            .put(url.clone())
            .header(IF_NONE_MATCH, "*")
            .body(b"first write".to_vec())
            .send()
            .await
            .unwrap();
        assert!(
            first.status().is_success(),
            "conditional create on absent object failed: {first:?}"
        );

        let replay = client
            .put(url)
            .header(IF_NONE_MATCH, "*")
            .body(b"overwrite attempt".to_vec())
            .send()
            .await
            .unwrap();
        assert_eq!(
            replay.status().as_u16(),
            412,
            "replay overwrite was not blocked: {replay:?}"
        );

        integration.delete(&path).await.unwrap();
    }

    #[tokio::test]
    async fn copy_multipart_file_with_signature() {
        maybe_skip_integration!();

        let bucket = "test-bucket-for-copy-if-not-exists";
        for checksum in [Checksum::SHA256, Checksum::CRC64NVME] {
            let store = AmazonS3Builder::from_env()
                .with_bucket_name(bucket)
                .with_checksum_algorithm(checksum)
                .with_copy_if_not_exists(S3CopyIfNotExists::Multipart)
                .build()
                .unwrap();

            let src = Path::parse("src.bin").unwrap();
            let dst = Path::parse("dst.bin").unwrap();
            store
                .put(&src, PutPayload::from(vec![0u8; 100_000]))
                .await
                .unwrap();
            if store.head(&dst).await.is_ok() {
                store.delete(&dst).await.unwrap();
            }
            store.copy_if_not_exists(&src, &dst).await.unwrap();
            store.delete(&src).await.unwrap();
            store.delete(&dst).await.unwrap();
        }
    }

    #[tokio::test]
    async fn copy_multipart_file_with_signature_change_checksum() {
        maybe_skip_integration!();

        let bucket = "test-bucket-for-copy-if-not-exists";
        let checksum_src = Checksum::SHA256;
        let checksum_dst = Checksum::CRC64NVME;

        let src = Path::parse("change_checksum_src.bin").unwrap();
        let dst = Path::parse("change_checksum_dst.bin").unwrap();

        let store = AmazonS3Builder::from_env()
            .with_bucket_name(bucket)
            .with_checksum_algorithm(checksum_src)
            .build()
            .unwrap();

        store
            .put(&src, PutPayload::from(vec![0u8; 100_000]))
            .await
            .unwrap();
        if store.head(&dst).await.is_ok() {
            store.delete(&dst).await.unwrap();
        }

        let store = AmazonS3Builder::from_env()
            .with_bucket_name(bucket)
            .with_checksum_algorithm(checksum_dst)
            .with_copy_if_not_exists(S3CopyIfNotExists::Multipart)
            .build()
            .unwrap();

        store.copy_if_not_exists(&src, &dst).await.unwrap();
        store.delete(&src).await.unwrap();
        store.delete(&dst).await.unwrap();
    }

    #[tokio::test]
    async fn write_multipart_file_with_signature_object_lock() {
        maybe_skip_integration!();

        for checksum in [Checksum::SHA256, Checksum::CRC64NVME] {
            let bucket = "test-object-lock";
            let store = AmazonS3Builder::from_env()
                .with_bucket_name(bucket)
                .with_checksum_algorithm(checksum)
                .build()
                .unwrap();

            let str = "test.bin";
            let path = Path::parse(str).unwrap();
            let opts = PutMultipartOptions::default();
            let mut upload = store.put_multipart_opts(&path, opts).await.unwrap();

            upload
                .put_part(PutPayload::from(vec![0u8; 10_000_000]))
                .await
                .unwrap();
            upload
                .put_part(PutPayload::from(vec![0u8; 5_000_000]))
                .await
                .unwrap();

            let res = upload.complete().await.unwrap();
            assert!(res.e_tag.is_some(), "Should have valid etag");

            store.delete(&path).await.unwrap();
        }
    }

    #[tokio::test]
    async fn s3_test() {
        maybe_skip_integration!();
        // tag the extensions of every HTTP response with a marker,
        // allowing response_extensions to verify their propagation
        let config =
            AmazonS3Builder::from_env().with_http_connector(MarkerHttpConnector::default());

        let integration = config.build().unwrap();
        let config = &integration.client.config;
        let test_not_exists = config.copy_if_not_exists.is_some();
        let test_conditional_put = config.conditional_put != S3ConditionalPut::Disabled;

        put_get_delete_list(&integration).await;
        list_with_offset_exclusivity(&integration).await;
        get_opts(&integration).await;
        list_uses_directories_correctly(&integration).await;
        list_with_delimiter(&integration).await;
        rename_and_copy(&integration).await;
        stream_get(&integration).await;
        multipart(&integration, &integration).await;
        multipart_with_opts(&integration, &integration).await;
        multipart_put_part_out_of_order(&integration, &integration).await;
        multipart_race_condition(&integration, true).await;
        multipart_out_of_order(&integration).await;
        signing(&integration).await;
        s3_encryption(&integration).await;
        put_get_attributes(&integration).await;
        list_paginated(&integration, &integration).await;
        response_extensions(&integration, true).await;

        // Object tagging is not supported by S3 Express One Zone
        if config.session_provider.is_none() {
            tagging(
                Arc::new(AmazonS3 {
                    client: Arc::clone(&integration.client),
                }),
                !config.disable_tagging,
                |p| {
                    let client = Arc::clone(&integration.client);
                    async move { client.get_object_tagging(&p).await }
                },
            )
            .await;
        }

        if test_not_exists {
            copy_if_not_exists(&integration).await;
        }
        if test_conditional_put {
            put_opts(&integration, true).await;
        }

        // run integration test with unsigned payload enabled
        let builder = AmazonS3Builder::from_env().with_unsigned_payload(true);
        let integration = builder.build().unwrap();
        put_get_delete_list(&integration).await;

        // run integration test with checksum set to sha256
        let builder = AmazonS3Builder::from_env().with_checksum_algorithm(Checksum::SHA256);
        let integration = builder.build().unwrap();
        put_get_delete_list(&integration).await;

        // run integration test with checksum set to crc64nvme
        let builder = AmazonS3Builder::from_env().with_checksum_algorithm(Checksum::CRC64NVME);
        let integration = builder.build().unwrap();
        put_get_delete_list(&integration).await;
    }

    #[tokio::test]
    async fn s3_test_get_nonexistent_location() {
        maybe_skip_integration!();
        let integration = AmazonS3Builder::from_env().build().unwrap();

        let location = Path::from_iter([NON_EXISTENT_NAME]);

        let err = get_nonexistent_object(&integration, Some(location))
            .await
            .unwrap_err();
        assert!(matches!(err, crate::Error::NotFound { .. }), "{}", err);
    }

    #[tokio::test]
    async fn s3_test_get_nonexistent_bucket() {
        maybe_skip_integration!();
        let config = AmazonS3Builder::from_env().with_bucket_name(NON_EXISTENT_NAME);
        let integration = config.build().unwrap();

        let location = Path::from_iter([NON_EXISTENT_NAME]);

        let err = integration.get(&location).await.unwrap_err();
        assert!(matches!(err, crate::Error::NotFound { .. }), "{}", err);
    }

    #[tokio::test]
    async fn s3_test_put_nonexistent_bucket() {
        maybe_skip_integration!();
        let config = AmazonS3Builder::from_env().with_bucket_name(NON_EXISTENT_NAME);
        let integration = config.build().unwrap();

        let location = Path::from_iter([NON_EXISTENT_NAME]);
        let data = PutPayload::from("arbitrary data");

        let err = integration.put(&location, data).await.unwrap_err();
        assert!(matches!(err, crate::Error::NotFound { .. }), "{}", err);
    }

    #[tokio::test]
    async fn s3_test_delete_nonexistent_location() {
        maybe_skip_integration!();
        let integration = AmazonS3Builder::from_env().build().unwrap();

        let location = Path::from_iter([NON_EXISTENT_NAME]);

        integration.delete(&location).await.unwrap();
    }

    #[tokio::test]
    async fn s3_test_delete_nonexistent_bucket() {
        maybe_skip_integration!();
        let config = AmazonS3Builder::from_env().with_bucket_name(NON_EXISTENT_NAME);
        let integration = config.build().unwrap();

        let location = Path::from_iter([NON_EXISTENT_NAME]);

        let err = integration.delete(&location).await.unwrap_err();
        assert!(matches!(err, crate::Error::NotFound { .. }), "{}", err);
    }

    #[tokio::test]
    #[ignore = "Tests shouldn't call use remote services by default"]
    async fn test_disable_creds() {
        // https://registry.opendata.aws/daylight-osm/
        let v1 = AmazonS3Builder::new()
            .with_bucket_name("daylight-map-distribution")
            .with_region("us-west-1")
            .with_access_key_id("local")
            .with_secret_access_key("development")
            .build()
            .unwrap();

        let prefix = Path::from("release");

        v1.list_with_delimiter(Some(&prefix)).await.unwrap_err();

        let v2 = AmazonS3Builder::new()
            .with_bucket_name("daylight-map-distribution")
            .with_region("us-west-1")
            .with_skip_signature(true)
            .build()
            .unwrap();

        v2.list_with_delimiter(Some(&prefix)).await.unwrap();
    }

    async fn s3_encryption(store: &AmazonS3) {
        maybe_skip_integration!();

        let data = PutPayload::from(vec![3u8; 1024]);

        let encryption_headers: HeaderMap = store.client.config.encryption_headers.clone().into();
        let expected_encryption =
            if let Some(encryption_type) = encryption_headers.get("x-amz-server-side-encryption") {
                encryption_type
            } else {
                eprintln!("Skipping S3 encryption test - encryption not configured");
                return;
            };

        let locations = [
            Path::from("test-encryption-1"),
            Path::from("test-encryption-2"),
            Path::from("test-encryption-3"),
        ];

        store.put(&locations[0], data.clone()).await.unwrap();
        store.copy(&locations[0], &locations[1]).await.unwrap();

        let mut upload = store.put_multipart(&locations[2]).await.unwrap();
        upload.put_part(data.clone()).await.unwrap();
        upload.complete().await.unwrap();

        for location in &locations {
            let mut context = RetryContext::new(&store.client.config.retry_config);

            let res = store
                .client
                .get_request(&mut context, location, GetOptions::default())
                .await
                .unwrap();

            let headers = res.headers();
            assert_eq!(
                headers
                    .get("x-amz-server-side-encryption")
                    .expect("object is not encrypted"),
                expected_encryption
            );

            store.delete(location).await.unwrap();
        }
    }

    /// See CONTRIBUTING.md for the MinIO setup for this test.
    #[tokio::test]
    async fn test_s3_ssec_encryption_with_minio() {
        if std::env::var("TEST_S3_SSEC_ENCRYPTION").is_err() {
            eprintln!("Skipping S3 SSE-C encryption test");
            return;
        }
        eprintln!("Running S3 SSE-C encryption test");

        let customer_key = "1234567890abcdef1234567890abcdef";
        let expected_md5 = "JMwgiexXqwuPqIPjYFmIZQ==";

        let store = AmazonS3Builder::from_env()
            .with_ssec_encryption(BASE64_STANDARD.encode(customer_key))
            .with_client_options(ClientOptions::default().with_allow_invalid_certificates(true))
            .build()
            .unwrap();

        let data = PutPayload::from(vec![3u8; 1024]);

        let locations = [
            Path::from("test-encryption-1"),
            Path::from("test-encryption-2"),
            Path::from("test-encryption-3"),
        ];

        // Test put with sse-c.
        store.put(&locations[0], data.clone()).await.unwrap();

        // Test copy with sse-c.
        store.copy(&locations[0], &locations[1]).await.unwrap();

        // Test multipart upload with sse-c.
        let mut upload = store.put_multipart(&locations[2]).await.unwrap();
        upload.put_part(data.clone()).await.unwrap();
        upload.complete().await.unwrap();

        // Test get with sse-c.
        for location in &locations {
            let mut context = RetryContext::new(&store.client.config.retry_config);

            let res = store
                .client
                .get_request(&mut context, location, GetOptions::default())
                .await
                .unwrap();

            let headers = res.headers();
            assert_eq!(
                headers
                    .get("x-amz-server-side-encryption-customer-algorithm")
                    .expect("object is not encrypted with SSE-C"),
                "AES256"
            );

            assert_eq!(
                headers
                    .get("x-amz-server-side-encryption-customer-key-MD5")
                    .expect("object is not encrypted with SSE-C"),
                expected_md5
            );

            store.delete(location).await.unwrap();
        }
    }

    /// Integration test that ensures I/O is done on an alternate threadpool
    /// when using the `SpawnedReqwestConnector`.
    #[cfg(feature = "reqwest")]
    #[test]
    fn s3_alternate_threadpool_spawned_request_connector() {
        maybe_skip_integration!();
        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();

        // Runtime with I/O enabled
        let io_runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all() // <-- turns on IO
            .build()
            .unwrap();

        // Runtime without I/O enabled
        let non_io_runtime = tokio::runtime::Builder::new_current_thread()
            // note: no call to enable_all
            .build()
            .unwrap();

        // run the io runtime in a different thread
        let io_handle = io_runtime.handle().clone();
        let thread_handle = std::thread::spawn(move || {
            io_runtime.block_on(async move {
                shutdown_rx.await.unwrap();
            });
        });

        let store = AmazonS3Builder::from_env()
            // use different bucket to avoid collisions with other tests
            .with_bucket_name("test-bucket-for-spawn")
            .with_http_connector(SpawnedReqwestConnector::new(io_handle))
            .build()
            .unwrap();

        // run a request on the non io runtime -- will fail if the connector
        // does not spawn the request to the io runtime
        non_io_runtime
            .block_on(async move {
                let path = Path::from("alternate_threadpool/test.txt");
                store.delete(&path).await.ok(); // remove the file if it exists from prior runs
                store.put(&path, "foo".into()).await?;
                let res = store.get(&path).await?.bytes().await?;
                assert_eq!(res.as_ref(), b"foo");
                store.delete(&path).await?; // cleanup
                Ok(()) as Result<()>
            })
            .expect("failed to run request on non io runtime");

        // shutdown the io runtime and thread
        shutdown_tx.send(()).ok();
        thread_handle.join().expect("runtime thread panicked");
    }
}
