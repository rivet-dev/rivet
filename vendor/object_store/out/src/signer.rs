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

//! Abstraction of signed URL generation for those object store implementations that support it

use crate::{Result, path::Path};
use async_trait::async_trait;
use std::{fmt, time::Duration};

// publicly re-export types from http/url used in API so downstream consumers do
// not have to explicitly add those crates as dependencies
pub use http::Method;
pub use http::{HeaderMap, HeaderName, HeaderValue};
pub use url::Url;

/// Additional parameters to fold into a presigned URL's signature, used with
/// [`Signer::signed_url_opts`].
///
/// All values are fixed at signing time: the recipient of the presigned URL must send exactly
/// these query parameters and headers, with these values, for the request to be accepted.
///
/// Construct with [`SignedUrlOptions::default`] (or the builder methods) and set only the fields
/// you need, so that future additions remain backwards compatible:
///
/// ```
/// # use object_store::signer::SignedUrlOptions;
/// # use http::header::{CONTENT_TYPE, HeaderValue};
/// let options = SignedUrlOptions::default()
///     .with_query([("partNumber", "1"), ("uploadId", "abc123")])
///     .with_signed_header(CONTENT_TYPE, HeaderValue::from_static("text/plain"));
/// ```
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct SignedUrlOptions {
    /// Query parameters to sign and append to the URL.
    ///
    /// Used for requests that carry signed query parameters — for example a multipart
    /// `UploadPart` (`partNumber`, `uploadId`) or pinning a `versionId`.
    pub extra_query: Vec<(String, String)>,
    /// Request headers to bind to the signature.
    ///
    /// Used to require the recipient to send specific headers, such as a checksum
    /// (`x-amz-checksum-sha256`), `content-type`, or server-side-encryption headers.
    pub signed_headers: HeaderMap,
}

impl SignedUrlOptions {
    /// Create an empty set of options, equivalent to [`SignedUrlOptions::default`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Append query parameters to sign.
    pub fn with_query<I, K, V>(mut self, query: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        self.extra_query
            .extend(query.into_iter().map(|(k, v)| (k.into(), v.into())));
        self
    }

    /// Bind a request header to the signature.
    pub fn with_signed_header(mut self, name: HeaderName, value: HeaderValue) -> Self {
        self.signed_headers.append(name, value);
        self
    }

    /// Returns `true` if no extra query parameters or signed headers have been set.
    pub fn is_empty(&self) -> bool {
        self.extra_query.is_empty() && self.signed_headers.is_empty()
    }
}

/// Universal API to generate presigned URLs from multiple object store services.
#[async_trait]
pub trait Signer: Send + Sync + fmt::Debug + 'static {
    /// Given the intended [`Method`] and [`Path`] to use and the desired length of time for which
    /// the URL should be valid, return a signed [`Url`] created with the object store
    /// implementation's credentials such that the URL can be handed to something that doesn't have
    /// access to the object store's credentials, to allow limited access to the object store.
    async fn signed_url(&self, method: Method, path: &Path, expires_in: Duration) -> Result<Url>;

    /// Like [`Signer::signed_url`], but additionally folds the query parameters and headers in
    /// `options` into the signature. See [`SignedUrlOptions`].
    ///
    /// This presigns a *single* request (for example one multipart `UploadPart`). Orchestrating a
    /// multipart upload — creating and completing it — is not in scope and is typically done by
    /// the credential-holding server via [`MultipartStore`](crate::multipart::MultipartStore).
    ///
    /// The default implementation delegates to [`Signer::signed_url`] when `options` is empty, and
    /// otherwise returns [`crate::Error::NotSupported`]: implementations that do not support
    /// signing additional query parameters or headers must not silently drop them, as that would
    /// produce a URL that does not enforce the requested constraints.
    ///
    /// There is intentionally no `signed_urls_opts` batch counterpart: signed parameters such as
    /// `partNumber` are per-request, so a batch sharing one set of options across many paths has
    /// no clear meaning.
    async fn signed_url_opts(
        &self,
        method: Method,
        path: &Path,
        expires_in: Duration,
        options: &SignedUrlOptions,
    ) -> Result<Url> {
        if options.is_empty() {
            return self.signed_url(method, path, expires_in).await;
        }
        Err(crate::Error::NotSupported {
            source: "this object store does not support signing URLs with additional \
                     query parameters or headers"
                .into(),
        })
    }

    /// Generate signed urls for multiple paths.
    ///
    /// See [`Signer::signed_url`] for more details.
    async fn signed_urls(
        &self,
        method: Method,
        paths: &[Path],
        expires_in: Duration,
    ) -> Result<Vec<Url>> {
        let mut urls = Vec::with_capacity(paths.len());
        for path in paths {
            urls.push(self.signed_url(method.clone(), path, expires_in).await?);
        }
        Ok(urls)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Error;
    use http::header::CONTENT_TYPE;

    /// A [`Signer`] that only implements the required `signed_url`, relying on the default
    /// `signed_url_opts` — mirroring providers, such as Azure, that only implement `signed_url`.
    #[derive(Debug)]
    struct MinimalSigner;

    #[async_trait]
    impl Signer for MinimalSigner {
        async fn signed_url(
            &self,
            _method: Method,
            path: &Path,
            _expires_in: Duration,
        ) -> Result<Url> {
            Ok(Url::parse(&format!("https://example.com/{path}")).unwrap())
        }
    }

    #[tokio::test]
    async fn default_signed_url_opts_delegates_when_empty() {
        let signer = MinimalSigner;
        let url = signer
            .signed_url_opts(
                Method::GET,
                &Path::from("file.txt"),
                Duration::from_secs(60),
                &SignedUrlOptions::default(),
            )
            .await
            .unwrap();
        assert_eq!(url.as_str(), "https://example.com/file.txt");
    }

    #[tokio::test]
    async fn default_signed_url_opts_rejects_extras() {
        let signer = MinimalSigner;

        let query_err = signer
            .signed_url_opts(
                Method::PUT,
                &Path::from("file.txt"),
                Duration::from_secs(60),
                &SignedUrlOptions::default().with_query([("partNumber", "1")]),
            )
            .await
            .unwrap_err();
        assert!(matches!(query_err, Error::NotSupported { .. }));

        let header_err = signer
            .signed_url_opts(
                Method::PUT,
                &Path::from("file.txt"),
                Duration::from_secs(60),
                &SignedUrlOptions::default()
                    .with_signed_header(CONTENT_TYPE, HeaderValue::from_static("text/plain")),
            )
            .await
            .unwrap_err();
        assert!(matches!(header_err, Error::NotSupported { .. }));
    }
}
