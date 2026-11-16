//! Routing for a cluster that is taking over one V2 namespace. This cluster serves the actors it
//! holds and proxies the rest to V2. New keyless creates move here by the configured share; a
//! keyed create's state machine places its key itself. With no `migrate_from_v2` config, or a
//! request in another namespace, routing is unchanged.

pub mod decision;
pub mod metrics;
pub mod proxy;

use anyhow::Result;
use hyper::{HeaderMap, header::SEC_WEBSOCKET_PROTOCOL};
use pegboard::{errors::ActorInvalidVersion, keys::partition::ActorVolume, wire};
use rivet_config::config::{MigrateFromV2, Root};
use rivet_engine_context::EngineContext;
use rivet_util::Id;
use std::{borrow::Cow, sync::Arc};

pub use decision::{Route, V3Actor};

/// The migration that applies to one request.
pub struct Active<'a> {
	pub config: &'a MigrateFromV2,
	/// The namespace, as a metric label.
	pub namespace: String,
}

impl<'a> Active<'a> {
	/// The migration for a request in `namespace`, when that is the namespace being migrated
	/// into. Records the configured create share as it is read.
	pub fn for_namespace(dynamic: Option<&'a Root>, namespace: &str) -> Option<Self> {
		let config = applies(dynamic, namespace)?;
		Some(Self {
			config,
			namespace: namespace.to_owned(),
		})
	}

	pub fn count_request(&self, route: &'static str) {
		metrics::REQUESTS
			.with_label_values(&[&self.namespace, route])
			.inc();
	}
}

/// Request header with which a create asks to land in this cluster while a V2 namespace migrates
/// in. `3` creates the actor here whatever `create_ratio` says: a keyless create lands here, and
/// a keyed getOrCreate lands here unless its key already belongs to V2. Without a migration the
/// header changes nothing, since every create lands here anyway.
pub const X_RIVET_ACTOR_VERSION: &str = "x-rivet-actor-version";

/// WebSocket subprotocol prefix carrying the same request as [`X_RIVET_ACTOR_VERSION`], as
/// `rivet_actor_version.3`, for clients such as browsers that cannot set upgrade headers.
pub const WS_PROTOCOL_ACTOR_VERSION: &str = "rivet_actor_version.";

/// The create share one request decides against: 1.0 when it asks for this cluster with
/// [`X_RIVET_ACTOR_VERSION`] or [`WS_PROTOCOL_ACTOR_VERSION`], and the configured `create_ratio`
/// otherwise. The header wins when a request carries both.
pub fn create_ratio(config: &MigrateFromV2, headers: &HeaderMap) -> Result<f64> {
	if forces_actor_v3(headers)? {
		Ok(1.0)
	} else {
		Ok(config.create_ratio)
	}
}

/// Whether the request asks to be created here with [`X_RIVET_ACTOR_VERSION`] or
/// [`WS_PROTOCOL_ACTOR_VERSION`]. The header wins when a request carries both, and any version
/// other than `3` is refused.
pub fn forces_actor_v3(headers: &HeaderMap) -> Result<bool> {
	let version = match headers.get(X_RIVET_ACTOR_VERSION) {
		Some(value) => Some(String::from_utf8_lossy(value.as_bytes())),
		None => requested_protocol_version(headers).map(Cow::Borrowed),
	};
	match version.as_deref() {
		None => Ok(false),
		Some("3") => Ok(true),
		Some(version) => Err(ActorInvalidVersion {
			version: version.to_owned(),
		}
		.build()),
	}
}

/// The version a WebSocket upgrade names with a [`WS_PROTOCOL_ACTOR_VERSION`] subprotocol.
fn requested_protocol_version(headers: &HeaderMap) -> Option<&str> {
	headers
		.get_all(SEC_WEBSOCKET_PROTOCOL)
		.iter()
		.filter_map(|value| value.to_str().ok())
		.flat_map(|protocols| protocols.split(','))
		.find_map(|protocol| protocol.trim().strip_prefix(WS_PROTOCOL_ACTOR_VERSION))
}

/// The dynamic config while a migration from V2 is configured, and `None` otherwise. The check
/// is one lock-free load, so a request path pays nothing more when no migration is configured;
/// the snapshot it returns may be held across awaits.
pub fn snapshot(ctx: &EngineContext) -> Option<Arc<Root>> {
	if ctx.config().dynamic_ref().migrate_from_v2().is_none() {
		return None;
	}
	Some(ctx.config().dynamic())
}

/// The migration config of a [`snapshot`] when it applies to `namespace`. Records the configured
/// create share as it is read.
pub fn applies<'a>(dynamic: Option<&'a Root>, namespace: &str) -> Option<&'a MigrateFromV2> {
	let config = dynamic?.migrate_from_v2()?;
	if namespace != config.target_namespace() {
		return None;
	}
	metrics::CREATE_RATIO
		.with_label_values(&[namespace])
		.set(config.create_ratio);
	Some(config)
}

/// Reads what this cluster holds for `actor_id`, whose volumes are keyed in `group`, without
/// waiting for it.
pub async fn read_v3_actor(
	ctx: &EngineContext,
	group: Option<rivet_keys::keys::codec::GroupId>,
	actor_id: Id,
) -> Result<V3Actor> {
	let read = ctx
		.depot()
		.command::<pegboard::commands::ActorRead>(&ActorVolume::in_namespace(group, actor_id), ())
		.await;
	let read = match read {
		Ok(read) => read,
		Err(error) if pegboard::commands::is_volume_gone(&error) => return Ok(V3Actor::None),
		Err(error) => return Err(error),
	};
	Ok(classify(read.data.is_some(), read.phase.as_ref()))
}

/// What a read's data presence and phase mean for routing.
pub fn classify(has_data: bool, phase: Option<&wire::ActorPhase>) -> V3Actor {
	match phase {
		Some(wire::ActorPhase::MigratingFromV2(_)) => V3Actor::Held,
		Some(wire::ActorPhase::DelegatedToV2) => V3Actor::Delegated,
		Some(wire::ActorPhase::Destroying(_) | wire::ActorPhase::Destroyed) | None => V3Actor::None,
		Some(
			wire::ActorPhase::Initializing(_)
			| wire::ActorPhase::Starting(_)
			| wire::ActorPhase::Running(_)
			| wire::ActorPhase::Stopping(_)
			| wire::ActorPhase::Sleeping(_),
		) => {
			// An actor whose data is not written yet is still being created.
			if has_data {
				V3Actor::Active
			} else {
				V3Actor::None
			}
		}
	}
}
