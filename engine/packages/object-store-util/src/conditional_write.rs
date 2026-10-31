//! Resolves one exact conditional mutation without owning membership or serving authority.
//!
//! The caller supplies a prepared request containing its original key, value, precondition,
//! mutation identity and any historical lease timestamps. This helper never edits that request.
//! Readback classification belongs to the adapter/domain: an unchanged base is unresolved,
//! not a failed PUT, and a superseding version does not prove the PUT never committed.
//!
//! Embed `ConditionalWrite` in the parent's exclusive I/O state. Map its runtime effects and
//! timers into the parent's existing event loop; allocate operation IDs from the parent so
//! replacing this helper cannot reuse an old token. Do not spawn an independent runner.
//! The runtime must deliver due I/O alarms before responses. Dispatch still requires a fresh
//! authority check; cancellation or suspension cannot recall a remotely outstanding write.

use rivet_state_machine_ephemeral::{Effects, RuntimeEvent};
use rivet_state_machine_util::{IoCall, IoTimer};
use serde::{Deserialize, Serialize};

/// Borrowed projection of a parent's serial control lane, not another operation registry.
/// Map the parent's activity once to share pending-call and publication-target read routing.
pub enum ControlIo<'a, W, O> {
	/// No executor call is pending, including retry waits and suspended activities.
	Idle,

	/// An ordinary read of the same target that publication readback inspects.
	Reading(&'a IoCall<O>),

	/// Another executor call, such as a heartbeat read, preparation, or entropy sampling.
	Other(&'a IoCall<O>),

	/// The embedded conditional write owns its pending call and readback phase.
	Publishing(&'a ConditionalWrite<W, O>),
}

impl<'a, W: Clone, O: Copy + Eq> ControlIo<'a, W, O> {
	/// Pending executor token and dispatch deadline borrowed from their sole owner.
	pub fn pending(&self) -> Option<&'a IoCall<O>> {
		match self {
			Self::Idle => None,
			Self::Reading(call) | Self::Other(call) => Some(call),
			Self::Publishing(progress) => progress.pending(),
		}
	}

	/// Whether a response reading the publication target can be routed to this activity.
	pub fn reading(&self) -> bool {
		match self {
			Self::Reading(_) => true,
			Self::Publishing(progress) => progress.reading(),
			Self::Idle | Self::Other(_) => false,
		}
	}
}

/// Caller-selected bounds; the helper does not sample randomness or choose ownership policy.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Config {
	/// Bounds each read or write exchange through the parent's runtime alarms.
	pub io_timeout_ms: u64,
	/// Positive retry interval, optionally sampled by the parent before starting.
	pub retry_delay_ms: u64,
}

/// Alarm identities are operation-scoped so retired helpers cannot wake newer work.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Timer<O> {
	/// Timeout of the exact outstanding read or write.
	Io(IoTimer<O>),
	/// Backoff following this read operation.
	Retry(O),
}

/// Conditional PUT response. A conflict also requires readback after transport retries.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WriteResult {
	/// The store confirmed this PUT; the caller must still validate its authority/time origin.
	Confirmed { etag: String },
	/// Includes timeout, conditional conflict and errors with uncertain dispatch/outcome.
	Unknown,
}

/// Domain classification of a GET; only positive evidence may complete the mutation.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Readback<R> {
	/// The observed value identifies the intended mutation.
	Confirmed(R),
	/// The original condition still holds. The outstanding PUT might arrive later.
	Unchanged,
	/// Failed/unreadable GET; neither absence nor permission to rebase.
	Unknown,
	/// Return changed/missing evidence to the parent with the original request retained.
	Reconcile(R),
}

/// Executor responses enter through the parent, which supplies readback classification.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Event<O, R> {
	/// Paired with `Effect::WriteRequest`; adapters may request readback even after success.
	WriteResponse { op: O, result: WriteResult },
	/// Paired with `Effect::ReadRequest`; stale or wrong-kind replies are ignored.
	ReadResponse { op: O, result: Readback<R> },
}

/// I/O mapped into the parent's executor protocol; no network client is owned here.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Effect<W, O> {
	/// Every retry carries the same prepared request and conditional precondition.
	WriteRequest { op: O, request: W },
	/// Read the target of the retained request, returning `Event::ReadResponse`.
	ReadRequest { op: O },
}

/// Evidence of publication; neither form grants serving permission by itself.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Confirmation<R> {
	/// Confirmed response to the exact PUT.
	Write { etag: String },
	/// Domain-validated GET identifies the intended mutation.
	Read(R),
}

/// Completion returned once; the request remains available for domain reconciliation.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Outcome<R> {
	/// Parent must check original lease deadlines and current ownership before acting.
	Confirmed(Confirmation<R>),
	/// Parent decides whether to stop, inspect ownership, or prepare a distinct conditional attempt.
	Reconcile(R),
}

/// Activity after retry backoff; unrelated ownership/takeover waits stay in the parent.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
enum Next {
	/// Inspect whether the retained mutation landed.
	Read,
	/// Repeat exactly the retained request after an unchanged readback.
	Write,
}

/// Exclusive I/O activity; the request is retained in every phase, including suspension.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
enum Phase<O> {
	/// One PUT is pending; a timeout cannot establish that it failed remotely.
	Writing(IoCall<O>),
	/// One GET is pending; its classification comes from the parent.
	Reading(IoCall<O>),
	/// No I/O is pending; the correlated retry alarm chooses the next activity.
	Waiting {
		/// Last read token identifies this backoff, even after helper replacement.
		after: O,
		/// Evidence determines whether another read or the original PUT is safe.
		next: Next,
	},
	/// Local dispatch is revoked while the unresolved request remains available.
	Suspended,
	/// Evidence was handed to the parent exactly once; no further activity is accepted.
	Finished,
}

/// One prepared mutation. Construction/start emits effects, never executes them.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ConditionalWrite<W, O> {
	request: W,
	config: Config,
	phase: Phase<O>,
	/// Earliest repeat PUT, anchored to the last PUT rather than reset by readbacks.
	retry_at_boot_ms: u64,
}

impl<W: Clone, O: Copy + Eq> ConditionalWrite<W, O> {
	/// Begin an already-authorized mutation. IDs must come from the parent's lifetime allocator.
	pub fn start(
		request: W,
		config: Config,
		now_boot_ms: u64,
		op: O,
		effects: &mut Effects<Effect<W, O>, Timer<O>>,
	) -> Self {
		assert!(
			config.io_timeout_ms > 0 && config.retry_delay_ms > 0,
			"positive conditional I/O bounds required"
		);
		let mut state = Self {
			request,
			config,
			phase: Phase::Suspended,
			retry_at_boot_ms: 0,
		};

		state.issue(Next::Write, now_boot_ms, op, effects);
		state
	}

	/// Original immutable request, including its precondition and historical lease time origin.
	pub fn request(&self) -> &W {
		&self.request
	}
	/// Consume retained evidence only when the parent deliberately completes/reconciles this attempt.
	pub fn into_request(self) -> W {
		self.request
	}
	/// Current I/O token and dispatch deadline, derived from the activity.
	pub fn pending(&self) -> Option<&IoCall<O>> {
		match &self.phase {
			Phase::Writing(call) | Phase::Reading(call) => Some(call),
			_ => None,
		}
	}
	/// Useful for routing responses from a parent protocol with separate read/write variants.
	pub fn reading(&self) -> bool {
		matches!(self.phase, Phase::Reading(_))
	}
	/// Only the writing activity may accept a PUT response.
	pub fn writing(&self) -> bool {
		matches!(self.phase, Phase::Writing(_))
	}
	/// Suspension retains evidence but permits no executor dispatch or timer continuation.
	pub fn suspended(&self) -> bool {
		matches!(self.phase, Phase::Suspended)
	}

	fn cancel(&self, out: &mut Effects<Effect<W, O>, Timer<O>>) {
		match self.phase {
			Phase::Reading(call) | Phase::Writing(call) => {
				call.accept(call.op, Timer::Io, out);
			}
			Phase::Waiting { after, .. } => out.cancel_timer(Timer::Retry(after)),
			_ => {}
		}
	}

	/// Revoke local dispatch without treating cancellation as remote rollback.
	pub fn suspend(&mut self, effects: &mut Effects<Effect<W, O>, Timer<O>>) {
		self.cancel(effects);
		if !matches!(self.phase, Phase::Finished) {
			self.phase = Phase::Suspended;
		}
	}

	/// A parent with fresh authority may resume by reading, never by assuming a suspended PUT failed.
	pub fn resume(
		&mut self,
		now_boot_ms: u64,
		op: O,
		effects: &mut Effects<Effect<W, O>, Timer<O>>,
	) {
		if self.suspended() {
			self.issue(Next::Read, now_boot_ms, op, effects);
		}
	}

	fn issue(
		&mut self,
		next: Next,
		now_boot_ms: u64,
		op: O,
		out: &mut Effects<Effect<W, O>, Timer<O>>,
	) {
		let call = IoCall::arm(op, self.config.io_timeout_ms, Timer::Io, out);
		match next {
			Next::Write => {
				self.retry_at_boot_ms = now_boot_ms.saturating_add(self.config.retry_delay_ms);
				self.phase = Phase::Writing(call);
				out.execute(Effect::WriteRequest {
					op,
					request: self.request.clone(),
				});
			}
			Next::Read => {
				self.phase = Phase::Reading(call);
				out.execute(Effect::ReadRequest { op });
			}
		}
	}

	fn wait(
		&mut self,
		after: O,
		next: Next,
		at_boot_ms: u64,
		out: &mut Effects<Effect<W, O>, Timer<O>>,
	) {
		self.phase = Phase::Waiting { after, next };
		out.arm_timer_at(Timer::Retry(after), at_boot_ms);
	}

	/// Accept one correlated input/alarm. IDs are allocated only when new I/O is emitted.
	/// The caller/runner orders due alarms before inputs and preserves independent authority checks.
	pub fn on_event<R>(
		&mut self,
		now_boot_ms: u64,
		event: RuntimeEvent<Event<O, R>, Timer<O>>,
		mut allocate: impl FnMut() -> O,
		effects: &mut Effects<Effect<W, O>, Timer<O>>,
	) -> Option<Outcome<R>> {
		let mut outcome = None;
		// A timeout cannot determine whether remote work completed; reuse response policy.
		let event = match event {
			RuntimeEvent::Timer(Timer::Io(IoTimer::Io(op))) if matches!(self.phase, Phase::Writing(call) if call.op == op) => {
				RuntimeEvent::Input(Event::WriteResponse {
					op,
					result: WriteResult::Unknown,
				})
			}
			RuntimeEvent::Timer(Timer::Io(IoTimer::Io(op))) if matches!(self.phase, Phase::Reading(call) if call.op == op) => {
				RuntimeEvent::Input(Event::ReadResponse {
					op,
					result: Readback::Unknown,
				})
			}
			event => event,
		};
		match event {
			RuntimeEvent::Input(Event::WriteResponse { op, result }) if matches!(self.phase, Phase::Writing(call) if call.op == op) =>
			{
				self.cancel(effects);
				match result {
					WriteResult::Confirmed { etag } if !etag.is_empty() => {
						self.phase = Phase::Finished;
						outcome = Some(Outcome::Confirmed(Confirmation::Write { etag }));
					}
					_ => self.issue(Next::Read, now_boot_ms, allocate(), effects),
				}
			}
			RuntimeEvent::Input(Event::ReadResponse { op, result }) if matches!(self.phase, Phase::Reading(call) if call.op == op) =>
			{
				self.cancel(effects);
				match result {
					Readback::Confirmed(observed) => {
						self.phase = Phase::Finished;
						outcome = Some(Outcome::Confirmed(Confirmation::Read(observed)));
					}
					Readback::Reconcile(observed) => {
						self.phase = Phase::Finished;
						outcome = Some(Outcome::Reconcile(observed));
					}
					Readback::Unchanged if now_boot_ms >= self.retry_at_boot_ms => {
						self.issue(Next::Write, now_boot_ms, allocate(), effects)
					}
					Readback::Unchanged => {
						self.wait(op, Next::Write, self.retry_at_boot_ms, effects)
					}
					Readback::Unknown => {
						self.phase = Phase::Waiting {
							after: op,
							next: Next::Read,
						};
						effects.arm_timer_after(Timer::Retry(op), self.config.retry_delay_ms);
					}
				}
			}
			RuntimeEvent::Timer(Timer::Retry(op)) => {
				if let Phase::Waiting { after, next } = self.phase
					&& op == after
				{
					self.cancel(effects);
					self.issue(next, now_boot_ms, allocate(), effects);
				}
			}
			_ => {}
		}
		outcome
	}
}
