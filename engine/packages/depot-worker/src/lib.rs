//! Depot storage worker ownership, tablet coordination, and filesystem maintenance.
//!
//! Storage workers keep topology admission Active while waiting for a durable stop
//! permit, then join their storage services before draining and releasing membership.

#![warn(unreachable_pub)]

#[cfg(not(target_arch = "wasm32"))]
use std::sync::Arc;

#[cfg(not(target_arch = "wasm32"))]
use rivet_topology::topology::Topology;
#[cfg(not(target_arch = "wasm32"))]
use rivet_topology::worker::WorkerId;
#[cfg(not(target_arch = "wasm32"))]
use universalpubsub::PubSub;

// The coordinator and maintainer state machines and their alarm data build for wasm32.
// The simulator drives them directly; I/O implementations stay native-only.
pub mod alarms;
#[cfg(not(target_arch = "wasm32"))]
pub mod bootstrap;
#[cfg(not(target_arch = "wasm32"))]
pub mod command;
#[cfg(not(target_arch = "wasm32"))]
pub mod coordinator_requests;

pub mod disk_budget;
#[cfg(not(target_arch = "wasm32"))]
pub mod error;
#[cfg(not(target_arch = "wasm32"))]
pub mod filesystem;
mod kv_refusals;
#[cfg(not(target_arch = "wasm32"))]
mod metrics;
#[cfg(target_arch = "wasm32")]
#[path = "metrics_wasm.rs"]
mod metrics;
#[cfg(not(target_arch = "wasm32"))]
pub mod object_store_first_attempt;
#[cfg(not(target_arch = "wasm32"))]
pub mod object_store_metrics;
pub mod repair_types;
#[cfg(not(target_arch = "wasm32"))]
pub mod services;
#[cfg(not(target_arch = "wasm32"))]
mod signals;
/// The tablet balancing policy the coordinator reads.
#[cfg(not(target_arch = "wasm32"))]
pub mod balancing_config {
	pub use rivet_config::config::TabletBalancingConfig;
}
#[cfg(target_arch = "wasm32")]
#[path = "balancing_config_wasm.rs"]
pub mod balancing_config;
pub mod keys;
mod split_snapshot;
pub mod tablet_coordinator_machine;
pub mod tablet_maintainer_machine;
#[cfg(not(target_arch = "wasm32"))]
pub mod tablet_supervisor;

#[cfg(not(target_arch = "wasm32"))]
use error::Result;

#[cfg(not(target_arch = "wasm32"))]
pub struct Worker {
	worker_id: WorkerId,
	topology: Arc<Topology>,
}

#[cfg(not(target_arch = "wasm32"))]
impl Worker {
	#[tracing::instrument(skip_all)]
	pub async fn start(
		config: &rivet_config::Config,
		pubsub: PubSub,
		topology: Arc<Topology>,
	) -> Result<Self> {
		config.api_peer().validate()?;

		anyhow::ensure!(
			topology.may_admit()?,
			"Depot requires a registered, live topology"
		);
		let registration = topology
			.snapshot()?
			.worker_registration
			.clone()
			.ok_or_else(|| anyhow::anyhow!("Depot requires worker registration"))?;
		let identity = registration.identity;

		anyhow::ensure!(
			identity.worker_id == *pubsub.worker_id()
				&& identity.instance_id == pubsub.instance_id().into(),
			"Depot and UPS must share worker identity"
		);

		let worker_id = identity.worker_id;
		tracing::info!(%worker_id, "depot worker started");

		Ok(Self {
			worker_id,
			topology,
		})
	}

	pub fn worker_id(&self) -> &WorkerId {
		&self.worker_id
	}

	pub async fn close(self) -> Result<()> {
		tracing::info!("closing depot worker");
		self.topology.begin_drain().await?;
		self.topology.shutdown().await?;
		tracing::info!("depot worker closed");
		Ok(())
	}
}

#[cfg(not(target_arch = "wasm32"))]
pub mod status_notifier;
#[cfg(not(target_arch = "wasm32"))]
pub mod stop_requests;
