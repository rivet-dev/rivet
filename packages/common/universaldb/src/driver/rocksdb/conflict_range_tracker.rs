use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::{FdbError, FdbResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TransactionId(u64);

impl TransactionId {
	pub fn new() -> Self {
		use std::sync::atomic::{AtomicU64, Ordering};
		static COUNTER: AtomicU64 = AtomicU64::new(0);
		TransactionId(COUNTER.fetch_add(1, Ordering::SeqCst))
	}
}

#[derive(Debug, Clone)]
struct ConflictRange {
	begin: Vec<u8>,
	end: Vec<u8>,
	is_write: bool,
}

impl ConflictRange {
	fn overlaps(&self, other: &ConflictRange) -> bool {
		// Ranges overlap if begin1 < end2 and begin2 < end1
		self.begin < other.end && other.begin < self.end
	}

	fn conflicts_with(&self, other: &ConflictRange) -> bool {
		// Two ranges conflict if they overlap and at least one is a write
		self.overlaps(other) && (self.is_write || other.is_write)
	}
}

struct TrackerState {
	// Map from transaction ID to its held conflict ranges
	transaction_ranges: HashMap<TransactionId, Vec<ConflictRange>>,
}

pub struct ConflictRangeTracker {
	state: Arc<RwLock<TrackerState>>,
}

impl ConflictRangeTracker {
	pub fn new() -> Self {
		ConflictRangeTracker {
			state: Arc::new(RwLock::new(TrackerState {
				transaction_ranges: HashMap::new(),
			})),
		}
	}

	/// Check if a range conflicts with any existing ranges from other transactions
	pub fn check_conflict(
		&self,
		tx_id: TransactionId,
		begin: &[u8],
		end: &[u8],
		is_write: bool,
	) -> FdbResult<()> {
		let new_range = ConflictRange {
			begin: begin.to_vec(),
			end: end.to_vec(),
			is_write,
		};

		let state = self.state.read().unwrap();

		// Check against all other transactions' ranges
		for (other_tx_id, ranges) in &state.transaction_ranges {
			if *other_tx_id == tx_id {
				// Skip our own ranges
				continue;
			}

			for existing_range in ranges {
				if new_range.conflicts_with(existing_range) {
					// Found a conflict - return retryable error
					return Err(FdbError::from_code(1020));
				}
			}
		}

		Ok(())
	}

	/// Add a conflict range for a transaction
	pub fn add_range(
		&self,
		tx_id: TransactionId,
		begin: &[u8],
		end: &[u8],
		is_write: bool,
	) -> FdbResult<()> {
		// First check for conflicts
		self.check_conflict(tx_id, begin, end, is_write)?;

		let new_range = ConflictRange {
			begin: begin.to_vec(),
			end: end.to_vec(),
			is_write,
		};

		let mut state = self.state.write().unwrap();
		state
			.transaction_ranges
			.entry(tx_id)
			.or_insert_with(Vec::new)
			.push(new_range);

		Ok(())
	}

	/// Release all conflict ranges for a transaction
	pub fn release_transaction(&self, tx_id: TransactionId) {
		let mut state = self.state.write().unwrap();
		state.transaction_ranges.remove(&tx_id);
	}

	/// Get all ranges held by a transaction (for debugging)
	pub fn get_transaction_ranges(&self, tx_id: TransactionId) -> Vec<(Vec<u8>, Vec<u8>, bool)> {
		let state = self.state.read().unwrap();
		state
			.transaction_ranges
			.get(&tx_id)
			.map(|ranges| {
				ranges
					.iter()
					.map(|r| (r.begin.clone(), r.end.clone(), r.is_write))
					.collect()
			})
			.unwrap_or_default()
	}

	/// Clear all conflict ranges (for testing)
	pub fn clear_all(&self) {
		let mut state = self.state.write().unwrap();
		state.transaction_ranges.clear();
	}
}

impl Clone for ConflictRangeTracker {
	fn clone(&self) -> Self {
		ConflictRangeTracker {
			state: Arc::clone(&self.state),
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_no_conflict_different_ranges() {
		let tracker = ConflictRangeTracker::new();
		let tx1 = TransactionId::new();
		let tx2 = TransactionId::new();

		// Add range for tx1
		tracker
			.add_range(tx1, b"a", b"b", false)
			.expect("Should add first range");

		// Add non-overlapping range for tx2
		tracker
			.add_range(tx2, b"c", b"d", false)
			.expect("Should add non-overlapping range");
	}

	#[test]
	fn test_read_read_no_conflict() {
		let tracker = ConflictRangeTracker::new();
		let tx1 = TransactionId::new();
		let tx2 = TransactionId::new();

		// Add read range for tx1
		tracker
			.add_range(tx1, b"a", b"c", false)
			.expect("Should add first read range");

		// Add overlapping read range for tx2 - should not conflict
		tracker
			.add_range(tx2, b"b", b"d", false)
			.expect("Should add overlapping read range");
	}

	#[test]
	fn test_read_write_conflict() {
		let tracker = ConflictRangeTracker::new();
		let tx1 = TransactionId::new();
		let tx2 = TransactionId::new();

		// Add read range for tx1
		tracker
			.add_range(tx1, b"a", b"c", false)
			.expect("Should add read range");

		// Try to add overlapping write range for tx2 - should conflict
		let result = tracker.add_range(tx2, b"b", b"d", true);
		assert!(result.is_err());
		// Check for conflict error code 1020
		if let Err(e) = result {
			assert_eq!(e.code(), 1020);
		}
	}

	#[test]
	fn test_write_write_conflict() {
		let tracker = ConflictRangeTracker::new();
		let tx1 = TransactionId::new();
		let tx2 = TransactionId::new();

		// Add write range for tx1
		tracker
			.add_range(tx1, b"a", b"c", true)
			.expect("Should add first write range");

		// Try to add overlapping write range for tx2 - should conflict
		let result = tracker.add_range(tx2, b"b", b"d", true);
		assert!(result.is_err());
		// Check for conflict error code 1020
		if let Err(e) = result {
			assert_eq!(e.code(), 1020);
		}
	}

	#[test]
	fn test_release_transaction() {
		let tracker = ConflictRangeTracker::new();
		let tx1 = TransactionId::new();
		let tx2 = TransactionId::new();

		// Add write range for tx1
		tracker
			.add_range(tx1, b"a", b"c", true)
			.expect("Should add write range");

		// Try to add overlapping range for tx2 - should conflict
		let result = tracker.add_range(tx2, b"b", b"d", true);
		assert!(result.is_err());

		// Release tx1's ranges
		tracker.release_transaction(tx1);

		// Now tx2 should be able to add the range
		tracker
			.add_range(tx2, b"b", b"d", true)
			.expect("Should add range after release");
	}

	#[test]
	fn test_write_read_conflict() {
		let tracker = ConflictRangeTracker::new();
		let tx1 = TransactionId::new();
		let tx2 = TransactionId::new();

		// Add write range for tx1
		tracker
			.add_range(tx1, b"a", b"c", true)
			.expect("Should add write range");

		// Try to add overlapping read range for tx2 - should conflict
		let result = tracker.add_range(tx2, b"b", b"d", false);
		assert!(result.is_err());
		// Check for conflict error code 1020
		if let Err(e) = result {
			assert_eq!(e.code(), 1020);
		}
	}

	#[test]
	fn test_same_transaction_no_conflict() {
		let tracker = ConflictRangeTracker::new();
		let tx1 = TransactionId::new();

		// Add write range for tx1
		tracker
			.add_range(tx1, b"a", b"c", true)
			.expect("Should add write range");

		// Add overlapping write range for same transaction - should not conflict
		tracker
			.add_range(tx1, b"b", b"d", true)
			.expect("Should add overlapping range for same transaction");
	}

	#[test]
	fn test_exact_boundary_no_conflict() {
		let tracker = ConflictRangeTracker::new();
		let tx1 = TransactionId::new();
		let tx2 = TransactionId::new();

		// Add range [a, b) for tx1
		tracker
			.add_range(tx1, b"a", b"b", true)
			.expect("Should add first range");

		// Add range [b, c) for tx2 - should not conflict (adjacent ranges)
		tracker
			.add_range(tx2, b"b", b"c", true)
			.expect("Should add adjacent range");
	}

	#[test]
	fn test_clear_all() {
		let tracker = ConflictRangeTracker::new();
		let tx1 = TransactionId::new();
		let tx2 = TransactionId::new();

		// Add write range for tx1
		tracker
			.add_range(tx1, b"a", b"c", true)
			.expect("Should add write range");

		// Try to add overlapping range for tx2 - should conflict
		let result = tracker.add_range(tx2, b"b", b"d", true);
		assert!(result.is_err());

		// Clear all ranges
		tracker.clear_all();

		// Now tx2 should be able to add the range
		tracker
			.add_range(tx2, b"b", b"d", true)
			.expect("Should add range after clear_all");
	}

	#[test]
	fn test_get_transaction_ranges() {
		let tracker = ConflictRangeTracker::new();
		let tx1 = TransactionId::new();

		// Add multiple ranges for tx1
		tracker
			.add_range(tx1, b"a", b"b", false)
			.expect("Should add read range");
		tracker
			.add_range(tx1, b"c", b"d", true)
			.expect("Should add write range");

		// Get ranges for tx1
		let ranges = tracker.get_transaction_ranges(tx1);
		assert_eq!(ranges.len(), 2);

		// Check first range (read)
		assert_eq!(ranges[0].0, b"a");
		assert_eq!(ranges[0].1, b"b");
		assert_eq!(ranges[0].2, false);

		// Check second range (write)
		assert_eq!(ranges[1].0, b"c");
		assert_eq!(ranges[1].1, b"d");
		assert_eq!(ranges[1].2, true);
	}
}
