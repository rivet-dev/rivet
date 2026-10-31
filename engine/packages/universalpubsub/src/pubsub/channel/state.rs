//! The state machine of one peer channel.
//!
//! It holds only the phase and the requests waiting for replies, keyed by the channel's own
//! request ids. It does no I/O: the writer and reader tasks call it and act on what it returns.
//! The waiter type `W` is how a request's caller is answered; it is opaque here, so tests use
//! plain values.

use std::collections::BTreeMap;

/// Where the channel is in its life.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Phase {
	/// The channel's HTTP stream is being opened. Requests are accepted and wait to be written
	/// until it is ready.
	Opening,
	/// Frames flow both ways.
	Ready,
	/// The channel is gone. Every request that was pending when it closed was failed once.
	Closed,
}

/// Why a request was not accepted. Pending requests have no cap of their own: each holds an
/// outgoing permit of its class, so class caps bound them without one class refusing another.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Refusal {
	/// The channel is closed. Nothing was sent; the caller opens another channel
	/// unless this worker is shutting down.
	NotAccepting,
}

pub(crate) struct ChannelState<W> {
	phase: Phase,
	next_id: u64,
	pending: BTreeMap<u64, W>,
}

impl<W> ChannelState<W> {
	pub(crate) fn new() -> Self {
		Self {
			phase: Phase::Opening,
			next_id: 0,
			pending: BTreeMap::new(),
		}
	}

	

	/// Whether the channel's stream is open.
	pub(crate) fn is_ready(&self) -> bool {
		self.phase == Phase::Ready
	}

	

	/// Registers a request and returns its id, or gives the waiter back with the refusal.
	pub(crate) fn submit(&mut self, waiter: W) -> Result<u64, (Refusal, W)> {
		match self.phase {
			Phase::Opening | Phase::Ready => {}
			Phase::Closed => return Err((Refusal::NotAccepting, waiter)),
		}
		let id = self.next_id;
		self.next_id += 1;
		self.pending.insert(id, waiter);
		Ok(id)
	}

	/// The HTTP stream is open. A channel that closed while it opened stays closed.
	pub(crate) fn opened(&mut self) {
		if self.phase == Phase::Opening {
			self.phase = Phase::Ready;
		}
	}

	/// Takes the waiter a reply answers. A reply for an unknown id, such as one whose caller
	/// already left, returns nothing.
	pub(crate) fn respond(&mut self, id: u64) -> Option<W> {
		self.pending.remove(&id)
	}

	/// The caller stopped waiting. Returns whether the request was still pending, in which case
	/// the writer sends a cancel so the peer can stop its handler.
	pub(crate) fn cancel(&mut self, id: u64) -> bool {
		self.pending.remove(&id).is_some()
	}

	/// The channel failed or ended. Returns every pending waiter exactly once, for the caller to
	/// fail with an unknown outcome; a later call returns nothing.
	pub(crate) fn fail(&mut self) -> Vec<W> {
		self.phase = Phase::Closed;
		std::mem::take(&mut self.pending).into_values().collect()
	}
}


