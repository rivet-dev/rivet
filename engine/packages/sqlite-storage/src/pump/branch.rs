use anyhow::{Context, Result};
use universaldb::utils::IsolationLevel;

use super::{
	keys,
	types::{ActorBranchId, NamespaceBranchId, decode_actor_pointer},
};

pub fn root_namespace_branch_id() -> NamespaceBranchId {
	NamespaceBranchId::nil()
}

pub async fn resolve_actor_branch(
	tx: &universaldb::Transaction,
	actor_id: &str,
	isolation_level: IsolationLevel,
) -> Result<Option<ActorBranchId>> {
	let Some(pointer_bytes) = tx
		.informal()
		.get(
			&keys::actor_pointer_cur_key(root_namespace_branch_id(), actor_id),
			isolation_level,
		)
		.await?
	else {
		return Ok(None);
	};

	let pointer = decode_actor_pointer(&pointer_bytes).context("decode sqlite actor pointer")?;
	Ok(Some(pointer.current_branch))
}
