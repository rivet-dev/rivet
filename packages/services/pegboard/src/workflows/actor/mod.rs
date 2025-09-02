use futures_util::FutureExt;
use gas::prelude::*;
use rivet_runner_protocol::protocol;
use rivet_types::actors::CrashPolicy;

use crate::{errors, workflows::runner::AllocatePendingActorsInput};

mod actor_keys;
mod destroy;
mod runtime;
mod setup;

/// Time to delay an actor from rescheduling after a rescheduling failure.
const BASE_RETRY_TIMEOUT_MS: usize = 2000;
/// How long to wait after creating and not receiving a starting state before setting actor as lost.
const ACTOR_START_THRESHOLD_MS: i64 = util::duration::seconds(30);
/// How long to wait after stopping and not receiving a stop state before setting actor as lost.
const ACTOR_STOP_THRESHOLD_MS: i64 = util::duration::seconds(30);
/// How long an actor goes without retries before it's retry count is reset to 0, effectively resetting its
/// backoff to 0.
const RETRY_RESET_DURATION_MS: i64 = util::duration::minutes(10);

#[derive(Clone, Debug, Serialize, Deserialize, Hash)]
pub struct Input {
	pub actor_id: Id,
	pub name: String,
	pub key: Option<String>,

	pub namespace_id: Id,
	pub runner_name_selector: String,
	pub crash_policy: CrashPolicy,

	/// Arbitrary user string.
	pub input: Option<String>,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct State {
	pub name: String,
	pub key: Option<String>,

	pub namespace_id: Id,
	pub runner_name_selector: String,
	pub crash_policy: CrashPolicy,

	pub create_ts: i64,
	pub create_complete_ts: Option<i64>,
	pub start_ts: Option<i64>,
	// NOTE: This is not the alarm ts, this is when the actor started sleeping. See `LifecycleState` for alarm
	pub sleep_ts: Option<i64>,
	pub complete_ts: Option<i64>,
	pub connectable_ts: Option<i64>,
	pub pending_allocation_ts: Option<i64>,
	pub destroy_ts: Option<i64>,

	// Null if not allocated
	pub runner_id: Option<Id>,
	pub runner_workflow_id: Option<Id>,
}

impl State {
	pub fn new(
		name: String,
		key: Option<String>,
		namespace_id: Id,
		runner_name_selector: String,
		crash_policy: CrashPolicy,
		create_ts: i64,
	) -> Self {
		State {
			name,
			key,

			namespace_id,
			runner_name_selector,
			crash_policy,

			create_ts,
			create_complete_ts: None,

			start_ts: None,
			pending_allocation_ts: None,
			sleep_ts: None,
			connectable_ts: None,
			complete_ts: None,
			destroy_ts: None,

			runner_id: None,
			runner_workflow_id: None,
		}
	}
}

#[workflow]
pub async fn pegboard_actor(ctx: &mut WorkflowCtx, input: &Input) -> Result<()> {
	// Actor creation follows a careful sequence to prevent race conditions:
	//
	// 1. **Add actor to UDB with no indexes** This ensures any services attempting on this actor
	//    by ID will find it exists, even before creation is complete.
	//
	// 2. **Reserve the key with Epoxy** This is slow as it traverses datacenters globally to
	//    ensure key is unique across the entire system. We do this before adding to indexes to
	//    prevent showing the actor in API requests before the creation is complete.
	//
	// 3. **Add actor to relevant indexes** Only done after confirming Epoxy key is reserved. If
	//    we added to indexes before Epoxy validation, actors could appear in lists with duplicate
	//    key (since reservation wasn't confirmed yet).

	let validation_res = ctx
		.activity(setup::ValidateInput {
			name: input.name.clone(),
			key: input.key.clone(),
			namespace_id: input.namespace_id,
			input: input.input.clone(),
		})
		.await?;

	if let Err(error) = validation_res {
		ctx.msg(Failed { error })
			.tag("actor_id", input.actor_id)
			.send()
			.await?;

		// TODO(RVT-3928): return Ok(Err);
		return Ok(());
	}

	ctx.activity(setup::InitStateAndUdbInput {
		actor_id: input.actor_id,
		name: input.name.clone(),
		key: input.key.clone(),
		namespace_id: input.namespace_id,
		runner_name_selector: input.runner_name_selector.clone(),
		crash_policy: input.crash_policy,
		create_ts: ctx.create_ts(),
	})
	.await?;

	if let Some(key) = &input.key {
		match actor_keys::reserve_key(
			ctx,
			input.namespace_id,
			input.name.clone(),
			key.clone(),
			input.actor_id,
		)
		.await?
		{
			actor_keys::ReserveKeyOutput::Success => {}
			actor_keys::ReserveKeyOutput::ForwardToDatacenter { dc_label } => {
				ctx.msg(Failed {
					error: errors::Actor::KeyReservedInDifferentDatacenter {
						datacenter_label: dc_label,
					},
				})
				.tag("actor_id", input.actor_id)
				.send()
				.await?;
				return Ok(());
			}
			actor_keys::ReserveKeyOutput::KeyExists { existing_actor_id } => {
				ctx.msg(Failed {
					error: errors::Actor::DuplicateKey {
						key: key.clone(),
						existing_actor_id,
					},
				})
				.tag("actor_id", input.actor_id)
				.send()
				.await?;

				// Destroyed early
				//
				// This will also deallocate any key that was already allocated to Epoxy
				ctx.workflow(destroy::Input {
					namespace_id: input.namespace_id,
					actor_id: input.actor_id,
					name: input.name.clone(),
					key: input.key.clone(),
					generation: 0,
					kill: false,
				})
				.output()
				.await?;

				// TODO(RVT-3928): return Ok(Err);
				return Ok(());
			}
		}
	}

	ctx.activity(setup::AddIndexesAndSetCreateCompleteInput {
		actor_id: input.actor_id,
	})
	.await?;

	ctx.msg(CreateComplete {})
		.tag("actor_id", input.actor_id)
		.send()
		.await?;

	let Some(allocate_res) = runtime::spawn_actor(ctx, input, 0).await? else {
		// Destroyed early
		ctx.workflow(destroy::Input {
			namespace_id: input.namespace_id,
			actor_id: input.actor_id,
			name: input.name.clone(),
			key: input.key.clone(),
			generation: 0,
			kill: false,
		})
		.output()
		.await?;

		return Ok(());
	};

	let lifecycle_res = ctx
		.loope(
			runtime::LifecycleState::new(allocate_res.runner_id, allocate_res.runner_workflow_id),
			|ctx, state| {
				let input = input.clone();

				async move {
					let sig = if let Some(gc_timeout_ts) = state.gc_timeout_ts {
						// Listen for signal with gc timeout. if a timeout happens, it means this actor is lost
						if let Some(sig) = ctx.listen_until::<Main>(gc_timeout_ts).await? {
							sig
						} else {
							tracing::warn!(actor_id=?input.actor_id, "actor lost");

							// Fake signal
							Main::Lost(Lost {
								generation: state.generation,
							})
						}
					} else if let Some(alarm_ts) = state.alarm_ts {
						// Listen for signal with timeout. if a timeout happens, it means this actor should
						// wake up
						if let Some(sig) = ctx.listen_until::<Main>(alarm_ts).await? {
							sig
						} else {
							tracing::debug!(actor_id=?input.actor_id, "actor wake");

							// Fake signal
							Main::Wake(Wake {})
						}
					} else {
						// Listen for signal normally
						ctx.listen::<Main>().await?
					};

					match sig {
						Main::Event(sig) => {
							// Ignore state updates for previous generations
							if sig.generation() != state.generation {
								return Ok(Loop::Continue);
							}

							match sig {
								protocol::Event::ActorIntent { intent, .. } => match intent {
									protocol::ActorIntent::Sleep => {
										state.gc_timeout_ts =
											Some(util::timestamp::now() + ACTOR_STOP_THRESHOLD_MS);
										state.sleeping = true;

										ctx.activity(runtime::SetSleepingInput {
											actor_id: input.actor_id,
										})
										.await?;

										// Send signal to kill actor now that we know it will be sleeping
										destroy::kill(
											ctx,
											input.actor_id,
											state.generation,
											state.runner_workflow_id,
										)
										.await?;
									}
									protocol::ActorIntent::Stop => {
										state.gc_timeout_ts =
											Some(util::timestamp::now() + ACTOR_STOP_THRESHOLD_MS);

										ctx.activity(runtime::SetNotConnectableInput {
											actor_id: input.actor_id,
										})
										.await?;

										destroy::kill(
											ctx,
											input.actor_id,
											state.generation,
											state.runner_workflow_id,
										)
										.await?;
									}
								},
								protocol::Event::ActorStateUpdate {
									state: actor_state, ..
								} => match actor_state {
									protocol::ActorState::Running => {
										state.gc_timeout_ts = None;

										ctx.activity(runtime::SetStartedInput {
											actor_id: input.actor_id,
										})
										.await?;

										ctx.msg(Ready {})
											.tag("actor_id", input.actor_id)
											.send()
											.await?;
									}
									protocol::ActorState::Stopped { code, .. } => {
										if let Some(res) =
											handle_stopped(ctx, &input, state, Some(code), false)
												.await?
										{
											return Ok(Loop::Break(res));
										}
									}
								},
								protocol::Event::ActorSetAlarm { alarm_ts, .. } => {
									state.alarm_ts = alarm_ts;
								}
							}
						}
						Main::Wake(_sig) => {
							// Ignore wake if we are not sleeping. This is expected to happen under certain
							// circumstances.
							if state.sleeping {
								state.alarm_ts = None;
								state.sleeping = false;

								if runtime::reschedule_actor(ctx, &input, state, true).await? {
									// Destroyed early
									return Ok(Loop::Break(runtime::LifecycleRes {
										generation: state.generation,
										// False here because if we received the destroy signal, it is
										// guaranteed that we did not allocate another actor.
										kill: false,
									}));
								}
							} else {
								tracing::debug!(
									actor_id=?input.actor_id,
									"cannot wake actor that is not sleeping",
								);
							}
						}
						Main::Lost(sig) => {
							// Ignore state updates for previous generations
							if sig.generation != state.generation {
								return Ok(Loop::Continue);
							}

							if let Some(res) =
								handle_stopped(ctx, &input, state, None, true).await?
							{
								return Ok(Loop::Break(res));
							}
						}
						Main::Destroy(_) => {
							return Ok(Loop::Break(runtime::LifecycleRes {
								generation: state.generation,
								kill: true,
							}));
						}
					}

					Ok(Loop::Continue)
				}
				.boxed()
			},
		)
		.await?;

	ctx.workflow(destroy::Input {
		namespace_id: input.namespace_id,
		actor_id: input.actor_id,
		name: input.name.clone(),
		key: input.key.clone(),
		generation: lifecycle_res.generation,
		kill: lifecycle_res.kill,
	})
	.output()
	.await?;

	// NOTE: The reason we allocate other actors from this actor workflow is because if we instead sent a
	// signal to the runner wf here it would incur a heavy throughput hit and we need the runner wf to be as
	// lightweight as possible; processing as few signals that aren't events/commands.
	// Allocate other pending actors from queue
	let res = ctx
		.activity(AllocatePendingActorsInput {
			namespace_id: input.namespace_id,
			name: input.runner_name_selector.clone(),
		})
		.await?;

	// Dispatch pending allocs
	for alloc in res.allocations {
		ctx.signal(alloc.signal)
			.to_workflow::<Workflow>()
			.tag("actor_id", alloc.actor_id)
			.send()
			.await?;
	}

	Ok(())
}

async fn handle_stopped(
	ctx: &mut WorkflowCtx,
	input: &Input,
	state: &mut runtime::LifecycleState,
	code: Option<protocol::StopCode>,
	lost: bool,
) -> Result<Option<runtime::LifecycleRes>> {
	tracing::debug!(?code, "actor stopped");

	// Reset retry count
	if let Some(protocol::StopCode::Ok) = code {
		state.reschedule_state = Default::default();
	}

	state.gc_timeout_ts = None;

	ctx.activity(runtime::DeallocateInput {
		actor_id: input.actor_id,
	})
	.await?;

	// Allocate other pending actors from queue
	let res = ctx
		.activity(AllocatePendingActorsInput {
			namespace_id: input.namespace_id,
			name: input.runner_name_selector.clone(),
		})
		.await?;

	// Dispatch pending allocs
	for alloc in res.allocations {
		ctx.signal(alloc.signal)
			.to_workflow::<Workflow>()
			.tag("actor_id", alloc.actor_id)
			.send()
			.await?;
	}

	if !state.sleeping {
		let failed = matches!(code, None | Some(protocol::StopCode::Error));

		match (failed, input.crash_policy) {
			(true, CrashPolicy::Restart) => {
				// Kill old actor immediately if lost
				if lost {
					destroy::kill(
						ctx,
						input.actor_id,
						state.generation,
						state.runner_workflow_id,
					)
					.await?;
				}

				if runtime::reschedule_actor(ctx, &input, state, false).await? {
					// Destroyed early
					return Ok(Some(runtime::LifecycleRes {
						generation: state.generation,
						// False here because if we received the destroy signal, it is
						// guaranteed that we did not allocate another actor.
						kill: false,
					}));
				}
			}
			(true, CrashPolicy::Sleep) => {
				tracing::debug!(actor_id=?input.actor_id, "actor sleeping due to crash");

				state.sleeping = true;

				ctx.activity(runtime::SetSleepingInput {
					actor_id: input.actor_id,
				})
				.await?;
			}
			_ => {
				ctx.activity(runtime::SetCompleteInput {}).await?;

				if lost {
					ctx.msg(Failed {
						error: errors::Actor::DestroyedWhileWaitingForReady,
					})
					.tag("actor_id", input.actor_id)
					.send()
					.await?;
				}

				return Ok(Some(runtime::LifecycleRes {
					generation: state.generation,
					kill: lost,
				}));
			}
		}
	}

	Ok(None)
}

#[message("pegboard_actor_create_complete")]
pub struct CreateComplete {}

#[message("pegboard_actor_failed")]
pub struct Failed {
	pub error: errors::Actor,
}

#[message("pegboard_actor_ready")]
pub struct Ready {}

#[signal("pegboard_actor_allocate")]
#[derive(Debug)]
pub struct Allocate {
	pub runner_id: Id,
	pub runner_workflow_id: Id,
}

#[signal("pegboard_actor_wake")]
pub struct Wake {}

#[signal("pegboard_actor_lost")]
pub struct Lost {
	pub generation: u32,
}

#[signal("pegboard_actor_destroy")]
pub struct Destroy {}

#[message("pegboard_actor_destroy_started")]
pub struct DestroyStarted {}

#[message("pegboard_actor_destroy_complete")]
pub struct DestroyComplete {}

join_signal!(PendingAllocation {
	Allocate,
	Destroy,
	// Comment to prevent invalid formatting
});

join_signal!(Main {
	Event(protocol::Event),
	Wake,
	Lost,
	Destroy,
});
