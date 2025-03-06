use chirp_workflow::prelude::*;
use futures_util::FutureExt;

use crate::protocol::{self, ActorState::*};

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
	ctx.loope(State::default(), |ctx, state| {
		let actor_id = input.actor_id;
		let client_workflow_id = input.client_workflow_id;
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
					client_workflow_id,
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

			// Forward signal to owner
			if !state.ignore_future_state {
				state.ignore_future_state = sig.ignore_future_state;

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
struct UpdateStateInput {
	actor_id: Uuid,
	client_workflow_id: Uuid,
	state: protocol::ActorState,
}

#[activity(UpdateState)]
async fn update_state(ctx: &ActivityCtx, input: &UpdateStateInput) -> GlobalResult<()> {
	let pool = &ctx.sqlite_for_workflow(input.client_workflow_id).await?;

	match &input.state {
		Starting => {
			sql_execute!(
				[ctx, pool]
				"
				UPDATE actors
				SET start_ts = $2
				WHERE actor_id = $1
				",
				input.actor_id,
				util::timestamp::now(),
			)
			.await?;
		}
		Running { pid, .. } => {
			sql_execute!(
				[ctx, pool]
				"
				UPDATE actors
				SET
					running_ts = $2,
					pid = $3
				WHERE actor_id = $1
				",
				input.actor_id,
				util::timestamp::now(),
				*pid as i64,
			)
			.await?;
		}
		Stopping => {
			sql_execute!(
				[ctx, pool]
				"
				UPDATE actors
				SET
					stopping_ts = $2
				WHERE actor_id = $1 AND stopping_ts IS NULL
				",
				input.actor_id,
				util::timestamp::now(),
			)
			.await?;
		}
		Stopped => {
			sql_execute!(
				[ctx, pool]
				"
				UPDATE actors
				SET stop_ts = $2
				WHERE actor_id = $1
				",
				input.actor_id,
				util::timestamp::now(),
			)
			.await?;
		}
		Lost => {
			sql_execute!(
				[ctx, pool]
				"
				UPDATE actors
				SET
					lost_ts = $2
				WHERE actor_id = $1
				RETURNING ignore_future_state
				",
				input.actor_id,
				util::timestamp::now(),
			)
			.await?;
		}
		Exited { exit_code } => {
			sql_execute!(
				[ctx, pool]
				"
				UPDATE actors
				SET
					exit_ts = $2,
					exit_code = $3
				WHERE actor_id = $1
				RETURNING ignore_future_state
				",
				input.actor_id,
				util::timestamp::now(),
				exit_code,
			)
			.await?;
		}
		_ => unreachable!(),
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
}

#[activity(ReleaseResources)]
async fn release_resources(ctx: &ActivityCtx, input: &ReleaseResourcesInput) -> GlobalResult<()> {
	ctx.op(crate::ops::client::update_allocation_idx::Input {
		client_id: input.client_id,
		client_workflow_id: input.client_workflow_id,
		flavor: input.client_flavor,
		action: crate::ops::client::update_allocation_idx::Action::ReleaseMemory {
			memory: input.memory,
		},
	})
	.await
}

#[signal("pegboard_actor_state_update")]
pub struct StateUpdate {
	pub state: protocol::ActorState,
	pub ignore_future_state: bool,
}
