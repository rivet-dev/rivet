use std::path::PathBuf;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub mod postgres;

pub use postgres::{Postgres, PostgresSsl};

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Database {
	Postgres(Postgres),
	FileSystem(FileSystem),
	SlateDb(SlateDb),
}

impl Default for Database {
	fn default() -> Self {
		Self::FileSystem(FileSystem::default())
	}
}

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FileSystem {
	pub path: PathBuf,
}

impl Default for FileSystem {
	fn default() -> Self {
		let default_path = dirs::data_local_dir()
			.map(|dir| dir.join("rivet-engine").join("db"))
			.unwrap_or_else(|| PathBuf::from("./data/db"));

		Self { path: default_path }
	}
}

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SlateDb {
	/// Object-store URL, e.g. "memory:///", "file:///var/lib/rivet/udb", or "s3://bucket/prefix".
	pub object_store_url: String,
	/// Optional database path/prefix inside the object store. Defaults to the path from `object_store_url`.
	pub path: Option<String>,
	/// Optional writer lease tuning for multi-node SlateDB.
	pub lease: Option<SlateDbLease>,
}

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SlateDbLease {
	/// Lease TTL in milliseconds. Defaults to 15 seconds when omitted by the pool mapping.
	pub ttl_ms: Option<u64>,
	/// Heartbeat interval in milliseconds. Defaults to 5 seconds when omitted by the pool mapping.
	pub heartbeat_ms: Option<u64>,
	/// Optional NATS subject advertised by the leader.
	pub nats_subject: Option<String>,
}
