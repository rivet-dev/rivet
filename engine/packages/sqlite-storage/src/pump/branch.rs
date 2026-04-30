use anyhow::{Context, Result};
use universaldb::{options::MutationType, utils::IsolationLevel};

use super::{
	keys, udb,
	types::{
		ActorBranchId, BranchState, NamespaceBranchId, NamespaceBranchRecord, NamespaceId,
		NamespacePointer, NamespaceTierState, Tier, decode_actor_pointer, decode_namespace_pointer,
		encode_namespace_branch_record, encode_namespace_pointer, encode_namespace_tier_state,
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
