use std::{
	collections::BTreeMap,
	ops::Bound,
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
	// Keyed by commit version, which is unique per committed transaction (unlike start version, which
	// concurrent transactions can share). The ordering lets the conflict scan skip transactions whose
	// commit version cannot overlap the committing transaction, and lets pruning drop expired entries
	// from the front since commit versions grow with commit time.
	txns: Arc<Mutex<BTreeMap<u64, PreviousTransaction>>>,
	global_version: Arc<AtomicU64>,
}

impl TransactionConflictTracker {
	pub fn new() -> Self {
		TransactionConflictTracker {
			txns: Arc::new(Mutex::new(BTreeMap::new())),
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

		// Prune old entries. Commit versions grow with commit time, so expired entries are
		// contiguous at the front of the map.
		while let Some((_, txn)) = txns.first_key_value() {
			if txn.insert_instant.elapsed() < TXN_CONFLICT_TTL {
				break;
			}

			txns.pop_first();
		}

		// A retained transaction can only conflict if its commit version is greater than this
		// transaction's start version, so skip everything at or below it.
		for (txn2_commit_version, txn2) in
			txns.range((Bound::Excluded(txn1_start_version), Bound::Unbounded))
		{
			// Check txn versions overlap (intersection or encapsulation)
			if txn2.start_version < txn1_commit_version {
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
								txn2_commit_version = %txn2_commit_version,
								"transaction conflict detected"
							);
							return true;
						}
					}
				}
			}
		}

		// If no conflicts were detected, save txn data
		txns.insert(
			txn1_commit_version,
			PreviousTransaction {
				insert_instant: Instant::now(),
				start_version: txn1_start_version,
				conflict_ranges: txn1_conflict_ranges,
			},
		);

		false
	}

	pub async fn remove(&self, txn_commit_version: u64) {
		let mut txns = self.txns.lock().await;
		txns.remove(&txn_commit_version);
	}

	/// Current retained transaction count. Diagnostic: the conflict scan in `check_and_insert` is
	/// O(this) per commit, so a growing map directly inflates per-commit service time.
	pub async fn len(&self) -> usize {
		self.txns.lock().await.len()
	}
}
