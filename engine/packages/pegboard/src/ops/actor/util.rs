use gas::db::WorkflowData;
use gas::prelude::*;
use rivet_types::actors::Actor;
use std::collections::{HashMap, HashSet};

use crate::workflows::actor::FailureReason as WorkflowFailureReason;
use crate::workflows::actor2::ActorError as WorkflowActorError;

enum ActorState {
	V1(crate::workflows::actor::State),
	V2(crate::workflows::actor2::State),
}

impl ActorState {
	fn namespace_id(&self) -> Id {
		match self {
			ActorState::V1(x) => x.namespace_id,
			ActorState::V2(x) => x.namespace_id,
		}
	}

	fn pool_name(&self) -> &str {
		match self {
			ActorState::V1(x) => &x.runner_name_selector,
			ActorState::V2(x) => &x.pool_name,
		}
	}

	/// Resolves an actor's error, enriching NoCapacity with RunnerPoolError if available.
	fn resolve_error(
		&self,
		runner_pool_errors: &HashMap<(Id, String), rivet_types::actor::RunnerPoolError>,
	) -> Option<rivet_types::actor::ActorError> {
		match self {
			ActorState::V1(x) => match &x.failure_reason {
				Some(WorkflowFailureReason::NoCapacity) => {
					let key = (x.namespace_id, x.runner_name_selector.clone());
					if let Some(pool_error) = runner_pool_errors.get(&key) {
						Some(rivet_types::actor::ActorError::RunnerPoolError(
							pool_error.clone(),
						))
					} else {
						Some(rivet_types::actor::ActorError::NoCapacity)
					}
				}
				Some(WorkflowFailureReason::RunnerNoResponse { runner_id }) => {
					Some(rivet_types::actor::ActorError::RunnerNoResponse {
						runner_id: *runner_id,
					})
				}
				Some(WorkflowFailureReason::RunnerConnectionLost { runner_id }) => {
					Some(rivet_types::actor::ActorError::RunnerConnectionLost {
						runner_id: *runner_id,
					})
				}
				Some(WorkflowFailureReason::RunnerDrainingTimeout { runner_id }) => {
					Some(rivet_types::actor::ActorError::RunnerDrainingTimeout {
						runner_id: *runner_id,
					})
				}
				Some(WorkflowFailureReason::Crashed { message }) => {
					Some(rivet_types::actor::ActorError::Crashed {
						message: message.clone(),
					})
				}
				None => None,
			},
			ActorState::V2(x) => match &x.error {
				Some(WorkflowActorError::ConcurrentActorLimitReached) => {
					Some(rivet_types::actor::ActorError::ConcurrentActorLimitReached)
				}
				Some(WorkflowActorError::NoEnvoys) => {
					let key = (x.namespace_id, x.pool_name.clone());
					if let Some(pool_error) = runner_pool_errors.get(&key) {
						Some(rivet_types::actor::ActorError::RunnerPoolError(
							pool_error.clone(),
						))
					} else {
						Some(rivet_types::actor::ActorError::NoEnvoys)
					}
				}
				Some(WorkflowActorError::EnvoyNoResponse { envoy_key }) => {
					Some(rivet_types::actor::ActorError::EnvoyNoResponse {
						envoy_key: envoy_key.clone(),
					})
				}
				Some(WorkflowActorError::EnvoyConnectionLost { envoy_key }) => {
					Some(rivet_types::actor::ActorError::EnvoyConnectionLost {
						envoy_key: envoy_key.clone(),
					})
				}
				Some(WorkflowActorError::Crashed { message }) => {
					Some(rivet_types::actor::ActorError::Crashed {
						message: message.clone(),
					})
				}
				None => None,
			},
		}
	}

	fn is_error_no_capacity(&self) -> bool {
		match self {
			ActorState::V1(x) => {
				matches!(x.failure_reason, Some(WorkflowFailureReason::NoCapacity))
			}
			ActorState::V2(x) => matches!(x.error, Some(WorkflowActorError::NoEnvoys)),
		}
	}
}

/// Builds Actor structs from workflow data.
pub async fn build_actors_from_workflows(
	ctx: &OperationCtx,
	actors_with_wf_ids: Vec<(Id, Id)>,
	wfs: Vec<WorkflowData>,
	dc_name: &str,
	fetch_error: bool,
) -> Result<Vec<Actor>> {
	// Parse all actor states
	let mut actor_data = Vec::with_capacity(wfs.len());
	for (actor_id, workflow_id) in actors_with_wf_ids {
		let Some(wf) = wfs.iter().find(|wf| wf.workflow_id == workflow_id) else {
			// Actor not found
			continue;
		};

		// TODO: Kinda hacky, should be using the version property from fdb
		let actor_state = match wf.name.as_str() {
			"pegboard_actor" => {
				match wf.parse_state::<Option<crate::workflows::actor::State>>() {
					Ok(Some(s)) => ActorState::V1(s),
					Ok(None) => {
						// Actor did not initialize state yet
						continue;
					}
					Err(err) => {
						tracing::error!(
							?actor_id,
							?workflow_id,
							?err,
							"failed to parse v1 wf state"
						);
						continue;
					}
				}
			}
			"pegboard_actor2" => {
				match wf.parse_state::<Option<crate::workflows::actor2::State>>() {
					Ok(Some(s)) => ActorState::V2(s),
					Ok(None) => {
						// Actor did not initialize state yet
						continue;
					}
					Err(err) => {
						tracing::error!(?actor_id, ?workflow_id, ?err, "failed to parse wf state");
						continue;
					}
				}
			}
			_ => {
				tracing::error!(?actor_id, ?workflow_id, wf_name=?wf.name, "unknown actor wf name");
				continue;
			}
		};

		actor_data.push((actor_id, wf, actor_state));
	}

	// Fetch runner pool errors if requested
	let runner_pool_errors = if fetch_error {
		fetch_runner_pool_errors(ctx, &actor_data).await?
	} else {
		HashMap::new()
	};

	// Build actors with resolved errors
	let mut actors = Vec::with_capacity(actor_data.len());
	for (actor_id, wf, actor_state) in actor_data {
		let error = if wf.is_dead() {
			Some(rivet_types::actor::ActorError::InternalError)
		} else if fetch_error {
			actor_state.resolve_error(&runner_pool_errors)
		} else {
			None
		};

		let actor = match actor_state {
			ActorState::V1(s) => Actor {
				actor_id,
				name: s.name.clone(),
				key: s.key.clone(),
				namespace_id: s.namespace_id,
				datacenter: dc_name.to_string(),
				runner_name_selector: s.runner_name_selector,
				crash_policy: s.crash_policy,

				create_ts: s.create_ts,
				start_ts: s.start_ts,
				pending_allocation_ts: s.pending_allocation_ts,
				connectable_ts: s.connectable_ts,
				sleep_ts: s.sleep_ts,
				reschedule_ts: s.reschedule_ts,
				destroy_ts: s.destroy_ts,

				error,
			},
			ActorState::V2(s) => Actor {
				actor_id,
				name: s.name.clone(),
				key: s.key.clone(),
				namespace_id: s.namespace_id,
				datacenter: dc_name.to_string(),
				runner_name_selector: s.pool_name,
				crash_policy: s.crash_policy,

				create_ts: s.create_ts,
				start_ts: s.start_ts,
				pending_allocation_ts: None,
				connectable_ts: s.connectable_ts,
				sleep_ts: s.sleep_ts,
				reschedule_ts: s.reschedule_ts,
				destroy_ts: s.destroy_ts,

				error,
			},
		};

		actors.push(actor);
	}

	Ok(actors)
}

/// Fetches runner pool errors for actors with NoCapacity failures.
async fn fetch_runner_pool_errors(
	ctx: &OperationCtx,
	actor_data: &[(Id, &WorkflowData, ActorState)],
) -> Result<HashMap<(Id, String), rivet_types::actor::RunnerPoolError>> {
	// Collect unique (namespace_id, runner_name) pairs that need error checks
	let runners_needing_check: Vec<_> = actor_data
		.iter()
		.filter(|(_, _, state)| state.is_error_no_capacity())
		.map(|(_, _, state)| (state.namespace_id(), state.pool_name().to_string()))
		.collect::<HashSet<_>>()
		.into_iter()
		.collect();

	if runners_needing_check.is_empty() {
		return Ok(HashMap::new());
	}

	let errors = ctx
		.op(crate::ops::runner_config::get_error::Input {
			runners: runners_needing_check,
		})
		.await?;

	Ok(errors
		.into_iter()
		.map(|e| ((e.namespace_id, e.runner_name), e.error))
		.collect())
}
