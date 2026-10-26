use anyhow::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;

pub mod api_peer;
pub mod auth;
pub mod depot;
pub mod epoxy;
pub mod guard;
pub mod logs;
pub mod metrics;
pub mod object_storage;
pub mod outbound;
pub mod pegboard;
pub mod pubsub;
pub mod runtime;
pub mod telemetry;
pub mod topology;

pub use api_peer::*;
pub use auth::*;
pub use depot::*;
pub use epoxy::*;
pub use guard::*;
pub use logs::*;
pub use metrics::*;
pub use object_storage::*;
pub use outbound::*;
pub use pegboard::*;
pub use pubsub::PubSub;
pub use runtime::*;
pub use telemetry::*;
pub use topology::*;

// IMPORTANT:
//
// Do not use Vec unless it is `Vec<String>`. Use a `HashMap` instead.
//
// This is because all values need to be able to be configured using environment variables.
// config-rs can only parse `Vec<String>` from the environment.
//
// IMPORTANT:
//
// Everything at the root should be `Option`.
//
// This is because we use the `config` crate. If we were to provide a default value to `config` of
// `Root::default` (without options), it'll try to merge the keys by key-value instead of
// intelligently "reassigning" structs.
//
// This means that a lot of properties like enums cannot be overridden.
//
// By using `Option`, we can manually implement our own default using methods + `LazyLock` (for
// performance).

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Root {
	#[serde(default)]
	pub auth: Option<Auth>,

	#[serde(default)]
	pub guard: Option<Guard>,

	#[serde(default)]
	#[serde(rename = "storage")]
	pub depot: Option<DepotConfig>,

	#[serde(default)]
	pub object_storage: Option<ObjectStorageConfig>,

	#[serde(default)]
	pub api_peer: Option<ApiPeer>,

	#[serde(default)]
	pub pegboard: Option<Pegboard>,

	#[serde(default)]
	pub epoxy: Option<Epoxy>,

	#[serde(default)]
	pub logs: Option<Logs>,

	#[serde(default)]
	pub topology: Option<Topology>,

	#[serde(default)]
	pub pubsub: Option<PubSub>,

	#[serde(default)]
	pub telemetry: Telemetry,

	#[serde(default)]
	pub runtime: Runtime,

	#[serde(default)]
	pub metrics: Metrics,

	#[serde(default)]
	pub outbound: Option<Outbound>,
}

impl Default for Root {
	fn default() -> Self {
		Root {
			auth: None,
			guard: None,
			depot: None,
			object_storage: None,
			api_peer: None,
			pegboard: None,
			epoxy: None,
			logs: None,
			topology: None,
			pubsub: None,
			telemetry: Default::default(),
			runtime: Default::default(),
			metrics: Default::default(),
			outbound: None,
		}
	}
}

impl Root {
	pub fn insecure_allow_unauthenticated(&self) -> bool {
		self.auth
			.as_ref()
			.is_some_and(|auth| auth.insecure_allow_unauthenticated)
	}

	pub fn guard(&self) -> &Guard {
		static DEFAULT: LazyLock<Guard> = LazyLock::new(Guard::default);
		self.guard.as_ref().unwrap_or(&DEFAULT)
	}

	pub fn depot(&self) -> &DepotConfig {
		static DEFAULT: LazyLock<DepotConfig> = LazyLock::new(DepotConfig::default);
		self.depot.as_ref().unwrap_or(&DEFAULT)
	}

	pub fn object_storage(&self) -> &ObjectStorageConfig {
		static DEFAULT: LazyLock<ObjectStorageConfig> = LazyLock::new(ObjectStorageConfig::default);
		self.object_storage.as_ref().unwrap_or(&DEFAULT)
	}

	pub fn api_peer(&self) -> &ApiPeer {
		static DEFAULT: LazyLock<ApiPeer> = LazyLock::new(ApiPeer::default);
		self.api_peer.as_ref().unwrap_or(&DEFAULT)
	}

	pub fn outbound(&self) -> &Outbound {
		static DEFAULT: LazyLock<Outbound> = LazyLock::new(Outbound::default);
		self.outbound.as_ref().unwrap_or(&DEFAULT)
	}

	pub fn epoxy(&self) -> &Epoxy {
		static DEFAULT: LazyLock<Epoxy> = LazyLock::new(Epoxy::default);
		self.epoxy.as_ref().unwrap_or(&DEFAULT)
	}

	pub fn pegboard(&self) -> &Pegboard {
		static DEFAULT: LazyLock<Pegboard> = LazyLock::new(Pegboard::default);
		self.pegboard.as_ref().unwrap_or(&DEFAULT)
	}

	pub fn logs(&self) -> &Logs {
		static DEFAULT: LazyLock<Logs> = LazyLock::new(Logs::default);
		self.logs.as_ref().unwrap_or(&DEFAULT)
	}

	pub fn topology(&self) -> &Topology {
		static DEFAULT: LazyLock<Topology> = LazyLock::new(Topology::default);
		self.topology.as_ref().unwrap_or(&DEFAULT)
	}

	pub fn pubsub(&self) -> &PubSub {
		static DEFAULT: LazyLock<PubSub> = LazyLock::new(PubSub::default);
		self.pubsub.as_ref().unwrap_or(&DEFAULT)
	}



	pub fn validate_and_set_defaults(&mut self) -> Result<()> {
		let auth = self
			.auth
			.as_ref()
			.context("Authentication is required. Configure auth.admin_token and send a Bearer token with requests. To restore legacy unauthenticated access behavior, also set auth.insecure_allow_unauthenticated=true.")?;
		let desired_issuer = auth::derive_issuer(self)?;
		auth.validate(&desired_issuer)?;

		self.pubsub().validate()?;

		self.depot().validate()?;
		self.pegboard().validate()?;
		self.epoxy().validate()?;
		self.runtime.validate()?;
		self.metrics.hardware.validate()?;
		// Validate that all datacenters have valid_hosts configured when there's more than one datacenter
		let topology = self.topology();
		if topology.datacenters.len() > 1 {
			for dc in &topology.datacenters {
				if dc.valid_hosts.is_none() {
					bail!(
						"datacenter '{}' must have valid_hosts configured when there is more than one datacenter",
						dc.name
					);
				}
			}
		}

		// Set name property for map topology
		if let Some(topology) = &mut self.topology {
			match &mut topology.datacenters {
				DatacentersRepr::Map(m) => {
					for (name, dc) in m {
						if !dc.name.is_empty() {
							bail!(
								"datacenter '{}' cannot have the `name` property set because it is automatically derived from key",
								dc.name
							);
						}

						dc.name = name.clone();
					}
				}
				DatacentersRepr::List(l) => {
					for (i, dc) in l.iter().enumerate() {
						if dc.name.is_empty() {
							bail!("datacenter at index {} must have a name", i);
						}
					}
				}
			}
		}

		// Validate force_shutdown_duration covers worker and guard shutdown durations
		let worker = self.runtime.worker_shutdown_duration();
		let guard = self.runtime.guard_shutdown_duration();
		let force = self.runtime.force_shutdown_duration();
		let max_graceful = worker.max(guard);
		if force < max_graceful {
			bail!(
				"force_shutdown_duration ({force:?}) must be greater than or equal to both \
				worker_shutdown_duration ({worker:?}) and guard_shutdown_duration ({guard:?})"
			);
		}

		Ok(())
	}

	/// Alias of `dc_label`. This is for convenience & clarify when reading code.
	pub fn epoxy_replica_id(&self) -> u64 {
		self.dc_label() as u64
	}

	/// Current datacenter's label.
	pub fn dc_label(&self) -> u16 {
		self.topology().datacenter_label
	}

	pub fn dc_name(&self) -> Result<&str> {
		Ok(&self.topology().current_dc()?.name)
	}

	pub fn dc_for_label(&self, label: u16) -> Option<&topology::Datacenter> {
		self.topology().dc_for_label(label)
	}

	pub fn dc_for_name(&self, name: &str) -> Option<&topology::Datacenter> {
		self.topology().dc_for_name(name)
	}

	pub fn leader_dc(&self) -> Result<&topology::Datacenter> {
		self.topology().leader_dc()
	}

	pub fn is_leader(&self) -> bool {
		self.topology().is_leader()
	}
}
