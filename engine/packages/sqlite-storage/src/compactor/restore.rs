//! PITR restore implementation for sqlite admin operations.

use std::{
	collections::{BTreeMap, BTreeSet},
	sync::Arc,
	time::{Instant, SystemTime},
};

use anyhow::{Context, Result, bail};
use futures_util::TryStreamExt;
use rivet_pools::NodeId;
use tokio_util::sync::CancellationToken;
use universaldb::{
	RangeOption,
	options::StreamingMode,
	utils::IsolationLevel::{Serializable, Snapshot},
};
use uuid::Uuid;

use crate::{
	admin::{
		self, OpResult, OpStatus, RestoreMode, RestoreTarget, SqliteAdminError,
		decode_admin_op_record, encode_admin_op_record,
	},
	pump::{
		keys::{self, SHARD_SIZE},
		ltx::decode_ltx_v3,
		quota,
		types::{
			Checkpoints, DBHead, RestoreMarker, RestoreStep, decode_checkpoint_meta,
			decode_checkpoints, decode_db_head, decode_delta_meta, decode_retention_config,
			encode_checkpoint_meta, encode_checkpoints, encode_db_head, encode_restore_marker,
		},
	},
};

use super::{TakeOutcome, fold_shard, lease, metrics};

const RESTORE_COPY_BATCH_ROWS: usize = 75;
const RESTORE_LEASE_TTL_MS: u64 = 30_000;

pub async fn handle_restore(
	udb: Arc<universaldb::Database>,
	op_id: Uuid,
	actor_id: String,
	target: RestoreTarget,
	mode: RestoreMode,
	holder_id: NodeId,
	cancel_token: CancellationToken,
) -> Result<()> {
	let started_at = Instant::now();
	let result = handle_restore_inner(
		Arc::clone(&udb),
		op_id,
		actor_id.clone(),
		target,
		mode,
		holder_id,
		cancel_token,
	)
	.await;

	match result {
		Ok(()) => {
			metrics::SQLITE_RESTORE_DURATION_SECONDS
				.with_label_values(&["success"])
				.observe(started_at.elapsed().as_secs_f64());
			Ok(())
		}
		Err(err) => {
			metrics::SQLITE_RESTORE_DURATION_SECONDS
				.with_label_values(&["failed"])
				.observe(started_at.elapsed().as_secs_f64());
			let rivet_error = rivet_error::RivetError::extract(&err);
			let _ = admin::fail(
				Arc::clone(&udb),
				op_id,
				OpResult::Message {
					message: format!("{}.{}", rivet_error.group(), rivet_error.code()),
				},
			)
			.await;
			let _ = release_lease(Arc::clone(&udb), actor_id, holder_id).await;
			Err(err)
		}
	}
}

async fn handle_restore_inner(
	udb: Arc<universaldb::Database>,
	op_id: Uuid,
	actor_id: String,
	target: RestoreTarget,
	mode: RestoreMode,
	holder_id: NodeId,
	cancel_token: CancellationToken,
) -> Result<()> {
	ensure_not_cancelled(&cancel_token)?;
	take_lease(Arc::clone(&udb), actor_id.clone(), holder_id).await?;
	admin::update_status(Arc::clone(&udb), op_id, OpStatus::InProgress, Some(holder_id)).await?;

	let existing_marker = read_restore_marker(Arc::clone(&udb), actor_id.clone()).await?;
	let mut pinned_on_resume = false;
	let plan = if let Some(marker) = existing_marker {
		pin_checkpoint_refcount(Arc::clone(&udb), actor_id.clone(), marker.ckp_txid, 1).await?;
		pinned_on_resume = true;
		load_plan_from_marker(Arc::clone(&udb), actor_id.clone(), marker).await?
	} else {
		match load_restore_plan(Arc::clone(&udb), actor_id.clone(), target).await {
			Ok(plan) => plan,
			Err(err) => {
				let rivet_error = rivet_error::RivetError::extract(&err);
				admin::fail(
					Arc::clone(&udb),
					op_id,
					OpResult::Message {
						message: format!("{}.{}", rivet_error.group(), rivet_error.code()),
					},
				)
				.await?;
				release_lease(Arc::clone(&udb), actor_id, holder_id).await?;
				return Ok(());
			}
		}
	};

	if mode == RestoreMode::DryRun {
		admin::complete(
			Arc::clone(&udb),
			op_id,
			OpResult::Message {
				message: format!(
					"restore dry run: target_txid={}, ckp_txid={}, deltas_replayed={}",
					plan.target_txid,
					plan.ckp_txid,
					plan.delta_txids.len()
				),
			},
		)
		.await?;
		release_lease(Arc::clone(&udb), actor_id, holder_id).await?;
		return Ok(());
	}

	match plan.last_completed_step {
		None => {
			write_restore_marker_and_clear_live_state(
				Arc::clone(&udb),
				actor_id.clone(),
				op_id,
				&plan,
				holder_id,
			)
			.await?;
			test_hooks::maybe_pause_after_marker_clear(&actor_id).await;
			copy_checkpoint_rows(Arc::clone(&udb), actor_id.clone(), &plan, &cancel_token).await?;
		}
		Some(RestoreStep::Started) => {
			copy_checkpoint_rows(Arc::clone(&udb), actor_id.clone(), &plan, &cancel_token).await?;
		}
		Some(RestoreStep::CheckpointCopied) => {}
		Some(RestoreStep::DeltasReplayed | RestoreStep::MetaWritten) => {}
	}

	if !matches!(
		plan.last_completed_step,
		Some(RestoreStep::DeltasReplayed | RestoreStep::MetaWritten)
	) {
		replay_deltas(Arc::clone(&udb), actor_id.clone(), &plan, &cancel_token).await?;
	}

	clear_later_deltas_and_recompute_quota(Arc::clone(&udb), actor_id.clone(), &plan).await?;
	complete_restore(Arc::clone(&udb), actor_id.clone(), op_id, &plan).await?;
	if pinned_on_resume {
		pin_checkpoint_refcount(Arc::clone(&udb), actor_id.clone(), plan.ckp_txid, -1).await?;
	}
	release_lease(Arc::clone(&udb), actor_id, holder_id).await?;
	metrics::SQLITE_RESTORE_DELTAS_REPLAYED.observe(plan.delta_txids.len() as f64);

	Ok(())
}

async fn take_lease(
	udb: Arc<universaldb::Database>,
	actor_id: String,
	holder_id: NodeId,
) -> Result<()> {
	let now_ms = now_ms()?;
	let outcome = udb
		.run(move |tx| {
			let actor_id = actor_id.clone();
			async move { lease::take(&tx, &actor_id, holder_id, RESTORE_LEASE_TTL_MS, now_ms).await }
		})
		.await?;
	match outcome {
		TakeOutcome::Acquired => Ok(()),
		TakeOutcome::Skip => bail!("sqlite restore could not take compactor lease"),
	}
}

async fn release_lease(
	udb: Arc<universaldb::Database>,
	actor_id: String,
	holder_id: NodeId,
) -> Result<()> {
	udb.run(move |tx| {
		let actor_id = actor_id.clone();
		async move { lease::release(&tx, &actor_id, holder_id).await }
	})
	.await
}

async fn load_restore_plan(
	udb: Arc<universaldb::Database>,
	actor_id: String,
	target: RestoreTarget,
) -> Result<RestorePlan> {
	udb.run(move |tx| {
		let actor_id = actor_id.clone();
		async move {
			let head = read_head(&tx, &actor_id).await?;
			let checkpoints = read_checkpoints(&tx, &actor_id).await?;
			let _retention = tx_get_value(&tx, &keys::meta_retention_key(&actor_id), Snapshot)
				.await?
				.as_deref()
				.map(decode_retention_config)
				.transpose()
				.context("decode sqlite retention config for restore")?;
			let delta_metas = load_delta_metas(&tx, &actor_id).await?;
			let target_txid = resolve_target(target, &head, &checkpoints, &delta_metas)?;
			let ckp = checkpoints
				.entries
				.iter()
				.filter(|entry| entry.ckp_txid <= target_txid)
				.max_by_key(|entry| entry.ckp_txid)
				.cloned()
				.ok_or_else(|| invalid_restore_point(target_txid, &checkpoints))?;
			validate_reachable(target_txid, ckp.ckp_txid, &delta_metas, &checkpoints)?;
			let checkpoint_meta = read_checkpoint_meta(&tx, &actor_id, ckp.ckp_txid).await?;
			let delta_txids = ((ckp.ckp_txid + 1)..=target_txid).collect::<Vec<_>>();

			Ok(RestorePlan {
				old_head_txid: head.head_txid,
				target_txid,
				ckp_txid: ckp.ckp_txid,
				checkpoint_db_size_pages: checkpoint_meta.db_size_pages,
				delta_txids,
				last_completed_step: None,
				started_at_ms: now_ms()?,
			})
		}
	})
	.await
}

async fn load_plan_from_marker(
	udb: Arc<universaldb::Database>,
	actor_id: String,
	marker: RestoreMarker,
) -> Result<RestorePlan> {
	udb.run(move |tx| {
		let actor_id = actor_id.clone();
		let marker = marker.clone();
		async move {
			let head = read_head(&tx, &actor_id).await?;
			let checkpoint_meta = read_checkpoint_meta(&tx, &actor_id, marker.ckp_txid).await?;
			let delta_metas = load_delta_metas(&tx, &actor_id).await?;
			let checkpoints = read_checkpoints(&tx, &actor_id).await?;
			validate_reachable(
				marker.target_txid,
				marker.ckp_txid,
				&delta_metas,
				&checkpoints,
			)?;
			let delta_txids = ((marker.ckp_txid + 1)..=marker.target_txid).collect::<Vec<_>>();

			Ok(RestorePlan {
				old_head_txid: head.head_txid.max(marker.target_txid),
				target_txid: marker.target_txid,
				ckp_txid: marker.ckp_txid,
				checkpoint_db_size_pages: checkpoint_meta.db_size_pages,
				delta_txids,
				last_completed_step: Some(marker.last_completed_step),
				started_at_ms: marker.started_at_ms,
			})
		}
	})
	.await
}

async fn write_restore_marker_and_clear_live_state(
	udb: Arc<universaldb::Database>,
	actor_id: String,
	op_id: Uuid,
	plan: &RestorePlan,
	holder_id: NodeId,
) -> Result<()> {
	let marker = RestoreMarker {
		target_txid: plan.target_txid,
		ckp_txid: plan.ckp_txid,
		started_at_ms: plan.started_at_ms,
		last_completed_step: RestoreStep::Started,
		holder_id,
		op_id,
	};
	let encoded_marker = encode_restore_marker(marker).context("encode sqlite restore marker")?;
	udb.run(move |tx| {
		let actor_id = actor_id.clone();
		let encoded_marker = encoded_marker.clone();
		async move {
			tx.informal()
				.set(&keys::meta_restore_in_progress_key(&actor_id), &encoded_marker);
			clear_prefix(&tx, &keys::shard_prefix(&actor_id));
			clear_prefix(&tx, &keys::pidx_delta_prefix(&actor_id));
			Ok(())
		}
	})
	.await?;
	metrics::SQLITE_RESTORE_IN_PROGRESS_ACTIVE.inc();
	Ok(())
}

async fn copy_checkpoint_rows(
	udb: Arc<universaldb::Database>,
	actor_id: String,
	plan: &RestorePlan,
	cancel_token: &CancellationToken,
) -> Result<()> {
	let rows = load_checkpoint_copy_rows(Arc::clone(&udb), actor_id.clone(), plan.ckp_txid).await?;
	for chunk in rows.chunks(RESTORE_COPY_BATCH_ROWS) {
		ensure_not_cancelled(cancel_token)?;
		let rows = chunk.to_vec();
		udb.run(move |tx| {
			let rows = rows.clone();
			async move {
				for row in rows {
					tx.informal().set(&row.dst_key, &row.value);
				}
				Ok(())
			}
		})
		.await?;
	}
	update_restore_marker_step(
		Arc::clone(&udb),
		actor_id.clone(),
		RestoreStep::CheckpointCopied,
	)
	.await?;
	write_head(
		Arc::clone(&udb),
		actor_id,
		plan.ckp_txid,
		plan.checkpoint_db_size_pages,
	)
	.await
}

async fn replay_deltas(
	udb: Arc<universaldb::Database>,
	actor_id: String,
	plan: &RestorePlan,
	cancel_token: &CancellationToken,
) -> Result<()> {
	for txid in &plan.delta_txids {
		ensure_not_cancelled(cancel_token)?;
		let delta = load_delta_blob(Arc::clone(&udb), actor_id.clone(), *txid).await?;
		let decoded = decode_ltx_v3(&delta).with_context(|| format!("decode restore delta {txid}"))?;
		let db_size_pages = decoded.header.commit;
		let mut shard_ids = BTreeSet::new();
		for page in &decoded.pages {
			shard_ids.insert(page.pgno / SHARD_SIZE);
		}
		let pages = decoded.pages.clone();
		let shard_ids_for_tx = shard_ids.clone();
		udb.run({
			let actor_id = actor_id.clone();
			move |tx| {
				let actor_id = actor_id.clone();
				let pages = pages.clone();
				let shard_ids = shard_ids_for_tx.clone();
				async move {
					for shard_id in shard_ids {
						let page_updates = pages
							.iter()
							.filter(|page| page.pgno / SHARD_SIZE == shard_id)
							.map(|page| (page.pgno, page.bytes.clone()))
							.collect::<Vec<_>>();
						fold_shard(&tx, &actor_id, shard_id, page_updates).await?;
					}
					let txid_bytes = txid.to_be_bytes();
					for page in &pages {
						tx.informal()
							.set(&keys::pidx_delta_key(&actor_id, page.pgno), &txid_bytes);
					}
					tx.informal().set(
						&keys::meta_head_key(&actor_id),
						&encode_db_head(DBHead {
							head_txid: *txid,
							db_size_pages,
							#[cfg(debug_assertions)]
							generation: 0,
						})?,
					);
					Ok(())
				}
			}
		})
		.await?;
	}

	if plan.delta_txids.is_empty() {
		write_head(
			Arc::clone(&udb),
			actor_id.clone(),
			plan.target_txid,
			plan.checkpoint_db_size_pages,
		)
		.await?;
	}

	update_restore_marker_step(Arc::clone(&udb), actor_id, RestoreStep::DeltasReplayed).await
}

async fn clear_later_deltas_and_recompute_quota(
	udb: Arc<universaldb::Database>,
	actor_id: String,
	plan: &RestorePlan,
) -> Result<()> {
	udb.run({
		let actor_id = actor_id.clone();
		move |tx| {
			let actor_id = actor_id.clone();
			async move {
				for txid in (plan.target_txid + 1)..=plan.old_head_txid {
					clear_prefix(&tx, &keys::delta_chunk_prefix(&actor_id, txid));
				}
				Ok(())
			}
		}
	})
	.await?;

	udb.run(move |tx| {
		let actor_id = actor_id.clone();
		async move {
			let recomputed = scan_live_storage_bytes(&tx, &actor_id).await?;
			let current = quota::read_live(&tx, &actor_id).await?;
			let delta = recomputed
				.checked_sub(current)
				.context("sqlite restore live quota delta overflowed")?;
			if delta != 0 {
				quota::atomic_add_live(&tx, &actor_id, delta);
			}
			Ok(())
		}
	})
	.await
}

async fn complete_restore(
	udb: Arc<universaldb::Database>,
	actor_id: String,
	op_id: Uuid,
	plan: &RestorePlan,
) -> Result<()> {
	let result = OpResult::Message {
		message: format!(
			"restored_to_txid={}, deltas_replayed={}",
			plan.target_txid,
			plan.delta_txids.len()
		),
	};
	let now_ms = now_ms()?;
	udb.run(move |tx| {
		let actor_id = actor_id.clone();
		let result = result.clone();
		async move {
			tx.informal()
				.clear(&keys::meta_restore_in_progress_key(&actor_id));
			let record_key = keys::meta_admin_op_key(&actor_id, op_id);
			let record_bytes = tx
				.informal()
				.get(&record_key, Serializable)
				.await?
				.context("sqlite restore admin op record missing")?;
			let mut record =
				decode_admin_op_record(&record_bytes).context("decode sqlite admin op record")?;
			record.status = OpStatus::Completed;
			record.holder_id = None;
			record.result = Some(result);
			record.last_progress_at_ms = now_ms.max(record.last_progress_at_ms.saturating_add(1));
			tx.informal().set(
				&record_key,
				&encode_admin_op_record(record).context("encode sqlite admin op record")?,
			);
			Ok(())
		}
	})
	.await?;
	metrics::SQLITE_RESTORE_IN_PROGRESS_ACTIVE.dec();
	Ok(())
}

async fn pin_checkpoint_refcount(
	udb: Arc<universaldb::Database>,
	actor_id: String,
	ckp_txid: u64,
	delta: i32,
) -> Result<()> {
	udb.run(move |tx| {
		let actor_id = actor_id.clone();
		async move {
			let meta_key = keys::checkpoint_meta_key(&actor_id, ckp_txid);
			let Some(meta_bytes) = tx.informal().get(&meta_key, Serializable).await? else {
				return Ok(());
			};
			let mut meta = decode_checkpoint_meta(&meta_bytes)?;
			meta.refcount = apply_refcount_delta(meta.refcount, delta)?;
			tx.informal()
				.set(&meta_key, &encode_checkpoint_meta(meta).context("encode checkpoint meta")?);

			let checkpoints_key = keys::meta_checkpoints_key(&actor_id);
			if let Some(checkpoints_bytes) = tx.informal().get(&checkpoints_key, Serializable).await?
			{
				let mut checkpoints = decode_checkpoints(&checkpoints_bytes)?;
				for entry in checkpoints
					.entries
					.iter_mut()
					.filter(|entry| entry.ckp_txid == ckp_txid)
				{
					entry.refcount = apply_refcount_delta(entry.refcount, delta)?;
				}
				tx.informal().set(
					&checkpoints_key,
					&encode_checkpoints(checkpoints).context("encode checkpoints")?,
				);
			}
			Ok(())
		}
	})
	.await
}

fn apply_refcount_delta(current: u32, delta: i32) -> Result<u32> {
	if delta >= 0 {
		current
			.checked_add(delta as u32)
			.context("sqlite checkpoint refcount overflowed")
	} else {
		current
			.checked_sub(delta.unsigned_abs())
			.context("sqlite checkpoint refcount underflowed")
	}
}

async fn read_restore_marker(
	udb: Arc<universaldb::Database>,
	actor_id: String,
) -> Result<Option<RestoreMarker>> {
	udb.run(move |tx| {
		let actor_id = actor_id.clone();
		async move {
			Ok(tx_get_value(&tx, &keys::meta_restore_in_progress_key(&actor_id), Snapshot)
				.await?
				.as_deref()
				.map(crate::pump::types::decode_restore_marker)
				.transpose()?)
		}
	})
	.await
}

async fn read_head(tx: &universaldb::Transaction, actor_id: &str) -> Result<DBHead> {
	let head_bytes = tx_get_value(tx, &keys::meta_head_key(actor_id), Snapshot)
		.await?
		.context("sqlite restore requires db head")?;
	decode_db_head(&head_bytes).context("decode sqlite restore db head")
}

async fn read_checkpoints(tx: &universaldb::Transaction, actor_id: &str) -> Result<Checkpoints> {
	Ok(tx_get_value(tx, &keys::meta_checkpoints_key(actor_id), Snapshot)
		.await?
		.as_deref()
		.map(decode_checkpoints)
		.transpose()
		.context("decode sqlite checkpoints for restore")?
		.unwrap_or(Checkpoints {
			entries: Vec::new(),
		}))
}

async fn read_checkpoint_meta(
	tx: &universaldb::Transaction,
	actor_id: &str,
	ckp_txid: u64,
) -> Result<crate::pump::types::CheckpointMeta> {
	let meta_bytes = tx_get_value(tx, &keys::checkpoint_meta_key(actor_id, ckp_txid), Snapshot)
		.await?
		.with_context(|| format!("sqlite checkpoint {ckp_txid} missing meta"))?;
	decode_checkpoint_meta(&meta_bytes).context("decode sqlite checkpoint meta")
}

async fn load_delta_metas(
	tx: &universaldb::Transaction,
	actor_id: &str,
) -> Result<BTreeMap<u64, i64>> {
	let mut metas = BTreeMap::new();
	for (key, value) in scan_prefix_values(tx, &keys::delta_prefix(actor_id), Snapshot).await? {
		let txid = keys::decode_delta_chunk_txid(actor_id, &key)?;
		if key == keys::delta_meta_key(actor_id, txid) {
			let meta = decode_delta_meta(&value).context("decode sqlite restore delta meta")?;
			metas.insert(txid, meta.taken_at_ms);
		}
	}
	Ok(metas)
}

fn resolve_target(
	target: RestoreTarget,
	head: &DBHead,
	checkpoints: &Checkpoints,
	delta_metas: &BTreeMap<u64, i64>,
) -> Result<u64> {
	let target_txid = match target {
		RestoreTarget::Txid(txid) => txid,
		RestoreTarget::LatestCheckpoint => checkpoints
			.entries
			.iter()
			.map(|entry| entry.ckp_txid)
			.max()
			.ok_or_else(|| invalid_restore_point(0, checkpoints))?,
		RestoreTarget::CheckpointTxid(txid) => {
			if !checkpoints.entries.iter().any(|entry| entry.ckp_txid == txid) {
				return Err(invalid_restore_point(txid, checkpoints));
			}
			txid
		}
		RestoreTarget::TimestampMs(timestamp_ms) => checkpoints
			.entries
			.iter()
			.filter(|entry| entry.taken_at_ms <= timestamp_ms)
			.map(|entry| entry.ckp_txid)
			.chain(
				delta_metas
					.iter()
					.filter_map(|(txid, taken_at_ms)| (*taken_at_ms <= timestamp_ms).then_some(*txid)),
			)
			.max()
			.ok_or_else(|| invalid_restore_point(0, checkpoints))?,
	};
	if target_txid > head.head_txid {
		return Err(invalid_restore_point(target_txid, checkpoints));
	}
	Ok(target_txid)
}

fn validate_reachable(
	target_txid: u64,
	ckp_txid: u64,
	delta_metas: &BTreeMap<u64, i64>,
	checkpoints: &Checkpoints,
) -> Result<()> {
	for txid in (ckp_txid + 1)..=target_txid {
		if !delta_metas.contains_key(&txid) {
			return Err(invalid_restore_point(target_txid, checkpoints));
		}
	}
	Ok(())
}

fn invalid_restore_point(target_txid: u64, checkpoints: &Checkpoints) -> anyhow::Error {
	SqliteAdminError::InvalidRestorePoint {
		target_txid,
		reachable_hints: checkpoints
			.entries
			.iter()
			.map(|entry| entry.ckp_txid)
			.collect(),
	}
	.build()
}

async fn load_checkpoint_copy_rows(
	udb: Arc<universaldb::Database>,
	actor_id: String,
	ckp_txid: u64,
) -> Result<Vec<CopyRow>> {
	udb.run(move |tx| {
		let actor_id = actor_id.clone();
		async move {
			let mut rows = Vec::new();
			for (key, value) in scan_prefix_values(
				&tx,
				&checkpoint_shard_prefix(&actor_id, ckp_txid),
				Snapshot,
			)
			.await?
			{
				let shard_id = decode_checkpoint_suffix_u32(&key)?;
				rows.push(CopyRow {
					dst_key: keys::shard_key(&actor_id, shard_id),
					value,
				});
			}
			for (key, value) in scan_prefix_values(
				&tx,
				&checkpoint_pidx_prefix(&actor_id, ckp_txid),
				Snapshot,
			)
			.await?
			{
				let pgno = decode_checkpoint_suffix_u32(&key)?;
				rows.push(CopyRow {
					dst_key: keys::pidx_delta_key(&actor_id, pgno),
					value,
				});
			}
			Ok(rows)
		}
	})
	.await
}

async fn load_delta_blob(
	udb: Arc<universaldb::Database>,
	actor_id: String,
	txid: u64,
) -> Result<Vec<u8>> {
	udb.run(move |tx| {
		let actor_id = actor_id.clone();
		async move {
			let mut chunks = Vec::new();
			for (key, value) in
				scan_prefix_values(&tx, &keys::delta_chunk_prefix(&actor_id, txid), Snapshot).await?
			{
				if key == keys::delta_meta_key(&actor_id, txid) {
					continue;
				}
				let idx = keys::decode_delta_chunk_idx(&actor_id, txid, &key)?;
				chunks.push((idx, value));
			}
			if chunks.is_empty() {
				bail!("sqlite restore missing delta {txid}");
			}
			chunks.sort_by_key(|(idx, _)| *idx);
			Ok(chunks
				.into_iter()
				.flat_map(|(_, value)| value)
				.collect::<Vec<_>>())
		}
	})
	.await
}

async fn write_head(
	udb: Arc<universaldb::Database>,
	actor_id: String,
	head_txid: u64,
	db_size_pages: u32,
) -> Result<()> {
	udb.run(move |tx| {
		let actor_id = actor_id.clone();
		async move {
			tx.informal().set(
				&keys::meta_head_key(&actor_id),
				&encode_db_head(DBHead {
					head_txid,
					db_size_pages,
					#[cfg(debug_assertions)]
					generation: 0,
				})?,
			);
			Ok(())
		}
	})
	.await
}

async fn update_restore_marker_step(
	udb: Arc<universaldb::Database>,
	actor_id: String,
	step: RestoreStep,
) -> Result<()> {
	udb.run(move |tx| {
		let actor_id = actor_id.clone();
		async move {
			let marker_key = keys::meta_restore_in_progress_key(&actor_id);
			let Some(marker_bytes) = tx.informal().get(&marker_key, Serializable).await? else {
				return Ok(());
			};
			let mut marker = crate::pump::types::decode_restore_marker(&marker_bytes)?;
			marker.last_completed_step = step;
			tx.informal()
				.set(&marker_key, &encode_restore_marker(marker)?);
			Ok(())
		}
	})
	.await
}

async fn scan_live_storage_bytes(
	tx: &universaldb::Transaction,
	actor_id: &str,
) -> Result<i64> {
	let mut total = 0;
	if let Some(head) = tx_get_value(tx, &keys::meta_head_key(actor_id), Snapshot).await? {
		total += tracked_entry_size(&keys::meta_head_key(actor_id), &head)?;
	}
	total += scan_tracked_prefix_bytes(tx, &keys::shard_prefix(actor_id)).await?;
	total += scan_tracked_prefix_bytes(tx, &keys::pidx_delta_prefix(actor_id)).await?;
	total += scan_tracked_prefix_bytes(tx, &keys::delta_prefix(actor_id)).await?;
	Ok(total)
}

async fn scan_tracked_prefix_bytes(tx: &universaldb::Transaction, prefix: &[u8]) -> Result<i64> {
	scan_prefix_values(tx, prefix, Snapshot)
		.await?
		.iter()
		.map(|(key, value)| tracked_entry_size(key, value))
		.sum()
}

fn tracked_entry_size(key: &[u8], value: &[u8]) -> Result<i64> {
	i64::try_from(key.len() + value.len()).context("sqlite tracked entry size exceeded i64")
}

async fn tx_get_value(
	tx: &universaldb::Transaction,
	key: &[u8],
	isolation_level: universaldb::utils::IsolationLevel,
) -> Result<Option<Vec<u8>>> {
	Ok(tx
		.informal()
		.get(key, isolation_level)
		.await?
		.map(Vec::<u8>::from))
}

async fn scan_prefix_values(
	tx: &universaldb::Transaction,
	prefix: &[u8],
	isolation_level: universaldb::utils::IsolationLevel,
) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
	let informal = tx.informal();
	let prefix_subspace =
		universaldb::Subspace::from(universaldb::tuple::Subspace::from_bytes(prefix.to_vec()));
	let mut stream = informal.get_ranges_keyvalues(
		RangeOption {
			mode: StreamingMode::WantAll,
			..RangeOption::from(&prefix_subspace)
		},
		isolation_level,
	);
	let mut rows = Vec::new();

	while let Some(entry) = stream.try_next().await? {
		rows.push((entry.key().to_vec(), entry.value().to_vec()));
	}

	Ok(rows)
}

fn clear_prefix(tx: &universaldb::Transaction, prefix: &[u8]) {
	let (begin, end) = prefix_range(prefix);
	tx.informal().clear_range(&begin, &end);
}

fn prefix_range(prefix: &[u8]) -> (Vec<u8>, Vec<u8>) {
	universaldb::tuple::Subspace::from_bytes(prefix.to_vec()).range()
}

fn checkpoint_shard_prefix(actor_id: &str, ckp_txid: u64) -> Vec<u8> {
	let mut prefix = keys::checkpoint_prefix(actor_id, ckp_txid);
	prefix.extend_from_slice(b"SHARD/");
	prefix
}

fn checkpoint_pidx_prefix(actor_id: &str, ckp_txid: u64) -> Vec<u8> {
	let mut prefix = keys::checkpoint_prefix(actor_id, ckp_txid);
	prefix.extend_from_slice(b"PIDX/delta/");
	prefix
}

fn decode_checkpoint_suffix_u32(key: &[u8]) -> Result<u32> {
	let suffix = key
		.get(key.len().saturating_sub(std::mem::size_of::<u32>())..)
		.context("checkpoint key suffix missing u32")?;
	Ok(u32::from_be_bytes(
		suffix
			.try_into()
			.map_err(|_| anyhow::anyhow!("checkpoint key suffix had invalid length"))?,
	))
}

fn ensure_not_cancelled(cancel_token: &CancellationToken) -> Result<()> {
	if cancel_token.is_cancelled() {
		bail!("sqlite restore cancelled");
	}
	Ok(())
}

fn now_ms() -> Result<i64> {
	let elapsed = SystemTime::now()
		.duration_since(SystemTime::UNIX_EPOCH)
		.context("system clock was before unix epoch")?;
	i64::try_from(elapsed.as_millis()).context("sqlite restore timestamp exceeded i64")
}

#[derive(Debug, Clone)]
struct RestorePlan {
	old_head_txid: u64,
	target_txid: u64,
	ckp_txid: u64,
	checkpoint_db_size_pages: u32,
	delta_txids: Vec<u64>,
	last_completed_step: Option<RestoreStep>,
	started_at_ms: i64,
}

#[derive(Debug, Clone)]
struct CopyRow {
	dst_key: Vec<u8>,
	value: Vec<u8>,
}

#[cfg(debug_assertions)]
pub mod test_hooks {
	use std::sync::Arc;

	use parking_lot::Mutex;
	use tokio::sync::Notify;

	static PAUSE_AFTER_MARKER_CLEAR: Mutex<Option<(String, Arc<Notify>, Arc<Notify>)>> =
		Mutex::new(None);

	pub struct PauseGuard;

	pub fn pause_after_marker_clear(actor_id: &str) -> (PauseGuard, Arc<Notify>, Arc<Notify>) {
		let reached = Arc::new(Notify::new());
		let release = Arc::new(Notify::new());
		*PAUSE_AFTER_MARKER_CLEAR.lock() =
			Some((actor_id.to_string(), Arc::clone(&reached), Arc::clone(&release)));
		(PauseGuard, reached, release)
	}

	pub(super) async fn maybe_pause_after_marker_clear(actor_id: &str) {
		let hook = PAUSE_AFTER_MARKER_CLEAR
			.lock()
			.as_ref()
			.filter(|(hook_actor_id, _, _)| hook_actor_id == actor_id)
			.map(|(_, reached, release)| (Arc::clone(reached), Arc::clone(release)));
		if let Some((reached, release)) = hook {
			reached.notify_waiters();
			release.notified().await;
		}
	}

	impl Drop for PauseGuard {
		fn drop(&mut self) {
			*PAUSE_AFTER_MARKER_CLEAR.lock() = None;
		}
	}
}

#[cfg(not(debug_assertions))]
mod test_hooks {
	pub(super) async fn maybe_pause_after_marker_clear(_actor_id: &str) {}
}
