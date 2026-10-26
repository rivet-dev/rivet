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

//! Retry policies for HTTP-backed multipart uploads
//!
//! Each multipart part first uses the backend's existing bounded request retry
//! loop. If those retries are exhausted, a [`RetryPolicy`] can restart the
//! entire part operation. Restarting at this boundary rebuilds the request,
//! including fetching credentials and generating a new signature.
//!
//! Configure a policy with [`PutMultipartOptions::with_retry_policy`]. The
//! policy applies independently to each part and does not change the retry
//! behavior of multipart initiation, completion, or abort requests.

use crate::client::retry::{RequestError, RetryError};
use crate::client::{HttpError, HttpErrorKind};
use crate::{Error, PutMultipartOptions};
use async_trait::async_trait;
use http::StatusCode;
use std::error::Error as StdError;
use std::fmt::Debug;
use std::sync::Arc;
use std::time::Duration;
#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
use std::time::Instant;
#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
use web_time::Instant;

/// A normalized HTTP failure passed to a [`RetryPolicy`]
///
/// Backends wrap request failures in provider-specific [`Error`] variants.
/// Before invoking a policy, `object_store` reduces a supported failure to an
/// HTTP response status or transport error kind. Failures that cannot be
/// represented by this enum are returned without invoking the policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum RetryFailure {
    /// The HTTP status of a response that the backend classified as failed
    ///
    /// This is usually a non-success status. Some services, including S3, can
    /// report an error in the body of a successful response. In that case this
    /// contains the actual successful status, such as `200 OK`.
    Status(StatusCode),

    /// The request failed before producing a usable HTTP response
    ///
    /// The kind distinguishes failures such as connection errors, timeouts,
    /// interrupted requests, and response decoding errors.
    Transport(HttpErrorKind),
}

/// The failure and retry state for one operation attempt
///
/// For multipart uploads, an attempt is one complete call to upload a part,
/// including its bounded request retries. Each part has an independent attempt
/// count and elapsed time.
//
// Note: deliberately does not implement `Copy` so that non-`Copy` details,
// such as the underlying error, can be added in the future without a
// breaking change.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct RetryContext {
    /// The normalized HTTP failure
    pub failure: RetryFailure,

    /// The failed operation attempt number, starting at one
    pub attempt: usize,

    /// Time since the first operation attempt started
    ///
    /// This includes time spent in requests and in earlier policy calls.
    pub elapsed: Duration,
}

/// Controls whether and when a failed operation should be attempted again
///
/// The policy is shared by concurrently uploaded parts. Implementations must
/// manage their own retry limit and backoff. `object_store` does not add a delay
/// or impose an outer retry limit after this method returns `true`.
#[async_trait]
pub trait RetryPolicy: Debug + Send + Sync + 'static {
    /// Decide whether to attempt the failed operation again
    ///
    /// Because this method is asynchronous, implementations can wait using
    /// their own backoff strategy, clock, or sleeper before returning `true`.
    /// Returning `false` returns the current operation error to the caller.
    async fn retry(&self, context: RetryContext) -> bool;
}

#[derive(Clone)]
struct MultipartRetryPolicy(Arc<dyn RetryPolicy>);

impl PutMultipartOptions {
    /// Retry failed multipart part uploads according to `policy`
    ///
    /// The policy runs after the existing bounded request retries are
    /// exhausted. A retry repeats the same logical part from the provider
    /// operation boundary, allowing credentials and request signatures to be
    /// refreshed. Initiating, completing, and aborting the multipart upload
    /// retain their existing retry behavior.
    #[must_use]
    pub fn with_retry_policy(mut self, policy: Arc<dyn RetryPolicy>) -> Self {
        self.extensions.insert(MultipartRetryPolicy(policy));
        self
    }

    pub(crate) fn retry_policy(&self) -> Option<Arc<dyn RetryPolicy>> {
        self.extensions
            .get::<MultipartRetryPolicy>()
            .map(|policy| Arc::clone(&policy.0))
    }
}

pub(crate) struct MultipartRetry {
    policy: Option<Arc<dyn RetryPolicy>>,
    attempt: usize,
    start: Instant,
}

impl MultipartRetry {
    pub(crate) fn new(policy: Option<Arc<dyn RetryPolicy>>) -> Self {
        Self {
            policy,
            attempt: 0,
            start: Instant::now(),
        }
    }

    pub(crate) async fn should_retry(&mut self, error: &Error) -> bool {
        let Some(policy) = self.policy.as_ref() else {
            return false;
        };
        let Some(failure) = classify_http_failure(error) else {
            return false;
        };

        self.attempt += 1;
        policy
            .retry(RetryContext {
                failure,
                attempt: self.attempt,
                elapsed: self.start.elapsed(),
            })
            .await
    }
}

/// Find and normalize an HTTP failure in an [`Error`] source chain.
///
/// Provider errors may wrap a terminal [`RetryError`] or [`HttpError`] several
/// levels deep. Returns `None` when the chain contains no failure representable
/// by [`RetryFailure`].
fn classify_http_failure(error: &Error) -> Option<RetryFailure> {
    let mut current: &(dyn StdError + 'static) = error;
    loop {
        if let Some(error) = current.downcast_ref::<RetryError>() {
            return classify_request_error(error.inner());
        }
        if let Some(error) = current.downcast_ref::<HttpError>() {
            return Some(RetryFailure::Transport(error.kind()));
        }
        current = current.source()?;
    }
}

/// Normalize a terminal request error for a [`RetryPolicy`].
///
/// Error responses retain their actual status, including successful statuses
/// whose bodies contain a provider error. A bare redirect has no status or
/// transport kind to expose and is therefore not policy-controlled.
fn classify_request_error(error: &RequestError) -> Option<RetryFailure> {
    match error {
        RequestError::Status { status, .. } | RequestError::Response { status, .. } => {
            Some(RetryFailure::Status(*status))
        }
        RequestError::Http(error) => Some(RetryFailure::Transport(error.kind())),
        RequestError::BareRedirect => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use parking_lot::Mutex;

    #[derive(Debug, Default)]
    struct RecordingPolicy(Mutex<Vec<RetryContext>>);

    #[async_trait]
    impl RetryPolicy for RecordingPolicy {
        async fn retry(&self, context: RetryContext) -> bool {
            self.0.lock().push(context);
            true
        }
    }

    #[derive(Debug, thiserror::Error)]
    #[error("test error")]
    struct TestError;

    #[tokio::test]
    async fn retries_http_errors() {
        let policy = Arc::new(RecordingPolicy::default());
        let mut retry = MultipartRetry::new(Some(Arc::clone(&policy) as Arc<dyn RetryPolicy>));
        let error = Error::Generic {
            store: "test",
            source: Box::new(HttpError::new(HttpErrorKind::Timeout, TestError)),
        };

        assert!(retry.should_retry(&error).await);

        let contexts = policy.0.lock();
        assert_eq!(contexts.len(), 1);
        assert_eq!(contexts[0].attempt, 1);
        assert_eq!(
            contexts[0].failure,
            RetryFailure::Transport(HttpErrorKind::Timeout)
        );
    }

    #[tokio::test]
    async fn does_not_retry_non_http_errors() {
        let policy = Arc::new(RecordingPolicy::default());
        let mut retry = MultipartRetry::new(Some(Arc::clone(&policy) as Arc<dyn RetryPolicy>));
        let error = Error::Generic {
            store: "test",
            source: Box::new(TestError),
        };

        assert!(!retry.should_retry(&error).await);
        assert!(policy.0.lock().is_empty());
    }

    #[test]
    fn classifies_error_responses_with_success_status() {
        let error = RequestError::Response {
            status: StatusCode::OK,
            body: "InternalError".into(),
        };

        assert_eq!(
            classify_request_error(&error),
            Some(RetryFailure::Status(StatusCode::OK))
        );
    }
}
