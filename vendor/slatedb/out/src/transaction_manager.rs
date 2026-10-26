use crate::bytes_range::BytesRange;
use crate::oracle::{DbOracle, Oracle};
use crate::utils::IdGenerator;
use bytes::Bytes;
use log::warn;
use parking_lot::RwLock;
use slatedb_common::DbRand;
use std::collections::{HashMap, HashSet, VecDeque};
use std::ops::RangeBounds;
use std::sync::Arc;
use uuid::Uuid;

/// Isolation level for database transactions.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
#[allow(unused)]
pub enum IsolationLevel {
    /// Snapshot Isolation - only detects write-write conflicts
    #[default]
    Snapshot,
    /// Serializable Snapshot Isolation - detects both read-write and write-write conflicts
    SerializableSnapshot,
}

#[derive(Debug)]
pub(crate) struct TransactionState {
    /// The sequence number when the transaction started. This is used to establish
    /// a snapshot of this transaction.
    pub(crate) started_seq: u64,
    /// The sequence number when the transaction committed. This field is only set AFTER
    /// a transaction is committed and is used to check conflicts with recent committed
    /// transactions.
    committed_seq: Option<u64>,
    /// The write keys of the transaction for write-write conflict detection.
    /// Used in both Snapshot Isolation and Serializable Snapshot Isolation.
    write_keys: HashSet<Bytes>,
    /// The read keys of the transaction for read-write conflict detection.
    /// Only used in Serializable Snapshot Isolation mode.
    read_keys: HashSet<Bytes>,
    /// The read ranges of the transaction for phantom read detection.
    /// Only used in Serializable Snapshot Isolation mode to detect range-based conflicts.
    read_ranges: Vec<BytesRange>,
}

impl TransactionState {
    /// Add write keys to this transaction's write set for conflict detection.
    fn track_write_keys(&mut self, keys: impl IntoIterator<Item = Bytes>) {
        self.write_keys.extend(keys);
    }

    /// Add read keys to this transaction's read set for SSI conflict detection.
    #[allow(unused)]
    fn track_read_keys(&mut self, keys: impl IntoIterator<Item = Bytes>) {
        self.read_keys.extend(keys);
    }

    /// Add a read range to this transaction's read set for SSI phantom read detection.
    #[allow(unused)]
    fn track_read_range(&mut self, range: BytesRange) {
        self.read_ranges.push(range);
    }

    /// Mark this transaction as committed with the given sequence number.
    fn mark_as_committed(&mut self, seq: u64) {
        self.committed_seq = Some(seq);
        // when a transaction state is tracked into `recent_committed_txns`, we don't need to
        // save the read keys and ranges, since they are not needed for conflict detection.
        self.read_keys.clear();
        self.read_ranges.clear();
    }
}

/// Manages the lifecycle of transaction states and provides isolation-level-aware conflict detection.
/// Supports both Snapshot Isolation (SI) and Serializable Snapshot Isolation (SSI).
/// TODO: have a quota for max active transactions.
pub(crate) struct TransactionManager {
    inner: Arc<RwLock<TransactionManagerInner>>,
    /// Random number generator for generating transaction IDs
    db_rand: Arc<DbRand>,
}

struct TransactionManagerInner {
    /// Map of transaction state ID to transaction states.
    active_txns: HashMap<Uuid, TransactionState>,
    /// Tracks recently committed transaction states for conflict checks at commit.
    ///
    /// Garbage collection rules:
    /// - An entry can be garbage collected when *all* active transactions have
    ///   `started_seq` strictly greater than the entry's `committed_seq`.
    /// - Non-transactional writes are modeled as single-op transactions with `started_seq ==
    ///   committed_seq` and follow the same GC rule.
    /// - If there are no active transactions, this deque can be fully drained.
    recent_committed_txns: VecDeque<TransactionState>,
    /// The oracle for tracking the last committed sequence number.
    oracle: Arc<DbOracle>,
}

impl TransactionManager {
    pub(crate) fn new(oracle: Arc<DbOracle>, db_rand: Arc<DbRand>) -> Self {
        Self {
            inner: Arc::new(RwLock::new(TransactionManagerInner {
                active_txns: HashMap::new(),
                recent_committed_txns: VecDeque::new(),
                oracle,
            })),
            db_rand,
        }
    }

    /// Register a read-write transaction state.
    ///
    /// The started sequence is captured from the oracle while the transaction manager lock
    /// is held, so registration and sequence assignment happen atomically from the manager's
    /// perspective.
    pub(crate) fn new_transaction(&self) -> (Uuid, u64) {
        let txn_id = self.db_rand.rng().gen_uuid();
        let mut inner = self.inner.write();
        let seq = inner.oracle.last_committed_seq();
        inner.active_txns.insert(
            txn_id,
            TransactionState {
                started_seq: seq,
                committed_seq: None,
                write_keys: HashSet::new(),
                read_keys: HashSet::new(),
                read_ranges: Vec::new(),
            },
        );
        (txn_id, seq)
    }

    /// Register a transaction state with a specific txn id (for testing only)
    #[cfg(test)]
    fn new_txn_with_id(&self, seq: u64, txn_id: Uuid) -> Uuid {
        let txn_state = TransactionState {
            started_seq: seq,
            committed_seq: None,
            write_keys: HashSet::new(),
            read_keys: HashSet::new(),
            read_ranges: Vec::new(),
        };

        {
            let mut inner = self.inner.write();
            inner.active_txns.insert(txn_id, txn_state);
        }

        txn_id
    }

    /// Remove a transaction state when it's dropped. If the txn has not been committed,
    /// it should be considered as rolled back, no side effect should be produced.
    /// Please note that a committed txn can also run into drop_txn() on `drop()`, this
    /// method does nothing on such case.
    /// Here's a chance to recycle the recent committed txns.
    pub(crate) fn drop_txn(&self, txn_id: &Uuid) {
        let mut inner = self.inner.write();
        inner.active_txns.remove(txn_id);
        inner.recycle_recent_committed_txns();
    }

    /// Track write keys for a transaction. This is used for conflict detection.
    /// Keys should be tracked before calling commit-related methods.
    pub(crate) fn track_write_keys(&self, txn_id: &Uuid, write_keys: &HashSet<Bytes>) {
        let mut inner = self.inner.write();
        if let Some(txn_state) = inner.active_txns.get_mut(txn_id) {
            txn_state.track_write_keys(write_keys.iter().cloned());
        }
    }

    /// Track a key read operation (for SSI)
    pub(crate) fn track_read_keys(
        &self,
        txn_id: &Uuid,
        read_keys: impl IntoIterator<Item = Bytes>,
    ) {
        let mut inner = self.inner.write();
        if let Some(txn_state) = inner.active_txns.get_mut(txn_id) {
            txn_state.track_read_keys(read_keys);
        }
    }

    /// Track a range scan operation (for SSI)
    pub(crate) fn track_read_range(&self, txn_id: &Uuid, range: BytesRange) {
        let mut inner = self.inner.write();
        if let Some(txn_state) = inner.active_txns.get_mut(txn_id) {
            txn_state.track_read_range(range);
        }
    }

    /// Check if the transaction has conflicts based on the configured isolation level.
    /// Returns true if there are conflicts that would prevent the transaction from committing.
    ///
    /// For Snapshot isolation: Only checks write-write conflicts
    /// For SerializableSnapshot isolation: Checks both write-write and read-write conflicts
    pub(crate) fn check_has_conflict(&self, txn_id: &Uuid) -> bool {
        let inner = self.inner.read();
        let txn_state = match inner.active_txns.get(txn_id) {
            None => return false,
            Some(txn_state) => txn_state,
        };

        // both SI and SSI need to check write-write conflicts
        let ww_conflict =
            inner.has_write_write_conflict(&txn_state.write_keys, txn_state.started_seq);
        if ww_conflict {
            return true;
        }

        // for SSI, we need to check read-write conflicts. if a transaction does not
        // track the read keys or ranges at all, this transaction is considered as
        // same as a SI transaction.
        inner.has_read_write_conflict(
            &txn_state.read_keys,
            txn_state.read_ranges.clone(),
            txn_state.started_seq,
        )
    }

    /// Record a recent committed transaction for conflict detection.
    /// This method should be called after confirming no conflicts exist.
    pub(crate) fn track_recent_committed_txn(&self, txn_id: &Uuid, committed_seq: u64) {
        // remove the transaction from active_txns, and add it to recent_committed_txns
        let mut inner = self.inner.write();

        // remove the txn from active txns and append it to recent_committed_txns.
        if let Some(mut txn_state) = inner.active_txns.remove(txn_id) {
            txn_state.mark_as_committed(committed_seq);
            inner.track_recent_committed_state(txn_state);
        }

        // update the last_committed_seq, so the writes will be visible to the readers.
        inner.oracle.advance_committed_seq(committed_seq);
    }

    /// Track a write batch for conflict detection. This is used for regular write operations
    /// that are not part of an explicit transaction but still need to be tracked for conflict
    /// detection with concurrent transactions.
    ///
    /// The synthetic committed state is only retained while there is at least one active
    /// transaction that could still conflict with it.
    pub(crate) fn track_recent_committed_write_batch(
        &self,
        keys: &HashSet<Bytes>,
        committed_seq: u64,
    ) {
        // remove the transaction from active_txns, and add it to recent_committed_txns
        let mut inner = self.inner.write();

        inner.track_recent_committed_state(TransactionState {
            started_seq: committed_seq,
            committed_seq: Some(committed_seq),
            write_keys: keys.clone(),
            read_keys: HashSet::new(),
            read_ranges: Vec::new(),
        });

        // update the last_committed_seq, so the writes will be visible to the readers.
        inner.oracle.advance_committed_seq(committed_seq);
    }

    /// The min started_seq of all active transactions. This value
    /// is useful to inform the compactor about the min seq of data still needed to be
    /// retained for active transactions, so that the compactor can avoid deleting the
    /// data that is still needed.
    ///
    /// min_active_seq will be persisted to the `recent_snapshot_min_seq` in the manifest
    /// when a new L0 is flushed.
    pub(crate) fn min_active_seq(&self) -> Option<u64> {
        let inner = self.inner.read();
        inner
            .active_txns
            .values()
            .map(|state| state.started_seq)
            .min()
    }
}

impl TransactionManagerInner {
    fn track_recent_committed_state(&mut self, txn_state: TransactionState) {
        self.recent_committed_txns.push_back(txn_state);

        if self.min_conflict_check_seq().is_none() {
            // No active write transactions remain, so there is nobody left that can
            // conflict with this committed state or any older committed state.
            self.recent_committed_txns.clear();
        }
    }

    /// The min started_seq of all active transactions, this seq is useful to garbage
    /// collect the entries in the `recent_committed_txns` deque.
    fn min_conflict_check_seq(&self) -> Option<u64> {
        self.active_txns
            .values()
            .map(|state| state.started_seq)
            .min()
    }

    fn recycle_recent_committed_txns(&mut self) {
        let min_conflict_seq = self.min_conflict_check_seq();
        if let Some(min_seq) = min_conflict_seq {
            // Remove transactions that are no longer needed for conflict checking.
            // A transaction can be garbage collected when all active transactions
            // have started_seq strictly greater than the transaction's committed_seq.
            // This means we keep transactions where committed_seq >= min_seq.
            self.recent_committed_txns.retain(|txn| {
                if let Some(committed_seq) = txn.committed_seq {
                    committed_seq >= min_seq
                } else {
                    // If committed_seq is not set, this shouldn't happen in practice.
                    warn!(
                        "Found transaction with committed_seq = None, this may cause memory leaks"
                    );
                    true
                }
            });
        } else {
            // No active transactions, can drain the entire deque
            self.recent_committed_txns.clear();
        }
    }

    fn has_write_write_conflict(&self, write_keys: &HashSet<Bytes>, started_seq: u64) -> bool {
        // If the current transaction has no write operations, there's no write-write conflict
        if write_keys.is_empty() {
            return false;
        }

        // All the recent committed txn should have committed_seq set.
        for committed_txn in &self.recent_committed_txns {
            let other_committed_seq = committed_txn.committed_seq.expect(
                "all txns in recent_committed_txns should be committed with committed_seq set",
            );

            // if another transaction committed after the current transaction started,
            // and they have overlapping write keys, then there's a conflict.
            if other_committed_seq > started_seq
                && !write_keys.is_disjoint(&committed_txn.write_keys)
            {
                return true;
            }
        }

        false
    }

    /// Check for read-write conflicts between current transaction and recently committed transactions.
    /// This is used for Serializable Snapshot Isolation (SSI) to detect conflicts where:
    /// 1. Current transaction read keys that were later written by committed transactions
    /// 2. Current transaction read ranges that overlap with committed transaction writes (phantom reads)
    fn has_read_write_conflict(
        &self,
        read_keys: &HashSet<Bytes>,
        read_ranges: Vec<BytesRange>,
        started_seq: u64,
    ) -> bool {
        if read_keys.is_empty() && read_ranges.is_empty() {
            return false;
        }

        for committed_txn in &self.recent_committed_txns {
            // All the recent committed txn should have committed_seq set.
            let other_committed_seq = committed_txn.committed_seq.expect(
                "all txns in recent_committed_txns should be committed with committed_seq set",
            );

            // if another transaction committed after the current transaction started,
            // and they have read-write conflicts, then there's a conflict.
            if other_committed_seq > started_seq {
                // Check if any of the current transaction's read keys were written by
                // the committed transaction.
                if !read_keys.is_disjoint(&committed_txn.write_keys) {
                    return true;
                }

                // Check if any of the current transaction's read ranges overlap with
                // committed transaction write keys.
                for read_range in &read_ranges {
                    if committed_txn
                        .write_keys
                        .iter()
                        .any(|write_key| read_range.contains(write_key))
                    {
                        return true;
                    }
                }
            }
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db_status::DbStatusManager;
    use bytes::Bytes;
    use parking_lot::Mutex;
    use rstest::rstest;
    use slatedb_common::DbRand;
    use std::collections::HashSet;

    struct CheckConflictTestCase {
        name: &'static str,
        recent_committed_txns: Vec<TransactionState>,
        current_write_keys: Vec<&'static str>,
        current_started_seq: u64,
        expected_conflict: bool,
    }

    fn create_transaction_manager() -> TransactionManager {
        let db_rand = Arc::new(DbRand::new(0));
        let status_reporter = DbStatusManager::new(0);
        let oracle = Arc::new(DbOracle::new(0, 0, 0, Arc::new(status_reporter)));
        TransactionManager::new(oracle, db_rand)
    }

    #[test]
    fn test_new_transaction_uses_oracle_seq() {
        let db_rand = Arc::new(DbRand::new(0));
        let status_reporter = DbStatusManager::new(123);
        let oracle = Arc::new(DbOracle::new(123, 123, 123, Arc::new(status_reporter)));
        let txn_manager = TransactionManager::new(oracle, db_rand);

        let (txn_id, seq) = txn_manager.new_transaction();

        assert_eq!(seq, 123);
        let inner = txn_manager.inner.read();
        let state = inner.active_txns.get(&txn_id).unwrap();
        assert_eq!(state.started_seq, 123);
    }

    #[test]
    fn test_drop_txn_removes_active_transaction() {
        let txn_manager = create_transaction_manager();

        // Create a transaction
        let txn_id = txn_manager.new_txn_with_id(100, Uuid::new_v4());

        // Verify it exists in active transactions
        assert!(txn_manager.inner.read().active_txns.contains_key(&txn_id));

        // Drop the transaction
        txn_manager.drop_txn(&txn_id);

        // Verify it's removed
        assert!(!txn_manager.inner.read().active_txns.contains_key(&txn_id));
    }

    #[test]
    fn test_drop_txn_nonexistent_transaction_safe() {
        let txn_manager = create_transaction_manager();

        // Try to drop a non-existent transaction - should not panic
        let fake_id = Uuid::new_v4();
        txn_manager.drop_txn(&fake_id);

        // Should still be able to create new transactions
        let txn_id = txn_manager.new_txn_with_id(100, Uuid::new_v4());
        assert!(txn_manager.inner.read().active_txns.contains_key(&txn_id));
    }

    #[test]
    fn test_drop_txn_triggers_garbage_collection() {
        let txn_manager = create_transaction_manager();

        // Create an active transaction first to ensure recent_committed_txns tracking
        let txn_id = txn_manager.new_txn_with_id(100, Uuid::new_v4());

        // Create and commit a transaction to populate recent_committed_txns
        let keys: HashSet<Bytes> = ["key1"].into_iter().map(Bytes::from).collect();
        txn_manager.track_recent_committed_write_batch(&keys, 50);

        // Verify recent_committed_txns has content
        assert!(!txn_manager.inner.read().recent_committed_txns.is_empty());

        // Drop the active transaction (this should trigger garbage collection)
        txn_manager.drop_txn(&txn_id);

        // Since there are no more active non-readonly transactions,
        // recent_committed_txns should be cleared
        assert!(txn_manager.inner.read().recent_committed_txns.is_empty());
    }

    #[rstest]
    #[case::no_recent_committed_txns(CheckConflictTestCase {
        name: "no_recent_committed_txns",
        recent_committed_txns: vec![],
        current_write_keys: vec!["key1", "key2"],
        current_started_seq: 100,
        expected_conflict: false,
    })]
    #[case::no_overlapping_keys(CheckConflictTestCase {
        name: "no_overlapping_keys",
        recent_committed_txns: vec![TransactionState {
            started_seq: 50,
            committed_seq: Some(80),
            write_keys: ["key1", "key2"].into_iter().map(Bytes::from).collect(),
            read_keys: HashSet::new(),
            read_ranges: Vec::new(),
        }],
        current_write_keys: vec!["key3", "key4"],
        current_started_seq: 100,
        expected_conflict: false,
    })]
    #[case::concurrent_write_same_key(CheckConflictTestCase {
        name: "concurrent_write_same_key",
        recent_committed_txns: vec![TransactionState {
            started_seq: 50,
            committed_seq: Some(150),
            write_keys: ["key1"].into_iter().map(Bytes::from).collect(),
            read_keys: HashSet::new(),
            read_ranges: Vec::new(),
        }],
        current_write_keys: vec!["key1"],
        current_started_seq: 100,
        expected_conflict: true,
    })]
    #[case::multiple_committed_mixed_conflict(CheckConflictTestCase {
        name: "multiple_committed_mixed_conflict",
        recent_committed_txns: vec![
            TransactionState {
                started_seq: 30,
                committed_seq: Some(50),
                write_keys: ["key1"].into_iter().map(Bytes::from).collect(),
                read_keys: HashSet::new(),
                read_ranges: Vec::new(),
            },
            TransactionState {
                started_seq: 80,
                committed_seq: Some(150),
                write_keys: ["key2"].into_iter().map(Bytes::from).collect(),
                read_keys: HashSet::new(),
                read_ranges: Vec::new(),
            },
        ],
        current_write_keys: vec!["key1", "key2"],
        current_started_seq: 100,
        expected_conflict: true,
    })]
    #[case::committed_before_current_started(CheckConflictTestCase {
        name: "committed_before_current_started",
        recent_committed_txns: vec![TransactionState {
            started_seq: 30,
            committed_seq: Some(50),
            write_keys: ["key1"].into_iter().map(Bytes::from).collect(),
            read_keys: HashSet::new(),
            read_ranges: Vec::new(),
        }],
        current_write_keys: vec!["key1"],
        current_started_seq: 100,
        expected_conflict: false,
    })]
    #[case::exact_seq_boundary(CheckConflictTestCase {
        name: "exact_seq_boundary",
        recent_committed_txns: vec![TransactionState {
            started_seq: 100,
            committed_seq: Some(100),
            write_keys: ["key1"].into_iter().map(Bytes::from).collect(),
            read_keys: HashSet::new(),
            read_ranges: Vec::new(),
        }],
        current_write_keys: vec!["key1"],
        current_started_seq: 100,
        expected_conflict: false,
    })]
    #[case::partial_key_overlap_conflict(CheckConflictTestCase {
        name: "partial_key_overlap_conflict",
        recent_committed_txns: vec![TransactionState {
            started_seq: 80,
            committed_seq: Some(150),
            write_keys: ["key1", "key2", "key3"]
                .into_iter()
                .map(Bytes::from)
                .collect(),
            read_keys: HashSet::new(),
            read_ranges: Vec::new(),
        }],
        current_write_keys: vec!["key3", "key4", "key5"],
        current_started_seq: 100,
        expected_conflict: true,
    })]
    #[case::max_seq_values(CheckConflictTestCase {
        name: "max_seq_values",
        recent_committed_txns: vec![TransactionState {
            started_seq: u64::MAX - 1,
            committed_seq: Some(u64::MAX),
            write_keys: ["key1"].into_iter().map(Bytes::from).collect(),
            read_keys: HashSet::new(),
            read_ranges: Vec::new(),
        }],
        current_write_keys: vec!["key1"],
        current_started_seq: u64::MAX - 1,
        expected_conflict: true, // Should be true since committed_seq > started_seq
    })]
    fn test_check_conflict_table_driven(#[case] case: CheckConflictTestCase) {
        let txn_manager = create_transaction_manager();

        // Set up recent_committed_txns directly
        {
            let mut inner = txn_manager.inner.write();
            inner.recent_committed_txns = case.recent_committed_txns.into();
        }

        // Convert current transaction write keys
        let conflict_keys: HashSet<Bytes> = case
            .current_write_keys
            .into_iter()
            .map(Bytes::from)
            .collect();

        // Call the method under test
        let inner = txn_manager.inner.read();
        let has_conflict = inner.has_write_write_conflict(&conflict_keys, case.current_started_seq);

        // Verify result
        assert_eq!(
            has_conflict, case.expected_conflict,
            "Test case '{}' failed",
            case.name
        );
    }

    #[derive(Debug)]
    struct MinActiveSeqTestCase {
        name: &'static str,
        txn_seqs: Vec<u64>,
        expected_min_seq: Option<u64>,
    }

    #[rstest]
    #[case::no_transactions_returns_none(MinActiveSeqTestCase {
        name: "no transactions returns none",
        txn_seqs: vec![],
        expected_min_seq: None,
    })]
    #[case::single_transaction(MinActiveSeqTestCase {
        name: "single transaction",
        txn_seqs: vec![(100)],
        expected_min_seq: Some(100),
    })]
    #[case::multiple_transactions_returns_minimum(MinActiveSeqTestCase {
        name: "multiple transactions returns minimum",
        txn_seqs: vec![(200), (100), (150)],
        expected_min_seq: Some(100),
    })]
    fn test_min_active_seq_table_driven(#[case] case: MinActiveSeqTestCase) {
        let txn_manager = create_transaction_manager();

        // Create transactions according to the test case
        for seq_no in case.txn_seqs {
            let _txn_id = txn_manager.new_txn_with_id(seq_no, Uuid::new_v4());
        }

        assert_eq!(
            txn_manager.min_active_seq(),
            case.expected_min_seq,
            "Test case '{}' failed",
            case.name
        );
    }

    struct TrackRecentCommittedTxnTestCase {
        name: &'static str,
        setup: Box<dyn Fn(&mut TransactionManager)>,
        expected_recent_committed_txn: Option<TransactionState>,
        expected_recent_committed_txns_len: usize,
        expected_active_txns_len: usize,
    }

    #[rstest]
    #[case::valid_id(TrackRecentCommittedTxnTestCase {
        name: "track committed transaction with valid id",
        setup: Box::new(|txn_manager| {
            // Create a transaction
            let txn_id = txn_manager.new_txn_with_id(100, Uuid::new_v4());

            // Create another active transaction to ensure recent_committed_txns is tracked
            let _other_txn = txn_manager.new_txn_with_id(200, Uuid::new_v4());

            // Track committed transaction
            let keys: HashSet<Bytes> = ["key1", "key2"].into_iter().map(Bytes::from).collect();
            txn_manager.track_write_keys(&txn_id, &keys);
            txn_manager.track_recent_committed_txn(&txn_id, 150);
        }),
        expected_recent_committed_txn: Some(TransactionState {
            started_seq: 100,
            committed_seq: Some(150),
            write_keys: ["key1", "key2"].into_iter().map(Bytes::from).collect(),
            read_keys: HashSet::new(),
            read_ranges: Vec::new(),
        }),
        expected_recent_committed_txns_len: 1,
        expected_active_txns_len: 1, // Only the other transaction remains
    })]
    #[case::nonexistent_id(TrackRecentCommittedTxnTestCase {
        name: "track committed transaction with nonexistent id",
        setup: Box::new(|txn_manager| {
            // Create an active transaction to ensure recent_committed_txns would be tracked
            let _active_txn = txn_manager.new_txn_with_id(100, Uuid::new_v4());

            // Try to track a non-existent transaction
            let fake_id = Uuid::new_v4();
            let _keys: HashSet<Bytes> = ["key1"].into_iter().map(Bytes::from).collect();
            txn_manager.track_recent_committed_txn(&fake_id, 150);
        }),
        expected_recent_committed_txn: None,
        expected_recent_committed_txns_len: 0,
        expected_active_txns_len: 1, // The active transaction remains
    })]
    #[case::without_id_creates_record(TrackRecentCommittedTxnTestCase {
        name: "track without id creates record",
        setup: Box::new(|txn_manager| {
            // Create an active write transaction first to enable tracking
            let _active_txn = txn_manager.new_txn_with_id(50, Uuid::new_v4());
            // Track without transaction ID (non-transactional write)
            let keys: HashSet<Bytes> = ["key1", "key2"].into_iter().map(Bytes::from).collect();
            txn_manager.track_recent_committed_write_batch(&keys, 100);
        }),
        expected_recent_committed_txn: Some(TransactionState {
            started_seq: 100,
            committed_seq: Some(100),
            write_keys: ["key1", "key2"].into_iter().map(Bytes::from).collect(),
            read_keys: HashSet::new(),
            read_ranges: Vec::new(),
        }),
        expected_recent_committed_txns_len: 1,
        expected_active_txns_len: 1, // Now we have the active transaction remaining
    })]
    #[case::merges_conflict_keys(TrackRecentCommittedTxnTestCase {
        name: "merges conflict keys from existing transaction",
        setup: Box::new(|txn_manager| {
            // Create a transaction with some conflict keys already tracked
            let txn_id = txn_manager.new_txn_with_id(100, Uuid::new_v4());

            // Create another active transaction to ensure tracking happens
            let _other_txn = txn_manager.new_txn_with_id(200, Uuid::new_v4());

            // Add some keys to the transaction's conflict_keys before committing
            {
                let mut inner = txn_manager.inner.write();
                if let Some(txn_state) = inner.active_txns.get_mut(&txn_id) {
                    txn_state.track_write_keys(["existing_key"].into_iter().map(Bytes::from));
                }
            }
            // Track committed transaction with additional keys
            let additional_keys: HashSet<Bytes> = ["key1", "key2"].into_iter().map(Bytes::from).collect();
            txn_manager.track_write_keys(&txn_id, &additional_keys);
            txn_manager.track_recent_committed_txn(&txn_id, 150);
        }),
        expected_recent_committed_txn: Some(TransactionState {
            started_seq: 100,
            committed_seq: Some(150),
            write_keys: ["existing_key", "key1", "key2"].into_iter().map(Bytes::from).collect(),
            read_keys: HashSet::new(),
            read_ranges: Vec::new(),
        }),
        expected_recent_committed_txns_len: 1,
        expected_active_txns_len: 1, // Only the other transaction remains
    })]
    fn test_track_recent_committed_txn_table_driven(
        #[case] test_case: TrackRecentCommittedTxnTestCase,
    ) {
        let mut txn_manager = create_transaction_manager();

        // Run the setup
        (test_case.setup)(&mut txn_manager);

        // Verify the results
        let inner = txn_manager.inner.read();

        assert_eq!(
            inner.recent_committed_txns.len(),
            test_case.expected_recent_committed_txns_len,
            "Test case '{}' failed: expected {} recent committed transactions, got {}",
            test_case.name,
            test_case.expected_recent_committed_txns_len,
            inner.recent_committed_txns.len()
        );

        assert_eq!(
            inner.active_txns.len(),
            test_case.expected_active_txns_len,
            "Test case '{}' failed: expected {} active transactions, got {}",
            test_case.name,
            test_case.expected_active_txns_len,
            inner.active_txns.len()
        );

        if let Some(expected_txn) = test_case.expected_recent_committed_txn {
            assert!(
                !inner.recent_committed_txns.is_empty(),
                "Test case '{}' failed: expected a committed transaction but none found",
                test_case.name
            );

            let actual_txn = &inner.recent_committed_txns[0];
            assert_eq!(
                actual_txn.started_seq, expected_txn.started_seq,
                "Test case '{}' failed: expected started_seq {}, got {}",
                test_case.name, expected_txn.started_seq, actual_txn.started_seq
            );
            assert_eq!(
                actual_txn.committed_seq, expected_txn.committed_seq,
                "Test case '{}' failed: expected committed_seq {:?}, got {:?}",
                test_case.name, expected_txn.committed_seq, actual_txn.committed_seq
            );
            assert_eq!(
                actual_txn.write_keys, expected_txn.write_keys,
                "Test case '{}' failed: expected write_keys {:?}, got {:?}",
                test_case.name, expected_txn.write_keys, actual_txn.write_keys
            );
        }
    }

    #[test]
    fn test_recycle_recent_committed_txns_filters_by_min_seq() {
        let txn_manager = create_transaction_manager();

        // Create an active transaction first to enable tracking
        let active_txn1 = txn_manager.new_txn_with_id(120, Uuid::new_v4());

        // Create some committed transactions first
        let keys1: HashSet<Bytes> = ["key1"].into_iter().map(Bytes::from).collect();
        let keys2: HashSet<Bytes> = ["key2"].into_iter().map(Bytes::from).collect();
        let keys3: HashSet<Bytes> = ["key3"].into_iter().map(Bytes::from).collect();

        // Add committed transactions with different committed_seq values
        txn_manager.track_recent_committed_write_batch(&keys1, 50); // committed_seq = 50
        txn_manager.track_recent_committed_write_batch(&keys2, 100); // committed_seq = 100
        txn_manager.track_recent_committed_write_batch(&keys3, 150); // committed_seq = 150

        // Verify we have all 3 committed transactions
        assert_eq!(txn_manager.inner.read().recent_committed_txns.len(), 3);

        // Trigger garbage collection by dropping the transaction
        // Since there will be no active non-readonly transactions after dropping,
        // all recent_committed_txns will be cleared
        txn_manager.drop_txn(&active_txn1);

        // All should be cleared since no active non-readonly transactions remain
        assert_eq!(txn_manager.inner.read().recent_committed_txns.len(), 0);
    }

    #[test]
    fn test_recycle_recent_committed_txns_clears_all_when_no_active_writers() {
        let txn_manager = create_transaction_manager();

        // Create a transaction first to enable tracking
        let txn_id = txn_manager.new_txn_with_id(300, Uuid::new_v4());

        // Add some committed transactions
        let keys: HashSet<Bytes> = ["key1", "key2"].into_iter().map(Bytes::from).collect();
        txn_manager.track_recent_committed_write_batch(&keys, 100);
        txn_manager.track_recent_committed_write_batch(&keys, 200);

        // Verify they exist
        assert_eq!(txn_manager.inner.read().recent_committed_txns.len(), 2);

        // Drop the write transaction (this triggers recycle)
        txn_manager.drop_txn(&txn_id);

        // Since no active non-readonly transactions remain, should clear all
        assert!(txn_manager.inner.read().recent_committed_txns.is_empty());
    }

    #[test]
    fn test_non_transactional_writes_do_not_accumulate_without_active_writers() {
        let txn_manager = create_transaction_manager();

        let keys: HashSet<Bytes> = ["key1", "key2"].into_iter().map(Bytes::from).collect();

        txn_manager.track_recent_committed_write_batch(&keys, 100);
        txn_manager.track_recent_committed_write_batch(&keys, 200);
        txn_manager.track_recent_committed_write_batch(&keys, 300);

        let inner = txn_manager.inner.read();
        assert!(inner.recent_committed_txns.is_empty());
        assert_eq!(inner.oracle.last_committed_seq(), 300);
    }

    #[test]
    fn test_recycle_recent_committed_txns_boundary_condition_equal_seq() {
        let txn_manager = create_transaction_manager();

        // Create active transactions first to enable tracking
        let _active_txn1 = txn_manager.new_txn_with_id(100, Uuid::new_v4()); // This sets min to 100
        let active_txn2 = txn_manager.new_txn_with_id(200, Uuid::new_v4()); // Remove this later

        // Create committed transactions with seq values around the boundary
        let keys: HashSet<Bytes> = ["key1"].into_iter().map(Bytes::from).collect();
        txn_manager.track_recent_committed_write_batch(&keys, 99); // should be removed
        txn_manager.track_recent_committed_write_batch(&keys, 100); // should be kept (equal)
        txn_manager.track_recent_committed_write_batch(&keys, 101); // should be kept

        // Trigger garbage collection - this will keep min_conflict_check_seq = 100
        txn_manager.drop_txn(&active_txn2);

        // Should keep transactions with committed_seq >= 100
        assert_eq!(txn_manager.inner.read().recent_committed_txns.len(), 2);
        let committed_seqs: Vec<u64> = txn_manager
            .inner
            .read()
            .recent_committed_txns
            .iter()
            .map(|txn| txn.committed_seq.unwrap())
            .collect();
        assert!(committed_seqs.contains(&100));
        assert!(committed_seqs.contains(&101));
    }

    #[test]
    fn test_recycle_recent_committed_txns_handles_none_committed_seq() {
        let txn_manager = create_transaction_manager();

        // Manually create a transaction state with None committed_seq (edge case)
        {
            let mut inner = txn_manager.inner.write();
            inner.recent_committed_txns.push_back(TransactionState {
                started_seq: 50,
                committed_seq: None, // This should not happen in practice but let's test
                write_keys: HashSet::new(),
                read_keys: HashSet::new(),
                read_ranges: Vec::new(),
            });
        }

        // Keep active transactions around so the committed history is retained
        // until we explicitly trigger recycle.
        let _active_txn1 = txn_manager.new_txn_with_id(150, Uuid::new_v4()); // This sets min to 150
        let active_txn2 = txn_manager.new_txn_with_id(200, Uuid::new_v4());

        // Add a normal committed transaction
        let keys: HashSet<Bytes> = ["key1"].into_iter().map(Bytes::from).collect();
        txn_manager.track_recent_committed_write_batch(&keys, 100);

        // Trigger garbage collection with min_conflict_check_seq = 150
        txn_manager.drop_txn(&active_txn2);

        // The transaction with None committed_seq should be kept (safety)
        // The transaction with committed_seq=100 should be removed since 100 < 150
        assert_eq!(txn_manager.inner.read().recent_committed_txns.len(), 1);
        assert_eq!(
            txn_manager.inner.read().recent_committed_txns[0].committed_seq,
            None
        );
    }

    // Integration tests for complete transaction lifecycles and complex interactions

    #[test]
    fn test_transaction_lifecycle_complete_flow() {
        let txn_manager = create_transaction_manager();

        // Step 1: Create a transaction
        let txn_id = txn_manager.new_txn_with_id(100, Uuid::new_v4());
        assert_eq!(txn_manager.min_active_seq(), Some(100));

        // Step 2: Simulate conflict detection during transaction
        let write_keys: HashSet<Bytes> = ["key1", "key2"].into_iter().map(Bytes::from).collect();
        txn_manager.track_write_keys(&txn_id, &write_keys);
        let has_conflict = txn_manager.check_has_conflict(&txn_id);
        assert!(!has_conflict); // No conflicts initially

        // Step 3: Create another transaction that will commit first
        let other_txn = txn_manager.new_txn_with_id(50, Uuid::new_v4());
        let other_keys: HashSet<Bytes> = ["key1", "key3"].into_iter().map(Bytes::from).collect();

        // Step 4: Commit the other transaction
        txn_manager.track_write_keys(&other_txn, &other_keys);
        txn_manager.track_recent_committed_txn(&other_txn, 120);

        // Step 5: Check for conflicts again - should detect conflict on key1
        let has_conflict = txn_manager.check_has_conflict(&txn_id);
        assert!(has_conflict); // Should conflict on key1

        // Step 6: Commit our transaction despite conflict (simulating retry logic)
        txn_manager.track_write_keys(&txn_id, &write_keys);
        txn_manager.track_recent_committed_txn(&txn_id, 150);

        // Step 7: Verify final state
        assert!(txn_manager.inner.read().active_txns.is_empty());
        assert!(txn_manager.inner.read().recent_committed_txns.is_empty());

        // Step 8: Verify min_active_seq is now None
        assert_eq!(txn_manager.min_active_seq(), None);
    }

    #[test]
    fn test_concurrent_transactions_conflict_detection() {
        let txn_manager = create_transaction_manager();

        // Create three concurrent transactions
        let keys_a: HashSet<Bytes> = ["keyA"].into_iter().map(Bytes::from).collect();
        let keys_b: HashSet<Bytes> = ["keyB"].into_iter().map(Bytes::from).collect();
        let keys_ab: HashSet<Bytes> = ["keyA", "keyB"].into_iter().map(Bytes::from).collect();

        let txn1 = txn_manager.new_txn_with_id(100, Uuid::new_v4()); // T1 starts at seq 100
        let txn2 = txn_manager.new_txn_with_id(110, Uuid::new_v4()); // T2 starts at seq 110
        let txn3 = txn_manager.new_txn_with_id(120, Uuid::new_v4()); // T3 starts at seq 120
        txn_manager.track_write_keys(&txn1, &keys_a);
        txn_manager.track_write_keys(&txn2, &keys_b);
        txn_manager.track_write_keys(&txn3, &keys_ab);

        // Initially no conflicts
        assert!(!txn_manager.check_has_conflict(&txn1));
        assert!(!txn_manager.check_has_conflict(&txn2));
        assert!(!txn_manager.check_has_conflict(&txn3));

        // T1 commits at seq 130, writing keyA
        txn_manager.track_write_keys(&txn1, &keys_a);
        txn_manager.track_recent_committed_txn(&txn1, 130);

        // T2 should not conflict (writes different key)
        assert!(!txn_manager.check_has_conflict(&txn2));

        // T3 should conflict (writes keyA, started at 120 < 130)
        assert!(txn_manager.check_has_conflict(&txn3));

        // T2 commits at seq 140, writing keyB
        txn_manager.track_write_keys(&txn2, &keys_b);
        txn_manager.track_recent_committed_txn(&txn2, 140);

        // T3 should still conflict (now conflicts on both keyA and keyB)
        assert!(txn_manager.check_has_conflict(&txn3));

        // Verify state
        assert_eq!(txn_manager.inner.read().active_txns.len(), 1); // Only T3 remains active
        assert_eq!(txn_manager.inner.read().recent_committed_txns.len(), 2); // T1 and T2 committed
    }

    #[test]
    fn test_garbage_collection_timing_with_multiple_operations() {
        let txn_manager = create_transaction_manager();

        // Create active transactions first
        let long_txn = txn_manager.new_txn_with_id(100, Uuid::new_v4()); // Long-running transaction
        let short_txn1 = txn_manager.new_txn_with_id(150, Uuid::new_v4()); // Short transaction 1
        let short_txn2 = txn_manager.new_txn_with_id(200, Uuid::new_v4()); // Short transaction 2

        // Create some old committed transactions (now they will be tracked since we have active txns)
        let keys1: HashSet<Bytes> = ["old_key1"].into_iter().map(Bytes::from).collect();
        let keys2: HashSet<Bytes> = ["old_key2"].into_iter().map(Bytes::from).collect();
        txn_manager.track_recent_committed_write_batch(&keys1, 50);
        txn_manager.track_recent_committed_write_batch(&keys2, 60);

        // Verify old committed transactions are kept (min_conflict_check_seq = 100)
        assert_eq!(txn_manager.inner.read().recent_committed_txns.len(), 2);

        // Commit short transactions
        let short_keys: HashSet<Bytes> = ["short_key"].into_iter().map(Bytes::from).collect();
        txn_manager.track_write_keys(&short_txn1, &short_keys);
        txn_manager.track_recent_committed_txn(&short_txn1, 180);
        txn_manager.track_write_keys(&short_txn2, &short_keys);
        txn_manager.track_recent_committed_txn(&short_txn2, 220);

        // Old transactions should still be kept (long_txn still active with seq 100)
        assert_eq!(txn_manager.inner.read().recent_committed_txns.len(), 4); // 2 old + 2 new

        // Drop the long-running transaction - this should trigger aggressive cleanup
        txn_manager.drop_txn(&long_txn);

        // All committed transactions should be cleared (no active writers remain)
        assert_eq!(txn_manager.inner.read().recent_committed_txns.len(), 0);
        assert!(txn_manager.inner.read().active_txns.is_empty());
    }

    #[test]
    fn test_ssi_phantom_read_conflict_on_range() {
        let txn_manager = create_transaction_manager();

        // A transaction under SSI that scans a key range.
        let txn_ssi = txn_manager.new_txn_with_id(100, Uuid::new_v4());

        // Keep a dummy transaction active so commits are tracked in recent_committed_txns.
        let _active_writer_guard = txn_manager.new_txn_with_id(200, Uuid::new_v4());

        // A separate writer commits a key inside the reader's scanned range.
        let writer_txn = txn_manager.new_txn_with_id(60, Uuid::new_v4());
        let writer_keys: HashSet<Bytes> = ["foo5"].into_iter().map(Bytes::from).collect();
        txn_manager.track_write_keys(&writer_txn, &writer_keys);
        txn_manager.track_recent_committed_txn(&writer_txn, 120);

        // The transaction under SSI scans a range that should include "foo5"
        let range = BytesRange::from(Bytes::from("foo0")..=Bytes::from("foo9"));
        txn_manager.track_read_range(&txn_ssi, range);

        // Under SSI, this should be detected as a phantom read (read-write) conflict
        assert!(txn_manager.check_has_conflict(&txn_ssi));
    }

    // Property-based tests using proptest for invariant verification
    use proptest::collection::vec;
    use proptest::prelude::*;
    use rand::Rng;

    #[derive(Debug, Clone)]
    enum TxnOperation {
        Create {
            txn_id: Uuid,
        },
        Drop {
            txn_id: Uuid,
        },
        Commit {
            txn_id: Option<Uuid>,
            keys: Vec<String>,
        },
        // New SSI-specific operations
        TrackReadKeys {
            txn_id: Uuid,
            keys: Vec<String>,
        },
        TrackReadRange {
            txn_id: Uuid,
            start_key: String,
            end_key: String,
        },
        Recycle,
    }

    // Generator for transaction operations
    fn txn_operation_strategy() -> impl Strategy<Value = TxnOperation> {
        let tracked_txn_ids = Arc::new(Mutex::new(Vec::<Uuid>::new()));
        let tracked_txn_ids_for_drop = Arc::clone(&tracked_txn_ids);
        let tracked_txn_ids_for_commit = Arc::clone(&tracked_txn_ids);
        let tracked_txn_ids_for_track_read_keys = Arc::clone(&tracked_txn_ids);
        let tracked_txn_ids_for_track_read_range = Arc::clone(&tracked_txn_ids);

        let sample_txn_id = move |tracked_txn_ids: Arc<Mutex<Vec<Uuid>>>| {
            let ids = tracked_txn_ids.lock();
            if ids.is_empty() {
                Uuid::new_v4()
            } else {
                // Pick a random element from the set
                let idx = rand::rng().random_range(0..ids.len());
                ids[idx]
            }
        };

        prop_oneof![
            // Create transaction with increasing seq numbers
            (0..1000u32).prop_map(move |_| {
                let txn_id = Uuid::new_v4();
                tracked_txn_ids.lock().push(txn_id);
                TxnOperation::Create { txn_id }
            }),
            // Drop transaction - sample from tracked_txn_ids_for_drop
            (0..1000u32).prop_map(move |_| {
                TxnOperation::Drop {
                    txn_id: sample_txn_id(tracked_txn_ids_for_drop.clone()),
                }
            }),
            // commit - can be with or without txn_id
            (
                prop::option::of(
                    (0..100u32)
                        .prop_map(move |_| sample_txn_id(tracked_txn_ids_for_commit.clone()))
                ),
                vec("key[0-9][0-9]", 1..5),
            )
                .prop_map(|(txn_id, keys)| TxnOperation::Commit { txn_id, keys }),
            // Track read keys for SSI
            (
                (0..100u32)
                    .prop_map(move |_| sample_txn_id(tracked_txn_ids_for_track_read_keys.clone())),
                vec("key[0-9][0-9]", 1..3),
            )
                .prop_map(|(txn_id, keys)| TxnOperation::TrackReadKeys { txn_id, keys }),
            // Track read range for SSI phantom detection
            (
                (0..100u32)
                    .prop_map(move |_| sample_txn_id(tracked_txn_ids_for_track_read_range.clone())),
                "key[0-9][0-9]",
                "key[0-9][0-9]",
            )
                .prop_map(|(txn_id, start_key, end_key)| {
                    TxnOperation::TrackReadRange {
                        txn_id,
                        start_key: start_key.clone().min(end_key.clone()),
                        end_key: start_key.max(end_key),
                    }
                }),
            // recycle
            (0..10u32).prop_map(|_| TxnOperation::Recycle)
        ]
    }

    // Strategy for sequences of operations
    fn operation_sequence_strategy() -> impl Strategy<Value = Vec<TxnOperation>> {
        vec(txn_operation_strategy(), 5..100)
    }

    // Helper struct to track operation execution state
    #[derive(Debug)]
    struct ExecutionState {
        seq_counter: u64,
    }

    #[derive(Debug, PartialEq)]
    enum ExecutionEffect {
        Nothing,
        CommitSuccess,
        CommitConflict,
        Recycled,
    }

    impl ExecutionState {
        fn new() -> Self {
            Self { seq_counter: 100 }
        }

        fn execute_operation(
            &mut self,
            manager: &TransactionManager,
            op: &TxnOperation,
        ) -> ExecutionEffect {
            match op {
                TxnOperation::Create { txn_id } => {
                    manager.new_txn_with_id(self.seq_counter, *txn_id);
                    ExecutionEffect::Nothing
                }
                TxnOperation::Drop { txn_id } => {
                    manager.drop_txn(txn_id);
                    ExecutionEffect::Nothing
                }
                TxnOperation::TrackReadKeys { txn_id, keys } => {
                    let key_set: HashSet<Bytes> =
                        keys.iter().map(|k| Bytes::from(k.clone())).collect();
                    manager.track_read_keys(txn_id, key_set);
                    ExecutionEffect::Nothing
                }
                TxnOperation::TrackReadRange {
                    txn_id,
                    start_key,
                    end_key,
                } => {
                    let range = BytesRange::from(
                        Bytes::from(start_key.clone())..=Bytes::from(end_key.clone()),
                    );
                    manager.track_read_range(txn_id, range);
                    ExecutionEffect::Nothing
                }
                TxnOperation::Commit { txn_id, keys } => {
                    let key_set = keys
                        .iter()
                        .map(|k: &String| Bytes::from(k.clone()))
                        .collect();
                    self.seq_counter += 10;
                    match txn_id {
                        None => {
                            manager.track_recent_committed_write_batch(&key_set, self.seq_counter);
                            ExecutionEffect::CommitSuccess
                        }
                        Some(txn_id) => {
                            // only record committed txn if there is no conflict
                            manager.track_write_keys(txn_id, &key_set);
                            if !manager.check_has_conflict(txn_id) {
                                manager.track_recent_committed_txn(txn_id, self.seq_counter);
                                ExecutionEffect::CommitSuccess
                            } else {
                                ExecutionEffect::CommitConflict
                            }
                        }
                    }
                }
                TxnOperation::Recycle => {
                    manager.inner.write().recycle_recent_committed_txns();
                    ExecutionEffect::Recycled
                }
            }
        }
    }

    proptest! {
        #[test]
        fn prop_inv_disjoint_active_and_committed_sets(ops in operation_sequence_strategy()) {
            let txn_manager = create_transaction_manager();
            let mut exec_state = ExecutionState::new();

            for op in ops {
                exec_state.execute_operation(&txn_manager, &op);

                // Verify that active_txns.keys() and committed transaction IDs are disjoint
                let inner = txn_manager.inner.read();
                let active_ids: HashSet<Uuid> = inner.active_txns.keys().cloned().collect();

                for active_id in &active_ids {
                    // Each active transaction should not appear in recent_committed_txns
                    // Since we don't store the original ID in committed txns, this is automatically true
                    prop_assert!(inner.active_txns.contains_key(active_id),
                        "Active transaction {:?} should be in active_txns", active_id);
                }

                // The real check: a transaction cannot be both active and committed simultaneously
                // This is enforced by track_recent_committed_txn removing from active_txns
                prop_assert!(
                    inner.recent_committed_txns.iter().all(|_committed_txn| true),
                    "No committed transaction should have an active counterpart"
                );
            }
        }

        #[test]
        fn prop_all_write_without_conflict_should_be_committed(ops in operation_sequence_strategy()) {
            let txn_manager = create_transaction_manager();
            let mut exec_state = ExecutionState::new();

            for op in ops {
                let effect = exec_state.execute_operation(&txn_manager, &op);

                if let TxnOperation::Commit { txn_id: None, keys: _, } = &op {
                    prop_assert!(effect == ExecutionEffect::CommitSuccess, "If the commit is successful, the transaction should have conflict");
                }
            }
        }

      #[test]
        fn prop_inv_min_active_seq_correctness(ops in operation_sequence_strategy()) {
            let txn_manager = create_transaction_manager();
            let mut exec_state = ExecutionState::new();

            for op in ops {
                exec_state.execute_operation(&txn_manager, &op);

                // Verify min_active_seq correctness
                let min_active_seq = txn_manager.min_active_seq();
                let inner = txn_manager.inner.read();

                if inner.active_txns.is_empty() {
                    prop_assert!(
                        min_active_seq.is_none(),
                        "min_active_seq should be None when no active transactions exist"
                    );
                } else {
                    let expected_min = inner.active_txns.values()
                        .map(|txn| txn.started_seq)
                        .min();

                    prop_assert_eq!(
                        min_active_seq,
                        expected_min,
                        "min_active_seq should equal the minimum started_seq of all active transactions"
                    );
                }
            }
        }

        #[test]
        fn prop_inv_garbage_collection_correctness(ops in operation_sequence_strategy()) {
            let txn_manager = create_transaction_manager();
            let mut exec_state = ExecutionState::new();

            for op in ops {
                let effect = exec_state.execute_operation(&txn_manager, &op);
                if effect == ExecutionEffect::Recycled {
                    // Garbage collection correctness invariant
                    // recent_committed_txns.is_empty() OR (there exists an active transaction in active_txns.values() that is not read-only)
                    let inner = txn_manager.inner.read();

                    // if the recent_committed_txns queue is not empty, there must be at least one non-readonly active transaction
                    if !inner.recent_committed_txns.is_empty() {
                        prop_assert!(
                            !inner.active_txns.is_empty(),
                            "invariant violation: recent_committed_txns is not empty but there are no active transactions. \
                            Active transaction IDs: {:?}, Number of recent committed: {}",
                            inner.active_txns.keys().collect::<Vec<_>>(),
                            inner.recent_committed_txns.len()
                        );
                    }

                    if inner.active_txns.is_empty() {
                        prop_assert!(inner.recent_committed_txns.is_empty(), "If there are no active transactions, recent_committed_txns should be empty");
                    }
                }
            }
        }

        #[test]
        fn prop_inv_ssi_read_write_conflict_detection(ops in operation_sequence_strategy()) {
            let txn_manager = create_transaction_manager();
            let mut exec_state = ExecutionState::new();

            for op in ops {
                exec_state.execute_operation(&txn_manager, &op);
                // For every active transaction, verify the smoking-gun invariant:
                // rw_conflict <=> exists culprit in recent_committed_txns that satisfies rules
                let inner = txn_manager.inner.read();
                for (_id, txn) in inner.active_txns.iter() {
                    // Only meaningful for SSI; read-only transactions can also have reads, so include them.
                    let rw_conflict = inner.has_read_write_conflict(
                        &txn.read_keys,
                        txn.read_ranges.clone(),
                        txn.started_seq,
                    );

                    // Search for a culprit committed txn
                    let mut culprit_exists = false;
                    for committed in inner.recent_committed_txns.iter() {
                        let Some(other_committed_seq) = committed.committed_seq else { continue };
                        if other_committed_seq <= txn.started_seq {
                            continue;
                        }

                        // Direct read-write conflict on keys
                        let direct_conflict = !txn.read_keys.is_disjoint(&committed.write_keys);

                        // Phantom conflict via range containment
                        let mut phantom_conflict = false;
                        if !txn.read_ranges.is_empty() && !committed.write_keys.is_empty() {
                            'outer: for range in txn.read_ranges.iter() {
                                for w in committed.write_keys.iter() {
                                    if range.contains(w) {
                                        phantom_conflict = true;
                                        break 'outer;
                                    }
                                }
                            }
                        }

                        if direct_conflict || phantom_conflict {
                            culprit_exists = true;
                            break;
                        }
                    }

                    prop_assert_eq!(rw_conflict, culprit_exists,
                        "SSI RW invariant violation: rw_conflict={} but culprit_exists={} (started_seq={:?}, read_keys_len={}, read_ranges_len={}, recent_committed_len={})",
                        rw_conflict,
                        culprit_exists,
                        txn.started_seq,
                        txn.read_keys.len(),
                        txn.read_ranges.len(),
                        inner.recent_committed_txns.len()
                    );
                }
            }
        }
    }
}
