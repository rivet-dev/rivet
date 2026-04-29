use std::sync::Arc;

use parking_lot::Mutex;
use rivet_pools::NodeId;
use tokio::time::Instant;
use universaldb::Database;

use crate::{compactor::Ups, page_index::DeltaPageIndex};

#[allow(dead_code)]
pub struct ActorDb {
	pub(super) udb: Arc<Database>,
	pub(super) ups: Ups,
	pub(super) actor_id: String,
	pub(super) node_id: NodeId,
	pub(super) cache: Mutex<DeltaPageIndex>,
	/// Cached `/META/storage_used_live`. Loaded once on the first UDB tx.
	pub(super) storage_used_live: Mutex<Option<i64>>,
	/// Cached `/META/storage_used_pitr`. Loaded once alongside live usage.
	pub(super) storage_used_pitr: Mutex<Option<i64>>,
	/// Bytes written across commits since the last metering rollup.
	pub(super) commit_bytes_since_rollup: Mutex<u64>,
	/// Bytes read across `get_pages` calls since the last metering rollup.
	pub(super) read_bytes_since_rollup: Mutex<u64>,
	/// Last time this actor published a compaction trigger.
	pub(super) last_trigger_at: Mutex<Option<Instant>>,
}

impl ActorDb {
	pub fn new(udb: Arc<Database>, ups: Ups, actor_id: String, node_id: NodeId) -> Self {
		#[cfg(debug_assertions)]
		crate::takeover::reconcile_blocking(udb.clone(), actor_id.clone(), node_id);

		Self {
			udb,
			ups,
			actor_id,
			node_id,
			cache: Mutex::new(DeltaPageIndex::new()),
			storage_used_live: Mutex::new(None),
			storage_used_pitr: Mutex::new(None),
			commit_bytes_since_rollup: Mutex::new(0),
			read_bytes_since_rollup: Mutex::new(0),
			last_trigger_at: Mutex::new(None),
		}
	}

	pub fn take_metering_snapshot(&self) -> (u64, u64) {
		let mut commit_bytes = self.commit_bytes_since_rollup.lock();
		let mut read_bytes = self.read_bytes_since_rollup.lock();
		let snapshot = (*commit_bytes, *read_bytes);

		*commit_bytes = 0;
		*read_bytes = 0;

		snapshot
	}
}
