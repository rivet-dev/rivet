//! Judging the envoy an actor was allocated to, after a request to it failed.
//!
//! An actor learns that its envoy no longer hosts its epoch only from whoever next needs the
//! actor: a gateway request, or the alarm probe the actor state machine runs. Both open a UPS
//! stream to the allocation's route; a failed open is the liveness check, and one read of the
//! envoy's record decides whether the envoy reconnected elsewhere, is draining, or is gone.
use crate::commands::{EnvoyRead, is_volume_gone};
use crate::keys::partition::{ActorVolume, EnvoyVolume};
use crate::wire;
use anyhow::{Context, Result};
use rivet_engine_context::EngineContext;
use rivet_keys::keys::codec::GroupId;
use rivet_util::Id;
use uuid::Uuid;

/// One envoy connection's address on a UPS worker.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Route {
	pub worker: universalpubsub::WorkerId,
	pub connection_id: Uuid,
}

impl Route {
	/// The route an allocation records. It is a hint: the envoy may have reconnected since, in
	/// which case an open fails and the envoy record names the current route.
	pub fn of_allocation(allocation: &wire::ActorAllocation) -> Result<Self> {
		Self::parse(&allocation.worker_id, &allocation.connection_id)
	}

	/// The route of an envoy record's current connection.
	pub fn of_envoy(envoy: &wire::EnvoyData) -> Result<Self> {
		Self::parse(&envoy.worker_id, &envoy.connection_id)
	}

	/// `allocation` with this route in place of its recorded one, for an `UpdateRoute`.
	pub fn into_allocation(self, allocation: &wire::ActorAllocation) -> wire::ActorAllocation {
		wire::ActorAllocation {
			envoy_key: allocation.envoy_key.clone(),
			worker_id: self.worker.as_bytes().to_vec().into(),
			connection_id: self.connection_id.as_bytes().to_vec().into(),
		}
	}

	pub fn subject(&self) -> crate::keys::ups::EnvoySubject {
		crate::keys::ups::EnvoySubject::new(self.connection_id)
	}

	fn parse(worker_id: &[u8], connection_id: &[u8]) -> Result<Self> {
		Ok(Self {
			worker: universalpubsub::WorkerId::from_bytes(
				worker_id.try_into().context("invalid envoy worker id")?,
			),
			connection_id: Uuid::from_slice(connection_id)
				.context("invalid envoy connection id")?,
		})
	}
}

/// What one fresh read of an envoy's record, and its worker's lease, says about the envoy an
/// actor was allocated to.
pub enum EnvoyCheck {
	/// The envoy serves on the allocation's connection, or is disconnected for now; wait.
	Serving,
	/// The same envoy process reconnected and kept its actors; open its newer connection.
	Newer(Route),
	/// The envoy or its connection is gone. Reallocation stops the old epoch before replacing it.
	Lost,
	/// The envoy is draining.
	Draining,
}

/// Reads the envoy's record once and judges it against `allocation`. Nothing is cached: each
/// check is one durable read, so callers run it only after the allocation's route refused them
/// or on a fallback poll. `namespace` is the namespace the actor is served in, whose envoys
/// host it: for an actor inherited through a namespace fork it is not the origin namespace its
/// data names. `route_refused` says the route's worker answered that it holds no
/// subscriber for the connection, as a `no_responders` open failure does.
pub async fn envoy_check(
	ctx: &EngineContext,
	namespace: &str,
	actor: &wire::ActorData,
	allocation: &wire::ActorAllocation,
	route_refused: bool,
) -> Result<EnvoyCheck> {
	let route = Route::of_allocation(allocation)?;
	let volume = EnvoyVolume::new(
		namespace.to_string(),
		actor.pool_name.to_string(),
		allocation.envoy_key.to_string(),
	)
	.volume_key();
	let envoy = match ctx.depot().command::<EnvoyRead>(&volume, ()).await {
		Ok(response) => response.data,
		Err(error) if is_volume_gone(&error) => None,
		Err(error) => return Err(error),
	};
	let Some(envoy) = envoy else {
		return Ok(EnvoyCheck::Lost);
	};
	if envoy.stopping {
		return Ok(EnvoyCheck::Draining);
	}

	let now = ctx.env().clock().observe()?.unix_ms;
	// A threshold past the timestamp range never expires.
	let lost_threshold_ms =
		i64::try_from(ctx.config().pegboard().envoy_lost_threshold()).unwrap_or(i64::MAX);
	let heartbeat_expired = envoy.heartbeat_unix_ms.saturating_add(lost_threshold_ms) < now;
	if !envoy.connected && heartbeat_expired {
		return Ok(EnvoyCheck::Lost);
	}

	let current = Route::of_envoy(&envoy)?;
	// A restarted engine keeps its worker ID, so its live lease cannot prove that an
	// old connection ended. A no-responders reply can. Prefer a newer connection
	// when the record has one; otherwise enter the existing stop protocol, which records
	// the disconnect and waits out the node's lost threshold before ending the epoch.
	if envoy.connected && current == route && route_refused {
		return Ok(EnvoyCheck::Lost);
	}
	let lease_ended = match ctx.topology().worker_lease(&route.worker).await {
		rivet_topology::WorkerLease::Expired | rivet_topology::WorkerLease::Released => true,
		rivet_topology::WorkerLease::Live | rivet_topology::WorkerLease::Unknown => false,
	};
	// An ended worker lease only closes the envoy's connection, and the dead worker may have
	// left the record marked connected. The envoy keeps its actors until its own lost threshold,
	// which it learned from this config, so it may still reconnect with them.
	if lease_ended && heartbeat_expired && current.connection_id == route.connection_id {
		return Ok(EnvoyCheck::Lost);
	}
	if envoy.connected && current.connection_id != route.connection_id {
		return Ok(EnvoyCheck::Newer(current));
	}
	Ok(EnvoyCheck::Serving)
}

/// Records the envoy's newer route on the actor's allocation, so later requests of the epoch
/// open it first. `group` is the group of the namespace the request was resolved in, which holds
/// the actor's state volume (absent for a pre-group namespace). Best effort and off the caller's path: the state machine applies it only if
/// the allocation still equals `from`, and a failure costs the next request one more fallback.
pub fn repair_route(
	ctx: &EngineContext,
	group: Option<GroupId>,
	actor_id: Id,
	epoch: u32,
	from: &wire::ActorAllocation,
	newer: Route,
) {
	let event = wire::ActorEvent::UpdateRoute(wire::UpdateRoute {
		epoch,
		from: from.clone(),
		to: newer.into_allocation(from),
	});
	let ctx2 = ctx.clone();
	let _detached = ctx.env().spawn(Box::pin(async move {
		let sent = ctx2
			.depot()
			.command::<crate::commands::ActorEvent>(&ActorVolume::in_namespace(group, actor_id), event)
			.await;
		if let Err(error) = sent {
			tracing::debug!(%actor_id, ?error, "actor route repair failed; the next request falls back again");
		}
	}));
}

/// Reports `epoch` for reallocation: lost when its envoy no longer hosts it, otherwise draining.
/// `group` is the group of the envoy's namespace, which holds the actor's state volume.
/// The state machine stops a lost epoch and waits for the envoy's stopped report, or its proven
/// absence, before starting the next, so a wrong report costs a restart, never a second writer.
pub async fn report_reallocate(
	ctx: &EngineContext,
	group: Option<GroupId>,
	actor_id: Id,
	epoch: u32,
	lost: bool,
) -> Result<()> {
	let event = wire::ActorEvent::Reallocate(wire::Reallocate {
		epoch,
		lost,
		waiter: None,
	});
	ctx.depot()
		.command::<crate::commands::ActorEvent>(&ActorVolume::in_namespace(group, actor_id), event)
		.await?;
	Ok(())
}

/// The HTTP abort detail and WebSocket close reason pegboard-envoy sends on a gateway tunnel
/// whose actor epoch stopped. The gateway answers an HTTP abort carrying it before response
/// headers with the same retryable refusal as an envoy admission refusal.
pub const ACTOR_STOPPED_DETAIL: &str = "actor.stopped";

/// Whether an envoy refused a request or WebSocket open with `code` before the actor admitted
/// it: it does not host the actor, or not the generation the request named.
pub fn refuses_admission(code: &str) -> bool {
	matches!(
		code,
		"envoy.actor_not_found" | "envoy.actor_generation_mismatch"
	)
}

/// Whether the peer has no registration for this connection. A live connection
/// draining accepted work retains its registration and returns consumer_closed.
pub fn no_responders(error: &anyhow::Error) -> bool {
	let structured = rivet_error::RivetError::extract(error);
	structured.group() == "ups" && structured.code() == "no_responders"
}

/// Whether a UPS request failed without the worker answering: its route is unknown, the
/// transport failed, or the request timed out.
pub fn unreachable_worker(error: &anyhow::Error) -> bool {
	let structured = rivet_error::RivetError::extract(error);
	structured.group() == "ups"
		&& matches!(
			structured.code(),
			"unknown_worker" | "transport" | "timeout"
		)
}

/// Whether a failed UPS request to an envoy connection may succeed on another route: a stale
/// route, a missing worker or a transport failure, as opposed to a refused or malformed request.
pub fn retryable_ups_error(error: &anyhow::Error) -> bool {
	let structured = rivet_error::RivetError::extract(error);
	structured.group() == "ups"
		&& matches!(
			structured.code(),
			"no_responders"
				| "unknown_worker"
				| "wrong_worker"
				| "timeout" | "consumer_closed"
				| "transport"
		)
}
