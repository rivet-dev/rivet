use std::ops::Range;

use bytes::Bytes;

use crate::error::SlateDBError;

pub(crate) trait ReadOnlyBlob {
    async fn len(&self) -> Result<u64, SlateDBError>;

    async fn read_range(&self, range: Range<u64>) -> Result<Bytes, SlateDBError>;

    #[allow(dead_code)]
    async fn read(&self) -> Result<Bytes, SlateDBError>;
}
