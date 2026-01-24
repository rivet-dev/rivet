use std::path::PathBuf;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::secret::Secret;

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PostgresSsl {
	/// Path to the root certificate file for verifying the server's certificate
	///
	/// Required when using custom certificate authorities (e.g., Supabase)
	/// Equivalent to PostgreSQL's `sslrootcert` parameter
	#[serde(default)]
	pub root_cert_path: Option<PathBuf>,

	/// Path to the client certificate file
	///
	/// Used for client certificate authentication
	/// Equivalent to PostgreSQL's `sslcert` parameter
	#[serde(default)]
	pub client_cert_path: Option<PathBuf>,

	/// Path to the client private key file
	///
	/// Used for client certificate authentication
	/// Equivalent to PostgreSQL's `sslkey` parameter
	#[serde(default)]
	pub client_key_path: Option<PathBuf>,
}

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Postgres {
	/// URL to connect to Postgres with
	///
	/// Supports standard PostgreSQL connection parameters including `sslmode`.
	/// Supported sslmode values: `disable`, `prefer` (default), `require`.
	/// To verify server certificates, use `sslmode=require` with `ssl.root_cert_path`.
	///
	/// Example with sslmode: `postgresql://user:pass@host:5432/db?sslmode=require`
	///
	/// See: https://docs.rs/postgres/0.19.10/postgres/config/struct.Config.html#url
	pub url: Secret<String>,

	/// UNSTABLE: Disable lock timeout customization
	///
	/// When `false` (default), the driver sets `lock_timeout = '0'` and `deadlock_timeout = '10ms'`
	/// during transaction commits to optimize conflict detection.
	///
	/// When `true`, these settings are NOT applied, which may be necessary for some PostgreSQL
	/// configurations or hosted services that don't support these settings.
	///
	/// **This is an unstable feature and may change or be removed in future versions.**
	#[serde(default)]
	pub unstable_disable_lock_customization: bool,

	/// SSL configuration options
	#[serde(default)]
	pub ssl: Option<PostgresSsl>,
}

impl Default for Postgres {
	fn default() -> Self {
		Self {
			url: Secret::new("postgresql://postgres:postgres@localhost:5432/postgres".into()),
			unstable_disable_lock_customization: false,
			ssl: None,
		}
	}
}
