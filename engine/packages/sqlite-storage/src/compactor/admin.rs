//! Short-running sqlite admin operation handlers.

use std::{collections::BTreeMap, sync::Arc};

use anyhow::{Context, Result};
use futures_util::TryStreamExt;
use gas::prelude::Id;
use namespace::types::SqliteNamespaceConfig;
use universaldb::{
	RangeOption,
	options::StreamingMode,
	utils::IsolationLevel::{Serializable, Snapshot},
};
use uuid::Uuid;

use crate::{
	admin as admin_record,
	admin::{
		CheckpointView, ClearRefcountResult, FineGrainedWindow, HeadView, OpKind, OpResult,
		OpStatus, RefcountKind, RetentionView, SqliteAdminError, decode_admin_op_record,
	},
	pump::{
		keys,
		quota,
		types::{
			Checkpoints, DBHead, DeltaMeta, RetentionConfig, decode_checkpoint_meta,
			decode_checkpoints, decode_db_head, decode_delta_meta, decode_retention_config,
			encode_checkpoint_meta, encode_checkpoints, encode_delta_meta, encode_retention_config,
		},
	},
};

#[derive(Clone, Debug)]
pub struct DescribeRetentionRequest {
	pub actor_id: String,
	pub audit: admin_record::AuditFields,
}

#[derive(Clone, Debug)]
pub struct GetRetentionRequest {
	pub actor_id: String,
	pub audit: admin_record::AuditFields,
}

#[derive(Clone, Debug)]
pub struct SetRetentionRequest {
	pub actor_id: String,
	pub config: RetentionConfig,
	pub audit: admin_record::AuditFields,
}

#[derive(Clone, Debug)]
pub struct ClearRefcountRequest {
	pub actor_id: String,
	pub kind: RefcountKind,
	pub txid: u64,
	pub audit: admin_record::AuditFields,
}

pub async fn handle_describe_retention(
	udb: Arc<universaldb::Database>,
	op_id: Uuid,
	req: DescribeRetentionRequest,
) -> Result<RetentionView> {
	start_short_op(Arc::clone(&udb), op_id).await?;
	let view = load_retention_view(Arc::clone(&udb), req.actor_id, req.audit).await?;
	admin_record::complete(udb, op_id, OpResult::RetentionView(view.clone())).await?;
	Ok(view)
}

pub async fn inspect_retention_view(
	udb: Arc<universaldb::Database>,
	actor_id: String,
	namespace_id: Uuid,
) -> Result<RetentionView> {
	load_retention_view(
		udb,
		actor_id,
		admin_record::AuditFields {
			caller_id: "sqlite-inspector".to_string(),
			request_origin_ts_ms: 0,
			namespace_id,
		},
	)
	.await
}

pub async fn handle_get_retention(
	udb: Arc<universaldb::Database>,
	op_id: Uuid,
	req: GetRetentionRequest,
) -> Result<RetentionConfig> {
	start_short_op(Arc::clone(&udb), op_id).await?;
	let config = read_effective_retention(Arc::clone(&udb), req.actor_id, req.audit).await?;
	admin_record::complete(udb, op_id, OpResult::RetentionConfig(config.clone())).await?;
	Ok(config)
}

pub async fn handle_set_retention(
	udb: Arc<universaldb::Database>,
	op_id: Uuid,
	req: SetRetentionRequest,
) -> Result<RetentionView> {
	start_short_op(Arc::clone(&udb), op_id).await?;
	let validate_result = validate_retention_config(Arc::clone(&udb), &req).await;
	if let Err(err) = validate_result {
		fail_short_op(Arc::clone(&udb), op_id, &err).await?;
		return Err(err);
	}

	let actor_id = req.actor_id.clone();
	let config = req.config.clone();
	udb.run(move |tx| {
		let actor_id = actor_id.clone();
		let config = config.clone();

		async move {
			tx.informal().set(
				&keys::meta_retention_key(&actor_id),
				&encode_retention_config(config).context("encode sqlite retention config")?,
			);
			Ok(())
		}
	})
	.await?;

	let view = load_retention_view(Arc::clone(&udb), req.actor_id, req.audit).await?;
	admin_record::complete(udb, op_id, OpResult::RetentionView(view.clone())).await?;
	Ok(view)
}

pub async fn handle_clear_refcount(
	udb: Arc<universaldb::Database>,
	op_id: Uuid,
	req: ClearRefcountRequest,
) -> Result<ClearRefcountResult> {
	start_short_op(Arc::clone(&udb), op_id).await?;
	let result = clear_refcount_inner(Arc::clone(&udb), &req).await;
	if let Err(err) = result {
		fail_short_op(Arc::clone(&udb), op_id, &err).await?;
		return Err(err);
	}

	let result = ClearRefcountResult {
		kind: req.kind,
		txid: req.txid,
	};
	emit_clear_refcount_audit(&req);
	admin_record::complete(udb, op_id, OpResult::ClearRefcount(result.clone())).await?;
	Ok(result)
}

async fn start_short_op(udb: Arc<universaldb::Database>, op_id: Uuid) -> Result<()> {
	admin_record::update_status(udb, op_id, OpStatus::InProgress, None).await
}

async fn fail_short_op(
	udb: Arc<universaldb::Database>,
	op_id: Uuid,
	err: &anyhow::Error,
) -> Result<()> {
	let rivet_error = rivet_error::RivetError::extract(err);
	admin_record::fail(
		udb,
		op_id,
		OpResult::Message {
			message: format!("{}.{}", rivet_error.group(), rivet_error.code()),
		},
	)
	.await
}

async fn validate_retention_config(
	udb: Arc<universaldb::Database>,
	req: &SetRetentionRequest,
) -> Result<()> {
	let namespace_config = read_namespace_config(Arc::clone(&udb), req.audit.namespace_id).await?;
	if namespace_config.max_retention_ms > 0
		&& req.config.retention_ms > namespace_config.max_retention_ms
	{
		return Err(SqliteAdminError::RetentionWindowExceeded {
			oldest_reachable_txid: namespace_config.max_retention_ms,
		}
		.build());
	}
	Ok(())
}

async fn read_effective_retention(
	udb: Arc<universaldb::Database>,
	actor_id: String,
	audit: admin_record::AuditFields,
) -> Result<RetentionConfig> {
	udb.run(move |tx| {
		let actor_id = actor_id.clone();

		async move {
			let namespace_config = read_namespace_config_from_tx(&tx, audit.namespace_id).await?;
			read_effective_retention_from_tx(&tx, &actor_id, &namespace_config).await
		}
	})
	.await
}

async fn load_retention_view(
	udb: Arc<universaldb::Database>,
	actor_id: String,
	audit: admin_record::AuditFields,
) -> Result<RetentionView> {
	udb.run(move |tx| {
		let actor_id = actor_id.clone();

		async move {
			let namespace_config = read_namespace_config_from_tx(&tx, audit.namespace_id).await?;
			let retention_config =
				read_effective_retention_from_tx(&tx, &actor_id, &namespace_config).await?;
			let head = read_head_from_tx(&tx, &actor_id).await?;
			let checkpoints = read_checkpoints_from_tx(&tx, &actor_id).await?;
			let delta_metas = read_delta_metas_from_tx(&tx, &actor_id).await?;
			let live_ops = read_live_op_kinds_from_tx(&tx, &actor_id).await?;
			let storage_used_live_bytes =
				i64_to_u64(quota::read_live(&tx, &actor_id).await?, "live storage used")?;
			let storage_used_pitr_bytes =
				i64_to_u64(quota::read_pitr(&tx, &actor_id).await?, "pitr storage used")?;
			let checkpoint_views =
				build_checkpoint_views(&tx, &actor_id, checkpoints, &live_ops).await?;

			Ok(RetentionView {
				head: HeadView {
					head_txid: head.head_txid,
					db_size_pages: head.db_size_pages,
				},
				fine_grained_window: fine_grained_window(&checkpoint_views, &delta_metas, &head),
				checkpoints: checkpoint_views,
				retention_config,
				storage_used_live_bytes,
				storage_used_pitr_bytes,
				pitr_namespace_budget_bytes: namespace_config.pitr_namespace_budget_bytes,
				pitr_namespace_used_bytes: storage_used_pitr_bytes,
			})
		}
	})
	.await
}

async fn read_namespace_config(
	udb: Arc<universaldb::Database>,
	namespace_id: Uuid,
) -> Result<SqliteNamespaceConfig> {
	udb.run(move |tx| async move { read_namespace_config_from_tx(&tx, namespace_id).await })
		.await
}

async fn read_namespace_config_from_tx(
	tx: &universaldb::Transaction,
	namespace_id: Uuid,
) -> Result<SqliteNamespaceConfig> {
	let namespace_tx = tx.with_subspace(namespace::keys::subspace());
	Ok(namespace_tx
		.read_opt(
			&namespace::keys::sqlite_config_key(namespace_id_for_storage(namespace_id)),
			Serializable,
		)
		.await?
		.unwrap_or_default())
}

fn namespace_id_for_storage(namespace_id: Uuid) -> Id {
	Id::v1(namespace_id, 0)
}

async fn read_effective_retention_from_tx(
	tx: &universaldb::Transaction,
	actor_id: &str,
	namespace_config: &SqliteNamespaceConfig,
) -> Result<RetentionConfig> {
	Ok(match tx
		.informal()
		.get(&keys::meta_retention_key(actor_id), Snapshot)
		.await?
	{
		Some(value) => decode_retention_config(&value).context("decode sqlite retention config")?,
		None => RetentionConfig {
			retention_ms: namespace_config.default_retention_ms,
			checkpoint_interval_ms: namespace_config.default_checkpoint_interval_ms,
			max_checkpoints: namespace_config.default_max_checkpoints,
		},
	})
}

async fn read_head_from_tx(
	tx: &universaldb::Transaction,
	actor_id: &str,
) -> Result<DBHead> {
	Ok(match tx
		.informal()
		.get(&keys::meta_head_key(actor_id), Snapshot)
		.await?
	{
		Some(value) => decode_db_head(&value).context("decode sqlite head")?,
		None => DBHead {
			head_txid: 0,
			db_size_pages: 0,
			#[cfg(debug_assertions)]
			generation: 0,
		},
	})
}

async fn read_checkpoints_from_tx(
	tx: &universaldb::Transaction,
	actor_id: &str,
) -> Result<Checkpoints> {
	Ok(match tx
		.informal()
		.get(&keys::meta_checkpoints_key(actor_id), Snapshot)
		.await?
	{
		Some(value) => decode_checkpoints(&value).context("decode sqlite checkpoints")?,
		None => Checkpoints {
			entries: Vec::new(),
		},
	})
}

async fn read_delta_metas_from_tx(
	tx: &universaldb::Transaction,
	actor_id: &str,
) -> Result<BTreeMap<u64, DeltaMeta>> {
	let mut rows = BTreeMap::new();
	for (key, value) in tx_scan_prefix_values(tx, &keys::delta_prefix(actor_id)).await? {
		let txid = keys::decode_delta_chunk_txid(actor_id, &key)?;
		if key != keys::delta_meta_key(actor_id, txid) {
			continue;
		}
		rows.insert(txid, decode_delta_meta(&value).context("decode sqlite delta meta")?);
	}
	Ok(rows)
}

async fn read_live_op_kinds_from_tx(
	tx: &universaldb::Transaction,
	actor_id: &str,
) -> Result<Vec<OpKind>> {
	let mut op_kinds = Vec::new();
	for (_key, value) in tx_scan_prefix_values(tx, &keys::meta_admin_op_prefix(actor_id)).await? {
		let record = decode_admin_op_record(&value).context("decode sqlite admin op record")?;
		if record.actor_id == actor_id
			&& matches!(record.status, OpStatus::Pending | OpStatus::InProgress)
		{
			op_kinds.push(record.op_kind);
		}
	}
	Ok(op_kinds)
}

async fn build_checkpoint_views(
	tx: &universaldb::Transaction,
	actor_id: &str,
	checkpoints: Checkpoints,
	live_ops: &[OpKind],
) -> Result<Vec<CheckpointView>> {
	let mut views = Vec::with_capacity(checkpoints.entries.len());
	for entry in checkpoints.entries {
		let checkpoint_meta = tx
			.informal()
			.get(&keys::checkpoint_meta_key(actor_id, entry.ckp_txid), Snapshot)
			.await?
			.as_deref()
			.map(|value| decode_checkpoint_meta(value))
			.transpose()
			.context("decode sqlite checkpoint meta")?;
		let refcount = checkpoint_meta
			.as_ref()
			.map(|meta| meta.refcount)
			.unwrap_or(entry.refcount);
		let pinned_reason = pinned_reason(refcount, checkpoint_meta.and_then(|meta| meta.pinned_reason), live_ops);
		views.push(CheckpointView {
			ckp_txid: entry.ckp_txid,
			taken_at_ms: entry.taken_at_ms,
			byte_count: entry.byte_count,
			refcount,
			pinned_reason,
		});
	}
	views.sort_by_key(|view| view.ckp_txid);
	Ok(views)
}

fn pinned_reason(
	refcount: u32,
	stored_reason: Option<String>,
	live_ops: &[OpKind],
) -> Option<String> {
	if refcount == 0 {
		return None;
	}
	if live_ops.iter().any(|kind| *kind == OpKind::Fork) {
		return Some("fork in progress".to_string());
	}
	if live_ops.iter().any(|kind| *kind == OpKind::Restore) {
		return Some("restore in progress".to_string());
	}
	stored_reason.or_else(|| Some("refcount pinned".to_string()))
}

fn fine_grained_window(
	checkpoints: &[CheckpointView],
	delta_metas: &BTreeMap<u64, DeltaMeta>,
	head: &DBHead,
) -> Option<FineGrainedWindow> {
	let latest_ckp = checkpoints.iter().max_by_key(|checkpoint| checkpoint.ckp_txid)?;
	let mut matching = delta_metas
		.iter()
		.filter(|(txid, _meta)| **txid > latest_ckp.ckp_txid && **txid <= head.head_txid)
		.peekable();
	matching.peek()?;

	let mut from_txid = u64::MAX;
	let mut to_txid = 0;
	let mut from_taken_at_ms = i64::MAX;
	let mut to_taken_at_ms = i64::MIN;
	let mut delta_count = 0u64;
	let mut total_bytes = 0u64;
	for (txid, meta) in matching {
		from_txid = from_txid.min(*txid);
		to_txid = to_txid.max(*txid);
		from_taken_at_ms = from_taken_at_ms.min(meta.taken_at_ms);
		to_taken_at_ms = to_taken_at_ms.max(meta.taken_at_ms);
		delta_count += 1;
		total_bytes = total_bytes.saturating_add(meta.byte_count);
	}

	Some(FineGrainedWindow {
		from_txid,
		to_txid,
		from_taken_at_ms,
		to_taken_at_ms,
		delta_count,
		total_bytes,
	})
}

async fn clear_refcount_inner(
	udb: Arc<universaldb::Database>,
	req: &ClearRefcountRequest,
) -> Result<()> {
	let req = req.clone();
	udb.run(move |tx| {
		let req = req.clone();

		async move {
			match req.kind {
				RefcountKind::Checkpoint => {
					clear_checkpoint_refcount(&tx, &req.actor_id, req.txid).await
				}
				RefcountKind::Delta => clear_delta_refcount(&tx, &req.actor_id, req.txid).await,
			}
		}
	})
	.await
}

async fn clear_checkpoint_refcount(
	tx: &universaldb::Transaction,
	actor_id: &str,
	txid: u64,
) -> Result<()> {
	let meta_key = keys::checkpoint_meta_key(actor_id, txid);
	let Some(meta_bytes) = tx.informal().get(&meta_key, Serializable).await? else {
		return Err(SqliteAdminError::InvalidRestorePoint {
			target_txid: txid,
			reachable_hints: Vec::new(),
		}
		.build());
	};
	let mut meta = decode_checkpoint_meta(&meta_bytes).context("decode sqlite checkpoint meta")?;
	meta.refcount = 0;
	tx.informal()
		.set(&meta_key, &encode_checkpoint_meta(meta).context("encode sqlite checkpoint meta")?);

	let checkpoints_key = keys::meta_checkpoints_key(actor_id);
	if let Some(checkpoints_bytes) = tx.informal().get(&checkpoints_key, Serializable).await? {
		let mut checkpoints =
			decode_checkpoints(&checkpoints_bytes).context("decode sqlite checkpoints")?;
		for entry in &mut checkpoints.entries {
			if entry.ckp_txid == txid {
				entry.refcount = 0;
			}
		}
		tx.informal().set(
			&checkpoints_key,
			&encode_checkpoints(checkpoints).context("encode sqlite checkpoints")?,
		);
	}

	Ok(())
}

async fn clear_delta_refcount(
	tx: &universaldb::Transaction,
	actor_id: &str,
	txid: u64,
) -> Result<()> {
	let meta_key = keys::delta_meta_key(actor_id, txid);
	let Some(meta_bytes) = tx.informal().get(&meta_key, Serializable).await? else {
		return Err(SqliteAdminError::InvalidRestorePoint {
			target_txid: txid,
			reachable_hints: Vec::new(),
		}
		.build());
	};
	let mut meta = decode_delta_meta(&meta_bytes).context("decode sqlite delta meta")?;
	meta.refcount = 0;
	tx.informal()
		.set(&meta_key, &encode_delta_meta(meta).context("encode sqlite delta meta")?);
	Ok(())
}

fn emit_clear_refcount_audit(req: &ClearRefcountRequest) {
	tracing::info!(
		actor_id = %req.actor_id,
		kind = ?req.kind,
		txid = req.txid,
		caller_id = %req.audit.caller_id,
		namespace_id = %req.audit.namespace_id,
		"sqlite admin clear refcount"
	);
	#[cfg(debug_assertions)]
	test_hooks::record_clear_refcount(req);
}

async fn tx_scan_prefix_values(
	tx: &universaldb::Transaction,
	prefix: &[u8],
) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
	let informal = tx.informal();
	let prefix_subspace =
		universaldb::Subspace::from(universaldb::tuple::Subspace::from_bytes(prefix.to_vec()));
	let mut stream = informal.get_ranges_keyvalues(
		RangeOption {
			mode: StreamingMode::WantAll,
			..RangeOption::from(&prefix_subspace)
		},
		Snapshot,
	);
	let mut rows = Vec::new();

	while let Some(entry) = stream.try_next().await? {
		rows.push((entry.key().to_vec(), entry.value().to_vec()));
	}

	Ok(rows)
}

fn i64_to_u64(value: i64, name: &str) -> Result<u64> {
	u64::try_from(value).with_context(|| format!("sqlite {name} was negative"))
}

#[cfg(debug_assertions)]
pub mod test_hooks {
	use parking_lot::Mutex;

	use super::*;

	static CLEAR_REFCOUNT_AUDIT_LOG: Mutex<Vec<ClearRefcountAuditEntry>> = Mutex::new(Vec::new());

	#[derive(Clone, Debug, PartialEq, Eq)]
	pub struct ClearRefcountAuditEntry {
		pub actor_id: String,
		pub kind: RefcountKind,
		pub txid: u64,
		pub caller_id: String,
		pub namespace_id: Uuid,
	}

	pub(super) fn record_clear_refcount(req: &ClearRefcountRequest) {
		CLEAR_REFCOUNT_AUDIT_LOG
			.lock()
			.push(ClearRefcountAuditEntry {
				actor_id: req.actor_id.clone(),
				kind: req.kind,
				txid: req.txid,
				caller_id: req.audit.caller_id.clone(),
				namespace_id: req.audit.namespace_id,
			});
	}

	pub fn take_clear_refcount_audit_log() -> Vec<ClearRefcountAuditEntry> {
		std::mem::take(&mut *CLEAR_REFCOUNT_AUDIT_LOG.lock())
	}
}
