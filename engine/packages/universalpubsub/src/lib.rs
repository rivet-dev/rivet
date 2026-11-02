//! Request/reply and packet streams only, both using exact formal byte subjects.
//!
//! Requests and packet streams address a known worker. Request/reply requires one
//! response packet and a clean end. The streaming helpers support one-to-many,
//! many-to-one, and duplex exchange over HTTP/2, with no automatic replay.
//! StreamOptions::lifetime defaults to no limit; opening still has a bounded timeout.
//! Split a duplex stream to send and receive concurrently. Finish the sender to
//! half-close; dropping an unfinished sender fails the other side.
//!
//! Packet buffers apply backpressure; new streams still fail fast at admission caps.
//! Rates come from `rate(rivet_ups_stream_packets_total[5m])` and
//! `rate(rivet_ups_stream_bytes_total[5m])`, grouped by sent/received direction.
//! These count payloads at the UPS boundary, not TCP acknowledgements or framing.
//! Active-stream, duration, error, buffer-depth, and configured-capacity metrics
//! describe resource use without worker or subject labels.
//! Address resolution uses shared worker topology; a deadline does not undo a mutation.
//! Worker-addressed APIs take topology's `WorkerId`; binary messages encode its
//! 16 UUID bytes and carry a separate process incarnation for fencing.
//! UPS has no broadcast. Multi-worker notification will be ephemeral, best-effort gossip.
//!
//! Every request names a `RequestClass`, and each class has its own outgoing and incoming
//! caps, so held tunnels cannot starve commands or Depot traffic. Depot classes travel on
//! reserved subjects whose window is their incoming cap. Header reads share one small pool that
//! is released once the class is known. Admission rejects excess work immediately; packet
//! buffers apply backpressure. Metrics expose capacity, usage, and rejection reasons by class;
//! threshold crossings warn.

pub mod errors;
mod metrics;
mod pubsub;
pub mod services;

pub use rivet_config::config::pubsub::{ClassCapacities, ProcessClassLimits};

pub use pubsub::{
	Options, PubSub, WorkerId,
	class::{ClassScope, RequestClass},
	order::{OrderTicket, Turn, Turns},
	request::{RequestMessage, RequestSubscriber},
	stream::{
		DuplexStream, PacketReceiver, PacketSender, RequestInputStream, StreamOptions,
		StreamSubscriber,
	},
	subject::{SubjectCapacities, SubjectReservation},
};
