use futures_util::FutureExt;
use gas::prelude::*;
use rivet_runner_protocol as protocol;
use rivet_types::actors::CrashPolicy;

use crate::{errors, workflows::runner2::AllocatePendingActorsInput};

mod destroy;
mod keys;
mod runtime;
mod setup;

pub use runtime::AllocationOverride;

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

	#[serde(default)]
	pub for_serverless: bool,
	// This is used for state management for incrementing/decrementing the serverless desired slots key
	#[serde(default)]
	pub allocated_serverless_slot: bool,

	pub start_ts: Option<i64>,
	// NOTE: This is not the alarm ts, this is when the actor started sleeping. See `LifecycleState` for alarm
	pub sleep_ts: Option<i64>,
	pub complete_ts: Option<i64>,
	pub connectable_ts: Option<i64>,
	pub pending_allocation_ts: Option<i64>,
	#[serde(default)]
	pub reschedule_ts: Option<i64>,
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

			for_serverless: false,
			allocated_serverless_slot: false,

			start_ts: None,
			pending_allocation_ts: None,
			sleep_ts: None,
			connectable_ts: None,
			complete_ts: None,
			reschedule_ts: None,
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
		match keys::reserve_key(
			ctx,
			input.namespace_id,
			input.name.clone(),
			key.clone(),
			input.actor_id,
		)
		.await?
		{
			keys::ReserveKeyOutput::Success => {}
			keys::ReserveKeyOutput::ForwardToDatacenter { dc_label } => {
				ctx.msg(Failed {
					error: errors::Actor::KeyReservedInDifferentDatacenter {
						datacenter_label: dc_label,
					},
				})
				.tag("actor_id", input.actor_id)
				.send()
				.await?;

				// Destroyed early
				ctx.workflow(destroy::Input {
					namespace_id: input.namespace_id,
					actor_id: input.actor_id,
					name: input.name.clone(),
					key: input.key.clone(),
					generation: 0,
				})
				.output()
				.await?;

				return Ok(());
			}
			keys::ReserveKeyOutput::KeyExists { existing_actor_id } => {
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
				ctx.workflow(destroy::Input {
					namespace_id: input.namespace_id,
					actor_id: input.actor_id,
					name: input.name.clone(),
					key: input.key.clone(),
					generation: 0,
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

	let lifecycle_state =
		match runtime::spawn_actor(ctx, input, 0, AllocationOverride::None).await? {
			runtime::SpawnActorOutput::Allocated {
				runner_id,
				runner_workflow_id,
			} => runtime::LifecycleState::new(
				runner_id,
				runner_workflow_id,
				ctx.config().pegboard().actor_start_threshold(),
			),
			runtime::SpawnActorOutput::Sleep => {
				ctx.activity(runtime::SetSleepingInput {
					actor_id: input.actor_id,
				})
				.await?;

				runtime::LifecycleState::new_sleeping()
			}
			runtime::SpawnActorOutput::Destroy => {
				// Destroyed early
				ctx.workflow(destroy::Input {
					namespace_id: input.namespace_id,
					actor_id: input.actor_id,
					name: input.name.clone(),
					key: input.key.clone(),
					generation: 0,
				})
				.output()
				.await?;

				return Ok(());
			}
		};

	let lifecycle_res = ctx
		.loope(
			lifecycle_state,
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
								force_reschedule: false,
								reset_rescheduling: false,
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
							Main::Wake(Wake { allocation_override: AllocationOverride::DontSleep { pending_timeout: None } })
						}
					} else {
						// Listen for signal normally
						ctx.listen::<Main>().await?
					};

					match sig {
						Main::Event(sig) => {
							// Ignore state updates for previous generations
							if crate::utils::event_generation(&sig.inner) != state.generation {
								return Ok(Loop::Continue);
							}

							let (Some(runner_id), Some(runner_workflow_id)) =
								(state.runner_id, state.runner_workflow_id)
							else {
								tracing::warn!("actor not allocated, ignoring event");
								return Ok(Loop::Continue);
							};

							match sig.inner {
								protocol::Event::EventActorIntent(protocol::EventActorIntent {
									intent,
									..
								}) => match intent {
									protocol::ActorIntent::ActorIntentSleep => {
										if !state.sleeping {
											state.gc_timeout_ts = Some(
												util::timestamp::now()
													+ ctx
														.config()
														.pegboard()
														.actor_stop_threshold(),
											);
											state.sleeping = true;

											ctx.activity(runtime::SetSleepingInput {
												actor_id: input.actor_id,
											})
											.await?;

											// Send signal to stop actor now that we know it will be sleeping
											ctx.signal(crate::workflows::runner2::Command {
												inner: protocol::Command::CommandStopActor(
													protocol::CommandStopActor {
														actor_id: input.actor_id.to_string(),
														generation: state.generation,
													},
												),
											})
											.to_workflow_id(runner_workflow_id)
											.send()
											.await?;
										}
									}
									protocol::ActorIntent::ActorIntentStop => {
										if !state.stopping {
											state.gc_timeout_ts = Some(
												util::timestamp::now()
													+ ctx
														.config()
														.pegboard()
														.actor_stop_threshold(),
											);
											state.stopping = true;

											ctx.activity(runtime::SetNotConnectableInput {
												actor_id: input.actor_id,
											})
											.await?;

											ctx.signal(crate::workflows::runner2::Command {
												inner: protocol::Command::CommandStopActor(
													protocol::CommandStopActor {
														actor_id: input.actor_id.to_string(),
														generation: state.generation,
													},
												),
											})
											.to_workflow_id(runner_workflow_id)
											.send()
											.await?;
										}
									}
								},
								protocol::Event::EventActorStateUpdate(
									protocol::EventActorStateUpdate {
										state: actor_state, ..
									},
								) => match actor_state {
									protocol::ActorState::ActorStateRunning => {
										state.gc_timeout_ts = None;

										ctx.activity(runtime::SetStartedInput {
											actor_id: input.actor_id,
										})
										.await?;

										ctx.msg(Ready { runner_id })
											.tag("actor_id", input.actor_id)
											.send()
											.await?;
									}
									protocol::ActorState::ActorStateStopped(
										protocol::ActorStateStopped { code, .. },
									) => {
										if let StoppedResult::Destroy = handle_stopped(
											ctx,
											&input,
											state,
											StoppedVariant::Normal { code },
										)
										.await?
										{
											return Ok(Loop::Break(runtime::LifecycleResult {
												generation: state.generation,
											}));
										}
									}
								},
								protocol::Event::EventActorSetAlarm(
									protocol::EventActorSetAlarm { alarm_ts, .. },
								) => {
									state.alarm_ts = alarm_ts;
								}
							}
						}
						Main::Wake(sig) => {
							if state.sleeping {
								if state.runner_id.is_none() {
									state.alarm_ts = None;
									state.sleeping = false;
									state.will_wake = false;

									match runtime::reschedule_actor(ctx, &input, state, sig.allocation_override)
										.await?
									{
										runtime::SpawnActorOutput::Allocated { .. } => {}
										runtime::SpawnActorOutput::Sleep => {
											state.sleeping = true;
										}
										runtime::SpawnActorOutput::Destroy => {
											// Destroyed early
											return Ok(Loop::Break(runtime::LifecycleResult {
												generation: state.generation,
											}));
										}
									}
								} else if !state.will_wake {
									state.will_wake = true;

									tracing::debug!(
										actor_id=?input.actor_id,
										"cannot wake an actor that intends to sleep but has not stopped yet, deferring wake until after stop",
									);
								}
							} else {
								tracing::debug!(
									actor_id=?input.actor_id,
									"cannot wake actor that is not sleeping",
								);
							}
						}
						Main::Lost(sig) => {
							// Ignore signals for previous generations
							if sig.generation != state.generation {
								return Ok(Loop::Continue);
							}

							if sig.reset_rescheduling {
								state.reschedule_state = Default::default();
							}

							if let StoppedResult::Destroy = handle_stopped(
								ctx,
								&input,
								state,
								StoppedVariant::Lost {
									force_reschedule: sig.force_reschedule,
								},
							)
							.await?
							{
								return Ok(Loop::Break(runtime::LifecycleResult {
									generation: state.generation,
								}));
							}
						}
						Main::GoingAway(sig) => {
							// Ignore signals for previous generations
							if sig.generation != state.generation {
								return Ok(Loop::Continue);
							}

							if sig.reset_rescheduling {
								state.reschedule_state = Default::default();
							}

							if !state.going_away {
								let Some(runner_workflow_id) = state.runner_workflow_id else {
									return Ok(Loop::Continue);
								};

								state.gc_timeout_ts = Some(
									util::timestamp::now()
										+ ctx.config().pegboard().actor_stop_threshold(),
								);
								state.going_away = true;

								ctx.activity(runtime::SetNotConnectableInput {
									actor_id: input.actor_id,
								})
								.await?;

								ctx.signal(crate::workflows::runner2::Command {
									inner: protocol::Command::CommandStopActor(protocol::CommandStopActor {
										actor_id: input.actor_id.to_string(),
										generation: state.generation,
									}),
								})
								.to_workflow_id(runner_workflow_id)
								.send()
								.await?;
							}
						}
						Main::Destroy(_) => {
							// If allocated, send stop actor command
							if let Some(runner_workflow_id) = state.runner_workflow_id {
								ctx.signal(crate::workflows::runner2::Command {
									inner: protocol::Command::CommandStopActor(protocol::CommandStopActor {
										actor_id: input.actor_id.to_string(),
										generation: state.generation,
									}),
								})
								.to_workflow_id(runner_workflow_id)
								.send()
								.await?;
							}

							return Ok(Loop::Break(runtime::LifecycleResult {
								generation: state.generation,
							}));
						}
					}

					Ok(Loop::Continue)
				}
				.boxed()
			},
		)
		.await?;

	// At this point, the actor is not allocated so no cleanup related to alloc idx/desired slots needs to be
	// done.

	ctx.workflow(destroy::Input {
		namespace_id: input.namespace_id,
		actor_id: input.actor_id,
		name: input.name.clone(),
		key: input.key.clone(),
		generation: lifecycle_res.generation,
	})
	.output()
	.await?;

	Ok(())
}

#[derive(Debug)]
enum StoppedVariant {
	Normal { code: protocol::StopCode },
	Lost { force_reschedule: bool },
}

enum StoppedResult {
	Continue,
	Destroy,
}

async fn handle_stopped(
	ctx: &mut WorkflowCtx,
	input: &Input,
	state: &mut runtime::LifecycleState,
	variant: StoppedVariant,
) -> Result<StoppedResult> {
	tracing::debug!(?variant, "actor stopped");

	let force_reschedule = match &variant {
		StoppedVariant::Normal { code } => {
			// Reset retry count on successful exit
			if let protocol::StopCode::Ok = code {
				state.reschedule_state = Default::default();
			}

			false
		}
		StoppedVariant::Lost { force_reschedule } => *force_reschedule,
	};

	// Clear stop gc timeout to prevent being marked as lost in the lifecycle loop
	state.gc_timeout_ts = None;
	state.stopping = false;
	state.runner_id = None;
	let old_runner_workflow_id = state.runner_workflow_id.take();

	let deallocate_res = ctx
		.activity(runtime::DeallocateInput {
			actor_id: input.actor_id,
		})
		.await?;

	// NOTE: The reason we allocate other actors from this actor workflow is because if we instead sent a
	// signal to the runner wf here it would incur a heavy throughput hit and we need the runner wf to be as
	// lightweight as possible; processing as few signals that aren't events/commands.
	// Allocate other pending actors from queue since a slot has now cleared
	let allocate_pending_res = ctx
		.activity(AllocatePendingActorsInput {
			namespace_id: input.namespace_id,
			name: input.runner_name_selector.clone(),
		})
		.await?;

	if allocate_pending_res.allocations.is_empty() {
		// Bump autoscaler so it can scale down if needed
		if deallocate_res.for_serverless {
			ctx.msg(rivet_types::msgs::pegboard::BumpServerlessAutoscaler {})
				.send()
				.await?;
		}
	} else {
		// Dispatch pending allocs (if any)
		for alloc in allocate_pending_res.allocations {
			ctx.signal(alloc.signal)
				.to_workflow::<Workflow>()
				.tag("actor_id", alloc.actor_id)
				.send()
				.await?;
		}
	}

	// We don't know the state of the previous generation of this actor actor if it becomes lost, send stop
	// command in case it ended up allocating
	if let (StoppedVariant::Lost { .. }, Some(old_runner_workflow_id)) =
		(&variant, old_runner_workflow_id)
	{
		ctx.signal(crate::workflows::runner2::Command {
			inner: protocol::Command::CommandStopActor(protocol::CommandStopActor {
				actor_id: input.actor_id.to_string(),
				generation: state.generation,
			}),
		})
		.to_workflow_id(old_runner_workflow_id)
		.send()
		.await?;
	}

	// Reschedule no matter what
	if force_reschedule {
		match runtime::reschedule_actor(
			ctx,
			&input,
			state,
			AllocationOverride::DontSleep {
				pending_timeout: None,
			},
		)
		.await?
		{
			runtime::SpawnActorOutput::Allocated { .. } => {}
			// NOTE: This should be unreachable because force_reschedule is true
			runtime::SpawnActorOutput::Sleep => {
				state.sleeping = true;

				ctx.activity(runtime::SetSleepingInput {
					actor_id: input.actor_id,
				})
				.await?;
			}
			// Destroyed early
			runtime::SpawnActorOutput::Destroy => return Ok(StoppedResult::Destroy),
		}
	}
	// Handle rescheduling if not marked as sleeping
	else if !state.sleeping {
		let graceful_exit = !state.going_away
			&& matches!(
				variant,
				StoppedVariant::Normal {
					code: protocol::StopCode::Ok
				}
			);

		match (input.crash_policy, graceful_exit) {
			(CrashPolicy::Restart, false) => {
				match runtime::reschedule_actor(ctx, &input, state, AllocationOverride::None)
					.await?
				{
					runtime::SpawnActorOutput::Allocated { .. } => {}
					// NOTE: Its not possible for `SpawnActorOutput::Sleep` to be returned here, the crash
					// policy is `Restart`.
					runtime::SpawnActorOutput::Sleep | runtime::SpawnActorOutput::Destroy => {
						// Destroyed early
						return Ok(StoppedResult::Destroy);
					}
				}
			}
			(CrashPolicy::Sleep, false) => {
				tracing::debug!(actor_id=?input.actor_id, "actor sleeping due to crash");

				state.sleeping = true;

				ctx.activity(runtime::SetSleepingInput {
					actor_id: input.actor_id,
				})
				.await?;
			}
			_ => {
				ctx.activity(runtime::SetCompleteInput {}).await?;

				return Ok(StoppedResult::Destroy);
			}
		}
	}
	// Rewake actor immediately after stopping if `will_wake` was set
	else if state.will_wake {
		state.sleeping = false;

		match runtime::reschedule_actor(ctx, &input, state, AllocationOverride::None).await? {
			runtime::SpawnActorOutput::Allocated { .. } => {}
			runtime::SpawnActorOutput::Sleep => {
				state.sleeping = true;
			}
			// Destroyed early
			runtime::SpawnActorOutput::Destroy => return Ok(StoppedResult::Destroy),
		}
	}

	state.will_wake = false;
	state.going_away = false;

	ctx.msg(Stopped {})
		.tag("actor_id", input.actor_id)
		.send()
		.await?;

	Ok(StoppedResult::Continue)
}

#[message("pegboard_actor_create_complete")]
pub struct CreateComplete {}

#[message("pegboard_actor_failed")]
pub struct Failed {
	pub error: errors::Actor,
}

#[message("pegboard_actor_ready")]
pub struct Ready {
	pub runner_id: Id,
}

#[message("pegboard_actor_stopped")]
pub struct Stopped {}

#[signal("pegboard_actor_allocate")]
#[derive(Debug)]
pub struct Allocate {
	pub runner_id: Id,
	pub runner_workflow_id: Id,
}

#[signal("pegboard_actor_event")]
pub struct Event {
	pub inner: protocol::Event,
}

#[signal("pegboard_actor_wake")]
pub struct Wake {
	#[serde(default)]
	pub allocation_override: AllocationOverride,
}

#[derive(Debug)]
#[signal("pegboard_actor_lost")]
pub struct Lost {
	pub generation: u32,
	/// Immediately reschedules the actor regardless of its crash policy.
	pub force_reschedule: bool,
	/// Resets the rescheduling retry count to 0.
	#[serde(default)]
	pub reset_rescheduling: bool,
}

#[derive(Debug)]
#[signal("pegboard_actor_going_away")]
pub struct GoingAway {
	pub generation: u32,
	/// Resets the rescheduling retry count to 0.
	#[serde(default)]
	pub reset_rescheduling: bool,
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
	Event(Event),
	Wake,
	Lost,
	GoingAway,
	Destroy,
});
