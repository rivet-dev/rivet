use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use universaldb::{
	options::MutationType,
	utils::IsolationLevel::{self, Serializable},
};

use super::{
	constants::{MAX_FORK_DEPTH, MAX_NAMESPACE_DEPTH},
	error::SqliteStorageError,
	keys, udb,
	types::{
		ActorBranchId, ActorBranchRecord, BookmarkRef, BranchState, DBHead, NamespaceBranchId,
		NamespaceBranchRecord, NamespaceId, NamespacePointer, NamespaceTierState, Tier,
		decode_actor_branch_record, decode_actor_pointer, decode_commit_row,
		decode_namespace_branch_record, decode_namespace_pointer, encode_actor_branch_record,
		encode_db_head, encode_namespace_branch_record, encode_namespace_pointer,
		encode_namespace_tier_state,
	},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NamespaceBranchResolution {
	pub branch_id: NamespaceBranchId,
	pub initialized: bool,
}

pub async fn resolve_or_allocate_root_namespace_branch(
	tx: &universaldb::Transaction,
	namespace_id: NamespaceId,
) -> Result<NamespaceBranchResolution> {
	if let Some(branch_id) =
		resolve_namespace_branch(tx, namespace_id, IsolationLevel::Serializable).await?
	{
		return Ok(NamespaceBranchResolution {
			branch_id,
			initialized: false,
		});
	}

	Ok(NamespaceBranchResolution {
		branch_id: NamespaceBranchId::new_v4(),
		initialized: true,
	})
}

pub async fn resolve_namespace_branch(
	tx: &universaldb::Transaction,
	namespace_id: NamespaceId,
	isolation_level: IsolationLevel,
) -> Result<Option<NamespaceBranchId>> {
	let Some(pointer_bytes) = tx
		.informal()
		.get(&keys::namespace_pointer_cur_key(namespace_id), isolation_level)
		.await?
	else {
		return Ok(None);
	};

	let pointer = decode_namespace_pointer(&pointer_bytes).context("decode sqlite namespace pointer")?;
	Ok(Some(pointer.current_branch))
}

pub fn write_root_namespace_metadata(
	tx: &universaldb::Transaction,
	namespace_id: NamespaceId,
	branch_id: NamespaceBranchId,
	now_ms: i64,
	root_versionstamp: &[u8; 16],
) -> Result<()> {
	let record = NamespaceBranchRecord {
		branch_id,
		parent: None,
		parent_versionstamp: None,
		root_versionstamp: *root_versionstamp,
		fork_depth: 0,
		created_at_ms: now_ms,
		created_from_bookmark: None,
		state: BranchState::Live,
	};
	let encoded_record =
		encode_namespace_branch_record(record).context("encode sqlite root namespace branch record")?;
	let versionstamped_record = udb::append_versionstamp_offset(encoded_record, root_versionstamp)
		.context("prepare versionstamped sqlite root namespace branch record")?;
	tx.informal().atomic_op(
		&keys::namespace_branches_list_key(branch_id),
		&versionstamped_record,
		MutationType::SetVersionstampedValue,
	);
	tx.informal().atomic_op(
		&keys::namespace_branches_refcount_key(branch_id),
		&1_i64.to_le_bytes(),
		MutationType::Add,
	);

	let pointer = NamespacePointer {
		current_branch: branch_id,
		last_swapped_at_ms: now_ms,
	};
	let encoded_pointer =
		encode_namespace_pointer(pointer).context("encode sqlite namespace pointer")?;
	tx.informal()
		.set(&keys::namespace_pointer_cur_key(namespace_id), &encoded_pointer);

	let tier_state = NamespaceTierState {
		tier: Tier::T0,
		promoted_at_versionstamp: *root_versionstamp,
	};
	let encoded_tier_state =
		encode_namespace_tier_state(tier_state).context("encode sqlite namespace tier state")?;
	let versionstamped_tier_state =
		udb::append_versionstamp_offset(encoded_tier_state, root_versionstamp)
			.context("prepare versionstamped sqlite namespace tier state")?;
	tx.informal().atomic_op(
		&keys::namespace_branches_tier_state_key(branch_id),
		&versionstamped_tier_state,
		MutationType::SetVersionstampedValue,
	);

	Ok(())
}

pub async fn resolve_actor_branch(
	tx: &universaldb::Transaction,
	namespace_id: NamespaceId,
	actor_id: &str,
	isolation_level: IsolationLevel,
) -> Result<Option<ActorBranchId>> {
	let Some(namespace_branch_id) =
		resolve_namespace_branch(tx, namespace_id, isolation_level).await?
	else {
		return resolve_actor_branch_in_namespace(
			tx,
			NamespaceBranchId::nil(),
			actor_id,
			isolation_level,
		)
		.await;
	};

	if let Some(branch_id) =
		resolve_actor_branch_in_namespace(tx, namespace_branch_id, actor_id, isolation_level).await?
	{
		return Ok(Some(branch_id));
	}

	resolve_actor_branch_in_namespace(tx, NamespaceBranchId::nil(), actor_id, isolation_level).await
}

pub async fn resolve_actor_branch_in_namespace(
	tx: &universaldb::Transaction,
	namespace_branch_id: NamespaceBranchId,
	actor_id: &str,
	isolation_level: IsolationLevel,
) -> Result<Option<ActorBranchId>> {
	let Some(pointer_bytes) = tx
		.informal()
		.get(
			&keys::actor_pointer_cur_key(namespace_branch_id, actor_id),
			isolation_level,
		)
		.await?
	else {
		return Ok(None);
	};

	let pointer = decode_actor_pointer(&pointer_bytes).context("decode sqlite actor pointer")?;
	Ok(Some(pointer.current_branch))
}

pub async fn derive_branch_at(
	tx: &universaldb::Transaction,
	source_branch_id: ActorBranchId,
	at_versionstamp: [u8; 16],
	new_branch_id: ActorBranchId,
	namespace_branch: NamespaceBranchId,
	bookmark_ref: Option<BookmarkRef>,
) -> Result<()> {
	let source = read_actor_branch_record(tx, source_branch_id).await?;
	if source.fork_depth >= MAX_FORK_DEPTH {
		return Err(SqliteStorageError::ForkChainTooDeep.into());
	}

	let bk_pin = read_versionstamp_pin(tx, &keys::branches_bk_pin_key(source_branch_id)).await?;
	if bk_pin > at_versionstamp {
		return Err(SqliteStorageError::ForkOutOfRetention.into());
	}

	let txid_at_versionstamp = lookup_txid_at_versionstamp(tx, source_branch_id, at_versionstamp)
		.await
		.with_context(|| {
			format!(
				"lookup sqlite VTX entry for actor branch {}",
				source_branch_id.as_uuid()
			)
		})?;
	let commit_at_versionstamp = read_commit_row(tx, source_branch_id, txid_at_versionstamp)
		.await
		.with_context(|| {
			format!(
				"read sqlite commit row {txid_at_versionstamp} for actor branch {}",
				source_branch_id.as_uuid()
			)
		})?;
	let head_at_fork = DBHead {
		head_txid: txid_at_versionstamp,
		db_size_pages: commit_at_versionstamp.db_size_pages,
		post_apply_checksum: commit_at_versionstamp.post_apply_checksum,
		branch_id: new_branch_id,
		#[cfg(debug_assertions)]
		generation: 0,
	};
	let encoded_head_at_fork =
		encode_db_head(head_at_fork).context("encode sqlite fork head snapshot")?;
	tx.informal().set(
		&keys::branch_meta_head_at_fork_key(new_branch_id),
		&encoded_head_at_fork,
	);

	let new_record = ActorBranchRecord {
		branch_id: new_branch_id,
		namespace_branch,
		parent: Some(source_branch_id),
		parent_versionstamp: Some(at_versionstamp),
		root_versionstamp: at_versionstamp,
		fork_depth: source.fork_depth + 1,
		created_at_ms: now_ms()?,
		created_from_bookmark: bookmark_ref,
		state: BranchState::Live,
	};
	let encoded_record =
		encode_actor_branch_record(new_record).context("encode sqlite derived actor branch record")?;
	tx.informal()
		.set(&keys::branches_list_key(new_branch_id), &encoded_record);
	tx.informal().atomic_op(
		&keys::branches_refcount_key(source_branch_id),
		&1_i64.to_le_bytes(),
		MutationType::Add,
	);
	tx.informal().atomic_op(
		&keys::branches_refcount_key(new_branch_id),
		&1_i64.to_le_bytes(),
		MutationType::Add,
	);
	tx.informal().atomic_op(
		&keys::branches_desc_pin_key(source_branch_id),
		&at_versionstamp,
		MutationType::ByteMin,
	);

	Ok(())
}

pub async fn derive_namespace_branch_at(
	tx: &universaldb::Transaction,
	source_branch_id: NamespaceBranchId,
	at_versionstamp: [u8; 16],
	new_branch_id: NamespaceBranchId,
	bookmark_ref: Option<BookmarkRef>,
) -> Result<()> {
	let source = read_namespace_branch_record(tx, source_branch_id).await?;
	if source.fork_depth >= MAX_NAMESPACE_DEPTH {
		return Err(SqliteStorageError::NamespaceForkChainTooDeep.into());
	}

	let bk_pin =
		read_versionstamp_pin(tx, &keys::namespace_branches_bk_pin_key(source_branch_id)).await?;
	if bk_pin > at_versionstamp {
		return Err(SqliteStorageError::ForkOutOfRetention.into());
	}

	let new_record = NamespaceBranchRecord {
		branch_id: new_branch_id,
		parent: Some(source_branch_id),
		parent_versionstamp: Some(at_versionstamp),
		root_versionstamp: at_versionstamp,
		fork_depth: source.fork_depth + 1,
		created_at_ms: now_ms()?,
		created_from_bookmark: bookmark_ref,
		state: BranchState::Live,
	};
	let encoded_record = encode_namespace_branch_record(new_record)
		.context("encode sqlite derived namespace branch record")?;
	tx.informal()
		.set(&keys::namespace_branches_list_key(new_branch_id), &encoded_record);
	tx.informal().atomic_op(
		&keys::namespace_branches_refcount_key(source_branch_id),
		&1_i64.to_le_bytes(),
		MutationType::Add,
	);
	tx.informal().atomic_op(
		&keys::namespace_branches_refcount_key(new_branch_id),
		&1_i64.to_le_bytes(),
		MutationType::Add,
	);
	tx.informal().atomic_op(
		&keys::namespace_branches_desc_pin_key(source_branch_id),
		&at_versionstamp,
		MutationType::ByteMin,
	);

	Ok(())
}

async fn read_actor_branch_record(
	tx: &universaldb::Transaction,
	branch_id: ActorBranchId,
) -> Result<ActorBranchRecord> {
	let bytes = tx
		.informal()
		.get(&keys::branches_list_key(branch_id), Serializable)
		.await?
		.context("sqlite actor branch record is missing")?;

	decode_actor_branch_record(&bytes).context("decode sqlite actor branch record")
}

async fn read_namespace_branch_record(
	tx: &universaldb::Transaction,
	branch_id: NamespaceBranchId,
) -> Result<NamespaceBranchRecord> {
	let bytes = tx
		.informal()
		.get(&keys::namespace_branches_list_key(branch_id), Serializable)
		.await?
		.context("sqlite namespace branch record is missing")?;

	decode_namespace_branch_record(&bytes).context("decode sqlite namespace branch record")
}

async fn read_versionstamp_pin(
	tx: &universaldb::Transaction,
	key: &[u8],
) -> Result<[u8; 16]> {
	let Some(bytes) = tx.informal().get(key, Serializable).await? else {
		return Ok([0; 16]);
	};
	let bytes = Vec::<u8>::from(bytes);
	bytes
		.as_slice()
		.try_into()
		.context("sqlite branch pin should be exactly 16 bytes")
}

async fn lookup_txid_at_versionstamp(
	tx: &universaldb::Transaction,
	branch_id: ActorBranchId,
	versionstamp: [u8; 16],
) -> Result<u64> {
	let bytes = tx
		.informal()
		.get(&keys::branch_vtx_key(branch_id, versionstamp), Serializable)
		.await?
		.ok_or(SqliteStorageError::BookmarkExpired)?;
	let bytes = Vec::<u8>::from(bytes);
	let bytes: [u8; 8] = bytes
		.as_slice()
		.try_into()
		.context("sqlite VTX entry should be exactly 8 bytes")?;

	Ok(u64::from_be_bytes(bytes))
}

async fn read_commit_row(
	tx: &universaldb::Transaction,
	branch_id: ActorBranchId,
	txid: u64,
) -> Result<super::types::CommitRow> {
	let bytes = tx
		.informal()
		.get(&keys::branch_commit_key(branch_id, txid), Serializable)
		.await?
		.ok_or(SqliteStorageError::BookmarkExpired)?;

	decode_commit_row(&bytes).context("decode sqlite commit row")
}

fn now_ms() -> Result<i64> {
	let millis = SystemTime::now()
		.duration_since(UNIX_EPOCH)
		.context("system clock is before unix epoch")?
		.as_millis();
	i64::try_from(millis).context("current timestamp exceeded i64 milliseconds")
}
