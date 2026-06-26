use std::{
	sync::{
		Arc,
		atomic::{AtomicU64, Ordering},
	},
	time::{Duration, Instant},
};

use tokio::sync::Mutex;

use crate::options::ConflictRangeType;

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
	txns: Arc<Mutex<Vec<PreviousTransaction>>>,
	global_version: Arc<AtomicU64>,
}

impl TransactionConflictTracker {
	pub fn new() -> Self {
		TransactionConflictTracker {
			txns: Arc::new(Mutex::new(Vec::new())),
			global_version: Arc::new(AtomicU64::new(1)),
		}
	}

	fn next_global_version(&self) -> u64 {
		self.global_version.fetch_add(1, Ordering::SeqCst)
	}

	pub async fn check_and_insert(
		&self,
		txn1_start_version: u64,
		txn1_conflict_ranges: Vec<(Vec<u8>, Vec<u8>, ConflictRangeType)>,
	) -> Option<u64> {
		let mut txns = self.txns.lock().await;
		let txn1_commit_version = self.next_global_version();

		txns.retain(|txn| txn.insert_instant.elapsed() < TXN_CONFLICT_TTL);

		for txn2 in &*txns {
			if txn1_start_version < txn2.commit_version && txn2.start_version < txn1_commit_version
			{
				for (cr1_start, cr1_end, cr1_type) in &txn1_conflict_ranges {
					for (cr2_start, cr2_end, cr2_type) in &txn2.conflict_ranges {
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
							return None;
						}
					}
				}
			}
		}

		txns.push(PreviousTransaction {
			insert_instant: Instant::now(),
			start_version: txn1_start_version,
			commit_version: txn1_commit_version,
			conflict_ranges: txn1_conflict_ranges,
		});

		Some(txn1_commit_version)
	}
}
