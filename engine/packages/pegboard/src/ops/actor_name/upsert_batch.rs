use anyhow::Result;
use gas::prelude::*;
use rivet_data::converted::ActorNameKeyData;

use crate::keys;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Input {
	pub namespace_id: Id,
	/// Actor names with their metadata. Metadata must be a JSON object.
	pub actor_names: Vec<ActorNameEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActorNameEntry {
	pub name: String,
	pub metadata: serde_json::Map<String, serde_json::Value>,
}

#[operation]
pub async fn pegboard_actor_name_upsert_batch(ctx: &OperationCtx, input: &Input) -> Result<()> {
	if input.actor_names.is_empty() {
		return Ok(());
	}

	ctx.udb()?
		.run(|tx| {
			let actor_names = input.actor_names.clone();
			let namespace_id = input.namespace_id;
			async move {
				let tx = tx.with_subspace(keys::subspace());

				for entry in actor_names {
					tx.write(
						&keys::ns::ActorNameKey::new(namespace_id, entry.name),
						ActorNameKeyData {
							metadata: entry.metadata,
						},
					)?;
				}

				Ok(())
			}
		})
		.custom_instrument(tracing::info_span!("actor_name_upsert_batch_tx"))
		.await?;

	Ok(())
}
