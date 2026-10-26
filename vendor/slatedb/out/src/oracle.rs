use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering::SeqCst;
use std::sync::Arc;

use crate::db_status::DbStatusManager;

/// Oracle is a trait that centralizes the generation & maintenance of various
/// sequence numbers. These sequence numbers are mostly related to the lifecycle
/// of a transaction commit.
pub(crate) trait Oracle: Send + Sync + 'static {
    /// The sequence number of the most recent write that has been fully committed.
    /// For reads with dirty=false, the maximum visible sequence number is capped
    /// at last_committed_seq.
    fn last_committed_seq(&self) -> u64;

    /// The sequence number of the most recent write that has been fully durable
    /// flushed to the remote storage.
    fn last_remote_persisted_seq(&self) -> u64;
}

pub(crate) struct DbOracle {
    // Fields are intentionally private so that all mutations go through methods
    // on this struct.
    last_seq: AtomicU64,
    last_committed_seq: AtomicU64,
    last_durable_seq: AtomicU64,
    status_reporter: Arc<DbStatusManager>,
}

impl DbOracle {
    pub(crate) fn new(
        last_seq: u64,
        last_committed_seq: u64,
        last_durable_seq: u64,
        status_reporter: Arc<DbStatusManager>,
    ) -> Self {
        Self {
            last_seq: AtomicU64::new(last_seq),
            last_committed_seq: AtomicU64::new(last_committed_seq),
            last_durable_seq: AtomicU64::new(last_durable_seq),
            status_reporter,
        }
    }

    pub(crate) fn next_seq(&self) -> u64 {
        self.last_seq.fetch_add(1, SeqCst) + 1
    }

    pub(crate) fn last_seq(&self) -> u64 {
        self.last_seq.load(SeqCst)
    }

    pub(crate) fn advance_last_seq(&self, seq: u64) {
        self.last_seq.fetch_max(seq, SeqCst);
    }

    pub(crate) fn advance_committed_seq(&self, seq: u64) {
        self.last_committed_seq.fetch_max(seq, SeqCst);
    }

    pub(crate) fn advance_durable_seq(&self, seq: u64) {
        self.last_durable_seq.fetch_max(seq, SeqCst);
        self.status_reporter.report_durable_seq(seq);
    }
}

impl Oracle for DbOracle {
    fn last_committed_seq(&self) -> u64 {
        self.last_committed_seq.load(SeqCst)
    }

    fn last_remote_persisted_seq(&self) -> u64 {
        self.last_durable_seq.load(SeqCst)
    }
}

pub(crate) struct DbReaderOracle {
    last_remote_persisted_seq: AtomicU64,
    status_reporter: DbStatusManager,
}

impl DbReaderOracle {
    /// for the read-only db instance (DbReader), only the last remote persisted sequence number
    /// is needed to be tracked, and last_seq and last_remote_persisted_seq are considered to be
    /// the same as last_committed_seq.
    pub(crate) fn new(last_remote_persisted_seq: u64, status_reporter: DbStatusManager) -> Self {
        Self {
            last_remote_persisted_seq: AtomicU64::new(last_remote_persisted_seq),
            status_reporter,
        }
    }

    pub(crate) fn advance_durable_seq(&self, seq: u64) {
        self.last_remote_persisted_seq.fetch_max(seq, SeqCst);
        self.status_reporter.report_durable_seq(seq);
    }
}

impl Oracle for DbReaderOracle {
    fn last_committed_seq(&self) -> u64 {
        self.last_remote_persisted_seq.load(SeqCst)
    }

    fn last_remote_persisted_seq(&self) -> u64 {
        self.last_remote_persisted_seq.load(SeqCst)
    }
}
