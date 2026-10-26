//! Singleton-owned key lifecycle. All retries, readback and deadlines live in this pure core.
//! ETag CAS fences every mutation. A successor tenure adopts the ring before rotating it,
//! so an already-sent old PUT can win only before adoption, and cannot overwrite adoption.
pub mod effect;
pub mod event;
pub mod executor;
pub mod snapshot;
pub mod state;
pub mod types;

use anyhow::{Context, Result, ensure};
use effect::Effect;
use event::Event;
use rivet_env::Clocks;
use rivet_object_store_util::conditional_write as cw;
use rivet_state_machine::Entropy;
use rivet_state_machine_ephemeral::{
	EphemeralStateMachine, EphemeralStateMachineEvent, RuntimeEffect, RuntimeEvent,
};
use rivet_state_machine_util::{IoCall, IoTimer};
use state::{Phase, State};
use types::*;

pub struct KeyRingStateMachine;
pub type Effects = rivet_state_machine_ephemeral::Effects<Effect, Timer>;
const IO_TIMEOUT_MS: u64 = 5_000;
const RETRY_MS: u64 = 1_000;

impl EphemeralStateMachine for KeyRingStateMachine {
	type Slot = Slot;
	type State = State;
	type Event = Event;
	type Effect = Effect;
	type Snapshot = snapshot::Snapshot;
	type Timer = Timer;
	const DEBUG_NAME: &'static str = "jwt_key_ring";
	fn timer_name(timer: &Timer) -> &'static str {
		match timer {
			Timer::Io(_) => "io",
			Timer::Publication(_) => "publication",
			Timer::Wake => "wake",
		}
	}
	fn start(state: &mut State, _: Clocks, _: Entropy, out: &mut Effects) {
		read(state, out);
	}
	fn on_event(state: &mut State, event: EphemeralStateMachineEvent<Self>, out: &mut Effects) {
		if matches!(state.phase, Phase::Stopped) {
			return;
		}
		if let Err(error) = transition(state, event.body, event.now, out) {
			// Invalid data never produces a replacement. Retry by reading the authoritative ring.
			finish_emergency(
				state,
				EmergencyRotateResponse::Rejected(Rejected::Unavailable),
				out,
			);
			cancel(state, out);
			state.diagnostic = Some(error.to_string());
			state.phase = Phase::Backoff;
			out.arm_timer_after(Timer::Wake, RETRY_MS);
		}
	}
	fn snapshot(state: &State) -> snapshot::Snapshot {
		state.into()
	}
	fn debug_validate(state: &State) -> Result<()> {
		state.settings.policy.validate()
	}
}

fn transition(
	state: &mut State,
	input: RuntimeEvent<Event, Timer>,
	now: Clocks,
	out: &mut Effects,
) -> Result<()> {
	match input {
		RuntimeEvent::Input(Event::EmergencyRotateRequest { call, request }) => {
			admit_emergency(state, call, request, out)?
		}
		RuntimeEvent::Input(Event::StopRequest) => {
			finish_emergency(
				state,
				EmergencyRotateResponse::Rejected(Rejected::NotCoordinator),
				out,
			);
			cancel(state, out);
			if let Phase::Publishing(progress) = &mut state.phase {
				let mut child = rivet_state_machine::Effects::new(now);
				progress.suspend(&mut child);
				publication_effects(child, out);
			}
			out.cancel_timer(Timer::Wake);
			state.phase = Phase::Stopped;
		}
		RuntimeEvent::Timer(Timer::Wake) => match &state.phase {
			Phase::Backoff => read(state, out),
			Phase::Waiting {
				stored,
				wake_unix_ms,
			} => {
				if now.unix_ms >= *wake_unix_ms {
					maintain(state, stored.clone(), now, out)?;
				} else {
					out.arm_timer_after(Timer::Wake, (*wake_unix_ms - now.unix_ms) as u64);
				}
			}
			Phase::Reading(_)
			| Phase::Generating { .. }
			| Phase::Publishing(_)
			| Phase::Stopped => {}
		},
		RuntimeEvent::Timer(Timer::Io(IoTimer::Io(op))) => {
			if matches!(&state.phase, Phase::Reading(call) | Phase::Generating { call, .. } if call.op == op)
			{
				finish_emergency(
					state,
					EmergencyRotateResponse::Rejected(Rejected::Unavailable),
					out,
				);
				cancel(state, out);
				state.phase = Phase::Backoff;
				state.diagnostic = Some("JWT store or key provider timed out".into());
				out.arm_timer_after(Timer::Wake, RETRY_MS);
			}
		}
		RuntimeEvent::Timer(Timer::Publication(timer)) => {
			publication(state, RuntimeEvent::Timer(timer), now, out)?
		}
		RuntimeEvent::Input(Event::ReadRingResponse { op, result }) => {
			if let Phase::Publishing(progress) = &state.phase {
				let request = progress.request();
				let readback = match &result {
					ReadResult::Found(found) if found.bytes == request.bytes => {
						cw::Readback::Confirmed(result)
					}
					ReadResult::Found(found)
						if Some(&found.etag) == request.expected_etag.as_ref() =>
					{
						cw::Readback::Unchanged
					}
					ReadResult::Missing if request.expected_etag.is_none() => {
						cw::Readback::Unchanged
					}
					ReadResult::Unknown(_) | ReadResult::Invalid(_) => cw::Readback::Unknown,
					ReadResult::Found(_) | ReadResult::Missing => cw::Readback::Reconcile(result),
				};
				publication(
					state,
					RuntimeEvent::Input(cw::Event::ReadResponse {
						op,
						result: readback,
					}),
					now,
					out,
				)?;
			} else if matches!(&state.phase, Phase::Reading(call) if call.op == op) {
				cancel(state, out);
				match result {
					ReadResult::Found(stored) => adopt(state, stored, now, out)?,
					ReadResult::Missing => {
						ensure!(
							state.generation_floor == 0,
							"JWT ring disappeared after it was observed; refusing to recreate it"
						);
						generate(state, None, out);
					}
					ReadResult::Unknown(error) | ReadResult::Invalid(error) => {
						anyhow::bail!("{error}")
					}
				}
			}
		}
		RuntimeEvent::Input(Event::GenerateKeyResponse { op, result }) => {
			if let Phase::Generating { call, base } = &state.phase {
				if call.op != op {
					return Ok(());
				}
				let base = base.clone();
				cancel(state, out);
				let material = result.map_err(anyhow::Error::msg)?;
				let signing_key = crate::SigningKey::from_seed(
					crate::KeyId::from_bytes(material.kid),
					material.seed,
				);
				let candidate = crate::SigningKeyRecord {
					public: crate::PublicKeyRecord {
						kid: signing_key.kid(),
						algorithm: crate::Algorithm::Ed25519,
						public_key: signing_key.public_key(),
						created_ts: now.unix_ms,
					},
					signing_key,
				};
				let ring = match &base {
					None => {
						crate::bootstrap(
							state.settings.issuer.clone(),
							1,
							state.settings.audience.clone(),
							state.settings.datacenter_id,
							state.settings.epoch,
							&candidate,
							now.unix_ms,
							state.settings.policy,
						)?
						.successor
					}
					Some(stored) if state.emergency.is_some() => {
						let request = &state.emergency.as_ref().expect("matched emergency").request;
						let selected = request
							.revoke_kids
							.iter()
							.copied()
							.map(crate::KeyId::from_bytes)
							.collect::<Vec<_>>();
						crate::emergency_rotate(
							&crate::decode_signing_key_ring(&stored.bytes)?,
							&candidate,
							request.request_id,
							request.expected_ring_version,
							&selected,
							now.unix_ms,
							state.settings.policy,
						)?
						.successor
					}
					Some(stored) => {
						crate::stage_pending(
							&crate::decode_signing_key_ring(&stored.bytes)?,
							&candidate,
							now.unix_ms,
							state.settings.policy,
						)?
						.successor
					}
				};
				publish(
					state,
					Mutation {
						bytes: crate::encode_signing_key_ring(&ring)?,
						expected_etag: base.map(|v| v.etag),
					},
					now,
					out,
				);
			}
		}
		RuntimeEvent::Input(Event::WriteRingResponse { op, result }) => publication(
			state,
			RuntimeEvent::Input(cw::Event::WriteResponse { op, result }),
			now,
			out,
		)?,
	}
	Ok(())
}

fn read(state: &mut State, out: &mut Effects) {
	out.cancel_timer(Timer::Wake);
	let call = IoCall::issue(&mut state.sequence, Slot, IO_TIMEOUT_MS, Timer::Io, out);
	state.phase = Phase::Reading(call);
	out.execute(Effect::ReadRingRequest { op: call.op });
}
fn cancel(state: &State, out: &mut Effects) {
	if let Some(call) = state.pending() {
		call.accept(call.op, Timer::Io, out);
		out.execute(Effect::CancelOperation { op: call.op });
	}
}
fn generate(state: &mut State, base: Option<StoredRing>, out: &mut Effects) {
	let call = IoCall::issue(&mut state.sequence, Slot, IO_TIMEOUT_MS, Timer::Io, out);
	state.phase = Phase::Generating { call, base };
	out.execute(Effect::GenerateKeyRequest { op: call.op });
}
/// Adopt the observed epoch before maintenance. Issuer changes require a separate cluster.
fn adopt(state: &mut State, stored: StoredRing, now: Clocks, out: &mut Effects) -> Result<()> {
	let mut ring = crate::decode_signing_key_ring(&stored.bytes)?;
	ensure!(
		ring.generation >= state.generation_floor,
		"JWT ring generation regressed"
	);
	state.generation_floor = ring.generation;
	ensure!(
		ring.issuer_state.active.issuer == state.settings.issuer
			&& ring.audience == state.settings.audience,
		"JWT issuer/audience differ from the cluster ring"
	);
	if ring.leader_epoch > state.settings.epoch {
		state.phase = Phase::Stopped;
	} else if ring.leader_epoch < state.settings.epoch {
		let previous = ring.clone();
		ring.leader_epoch = state.settings.epoch;
		ring.leader_datacenter_id = state.settings.datacenter_id;
		ring.generation = ring
			.generation
			.checked_add(1)
			.context("JWT ring generation overflow")?;
		previous.validate_successor(&ring)?;
		publish(
			state,
			Mutation {
				bytes: crate::encode_signing_key_ring(&ring)?,
				expected_etag: Some(stored.etag),
			},
			now,
			out,
		);
	} else {
		maintain(state, stored, now, out)?;
	}
	Ok(())
}
/// Apply exactly one due lifecycle step, or arm its known deadline without polling.
fn maintain(state: &mut State, stored: StoredRing, now: Clocks, out: &mut Effects) -> Result<()> {
	let ring = crate::decode_signing_key_ring(&stored.bytes)?;
	ensure!(
		ring.generation >= state.generation_floor,
		"JWT ring generation regressed"
	);
	state.generation_floor = ring.generation;
	if let Some(pruned) = crate::prune_retiring(&ring, now.unix_ms)? {
		publish(
			state,
			Mutation {
				bytes: crate::encode_signing_key_ring(&pruned.successor)?,
				expected_etag: Some(stored.etag),
			},
			now,
			out,
		);
	} else if ring
		.pending
		.as_ref()
		.is_some_and(|key| now.unix_ms >= key.activate_after_ts)
	{
		let successor =
			crate::activate_pending(&ring, now.unix_ms, state.settings.policy)?.successor;
		publish(
			state,
			Mutation {
				bytes: crate::encode_signing_key_ring(&successor)?,
				expected_etag: Some(stored.etag),
			},
			now,
			out,
		);
	} else {
		let wake_unix_ms = crate::next_wake_ts(&ring, state.settings.policy)?;
		if now.unix_ms >= wake_unix_ms {
			generate(state, Some(stored), out);
		} else {
			state.phase = Phase::Waiting {
				stored,
				wake_unix_ms,
			};
			state.diagnostic = None;
			out.arm_timer_after(Timer::Wake, wake_unix_ms.saturating_sub(now.unix_ms) as u64);
		}
	}
	Ok(())
}
fn publish(state: &mut State, request: Mutation, now: Clocks, out: &mut Effects) {
	let mut child = rivet_state_machine::Effects::new(now);
	let progress = cw::ConditionalWrite::start(
		request,
		cw::Config {
			io_timeout_ms: IO_TIMEOUT_MS,
			retry_delay_ms: RETRY_MS,
		},
		now.boot_ms,
		state.sequence.next(Slot),
		&mut child,
	);
	state.phase = Phase::Publishing(progress);
	publication_effects(child, out);
}
fn publication(
	state: &mut State,
	input: RuntimeEvent<cw::Event<OpId, ReadResult>, cw::Timer<OpId>>,
	now: Clocks,
	out: &mut Effects,
) -> Result<()> {
	let Phase::Publishing(progress) = &mut state.phase else {
		return Ok(());
	};
	let mut child = rivet_state_machine::Effects::new(now);
	let update = progress.on_event(now.boot_ms, input, || state.sequence.next(Slot), &mut child);
	publication_effects(child, out);
	match update {
		Some(cw::Outcome::Confirmed(cw::Confirmation::Write { etag })) => {
			let stored = StoredRing {
				bytes: progress.request().bytes.clone(),
				etag,
			};
			confirm_emergency(state, &stored, out)?;
			maintain(state, stored, now, out)?;
		}
		Some(cw::Outcome::Confirmed(cw::Confirmation::Read(ReadResult::Found(stored)))) => {
			confirm_emergency(state, &stored, out)?;
			maintain(state, stored, now, out)?
		}
		Some(cw::Outcome::Reconcile(ReadResult::Found(stored))) => {
			confirm_emergency(state, &stored, out)?;
			adopt(state, stored, now, out)?;
		}
		Some(cw::Outcome::Reconcile(ReadResult::Missing)) => {
			anyhow::bail!("JWT ring was deleted during publication; refusing to recreate it")
		}
		Some(cw::Outcome::Confirmed(cw::Confirmation::Read(
			ReadResult::Missing | ReadResult::Unknown(_) | ReadResult::Invalid(_),
		)))
		| Some(cw::Outcome::Reconcile(ReadResult::Unknown(_) | ReadResult::Invalid(_))) => {
			unreachable!("readback classification requires complete matching ring bytes")
		}
		None => {}
	}
	Ok(())
}
fn publication_effects(
	child: rivet_state_machine_ephemeral::Effects<cw::Effect<Mutation, OpId>, cw::Timer<OpId>>,
	out: &mut Effects,
) {
	for effect in child.as_slice().iter().cloned() {
		match effect {
			RuntimeEffect::Execute(cw::Effect::ReadRequest { op }) => {
				out.execute(Effect::ReadRingRequest { op })
			}
			RuntimeEffect::Execute(cw::Effect::WriteRequest { op, request }) => {
				out.execute(Effect::WriteRingRequest { op, request })
			}
			RuntimeEffect::ArmTimer { timer, at_boot_ms } => {
				out.arm_timer_at(Timer::Publication(timer), at_boot_ms)
			}
			RuntimeEffect::CancelTimer(timer) => {
				if let cw::Timer::Io(IoTimer::Io(op)) = timer {
					out.execute(Effect::CancelOperation { op });
				}
				out.cancel_timer(Timer::Publication(timer));
			}
			RuntimeEffect::Exit => out.exit(),
		}
	}
}

/// Reject rather than queue across unrelated normal maintenance. Operators can safely repeat
/// the same logical request after Busy; an admitted command retains its exact CAS mutation.
fn admit_emergency(
	state: &mut State,
	call: u64,
	request: EmergencyRotateRequest,
	out: &mut Effects,
) -> Result<()> {
	let reject = |reason, out: &mut Effects| {
		out.execute(Effect::EmergencyRotateResponse {
			call,
			response: EmergencyRotateResponse::Rejected(reason),
		})
	};
	if request.expected_coordinator_epoch != state.settings.epoch {
		reject(Rejected::NotCoordinator, out);
		return Ok(());
	}
	let mut selected = request.revoke_kids.clone();
	selected.sort_unstable();
	if selected.len() > crate::ABSOLUTE_KEY_LIMIT
		|| selected.windows(2).any(|pair| pair[0] == pair[1])
	{
		reject(Rejected::InvalidRequest, out);
		return Ok(());
	}
	if let Some(pending) = &state.emergency {
		let mut previous = pending.request.revoke_kids.clone();
		previous.sort_unstable();
		let reused = pending.request.request_id == request.request_id
			&& (pending.request.expected_ring_version != request.expected_ring_version
				|| previous != selected);
		reject(
			if reused {
				Rejected::RequestIdReused
			} else {
				Rejected::Busy
			},
			out,
		);
		return Ok(());
	}
	let Phase::Waiting { stored, .. } = &state.phase else {
		reject(Rejected::Busy, out);
		return Ok(());
	};
	let stored = stored.clone();
	let ring = crate::decode_signing_key_ring(&stored.bytes)?;
	if let Some(response) = receipt_response(&ring, &request) {
		out.execute(Effect::EmergencyRotateResponse { call, response });
		return Ok(());
	}
	if ring.generation != request.expected_ring_version {
		reject(Rejected::VersionChanged, out);
		return Ok(());
	}
	let selected = selected
		.into_iter()
		.map(crate::KeyId::from_bytes)
		.collect::<Vec<_>>();
	if crate::validate_emergency_request(&ring, request.expected_ring_version, &selected).is_err() {
		reject(Rejected::InvalidRequest, out);
		return Ok(());
	}
	state.emergency = Some(EmergencyCall { call, request });
	out.cancel_timer(Timer::Wake);
	generate(state, Some(stored), out);
	Ok(())
}

/// Only the latest emergency receipt is retained. Once superseded, expected ring version
/// prevents an old request from executing again, and the caller receives VersionChanged.
fn receipt_response(
	ring: &crate::SigningKeyRing,
	request: &EmergencyRotateRequest,
) -> Option<EmergencyRotateResponse> {
	let receipt = ring
		.last_emergency_receipt
		.as_ref()
		.filter(|receipt| receipt.request_id == request.request_id)?;
	let mut selected = request.revoke_kids.clone();
	selected.sort_unstable();
	let original = receipt
		.requested_revoked_kids
		.iter()
		.map(|kid| *kid.as_bytes())
		.collect::<Vec<_>>();
	if receipt.expected_generation != request.expected_ring_version || original != selected {
		return Some(EmergencyRotateResponse::Rejected(Rejected::RequestIdReused));
	}
	Some(EmergencyRotateResponse::Committed(Committed {
		ring_version: receipt.committed_generation,
		revoked_kids: receipt
			.revoked_kids
			.iter()
			.map(|kid| *kid.as_bytes())
			.collect(),
	}))
}

fn finish_emergency(state: &mut State, response: EmergencyRotateResponse, out: &mut Effects) {
	if let Some(pending) = state.emergency.take() {
		out.execute(Effect::EmergencyRotateResponse {
			call: pending.call,
			response,
		});
	}
}

/// A write confirmation or a durable readback, never an unchanged base, authorizes success.
fn confirm_emergency(state: &mut State, stored: &StoredRing, out: &mut Effects) -> Result<()> {
	let Some(pending) = &state.emergency else {
		return Ok(());
	};
	let ring = crate::decode_signing_key_ring(&stored.bytes)?;
	let response = if ring.leader_epoch > state.settings.epoch {
		EmergencyRotateResponse::Rejected(Rejected::NotCoordinator)
	} else {
		receipt_response(&ring, &pending.request)
			.unwrap_or(EmergencyRotateResponse::Rejected(Rejected::VersionChanged))
	};
	finish_emergency(state, response, out);
	Ok(())
}
