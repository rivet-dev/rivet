//! Embeds conditional I/O for claims and clean releases; takeover and lease policy stay in Topology.
use super::{effect::*, event::Event, lease, state::*, types::*};
use crate::heartbeat::{Heartbeat, HeartbeatState, Read, Write};
use rivet_object_store_util::conditional_write as cw;
use rivet_state_machine_ephemeral::RuntimeEvent;

/// Projects immutable heartbeat requests into the existing storage adapter.
trait PreparedHeartbeat {
	fn write(&self) -> &HeartbeatWrite;
	fn expected_etag(&self) -> Option<&str>;
}

impl PreparedHeartbeat for ClaimAttempt {
	fn write(&self) -> &HeartbeatWrite {
		&self.write
	}

	fn expected_etag(&self) -> Option<&str> {
		self.observed.as_ref().map(|p| p.etag.as_str())
	}
}

impl PreparedHeartbeat for ReleaseAttempt {
	fn write(&self) -> &HeartbeatWrite {
		&self.write
	}

	fn expected_etag(&self) -> Option<&str> {
		Some(&self.expected_etag)
	}
}

fn config(state: &State) -> cw::Config {
	cw::Config {
		io_timeout_ms: state.config.lease.io_timeout_ms,
		retry_delay_ms: state.config.lease.io_timeout_ms,
	}
}

fn effects<W: PreparedHeartbeat>(
	registration: &WorkerRegistration,
	input: rivet_state_machine_util::Effects<cw::Effect<W, OperationId>, cw::Timer<OperationId>>,
	out: &mut Effects,
) {
	out.extend_mapped(input, Timer::Publication, |effect| match effect {
		cw::Effect::ReadRequest { op } => Effect::ReadLocalHeartbeatRequest {
			op,
			worker_registration: registration.clone(),
		},
		cw::Effect::WriteRequest { op, request } => {
			let write = request.write();
			Effect::WriteLocalHeartbeatRequest {
				op,
				worker_registration: registration.clone(),
				expected_etag: request.expected_etag().map(str::to_owned),
				heartbeat: Heartbeat {
					storage_capacity_millicores: registration.storage_capacity_millicores,
					build_version: registration.build_version.clone().into(),
					build_git_sha: registration.build_git_sha.clone().into(),
					started_at_unix_ms: registration.started_at_unix_ms,
					instance_id: *registration.identity.instance_id.as_bytes(),
					api_peer_address: registration.api_peer_address.clone().into(),
					write_sequence: write.write_sequence,
					state: write.state.clone(),
					versions: registration.versions.to_vec(),
					directory_lock: registration.directory_lock.clone(),
				},
			}
		}
	});
}

pub(super) fn claim(state: &mut State, request: ClaimAttempt, now_boot_ms: u64, out: &mut Effects) {
	let settings = config(state);
	let op = state.publication_sequence.next(Slot::Publication);
	let LeaseState::Registering {
		worker_registration,
		phase,
		..
	} = &mut state.lease
	else {
		return;
	};
	let mut child = rivet_state_machine_util::Effects::new(out.clocks());
	let progress = cw::ConditionalWrite::start(request, settings, now_boot_ms, op, &mut child);
	*phase = RegistrationPhase::Publishing(progress);
	effects(worker_registration, child, out);
}

pub(super) fn release(
	state: &mut State,
	worker_registration: WorkerRegistration,
	request: ReleaseAttempt,
	deadline_boot_ms: u64,
	now_boot_ms: u64,
	out: &mut Effects,
) {
	let op = state.publication_sequence.next(Slot::Publication);
	let mut child = rivet_state_machine_util::Effects::new(out.clocks());
	let progress = cw::ConditionalWrite::start(request, config(state), now_boot_ms, op, &mut child);
	effects(&worker_registration, child, out);
	state.lease = LeaseState::Releasing {
		worker_registration,
		deadline_boot_ms,
		progress,
	};
}

fn readback<W: PreparedHeartbeat>(
	request: &W,
	registration: &WorkerRegistration,
	result: Read,
) -> cw::Readback<Read> {
	match &result {
		Read::Unknown => cw::Readback::Unknown,
		Read::Found(entry)
			if entry.heartbeat.instance_id == *registration.identity.instance_id.as_bytes()
				&& entry.heartbeat.write_sequence == request.write().write_sequence
				&& entry.heartbeat.state == request.write().state =>
		{
			cw::Readback::Confirmed(result)
		}
		Read::Missing if request.expected_etag().is_none() => cw::Readback::Unchanged,
		Read::Found(entry) if Some(entry.etag.as_str()) == request.expected_etag() => {
			cw::Readback::Unchanged
		}
		Read::Found(_) | Read::Missing => cw::Readback::Reconcile(result),
	}
}

pub(super) fn response(state: &mut State, event: Event, now_boot_ms: u64, out: &mut Effects) {
	let input = match event {
		Event::WriteLocalHeartbeatResponse { op, result } => cw::Event::WriteResponse {
			op,
			result: match result {
				Write::Confirmed(etag) => cw::WriteResult::Confirmed { etag },
				Write::Unknown | Write::Conflict => cw::WriteResult::Unknown,
			},
		},
		Event::ReadLocalHeartbeatResponse { op, result } => {
			let result = match &state.lease {
				LeaseState::Registering {
					worker_registration,
					phase: RegistrationPhase::Publishing(progress),
					..
				} => readback(progress.request(), worker_registration, result),
				LeaseState::Releasing {
					worker_registration,
					progress,
					..
				} => readback(progress.request(), worker_registration, result),
				LeaseState::Registering {
					phase:
						RegistrationPhase::ReadingHeartbeat { .. } | RegistrationPhase::Waiting { .. },
					..
				}
				| LeaseState::Observer
				| LeaseState::Live { .. }
				| LeaseState::Fenced { .. }
				| LeaseState::Stopped { .. } => return,
			};
			cw::Event::ReadResponse { op, result }
		}
		Event::ResolveEndpointRequest { .. }
		| Event::InvalidateEndpointRequest { .. }
		| Event::EndpointTimedOut { .. }
		| Event::InspectHeartbeatRequest { .. }
		| Event::WatchRequest { .. }
		| Event::UnwatchRequest { .. }
		| Event::ReadWatchedHeartbeatResponse { .. }
		| Event::LookupEndpointResponse { .. }
		| Event::ReadStorageHeartbeatResponse { .. }
		| Event::Refresh
		| Event::BeginDrain
		| Event::Fence(_)
		| Event::StopRequest
		| Event::ListStorageWorkersResponse { .. }
		| Event::ListStorageWorkersPartialResponse { .. } => return,
	};
	advance(state, RuntimeEvent::Input(input), now_boot_ms, out);
}

fn advance(
	state: &mut State,
	input: RuntimeEvent<cw::Event<OperationId, Read>, cw::Timer<OperationId>>,
	now_boot_ms: u64,
	out: &mut Effects,
) {
	match &mut state.lease {
		LeaseState::Registering {
			worker_registration,
			phase: RegistrationPhase::Publishing(progress),
			..
		} => {
			let mut child = rivet_state_machine_util::Effects::new(out.clocks());
			let update = progress.on_event(
				now_boot_ms,
				input,
				|| state.publication_sequence.next(Slot::Publication),
				&mut child,
			);
			effects(worker_registration, child, out);
			match update {
				Some(cw::Outcome::Confirmed(cw::Confirmation::Write { etag })) => {
					lease::confirm(state, 1, etag, now_boot_ms, out)
				}
				Some(cw::Outcome::Confirmed(cw::Confirmation::Read(Read::Found(entry)))) => {
					lease::confirm(state, 1, entry.etag, now_boot_ms, out)
				}
				Some(cw::Outcome::Reconcile(result)) => {
					let attempt = progress.request().clone();
					if let LeaseState::Registering { phase, .. } = &mut state.lease {
						*phase = RegistrationPhase::ReadingHeartbeat {
							context: HeartbeatReadContext::ResolveClaim(attempt),
						};
					}
					lease::handle_initial_heartbeat_read(state, result, now_boot_ms, out);
				}
				Some(_) => unreachable!("only a matching heartbeat can confirm a claim"),
				None => {}
			}
		}
		LeaseState::Releasing {
			worker_registration,
			progress,
			deadline_boot_ms,
		} => {
			let mut child = rivet_state_machine_util::Effects::new(out.clocks());
			let update = progress.on_event(
				now_boot_ms,
				input,
				|| state.publication_sequence.next(Slot::Publication),
				&mut child,
			);
			effects(worker_registration, child, out);
			match update {
				Some(cw::Outcome::Confirmed(_)) => lease::stop(state, out),
				Some(cw::Outcome::Reconcile(Read::Missing)) => {
					lease::fence(state, FenceReason::MissingHeartbeat, out)
				}
				Some(cw::Outcome::Reconcile(Read::Found(entry))) => {
					let request = progress.request();
					if entry.heartbeat.instance_id
						== *worker_registration.identity.instance_id.as_bytes()
						&& entry.heartbeat.state != HeartbeatState::Released
						&& entry.heartbeat.write_sequence < request.write.write_sequence
					{
						// Domain-authorized NEW CAS after a preceding renewal won; retain the
						// release payload and original confirmed lease's expiration bound.
						let next = ReleaseAttempt {
							write: request.write.clone(),
							expected_etag: entry.etag,
						};
						let registration = worker_registration.clone();
						let deadline = *deadline_boot_ms;
						release(state, registration, next, deadline, now_boot_ms, out);
					} else {
						lease::fence(state, FenceReason::ForeignHeartbeat, out);
					}
				}
				Some(_) => unreachable!("unknown reads remain pending in the helper"),
				None => {}
			}
		}
		LeaseState::Registering {
			phase: RegistrationPhase::ReadingHeartbeat { .. } | RegistrationPhase::Waiting { .. },
			..
		}
		| LeaseState::Observer
		| LeaseState::Live { .. }
		| LeaseState::Fenced { .. }
		| LeaseState::Stopped { .. } => {}
	}
}

pub(super) fn timer(state: &mut State, timer: Timer, now_boot_ms: u64, out: &mut Effects) {
	if let Timer::Publication(timer) = timer {
		advance(state, RuntimeEvent::Timer(timer), now_boot_ms, out)
	}
}

pub(super) fn cancel(state: &mut State, out: &mut Effects) {
	match &mut state.lease {
		LeaseState::Registering {
			worker_registration,
			phase: RegistrationPhase::Publishing(progress),
			..
		} => {
			let mut child = rivet_state_machine_util::Effects::new(out.clocks());
			progress.suspend(&mut child);
			effects(worker_registration, child, out);
		}
		LeaseState::Releasing {
			worker_registration,
			progress,
			..
		} => {
			let mut child = rivet_state_machine_util::Effects::new(out.clocks());
			progress.suspend(&mut child);
			effects(worker_registration, child, out);
		}
		LeaseState::Registering {
			phase: RegistrationPhase::ReadingHeartbeat { .. } | RegistrationPhase::Waiting { .. },
			..
		}
		| LeaseState::Observer
		| LeaseState::Live { .. }
		| LeaseState::Fenced { .. }
		| LeaseState::Stopped { .. } => {}
	}
}
