use anyhow::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;

pub mod api_peer;
pub mod api_public;
pub mod cache;
pub mod clickhouse;
pub mod db;
pub mod guard;
pub mod logs;
pub mod pegboard;
pub mod pegboard_gateway;
pub mod pegboard_tunnel;
pub mod pubsub;
pub mod topology;
pub mod vector;

pub use api_peer::*;
pub use api_public::*;
pub use cache::*;
pub use clickhouse::*;
pub use db::Database;
pub use guard::*;
pub use logs::*;
pub use pegboard::*;
pub use pegboard_gateway::*;
pub use pegboard_tunnel::*;
pub use pubsub::PubSub;
pub use topology::*;
pub use vector::*;

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
// For example:
//
// Default config: { "memory": {} }
// User config: { "file_system": {} }
// Will merge to: { "memory: {}, "file_system": {} }
//
// This is because `Config` is not intelligent enough to know that `Database` is an enum.
//
// By using `Option`, we can manually implement our own default using methods + `LazyLock` (for
// performance).

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Root {
	#[serde(default)]
	pub guard: Option<Guard>,

	#[serde(default)]
	pub api_public: Option<ApiPublic>,

	#[serde(default)]
	pub api_peer: Option<ApiPeer>,

	#[serde(default)]
	pub pegboard: Option<Pegboard>,

	#[serde(default)]
	pub pegboard_gateway: Option<PegboardGateway>,

	#[serde(default)]
	pub pegboard_tunnel: Option<PegboardTunnel>,

	#[serde(default)]
	pub logs: Option<Logs>,

	#[serde(default)]
	pub topology: Option<Topology>,

	#[serde(default, flatten)]
	pub database: Option<Database>,

	#[serde(default, flatten)]
	pub pubsub: Option<PubSub>,

	#[serde(default)]
	pub cache: Option<Cache>,

	#[serde(default)]
	pub clickhouse: Option<ClickHouse>,

	#[serde(default)]
	pub vector_http: Option<VectorHttp>,
}

impl Default for Root {
	fn default() -> Self {
		Root {
			guard: None,
			api_public: None,
			api_peer: None,
			pegboard: None,
			pegboard_gateway: None,
			pegboard_tunnel: None,
			logs: None,
			topology: None,
			database: None,
			pubsub: None,
			cache: None,
			clickhouse: None,
			vector_http: None,
		}
	}
}

impl Root {
	pub fn guard(&self) -> &Guard {
		static DEFAULT: LazyLock<Guard> = LazyLock::new(Guard::default);
		self.guard.as_ref().unwrap_or(&DEFAULT)
	}

	pub fn api_public(&self) -> &ApiPublic {
		static DEFAULT: LazyLock<ApiPublic> = LazyLock::new(ApiPublic::default);
		self.api_public.as_ref().unwrap_or(&DEFAULT)
	}

	pub fn api_peer(&self) -> &ApiPeer {
		static DEFAULT: LazyLock<ApiPeer> = LazyLock::new(ApiPeer::default);
		self.api_peer.as_ref().unwrap_or(&DEFAULT)
	}

	pub fn pegboard(&self) -> &Pegboard {
		static DEFAULT: LazyLock<Pegboard> = LazyLock::new(Pegboard::default);
		self.pegboard.as_ref().unwrap_or(&DEFAULT)
	}

	pub fn pegboard_gateway(&self) -> &PegboardGateway {
		static DEFAULT: LazyLock<PegboardGateway> = LazyLock::new(PegboardGateway::default);
		self.pegboard_gateway.as_ref().unwrap_or(&DEFAULT)
	}

	pub fn pegboard_tunnel(&self) -> &PegboardTunnel {
		static DEFAULT: LazyLock<PegboardTunnel> = LazyLock::new(PegboardTunnel::default);
		self.pegboard_tunnel.as_ref().unwrap_or(&DEFAULT)
	}

	pub fn logs(&self) -> &Logs {
		static DEFAULT: LazyLock<Logs> = LazyLock::new(Logs::default);
		self.logs.as_ref().unwrap_or(&DEFAULT)
	}

	pub fn topology(&self) -> &Topology {
		static DEFAULT: LazyLock<Topology> = LazyLock::new(Topology::default);
		self.topology.as_ref().unwrap_or(&DEFAULT)
	}

	pub fn database(&self) -> &Database {
		static DEFAULT: LazyLock<Database> = LazyLock::new(Database::default);
		self.database.as_ref().unwrap_or(&DEFAULT)
	}

	pub fn pubsub(&self) -> &PubSub {
		static DEFAULT: LazyLock<PubSub> = LazyLock::new(PubSub::default);
		self.pubsub.as_ref().unwrap_or(&DEFAULT)
	}

	pub fn cache(&self) -> &Cache {
		static DEFAULT: LazyLock<Cache> = LazyLock::new(Cache::default);
		self.cache.as_ref().unwrap_or(&DEFAULT)
	}

	pub fn clickhouse(&self) -> Option<&ClickHouse> {
		self.clickhouse.as_ref()
	}

	pub fn vector_http(&self) -> Option<&VectorHttp> {
		self.vector_http.as_ref()
	}

	pub fn validate_and_set_defaults(&mut self) -> Result<()> {
		// TODO: Add back
		//// Set default pubsub to Postgres if configured for database
		//if self.pubsub.is_none()
		//	&& let Some(Database::Postgres(pg)) = &self.database
		//{
		//	self.pubsub = Some(PubSub::PostgresNotify(pubsub::Postgres {
		//		url: pg.url.clone(),
		//	}));
		//}

		Ok(())
	}

	/// Alias of `dc_label`. This is for convenience & clarify when reading code.
	pub fn epoxy_replica_id(&self) -> u64 {
		self.dc_label() as u64
	}

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
