use std::{sync::Arc, time::SystemTime};

use anyhow::{Context, Result, bail};
use futures_util::TryStreamExt;
use rivet_pools::NodeId;
use universaldb::{
	options::StreamingMode,
	RangeOption, tuple,
	utils::IsolationLevel::Serializable,
};
use uuid::Uuid;

use crate::{
	admin::types::{
		AdminOpRecord, AuditFields, OpKind, OpProgress, OpResult, OpStatus,
		decode_admin_op_record, encode_admin_op_record,
	},
	pump::keys,
};

pub async fn create_record(
	udb: Arc<universaldb::Database>,
	op_id: Uuid,
	op_kind: OpKind,
	actor_id: String,
	audit: AuditFields,
) -> Result<()> {
	let now_ms = now_ms()?;
	udb.run(move |tx| {
		let actor_id = actor_id.clone();
		let audit = audit.clone();

		async move {
			let record = AdminOpRecord {
				operation_id: op_id,
				op_kind,
				actor_id: actor_id.clone(),
				created_at_ms: now_ms,
				last_progress_at_ms: now_ms,
				status: OpStatus::Pending,
				holder_id: None,
				progress: None,
				result: None,
				audit,
			};
			let key = keys::meta_admin_op_key(&actor_id, op_id);
			tx.informal()
				.set(&key, &encode_admin_op_record(record).context("encode sqlite admin op")?);
			Ok(())
		}
	})
	.await
}

pub async fn update_status(
	udb: Arc<universaldb::Database>,
	op_id: Uuid,
	status: OpStatus,
	holder: Option<NodeId>,
) -> Result<()> {
	let now_ms = now_ms()?;
	udb.run(move |tx| async move {
		let (key, mut record) = read_record_for_update(&tx, op_id).await?;
		validate_transition(record.status, status)?;
		record.status = status;
		record.holder_id = if is_terminal(status) { None } else { holder };
		record.last_progress_at_ms = bumped_at(record.last_progress_at_ms, now_ms);
		tx.informal()
			.set(&key, &encode_admin_op_record(record).context("encode sqlite admin op")?);
		Ok(())
	})
	.await
}

pub async fn start_work(
	udb: Arc<universaldb::Database>,
	op_id: Uuid,
	holder: NodeId,
) -> Result<Option<AdminOpRecord>> {
	let now_ms = now_ms()?;
	udb.run(move |tx| async move {
		let Some((key, mut record)) = find_record(&tx, op_id).await? else {
			return Ok(None);
		};

		match record.status {
			OpStatus::Pending => {
				record.status = OpStatus::InProgress;
				record.holder_id = Some(holder);
				record.last_progress_at_ms = bumped_at(record.last_progress_at_ms, now_ms);
				tx.informal().set(
					&key,
					&encode_admin_op_record(record.clone()).context("encode sqlite admin op")?,
				);
				Ok(Some(record))
			}
			OpStatus::InProgress => Ok(Some(record)),
			OpStatus::Completed | OpStatus::Failed | OpStatus::Orphaned => Ok(None),
		}
	})
	.await
}

pub async fn update_progress(
	udb: Arc<universaldb::Database>,
	op_id: Uuid,
	progress: OpProgress,
) -> Result<()> {
	let now_ms = now_ms()?;
	udb.run(move |tx| {
		let progress = progress.clone();

		async move {
			let (key, mut record) = read_record_for_update(&tx, op_id).await?;
			if is_terminal(record.status) {
				bail!(
					"cannot update sqlite admin op progress after terminal status {:?}",
					record.status
				);
			}
			record.progress = Some(progress);
			record.last_progress_at_ms = bumped_at(record.last_progress_at_ms, now_ms);
			tx.informal()
				.set(&key, &encode_admin_op_record(record).context("encode sqlite admin op")?);
			Ok(())
		}
	})
	.await
}

pub async fn complete(
	udb: Arc<universaldb::Database>,
	op_id: Uuid,
	result: OpResult,
) -> Result<()> {
	let now_ms = now_ms()?;
	udb.run(move |tx| {
		let result = result.clone();

		async move {
			let (key, mut record) = read_record_for_update(&tx, op_id).await?;
			validate_transition(record.status, OpStatus::Completed)?;
			record.status = OpStatus::Completed;
			record.holder_id = None;
			record.result = Some(result);
			record.last_progress_at_ms = bumped_at(record.last_progress_at_ms, now_ms);
			tx.informal()
				.set(&key, &encode_admin_op_record(record).context("encode sqlite admin op")?);
			Ok(())
		}
	})
	.await
}

pub async fn read(
	udb: Arc<universaldb::Database>,
	op_id: Uuid,
) -> Result<Option<AdminOpRecord>> {
	udb.run(move |tx| async move {
		Ok(match find_record(&tx, op_id).await? {
			Some((_key, record)) => Some(record),
			None => None,
		})
	})
	.await
}

async fn read_record_for_update(
	tx: &universaldb::Transaction,
	op_id: Uuid,
) -> Result<(Vec<u8>, AdminOpRecord)> {
	find_record(tx, op_id)
		.await?
		.with_context(|| format!("sqlite admin op record not found: {op_id}"))
}

async fn find_record(
	tx: &universaldb::Transaction,
	op_id: Uuid,
) -> Result<Option<(Vec<u8>, AdminOpRecord)>> {
	let informal = tx.informal();
	let sqlite_subspace =
		universaldb::Subspace::from(tuple::Subspace::from_bytes(vec![keys::SQLITE_SUBSPACE_PREFIX]));
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

		let record = decode_admin_op_record(entry.value())
			.context("decode sqlite admin op record")?;
		if record.operation_id == op_id {
			return Ok(Some((key.to_vec(), record)));
		}
	}

	Ok(None)
}

fn key_has_admin_op_suffix(key: &[u8]) -> bool {
	key.windows(b"/META/admin_op/".len())
		.any(|window| window == b"/META/admin_op/")
}

fn validate_transition(from: OpStatus, to: OpStatus) -> Result<()> {
	if from == to {
		return Ok(());
	}

	let valid = match from {
		OpStatus::Pending => matches!(to, OpStatus::InProgress | OpStatus::Orphaned),
		OpStatus::InProgress => {
			matches!(
				to,
				OpStatus::Completed | OpStatus::Failed | OpStatus::Orphaned
			)
		}
		OpStatus::Completed | OpStatus::Failed | OpStatus::Orphaned => false,
	};

	if valid {
		Ok(())
	} else {
		bail!("invalid sqlite admin op status transition: {from:?} -> {to:?}")
	}
}

fn is_terminal(status: OpStatus) -> bool {
	matches!(
		status,
		OpStatus::Completed | OpStatus::Failed | OpStatus::Orphaned
	)
}

fn bumped_at(previous: i64, now_ms: i64) -> i64 {
	now_ms.max(previous.saturating_add(1))
}

fn now_ms() -> Result<i64> {
	let elapsed = SystemTime::now()
		.duration_since(SystemTime::UNIX_EPOCH)
		.context("system clock was before unix epoch")?;
	i64::try_from(elapsed.as_millis()).context("sqlite admin op timestamp exceeded i64")
}
