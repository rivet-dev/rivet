//! PITR fork implementation for sqlite admin operations.

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
use universalpubsub::PubSub;
use uuid::Uuid;

use crate::{
	admin::{
		self, ForkDstSpec, ForkMode, OpResult, OpStatus, RestoreTarget, SqliteAdminError,
		decode_admin_op_record, encode_admin_op_record,
	},
	pump::{
		keys::{self, SHARD_SIZE},
		ltx::decode_ltx_v3,
		quota,
		types::{
			CheckpointMeta, Checkpoints, DBHead, DeltaMeta, ForkMarker, ForkStep,
			RetentionConfig, decode_checkpoint_meta, decode_checkpoints, decode_db_head,
			decode_delta_meta, decode_retention_config, encode_checkpoint_meta,
			encode_checkpoints, encode_db_head, encode_delta_meta, encode_fork_marker,
			encode_retention_config,
		},
	},
};

use super::{TakeOutcome, fold_shard, lease, metrics};

const FORK_COPY_BATCH_ROWS: usize = 75;
const FORK_LEASE_TTL_MS: u64 = 30_000;

#[allow(clippy::too_many_arguments)]
pub async fn handle_fork(
	udb: Arc<universaldb::Database>,
	_ups: PubSub,
	op_id: Uuid,
	src_actor_id: String,
	target: RestoreTarget,
	mode: ForkMode,
	dst: ForkDstSpec,
	holder_id: NodeId,
	cancel_token: CancellationToken,
) -> Result<()> {
	let started_at = Instant::now();
	let result = handle_fork_inner(
		Arc::clone(&udb),
		op_id,
		src_actor_id.clone(),
		target,
		mode,
		dst,
		holder_id,
		cancel_token,
	)
	.await;

	match result {
		Ok(()) => {
			metrics::SQLITE_FORK_DURATION_SECONDS
				.with_label_values(&["success"])
				.observe(started_at.elapsed().as_secs_f64());
			Ok(())
		}
		Err(err) => {
			metrics::SQLITE_FORK_DURATION_SECONDS
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
			let _ = release_lease(Arc::clone(&udb), src_actor_id, holder_id).await;
			Err(err)
		}
	}
}

#[allow(clippy::too_many_arguments)]
async fn handle_fork_inner(
	udb: Arc<universaldb::Database>,
	op_id: Uuid,
	src_actor_id: String,
	target: RestoreTarget,
	mode: ForkMode,
	dst: ForkDstSpec,
	holder_id: NodeId,
	cancel_token: CancellationToken,
) -> Result<()> {
	ensure_not_cancelled(&cancel_token)?;
	take_lease(Arc::clone(&udb), src_actor_id.clone(), holder_id).await?;
	admin::update_status(Arc::clone(&udb), op_id, OpStatus::InProgress, Some(holder_id)).await?;

	let mut dst_actor_id = match &dst {
		ForkDstSpec::Allocate { .. } => None,
		ForkDstSpec::Existing { dst_actor_id } => Some(dst_actor_id.clone()),
	};
	let existing_marker = if let Some(dst_actor_id) = &dst_actor_id {
		read_fork_marker(Arc::clone(&udb), dst_actor_id.clone()).await?
	} else {
		None
	};
	let mut resuming = false;
	let mut refs_pinned = false;
	let plan = if let Some(marker) = existing_marker {
		dst_actor_id = Some(marker_actor_id_from_marker(&marker, &dst));
		resuming = true;
		refs_pinned = true;
		load_plan_from_marker(Arc::clone(&udb), src_actor_id.clone(), marker).await?
	} else {
		load_fork_plan(Arc::clone(&udb), src_actor_id.clone(), target).await?
	};

	if mode == ForkMode::DryRun {
		admin::complete(
			Arc::clone(&udb),
			op_id,
			OpResult::Message {
				message: format!(
					"fork dry run: target_txid={}, ckp_txid={}, deltas_to_replay={}, estimated_bytes={}, estimated_duration_ms={}",
					plan.target_txid,
					plan.ckp_txid,
					plan.delta_txids.len(),
					plan.estimated_bytes,
					estimated_duration_ms(&plan)
				),
			},
		)
		.await?;
		release_lease(Arc::clone(&udb), src_actor_id, holder_id).await?;
		return Ok(());
	}

	if !refs_pinned {
		pin_source_refs(
			Arc::clone(&udb),
			src_actor_id.clone(),
			plan.ckp_txid,
			&plan.delta_txids,
			1,
		)
		.await?;
		refs_pinned = true;
	}
	test_hooks::maybe_pause_after_source_refs_pinned(&src_actor_id).await;
	release_lease(Arc::clone(&udb), src_actor_id.clone(), holder_id).await?;

	let dst_actor_id = match dst_actor_id {
		Some(dst_actor_id) => dst_actor_id,
		None => allocate_dst_actor_id(&dst),
	};
	let mut dst_lease_taken = false;
	let mut marker_written = false;
	let work_result = async {
		take_lease(Arc::clone(&udb), dst_actor_id.clone(), holder_id).await?;
		dst_lease_taken = true;
		if !resuming {
			validate_dst_empty(Arc::clone(&udb), dst_actor_id.clone()).await?;

			write_fork_marker_and_head(
				Arc::clone(&udb),
				&src_actor_id,
				&dst_actor_id,
				op_id,
				&plan,
				holder_id,
			)
			.await?;
			marker_written = true;
			test_hooks::maybe_pause_after_marker_write(&dst_actor_id).await;
		}

		match plan.last_completed_step {
			None | Some(ForkStep::Started) => {
				copy_checkpoint_rows(
					Arc::clone(&udb),
					src_actor_id.clone(),
					dst_actor_id.clone(),
					&plan,
					&cancel_token,
				)
				.await?;
			}
			Some(ForkStep::CheckpointCopied) => {}
			Some(ForkStep::DeltasReplayed | ForkStep::MetaWritten) => {}
		}

		if !matches!(
			plan.last_completed_step,
			Some(ForkStep::DeltasReplayed | ForkStep::MetaWritten)
		) {
			replay_deltas(
				Arc::clone(&udb),
				src_actor_id.clone(),
				dst_actor_id.clone(),
				&plan,
				&cancel_token,
			)
			.await?;
		}
		finalize_dst_meta(
			Arc::clone(&udb),
			dst_actor_id.clone(),
			&plan,
			op_id,
		)
		.await?;
		Ok::<(), anyhow::Error>(())
	}
	.await;

	if let Err(err) = work_result {
		if marker_written || dst_lease_taken {
			let _ = clear_actor_prefix(Arc::clone(&udb), dst_actor_id.clone()).await;
		}
		if refs_pinned {
			let _ = pin_source_refs(
				Arc::clone(&udb),
				src_actor_id.clone(),
				plan.ckp_txid,
				&plan.delta_txids,
				-1,
			)
			.await;
		}
		if dst_lease_taken {
			let _ = release_lease(Arc::clone(&udb), dst_actor_id, holder_id).await;
		}
		return Err(err);
	}

	release_lease(Arc::clone(&udb), dst_actor_id.clone(), holder_id).await?;
	pin_source_refs(
		Arc::clone(&udb),
		src_actor_id,
		plan.ckp_txid,
		&plan.delta_txids,
		-1,
	)
	.await?;
	metrics::SQLITE_FORK_DELTAS_REPLAYED.observe(plan.delta_txids.len() as f64);

	Ok(())
}

async fn take_lease(
	udb: Arc<universaldb::Database>,
	actor_id: String,
	holder_id: NodeId,
) -> Result<()> {
	for _ in 0..100 {
		let now_ms = now_ms()?;
		let outcome = udb
			.run({
				let actor_id = actor_id.clone();
				move |tx| {
					let actor_id = actor_id.clone();
					async move {
						lease::take(&tx, &actor_id, holder_id, FORK_LEASE_TTL_MS, now_ms).await
					}
				}
			})
			.await?;
		match outcome {
			TakeOutcome::Acquired => return Ok(()),
			TakeOutcome::Skip => tokio::time::sleep(std::time::Duration::from_millis(10)).await,
		}
	}
	bail!("sqlite fork could not take compactor lease")
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

async fn load_fork_plan(
	udb: Arc<universaldb::Database>,
	src_actor_id: String,
	target: RestoreTarget,
) -> Result<ForkPlan> {
	udb.run(move |tx| {
		let src_actor_id = src_actor_id.clone();
		async move {
			let head = read_head(&tx, &src_actor_id).await?;
			let checkpoints = read_checkpoints(&tx, &src_actor_id).await?;
			let retention = tx_get_value(&tx, &keys::meta_retention_key(&src_actor_id), Snapshot)
				.await?
				.as_deref()
				.map(decode_retention_config)
				.transpose()
				.context("decode sqlite retention config for fork")?
				.unwrap_or_default();
			let delta_metas = load_delta_metas(&tx, &src_actor_id).await?;
			let target_txid = resolve_target(target, &head, &checkpoints, &delta_metas)?;
			let ckp = checkpoints
				.entries
				.iter()
				.filter(|entry| entry.ckp_txid <= target_txid)
				.max_by_key(|entry| entry.ckp_txid)
				.cloned()
				.ok_or_else(|| invalid_restore_point(target_txid, &checkpoints))?;
			validate_reachable(target_txid, ckp.ckp_txid, &delta_metas, &checkpoints)?;
			let checkpoint_meta = read_checkpoint_meta(&tx, &src_actor_id, ckp.ckp_txid).await?;
			let delta_txids = ((ckp.ckp_txid + 1)..=target_txid).collect::<Vec<_>>();
			let delta_bytes = delta_txids
				.iter()
				.map(|txid| {
					Ok(delta_metas
						.get(txid)
						.context("validated delta meta missing")?
						.byte_count)
				})
				.sum::<Result<u64>>()?;

			Ok(ForkPlan {
				target_txid,
				ckp_txid: ckp.ckp_txid,
				checkpoint_db_size_pages: checkpoint_meta.db_size_pages,
				delta_txids,
				retention,
				estimated_bytes: checkpoint_meta.byte_count.saturating_add(delta_bytes),
				last_completed_step: None,
				started_at_ms: now_ms()?,
			})
		}
	})
	.await
}

async fn load_plan_from_marker(
	udb: Arc<universaldb::Database>,
	src_actor_id: String,
	marker: ForkMarker,
) -> Result<ForkPlan> {
	udb.run(move |tx| {
		let src_actor_id = src_actor_id.clone();
		let marker = marker.clone();
		async move {
			let checkpoints = read_checkpoints(&tx, &src_actor_id).await?;
			let checkpoint_meta = read_checkpoint_meta(&tx, &src_actor_id, marker.ckp_txid).await?;
			let delta_metas = load_delta_metas(&tx, &src_actor_id).await?;
			validate_reachable(
				marker.target_txid,
				marker.ckp_txid,
				&delta_metas,
				&checkpoints,
			)?;
			let delta_txids = ((marker.ckp_txid + 1)..=marker.target_txid).collect::<Vec<_>>();
			let delta_bytes = delta_txids
				.iter()
				.filter_map(|txid| delta_metas.get(txid).map(|meta| meta.byte_count))
				.sum();
			let retention = tx_get_value(&tx, &keys::meta_retention_key(&src_actor_id), Snapshot)
				.await?
				.as_deref()
				.map(decode_retention_config)
				.transpose()
				.context("decode sqlite retention config for fork")?
				.unwrap_or_default();

			Ok(ForkPlan {
				target_txid: marker.target_txid,
				ckp_txid: marker.ckp_txid,
				checkpoint_db_size_pages: checkpoint_meta.db_size_pages,
				delta_txids,
				retention,
				estimated_bytes: checkpoint_meta.byte_count.saturating_add(delta_bytes),
				last_completed_step: Some(marker.last_completed_step),
				started_at_ms: marker.started_at_ms,
			})
		}
	})
	.await
}

async fn pin_source_refs(
	udb: Arc<universaldb::Database>,
	src_actor_id: String,
	ckp_txid: u64,
	delta_txids: &[u64],
	delta: i32,
) -> Result<()> {
	let delta_txids = delta_txids.to_vec();
	udb.run(move |tx| {
		let src_actor_id = src_actor_id.clone();
		let delta_txids = delta_txids.clone();
		async move {
			let checkpoint_key = keys::checkpoint_meta_key(&src_actor_id, ckp_txid);
			let Some(checkpoint_bytes) = tx.informal().get(&checkpoint_key, Serializable).await?
			else {
				bail!("sqlite fork checkpoint {ckp_txid} missing");
			};
			let mut checkpoint =
				decode_checkpoint_meta(&checkpoint_bytes).context("decode checkpoint meta")?;
			checkpoint.refcount = apply_refcount_delta(checkpoint.refcount, delta)?;
			tx.informal().set(
				&checkpoint_key,
				&encode_checkpoint_meta(checkpoint).context("encode checkpoint meta")?,
			);

			let checkpoints_key = keys::meta_checkpoints_key(&src_actor_id);
			if let Some(checkpoints_bytes) = tx.informal().get(&checkpoints_key, Serializable).await?
			{
				let mut checkpoints =
					decode_checkpoints(&checkpoints_bytes).context("decode checkpoints")?;
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

			for txid in delta_txids {
				let key = keys::delta_meta_key(&src_actor_id, txid);
				let Some(delta_bytes) = tx.informal().get(&key, Serializable).await? else {
					bail!("sqlite fork delta {txid} missing meta");
				};
				let mut meta = decode_delta_meta(&delta_bytes).context("decode delta meta")?;
				meta.refcount = apply_refcount_delta(meta.refcount, delta)?;
				tx.informal()
					.set(&key, &encode_delta_meta(meta).context("encode delta meta")?);
			}

			Ok(())
		}
	})
	.await
}

async fn validate_dst_empty(
	udb: Arc<universaldb::Database>,
	dst_actor_id: String,
) -> Result<()> {
	udb.run(move |tx| {
		let dst_actor_id = dst_actor_id.clone();
		async move {
			if tx
				.informal()
				.get(&keys::meta_head_key(&dst_actor_id), Serializable)
				.await?
				.is_some()
			{
				return Err(SqliteAdminError::ForkDestinationAlreadyExists {
					dst_actor_id: dst_actor_id.clone(),
				}
				.build()
				.into());
			}
			let lease_key = keys::meta_compactor_lease_key(&dst_actor_id);
			let has_existing_rows = scan_prefix_values(&tx, &keys::actor_prefix(&dst_actor_id), Snapshot)
				.await?
				.into_iter()
				.any(|(key, _)| key != lease_key);
			if has_existing_rows {
				return Err(SqliteAdminError::ForkDestinationAlreadyExists {
					dst_actor_id: dst_actor_id.clone(),
				}
				.build()
				.into());
			}
			Ok(())
		}
	})
	.await
}

async fn write_fork_marker_and_head(
	udb: Arc<universaldb::Database>,
	src_actor_id: &str,
	dst_actor_id: &str,
	op_id: Uuid,
	plan: &ForkPlan,
	holder_id: NodeId,
) -> Result<()> {
	let marker = ForkMarker {
		src_actor_id: src_actor_id.to_string(),
		ckp_txid: plan.ckp_txid,
		target_txid: plan.target_txid,
		started_at_ms: plan.started_at_ms,
		last_completed_step: ForkStep::Started,
		holder_id,
		op_id,
	};
	let encoded_marker = encode_fork_marker(marker).context("encode fork marker")?;
	let dst_actor_id = dst_actor_id.to_string();
	udb.run(move |tx| {
		let dst_actor_id = dst_actor_id.clone();
		let encoded_marker = encoded_marker.clone();
		async move {
			tx.informal()
				.set(&keys::meta_fork_in_progress_key(&dst_actor_id), &encoded_marker);
			tx.informal().set(
				&keys::meta_head_key(&dst_actor_id),
				&encode_db_head(DBHead {
					head_txid: 0,
					db_size_pages: 0,
					#[cfg(debug_assertions)]
					generation: 0,
				})?,
			);
			Ok(())
		}
	})
	.await?;
	metrics::SQLITE_FORK_IN_PROGRESS_ACTIVE.inc();
	Ok(())
}

async fn copy_checkpoint_rows(
	udb: Arc<universaldb::Database>,
	src_actor_id: String,
	dst_actor_id: String,
	plan: &ForkPlan,
	cancel_token: &CancellationToken,
) -> Result<()> {
	let rows = load_checkpoint_copy_rows(
		Arc::clone(&udb),
		src_actor_id,
		dst_actor_id.clone(),
		plan.ckp_txid,
	)
	.await?;
	for chunk in rows.chunks(FORK_COPY_BATCH_ROWS) {
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
	update_fork_marker_step(Arc::clone(&udb), dst_actor_id.clone(), ForkStep::CheckpointCopied)
		.await?;
	write_head(
		Arc::clone(&udb),
		dst_actor_id,
		plan.ckp_txid,
		plan.checkpoint_db_size_pages,
	)
	.await
}

async fn replay_deltas(
	udb: Arc<universaldb::Database>,
	src_actor_id: String,
	dst_actor_id: String,
	plan: &ForkPlan,
	cancel_token: &CancellationToken,
) -> Result<()> {
	for txid in &plan.delta_txids {
		ensure_not_cancelled(cancel_token)?;
		copy_delta_rows(
			Arc::clone(&udb),
			src_actor_id.clone(),
			dst_actor_id.clone(),
			*txid,
		)
		.await?;
		let delta = load_delta_blob(Arc::clone(&udb), src_actor_id.clone(), *txid).await?;
		let decoded = decode_ltx_v3(&delta).with_context(|| format!("decode fork delta {txid}"))?;
		let db_size_pages = decoded.header.commit;
		let mut shard_ids = BTreeSet::new();
		for page in &decoded.pages {
			shard_ids.insert(page.pgno / SHARD_SIZE);
		}
		let pages = decoded.pages.clone();
		let shard_ids_for_tx = shard_ids.clone();
		udb.run({
			let dst_actor_id = dst_actor_id.clone();
			move |tx| {
				let dst_actor_id = dst_actor_id.clone();
				let pages = pages.clone();
				let shard_ids = shard_ids_for_tx.clone();
				async move {
					for shard_id in shard_ids {
						let page_updates = pages
							.iter()
							.filter(|page| page.pgno / SHARD_SIZE == shard_id)
							.map(|page| (page.pgno, page.bytes.clone()))
							.collect::<Vec<_>>();
						fold_shard(&tx, &dst_actor_id, shard_id, page_updates).await?;
					}
					let txid_bytes = txid.to_be_bytes();
					for page in &pages {
						tx.informal()
							.set(&keys::pidx_delta_key(&dst_actor_id, page.pgno), &txid_bytes);
					}
					tx.informal().set(
						&keys::meta_head_key(&dst_actor_id),
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
	update_fork_marker_step(Arc::clone(&udb), dst_actor_id, ForkStep::DeltasReplayed).await
}

async fn copy_delta_rows(
	udb: Arc<universaldb::Database>,
	src_actor_id: String,
	dst_actor_id: String,
	txid: u64,
) -> Result<()> {
	let rows = udb
		.run(move |tx| {
			let src_actor_id = src_actor_id.clone();
			let dst_actor_id = dst_actor_id.clone();
			async move {
				let mut rows = Vec::new();
				for (key, value) in
					scan_prefix_values(&tx, &keys::delta_chunk_prefix(&src_actor_id, txid), Snapshot)
						.await?
				{
					let dst_key = if key == keys::delta_meta_key(&src_actor_id, txid) {
						keys::delta_meta_key(&dst_actor_id, txid)
					} else {
						let chunk_idx = keys::decode_delta_chunk_idx(&src_actor_id, txid, &key)?;
						keys::delta_chunk_key(&dst_actor_id, txid, chunk_idx)
					};
					rows.push(CopyRow { dst_key, value });
				}
				Ok(rows)
			}
		})
		.await?;
	udb.run(move |tx| {
		let rows = rows.clone();
		async move {
			for row in rows {
				tx.informal().set(&row.dst_key, &row.value);
			}
			Ok(())
		}
	})
	.await
}

async fn finalize_dst_meta(
	udb: Arc<universaldb::Database>,
	dst_actor_id: String,
	plan: &ForkPlan,
	op_id: Uuid,
) -> Result<()> {
	write_head(
		Arc::clone(&udb),
		dst_actor_id.clone(),
		plan.target_txid,
		final_db_size_pages(Arc::clone(&udb), dst_actor_id.clone(), plan).await?,
	)
	.await?;
	update_fork_marker_step(Arc::clone(&udb), dst_actor_id.clone(), ForkStep::MetaWritten).await?;

	let result = OpResult::Message {
		message: format!(
			"forked_to_actor={}, head_txid={}",
			dst_actor_id, plan.target_txid
		),
	};
	let now_ms = now_ms()?;
	udb.run(move |tx| {
		let dst_actor_id = dst_actor_id.clone();
		let result = result.clone();
		let retention = plan.retention.clone();
		async move {
			let live_bytes = scan_live_storage_bytes(&tx, &dst_actor_id).await?;
			let current_live = quota::read_live(&tx, &dst_actor_id).await?;
			let live_delta = live_bytes
				.checked_sub(current_live)
				.context("sqlite fork live quota delta overflowed")?;
			if live_delta != 0 {
				quota::atomic_add_live(&tx, &dst_actor_id, live_delta);
			}
			tx.informal().set(
				&keys::meta_retention_key(&dst_actor_id),
				&encode_retention_config(retention).context("encode fork retention")?,
			);
			tx.informal().set(
				&keys::meta_checkpoints_key(&dst_actor_id),
				&encode_checkpoints(Checkpoints {
					entries: Vec::new(),
				})
				.context("encode fork checkpoints")?,
			);
			tx.informal()
				.clear(&keys::meta_fork_in_progress_key(&dst_actor_id));

			let (record_key, mut record) = read_record_for_update(&tx, op_id).await?;
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
	metrics::SQLITE_FORK_IN_PROGRESS_ACTIVE.dec();
	Ok(())
}

async fn final_db_size_pages(
	udb: Arc<universaldb::Database>,
	dst_actor_id: String,
	plan: &ForkPlan,
) -> Result<u32> {
	if !plan.delta_txids.is_empty() {
		let head = udb
			.run(move |tx| {
				let dst_actor_id = dst_actor_id.clone();
				async move { read_head(&tx, &dst_actor_id).await }
			})
			.await?;
		return Ok(head.db_size_pages);
	}
	Ok(plan.checkpoint_db_size_pages)
}

async fn read_fork_marker(
	udb: Arc<universaldb::Database>,
	dst_actor_id: String,
) -> Result<Option<ForkMarker>> {
	udb.run(move |tx| {
		let dst_actor_id = dst_actor_id.clone();
		async move {
			Ok(tx_get_value(&tx, &keys::meta_fork_in_progress_key(&dst_actor_id), Snapshot)
				.await?
				.as_deref()
				.map(crate::pump::types::decode_fork_marker)
				.transpose()?)
		}
	})
	.await
}

async fn read_head(tx: &universaldb::Transaction, actor_id: &str) -> Result<DBHead> {
	let head_bytes = tx_get_value(tx, &keys::meta_head_key(actor_id), Snapshot)
		.await?
		.context("sqlite fork requires db head")?;
	decode_db_head(&head_bytes).context("decode sqlite fork db head")
}

async fn read_checkpoints(tx: &universaldb::Transaction, actor_id: &str) -> Result<Checkpoints> {
	Ok(tx_get_value(tx, &keys::meta_checkpoints_key(actor_id), Snapshot)
		.await?
		.as_deref()
		.map(decode_checkpoints)
		.transpose()
		.context("decode sqlite checkpoints for fork")?
		.unwrap_or(Checkpoints {
			entries: Vec::new(),
		}))
}

async fn read_checkpoint_meta(
	tx: &universaldb::Transaction,
	actor_id: &str,
	ckp_txid: u64,
) -> Result<CheckpointMeta> {
	let meta_bytes = tx_get_value(tx, &keys::checkpoint_meta_key(actor_id, ckp_txid), Snapshot)
		.await?
		.with_context(|| format!("sqlite checkpoint {ckp_txid} missing meta"))?;
	decode_checkpoint_meta(&meta_bytes).context("decode sqlite checkpoint meta")
}

async fn load_delta_metas(
	tx: &universaldb::Transaction,
	actor_id: &str,
) -> Result<BTreeMap<u64, DeltaMeta>> {
	let mut metas = BTreeMap::new();
	for (key, value) in scan_prefix_values(tx, &keys::delta_prefix(actor_id), Snapshot).await? {
		let txid = keys::decode_delta_chunk_txid(actor_id, &key)?;
		if key == keys::delta_meta_key(actor_id, txid) {
			let meta = decode_delta_meta(&value).context("decode sqlite fork delta meta")?;
			metas.insert(txid, meta);
		}
	}
	Ok(metas)
}

fn resolve_target(
	target: RestoreTarget,
	head: &DBHead,
	checkpoints: &Checkpoints,
	delta_metas: &BTreeMap<u64, DeltaMeta>,
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
			.chain(delta_metas.iter().filter_map(|(txid, meta)| {
				(meta.taken_at_ms <= timestamp_ms).then_some(*txid)
			}))
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
	delta_metas: &BTreeMap<u64, DeltaMeta>,
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
	src_actor_id: String,
	dst_actor_id: String,
	ckp_txid: u64,
) -> Result<Vec<CopyRow>> {
	udb.run(move |tx| {
		let src_actor_id = src_actor_id.clone();
		let dst_actor_id = dst_actor_id.clone();
		async move {
			let mut rows = Vec::new();
			let mut checkpoint_delta_txids = BTreeSet::new();
			for (key, value) in scan_prefix_values(
				&tx,
				&checkpoint_shard_prefix(&src_actor_id, ckp_txid),
				Snapshot,
			)
			.await?
			{
				let shard_id = decode_checkpoint_suffix_u32(&key)?;
				rows.push(CopyRow {
					dst_key: keys::shard_key(&dst_actor_id, shard_id),
					value,
				});
			}
			for (key, value) in scan_prefix_values(
				&tx,
				&checkpoint_pidx_prefix(&src_actor_id, ckp_txid),
				Snapshot,
			)
			.await?
			{
				let pgno = decode_checkpoint_suffix_u32(&key)?;
				checkpoint_delta_txids.insert(decode_pidx_txid(&value)?);
				rows.push(CopyRow {
					dst_key: keys::pidx_delta_key(&dst_actor_id, pgno),
					value,
				});
			}
			for txid in checkpoint_delta_txids {
				for (key, value) in
					scan_prefix_values(&tx, &keys::delta_chunk_prefix(&src_actor_id, txid), Snapshot)
						.await?
				{
					let dst_key = if key == keys::delta_meta_key(&src_actor_id, txid) {
						keys::delta_meta_key(&dst_actor_id, txid)
					} else {
						let chunk_idx = keys::decode_delta_chunk_idx(&src_actor_id, txid, &key)?;
						keys::delta_chunk_key(&dst_actor_id, txid, chunk_idx)
					};
					rows.push(CopyRow { dst_key, value });
				}
			}
			Ok(rows)
		}
	})
	.await
}

async fn load_delta_blob(
	udb: Arc<universaldb::Database>,
	src_actor_id: String,
	txid: u64,
) -> Result<Vec<u8>> {
	udb.run(move |tx| {
		let src_actor_id = src_actor_id.clone();
		async move {
			let mut chunks = Vec::new();
			for (key, value) in
				scan_prefix_values(&tx, &keys::delta_chunk_prefix(&src_actor_id, txid), Snapshot)
					.await?
			{
				if key == keys::delta_meta_key(&src_actor_id, txid) {
					continue;
				}
				let idx = keys::decode_delta_chunk_idx(&src_actor_id, txid, &key)?;
				chunks.push((idx, value));
			}
			if chunks.is_empty() {
				bail!("sqlite fork missing delta {txid}");
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

async fn update_fork_marker_step(
	udb: Arc<universaldb::Database>,
	dst_actor_id: String,
	step: ForkStep,
) -> Result<()> {
	udb.run(move |tx| {
		let dst_actor_id = dst_actor_id.clone();
		async move {
			let marker_key = keys::meta_fork_in_progress_key(&dst_actor_id);
			let Some(marker_bytes) = tx.informal().get(&marker_key, Serializable).await? else {
				return Ok(());
			};
			let mut marker = crate::pump::types::decode_fork_marker(&marker_bytes)?;
			marker.last_completed_step = step;
			tx.informal().set(&marker_key, &encode_fork_marker(marker)?);
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

async fn clear_actor_prefix(udb: Arc<universaldb::Database>, actor_id: String) -> Result<()> {
	udb.run(move |tx| {
		let actor_id = actor_id.clone();
		async move {
			let (begin, end) = prefix_range(&keys::actor_prefix(&actor_id));
			tx.informal().clear_range(&begin, &end);
			Ok(())
		}
	})
	.await
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

fn decode_pidx_txid(value: &[u8]) -> Result<u64> {
	Ok(u64::from_be_bytes(
		value
			.try_into()
			.context("PIDX value should decode as u64")?,
	))
}

async fn read_record_for_update(
	tx: &universaldb::Transaction,
	op_id: Uuid,
) -> Result<(Vec<u8>, admin::AdminOpRecord)> {
	let sqlite_subspace =
		universaldb::Subspace::from(universaldb::tuple::Subspace::from_bytes(vec![
			keys::SQLITE_SUBSPACE_PREFIX,
		]));
	let informal = tx.informal();
	let mut stream = informal.get_ranges_keyvalues(
		RangeOption {
			mode: StreamingMode::WantAll,
			..RangeOption::from(&sqlite_subspace)
		},
		Serializable,
	);

	while let Some(entry) = stream.try_next().await? {
		let key = entry.key();
		if !key.ends_with(op_id.as_bytes()) || !key_has_admin_op_suffix(key) {
			continue;
		}
		let record =
			decode_admin_op_record(entry.value()).context("decode sqlite admin op record")?;
		if record.operation_id == op_id {
			return Ok((key.to_vec(), record));
		}
	}

	bail!("sqlite admin op record not found: {op_id}")
}

fn key_has_admin_op_suffix(key: &[u8]) -> bool {
	key.windows(b"/META/admin_op/".len())
		.any(|window| window == b"/META/admin_op/")
}

fn apply_refcount_delta(current: u32, delta: i32) -> Result<u32> {
	if delta >= 0 {
		current
			.checked_add(delta as u32)
			.context("sqlite refcount overflowed")
	} else {
		current
			.checked_sub(delta.unsigned_abs())
			.context("sqlite refcount underflowed")
	}
}

fn allocate_dst_actor_id(dst: &ForkDstSpec) -> String {
	match dst {
		ForkDstSpec::Allocate { dst_namespace_id } => {
			format!("{dst_namespace_id}:{}", Uuid::new_v4())
		}
		ForkDstSpec::Existing { dst_actor_id } => dst_actor_id.clone(),
	}
}

fn marker_actor_id_from_marker(marker: &ForkMarker, dst: &ForkDstSpec) -> String {
	match dst {
		ForkDstSpec::Allocate { .. } => marker.src_actor_id.clone(),
		ForkDstSpec::Existing { dst_actor_id } => dst_actor_id.clone(),
	}
}

fn estimated_duration_ms(plan: &ForkPlan) -> u64 {
	10 + plan.delta_txids.len() as u64 * 5
}

fn ensure_not_cancelled(cancel_token: &CancellationToken) -> Result<()> {
	if cancel_token.is_cancelled() {
		bail!("sqlite fork cancelled");
	}
	Ok(())
}

fn now_ms() -> Result<i64> {
	let elapsed = SystemTime::now()
		.duration_since(SystemTime::UNIX_EPOCH)
		.context("system clock was before unix epoch")?;
	i64::try_from(elapsed.as_millis()).context("sqlite fork timestamp exceeded i64")
}

#[derive(Debug, Clone)]
struct ForkPlan {
	target_txid: u64,
	ckp_txid: u64,
	checkpoint_db_size_pages: u32,
	delta_txids: Vec<u64>,
	retention: RetentionConfig,
	estimated_bytes: u64,
	last_completed_step: Option<ForkStep>,
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

	static PAUSE_AFTER_SOURCE_REFS_PINNED: Mutex<Option<(String, Arc<Notify>, Arc<Notify>)>> =
		Mutex::new(None);
	static PAUSE_AFTER_MARKER_WRITE: Mutex<Option<(String, Arc<Notify>, Arc<Notify>)>> =
		Mutex::new(None);

	pub struct PauseGuard;

	pub fn pause_after_source_refs_pinned(
		actor_id: &str,
	) -> (PauseGuard, Arc<Notify>, Arc<Notify>) {
		let reached = Arc::new(Notify::new());
		let release = Arc::new(Notify::new());
		*PAUSE_AFTER_SOURCE_REFS_PINNED.lock() =
			Some((actor_id.to_string(), Arc::clone(&reached), Arc::clone(&release)));
		(PauseGuard, reached, release)
	}

	pub fn pause_after_marker_write(
		actor_id: &str,
	) -> (PauseGuard, Arc<Notify>, Arc<Notify>) {
		let reached = Arc::new(Notify::new());
		let release = Arc::new(Notify::new());
		*PAUSE_AFTER_MARKER_WRITE.lock() =
			Some((actor_id.to_string(), Arc::clone(&reached), Arc::clone(&release)));
		(PauseGuard, reached, release)
	}

	pub(super) async fn maybe_pause_after_source_refs_pinned(actor_id: &str) {
		let hook = PAUSE_AFTER_SOURCE_REFS_PINNED
			.lock()
			.as_ref()
			.filter(|(hook_actor_id, _, _)| hook_actor_id == actor_id)
			.map(|(_, reached, release)| (Arc::clone(reached), Arc::clone(release)));
		if let Some((reached, release)) = hook {
			reached.notify_waiters();
			release.notified().await;
		}
	}

	pub(super) async fn maybe_pause_after_marker_write(actor_id: &str) {
		let hook = PAUSE_AFTER_MARKER_WRITE
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
			*PAUSE_AFTER_SOURCE_REFS_PINNED.lock() = None;
			*PAUSE_AFTER_MARKER_WRITE.lock() = None;
		}
	}
}

#[cfg(not(debug_assertions))]
mod test_hooks {
	pub(super) async fn maybe_pause_after_source_refs_pinned(_actor_id: &str) {}
	pub(super) async fn maybe_pause_after_marker_write(_actor_id: &str) {}
}
