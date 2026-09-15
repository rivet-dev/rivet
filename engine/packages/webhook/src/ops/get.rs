use epoxy_protocol::protocol::CachingBehavior;
use gas::prelude::*;
use universaldb::prelude::FormalKey;

use crate::{keys, types::WebhookConfig};

#[derive(Debug)]
pub struct Input {
	pub namespace_id: Id,
	pub name: String,
}

#[operation]
pub async fn webhook_config_get(
	ctx: &OperationCtx,
	input: &Input,
) -> Result<Option<WebhookConfig>> {
	let global_key = keys::GlobalDataKey::new(input.namespace_id, input.name.clone());

	let res = ctx
		.op(epoxy::ops::kv::get_optimistic::Input {
			replica_id: ctx.config().epoxy_replica_id(),
			key: namespace::keys::subspace().pack(&global_key),
			caching_behavior: CachingBehavior::Optimistic,
			target_replicas: None,
			save_empty: false,
		})
		.await?;

	res.value
		.map(|raw| global_key.deserialize(&raw))
		.transpose()
}
