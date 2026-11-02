use std::borrow::Cow;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use backon::{ExponentialBuilder, Retryable, Sleeper};
use futures::stream::BoxStream;
use futures::{stream, StreamExt, TryStreamExt};
use log::{debug, info};
use object_store::path::Path;
use object_store::{
    Attribute, CopyOptions, Extensions, GetOptions, GetRange, GetResult, GetResultPayload,
    ListResult, MultipartUpload, ObjectMeta, ObjectStore, ObjectStoreExt, PutMultipartOptions,
    PutOptions, PutPayload, PutResult, RenameOptions,
};

use crate::utils::IdGenerator;
use slatedb_common::clock::SystemClock;
use slatedb_common::DbRand;

const MIN_RETRY_DELAY: Duration = Duration::from_millis(100);
const MAX_RETRY_DELAY: Duration = Duration::from_secs(1);

/// Metadata key used to store the ULID for put operations.
/// This is used to verify if a failed put actually succeeded.
/// There's not separator between "slatedb", "put", and "id" to avoid issues
/// with object stores that restrict metadata keys.
const PUT_ID_ATTRIBUTE: &str = "slatedbputid";

type SystemClockSleep = Pin<Box<dyn Future<Output = ()> + Send>>;

/// Backon's sleeper interface backed by SlateDB's configured system clock.
#[derive(Debug, Clone)]
struct SystemClockSleeper {
    clock: Arc<dyn SystemClock>,
}

impl Sleeper for SystemClockSleeper {
    type Sleep = SystemClockSleep;

    fn sleep(&self, dur: Duration) -> Self::Sleep {
        let clock = Arc::clone(&self.clock);
        Box::pin(async move {
            clock.sleep(dur).await;
        })
    }
}

/// A thin wrapper around an `ObjectStore` that retries transient errors with
/// exponential backoff using the configured [`SystemClock`] for sleeps.
///
/// Retries are unbounded by default; a bound can be configured via
/// `max_retries`, in which case an operation that keeps failing eventually
/// returns its underlying error instead of retrying forever. This applies to
/// both foreground and background object-store operations, since both go
/// through this wrapper.
#[derive(Debug, Clone)]
pub(crate) struct RetryingObjectStore {
    inner: Arc<dyn ObjectStore>,
    rand: Arc<DbRand>,
    clock: Arc<dyn SystemClock>,
    /// Maximum wrapper-level retries per operation. `None` = unbounded.
    max_retries: Option<u32>,
}

impl RetryingObjectStore {
    pub(crate) fn new(
        inner: Arc<dyn ObjectStore>,
        rand: Arc<DbRand>,
        clock: Arc<dyn SystemClock>,
        max_retries: Option<u32>,
    ) -> Self {
        Self {
            inner,
            rand,
            clock,
            max_retries,
        }
    }

    #[inline]
    fn retry_builder(&self) -> ExponentialBuilder {
        let builder = ExponentialBuilder::default()
            .with_min_delay(MIN_RETRY_DELAY)
            .with_max_delay(MAX_RETRY_DELAY);
        match self.max_retries {
            Some(max_retries) => builder.with_max_times(max_retries as usize),
            None => builder.without_max_times(),
        }
    }

    #[inline]
    fn sleeper(&self) -> SystemClockSleeper {
        SystemClockSleeper {
            clock: Arc::clone(&self.clock),
        }
    }

    #[inline]
    fn notify(err: &object_store::Error, duration: Duration) {
        info!(
            "retrying object store operation [error={:?}, duration={:?}]",
            err, duration
        );
    }

    #[inline]
    pub(crate) fn should_retry(err: &object_store::Error) -> bool {
        let retry = !matches!(
            err,
            object_store::Error::AlreadyExists { .. }
                | object_store::Error::Precondition { .. }
                | object_store::Error::NotModified { .. }
                | object_store::Error::NotFound { .. }
                | object_store::Error::NotImplemented { .. }
                | object_store::Error::NotSupported { .. }
        );
        if !retry {
            debug!("not retrying object store operation [error={:?}]", err);
        }
        retry
    }

    /// Checks if a failed put actually succeeded by verifying the ULID in remote metadata.
    ///
    /// When a put operation times out after the file was successfully written,
    /// a retry would encounter an AlreadyExists or Precondition error. This method
    /// checks if the object in the store has our ULID, indicating our write succeeded.
    ///
    /// Returns `Some(ObjectMeta)` if verification succeeds, `None` otherwise.
    async fn verify_put_succeeded(&self, location: &Path, expected_id: &str) -> Option<ObjectMeta> {
        let get_opts = GetOptions {
            head: true,
            ..Default::default()
        };
        let result = (|| async { self.inner.get_opts(location, get_opts.clone()).await })
            .retry(self.retry_builder())
            .sleep(self.sleeper())
            .notify(Self::notify)
            .when(Self::should_retry)
            .await;

        match result {
            Ok(get_result) => {
                let key = Attribute::Metadata(Cow::Borrowed(PUT_ID_ATTRIBUTE));
                if get_result
                    .attributes
                    .get(&key)
                    .is_some_and(|v| v.as_ref() == expected_id)
                {
                    Some(get_result.meta)
                } else {
                    None
                }
            }
            Err(_) => None,
        }
    }

    /// Creates a new Attributes with our ULID attribute merged with existing attributes.
    fn with_put_id(attrs: object_store::Attributes, put_id: &str) -> object_store::Attributes {
        let mut new_attrs = object_store::Attributes::new();
        for (key, value) in attrs.iter() {
            new_attrs.insert(key.clone(), value.clone());
        }

        new_attrs.insert(
            Attribute::Metadata(Cow::Owned(PUT_ID_ATTRIBUTE.to_string())),
            object_store::AttributeValue::from(put_id.to_string()),
        );
        new_attrs
    }

    /// Compute the expected byte length of a range read, truncating at the
    /// actual file size. This handles the case where a `GetRange::Bounded`
    /// end exceeds the file length.
    fn expected_range_len(range: &GetRange, file_size: u64) -> u64 {
        match range {
            GetRange::Bounded(r) => {
                let end = r.end.min(file_size);
                end.saturating_sub(r.start)
            }
            GetRange::Offset(offset) => file_size.saturating_sub(*offset),
            GetRange::Suffix(suffix) => (*suffix).min(file_size),
        }
    }
}

impl std::fmt::Display for RetryingObjectStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "RetryingObjectStore({})", self.inner)
    }
}

#[cfg(any(feature = "aws", feature = "azure", feature = "gcp"))]
#[derive(Debug)]
struct RetryingObjectStorePolicy {
    clock: Arc<dyn SystemClock>,
    max_retries: Option<u32>,
}

#[cfg(any(feature = "aws", feature = "azure", feature = "gcp"))]
#[async_trait]
impl object_store::retry::RetryPolicy for RetryingObjectStorePolicy {
    async fn retry(&self, context: object_store::retry::RetryContext) -> bool {
        self.retry_failure(context.failure, context.attempt).await
    }
}

#[cfg(any(feature = "aws", feature = "azure", feature = "gcp"))]
impl RetryingObjectStorePolicy {
    async fn retry_failure(
        &self,
        failure: object_store::retry::RetryFailure,
        attempt: usize,
    ) -> bool {
        use object_store::retry::RetryFailure;

        // These statuses map to the errors excluded by `should_retry`.
        // TODO: Use `RetryingObjectStore::should_retry` directly when
        // `object_store::retry::RetryContext` includes the actual `Error`.
        if matches!(failure, RetryFailure::Status(status)
            if matches!(status.as_u16(), 304 | 404 | 409 | 412))
        {
            debug!("not retrying multipart part upload [failure={:?}]", failure);
            return false;
        }
        if self
            .max_retries
            .is_some_and(|max_retries| attempt > max_retries as usize)
        {
            return false;
        }

        // Double the delay for each retry, up to one second.
        // Stop at one second so a large attempt count does not require a long loop.
        let mut delay = MIN_RETRY_DELAY;
        for _ in 1..attempt {
            if delay >= MAX_RETRY_DELAY {
                break;
            }
            delay = (delay * 2).min(MAX_RETRY_DELAY);
        }
        info!(
            "retrying multipart part upload [failure={:?}, attempt={}, duration={:?}]",
            failure, attempt, delay
        );
        self.clock.sleep(delay).await;
        true
    }
}

/// Wrapper around MultipartUpload that adds ULID verification on complete() failure.
struct RetryingMultipartUpload {
    inner: Box<dyn MultipartUpload>,
    retrying_store: RetryingObjectStore,
    location: Path,
    put_id: String,
}

impl std::fmt::Debug for RetryingMultipartUpload {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RetryingMultipartUpload")
            .field("location", &self.location)
            .field("put_id", &self.put_id)
            .finish()
    }
}

#[async_trait]
impl MultipartUpload for RetryingMultipartUpload {
    fn put_part(&mut self, data: PutPayload) -> object_store::UploadPart {
        self.inner.put_part(data)
    }

    async fn complete(&mut self) -> object_store::Result<PutResult> {
        let result = self.inner.complete().await;

        match &result {
            Err(object_store::Error::AlreadyExists { .. })
            | Err(object_store::Error::Precondition { .. }) => {
                if let Some(meta) = self
                    .retrying_store
                    .verify_put_succeeded(&self.location, &self.put_id)
                    .await
                {
                    return Ok(PutResult {
                        e_tag: meta.e_tag,
                        version: meta.version,
                        extensions: Extensions::new(),
                    });
                }
                result
            }
            _ => result,
        }
    }

    async fn abort(&mut self) -> object_store::Result<()> {
        self.inner.abort().await
    }
}

#[async_trait]
impl ObjectStore for RetryingObjectStore {
    async fn get_opts(
        &self,
        location: &Path,
        options: GetOptions,
    ) -> object_store::Result<GetResult> {
        // For range reads, drain the body inside the retry closure so a
        // transient mid-stream error retries the entire range. Pre-0.13
        // SlateDB explicitly overrode `ObjectStore::get_range`, which
        // returned `Bytes` and therefore retried the whole read; in 0.13
        // `get_range` lives on `ObjectStoreExt` and reads the body after
        // `get_opts` returns, so without this buffering the body bytes
        // would fall outside the retry loop.
        (|| async {
            // Options and location must be owned per-attempt.
            let options = options.clone();
            let options_range = options.range.clone();
            let result = self.inner.get_opts(location, options).await?;
            let meta = result.meta.clone();
            let file_size = meta.size;

            if options_range.is_none() {
                // No range requested — don't buffer the body. The buffer size
                // can't be validated without buffering.
                return Ok(result);
            }

            // Range read: buffer the body and validate against the expected
            // size (requested range truncated at file size).
            let expected_len =
                Self::expected_range_len(&options_range.expect("range is set"), file_size);
            let range = result.range.clone();
            let attributes = result.attributes.clone();
            let extensions = result.extensions.clone();
            let bytes = result.bytes().await?;
            let bytes_len = bytes.len() as u64;

            if bytes_len != expected_len {
                return Err(object_store::Error::Generic {
                    store: "retrying_object_store",
                    source: format!(
                        "Size check failed: {bytes_len} bytes read, but expected \
                         {expected_len} bytes (requested range truncated at file \
                         size {file_size})"
                    )
                    .into(),
                });
            }

            Ok(GetResult {
                payload: GetResultPayload::Stream(stream::once(async move { Ok(bytes) }).boxed()),
                meta,
                range,
                attributes,
                extensions,
            })
        })
        .retry(self.retry_builder())
        .sleep(self.sleeper())
        .notify(Self::notify)
        .when(Self::should_retry)
        .await
    }

    async fn put_opts(
        &self,
        location: &Path,
        payload: PutPayload,
        opts: PutOptions,
    ) -> object_store::Result<PutResult> {
        // Only add ULID metadata for conditional puts (Create/Update) where
        // we need to verify if a timeout-after-write actually succeeded.
        // For Overwrite mode, retries are safe without verification.
        let is_conditional = !matches!(opts.mode, object_store::PutMode::Overwrite);

        let put_id = if is_conditional {
            Some(self.rand.rng().gen_ulid(self.clock.as_ref()).to_string())
        } else {
            None
        };

        let opts_with_id = if let Some(ref id) = put_id {
            PutOptions {
                attributes: Self::with_put_id(opts.attributes.clone(), id),
                ..opts.clone()
            }
        } else {
            opts.clone()
        };

        let result = (|| async {
            self.inner
                .put_opts(location, payload.clone(), opts_with_id.clone())
                .await
        })
        .retry(self.retry_builder())
        .sleep(self.sleeper())
        .notify(Self::notify)
        .when(Self::should_retry)
        .await;

        // If attributes aren't supported, fall back to put without ULID
        if matches!(
            &result,
            Err(object_store::Error::NotSupported { .. }
                | object_store::Error::NotImplemented { .. })
        ) && put_id.is_some()
        {
            return (|| async {
                self.inner
                    .put_opts(location, payload.clone(), opts.clone())
                    .await
            })
            .retry(self.retry_builder())
            .sleep(self.sleeper())
            .notify(Self::notify)
            .when(Self::should_retry)
            .await;
        }

        match (&result, &put_id) {
            (Err(object_store::Error::AlreadyExists { .. }), Some(id))
            | (Err(object_store::Error::Precondition { .. }), Some(id)) => {
                if let Some(meta) = self.verify_put_succeeded(location, id).await {
                    Ok(PutResult {
                        e_tag: meta.e_tag,
                        version: meta.version,
                        extensions: Extensions::new(),
                    })
                } else {
                    result
                }
            }
            _ => result,
        }
    }

    async fn put_multipart_opts(
        &self,
        location: &Path,
        opts: PutMultipartOptions,
    ) -> object_store::Result<Box<dyn MultipartUpload>> {
        #[cfg(any(feature = "aws", feature = "azure", feature = "gcp"))]
        let opts = opts.with_retry_policy(Arc::new(RetryingObjectStorePolicy {
            clock: Arc::clone(&self.clock),
            max_retries: self.max_retries,
        }));

        let put_id = self.rand.rng().gen_ulid(self.clock.as_ref()).to_string();
        let opts_with_id = PutMultipartOptions {
            attributes: Self::with_put_id(opts.attributes.clone(), &put_id),
            ..opts.clone()
        };

        let result = (|| async {
            self.inner
                .put_multipart_opts(location, opts_with_id.clone())
                .await
        })
        .retry(self.retry_builder())
        .sleep(self.sleeper())
        .notify(Self::notify)
        .when(Self::should_retry)
        .await;

        // If attributes aren't supported, fall back without ULID
        let inner = match result {
            Ok(inner) => inner,
            Err(
                object_store::Error::NotSupported { .. }
                | object_store::Error::NotImplemented { .. },
            ) => {
                (|| async { self.inner.put_multipart_opts(location, opts.clone()).await })
                    .retry(self.retry_builder())
                    .sleep(self.sleeper())
                    .notify(Self::notify)
                    .when(Self::should_retry)
                    .await?
            }
            Err(e) => return Err(e),
        };

        Ok(Box::new(RetryingMultipartUpload {
            inner,
            retrying_store: self.clone(),
            location: location.clone(),
            put_id,
        }))
    }

    fn delete_stream(
        &self,
        locations: BoxStream<'static, object_store::Result<Path>>,
    ) -> BoxStream<'static, object_store::Result<Path>> {
        let inner = Arc::clone(&self.inner);
        let sleeper = self.sleeper();
        let retry_builder = self.retry_builder();
        locations
            .then(move |loc| {
                let inner = Arc::clone(&inner);
                let sleeper = sleeper.clone();
                async move {
                    let loc = loc?;
                    (|| async { inner.delete(&loc).await })
                        .retry(retry_builder)
                        .sleep(sleeper)
                        .notify(Self::notify)
                        .when(Self::should_retry)
                        .await?;
                    Ok(loc)
                }
            })
            .boxed()
    }

    fn list(&self, prefix: Option<&Path>) -> BoxStream<'static, object_store::Result<ObjectMeta>> {
        let inner = Arc::clone(&self.inner);
        let sleeper = self.sleeper();
        let retry_builder = self.retry_builder();
        let prefix_owned = prefix.cloned();

        // list() is a little more complex than the other functions because:
        // 1. it's sync, not async
        // 2. it paginates and returns a stream of results
        //
        // (2) is particularly challenging because it means it returns before we know the full
        // result. This is problematic--because we can't easily retry half-way through the
        // iteration.
        //
        // To get around this, we convert the entire list into a vector in a single attempt,
        // and then return a stream of those results.
        stream::once(async move {
            (|| async {
                let stream = inner.list(prefix_owned.as_ref());
                // Any error in the stream will return an error for try_collect
                stream.try_collect::<Vec<_>>().await
            })
            .retry(retry_builder)
            .sleep(sleeper)
            .notify(Self::notify)
            .when(Self::should_retry)
            .await
        })
        .map_ok(|entries| {
            // If the list() call succeeded, we need to convert the vector back into
            // a stream of results.
            stream::iter(
                entries
                    .into_iter()
                    .map(Ok::<ObjectMeta, object_store::Error>),
            )
            .boxed()
        })
        .try_flatten()
        .boxed()
    }

    fn list_with_offset(
        &self,
        prefix: Option<&Path>,
        offset: &Path,
    ) -> BoxStream<'static, object_store::Result<ObjectMeta>> {
        let inner = Arc::clone(&self.inner);
        let sleeper = self.sleeper();
        let retry_builder = self.retry_builder();
        let prefix_owned = prefix.cloned();
        let offset_owned = offset.clone();

        // See the comment in list() for details on why we do this.
        stream::once(async move {
            (|| async {
                let stream = inner.list_with_offset(prefix_owned.as_ref(), &offset_owned);
                stream.try_collect::<Vec<_>>().await
            })
            .retry(retry_builder)
            .sleep(sleeper)
            .notify(Self::notify)
            .when(Self::should_retry)
            .await
        })
        .map_ok(|entries| {
            stream::iter(
                entries
                    .into_iter()
                    .map(Ok::<ObjectMeta, object_store::Error>),
            )
            .boxed()
        })
        .try_flatten()
        .boxed()
    }

    async fn list_with_delimiter(&self, prefix: Option<&Path>) -> object_store::Result<ListResult> {
        (|| async { self.inner.list_with_delimiter(prefix).await })
            .retry(self.retry_builder())
            .sleep(self.sleeper())
            .notify(Self::notify)
            .when(Self::should_retry)
            .await
    }

    async fn copy_opts(
        &self,
        from: &Path,
        to: &Path,
        options: CopyOptions,
    ) -> object_store::Result<()> {
        (|| async { self.inner.copy_opts(from, to, options.clone()).await })
            .retry(self.retry_builder())
            .sleep(self.sleeper())
            .notify(Self::notify)
            .when(Self::should_retry)
            .await
    }

    async fn rename_opts(
        &self,
        from: &Path,
        to: &Path,
        options: RenameOptions,
    ) -> object_store::Result<()> {
        (|| async { self.inner.rename_opts(from, to, options.clone()).await })
            .retry(self.retry_builder())
            .sleep(self.sleeper())
            .notify(Self::notify)
            .when(Self::should_retry)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::RetryingObjectStore;
    use crate::test_utils::{ExtensionMarker, ExtensionObjectStore, FlakyObjectStore};
    use bytes::Bytes;
    use futures::TryStreamExt;
    use object_store::memory::InMemory;
    use object_store::path::Path;
    use object_store::{GetOptions, ObjectStore, ObjectStoreExt, PutMode, PutOptions, PutPayload};
    use slatedb_common::clock::{DefaultSystemClock, SystemClock};
    use slatedb_common::DbRand;
    use slatedb_common::MockSystemClock;
    use std::sync::Arc;
    use std::time::Duration;

    fn test_rand() -> Arc<DbRand> {
        Arc::new(DbRand::default())
    }

    fn test_clock() -> Arc<DefaultSystemClock> {
        Arc::new(DefaultSystemClock::new())
    }

    #[tokio::test]
    async fn test_put_opts_retries_transient_until_success() {
        let inner: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let flaky = Arc::new(FlakyObjectStore::new(inner, 1));
        let retrying = RetryingObjectStore::new(flaky.clone(), test_rand(), test_clock(), None);

        let path = Path::from("/data/obj");
        retrying
            .put_opts(
                &path,
                PutPayload::from_bytes(Bytes::from_static(b"hello")),
                PutOptions::default(),
            )
            .await
            .expect("put should succeed after retries");

        // 1 failure + 1 success
        assert_eq!(flaky.put_attempts(), 2);

        let got = retrying.get(&path).await.unwrap();
        assert_eq!(got.bytes().await.unwrap(), Bytes::from_static(b"hello"));
    }

    #[tokio::test]
    async fn test_put_opts_preserves_extensions() {
        let inner: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let marking: Arc<dyn ObjectStore> = Arc::new(ExtensionObjectStore::new(inner));
        let retrying = RetryingObjectStore::new(marking, test_rand(), test_clock(), None);

        let path = Path::from("/data/extension-put");
        let result = retrying
            .put_opts(
                &path,
                PutPayload::from_bytes(Bytes::from_static(b"hello")),
                PutOptions::default(),
            )
            .await
            .expect("put should succeed");

        assert!(result.extensions.get::<ExtensionMarker>().is_some());
    }

    #[cfg(any(feature = "aws", feature = "azure", feature = "gcp"))]
    #[tokio::test]
    async fn test_multipart_retry_delays_use_system_clock() {
        use super::RetryingObjectStorePolicy;
        use futures::FutureExt;
        use object_store::client::HttpErrorKind;
        use object_store::retry::RetryFailure;

        let clock = Arc::new(MockSystemClock::new());
        let policy = RetryingObjectStorePolicy {
            clock: clock.clone(),
            max_retries: None,
        };

        for (attempt, delay_ms) in [
            (1, 100),
            (2, 200),
            (3, 400),
            (4, 800),
            (5, 1000),
            (usize::MAX, 1000),
        ] {
            let mut retry = Box::pin(
                policy.retry_failure(RetryFailure::Transport(HttpErrorKind::Timeout), attempt),
            );
            assert!(retry.as_mut().now_or_never().is_none());
            clock.advance(Duration::from_millis(delay_ms - 1)).await;
            assert!(retry.as_mut().now_or_never().is_none());
            clock.advance(Duration::from_millis(1)).await;
            assert_eq!(retry.now_or_never(), Some(true));
        }
    }

    #[cfg(any(feature = "aws", feature = "azure", feature = "gcp"))]
    #[rstest::rstest]
    #[case(None, usize::MAX, true)]
    #[case(Some(0), 1, false)]
    #[case(Some(2), 1, true)]
    #[case(Some(2), 2, true)]
    #[case(Some(2), 3, false)]
    #[tokio::test(start_paused = true)]
    async fn test_multipart_retry_limit(
        #[case] max_retries: Option<u32>,
        #[case] attempt: usize,
        #[case] expected_retry: bool,
    ) {
        use super::RetryingObjectStorePolicy;
        use object_store::client::HttpErrorKind;
        use object_store::retry::RetryFailure;

        let policy = RetryingObjectStorePolicy {
            clock: test_clock(),
            max_retries,
        };
        assert_eq!(
            policy
                .retry_failure(RetryFailure::Transport(HttpErrorKind::Timeout), attempt)
                .await,
            expected_retry
        );
    }

    #[tokio::test]
    async fn test_get_opts_range_preserves_extensions() {
        let inner: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = Path::from("/data/extension-get");
        inner
            .put(
                &path,
                PutPayload::from_bytes(Bytes::from_static(b"hello world")),
            )
            .await
            .unwrap();
        let marking: Arc<dyn ObjectStore> = Arc::new(ExtensionObjectStore::new(inner));
        let retrying = RetryingObjectStore::new(marking, test_rand(), test_clock(), None);

        let result = retrying
            .get_opts(
                &path,
                GetOptions {
                    range: Some((0..5).into()),
                    ..GetOptions::default()
                },
            )
            .await
            .expect("range read should succeed");

        assert!(result.extensions.get::<ExtensionMarker>().is_some());
        assert_eq!(result.bytes().await.unwrap(), Bytes::from_static(b"hello"));
    }

    #[tokio::test]
    async fn test_put_opts_retry_sleep_uses_system_clock() {
        let inner: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let flaky = Arc::new(FlakyObjectStore::new(inner, 1));
        let clock = Arc::new(MockSystemClock::new());
        let retrying = RetryingObjectStore::new(flaky.clone(), test_rand(), clock.clone(), None);
        let path = Path::from("/data/obj");

        let handle = tokio::spawn({
            let retrying = retrying.clone();
            let path = path.clone();
            async move {
                retrying
                    .put_opts(
                        &path,
                        PutPayload::from_bytes(Bytes::from_static(b"hello")),
                        PutOptions::default(),
                    )
                    .await
            }
        });

        flaky.wait_for_put_attempts(1).await;
        assert_eq!(flaky.put_attempts(), 1);
        assert!(!handle.is_finished());

        clock.advance(Duration::from_millis(99)).await;
        tokio::task::yield_now().await;
        assert_eq!(flaky.put_attempts(), 1);
        assert!(!handle.is_finished());

        clock.advance(Duration::from_millis(1)).await;

        let result = handle.await.unwrap();
        assert!(
            result.is_ok(),
            "put should succeed after clock-driven retry"
        );
        assert_eq!(flaky.put_attempts(), 2);

        let got = retrying.get(&path).await.unwrap();
        assert_eq!(got.bytes().await.unwrap(), Bytes::from_static(b"hello"));
    }

    #[tokio::test]
    async fn test_put_opts_does_not_retry_on_already_exists() {
        let inner: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let flaky = Arc::new(FlakyObjectStore::new(inner, 0));
        let retrying = RetryingObjectStore::new(flaky.clone(), test_rand(), test_clock(), None);
        let path = Path::from("/data/obj");

        retrying
            .put_opts(
                &path,
                PutPayload::from_bytes(Bytes::from_static(b"v1")),
                PutOptions::from(PutMode::Create),
            )
            .await
            .unwrap();

        let attempts_before = flaky.put_attempts();
        let err = retrying
            .put_opts(
                &path,
                PutPayload::from_bytes(Bytes::from_static(b"v2")),
                PutOptions::from(PutMode::Create),
            )
            .await
            .expect_err("second put should fail with AlreadyExists");

        // Should be AlreadyExists
        match err {
            object_store::Error::AlreadyExists { .. } => {}
            other => panic!("unexpected error: {other:?}"),
        }

        // Should not retry on AlreadyExists → exactly one new attempt
        assert_eq!(flaky.put_attempts(), attempts_before + 1);
    }

    #[tokio::test]
    async fn test_head_retries_transient_until_success() {
        let inner: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = Path::from("/x");
        inner
            .put(&path, PutPayload::from_bytes(Bytes::from_static(b"data")))
            .await
            .unwrap();

        let flaky = Arc::new(FlakyObjectStore::new(inner, 0).with_head_failures(1));
        let retrying = RetryingObjectStore::new(flaky.clone(), test_rand(), test_clock(), None);

        let meta = retrying.head(&path).await.expect("head should succeed");
        assert_eq!(meta.size, 4);
        assert_eq!(flaky.head_attempts(), 2);
    }

    #[tokio::test]
    async fn test_put_opts_does_not_retry_on_precondition() {
        let inner: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let failing = Arc::new(FlakyObjectStore::new(inner, 0).with_put_precondition_always());
        let retrying = RetryingObjectStore::new(failing.clone(), test_rand(), test_clock(), None);
        let path = Path::from("/p");

        let err = retrying
            .put_opts(
                &path,
                PutPayload::from_bytes(Bytes::from_static(b"x")),
                PutOptions::default(),
            )
            .await
            .expect_err("expected precondition error");

        match err {
            object_store::Error::Precondition { .. } => {}
            e => panic!("unexpected error: {e:?}"),
        }
        assert_eq!(failing.put_attempts(), 1);
    }

    #[tokio::test]
    async fn test_get_opts_does_not_retry_on_not_modified() {
        let inner: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let retrying = RetryingObjectStore::new(inner.clone(), test_rand(), test_clock(), None);
        let path = Path::from("/data/obj");

        retrying
            .put(&path, PutPayload::from_bytes(Bytes::from_static(b"data")))
            .await
            .unwrap();
        let e_tag = retrying.head(&path).await.unwrap().e_tag.unwrap();

        let err = retrying
            .get_opts(
                &path,
                GetOptions {
                    if_none_match: Some(e_tag),
                    ..GetOptions::default()
                },
            )
            .await
            .expect_err("matching if-none-match should surface NotModified");

        match err {
            object_store::Error::NotModified { .. } => {}
            e => panic!("unexpected error: {e:?}"),
        }
    }

    #[tokio::test]
    async fn test_list_retries_transient_until_success() {
        let inner: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let paths = [
            Path::from("/items/a"),
            Path::from("/items/b"),
            Path::from("/items/c"),
        ];
        for (idx, path) in paths.iter().enumerate() {
            inner
                .put(
                    path,
                    PutPayload::from_bytes(Bytes::from(format!("val-{idx}").into_bytes())),
                )
                .await
                .unwrap();
        }

        let flaky = Arc::new(FlakyObjectStore::new(inner, 0).with_list_failures(1, 1));
        let retrying = RetryingObjectStore::new(flaky.clone(), test_rand(), test_clock(), None);

        let listed: Vec<_> = retrying
            .list(None)
            .try_collect()
            .await
            .expect("list should eventually succeed");
        assert_eq!(listed.len(), paths.len());
        let mut names: Vec<_> = listed.into_iter().map(|m| m.location.to_string()).collect();
        names.sort();
        let mut expected: Vec<_> = paths.iter().map(|p| p.to_string()).collect();
        expected.sort();
        assert_eq!(names, expected);
        assert_eq!(flaky.list_attempts(), 2);
    }

    #[tokio::test]
    async fn test_list_with_offset_retries_transient_until_success() {
        let inner: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let paths = [
            Path::from("/items/a"),
            Path::from("/items/b"),
            Path::from("/items/c"),
        ];
        for (idx, path) in paths.iter().enumerate() {
            inner
                .put(
                    path,
                    PutPayload::from_bytes(Bytes::from(format!("val-{idx}").into_bytes())),
                )
                .await
                .unwrap();
        }

        let flaky = Arc::new(FlakyObjectStore::new(inner, 0).with_list_with_offset_failures(1, 1));
        let retrying = RetryingObjectStore::new(flaky.clone(), test_rand(), test_clock(), None);
        let offset = Path::from("/items/a");

        let listed: Vec<_> = retrying
            .list_with_offset(None, &offset)
            .try_collect()
            .await
            .expect("list_with_offset should eventually succeed");

        // Expect entries after the offset (at least b and c)
        let mut names: Vec<_> = listed.into_iter().map(|m| m.location.to_string()).collect();
        names.sort();
        assert!(names.contains(&"items/b".to_string()));
        assert!(names.contains(&"items/c".to_string()));
        assert_eq!(flaky.list_with_offset_attempts(), 2);
    }

    #[tokio::test]
    async fn test_put_opts_succeeds_on_matching_ulid() {
        // Simulate: put succeeds but returns AlreadyExists error (timeout after write)
        // The ULID in the object's metadata should match, so we return success
        let inner: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let flaky = Arc::new(
            FlakyObjectStore::new(inner, 0).with_put_succeeds_but_returns_already_exists(),
        );
        let retrying = RetryingObjectStore::new(flaky.clone(), test_rand(), test_clock(), None);
        let path = Path::from("/data/obj");

        // Must use PutMode::Create to trigger ULID verification
        let result = retrying
            .put_opts(
                &path,
                PutPayload::from_bytes(Bytes::from_static(b"hello")),
                PutOptions::from(PutMode::Create),
            )
            .await;

        assert!(result.is_ok(), "put should succeed via ULID verification");

        let got = retrying.get(&path).await.unwrap();
        assert_eq!(got.bytes().await.unwrap(), Bytes::from_static(b"hello"));
    }

    #[tokio::test]
    async fn test_put_opts_fails_on_mismatched_ulid() {
        // First write a file with different ULID (simulating another client's write)
        let inner: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = Path::from("/data/obj");

        // Write directly to inner store (no ULID from RetryingObjectStore)
        inner
            .put_opts(
                &path,
                PutPayload::from_bytes(Bytes::from_static(b"other client data")),
                PutOptions::from(PutMode::Create),
            )
            .await
            .unwrap();

        // Now try to write via RetryingObjectStore - should fail because ULID won't match
        let retrying = RetryingObjectStore::new(inner.clone(), test_rand(), test_clock(), None);
        let err = retrying
            .put_opts(
                &path,
                PutPayload::from_bytes(Bytes::from_static(b"my data")),
                PutOptions::from(PutMode::Create),
            )
            .await
            .expect_err("put should fail because file exists with different ULID");

        match err {
            object_store::Error::AlreadyExists { .. } => {}
            e => panic!("unexpected error: {e:?}"),
        }

        let got = retrying.get(&path).await.unwrap();
        assert_eq!(
            got.bytes().await.unwrap(),
            Bytes::from_static(b"other client data")
        );
    }

    #[tokio::test]
    async fn test_get_range_retries_transient_until_success() {
        let inner: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = Path::from("/data/obj");
        inner
            .put(
                &path,
                PutPayload::from_bytes(Bytes::from_static(b"hello world")),
            )
            .await
            .unwrap();

        let flaky = Arc::new(FlakyObjectStore::new(inner, 0).with_get_range_failures(2));
        let retrying = RetryingObjectStore::new(flaky.clone(), test_rand(), test_clock(), None);

        let result = retrying
            .get_range(&path, 0..5)
            .await
            .expect("should succeed after retries");
        assert_eq!(result, Bytes::from_static(b"hello"));
        // 2 failures + 1 success
        assert_eq!(flaky.get_range_attempts(), 3);
    }

    #[tokio::test]
    async fn test_get_ranges_retries_transient_until_success() {
        let inner: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = Path::from("/data/obj");
        inner
            .put(
                &path,
                PutPayload::from_bytes(Bytes::from_static(b"hello world")),
            )
            .await
            .unwrap();

        // get_ranges calls get_range internally, so flaky get_range failures will trigger retries
        let flaky = Arc::new(FlakyObjectStore::new(inner, 0).with_get_range_failures(2));
        let retrying = RetryingObjectStore::new(flaky.clone(), test_rand(), test_clock(), None);

        let ranges = vec![0..5, 6..11];
        let result = retrying
            .get_ranges(&path, &ranges)
            .await
            .expect("should succeed after retries");
        assert_eq!(result.len(), 2);
        assert_eq!(result[0], Bytes::from_static(b"hello"));
        assert_eq!(result[1], Bytes::from_static(b"world"));
    }

    #[tokio::test]
    async fn test_put_opts_preserves_user_attributes() {
        use object_store::{Attribute, Attributes, GetOptions};
        use std::borrow::Cow;

        let inner: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let retrying = RetryingObjectStore::new(inner.clone(), test_rand(), test_clock(), None);
        let path = Path::from("/data/obj");

        let mut user_attrs = Attributes::new();
        user_attrs.insert(
            Attribute::ContentType,
            object_store::AttributeValue::from("application/json"),
        );
        user_attrs.insert(
            Attribute::Metadata(Cow::Owned("custom-key".to_string())),
            object_store::AttributeValue::from("custom-value"),
        );

        // Must use PutMode::Create to trigger ULID attribute addition
        let opts = PutOptions {
            attributes: user_attrs,
            mode: PutMode::Create,
            ..Default::default()
        };

        retrying
            .put_opts(
                &path,
                PutPayload::from_bytes(Bytes::from_static(b"{}")),
                opts,
            )
            .await
            .unwrap();

        let get_opts = GetOptions {
            head: true,
            ..Default::default()
        };
        let result = inner.get_opts(&path, get_opts).await.unwrap();

        assert_eq!(
            result.attributes.get(&Attribute::ContentType),
            Some(&object_store::AttributeValue::from("application/json"))
        );
        assert_eq!(
            result
                .attributes
                .get(&Attribute::Metadata(Cow::Borrowed("custom-key"))),
            Some(&object_store::AttributeValue::from("custom-value"))
        );

        assert!(result
            .attributes
            .get(&Attribute::Metadata(Cow::Borrowed(super::PUT_ID_ATTRIBUTE)))
            .is_some());
    }

    #[tokio::test]
    async fn test_get_opts_range_read_size_check_passes() {
        let inner: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let retrying = RetryingObjectStore::new(inner.clone(), test_rand(), test_clock(), None);
        let path = Path::from("/data/obj");

        inner
            .put(
                &path,
                PutPayload::from_bytes(Bytes::from_static(b"hello world")),
            )
            .await
            .unwrap();

        let result = retrying
            .get_opts(
                &path,
                GetOptions {
                    range: Some((0..5).into()),
                    ..GetOptions::default()
                },
            )
            .await
            .expect("range read should pass size check");

        let bytes = result.bytes().await.unwrap();
        assert_eq!(bytes, Bytes::from_static(b"hello"));
    }

    #[tokio::test]
    async fn test_get_opts_range_read_size_mismatch_returns_error() {
        let inner: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let path = Path::from("/data/obj");
        inner
            .put(
                &path,
                PutPayload::from_bytes(Bytes::from_static(b"hello world")),
            )
            .await
            .unwrap();

        let flaky = Arc::new(FlakyObjectStore::new(inner, 0).with_truncate_get_range_bytes(1, 1));
        let retrying = RetryingObjectStore::new(flaky.clone(), test_rand(), test_clock(), None);

        // First attempt returns truncated body (1 byte vs 5 expected),
        // triggering the size check error. The retry succeeds normally.
        let result = retrying
            .get_opts(
                &path,
                GetOptions {
                    range: Some((0..5).into()),
                    ..GetOptions::default()
                },
            )
            .await
            .expect("should succeed after retrying past truncated response");

        let bytes = result.bytes().await.unwrap();
        assert_eq!(bytes, Bytes::from_static(b"hello"));
        // 1 truncated attempt + 1 successful retry
        assert_eq!(flaky.get_range_attempts(), 2);
    }

    #[tokio::test]
    async fn test_bounded_max_retries_gives_up_instead_of_retrying_forever() {
        // Store fails more times (5) than the configured retry bound (2).
        let inner: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let flaky = Arc::new(FlakyObjectStore::new(inner, 5));
        let retrying = RetryingObjectStore::new(flaky.clone(), test_rand(), test_clock(), Some(2));

        let path = Path::from("/data/obj");
        let err = retrying
            .put_opts(
                &path,
                PutPayload::from_bytes(Bytes::from_static(b"hello")),
                PutOptions::default(),
            )
            .await
            .expect_err("bounded retries should exhaust and surface the underlying error");

        // The underlying transient error is returned rather than being retried
        // forever, so callers/background tasks can fail fast.
        assert!(matches!(err, object_store::Error::Generic { .. }));
        // 1 initial attempt + 2 retries = 3 total attempts.
        assert_eq!(flaky.put_attempts(), 3);
    }
}
