use futures_util::{StreamExt, TryStreamExt};
use gas::prelude::*;
use universaldb::utils::{FormalKey, IsolationLevel::*};

use crate::keys;

#[derive(Debug)]
pub struct Input {
	pub actor_id: Id,
}

#[derive(Debug)]
pub struct Output {
	pub name: String,
	pub runner_id: Id,
	pub is_connectable: bool,
}

// TODO: Add cache (remember to purge cache when runner changes)
#[operation]
pub async fn pegboard_actor_get_for_runner(
	ctx: &OperationCtx,
	input: &Input,
) -> Result<Option<Output>> {
	let (name, runner_id, is_connectable) = ctx
		.udb()?
		.run(|tx| async move {
			let tx = tx.with_subspace(keys::subspace());

			let name_key = keys::actor::NameKey::new(actor_id);
			let runner_id_key = keys::actor::RunnerIdKey::new(actor_id);
			let connectable_key = keys::actor::ConnectableKey::new(actor_id);

			let (name_entry, runner_id_entry, is_connectable) = tokio::try_join!(
				tx.read_opt(&name_key, Serializable),
				tx.read_opt(&runner_id_key, Serializable),
				tx.exists(&connectable_key, Serializable),
			)?;

			let (Some(name), Some(runner_id)) = (name_entry, runner_id_entry) else {
				return Ok(None);
			};

			Ok(Some(Actor {
				name,
				runner_id,
				is_connectable,
			}))
		})
		.custom_instrument(tracing::info_span!("actor_get_for_runner_tx"))
		.await?;

	// NOTE: The name key was added via backfill. If the actor has not backfilled the key yet (key is none),
	// we need to fetch it from the actor state
	let name = if let Some(name) = name {
		name
	} else {
		let wfs = ctx
			.get_workflows(
				actors_with_wf_ids
					.iter()
					.map(|(_, workflow_id)| *workflow_id)
					.collect(),
			)
			.await?;

		let Some(wf) = wfs.first() else {
			return Ok(None);
		};

		let actor_state = match wf.parse_state::<Option<crate::workflows::actor::State>>() {
			Ok(Some(s)) => s,
			Ok(None) => {
				// Actor did not initialize state yet
				return Ok(None);
			}
			Err(err) => {
				tracing::error!(?actor_id, ?workflow_id, ?err, "failed to parse wf state");
				return Ok(None);
			}
		};

		actor_state.name
	};

	Ok(Some(Actor {
		name,
		runner_id,
		is_connectable,
	}))
}
