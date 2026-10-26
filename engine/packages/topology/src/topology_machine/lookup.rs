use super::types::*;
use super::{effect::*, state::*};
use crate::topology_machine::snapshot::{endpoint_hint, same_endpoint};
use crate::topology_machine::types::EndpointEntry;
use crate::worker::{Worker, WorkerId};

pub(super) fn handle_resolve_endpoint_request(
	state: &mut State,
	request_id: u64,
	worker: WorkerId,
	now_boot_ms: u64,
	out: &mut Effects,
) {
	if let Some(value) = endpoint_hint(
		state
			.storage_entries(now_boot_ms)
			.find(|entry| entry.worker_id == worker)
			.map(crate::heartbeat::Observation::worker),
		&state.lookups.entries,
		&state.lookups.invalidated,
		&worker,
		now_boot_ms,
	) {
		out.execute(Effect::ResolveEndpointResponse {
			request_id,
			result: Ok(value),
		});
		return;
	}

	if let Some(pending) = state.lookups.pending.get_mut(&worker) {
		if pending.waiters.len() >= 128 {
			out.execute(Effect::RecordLookupRejected);

			out.execute(Effect::ResolveEndpointResponse {
				request_id,
				result: Err(RequestError::Overloaded),
			});
		} else {
			pending.waiters.push(request_id);
		}
		return;
	}
	if state.lookups.pending.len() + state.lookups.heartbeats.len()
		>= state.config.lookup_concurrency
	{
		out.execute(Effect::RecordLookupRejected);

		out.execute(Effect::ResolveEndpointResponse {
			request_id,
			result: Err(RequestError::Overloaded),
		});
		return;
	}
	state.lookups.next_generation = state
		.lookups
		.next_generation
		.checked_add(1)
		.expect("lookup generation exhausted");

	let generation = state.lookups.next_generation;

	let deadline_boot_ms = now_boot_ms.saturating_add(state.config.lease.io_timeout_ms);

	state.lookups.pending.insert(
		worker.clone(),
		PendingLookup {
			generation,
			deadline_boot_ms,
			waiters: vec![request_id],
			invalidated: false,
		},
	);

	let first_role = last_seen_role(state, &worker);
	out.execute(Effect::LookupEndpointRequest {
		worker,
		generation,
		first_role,
	});
}

/// The role `worker` was last seen in: listed by discovery, cached, or hidden after a
/// failure. Unknown workers are assumed stateless, the high-cardinality group that point
/// reads exist for.
fn last_seen_role(state: &State, worker: &WorkerId) -> crate::worker::WorkerRole {
	use crate::worker::WorkerRole;

	let listed = state
		.discovery
		.entries
		.iter()
		.flatten()
		.any(|entry| entry.worker_id == *worker && entry.role == WorkerRole::Storage);
	let cached = state
		.lookups
		.entries
		.get(worker)
		.and_then(|entry| entry.worker.as_ref())
		.or_else(|| state.lookups.invalidated.get(worker))
		.map(|known| known.role);
	if listed {
		WorkerRole::Storage
	} else {
		cached.unwrap_or(WorkerRole::Stateless)
	}
}

pub(super) fn handle_lookup_endpoint_response(
	state: &mut State,
	worker: WorkerId,
	generation: u64,
	result: EndpointResult,
	heartbeat_modified_unix_ms: Option<i64>,
	now_boot_ms: u64,
	out: &mut Effects,
) {
	let Some(pending) = state.lookups.pending.get(&worker) else {
		return;
	};

	if pending.generation != generation {
		return;
	}

	let pending = state.lookups.pending.remove(&worker).unwrap();

	let result = if pending.invalidated {
		Err(RequestError::Invalidated)
	} else {
		result
	};

	// A point read found a live storage worker that discovery does not list, so the
	// listing is stale; refresh it early instead of waiting for the interval.
	if let Ok(Some(found)) = &result
		&& found.role == crate::worker::WorkerRole::Storage
		&& !state
			.storage_entries(now_boot_ms)
			.any(|entry| entry.worker_id == found.id)
	{
		super::lease::hint_refresh(state, "hint", now_boot_ms, out);
	}

	if let Ok(value) = &result {
		// A post-invalidation point read can revalidate the exact discovery route.
		// Keep a different stale incarnation/address suppressed until discovery changes.
		// An invalidation during this read was already rejected above.
		if let Some(confirmed) = value
			&& state
				.lookups
				.invalidated
				.get(&worker)
				.is_some_and(|bad| same_endpoint(bad, confirmed))
		{
			state.lookups.invalidated.remove(&worker);
		}
		remove_expired_entries(state, now_boot_ms);

		if state.lookups.entries.len() >= state.config.endpoint_capacity
			&& !state.lookups.entries.contains_key(&worker)
		{
			// Evict the soonest-expiring hint. Never bypass the bounded read path at capacity.
			let soonest = state
				.lookups
				.entry_expiry
				.iter()
				.next()
				.map(|(_, id)| id.clone());
			if let Some(id) = soonest {
				remove_entry(state, &id);
			}
		}
		// A found endpoint stays a hint until its heartbeat's lease would end; a miss, or a
		// result read before store time is known, keeps the short lookup TTL.
		let store_now = state
			.discovery
			.store_clock
			.as_ref()
			.map(|clock| clock.lower_bound(now_boot_ms));
		let ttl_ms = match (value, heartbeat_modified_unix_ms, store_now) {
			(Some(_), Some(modified), Some(store_now)) => {
				state
					.config
					.lease
					.hint_ttl_ms(modified, store_now, state.config.lookup_ttl_ms)
			}
			_ => state.config.lookup_ttl_ms,
		};
		let expires_at_boot_ms = now_boot_ms.saturating_add(ttl_ms);
		state
			.lookups
			.entry_expiry
			.schedule(worker.clone(), expires_at_boot_ms);
		state.lookups.entries.insert(
			worker,
			EndpointEntry {
				worker: value.clone(),
				expires_at_boot_ms,
				confirmed_at_boot_ms: now_boot_ms,
			},
		);
	}

	for request_id in pending.waiters {
		out.execute(Effect::ResolveEndpointResponse {
			request_id,
			result: result.clone(),
		});
	}
}

/// Hides a failed endpoint and voids an in-flight read only when the failure is new evidence.
///
/// A read that is in flight while this exact endpoint is already invalidated was started to
/// recheck that belief, so repeated failures from requests sent before the first invalidation
/// must not void the read that could revalidate or replace it.
pub(super) fn handle_invalidate_endpoint_request(
	state: &mut State,
	request_id: u64,
	worker: Worker,
	now_boot_ms: u64,
	out: &mut Effects,
) {
	let already_invalidated = state
		.lookups
		.invalidated
		.get(&worker.id)
		.is_some_and(|bad| same_endpoint(bad, &worker));

	// Forget invalidations whose discovery route has since changed.
	let changed_routes: Vec<WorkerId> = state
		.lookups
		.invalidated
		.iter()
		.filter(|(id, bad)| !listed_endpoint_matches(state, id, bad, now_boot_ms))
		.map(|(id, _)| id.clone())
		.collect();
	let hide_listed_route =
		!already_invalidated && listed_endpoint_matches(state, &worker.id, &worker, now_boot_ms);

	for id in changed_routes {
		state.lookups.invalidated.remove(&id);
	}
	if hide_listed_route {
		state
			.lookups
			.invalidated
			.insert(worker.id.clone(), worker.clone());
	}

	if state
		.lookups
		.entries
		.get(&worker.id)
		.is_some_and(|e| e.worker.as_ref().is_none_or(|w| same_endpoint(&worker, w)))
	{
		remove_entry(state, &worker.id);
	}

	if !already_invalidated && let Some(pending) = state.lookups.pending.get_mut(&worker.id) {
		pending.invalidated = true;
	}

	out.execute(Effect::InvalidateEndpointResponse {
		request_id,
		result: Ok(()),
	});
}

/// Drops a stateless worker's cached endpoint after an opening timeout only when its last
/// point-read confirmation is old. Under overload a timeout says nothing about the route, so
/// a recently confirmed one is kept and the caller's next request reuses it.
pub(super) fn handle_endpoint_timed_out(
	state: &mut State,
	worker: Worker,
	now_boot_ms: u64,
	out: &mut Effects,
) {
	let lookup_running = state.lookups.pending.contains_key(&worker.id);
	let stale = state.lookups.entries.get(&worker.id).is_some_and(|entry| {
		entry
			.worker
			.as_ref()
			.is_some_and(|cached| same_endpoint(&worker, cached))
			&& now_boot_ms.saturating_sub(entry.confirmed_at_boot_ms)
				>= state.config.endpoint_timeout_invalidate_after_ms
	});
	let result = if stale && !lookup_running {
		remove_entry(state, &worker.id);
		EndpointTimeoutResult::Invalidated
	} else {
		EndpointTimeoutResult::Ignored
	};
	out.execute(Effect::RecordEndpointTimeout { result });
}

/// Whether current, unexpired storage discovery lists exactly this endpoint for `id`.
fn listed_endpoint_matches(
	state: &State,
	id: &WorkerId,
	endpoint: &Worker,
	now_boot_ms: u64,
) -> bool {
	state
		.storage_entries(now_boot_ms)
		.find(|entry| entry.worker_id == *id)
		.is_some_and(|entry| same_endpoint(endpoint, &entry.worker()))
}

pub(super) fn finish_event(state: &State, out: &mut Effects) {
	out.execute(Effect::RecordLookupUsage {
		entries: state.lookups.entries.len(),
		reads: state.lookups.pending.len() + state.lookups.heartbeats.len(),
	});
	schedule_lookup_expiry(state, out);
}

fn schedule_lookup_expiry(state: &State, out: &mut Effects) {
	let next = state
		.lookups
		.pending
		.values()
		.map(|p| p.deadline_boot_ms)
		.chain(
			state
				.lookups
				.heartbeats
				.values()
				.map(|p| p.deadline_boot_ms),
		)
		.chain(state.lookups.entry_expiry.next_at())
		.min();

	if let Some(at) = next {
		out.arm_timer_at(Timer::LookupExpiry, at);
	} else {
		out.cancel_timer(Timer::LookupExpiry);
	}
}

/// One shared alarm selects expired lookup attempts; responses only check correlation.
/// Keep these deadlines to select the due subset and arm the next earliest alarm.
pub(super) fn expire(state: &mut State, now_boot_ms: u64, out: &mut Effects) {
	let expired: Vec<_> = state
		.lookups
		.heartbeats
		.iter()
		.filter(|(_, p)| now_boot_ms >= p.deadline_boot_ms)
		.map(|(id, p)| (id.clone(), p.generation))
		.collect();
	for (worker, generation) in expired {
		handle_read_storage_heartbeat_response(
			state,
			worker,
			generation,
			crate::heartbeat::Read::Unknown,
			now_boot_ms,
			out,
		);
	}

	remove_expired_entries(state, now_boot_ms);

	let expired: Vec<_> = state
		.lookups
		.pending
		.iter()
		.filter(|(_, p)| now_boot_ms >= p.deadline_boot_ms)
		.map(|(id, p)| (id.clone(), p.generation))
		.collect();

	for (id, generation) in expired {
		handle_lookup_endpoint_response(
			state,
			id,
			generation,
			Err(RequestError::Unavailable),
			None,
			now_boot_ms,
			out,
		);
	}
}

/// Removes one cached entry and its expiry together, so the expiry queue stays in step.
fn remove_entry(state: &mut State, worker: &WorkerId) {
	if state.lookups.entry_expiry.cancel(worker).is_some() {
		state.lookups.entries.remove(worker);
	}
}

/// Removes the cached entries whose expiry is due. Only due entries are visited.
fn remove_expired_entries(state: &mut State, now_boot_ms: u64) {
	let due = state.lookups.entry_expiry.pop_due(now_boot_ms, usize::MAX);
	crate::metrics::LOOKUP_EXPIRY_VISITS.inc_by(due.len() as u64);
	for (_, worker) in due {
		state.lookups.entries.remove(&worker);
	}
}

/// Every cached entry has exactly one queued expiry at its `expires_at_boot_ms`.
pub(super) fn validate(state: &State) -> anyhow::Result<()> {
	anyhow::ensure!(
		state.lookups.entry_expiry.len() == state.lookups.entries.len(),
		"endpoint expiry queue does not match the cache"
	);
	for (worker, entry) in state.lookups.entries.iter() {
		anyhow::ensure!(
			state.lookups.entry_expiry.scheduled_at(worker) == Some(entry.expires_at_boot_ms),
			"endpoint expiry queue does not match the cache"
		);
	}
	Ok(())
}

pub(super) fn stop(state: &mut State, out: &mut Effects) {
	for pending in std::mem::take(&mut state.lookups.heartbeats).into_values() {
		for request_id in pending.waiters.into_iter().chain(pending.queued) {
			reject_inspection(request_id, out);
		}
	}

	for pending in std::mem::take(&mut state.lookups.pending).into_values() {
		for request_id in pending.waiters {
			reject_resolution(request_id, out);
		}
	}
	state.lookups.entries.clear();
	state.lookups.entry_expiry = Default::default();
	state.lookups.invalidated.clear();

	out.execute(Effect::CancelLookups);
}

// Fresh owner evidence bypasses endpoint hints, but shares the same read and waiter caps.
pub(super) fn handle_inspect_heartbeat_request(
	state: &mut State,
	request_id: u64,
	worker: WorkerId,
	now_boot_ms: u64,
	out: &mut Effects,
) {
	if let Some(pending) = state.lookups.heartbeats.get_mut(&worker) {
		// A read already in flight may predate the caller's owner-record read.
		// Queue these callers for one following read; only that new read is fresh
		// enough to prove that their recorded owner's process has changed.
		if pending.waiters.len() + pending.queued.len() < 128 {
			pending.queued.push(request_id);
			return;
		}
	} else if state.lookups.pending.len() + state.lookups.heartbeats.len()
		< state.config.lookup_concurrency
	{
		state.lookups.next_generation = state
			.lookups
			.next_generation
			.checked_add(1)
			.expect("lookup generation exhausted");
		let generation = state.lookups.next_generation;
		state.lookups.heartbeats.insert(
			worker.clone(),
			PendingInspection {
				generation,
				deadline_boot_ms: now_boot_ms.saturating_add(state.config.lease.io_timeout_ms),
				waiters: vec![request_id],
				queued: Vec::new(),
			},
		);
		out.execute(Effect::ReadStorageHeartbeatRequest { worker, generation });
		return;
	}

	out.execute(Effect::RecordLookupRejected);
	out.execute(Effect::InspectHeartbeatResponse {
		request_id,
		result: crate::heartbeat::Read::Unknown,
	});
}

pub(super) fn handle_read_storage_heartbeat_response(
	state: &mut State,
	worker: WorkerId,
	generation: u64,
	result: crate::heartbeat::Read,
	now_boot_ms: u64,
	out: &mut Effects,
) {
	if !state
		.lookups
		.heartbeats
		.get(&worker)
		.is_some_and(|p| p.generation == generation)
	{
		return;
	}
	let pending = state.lookups.heartbeats.remove(&worker).unwrap();
	for request_id in pending.waiters {
		out.execute(Effect::InspectHeartbeatResponse {
			request_id,
			result: result.clone(),
		});
	}
	if !pending.queued.is_empty() {
		state.lookups.next_generation = state
			.lookups
			.next_generation
			.checked_add(1)
			.expect("lookup generation exhausted");
		let generation = state.lookups.next_generation;
		state.lookups.heartbeats.insert(
			worker.clone(),
			PendingInspection {
				generation,
				deadline_boot_ms: now_boot_ms.saturating_add(state.config.lease.io_timeout_ms),
				waiters: pending.queued,
				queued: Vec::new(),
			},
		);
		out.execute(Effect::ReadStorageHeartbeatRequest { worker, generation });
	}
}

/// Reject new requests with the same outcomes used when draining queued waiters.
pub(super) fn reject(event: super::Event, out: &mut Effects) {
	use super::Event;
	match event {
		Event::InspectHeartbeatRequest { request_id, .. } => reject_inspection(request_id, out),
		Event::ResolveEndpointRequest { request_id, .. } => reject_resolution(request_id, out),
		Event::InvalidateEndpointRequest { request_id, .. } => {
			out.execute(Effect::InvalidateEndpointResponse {
				request_id,
				result: Err(RequestError::Stopped),
			})
		}
		Event::EndpointTimedOut { .. } => out.execute(Effect::RecordEndpointTimeout {
			result: EndpointTimeoutResult::Ignored,
		}),
		// A stopped topology watches nothing; the handles of a stopped worker never resolve.
		Event::WatchRequest { .. }
		| Event::UnwatchRequest { .. }
		| Event::ReadWatchedHeartbeatResponse { .. } => {}
		Event::ReadLocalHeartbeatResponse { .. }
		| Event::WriteLocalHeartbeatResponse { .. }
		| Event::ListStorageWorkersResponse { .. }
		| Event::ListStorageWorkersPartialResponse { .. }
		| Event::LookupEndpointResponse { .. }
		| Event::ReadStorageHeartbeatResponse { .. }
		| Event::StopRequest
		| Event::Fence(_)
		| Event::BeginDrain
		| Event::Refresh => {}
	}
}

fn reject_inspection(request_id: u64, out: &mut Effects) {
	out.execute(Effect::InspectHeartbeatResponse {
		request_id,
		result: crate::heartbeat::Read::Unknown,
	});
}

fn reject_resolution(request_id: u64, out: &mut Effects) {
	out.execute(Effect::ResolveEndpointResponse {
		request_id,
		result: Err(RequestError::Stopped),
	});
}
