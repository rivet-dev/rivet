use crate::wal::{WalError, WalObserver, WalStatus, WalStatusListener};

#[derive(Clone, Debug)]
pub(crate) struct DisabledWalObserver {
    status: WalStatus,
}

impl DisabledWalObserver {
    pub(crate) fn new(status: WalStatus) -> Self {
        Self { status }
    }
}

impl WalObserver for DisabledWalObserver {
    fn status(&self) -> Result<WalStatus, WalStatus> {
        Ok(self.status.clone())
    }

    fn subscribe(&self, _listener: WalStatusListener) -> Result<(), WalError> {
        Ok(())
    }
}
