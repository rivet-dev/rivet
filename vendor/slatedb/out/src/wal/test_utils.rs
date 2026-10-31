use crate::wal::{
    FlushResultFuture, WalError, WalObserver, WalStatus, WalStatusListener, WalWriter,
};
use crate::RowEntry;
use futures::FutureExt;

pub(crate) struct FakeWalWriter {
    status: WalStatus,
}

impl FakeWalWriter {
    pub(crate) fn new(last_flushed_wal_id: u64) -> Self {
        Self::new_with_closed_reason(last_flushed_wal_id, None)
    }

    pub(crate) fn new_with_closed_reason(
        last_flushed_wal_id: u64,
        closed_reason: Option<WalError>,
    ) -> Self {
        let status = WalStatus {
            estimated_bytes: 0,
            last_flushed_wal_id,
            last_flushed_seq: None,
            buffered_wal_entries_count: 0,
            closed_reason,
        };
        Self { status }
    }
}

#[async_trait::async_trait]
impl WalWriter for FakeWalWriter {
    async fn append(&mut self, _write_batch: &[RowEntry]) -> Result<(), WalError> {
        Ok(())
    }

    async fn flush(&mut self) -> Result<FlushResultFuture, WalError> {
        Ok(async { Ok(()) }.boxed())
    }

    fn observer(&self) -> Box<dyn WalObserver> {
        Box::new(FakeWalObserver {
            status: self.status.clone(),
        })
    }

    fn status(&self) -> Result<WalStatus, WalStatus> {
        Ok(self.status.clone())
    }

    async fn close(&mut self) -> Result<(), WalError> {
        Ok(())
    }
}

pub(crate) struct FakeWalObserver {
    status: WalStatus,
}

impl WalObserver for FakeWalObserver {
    fn status(&self) -> Result<WalStatus, WalStatus> {
        Ok(self.status.clone())
    }

    fn subscribe(&self, _listener: WalStatusListener) -> Result<(), WalError> {
        Ok(())
    }
}
