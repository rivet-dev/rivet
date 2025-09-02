use gas::prelude::*;
use rivet_key_data::converted::ActorByKeyKeyData;
use rivet_runner_protocol::protocol;
use udb_util::{SERIALIZABLE, TxnExt};
use universaldb as udb;

use super::{DestroyComplete, DestroyStarted, State};

use crate::keys;

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Input {
	pub namespace_id: Id,
	pub actor_id: Id,
	pub name: String,
	pub key: Option<String>,
	pub generation: u32,
	/// Whether or not to send signals to the pb actor. In the case that the actor was already stopped
	/// or exited, signals are unnecessary.
	pub kill: bool,
}

#[workflow]
pub(crate) async fn pegboard_actor_destroy(ctx: &mut WorkflowCtx, input: &Input) -> Result<()> {
	ctx.msg(DestroyStarted {})
		.tag("actor_id", input.actor_id)
		.send()
		.await?;

	let res = ctx
		.activity(UpdateStateAndFdbInput {
			actor_id: input.actor_id,
		})
		.await?;

	// Destroy actor
	if let (Some(runner_workflow_id), true) = (res.runner_workflow_id, &input.kill) {
		kill(ctx, input.actor_id, input.generation, runner_workflow_id).await?;
	}

	// Clear KV
	ctx.activity(ClearKvInput {
		actor_id: input.actor_id,
	})
	.await?;

	ctx.msg(DestroyComplete {})
		.tag("actor_id", input.actor_id)
		.send()
		.await?;

	Ok(())
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct UpdateStateAndFdbInput {
	actor_id: Id,
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct UpdateStateAndFdbOutput {
	runner_workflow_id: Option<Id>,
}

#[activity(UpdateStateAndFdb)]
async fn update_state_and_fdb(
	ctx: &ActivityCtx,
	input: &UpdateStateAndFdbInput,
) -> Result<UpdateStateAndFdbOutput> {
	let mut state = ctx.state::<State>()?;
	let destroy_ts = util::timestamp::now();

	ctx.udb()?
		.run(|tx, _mc| {
			let state = (*state).clone();

			async move {
				let txs = tx.subspace(keys::subspace());

				txs.write(&keys::actor::DestroyTsKey::new(input.actor_id), destroy_ts)?;

				if let Some(runner_id) = state.runner_id {
					clear_slot(
						input.actor_id,
						state.namespace_id,
						&state.runner_name_selector,
						runner_id,
						&tx,
					)
					.await?;
				}

				// Update namespace indexes
				txs.delete(&keys::ns::ActiveActorKey::new(
					state.namespace_id,
					state.name.clone(),
					state.create_ts,
					input.actor_id,
				));

				if let Some(k) = &state.key {
					txs.write(
						&keys::ns::ActorByKeyKey::new(
							state.namespace_id,
							state.name.clone(),
							k.clone(),
							state.create_ts,
							input.actor_id,
						),
						ActorByKeyKeyData {
							workflow_id: ctx.workflow_id(),
							is_destroyed: true,
						},
					)?;
				}

				Ok(())
			}
		})
		.custom_instrument(tracing::info_span!("actor_destroy_tx"))
		.await?;

	state.destroy_ts = Some(destroy_ts);
	state.runner_id = None;
	let runner_workflow_id = state.runner_workflow_id.take();

	Ok(UpdateStateAndFdbOutput { runner_workflow_id })
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct ClearKvInput {
	actor_id: Id,
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct ClearKvOutput {
	final_size: i64,
}

#[activity(ClearKv)]
async fn clear_kv(ctx: &ActivityCtx, input: &ClearKvInput) -> Result<ClearKvOutput> {
	// Matches `delete_all` from actor_kv (can't import because of cyclical dep)
	let final_size = ctx
		.udb()?
		.run(|tx, _mc| async move {
			let subspace = keys::actor_kv_subspace().subspace(&input.actor_id);

			let (start, end) = subspace.range();
			let final_size = tx.get_estimated_range_size_bytes(&start, &end).await?;

			tx.clear_subspace_range(&subspace);

			Ok(final_size)
		})
		.await?;

	Ok(ClearKvOutput { final_size })
}

pub(crate) async fn clear_slot(
	actor_id: Id,
	namespace_id: Id,
	runner_name_selector: &str,
	runner_id: Id,
	tx: &udb::RetryableTransaction,
) -> Result<(), udb::FdbBindingError> {
	let txs = tx.subspace(keys::subspace());

	txs.delete(&keys::actor::RunnerIdKey::new(actor_id));

	// This is cleared when the state changes as well as when the actor is destroyed to ensure
	// consistency during rescheduling and forced deletion.
	txs.delete(&keys::runner::ActorKey::new(runner_id, actor_id));

	let runner_workflow_id_key = keys::runner::WorkflowIdKey::new(runner_id);
	let runner_version_key = keys::runner::VersionKey::new(runner_id);
	let runner_remaining_slots_key = keys::runner::RemainingSlotsKey::new(runner_id);
	let runner_total_slots_key = keys::runner::TotalSlotsKey::new(runner_id);
	let runner_last_ping_ts_key = keys::runner::LastPingTsKey::new(runner_id);

	let (
		runner_workflow_id,
		runner_version,
		runner_remaining_slots,
		runner_total_slots,
		runner_last_ping_ts,
	) = tokio::try_join!(
		txs.read(&runner_workflow_id_key, SERIALIZABLE),
		txs.read(&runner_version_key, SERIALIZABLE),
		txs.read(&runner_remaining_slots_key, SERIALIZABLE),
		txs.read(&runner_total_slots_key, SERIALIZABLE),
		txs.read(&runner_last_ping_ts_key, SERIALIZABLE),
	)?;

	let old_runner_remaining_millislots = (runner_remaining_slots * 1000) / runner_total_slots;
	let new_runner_remaining_slots = runner_remaining_slots + 1;

	// Write new remaining slots
	txs.write(&runner_remaining_slots_key, new_runner_remaining_slots)?;

	let old_runner_alloc_key = keys::datacenter::RunnerAllocIdxKey::new(
		namespace_id,
		runner_name_selector.to_string(),
		runner_version,
		old_runner_remaining_millislots,
		runner_last_ping_ts,
		runner_id,
	);

	// Only update allocation idx if it existed before
	if txs.exists(&old_runner_alloc_key, SERIALIZABLE).await? {
		// Clear old key
		txs.delete(&old_runner_alloc_key);

		let new_remaining_millislots = (new_runner_remaining_slots * 1000) / runner_total_slots;
		let new_runner_alloc_key = keys::datacenter::RunnerAllocIdxKey::new(
			namespace_id,
			runner_name_selector.to_string(),
			runner_version,
			new_remaining_millislots,
			runner_last_ping_ts,
			runner_id,
		);

		txs.write(
			&new_runner_alloc_key,
			rivet_key_data::converted::RunnerAllocIdxKeyData {
				workflow_id: runner_workflow_id,
				remaining_slots: new_runner_remaining_slots,
				total_slots: runner_total_slots,
			},
		)?;
	}

	Ok(())
}

pub(crate) async fn kill(
	ctx: &mut WorkflowCtx,
	actor_id: Id,
	generation: u32,
	runner_workflow_id: Id,
) -> Result<()> {
	ctx.signal(protocol::Command::StopActor {
		actor_id,
		generation,
	})
	.to_workflow_id(runner_workflow_id)
	.send()
	.await?;

	Ok(())
}
