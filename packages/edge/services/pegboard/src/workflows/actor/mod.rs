use std::collections::HashMap;

use chirp_workflow::prelude::*;
use destroy::KillCtx;
use futures_util::FutureExt;
use util::serde::AsHashableExt;

use crate::{
	protocol,
	types::{ActorLifecycle, ActorResources, EndpointType, NetworkMode, Routing},
};

pub mod destroy;
mod migrations;
mod runtime;
mod setup;

// A small amount of time to separate the completion of the drain to the deletion of the cluster server. We
// want the drain to complete first.
const DRAIN_PADDING_MS: i64 = 10000;
/// Time to delay an actor from rescheduling after a rescheduling failure.
const BASE_RETRY_TIMEOUT_MS: usize = 2000;
/// How long to wait after creating and not receiving a starting state before setting actor as lost.
const ACTOR_START_THRESHOLD_MS: i64 = util::duration::seconds(30);
/// How long to wait after stopping and not receiving a stop state before setting actor as lost.
const ACTOR_STOP_THRESHOLD_MS: i64 = util::duration::seconds(30);
/// How long to wait after stopped and not receiving an exit state before setting actor as lost.
const ACTOR_EXIT_THRESHOLD_MS: i64 = util::duration::seconds(5);
/// How long an actor goes without retries before it's retry count is reset to 0, effectively resetting its
/// backoff to 0.
const RETRY_RESET_DURATION_MS: i64 = util::duration::minutes(10);

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Input {
	pub actor_id: Uuid,
	pub env_id: Uuid,
	pub tags: HashMap<String, String>,
	pub resources: ActorResources,
	pub lifecycle: ActorLifecycle,
	pub image_id: Uuid,
	pub root_user_enabled: bool,
	pub args: Vec<String>,
	pub network_mode: NetworkMode,
	pub environment: HashMap<String, String>,
	pub network_ports: HashMap<String, Port>,
	pub endpoint_type: Option<EndpointType>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub struct Port {
	// Null when using host networking since one is automatically assigned
	pub internal_port: Option<u16>,
	pub routing: Routing,
}

#[workflow]
pub async fn pegboard_actor(ctx: &mut WorkflowCtx, input: &Input) -> GlobalResult<()> {
	migrations::run(ctx).await?;

	let validation_res = ctx
		.activity(setup::ValidateInput {
			env_id: input.env_id,
			tags: input.tags.as_hashable(),
			resources: input.resources.clone(),
			image_id: input.image_id,
			root_user_enabled: input.root_user_enabled,
			args: input.args.clone(),
			network_mode: input.network_mode,
			environment: input.environment.as_hashable(),
			network_ports: input.network_ports.as_hashable(),
		})
		.await?;

	if let Some(error_message) = validation_res {
		ctx.msg(Failed {
			message: error_message,
		})
		.tag("actor_id", input.actor_id)
		.send()
		.await?;

		// TODO(RVT-3928): return Ok(Err);
		return Ok(());
	}

	let network_ports = ctx
		.activity(setup::DisableTlsPortsInput {
			network_ports: input.network_ports.as_hashable(),
		})
		.await?;

	let res = setup::setup(
		ctx,
		input,
		setup::SetupCtx::Init {
			network_ports: network_ports.clone(),
		},
	)
	.await;
	let initial_actor_setup = match ctx.catch_unrecoverable(res)? {
		Ok(res) => res,
		Err(err) => {
			tracing::error!(?err, "unrecoverable setup");

			ctx.msg(Failed {
				message: "Failed setup.".into(),
			})
			.tag("actor_id", input.actor_id)
			.send()
			.await?;

			ctx.workflow(destroy::Input {
				actor_id: input.actor_id,
				build_kind: None,
				kill: None,
			})
			.output()
			.await?;

			// Throw the original error from the setup activities
			return Err(err);
		}
	};

	ctx.msg(CreateComplete {})
		.tag("actor_id", input.actor_id)
		.send()
		.await?;

	let Some(res) = runtime::spawn_actor(ctx, input, &initial_actor_setup, 0).await? else {
		ctx.msg(Failed {
			message: "Failed to allocate (no availability).".into(),
		})
		.tag("actor_id", input.actor_id)
		.send()
		.await?;

		ctx.workflow(destroy::Input {
			actor_id: input.actor_id,
			build_kind: Some(initial_actor_setup.meta.build_kind),
			kill: None,
		})
		.output()
		.await?;

		return Ok(());
	};

	let state_res = ctx
		.loope(
			runtime::State::new(res.client_id, res.client_workflow_id, input.image_id),
			|ctx, state| {
				let input = input.clone();

				async move {
					let sig = if let Some(drain_timeout_ts) = state.drain_timeout_ts {
						// Listen for signal with drain timeout
						if let Some(sig) = ctx.listen_until::<Main>(drain_timeout_ts).await? {
							sig
						}
						// Reschedule durable actor on drain end
						else if input.lifecycle.durable {
							ctx.activity(runtime::SetConnectableInput { connectable: false })
								.await?;

							// Kill old actor immediately
							destroy::kill(
								ctx,
								input.actor_id,
								state.generation,
								state.client_workflow_id,
								0,
								true,
							)
							.await?;

							if let Some(sig) = runtime::reschedule_actor(
								ctx,
								&input,
								state,
								state.image_id.unwrap_or(input.image_id),
							)
							.await?
							{
								// Destroyed early
								return Ok(Loop::Break(runtime::StateRes {
									kill: Some(KillCtx {
										generation: state.generation,
										kill_timeout_ms: sig
											.override_kill_timeout_ms
											.unwrap_or(input.lifecycle.kill_timeout_ms),
									}),
								}));
							} else {
								state.drain_timeout_ts = None;
								return Ok(Loop::Continue);
							}
						} else {
							return Ok(Loop::Break(runtime::StateRes {
								kill: Some(KillCtx {
									generation: state.generation,
									kill_timeout_ms: input.lifecycle.kill_timeout_ms,
								}),
							}));
						}
					} else if let Some(gc_timeout_ts) = state.gc_timeout_ts {
						// Listen for signal with gc timeout. if a timeout happens, it means this actor is lost
						if let Some(sig) = ctx.listen_until::<Main>(gc_timeout_ts).await? {
							sig
						} else {
							tracing::warn!(actor_id=?input.actor_id, "actor lost");

							// Fake signal
							Main::StateUpdate(StateUpdate {
								generation: state.generation,
								state: protocol::ActorState::Lost,
							})
						}
					} else {
						// Listen for signal normally
						ctx.listen::<Main>().await?
					};

					match sig {
						Main::StateUpdate(sig) => {
							// Ignore state updates for previous generations
							if sig.generation != state.generation {
								return Ok(Loop::Continue);
							}

							ctx.activity(runtime::UpdateFdbInput {
								actor_id: input.actor_id,
								client_id: state.client_id,
								state: sig.state.clone(),
							})
							.await?;

							match sig.state {
								protocol::ActorState::Starting => {
									state.gc_timeout_ts = None;

									ctx.activity(runtime::SetStartedInput {}).await?;
								}
								protocol::ActorState::Running { ports, .. } => {
									ctx.join((
										activity(runtime::InsertPortsInput {
											ports: ports.clone(),
										}),
										activity(runtime::InsertPortsFdbInput {
											actor_id: input.actor_id,
											ports,
										}),
									))
									.await?;

									// Old traefik timeout
									ctx.removed::<Activity<WaitForTraefikPoll>>().await?;

									let updated = ctx
										.activity(runtime::SetConnectableInput {
											connectable: true,
										})
										.await?;

									if updated {
										ctx.msg(Ready {})
											.tag("actor_id", input.actor_id)
											.send()
											.await?;
									}
								}
								protocol::ActorState::Stopping => {
									state.gc_timeout_ts =
										Some(util::timestamp::now() + ACTOR_STOP_THRESHOLD_MS);
								}
								protocol::ActorState::Stopped => {
									state.gc_timeout_ts =
										Some(util::timestamp::now() + ACTOR_EXIT_THRESHOLD_MS);
								}
								protocol::ActorState::Exited { .. }
								| protocol::ActorState::Lost => {
									let exit_code =
										if let protocol::ActorState::Exited { exit_code } =
											sig.state
										{
											exit_code
										} else {
											None
										};

									tracing::debug!(?exit_code, "actor stopped");

									let failed =
										exit_code.map(|exit_code| exit_code != 0).unwrap_or(true);

									// Reschedule durable actor if it errored
									if input.lifecycle.durable && failed {
										ctx.activity(runtime::SetConnectableInput {
											connectable: false,
										})
										.await?;

										// Kill old actor immediately if lost
										if let protocol::ActorState::Lost = sig.state {
											destroy::kill(
												ctx,
												input.actor_id,
												state.generation,
												state.client_workflow_id,
												0,
												true,
											)
											.await?;
										}

										if runtime::reschedule_actor(
											ctx,
											&input,
											state,
											state.image_id.unwrap_or(input.image_id),
										)
										.await?
										.is_some()
										{
											// Destroyed early
											return Ok(Loop::Break(runtime::StateRes {
												// Destroy actor is none here because if we received the destroy
												// signal, it is guaranteed that we did not allocate another actor.
												kill: None,
											}));
										}
									} else {
										ctx.activity(runtime::SetFinishedInput {}).await?;

										if let protocol::ActorState::Lost = sig.state {
											ctx.msg(Failed {
												message:
													"Actor timed out trying to reach a ready state."
														.into(),
											})
											.tag("actor_id", input.actor_id)
											.send()
											.await?;
										}

										return Ok(Loop::Break(runtime::StateRes {
											// No need to kill if already exited
											kill: matches!(sig.state, protocol::ActorState::Lost)
												.then_some(KillCtx {
													generation: state.generation,
													kill_timeout_ms: 0,
												}),
										}));
									}
								}
							}
						}
						Main::Upgrade(sig) => {
							ctx.msg(UpgradeStarted {})
								.tag("actor_id", input.actor_id)
								.send()
								.await?;

							ctx.activity(runtime::SetConnectableInput { connectable: false })
								.await?;

							// Kill old actor immediately
							destroy::kill(
								ctx,
								input.actor_id,
								state.generation,
								state.client_workflow_id,
								0,
								true,
							)
							.await?;

							ctx.activity(runtime::UpdateImageInput {
								image_id: sig.image_id,
							})
							.await?;
							state.image_id = Some(sig.image_id);

							if let Some(sig) = runtime::reschedule_actor(
								ctx,
								&input,
								state,
								state.image_id.unwrap_or(input.image_id),
							)
							.await?
							{
								// Destroyed early
								return Ok(Loop::Break(runtime::StateRes {
									kill: Some(KillCtx {
										generation: state.generation,
										kill_timeout_ms: sig
											.override_kill_timeout_ms
											.unwrap_or(input.lifecycle.kill_timeout_ms),
									}),
								}));
							}

							ctx.msg(UpgradeComplete {})
								.tag("actor_id", input.actor_id)
								.send()
								.await?;
						}
						Main::Drain(sig) => {
							state.drain_timeout_ts = Some(
								sig.drain_timeout_ts
									- DRAIN_PADDING_MS - input.lifecycle.kill_timeout_ms,
							);
						}
						Main::Undrain(_) => {
							state.drain_timeout_ts = None;
						}
						Main::Destroy(sig) => {
							return Ok(Loop::Break(runtime::StateRes {
								kill: Some(KillCtx {
									generation: state.generation,
									kill_timeout_ms: sig
										.override_kill_timeout_ms
										.unwrap_or(input.lifecycle.kill_timeout_ms),
								}),
							}))
						}
					}

					Ok(Loop::Continue)
				}
				.boxed()
			},
		)
		.await?;

	ctx.workflow(destroy::Input {
		actor_id: input.actor_id,
		build_kind: Some(initial_actor_setup.meta.build_kind),
		kill: state_res.kill,
	})
	.output()
	.await?;

	Ok(())
}

#[message("pegboard_actor_create_complete")]
pub struct CreateComplete {}

#[message("pegboard_actor_failed")]
pub struct Failed {
	pub message: String,
}

#[message("pegboard_actor_ready")]
pub struct Ready {}

#[signal("pegboard_actor_destroy")]
pub struct Destroy {
	pub override_kill_timeout_ms: Option<i64>,
}

#[signal("pegboard_actor_drain")]
pub struct Drain {
	pub drain_timeout_ts: i64,
}

#[signal("pegboard_actor_undrain")]
pub struct Undrain {}

#[message("pegboard_actor_destroy_started")]
pub struct DestroyStarted {}

#[message("pegboard_actor_destroy_complete")]
pub struct DestroyComplete {}

#[signal("pegboard_actor_upgrade")]
pub struct Upgrade {
	pub image_id: Uuid,
}

#[signal("pegboard_actor_state_update")]
pub struct StateUpdate {
	#[serde(default)]
	pub generation: u32,
	pub state: protocol::ActorState,
}

#[message("pegboard_actor_upgrade_started")]
pub struct UpgradeStarted {}

#[message("pegboard_actor_upgrade_complete")]
pub struct UpgradeComplete {}

join_signal!(Main {
	StateUpdate,
	Upgrade,
	Drain,
	Undrain,
	Destroy,
});

// Stub definition
#[derive(Debug, Serialize, Deserialize, Hash)]
pub struct WaitForTraefikPollInput {}
#[activity(WaitForTraefikPoll)]
pub async fn wait_for_traefik_poll(
	_ctx: &ActivityCtx,
	_input: &WaitForTraefikPollInput,
) -> GlobalResult<()> {
	Ok(())
}
