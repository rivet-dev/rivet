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
// that
const TXN_CONFLICT_TTL: Duration = Duration::from_secs(10);

#[derive(Debug)]
struct PreviousTransaction {
	insert_instant: Instant,
	start_version: u64,
	commit_version: u64,
	conflict_ranges: Vec<(Vec<u8>, Vec<u8>, ConflictRangeType)>,
}

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

	/// Each number returned is unique.
	pub fn next_global_version(&self) -> u64 {
		self.global_version.fetch_add(1, Ordering::SeqCst)
	}

	pub async fn check_and_insert(
		&self,
		txn1_start_version: u64,
		txn1_conflict_ranges: Vec<(Vec<u8>, Vec<u8>, ConflictRangeType)>,
	) -> bool {
		let mut txns = self.txns.lock().await;
		let txn1_commit_version = self.next_global_version();

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
