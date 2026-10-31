//! Maps shared conditional I/O into coordinator effects; leadership policy remains in the parent.
use super::*;

/// `known_version` is the retained manifest version; readback conditions on it so an
/// unchanged base costs no body transfer.
pub(super) fn effects(
	input: rivet_state_machine::Effects<cw::Effect<Mutation, OpId>, cw::Timer<OpId>>,
	known_version: Option<String>,
	out: &mut Effects,
) {
	out.extend_mapped(input, Timer::Publication, |effect| match effect {
		cw::Effect::ReadRequest { op } => Effect::ReadLeaseRequest {
			op,
			known_version: known_version.clone(),
		},
		cw::Effect::WriteRequest { op, request } => match request {
			Mutation::Acquire(attempt) => Effect::AcquireLeaseRequest { op, attempt },
			Mutation::StepDown {
				observed,
				commit_id,
				step_down_ms,
			} => Effect::StepDownRequest {
				op,
				observed,
				commit_id,
				step_down_ms,
			},
			Mutation::WithdrawStepDown {
				observed,
				commit_id,
			} => Effect::WithdrawStepDownRequest {
				op,
				observed,
				commit_id,
			},
			Mutation::Release {
				observed,
				commit_id,
			} => Effect::ReleaseLeaseRequest {
				op,
				observed,
				commit_id,
			},
		},
	});
}

pub(super) fn write_response(state: &mut State, op: OpId, out: &mut Effects) {
	// The lease adapter intentionally requires a readback for every PUT outcome.
	event(
		state,
		RuntimeEvent::Input(cw::Event::WriteResponse {
			op,
			result: cw::WriteResult::Unknown,
		}),
		out,
	);
}

pub(super) fn read_response(state: &mut State, op: OpId, result: LeaseRead, out: &mut Effects) {
	let Some(mutation) = state.control.mutation() else {
		return;
	};
	let observed = match &result {
		LeaseRead::Found(found) => Some(found),
		LeaseRead::Missing | LeaseRead::Unknown => None,
	};
	let classification = if result == LeaseRead::Unknown {
		cw::Readback::Unknown
	} else {
		let (confirmed, unchanged) = match mutation {
			Mutation::Acquire(attempt) => (
				observed.is_some_and(|found| found.tenure == attempt.tenure && !found.released),
				observed.map(|found| &found.version)
					== attempt.expected.as_ref().map(|base| &base.version),
			),
			Mutation::Release { observed: base, .. } => (
				false,
				observed.is_some_and(|found| found.version == base.version),
			),
			// Commit IDs are unique per write, so finding this one proves this write landed.
			Mutation::StepDown {
				observed: base,
				commit_id,
				..
			}
			| Mutation::WithdrawStepDown {
				observed: base,
				commit_id,
			} => (
				observed.is_some_and(|found| {
					found.commit_id == *commit_id && found.tenure == base.tenure && !found.released
				}),
				observed.is_some_and(|found| found.version == base.version),
			),
		};
		if confirmed {
			cw::Readback::Confirmed(result)
		} else if unchanged {
			cw::Readback::Unchanged
		} else {
			cw::Readback::Reconcile(result)
		}
	};
	event(
		state,
		RuntimeEvent::Input(cw::Event::ReadResponse {
			op,
			result: classification,
		}),
		out,
	);
}

pub(super) fn event(
	state: &mut State,
	input: RuntimeEvent<cw::Event<OpId, LeaseRead>, cw::Timer<OpId>>,
	out: &mut Effects,
) {
	if !can_work(state) {
		suspend(state, out);
		return;
	}
	let ControlState::Publishing(progress) = &mut state.control else {
		return;
	};
	let sequence = &mut state.sequence;
	let known_version = state
		.manifest
		.as_ref()
		.and_then(|manifest| manifest.lease.as_ref())
		.map(|lease| lease.version.clone());
	let mut child = rivet_state_machine::Effects::new(out.clocks());
	let update = progress.on_event(state.now_boot_ms, input, || sequence.next(Slot), &mut child);
	effects(child, known_version, out);
	let Some(outcome) = update else {
		return;
	};
	let ControlState::Publishing(progress) =
		std::mem::replace(&mut state.control, ControlState::Idle)
	else {
		unreachable!()
	};
	match outcome {
		cw::Outcome::Confirmed(cw::Confirmation::Read(LeaseRead::Found(found))) => {
			match progress.into_request() {
				Mutation::Acquire(attempt) => {
					let tenure = attempt.tenure;
					if state.shutdown || !state.eligible {
						state.services = ServicesState::SafeToRelease(tenure);
					} else {
						tracing::info!(
							coordinator_generation = tenure.coordinator_generation,
							"singleton tenure serving"
						);
						state.services = ServicesState::Running(tenure.clone());
						out.execute(Effect::StartScope { tenure });
					}
				}
				// The deadline was armed before the first send; the readback decides what next.
				Mutation::StepDown { .. } => {
					handle_read(state, LeaseRead::Found(found), out);
					return;
				}
				Mutation::WithdrawStepDown { .. } => {
					if let ServicesState::SteppingDown { tenure, .. } = state.services.clone()
						&& tenure == found.tenure
					{
						leave_step_down(state, tenure, out);
					}
					handle_read(state, LeaseRead::Found(found), out);
					return;
				}
				Mutation::Release { .. } => {
					unreachable!("release writes are never classified as confirmed")
				}
			}
			schedule(state, out);
		}
		cw::Outcome::Reconcile(observed) => handle_read(state, observed, out),
		cw::Outcome::Confirmed(_) => {
			unreachable!("coordinator always confirms through lease readback")
		}
	}
}
