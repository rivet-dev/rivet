use rivet_state_machine::Effects;
use rivet_state_machine::{OpId, OpSequence};
use serde::{Deserialize, Serialize};

/// One phase-owned executor exchange; completion correlation does not recall remote work.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct IoCall<O> {
	/// Captured by the request and returned by its response.
	pub op: O,

	/// Latest permitted dispatch time, also used to arm the operation's timeout.
	pub deadline_boot_ms: u64,
}

/// Correlated alarm embedded in a parent's timer enum.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum IoTimer<O> {
	/// The parent decides whether this exchange retries, reconciles, or enters cleanup.
	Io(O),
}

impl<S: Copy + Ord> IoCall<OpId<S>> {
	/// Mints an ID and arms its timeout; the caller stores this call in its current phase.
	pub fn issue<E, T>(
		seq: &mut OpSequence<S>,
		slot: S,
		timeout_ms: u64,
		wrap: impl Fn(IoTimer<OpId<S>>) -> T,
		out: &mut Effects<E, T>,
	) -> Self {
		Self::arm(seq.next(slot), timeout_ms, wrap, out)
	}
}

impl<O: Copy + Eq> IoCall<O> {
	/// Arms an already allocated ID, including IDs tracked by InFlightOps or a publication lane.
	pub fn arm<E, T>(
		op: O,
		timeout_ms: u64,
		wrap: impl Fn(IoTimer<O>) -> T,
		out: &mut Effects<E, T>,
	) -> Self {
		let call = Self {
			op,
			deadline_boot_ms: out.now_boot_ms().saturating_add(timeout_ms),
		};
		out.arm_timer_after(wrap(IoTimer::Io(op)), timeout_ms);
		call
	}

	/// Cancels only a matching response's alarm. The parent must then leave the pending phase.
	pub fn accept<E, T>(
		&self,
		op: O,
		wrap: impl Fn(IoTimer<O>) -> T,
		out: &mut Effects<E, T>,
	) -> bool {
		if self.op != op {
			return false;
		}
		out.cancel_timer(wrap(IoTimer::Io(op)));
		true
	}
}
