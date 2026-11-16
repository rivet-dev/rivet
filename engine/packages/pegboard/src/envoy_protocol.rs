//! The envoy protocol version engine workers encode the envoy messages they pass to each other over
//! UPS: pegboard's start and stop commands and SQLite tail proof requests and the gateway's tunnel
//! messages to pegboard-envoy, and pegboard-envoy's tunnel replies to the gateway. During a rolling
//! deploy an engine worker from before the latest version refuses a message encoded at it, so each
//! sender encodes at the version its receiving worker reads. That worker advertises the
//! `envoy.protocol` version family in its topology heartbeat; a binary from before the family does
//! not advertise it and reads the oldest supported version.
//!
//! The receiver's versions come from topology's in-memory view only: discovery for storage
//! workers and the exact-id endpoint cache for stateless workers, which the UPS request to that
//! worker fills. A message to this worker itself is delivered in process, so its receiver is
//! this binary and gets the latest version. A receiver the view does not know gets the oldest
//! supported version, which every engine worker reads, so a cold cache costs the newest
//! features of one message, never its delivery. The envoy WebSocket is separate: it negotiates
//! its version per connection.

use rivet_engine_context::EngineContext;
use rivet_envoy_protocol::{PROTOCOL_VERSION, versioned::MIN_PROTOCOL_VERSION};
use rivet_topology::{versions::VersionFamily, worker::WorkerId};

/// How long a caller refused while older engine workers run waits before retrying.
pub const FLEET_PENDING_RETRY_AFTER_MS: u64 = 5_000;

/// The version family engine workers advertise for the envoy protocol.
pub const FAMILY: VersionFamily = VersionFamily::ENVOY_PROTOCOL;

/// The kind of engine-internal envoy message being encoded, for the downgrade counter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InternalMessage {
	/// A start command to the envoy's host worker.
	Start,
	/// A stop command to the envoy's host worker.
	Stop,
	/// A SQLite tail proof request to the worker holding a running fork source's database.
	ProveTail,
	/// A gateway tunnel to the envoy's host worker, counted once per tunnel.
	Tunnel,
	/// Pegboard-envoy's replies to the gateway on one tunnel, counted once per tunnel.
	TunnelReply,
}

impl InternalMessage {
	fn as_str(self) -> &'static str {
		match self {
			Self::Start => "start",
			Self::Stop => "stop",
			Self::ProveTail => "prove_tail",
			Self::Tunnel => "tunnel",
			Self::TunnelReply => "tunnel_reply",
		}
	}
}

/// Whether every engine worker this worker can see reads the latest envoy protocol. Gates
/// actor forks at admission, because the envoy that will host the forked actor is not known
/// then and the start carrying the fork always encodes the latest version.
///
/// Topology lists only storage workers, and stateless workers are never listed, so this reads
/// the storage workers' floor of the envoy protocol family. A forked start to a stateless worker
/// that still reads an older version fails, either before sending when topology's view knows
/// that worker is old or by the request failing when it does not; the actor then reports a
/// failed start and sleeps, and its next wake allocates again. So a forked actor never starts
/// without its fork. An unknown view of the fleet reads as not current.
///
/// Reads the topology snapshot in memory; no I/O.
pub fn fleet_current(ctx: &EngineContext) -> bool {
	fleet_current_for(ctx.topology().version_for(FAMILY))
}

/// [`fleet_current`] from a fleet read of the envoy protocol family.
pub fn fleet_current_for(fleet_version: anyhow::Result<u16>) -> bool {
	matches!(fleet_version, Ok(version) if version >= PROTOCOL_VERSION)
}

/// The version an engine-internal envoy message to `worker` is encoded at, recorded as a
/// downgrade when it is older than the latest. Reads memory only; see the module doc.
pub fn version_for_worker(ctx: &EngineContext, worker: &WorkerId, message: InternalMessage) -> u16 {
	let version = peer_version(ctx, worker);
	record(version, message);
	version
}

/// The version pegboard-envoy encodes its tunnel replies to `gateway` at: the version the
/// gateway reads by topology's view, or the version of the gateway's own first message when
/// that is newer, since a worker that writes a version reads it.
pub fn reply_version(ctx: &EngineContext, gateway: &WorkerId, request_version: u16) -> u16 {
	let version = reply_version_for(peer_version(ctx, gateway), request_version);
	record(version, InternalMessage::TunnelReply);
	version
}

/// [`reply_version`] from the gateway's version by topology's view and its request's version.
pub fn reply_version_for(peer_version: u16, request_version: u16) -> u16 {
	peer_version
		.max(request_version)
		.clamp(MIN_PROTOCOL_VERSION, PROTOCOL_VERSION)
}

/// The version `worker` reads by topology's in-memory view, without recording it.
fn peer_version(ctx: &EngineContext, worker: &WorkerId) -> u16 {
	receiver_version(ctx, worker).unwrap_or(MIN_PROTOCOL_VERSION)
}

/// Whether topology's view knows that `worker` reads an envoy protocol older than the latest,
/// so a message only the latest version carries, such as a forked start, cannot reach it. A
/// receiver the view does not know is not known to be old. Reads memory only.
pub(crate) fn known_older(ctx: &EngineContext, worker: &WorkerId) -> bool {
	known_older_for(receiver_version(ctx, worker))
}

/// [`known_older`] from the receiver's version, `None` when the view does not know it.
fn known_older_for(receiver_version: Option<u16>) -> bool {
	receiver_version.is_some_and(|version| version < PROTOCOL_VERSION)
}

/// The version `worker` reads, or `None` when topology's in-memory view does not know it.
///
/// A message to this worker itself never leaves the process: UPS delivers self-addressed
/// requests and streams locally without resolving the receiver, so a stateless worker's own id
/// is never in the endpoint cache. The receiver is then this binary, which reads the latest.
fn receiver_version(ctx: &EngineContext, worker: &WorkerId) -> Option<u16> {
	let own = worker == &ctx.worker_identity().worker_id;
	let lookup = if own {
		None
	} else {
		match ctx.topology().cached_peer_version(FAMILY, worker) {
			Ok(lookup) => lookup,
			Err(error) => {
				tracing::debug!(
					?error,
					"topology view unavailable; encoding the envoy message at the oldest supported version"
				);
				None
			}
		}
	};
	receiver_version_for(own, lookup)
}

/// The version a receiver reads: the latest when it is this worker, `None` when topology's view
/// does not know it, and otherwise its lookup. A lookup error means the receiver advertises no
/// envoy protocol version this binary writes, as a binary from before the family does not, and
/// reads as the oldest supported version, which every engine worker decodes.
fn receiver_version_for(own: bool, lookup: Option<anyhow::Result<u16>>) -> Option<u16> {
	if own {
		return Some(PROTOCOL_VERSION);
	}
	match lookup? {
		Ok(version) => Some(version.clamp(MIN_PROTOCOL_VERSION, PROTOCOL_VERSION)),
		Err(_) => Some(MIN_PROTOCOL_VERSION),
	}
}

/// Counts a message encoded below the latest version and warns at a falling rate: on the 1st,
/// 2nd, 4th, 8th and so on per message kind, and then every 1024th. Downgrades are expected
/// while a deploy rolls; a count that keeps growing after it means an old engine worker or a
/// receiver topology never resolves is still being sent to.
fn record(version: u16, message: InternalMessage) {
	if version >= PROTOCOL_VERSION {
		return;
	}
	let counter = crate::metrics::ENVOY_INTERNAL_DOWNGRADES.with_label_values(&[message.as_str()]);
	counter.inc();
	let count = counter.get();
	if (count.is_power_of_two() && count <= 1024) || count % 1024 == 0 {
		tracing::warn!(
			message_kind = message.as_str(),
			version,
			latest = PROTOCOL_VERSION,
			count,
			"encoded an engine-internal envoy message below the latest version; the receiving engine worker runs an older binary or topology does not know it yet"
		);
	}
}
