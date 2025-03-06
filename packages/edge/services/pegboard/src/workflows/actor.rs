use chirp_workflow::prelude::*;
use fdb_util::FormalKey;
use foundationdb as fdb;
use futures_util::FutureExt;

use crate::{
	keys,
	protocol::{self, ActorState::*},
};

/// How long to wait after creating and not receiving a starting state before setting actor as lost.
const ACTOR_START_THRESHOLD_MS: i64 = util::duration::seconds(30);
/// How long to wait after stopping and not receiving a stop state before setting actor as lost.
const ACTOR_STOP_THRESHOLD_MS: i64 = util::duration::seconds(30);
/// How long to wait after stopped and not receiving an exit state before setting actor as lost.
const ACTOR_EXIT_THRESHOLD_MS: i64 = util::duration::seconds(5);

#[derive(Debug, Serialize, Deserialize)]
pub struct Input {
	pub actor_id: Uuid,
	pub client_id: Uuid,
	pub client_workflow_id: Uuid,
	pub config: protocol::ActorConfig,
}

#[workflow]
pub async fn pegboard_actor(ctx: &mut WorkflowCtx, input: &Input) -> GlobalResult<()> {
	ctx.activity(InsertFdbInput {
		actor_id: input.actor_id,
		client_id: input.client_id,
	})
	.await?;

	ctx.loope(State::default(), |ctx, state| {
		let actor_id = input.actor_id;
		let client_id = input.client_id;
		let owner = input.config.owner.clone();
		async move {
			let sig = if let Some(timeout_ts) = state.timeout_ts {
				// If a timeout happens, it means this actor is lost
				if let Some(sig) = ctx.listen_until::<StateUpdate>(timeout_ts).await? {
					sig
				} else {
					// Fake signal
					StateUpdate {
						state: Lost,
						ignore_future_state: false,
					}
				}
			} else {
				ctx.listen::<StateUpdate>().await?
			};

			// Write to DB
			if !matches!(sig.state, Draining { .. } | Undrained) {
				ctx.activity(UpdateStateInput {
					actor_id,
					client_id,
					state: sig.state.clone(),
				})
				.await?;
			}

			// Update timeout
			let complete = match &sig.state {
				Starting => {
					state.timeout_ts = None;
					false
				}
				Running { .. } => false,
				Stopping => {
					state.timeout_ts = Some(util::timestamp::now() + ACTOR_STOP_THRESHOLD_MS);
					false
				}
				Stopped => {
					state.timeout_ts = Some(util::timestamp::now() + ACTOR_EXIT_THRESHOLD_MS);
					false
				}
				Exited { .. } | Lost => true,
				Draining { .. } | Undrained => false,
			};

			state.ignore_future_state |= sig.ignore_future_state;

			// Forward signal to owner
			if !state.ignore_future_state {
				match owner {
					protocol::ActorOwner::DynamicServer { workflow_id, .. } => {
						ctx.signal(sig).to_workflow_id(workflow_id).send().await?;
					}
				}
			}

			if complete {
				Ok(Loop::Break(()))
			} else {
				Ok(Loop::Continue)
			}
		}
		.boxed()
	})
	.await?;

	ctx.activity(ReleaseResourcesInput {
		client_id: input.client_id,
		client_workflow_id: input.client_workflow_id,
		client_flavor: input.config.image.kind.client_flavor(),
		memory: input.config.resources.memory / 1024 / 1024,
		cpu: input.config.resources.cpu,
	})
	.await?;

	Ok(())
}

#[derive(Serialize, Deserialize)]
struct State {
	timeout_ts: Option<i64>,
	ignore_future_state: bool,
}

impl Default for State {
	fn default() -> Self {
		State {
			timeout_ts: Some(util::timestamp::now() + ACTOR_START_THRESHOLD_MS),
			ignore_future_state: false,
		}
	}
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct InsertFdbInput {
	actor_id: Uuid,
	client_id: Uuid,
}

#[activity(InsertFdb)]
async fn insert_fdb(ctx: &ActivityCtx, input: &InsertFdbInput) -> GlobalResult<()> {
	ctx.fdb()
		.await?
		.run(|tx, _mc| async move {
			let actor_key = keys::client::ActorKey::new(input.client_id, input.actor_id);

			tx.set(
				&keys::subspace().pack(&actor_key),
				&actor_key
					.serialize(())
					.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?,
			);

			Ok(())
		})
		.await?;

	Ok(())
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct UpdateStateInput {
	actor_id: Uuid,
	client_id: Uuid,
	state: protocol::ActorState,
}

#[activity(UpdateState)]
async fn update_state(ctx: &ActivityCtx, input: &UpdateStateInput) -> GlobalResult<()> {
	match &input.state {
		Starting | Running { .. } | Stopping => {}
		Stopped | Lost | Exited { .. } => {
			ctx.fdb()
				.await?
				.run(|tx, _mc| async move {
					let actor_key = keys::client::ActorKey::new(input.client_id, input.actor_id);

					tx.clear(&keys::subspace().pack(&actor_key));

					Ok(())
				})
				.await?;
		}
		Draining { .. } | Undrained => unreachable!(),
	}

	Ok(())
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct ReleaseResourcesInput {
	client_id: Uuid,
	client_workflow_id: Uuid,
	client_flavor: protocol::ClientFlavor,
	/// MiB.
	memory: u64,
	/// Millicores.
	cpu: u64,
}

#[activity(ReleaseResources)]
async fn release_resources(ctx: &ActivityCtx, input: &ReleaseResourcesInput) -> GlobalResult<()> {
	ctx.op(crate::ops::client::update_allocation_idx::Input {
		client_id: input.client_id,
		client_workflow_id: input.client_workflow_id,
		flavor: input.client_flavor,
		action: crate::ops::client::update_allocation_idx::Action::ReleaseResources {
			memory: input.memory,
			cpu: input.cpu,
		},
	})
	.await
}

#[signal("pegboard_actor_state_update")]
pub struct StateUpdate {
	pub state: protocol::ActorState,
	pub ignore_future_state: bool,
}
