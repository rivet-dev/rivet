use gas::prelude::*;
use rivet_types::runner_configs::RunnerConfig;
use universaldb::prelude::FormalKey;

use crate::keys;

#[derive(Debug)]
pub struct Input {
	pub namespace_id: Id,
	pub name: String,
}

#[operation]
pub async fn namespace_runner_config_get_default(
	ctx: &OperationCtx,
	input: &Input,
) -> Result<Option<RunnerConfig>> {
	let key = keys::runner_config::DefaultKey::new(input.namespace_id, input.name.clone());
	let key_packed = keys::subspace().pack(&key);

	let data = ctx
		.op(epoxy::ops::kv::get_optimistic::Input {
			replica_id: ctx.config().epoxy_replica_id(),
			key: key_packed,
		})
		.await?;

	let Some(value) = data.value else {
		return Ok(None);
	};

	Ok(Some(key.deserialize(&value)?))
}
