use super::types::*;
use super::{effect::*, event::*, state::*};
use crate::heartbeat::*;
use rivet_state_machine_util::{IoCall, IoTimer, LeaseChange};

pub(super) fn read_local_heartbeat(state: &mut State, out: &mut Effects) {
	let Some(worker_registration) = state.lease.worker_registration().cloned() else {
		return;
	};

	let op = state.sequence.next(Slot::Heartbeat);
	let timeout_ms = heartbeat_timeout_ms(state, out.now_boot_ms());
	let call = IoCall::arm(op, timeout_ms, Timer::Io, out);
	state.calls.insert(op.slot, call);

	out.execute(Effect::ReadLocalHeartbeatRequest {
		op,
		worker_registration,
	});
}

/// The I/O budget for one heartbeat request. A live lease gives each renewal attempt
/// `heartbeat_attempt_ms`, cut short at the lease deadline because no answer after it can
/// help: the deadline fences first. Registration keeps the ordinary `io_timeout_ms`, bounded
/// by its own registration deadline.
fn heartbeat_timeout_ms(state: &State, now_boot_ms: u64) -> u64 {
	let lease = &state.config.lease;
	if !matches!(state.lease, LeaseState::Live { .. }) {
		return lease.io_timeout_ms;
	}

	match state.worker_lease.deadline_boot_ms() {
		Some(deadline_boot_ms) => {
			let remaining_ms = deadline_boot_ms.saturating_sub(now_boot_ms).max(1);
			lease.heartbeat_attempt_ms.min(remaining_ms)
		}
		None => lease.heartbeat_attempt_ms,
	}
}

/// How long a live lease's renewal write may run: until the current lease deadline, after
/// which no answer can help because the deadline fences first. `None` outside a live lease.
fn renewal_lifetime_ms(state: &State, now_boot_ms: u64) -> Option<u64> {
	if !matches!(state.lease, LeaseState::Live { .. }) {
		return None;
	}
	let lifetime_ms = match state.worker_lease.deadline_boot_ms() {
		Some(deadline_boot_ms) => deadline_boot_ms.saturating_sub(now_boot_ms).max(1),
		None => state.config.lease.serving_ms(),
	};
	Some(lifetime_ms)
}

/// `Timer::RenewalAttempt`: the current renewal write has waited its attempt budget. It keeps
/// running as a superseded write, and a fresh CAS write on the same confirmed ETag starts. At
/// most one of them can land, and each answer is credited only to the write it carried.
fn supersede_renewal(state: &mut State, now_boot_ms: u64, out: &mut Effects) {
	let LeaseState::Live {
		phase: RenewalPhase::WritingHeartbeat { write_sequence },
		..
	} = &state.lease
	else {
		return;
	};
	let write_sequence = *write_sequence;
	let Some(op) = state.pending(Slot::Heartbeat) else {
		return;
	};

	state.complete(op);
	out.cancel_timer(Timer::Io(IoTimer::Io(op)));
	if let LeaseState::Live {
		superseded_writes, ..
	} = &mut state.lease
	{
		superseded_writes.push(SupersededWrite { op, write_sequence });
	}
	crate::metrics::HEARTBEAT_RENEWAL_ATTEMPTS
		.with_label_values(&["superseded"])
		.inc();

	write_renewal(state, now_boot_ms, out);
}

/// An answer to a superseded renewal write. A confirmation credits exactly that write at its
/// send time; a conflict or unknown outcome changes nothing, since the current attempt or its
/// readback still decides. Returns false when `op` is not a superseded write.
fn handle_superseded_response(
	state: &mut State,
	op: OperationId,
	result: &Write,
	now_boot_ms: u64,
	out: &mut Effects,
) -> bool {
	let LeaseState::Live {
		superseded_writes, ..
	} = &mut state.lease
	else {
		return false;
	};
	let Some(index) = superseded_writes.iter().position(|write| write.op == op) else {
		return false;
	};
	let superseded = superseded_writes.remove(index);

	match result {
		Write::Confirmed(etag) => {
			crate::metrics::HEARTBEAT_RENEWAL_ATTEMPTS
				.with_label_values(&["late_confirmed"])
				.inc();
			confirm(
				state,
				superseded.write_sequence,
				etag.clone(),
				now_boot_ms,
				out,
			);
		}
		Write::Conflict | Write::Unknown => {}
	}
	true
}

fn emit_write(
	state: &mut State,
	worker_registration: WorkerRegistration,
	write: &HeartbeatWrite,
	expected_etag: Option<String>,
	out: &mut Effects,
) {
	let heartbeat = Heartbeat {
		storage_capacity_millicores: worker_registration.storage_capacity_millicores,
		build_version: worker_registration.build_version.clone().into(),
		build_git_sha: worker_registration.build_git_sha.clone().into(),
		started_at_unix_ms: worker_registration.started_at_unix_ms,
		instance_id: *worker_registration.identity.instance_id.as_bytes(),
		api_peer_address: worker_registration.api_peer_address.clone().into(),
		write_sequence: write.write_sequence,
		state: write.state.clone(),
		versions: worker_registration.versions.to_vec(),
		directory_lock: worker_registration.directory_lock.clone(),
	};

	let op = state.sequence.next(Slot::Heartbeat);
	let now_boot_ms = out.now_boot_ms();
	let attempt_ms = heartbeat_timeout_ms(state, now_boot_ms);
	// A live renewal's request keeps running until the lease deadline, because a late answer
	// still confirms its write; `RenewalAttempt` starts the next attempt beside it instead.
	let timeout_ms = match renewal_lifetime_ms(state, now_boot_ms) {
		Some(lifetime_ms) => {
			out.arm_timer_after(Timer::RenewalAttempt, attempt_ms);
			lifetime_ms
		}
		None => attempt_ms,
	};
	let call = IoCall::arm(op, timeout_ms, Timer::Io, out);
	state.calls.insert(op.slot, call);

	out.execute(Effect::WriteLocalHeartbeatRequest {
		op,
		worker_registration,
		heartbeat,
		expected_etag,
	});
}

pub(super) fn write_claim(
	state: &mut State,
	observed: Option<TakeoverObservation>,
	retained_claim: Option<HeartbeatWrite>,
	now_boot_ms: u64,
	out: &mut Effects,
) {
	let LeaseState::Registering { participation, .. } = &mut state.lease else {
		return;
	};

	// An uncertain claim may still apply. Retries retain its payload and original deadline.
	let write = retained_claim.unwrap_or_else(|| HeartbeatWrite {
		write_sequence: 1,
		issued_boot_ms: now_boot_ms,
		state: (*participation).into(),
	});
	let request = ClaimAttempt { write, observed };
	super::publication::claim(state, request, now_boot_ms, out);
}

pub(super) fn write_renewal(state: &mut State, now_boot_ms: u64, out: &mut Effects) {
	let LeaseState::Live {
		worker_registration,
		participation,
		confirmed_write,
		unconfirmed_writes,
		phase,
		..
	} = &mut state.lease
	else {
		return;
	};

	let latest = unconfirmed_writes.last().unwrap_or(&confirmed_write.write);
	let write = HeartbeatWrite {
		write_sequence: latest
			.write_sequence
			.checked_add(1)
			.expect("heartbeat write sequence exhausted"),
		issued_boot_ms: now_boot_ms,
		state: (*participation).into(),
	};
	unconfirmed_writes.push(write.clone());
	*phase = RenewalPhase::WritingHeartbeat {
		write_sequence: write.write_sequence,
	};
	let worker_registration = worker_registration.clone();
	let expected_etag = Some(confirmed_write.etag.clone());

	emit_write(state, worker_registration, &write, expected_etag, out);
}

fn renew_later(state: &mut State, now_boot_ms: u64, out: &mut Effects) {
	let LeaseState::Live {
		confirmed_write,
		unconfirmed_writes,
		..
	} = &state.lease
	else {
		return;
	};

	let issued_boot_ms = unconfirmed_writes
		.last()
		.unwrap_or(&confirmed_write.write)
		.issued_boot_ms;
	let at_boot_ms =
		now_boot_ms.max(issued_boot_ms.saturating_add(state.config.lease.renew_interval_ms));

	out.arm_timer_at(Timer::Renew, at_boot_ms);
}

pub(super) fn confirm(
	state: &mut State,
	write_sequence: u64,
	etag: String,
	now_boot_ms: u64,
	out: &mut Effects,
) {
	let write = match &state.lease {
		LeaseState::Registering { phase, .. } => phase
			.retained_claim()
			.filter(|write| write.write_sequence == write_sequence),
		LeaseState::Live {
			unconfirmed_writes, ..
		} => unconfirmed_writes
			.iter()
			.find(|write| write.write_sequence == write_sequence),
		LeaseState::Observer
		| LeaseState::Releasing { .. }
		| LeaseState::Fenced { .. }
		| LeaseState::Stopped { .. } => None,
	}
	.cloned();
	let Some(write) = write else {
		fence(state, FenceReason::ForeignHeartbeat, out);
		return;
	};

	let deadline_boot_ms = write
		.issued_boot_ms
		.saturating_add(state.config.lease.serving_ms());

	if state.worker_lease.observe(Some(deadline_boot_ms), out) == LeaseChange::Fenced {
		fence(state, FenceReason::Expired, out);
		return;
	}

	let confirmed_state = write.state.clone();
	let issued_boot_ms = write.issued_boot_ms;
	let latency_ms = now_boot_ms.saturating_sub(issued_boot_ms);
	let late_after_ms = renewal_late_after_ms(&state.config.lease);
	let confirmed = ConfirmedHeartbeatWrite {
		write,
		etag,
		confirmed_boot_ms: now_boot_ms,
	};
	let registering = matches!(state.lease, LeaseState::Registering { .. });

	match &mut state.lease {
		LeaseState::Registering {
			worker_registration,
			participation,
			..
		} => {
			state.lease = LeaseState::Live {
				worker_registration: worker_registration.clone(),
				participation: *participation,
				confirmed_write: confirmed,
				unconfirmed_writes: Vec::new(),
				superseded_writes: Vec::new(),
				phase: RenewalPhase::Idle,
				claimed_boot_ms: now_boot_ms,
				renewal_late: false,
			};
		}
		LeaseState::Live {
			confirmed_write,
			unconfirmed_writes,
			superseded_writes,
			phase,
			renewal_late,
			..
		} => {
			// A renewal that confirms before the lease reaches half its window ends a late
			// episode. Slower ones leave it open, so sustained store latency warns once.
			let lease_age_ms = now_boot_ms.saturating_sub(confirmed_write.write.issued_boot_ms);
			let recovered = *renewal_late && lease_age_ms < late_after_ms;
			if recovered {
				*renewal_late = false;
			}

			unconfirmed_writes.retain(|write| write.write_sequence > write_sequence);
			*confirmed_write = confirmed;
			*phase = RenewalPhase::Idle;
			out.execute(Effect::RecordRenewal {
				latency_ms,
				remaining_lease_ms: deadline_boot_ms.saturating_sub(now_boot_ms),
				recovered,
			});
			// Every write or readback still in flight was issued against the ETag this write
			// replaced, so none of their answers can confirm anything more.
			superseded_writes.clear();
			if let Some(op) = state.calls.get(&Slot::Heartbeat).map(|call| call.op) {
				state.calls.remove(&Slot::Heartbeat);
				out.cancel_timer(Timer::Io(IoTimer::Io(op)));
			}
		}
		LeaseState::Observer
		| LeaseState::Releasing { .. }
		| LeaseState::Fenced { .. }
		| LeaseState::Stopped { .. } => return,
	}

	out.cancel_timer(Timer::Registration);
	out.cancel_timer(Timer::Retry);
	out.cancel_timer(Timer::Takeover);
	out.cancel_timer(Timer::RenewalAttempt);
	out.arm_timer_at(
		Timer::RenewalLate,
		issued_boot_ms.saturating_add(late_after_ms),
	);

	renew_later(state, now_boot_ms, out);

	if registering {
		// A takeover listing still in flight was read before this claim, so its answer
		// cannot anchor the store clock on the new heartbeat. Start a fresh one instead.
		if let Some(op) = state.pending(Slot::Discovery) {
			state.complete(op);
			out.cancel_timer(Timer::Io(IoTimer::Io(op)));
		}
		list_storage_workers(state, "start", now_boot_ms, out);
	}

	if state.lease.heartbeat_state() != Some(confirmed_state) {
		write_renewal(state, now_boot_ms, out);
	}
}

/// Half the serving window, measured from a confirmed write's send time like the deadline.
/// A renewal still unconfirmed this late has used half the time left before the fence, and
/// the lease degrades at the same instant: see `LeaseTiming::degrade_after_ms`.
fn renewal_late_after_ms(lease: &crate::config::LeaseTiming) -> u64 {
	lease.degrade_after_ms()
}

/// Reports a live lease that reached half its serving window without a newer confirmation,
/// once per episode. The alarm never fires early and each confirmation rearms it, so the
/// confirmed write's age is at least the threshold here.
fn report_renewal_late(state: &mut State, now_boot_ms: u64, out: &mut Effects) {
	let deadline_boot_ms = state.worker_lease.deadline_boot_ms();
	let LeaseState::Live {
		worker_registration,
		confirmed_write,
		unconfirmed_writes,
		renewal_late,
		..
	} = &mut state.lease
	else {
		return;
	};
	if *renewal_late {
		return;
	}
	*renewal_late = true;

	let report = RenewalLateReport {
		worker_id: worker_registration.identity.worker_id.clone(),
		since_confirmed_send_ms: now_boot_ms.saturating_sub(confirmed_write.write.issued_boot_ms),
		pending_renewal_wait_ms: unconfirmed_writes
			.first()
			.map(|write| now_boot_ms.saturating_sub(write.issued_boot_ms)),
		unconfirmed_writes: unconfirmed_writes.len(),
		remaining_lease_ms: deadline_boot_ms
			.map(|deadline_boot_ms| deadline_boot_ms.saturating_sub(now_boot_ms))
			.unwrap_or(0),
	};
	out.execute(Effect::RecordRenewalLate { report });
}

/// Lists storage workers early because a caller or a point read saw evidence that the
/// view is stale. At most one listing is in flight, whose response also answers this
/// hint. Hints start a listing at most every `refresh_hint_interval_ms`; a throttled hint
/// moves `Timer::Refresh` to the end of that spacing. A hint never touches the lease.
pub(super) fn hint_refresh(
	state: &mut State,
	trigger: &'static str,
	now_boot_ms: u64,
	out: &mut Effects,
) {
	if !state.discovering() || state.pending(Slot::Discovery).is_some() {
		return;
	}
	let spacing_ends_at = state
		.discovery
		.last_list_issued_boot_ms
		.map(|last| last.saturating_add(state.config.lease.refresh_hint_interval_ms));
	match spacing_ends_at {
		Some(at) if at > now_boot_ms => out.arm_timer_at(Timer::Refresh, at),
		Some(_) | None => list_storage_workers(state, trigger, now_boot_ms, out),
	}
}

/// Starts one listing unless one is in flight. `trigger` is a code-defined metric label.
pub(super) fn list_storage_workers(
	state: &mut State,
	trigger: &'static str,
	now_boot_ms: u64,
	out: &mut Effects,
) {
	if state.pending(Slot::Discovery).is_some() {
		return;
	}
	crate::metrics::DISCOVERY_REFRESHES
		.with_label_values(&[trigger])
		.inc();

	state.discovery.last_list_issued_boot_ms = Some(now_boot_ms);
	out.cancel_timer(Timer::Refresh);

	let op = state.sequence.next(Slot::Discovery);
	let call = IoCall::arm(op, state.config.discovery_timeout_ms, Timer::Io, out);
	state.calls.insert(op.slot, call);

	out.execute(Effect::ListStorageWorkersRequest {
		op,
		read_timeout_ms: state.config.lease.io_timeout_ms,
		worker_registration: state.lease.worker_registration().cloned(),
		known: state.discovery.entries.clone().unwrap_or_default(),
	});
}

pub(super) fn handle_response(
	state: &mut State,
	event: Event,
	now_boot_ms: u64,
	out: &mut Effects,
) {
	if matches!(
		state.lease,
		LeaseState::Registering {
			phase: RegistrationPhase::Publishing(_),
			..
		}
	) && matches!(
		event,
		Event::ReadLocalHeartbeatResponse { .. } | Event::WriteLocalHeartbeatResponse { .. }
	) {
		super::publication::response(state, event, now_boot_ms, out);
		return;
	}
	let Some(op) = event.operation_id() else {
		return;
	};
	if let Event::WriteLocalHeartbeatResponse { result, .. } = &event
		&& handle_superseded_response(state, op, result, now_boot_ms, out)
	{
		return;
	}

	// A matching generation cannot make a response valid for a different kind of I/O.
	let valid_kind = match &event {
		Event::ReadLocalHeartbeatResponse { .. } => {
			op.slot == Slot::Heartbeat
				&& matches!(
					&state.lease,
					LeaseState::Registering {
						phase: RegistrationPhase::ReadingHeartbeat { .. },
						..
					} | LeaseState::Live {
						phase: RenewalPhase::ReadingHeartbeat,
						..
					}
				)
		}
		Event::WriteLocalHeartbeatResponse { .. } => {
			op.slot == Slot::Heartbeat && writing_heartbeat(&state.lease)
		}
		Event::ListStorageWorkersResponse { .. }
		| Event::ListStorageWorkersPartialResponse { .. } => op.slot == Slot::Discovery,
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
		| Event::StopRequest => false,
	};

	if !valid_kind || !state.complete(op) {
		return;
	}
	out.cancel_timer(Timer::Io(IoTimer::Io(op)));
	if op.slot == Slot::Heartbeat {
		out.cancel_timer(Timer::RenewalAttempt);
	}

	match event {
		Event::ReadLocalHeartbeatResponse { result, .. } => match state.lease {
			LeaseState::Registering { .. } => {
				handle_initial_heartbeat_read(state, result, now_boot_ms, out)
			}
			LeaseState::Live { .. } => {
				handle_renewal_heartbeat_read(state, result, now_boot_ms, out)
			}
			LeaseState::Observer
			| LeaseState::Releasing { .. }
			| LeaseState::Fenced { .. }
			| LeaseState::Stopped { .. } => {}
		},
		Event::WriteLocalHeartbeatResponse { result, .. } => match result {
			Write::Confirmed(etag) => {
				// The response answers the one write this phase sent, so it confirms that write's
				// sequence and send time, never a later or earlier write's.
				let write_sequence = match &state.lease {
					LeaseState::Live {
						phase: RenewalPhase::WritingHeartbeat { write_sequence },
						..
					} => *write_sequence,
					LeaseState::Live { .. }
					| LeaseState::Observer
					| LeaseState::Registering { .. }
					| LeaseState::Releasing { .. }
					| LeaseState::Fenced { .. }
					| LeaseState::Stopped { .. } => return,
				};

				confirm(state, write_sequence, etag, now_boot_ms, out);
			}
			// Another write changed the ETag, possibly an earlier write of ours that landed late,
			// so only a readback can tell which.
			Write::Conflict => {
				match &mut state.lease {
					LeaseState::Live { phase, .. } => *phase = RenewalPhase::ReadingHeartbeat,
					LeaseState::Observer
					| LeaseState::Registering { .. }
					| LeaseState::Releasing { .. }
					| LeaseState::Fenced { .. }
					| LeaseState::Stopped { .. } => return,
				}

				read_local_heartbeat(state, out);
			}
			Write::Unknown => retry_unknown_write(state, now_boot_ms, out),
		},
		Event::ListStorageWorkersResponse { result, .. } => {
			handle_list_storage_workers_response(state, result, false, now_boot_ms, out)
		}
		Event::ListStorageWorkersPartialResponse { entries, .. } => {
			handle_list_storage_workers_response(state, Some(entries), true, now_boot_ms, out)
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
		| Event::StopRequest => {}
	}
}

/// Replaces a renewal write whose outcome is unknown with a fresh CAS write on the same
/// confirmed ETag. At most one of them can land: if the abandoned write landed, the fresh one
/// conflicts and the readback confirms the abandoned write at its own, earlier send time. A
/// write that failed sooner than `io_timeout_ms` after its send waits out the rest of that
/// spacing first, so instant failures cannot loop.
fn retry_unknown_write(state: &mut State, now_boot_ms: u64, out: &mut Effects) {
	let LeaseState::Live {
		unconfirmed_writes,
		phase,
		..
	} = &mut state.lease
	else {
		return;
	};

	let RenewalPhase::WritingHeartbeat { write_sequence } = *phase else {
		return;
	};
	let issued_boot_ms = unconfirmed_writes
		.iter()
		.find(|write| write.write_sequence == write_sequence)
		.map_or(now_boot_ms, |write| write.issued_boot_ms);
	let retry_at_boot_ms = issued_boot_ms.saturating_add(state.config.lease.io_timeout_ms);
	if retry_at_boot_ms <= now_boot_ms {
		write_renewal(state, now_boot_ms, out);
	} else {
		*phase = RenewalPhase::Waiting {
			next: RenewalRetry::WriteHeartbeat,
		};
		out.arm_timer_at(Timer::Retry, retry_at_boot_ms);
	}
}

fn handle_list_storage_workers_response(
	state: &mut State,
	result: Option<Vec<Observation>>,
	partial: bool,
	now_boot_ms: u64,
	out: &mut Effects,
) {
	state.discovery.round = state
		.discovery
		.round
		.checked_add(1)
		.expect("discovery counter exhausted");
	state.discovery.ok = result.is_some() && !partial;

	let Some(mut entries) = result else {
		// An entry waiting for evidence of its expiry keeps a possibly dead worker listed, so
		// retry sooner than the full interval, backing off from the hint spacing so a store
		// outage does not multiply listings on every process.
		state.discovery.failed_listings = state.discovery.failed_listings.saturating_add(1);
		let lease = &state.config.lease;
		let retry_ms = match state.discovery.awaiting_expiry_evidence {
			true => {
				let shift = (state.discovery.failed_listings - 1).min(16);
				lease
					.refresh_hint_interval_ms
					.saturating_mul(1 << shift)
					.min(lease.refresh_interval_ms)
			}
			false => lease.refresh_interval_ms,
		};
		out.arm_timer_after(Timer::Refresh, retry_ms);
		return;
	};
	state.discovery.failed_listings = 0;
	state.discovery.partial = partial;
	if !partial {
		state.discovery.entries_listed_boot_ms = state.discovery.last_list_issued_boot_ms;
	}

	entries.sort_by(|a, b| a.worker_id.cmp(&b.worker_id));

	// Discovery may lag a confirmed renewal. It updates hints and store time only;
	// heartbeat renewal and uncertain-write readback own the local lease I/O.
	match &state.lease {
		LeaseState::Live {
			worker_registration,
			confirmed_write,
			..
		} => {
			let own = entries.iter().find(|entry| {
				entry.worker_id == worker_registration.identity.worker_id
					&& entry.role == worker_registration.role
					&& entry.etag == confirmed_write.etag
			});

			if let Some(own) = own {
				// Confirmation occurred after the store write, making this a lower bound.
				state.discovery.store_clock = Some(StoreClock {
					lower_unix_ms: own.modified_unix_ms,
					at_boot_ms: confirmed_write.confirmed_boot_ms,
				});
			}
		}
		LeaseState::Observer => {
			// Observe store time without trusting the local wall clock or listing stateless workers.
			state.discovery.store_clock = entries
				.iter()
				.map(|entry| entry.modified_unix_ms)
				.max()
				.map(|lower_unix_ms| StoreClock {
					lower_unix_ms,
					at_boot_ms: now_boot_ms,
				});
		}
		LeaseState::Registering { phase, .. } => {
			// The newest listed write is a store-time lower bound for the takeover wait. Keep
			// whichever bound is later, so a listing never moves the wait's end later.
			let listed = entries
				.iter()
				.map(|entry| entry.modified_unix_ms)
				.max()
				.map(|lower_unix_ms| StoreClock {
					lower_unix_ms,
					at_boot_ms: now_boot_ms,
				});
			let advances = match (&listed, &state.discovery.store_clock) {
				(Some(listed), Some(current)) => {
					listed.lower_bound(now_boot_ms) > current.lower_bound(now_boot_ms)
				}
				(Some(_), None) => true,
				(None, _) => false,
			};
			let waiting = match phase {
				RegistrationPhase::Waiting {
					next:
						RegistrationAction::ReadHeartbeat(HeartbeatReadContext::AfterTakeoverWait {
							observed,
							observed_boot_ms,
							..
						}),
				} => Some((observed.clone(), *observed_boot_ms)),
				RegistrationPhase::Waiting { .. }
				| RegistrationPhase::ReadingHeartbeat { .. }
				| RegistrationPhase::Publishing(_) => None,
			};

			if advances {
				state.discovery.store_clock = listed;

				if let Some((observed, observed_boot_ms)) = waiting {
					let at_boot_ms = takeover_at_boot_ms(state, &observed, observed_boot_ms);
					out.arm_timer_at(Timer::Takeover, at_boot_ms.max(now_boot_ms));
				}
			}
		}
		LeaseState::Releasing { .. } | LeaseState::Fenced { .. } | LeaseState::Stopped { .. } => {}
	}

	// Storage watches are judged by the raw listing, before discovery drops old entries.
	let issued_boot_ms = state.discovery.last_list_issued_boot_ms.unwrap_or(0);
	super::watch::listing_observed(state, &entries, issued_boot_ms, now_boot_ms, out);
	state.discovery.entries = Some(entries);
	state.recompute_read_floor();

	// A storage worker that other live storage workers could outpace must stop serving
	// before it reads or replicates data at a version it cannot decode.
	if !partial {
		if let Some(incompatibility) = incompatible_version(state, now_boot_ms) {
			out.execute(Effect::RecordIncompatibleVersion { incompatibility });
			fence(state, FenceReason::IncompatibleVersion, out);
			return;
		}
	}

	// List again after the interval, or sooner when the view would start to expire so
	// discovery never drops a storage worker that is still renewing.
	state.refresh_discovery(now_boot_ms);
	let interval_at = now_boot_ms.saturating_add(state.config.lease.refresh_interval_ms);
	let expiry_at = state.discovery.valid_until_boot_ms;
	let mut next_at = if expiry_at > now_boot_ms {
		interval_at.min(expiry_at)
	} else {
		interval_at
	};
	if state.discovery.awaiting_expiry_evidence {
		next_at =
			next_at.min(now_boot_ms.saturating_add(state.config.lease.refresh_hint_interval_ms));
	}
	out.arm_timer_at(Timer::Refresh, next_at);
}

/// Checks a newly live storage registration against the other live storage workers in the
/// latest listing. Observers, stateless workers and registering workers are never checked:
/// the check runs only after the claim, so every writer listing later already counts this
/// worker, and only within `ttl_ms` of it, while an older view may still be in use.
fn incompatible_version(
	state: &State,
	now_boot_ms: u64,
) -> Option<crate::versions::Incompatibility> {
	let LeaseState::Live {
		worker_registration,
		claimed_boot_ms,
		..
	} = &state.lease
	else {
		return None;
	};
	if worker_registration.role != crate::worker::WorkerRole::Storage {
		return None;
	}
	// Past the window, every writer's storage view was listed after this worker registered,
	// so each counts it and selects nothing it cannot read. Checking later would fence the
	// last old worker of an ordinary rolling deploy, whose peers still select its versions.
	if now_boot_ms >= claimed_boot_ms.saturating_add(state.config.lease.ttl_ms) {
		return None;
	}

	let own_id = &worker_registration.identity.worker_id;
	let store_now = state
		.discovery
		.store_clock
		.as_ref()
		.map(|clock| clock.lower_bound(now_boot_ms));
	let ttl_ms = state.config.lease.ttl_ms as i128;
	let others: Vec<&[crate::versions::FamilyVersions]> = state
		.reader_entries()
		.filter(|entry| &entry.worker_id != own_id)
		.filter(|entry| {
			store_now
				.is_some_and(|now| (now as i128 - entry.modified_unix_ms as i128).max(0) < ttl_ms)
		})
		.map(|entry| entry.heartbeat.versions.as_slice())
		.collect();
	crate::versions::incompatibility(&worker_registration.versions, &others)
}

fn wait_registration(
	state: &mut State,
	next: RegistrationAction,
	timer: Timer,
	until_boot_ms: u64,
	out: &mut Effects,
) {
	if let LeaseState::Registering { phase, .. } = &mut state.lease {
		*phase = RegistrationPhase::Waiting { next };
		out.arm_timer_at(timer, until_boot_ms);
	}
}

pub(super) fn handle_initial_heartbeat_read(
	state: &mut State,
	result: Read,
	now_boot_ms: u64,
	out: &mut Effects,
) {
	let LeaseState::Registering {
		phase: RegistrationPhase::ReadingHeartbeat { context },
		worker_registration,
		..
	} = &state.lease
	else {
		return;
	};
	let retained_claim = context.retained_claim().cloned();
	let retry_at = now_boot_ms.saturating_add(state.config.lease.io_timeout_ms);
	match result {
		Read::Missing => {
			if let HeartbeatReadContext::ResolveClaim(attempt) = context {
				wait_registration(
					state,
					RegistrationAction::WriteHeartbeat(attempt.write.clone()),
					Timer::Retry,
					retry_at,
					out,
				);
			} else {
				write_claim(state, None, retained_claim, now_boot_ms, out);
			}
		}
		Read::Unknown => {
			wait_registration(
				state,
				RegistrationAction::ReadHeartbeat(context.clone()),
				Timer::Retry,
				retry_at,
				out,
			);
		}
		Read::Found(entry) => {
			if retained_claim.is_some()
				&& entry.heartbeat.instance_id
					== *worker_registration.identity.instance_id.as_bytes()
				&& entry.heartbeat.write_sequence == 1
			{
				confirm(state, 1, entry.etag, now_boot_ms, out);
				return;
			}
			let observed = TakeoverObservation {
				etag: entry.etag,
				modified_unix_ms: entry.modified_unix_ms,
			};
			let waited = match context {
				HeartbeatReadContext::AfterTakeoverWait { observed, .. }
				| HeartbeatReadContext::ResolveClaim(ClaimAttempt {
					observed: Some(observed),
					..
				}) => Some(observed),
				HeartbeatReadContext::Initial
				| HeartbeatReadContext::ResolveClaim(ClaimAttempt { observed: None, .. }) => None,
			};
			let exited = predecessor_exited(worker_registration, &entry.heartbeat);
			if entry.heartbeat.state == HeartbeatState::Released
				|| exited || waited == Some(&observed)
			{
				if exited {
					crate::metrics::EXITED_PREDECESSOR_CLAIMS.inc();
					tracing::info!(
						predecessor = %uuid::Uuid::from_bytes(entry.heartbeat.instance_id),
						"claiming the heartbeat of an exited predecessor without the takeover wait"
					);
				}
				write_claim(state, Some(observed), retained_claim, now_boot_ms, out);
				return;
			}
			let takeover_at_boot_ms = takeover_at_boot_ms(state, &observed, now_boot_ms);
			wait_registration(
				state,
				RegistrationAction::ReadHeartbeat(HeartbeatReadContext::AfterTakeoverWait {
					observed,
					observed_boot_ms: now_boot_ms,
					retained_claim,
				}),
				Timer::Takeover,
				takeover_at_boot_ms.max(now_boot_ms),
				out,
			);

			// A storage listing gives a store-time lower bound that can end the wait early.
			list_storage_workers(state, "takeover", now_boot_ms, out);
		}
	}
}

/// Whether `heartbeat` was written by an earlier process that has exited, proven by the log
/// directory lock rather than by time.
///
/// Every storage process takes the exclusive lock on its log directory before it registers
/// and keeps it until its storage services, including every tablet maintainer, have stopped.
/// It advertises the lock's scope, one kernel boot and one inode, in each heartbeat. This
/// process holds that lock now. If the heartbeat names another process under the same scope,
/// that process took the same lock under the same kernel and has since released it, so it
/// serves nothing. A different or absent scope proves nothing: a disk attached to another
/// machine, a reboot, or a copied directory all report another scope, and their predecessors
/// still wait out the lease.
///
/// The claim stays a CAS on the observed ETag, so it replaces exactly the heartbeat this
/// judged. A late renewal of the exited process that lands after the read changes the ETag,
/// and the re-read judges that heartbeat in turn.
fn predecessor_exited(registration: &WorkerRegistration, heartbeat: &Heartbeat) -> bool {
	let Some(own_lock) = &registration.directory_lock else {
		return false;
	};
	heartbeat.instance_id != *registration.identity.instance_id.as_bytes()
		&& heartbeat.directory_lock.as_ref() == Some(own_lock)
}

/// Boot time at which an unchanged Active or Draining heartbeat can no longer belong to a
/// serving process, so `Timer::Takeover` may re-read it and claim on an unchanged ETag.
///
/// The fallback is `ttl_ms + zombie_grace_ms` after the read, which is safe with no store
/// time because `Last-Modified` precedes the read. With a store-time lower bound the wait
/// can end earlier, once that bound passes `Last-Modified` by the same wait plus
/// `store_timestamp_uncertainty_ms`, the rule `worker_lease::judge` uses. The previous
/// process serves only until its last confirmed renewal's send time plus `ttl_ms` minus
/// `safety_margin_ms`, and that send time precedes `Last-Modified`; config validation keeps
/// `zombie_grace_ms + safety_margin_ms` above the uncertainty. The bound may be anchored after
/// the read: a renewal that lands after the read changes the ETag, so the re-read restarts the
/// wait and the claim CAS fails.
fn takeover_at_boot_ms(
	state: &State,
	observed: &TakeoverObservation,
	observed_boot_ms: u64,
) -> u64 {
	let lease = &state.config.lease;
	let wait_ms = lease.ttl_ms.saturating_add(lease.zombie_grace_ms);
	let boot_at_ms = observed_boot_ms.saturating_add(wait_ms);

	let Some(store_clock) = &state.discovery.store_clock else {
		return boot_at_ms;
	};
	let store_wait_ms = wait_ms.saturating_add(lease.store_timestamp_uncertainty_ms);
	let store_expires_unix_ms = i128::from(observed.modified_unix_ms) + i128::from(store_wait_ms);
	let remaining_ms = store_expires_unix_ms - i128::from(store_clock.lower_unix_ms);
	let store_at_ms =
		(i128::from(store_clock.at_boot_ms) + remaining_ms).clamp(0, i128::from(u64::MAX)) as u64;

	boot_at_ms.min(store_at_ms)
}

fn handle_renewal_heartbeat_read(
	state: &mut State,
	result: Read,
	now_boot_ms: u64,
	out: &mut Effects,
) {
	let LeaseState::Live {
		worker_registration,
		confirmed_write,
		unconfirmed_writes,
		..
	} = &state.lease
	else {
		return;
	};

	match result {
		Read::Missing => fence(state, FenceReason::MissingHeartbeat, out),
		Read::Unknown => {
			let until_boot_ms = now_boot_ms.saturating_add(state.config.lease.io_timeout_ms);
			if let LeaseState::Live { phase, .. } = &mut state.lease {
				*phase = RenewalPhase::Waiting {
					next: RenewalRetry::ReadHeartbeat,
				};
			}
			out.arm_timer_at(Timer::Retry, until_boot_ms);
		}
		Read::Found(entry) => {
			let confirmed_sequence = confirmed_write.write.write_sequence;
			let last_issued_sequence = unconfirmed_writes
				.last()
				.unwrap_or(&confirmed_write.write)
				.write_sequence;

			if entry.heartbeat.instance_id != *worker_registration.identity.instance_id.as_bytes()
				|| entry.heartbeat.write_sequence < confirmed_sequence
				|| entry.heartbeat.write_sequence > last_issued_sequence
			{
				fence(state, FenceReason::ForeignHeartbeat, out);
			} else if entry.heartbeat.write_sequence > confirmed_sequence {
				confirm(
					state,
					entry.heartbeat.write_sequence,
					entry.etag,
					now_boot_ms,
					out,
				);
			} else {
				if let LeaseState::Live { phase, .. } = &mut state.lease {
					*phase = RenewalPhase::Idle;
				}

				renew_later(state, now_boot_ms, out);
			}
		}
	}
}

fn writing_heartbeat(lease: &LeaseState) -> bool {
	matches!(
		lease,
		LeaseState::Live {
			phase: RenewalPhase::WritingHeartbeat { .. },
			..
		}
	)
}

pub(super) fn timer(state: &mut State, timer: Timer, now_boot_ms: u64, out: &mut Effects) {
	match timer {
		Timer::Publication(_) => super::publication::timer(state, timer, now_boot_ms, out),
		// The parent checked the appropriate domain deadline before dispatching this alarm.
		Timer::Registration | Timer::WorkerLease | Timer::ReleaseDeadline => {}
		Timer::RenewalLate => report_renewal_late(state, now_boot_ms, out),
		Timer::Renew
			if matches!(state.lease, LeaseState::Live { .. })
				&& state.pending(Slot::Heartbeat).is_none() =>
		{
			write_renewal(state, now_boot_ms, out);
		}
		Timer::RenewalAttempt => supersede_renewal(state, now_boot_ms, out),
		Timer::Refresh if state.discovering() => {
			list_storage_workers(state, "interval", now_boot_ms, out)
		}
		Timer::Retry | Timer::Takeover if state.pending(Slot::Heartbeat).is_none() => {
			match &mut state.lease {
				LeaseState::Registering {
					phase: RegistrationPhase::Waiting { next },
					..
				} => match next.clone() {
					RegistrationAction::ReadHeartbeat(context) => {
						if let LeaseState::Registering { phase, .. } = &mut state.lease {
							*phase = RegistrationPhase::ReadingHeartbeat { context };
						}
						read_local_heartbeat(state, out);
					}
					RegistrationAction::WriteHeartbeat(write) => {
						write_claim(state, None, Some(write), now_boot_ms, out)
					}
				},
				LeaseState::Live {
					phase: RenewalPhase::Waiting { next },
					..
				} => match *next {
					RenewalRetry::ReadHeartbeat => {
						if let LeaseState::Live { phase, .. } = &mut state.lease {
							*phase = RenewalPhase::ReadingHeartbeat;
						}
						read_local_heartbeat(state, out);
					}
					RenewalRetry::WriteHeartbeat => write_renewal(state, now_boot_ms, out),
				},
				LeaseState::Observer
				| LeaseState::Releasing { .. }
				| LeaseState::Fenced { .. }
				| LeaseState::Stopped { .. }
				| LeaseState::Registering {
					phase:
						RegistrationPhase::ReadingHeartbeat { .. } | RegistrationPhase::Publishing(_),
					..
				}
				| LeaseState::Live { .. } => {}
			}
		}

		Timer::Io(IoTimer::Io(op)) => {
			if state.pending(op.slot) != Some(op) {
				return;
			}
			let event = match op.slot {
				// Publication helpers use operation-scoped Timer::Publication alarms.
				Slot::Publication => return,
				Slot::Discovery => Event::ListStorageWorkersResponse { op, result: None },
				Slot::Heartbeat if writing_heartbeat(&state.lease) => {
					Event::WriteLocalHeartbeatResponse {
						op,
						result: Write::Unknown,
					}
				}
				Slot::Heartbeat => Event::ReadLocalHeartbeatResponse {
					op,
					result: Read::Unknown,
				},
			};

			handle_response(state, event, now_boot_ms, out);
		}
		Timer::Renew
		| Timer::Refresh
		| Timer::Retry
		| Timer::Takeover
		| Timer::LookupExpiry
		| Timer::WatchPoll => {}
	}
}

pub(super) fn stop(state: &mut State, out: &mut Effects) {
	super::lookup::stop(state, out);
	super::watch::stop(state, out);
	super::publication::cancel(state, out);

	// Cancellation is idempotent; the runner owns which of these timers are armed.
	for timer in [
		Timer::Registration,
		Timer::Renew,
		Timer::Refresh,
		Timer::ReleaseDeadline,
		Timer::Retry,
		Timer::Takeover,
		Timer::WorkerLease,
		Timer::RenewalLate,
		Timer::RenewalAttempt,
	] {
		out.cancel_timer(timer);
	}

	for slot in [Slot::Heartbeat, Slot::Discovery] {
		if let Some(op) = state.pending(slot) {
			state.complete(op);
			out.cancel_timer(Timer::Io(IoTimer::Io(op)));
		}
	}

	state.lease = LeaseState::Stopped {
		worker_registration: state.lease.worker_registration().cloned(),
	};
	state.discovery.valid_until_boot_ms = 0;
}

pub(super) fn fence(state: &mut State, reason: FenceReason, out: &mut Effects) {
	if !state.lease.terminal() {
		let report = fence_report(state, reason, out.now_boot_ms());
		out.execute(Effect::RecordFence { report });
	}
	stop(state, out);

	state.lease = LeaseState::Fenced {
		worker_registration: state.lease.worker_registration().cloned(),
		reason,
	};
}

/// Captures the lease timing a fence log needs before `stop` discards it.
fn fence_report(state: &State, reason: FenceReason, now_boot_ms: u64) -> FenceReport {
	let worker_id = state
		.lease
		.worker_registration()
		.map(|registration| registration.identity.worker_id.clone());
	let mut report = FenceReport {
		reason,
		worker_id,
		now_boot_ms,
		last_confirmed_issued_boot_ms: None,
		last_confirmed_latency_ms: None,
		deadline_boot_ms: state.worker_lease.deadline_boot_ms(),
		unconfirmed_write_age_ms: None,
		unconfirmed_writes: 0,
	};

	if let LeaseState::Live {
		confirmed_write,
		unconfirmed_writes,
		..
	} = &state.lease
	{
		let issued_boot_ms = confirmed_write.write.issued_boot_ms;
		let serving_ms = state.config.lease.serving_ms();
		report.last_confirmed_issued_boot_ms = Some(issued_boot_ms);
		// A confirmation that arrived too late has already replaced the watched deadline, so
		// report the deadline the last confirmed write established.
		report.deadline_boot_ms = Some(issued_boot_ms.saturating_add(serving_ms));
		report.last_confirmed_latency_ms = Some(
			confirmed_write
				.confirmed_boot_ms
				.saturating_sub(issued_boot_ms),
		);
		report.unconfirmed_write_age_ms = unconfirmed_writes
			.first()
			.map(|write| now_boot_ms.saturating_sub(write.issued_boot_ms));
		report.unconfirmed_writes = unconfirmed_writes.len();
	}

	report
}

pub(super) fn begin_release(state: &mut State, now_boot_ms: u64, out: &mut Effects) {
	let LeaseState::Live {
		worker_registration,
		confirmed_write,
		unconfirmed_writes,
		..
	} = &state.lease
	else {
		stop(state, out);
		return;
	};
	let worker_registration = worker_registration.clone();
	let deadline_boot_ms = confirmed_write
		.write
		.issued_boot_ms
		.saturating_add(state.config.lease.serving_ms());
	let write = HeartbeatWrite {
		write_sequence: unconfirmed_writes
			.last()
			.unwrap_or(&confirmed_write.write)
			.write_sequence
			.checked_add(1)
			.expect("heartbeat write sequence exhausted"),
		issued_boot_ms: now_boot_ms,
		state: HeartbeatState::Released,
	};
	let expected_etag = confirmed_write.etag.clone();

	// Invalidate renewal responses and stop discovery before publishing a non-serving snapshot.
	stop(state, out);
	super::publication::release(
		state,
		worker_registration,
		ReleaseAttempt {
			write,
			expected_etag,
		},
		deadline_boot_ms,
		now_boot_ms,
		out,
	);
	out.arm_timer_at(Timer::ReleaseDeadline, deadline_boot_ms);
}
