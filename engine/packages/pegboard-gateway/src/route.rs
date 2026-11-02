//! The UPS route to an envoy connection, as an actor's allocation or an envoy record names it.
use anyhow::{Context, Result};
use pegboard::wire;
use uuid::Uuid;

/// One envoy connection's address on a UPS worker.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Route {
	pub(crate) worker: universalpubsub::WorkerId,
	pub(crate) connection_id: Uuid,
}

impl Route {
	/// The route an allocation records. It is a hint: the envoy may have reconnected since, in
	/// which case an open fails and the envoy record names the current route.
	pub(crate) fn of_allocation(allocation: &wire::ActorAllocation) -> Result<Self> {
		Self::parse(&allocation.worker_id, &allocation.connection_id)
	}

	/// The route of an envoy record's current connection.
	pub(crate) fn of_envoy(envoy: &wire::EnvoyData) -> Result<Self> {
		Self::parse(&envoy.worker_id, &envoy.connection_id)
	}

	/// `allocation` with this route in place of its recorded one, for an `UpdateRoute`.
	pub(crate) fn into_allocation(
		self,
		allocation: &wire::ActorAllocation,
	) -> wire::ActorAllocation {
		wire::ActorAllocation {
			envoy_key: allocation.envoy_key.clone(),
			worker_id: self.worker.as_bytes().to_vec().into(),
			connection_id: self.connection_id.as_bytes().to_vec().into(),
		}
	}

	pub(crate) fn subject(&self) -> pegboard::keys::ups::EnvoySubject {
		pegboard::keys::ups::EnvoySubject::new(self.connection_id)
	}

	fn parse(worker_id: &[u8], connection_id: &[u8]) -> Result<Self> {
		Ok(Self {
			worker: universalpubsub::WorkerId::from_bytes(
				worker_id.try_into().context("invalid envoy worker id")?,
			),
			connection_id: Uuid::from_slice(connection_id).context("invalid connection id")?,
		})
	}
}
