use gas::db::WorkflowData;
use gas::prelude::*;
use rivet_types::actors::Actor;
use std::collections::{HashMap, HashSet};

use crate::workflows::actor::FailureReason as WorkflowFailureReason;

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

		let actor_state = match wf.parse_state::<Option<crate::workflows::actor::State>>() {
			Ok(Some(s)) => s,
			Ok(None) => {
				// Actor did not initialize state yet
				continue;
			}
			Err(err) => {
				tracing::error!(?actor_id, ?workflow_id, ?err, "failed to parse wf state");
				continue;
			}
		};

		actor_data.push((actor_id, actor_state));
	}

	// Fetch runner pool errors if requested
	let runner_pool_errors = if fetch_error {
		fetch_runner_pool_errors(ctx, &actor_data).await?
	} else {
		HashMap::new()
	};

	// Build actors with resolved errors
	let mut actors = Vec::with_capacity(actor_data.len());
	for (actor_id, actor_state) in actor_data {
		let error = if fetch_error {
			resolve_actor_error(
				&actor_state.failure_reason,
				&actor_state,
				&runner_pool_errors,
			)
		} else {
			None
		};

		actors.push(Actor {
			actor_id,
			name: actor_state.name.clone(),
			key: actor_state.key.clone(),
			namespace_id: actor_state.namespace_id,
			datacenter: dc_name.to_string(),
			runner_name_selector: actor_state.runner_name_selector,
			crash_policy: actor_state.crash_policy,

			create_ts: actor_state.create_ts,
			start_ts: actor_state.start_ts,
			pending_allocation_ts: actor_state.pending_allocation_ts,
			connectable_ts: actor_state.connectable_ts,
			sleep_ts: actor_state.sleep_ts,
			reschedule_ts: actor_state.reschedule_ts,
			destroy_ts: actor_state.destroy_ts,

			error,
		});
	}

	Ok(actors)
}

/// Fetches runner pool errors for actors with NoCapacity failures.
async fn fetch_runner_pool_errors(
	ctx: &OperationCtx,
	actor_data: &[(Id, crate::workflows::actor::State)],
) -> Result<HashMap<(Id, String), rivet_types::actor::RunnerPoolError>> {
	// Collect unique (namespace_id, runner_name) pairs that need error checks
	let runners_needing_check: Vec<_> = actor_data
		.iter()
		.filter(|(_, state)| {
			matches!(
				state.failure_reason,
				Some(WorkflowFailureReason::NoCapacity)
			)
		})
		.map(|(_, state)| (state.namespace_id, state.runner_name_selector.clone()))
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

/// Resolves an actor's error, enriching NoCapacity with RunnerPoolError if available.
fn resolve_actor_error(
	failure_reason: &Option<WorkflowFailureReason>,
	actor_state: &crate::workflows::actor::State,
	runner_pool_errors: &HashMap<(Id, String), rivet_types::actor::RunnerPoolError>,
) -> Option<rivet_types::actor::ActorError> {
	match failure_reason {
		Some(WorkflowFailureReason::NoCapacity) => {
			let key = (
				actor_state.namespace_id,
				actor_state.runner_name_selector.clone(),
			);
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
	}
}
