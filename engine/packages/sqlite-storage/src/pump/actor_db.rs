use std::{sync::Arc, time::Instant};

use parking_lot::Mutex;
use universaldb::Database;

use crate::page_index::DeltaPageIndex;

#[allow(dead_code)]
pub struct ActorDb {
	pub(super) udb: Arc<Database>,
	pub(super) actor_id: String,
	pub(super) cache: Mutex<DeltaPageIndex>,
	/// Cached `/META/quota`. Loaded once on the first UDB tx.
	pub(super) storage_used: Mutex<Option<i64>>,
	/// Bytes written across commits since the last metering rollup.
	pub(super) commit_bytes_since_rollup: Mutex<u64>,
	/// Bytes read across `get_pages` calls since the last metering rollup.
	pub(super) read_bytes_since_rollup: Mutex<u64>,
	/// Last time this actor published a compaction trigger.
	pub(super) last_trigger_at: Mutex<Option<Instant>>,
}

impl ActorDb {
	pub fn new(udb: Arc<Database>, actor_id: String) -> Self {
		#[cfg(debug_assertions)]
		crate::takeover::reconcile(&udb, &actor_id);

		Self {
			udb,
			actor_id,
			cache: Mutex::new(DeltaPageIndex::new()),
			storage_used: Mutex::new(None),
			commit_bytes_since_rollup: Mutex::new(0),
			read_bytes_since_rollup: Mutex::new(0),
			last_trigger_at: Mutex::new(None),
		}
	}

}
