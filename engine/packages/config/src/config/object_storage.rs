use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::secret::Secret;

#[derive(Debug, Serialize, Deserialize, Clone, Default, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ObjectStorageConfig {
	#[serde(default)]
	pub primary: ObjectStoreBackend,
	/// Milliseconds the first attempt of a small GET or HEAD may wait for response headers
	/// before the client cuts it off and retries it with its normal timeouts. A request on a
	/// stalled connection otherwise waits the full 30 s client timeout. Default 250, set
	/// from the load cluster's Azure account, where about 0.02% of small requests took
	/// longer; 0 disables it. Response bodies are never cut off.
	#[serde(default)]
	pub read_first_attempt_ms: Option<u64>,
	/// Milliseconds the first attempt of a small conditional PUT (a lease, heartbeat or
	/// manifest) may wait for response headers, before the upload time its body needs at
	/// `write_first_attempt_bytes_per_second`. Unset by default, which disables it. Healthy
	/// conditional PUT latency is calibrated only on Azure; a healthy GCS PUT can take over
	/// 250 ms. GCS also allows about one write per second to one object, so a retry shortly
	/// after a cut-off on the same lease or heartbeat object risks a 429, an unknown outcome
	/// and an extra readback. Set it only from measured conditional PUT latency of the
	/// backend.
	#[serde(default)]
	pub write_first_attempt_ms: Option<u64>,
	/// The upload rate a conditional PUT's first attempt may fall to before it is cut off,
	/// which extends `write_first_attempt_ms` by the body's size. Default 8 MiB/s.
	#[serde(default)]
	pub write_first_attempt_bytes_per_second: Option<u64>,
}

impl ObjectStorageConfig {
	/// The read first-attempt deadline, or `None` when disabled.
	pub fn read_first_attempt_ms(&self) -> Option<u64> {
		match self.read_first_attempt_ms.unwrap_or(250) {
			0 => None,
			ms => Some(ms),
		}
	}

	/// The conditional PUT first-attempt deadline, or `None` when disabled.
	pub fn write_first_attempt_ms(&self) -> Option<u64> {
		self.write_first_attempt_ms.filter(|ms| *ms > 0)
	}

	pub fn write_first_attempt_bytes_per_second(&self) -> u64 {
		self.write_first_attempt_bytes_per_second
			.filter(|rate| *rate > 0)
			.unwrap_or(8 * 1024 * 1024)
	}
}

#[derive(Debug, Serialize, Deserialize, Clone, Default, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum ObjectStoreBackend {
	#[default]
	InMemory,
	/// A local directory, for a single engine process that keeps its data across restarts.
	FileSystem(FileSystemObjectStoreConfig),
	S3(S3ObjectStoreConfig),
	Azure(AzureObjectStoreConfig),
	Gcs(GcsObjectStoreConfig),
}

/// A local directory used by one engine process at a time. Conditional writes are atomic
/// only among requests of that process.
#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FileSystemObjectStoreConfig {
	/// Created when it does not exist.
	pub path: std::path::PathBuf,
}

/// Google Cloud Storage; defaults to the adapter's application default credentials.
#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GcsObjectStoreConfig {
	pub bucket: String,
	#[serde(default)]
	pub prefix: Option<String>,
	/// Optional JSON service account credential override.
	#[serde(default)]
	pub service_account_key: Option<Secret<String>>,
}

/// Azure Blob storage used by SlateDB, topology, and cluster configuration.
#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AzureObjectStoreConfig {
	pub account: String,
	pub container: String,
	#[serde(default)]
	pub prefix: Option<String>,
	/// Optional override; otherwise use the adapter's environment/managed identity credentials.
	#[serde(default)]
	pub access_key: Option<Secret<String>>,
}

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct S3ObjectStoreConfig {
	pub bucket: String,
	#[serde(default)]
	pub prefix: Option<String>,
	#[serde(default)]
	pub region: Option<String>,
	#[serde(default)]
	pub endpoint: Option<String>,
	#[serde(default)]
	pub access_key_id: Option<String>,
	#[serde(default)]
	pub secret_access_key: Option<Secret<String>>,
}
