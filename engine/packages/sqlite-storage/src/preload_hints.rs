//! Recent-page preload hint persistence for sqlite-storage.

use anyhow::{Context, Result, ensure};

use crate::engine::SqliteEngine;
use crate::error::SqliteStorageError;
use crate::keys::{meta_key, preload_hints_key};
use crate::types::{PreloadHints, decode_db_head, encode_preload_hints};
use crate::udb;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistPreloadHintsRequest {
	pub generation: u64,
	pub hints: PreloadHints,
}

impl SqliteEngine {
	pub async fn persist_preload_hints(
		&self,
		actor_id: &str,
		request: PersistPreloadHintsRequest,
	) -> Result<()> {
		self.ensure_open(actor_id, request.generation, "persist_preload_hints")
			.await?;

		let actor_id = actor_id.to_string();
		let actor_id_for_tx = actor_id.clone();
		let subspace = self.subspace.clone();
		udb::run_db_op(&self.db, self.op_counter.as_ref(), move |tx| {
			let actor_id = actor_id_for_tx.clone();
			let subspace = subspace.clone();
			let request = request.clone();
			async move {
				let meta_storage_key = meta_key(&actor_id);
				let meta_bytes =
					udb::tx_get_value_serializable(&tx, &subspace, &meta_storage_key)
						.await?
						.context("sqlite meta missing")?;
				let head = decode_db_head(&meta_bytes)?;
				ensure!(
					head.generation == request.generation,
					SqliteStorageError::FenceMismatch {
						reason: format!(
							"persist_preload_hints generation {} did not match current generation {}",
							request.generation, head.generation
						),
					},
				);

				let hints_storage_key = preload_hints_key(&actor_id);
				if request.hints.pgnos.is_empty() && request.hints.ranges.is_empty() {
					udb::tx_delete_value_precise(&tx, &subspace, &hints_storage_key).await?;
				} else {
					let encoded = encode_preload_hints(&request.hints)?;
					udb::tx_write_value(&tx, &subspace, &hints_storage_key, &encoded)?;
				}

				Ok(())
			}
		})
		.await
	}
}

#[cfg(test)]
mod tests {
	use anyhow::Result;

	use super::PersistPreloadHintsRequest;
	use crate::engine::SqliteEngine;
	use crate::error::SqliteStorageError;
	use crate::keys::{meta_key, preload_hints_key, shard_key};
	use crate::open::OpenConfig;
	use crate::test_utils::{read_value, test_db};
	use crate::types::{
		PreloadHintRange, PreloadHints, SQLITE_PAGE_SIZE, decode_preload_hints, encode_db_head,
		new_db_head,
	};
	use crate::udb::{WriteOp, apply_write_ops};

	const TEST_ACTOR: &str = "test-actor";

	#[tokio::test]
	async fn persist_preload_hints_writes_separate_v2_key() -> Result<()> {
		let (db, subspace) = test_db().await?;
		let (engine, _compaction_rx) = SqliteEngine::new(db, subspace);
		engine.open(TEST_ACTOR, OpenConfig::new(1)).await?;
		let hints = PreloadHints {
			pgnos: vec![1, 3, 5],
			ranges: vec![PreloadHintRange {
				start_pgno: 64,
				page_count: 16,
			}],
		};

		engine
			.persist_preload_hints(
				TEST_ACTOR,
				PersistPreloadHintsRequest {
					generation: 1,
					hints: hints.clone(),
				},
			)
			.await?;

		let stored = read_value(&engine, preload_hints_key(TEST_ACTOR))
			.await?
			.expect("preload hints should be persisted");
		assert_eq!(decode_preload_hints(&stored)?, hints);
		assert!(
			read_value(&engine, shard_key(TEST_ACTOR, 0))
				.await?
				.is_none(),
			"hint persistence should not write normal page data"
		);

		Ok(())
	}

	#[tokio::test]
	async fn persist_preload_hints_is_generation_fenced() -> Result<()> {
		let (db, subspace) = test_db().await?;
		let (engine, _compaction_rx) = SqliteEngine::new(db, subspace);
		let mut head = new_db_head(1);
		head.generation = 7;
		head.db_size_pages = 1;
		head.page_size = SQLITE_PAGE_SIZE;
		apply_write_ops(
			&engine.db,
			&engine.subspace,
			engine.op_counter.as_ref(),
			vec![WriteOp::put(meta_key(TEST_ACTOR), encode_db_head(&head)?)],
		)
		.await?;
		engine.open(TEST_ACTOR, OpenConfig::new(1)).await?;

		let error = engine
			.persist_preload_hints(
				TEST_ACTOR,
				PersistPreloadHintsRequest {
					generation: 6,
					hints: PreloadHints {
						pgnos: vec![1],
						ranges: vec![],
					},
				},
			)
			.await
			.expect_err("stale generation should be rejected");

		assert!(matches!(
			error.downcast_ref::<SqliteStorageError>(),
			Some(SqliteStorageError::FenceMismatch { .. })
		));
		assert!(
			read_value(&engine, preload_hints_key(TEST_ACTOR))
				.await?
				.is_none()
		);

		Ok(())
	}
}
