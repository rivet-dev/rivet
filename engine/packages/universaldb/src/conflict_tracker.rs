use std::{
	sync::{
		Arc,
		atomic::{AtomicU64, Ordering},
	},
	time::{Duration, Instant},
};

use tokio::sync::Mutex;

use crate::options::ConflictRangeType;

// Transactions cannot live longer than 5 seconds so we don't need to store transaction conflicts longer than
// that.
const TXN_CONFLICT_TTL: Duration = Duration::from_secs(10);

#[derive(Debug)]
struct PreviousTransaction {
	insert_instant: Instant,
	start_version: u64,
	commit_version: u64,
	conflict_ranges: Vec<(Vec<u8>, Vec<u8>, ConflictRangeType)>,
}

/// In-process FoundationDB-style resolver. Holds the last `TXN_CONFLICT_TTL` of committed
/// transactions and rejects a committing transaction if any retained transaction has both an
/// overlapping version window and an overlapping conflict range of a differing type.
///
/// Used by the rocksdb driver (single process) and by the postgres leader-resolver. The two
/// differ only in where the commit version comes from: rocksdb generates it from the in-process
/// `global_version` counter, while the postgres leader assigns it from the durable
/// `udb_version_seq` so it survives leader failover and matches the versionstamp. For that reason
/// `check_and_insert` takes the commit version from the caller instead of generating it.
#[derive(Clone)]
pub struct TransactionConflictTracker {
	// NOTE: We use a mutex because we need to lock reads across all active txns. This could be optimized to
	// only lock txns that have overlapping ranges with the currently checking one, but its a small
	// optimization because most txns are going to be very recent and this only stores the last 10 seconds of
	// txns.
	txns: Arc<Mutex<Vec<PreviousTransaction>>>,
	global_version: Arc<AtomicU64>,
}

impl TransactionConflictTracker {
	pub fn new() -> Self {
		TransactionConflictTracker {
			txns: Arc::new(Mutex::new(Vec::new())),
			global_version: Arc::new(AtomicU64::new(0)),
		}
	}

	/// Each number returned is unique. Used by the in-process rocksdb driver to assign both start
	/// and commit versions. The postgres leader does not use this; it assigns versions from the
	/// durable Postgres sequence.
	pub fn next_global_version(&self) -> u64 {
		self.global_version.fetch_add(1, Ordering::SeqCst)
	}

	/// Returns `true` on conflict (same polarity as the original rocksdb tracker). The caller
	/// supplies `commit_version` (e.g. `nextval('udb_version_seq')` on the postgres leader, or
	/// `next_global_version()` on rocksdb) so version assignment stays the caller's responsibility.
	pub async fn check_and_insert(
		&self,
		txn1_start_version: u64,
		txn1_commit_version: u64,
		txn1_conflict_ranges: Vec<(Vec<u8>, Vec<u8>, ConflictRangeType)>,
	) -> bool {
		let mut txns = self.txns.lock().await;

		// Prune old entries
		txns.retain(|txn| txn.insert_instant.elapsed() < TXN_CONFLICT_TTL);

		for txn2 in &*txns {
			// Check txn versions overlap (intersection or encapsulation)
			if txn1_start_version < txn2.commit_version && txn2.start_version < txn1_commit_version
			{
				for (cr1_start, cr1_end, cr1_type) in &txn1_conflict_ranges {
					for (cr2_start, cr2_end, cr2_type) in &txn2.conflict_ranges {
						// Check conflict ranges overlap
						if cr1_start < cr2_end && cr2_start < cr1_end && cr1_type != cr2_type {
							tracing::debug!(
								cr1_start=%hex::encode(cr1_start),
								cr1_end=%hex::encode(cr1_end),
								?cr1_type,
								cr2_start=%hex::encode(cr2_start),
								cr2_end=%hex::encode(cr2_end),
								?cr2_type,
								txn1_start_version,
								txn1_commit_version,
								txn2_start_version = txn2.start_version,
								txn2_commit_version = txn2.commit_version,
								"transaction conflict detected"
							);
							return true;
						}
					}
				}
			}
		}

		// If no conflicts were detected, save txn data
		txns.push(PreviousTransaction {
			insert_instant: Instant::now(),
			start_version: txn1_start_version,
			commit_version: txn1_commit_version,
			conflict_ranges: txn1_conflict_ranges,
		});

		false
	}

	pub async fn remove(&self, txn_start_version: u64) {
		let mut txns = self.txns.lock().await;

		if let Some(i) = txns
			.iter()
			.enumerate()
			.find_map(|(i, txn)| (txn.start_version == txn_start_version).then_some(i))
		{
			txns.remove(i);
		}
	}
}
