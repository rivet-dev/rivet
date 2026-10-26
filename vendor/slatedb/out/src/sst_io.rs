use std::ops::Range;
use std::sync::Arc;

use bytes::Bytes;
use log::warn;
use object_store::path::Path;
use object_store::{Extensions, GetOptions, GetRange, ObjectStore};

use crate::blob::ReadOnlyBlob;
use crate::error::SlateDBError;
use crate::object_store_tag::ObjectStoreCallTag;

/// Reads one SST object with validation retry while attaching the supplied
/// object-store call tag to every attempt.
macro_rules! read_obj {
    ($object_store:expr, $path:expr, $tag:expr, |$obj:ident| $read:expr) => {{
        let object_store = $object_store;
        let path = $path;
        $crate::sst_io::read_with_validation_retry($tag, move |tag| {
            let object_store = object_store.clone();
            let path = path.clone();
            async move {
                let $obj = $crate::sst_io::ReadOnlyObject {
                    object_store,
                    path,
                    tag,
                };
                $read.await.map_err(|error| error.with_path(&$obj.path))
            }
        })
    }};
}

pub(crate) use read_obj;

/// An object-store object exposed through the read-only interface consumed by
/// the shared SST format decoder.
pub(crate) struct ReadOnlyObject {
    pub(crate) object_store: Arc<dyn ObjectStore>,
    pub(crate) path: Path,
    pub(crate) tag: ObjectStoreCallTag,
}

impl ReadOnlyObject {
    fn extensions(&self) -> Extensions {
        self.tag.clone().into()
    }
}

impl ReadOnlyBlob for ReadOnlyObject {
    async fn len(&self) -> Result<u64, SlateDBError> {
        let opts = GetOptions {
            head: true,
            extensions: self.extensions(),
            ..GetOptions::default()
        };
        let result = self.object_store.get_opts(&self.path, opts).await?;
        Ok(result.meta.size)
    }

    async fn read_range(&self, range: Range<u64>) -> Result<Bytes, SlateDBError> {
        let opts = GetOptions {
            range: Some(GetRange::Bounded(range)),
            extensions: self.extensions(),
            ..GetOptions::default()
        };
        let result = self.object_store.get_opts(&self.path, opts).await?;
        Ok(result.bytes().await?)
    }

    async fn read(&self) -> Result<Bytes, SlateDBError> {
        let opts = GetOptions {
            extensions: self.extensions(),
            ..GetOptions::default()
        };
        let result = self.object_store.get_opts(&self.path, opts).await?;
        Ok(result.bytes().await?)
    }
}

/// Number of additional attempts after an SST read fails validation.
pub(crate) const MAX_VALIDATION_RETRIES: usize = 1;

/// Reissues recoverable validation failures with a retry reason on the object
/// store call tag. Caching object-store wrappers use that reason to invalidate
/// a corrupt local copy before the retry.
pub(crate) async fn read_with_validation_retry<T, Fut>(
    mut tag: ObjectStoreCallTag,
    mut read: impl FnMut(ObjectStoreCallTag) -> Fut,
) -> Result<T, SlateDBError>
where
    Fut: std::future::Future<Output = Result<T, SlateDBError>>,
{
    for _ in 0..MAX_VALIDATION_RETRIES {
        let result = read(tag.clone()).await;
        match result {
            Err(ref err) => match err.maybe_validation_retry_reason() {
                Some(reason) => {
                    warn!(
                        "retrying SST read after validation failure [reason={:?}, error={}]",
                        reason, err
                    );
                    tag.retry = Some(reason);
                }
                None => return result,
            },
            Ok(_) => return result,
        }
    }
    read(tag).await
}
