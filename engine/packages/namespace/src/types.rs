use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use vbare::OwnedVersionedData;

pub const SQLITE_NAMESPACE_CONFIG_VERSION: u16 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SqliteNamespaceConfig {
	pub default_retention_ms: u64,
	pub default_checkpoint_interval_ms: u64,
	pub default_max_checkpoints: u32,
	pub allow_pitr_read: bool,
	pub allow_pitr_destructive: bool,
	pub allow_pitr_admin: bool,
	pub allow_fork: bool,
	pub pitr_max_bytes_per_actor: u64,
	pub pitr_namespace_budget_bytes: u64,
	pub max_retention_ms: u64,
	pub admin_op_rate_per_min: u32,
	pub concurrent_admin_ops: u32,
	pub concurrent_forks_per_src: u32,
}

impl Default for SqliteNamespaceConfig {
	fn default() -> Self {
		Self {
			default_retention_ms: 0,
			default_checkpoint_interval_ms: 3_600_000,
			default_max_checkpoints: 25,
			allow_pitr_read: false,
			allow_pitr_destructive: false,
			allow_pitr_admin: false,
			allow_fork: false,
			pitr_max_bytes_per_actor: 0,
			pitr_namespace_budget_bytes: 0,
			max_retention_ms: 0,
			admin_op_rate_per_min: 10,
			concurrent_admin_ops: 4,
			concurrent_forks_per_src: 2,
		}
	}
}

enum VersionedSqliteNamespaceConfig {
	V1(SqliteNamespaceConfig),
}

impl OwnedVersionedData for VersionedSqliteNamespaceConfig {
	type Latest = SqliteNamespaceConfig;

	fn wrap_latest(latest: Self::Latest) -> Self {
		Self::V1(latest)
	}

	fn unwrap_latest(self) -> Result<Self::Latest> {
		match self {
			Self::V1(data) => Ok(data),
		}
	}

	fn deserialize_version(payload: &[u8], version: u16) -> Result<Self> {
		match version {
			1 => Ok(Self::V1(serde_bare::from_slice(payload)?)),
			_ => bail!("invalid sqlite namespace config version: {version}"),
		}
	}

	fn serialize_version(self, _version: u16) -> Result<Vec<u8>> {
		match self {
			Self::V1(data) => serde_bare::to_vec(&data).map_err(Into::into),
		}
	}
}

pub fn encode_sqlite_namespace_config(config: SqliteNamespaceConfig) -> Result<Vec<u8>> {
	VersionedSqliteNamespaceConfig::wrap_latest(config)
		.serialize_with_embedded_version(SQLITE_NAMESPACE_CONFIG_VERSION)
}

pub fn decode_sqlite_namespace_config(payload: &[u8]) -> Result<SqliteNamespaceConfig> {
	VersionedSqliteNamespaceConfig::deserialize_with_embedded_version(payload)
}
