//! Engine entry points for sqlite-storage operations.

use std::sync::Arc;
use std::sync::atomic::AtomicUsize;

use anyhow::{Context, Result};
use scc::{HashMap, hash_map::Entry};
use tokio::sync::{Mutex, mpsc};
use universaldb::Subspace;

use crate::keys::{meta_key, pidx_delta_prefix};
use crate::metrics::SqliteStorageMetrics;
use crate::page_index::DeltaPageIndex;
use crate::types::{DBHead, SQLITE_MAX_DELTA_BYTES, SqliteMeta, decode_db_head};
use crate::udb;

pub struct SqliteEngine {
	pub db: universaldb::Database,
	pub subspace: Subspace,
	pub op_counter: Arc<AtomicUsize>,
	pub open_dbs: HashMap<String, OpenDb>,
	pub page_indices: HashMap<String, DeltaPageIndex>,
	pub pending_stages: HashMap<(String, u64), PendingStage>,
	pub actor_op_locks: HashMap<String, Arc<Mutex<()>>>,
	pub compaction_tx: mpsc::UnboundedSender<String>,
	pub metrics: SqliteStorageMetrics,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenDb {
	pub generation: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingStage {
	pub next_chunk_idx: u32,
	pub saw_last_chunk: bool,
	pub error_message: Option<String>,
}

impl SqliteEngine {
	pub fn new(
		db: universaldb::Database,
		subspace: Subspace,
	) -> (Self, mpsc::UnboundedReceiver<String>) {
		let (compaction_tx, compaction_rx) = mpsc::unbounded_channel();
		let engine = Self {
			db,
			subspace,
			op_counter: Arc::new(AtomicUsize::new(0)),
			open_dbs: HashMap::default(),
			page_indices: HashMap::default(),
			pending_stages: HashMap::default(),
			actor_op_locks: HashMap::default(),
			compaction_tx,
			metrics: SqliteStorageMetrics,
		};

		(engine, compaction_rx)
	}

	pub fn metrics(&self) -> &SqliteStorageMetrics {
		&self.metrics
	}

	pub async fn actor_op_lock(&self, actor_id: &str) -> Arc<Mutex<()>> {
		match self.actor_op_locks.entry_async(actor_id.to_string()).await {
			Entry::Occupied(entry) => Arc::clone(entry.get()),
			Entry::Vacant(entry) => Arc::clone(entry.insert_entry(Arc::new(Mutex::new(()))).get()),
		}
	}

	pub async fn load_head(&self, actor_id: &str) -> Result<DBHead> {
		self.try_load_head(actor_id)
			.await?
			.context("sqlite meta missing")
	}

	pub async fn try_load_head(&self, actor_id: &str) -> Result<Option<DBHead>> {
		let meta_bytes = udb::get_value(
			&self.db,
			&self.subspace,
			self.op_counter.as_ref(),
			meta_key(actor_id),
		)
		.await?;

		meta_bytes
			.map(|meta_bytes| decode_db_head(&meta_bytes))
			.transpose()
	}

	pub async fn load_meta(&self, actor_id: &str) -> Result<SqliteMeta> {
		self.try_load_meta(actor_id)
			.await?
			.context("sqlite meta missing")
	}

	pub async fn try_load_meta(&self, actor_id: &str) -> Result<Option<SqliteMeta>> {
		Ok(self
			.try_load_head(actor_id)
			.await?
			.map(|head| SqliteMeta::from((head, SQLITE_MAX_DELTA_BYTES))))
	}

	pub async fn get_or_load_pidx(
		&self,
		actor_id: &str,
	) -> Result<scc::hash_map::OccupiedEntry<'_, String, DeltaPageIndex>> {
		let actor_id = actor_id.to_string();

		match self.page_indices.entry_async(actor_id.clone()).await {
			Entry::Occupied(entry) => Ok(entry),
			Entry::Vacant(entry) => {
				drop(entry);

				let index = DeltaPageIndex::load_from_store(
					&self.db,
					&self.subspace,
					self.op_counter.as_ref(),
					pidx_delta_prefix(&actor_id),
				)
				.await?;

				match self.page_indices.entry_async(actor_id).await {
					Entry::Occupied(entry) => Ok(entry),
					Entry::Vacant(entry) => Ok(entry.insert_entry(index)),
				}
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use anyhow::Result;
	use tokio::sync::mpsc::error::TryRecvError;

	use super::SqliteEngine;
	use crate::keys::{pidx_delta_key, pidx_delta_prefix};
	use crate::test_utils::{
		assert_op_count, clear_op_count, read_value, scan_prefix_values, test_db,
	};

	const TEST_ACTOR: &str = "test-actor";

	#[tokio::test]
	async fn new_returns_compaction_receiver() {
		let (db, subspace) = test_db().await.expect("test db");
		let (engine, mut compaction_rx) = SqliteEngine::new(db, subspace);
		let _ = engine.metrics();

		assert!(matches!(compaction_rx.try_recv(), Err(TryRecvError::Empty)));

		engine
			.compaction_tx
			.send("actor-a".to_string())
			.expect("compaction send should succeed");

		assert_eq!(compaction_rx.recv().await, Some("actor-a".to_string()));
	}

	#[tokio::test]
	async fn get_or_load_pidx_scans_store_once_per_actor() -> Result<()> {
		let (db, subspace) = test_db().await?;
		let (engine, _compaction_rx) = SqliteEngine::new(db, subspace);
		crate::udb::apply_write_ops(
			&engine.db,
			&engine.subspace,
			engine.op_counter.as_ref(),
			vec![
				crate::udb::WriteOp::put(
					pidx_delta_key(TEST_ACTOR, 2),
					20_u64.to_be_bytes().to_vec(),
				),
				crate::udb::WriteOp::put(
					pidx_delta_key(TEST_ACTOR, 9),
					90_u64.to_be_bytes().to_vec(),
				),
			],
		)
		.await?;
		clear_op_count(&engine);

		{
			let actor_a = engine.get_or_load_pidx(TEST_ACTOR).await?;
			assert_eq!(actor_a.get().get(2), Some(20));
			assert_eq!(actor_a.get().get(9), Some(90));
		}

		{
			let actor_a = engine.get_or_load_pidx(TEST_ACTOR).await?;
			assert_eq!(actor_a.get().range(1, 10), vec![(2, 20), (9, 90)]);
		}

		{
			let actor_b = engine.get_or_load_pidx("actor-b").await?;
			assert_eq!(actor_b.get().get(2), None);
		}

		assert_op_count(&engine, 2);
		assert_eq!(
			scan_prefix_values(&engine, pidx_delta_prefix(TEST_ACTOR))
				.await?
				.len(),
			2
		);
		assert_eq!(
			read_value(&engine, pidx_delta_key(TEST_ACTOR, 2)).await?,
			Some(20_u64.to_be_bytes().to_vec())
		);

		Ok(())
	}
}
