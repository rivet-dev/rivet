use anyhow::Result;
use gas::prelude::*;

#[derive(Debug, Serialize, Deserialize, Hash)]
pub(crate) struct ApplyPatchInput {
	pub actor_id: Id,
	pub patch: Vec<crate::actor_metadata::PatchEntry>,
}

#[activity(ApplyMetadataPatch)]
pub(crate) async fn apply_patch(ctx: &ActivityCtx, input: &ApplyPatchInput) -> Result<()> {
	crate::actor_metadata::apply_patch(&*ctx.udb()?, input.actor_id, &input.patch).await?;
	Ok(())
}

pub(crate) fn protocol_patch_entry_to_storage(
	entry: rivet_runner_protocol::mk2::MetadataPatchEntry,
) -> crate::actor_metadata::PatchEntry {
	crate::actor_metadata::PatchEntry {
		key: entry.key,
		value: entry.value,
	}
}
