use std::path::PathBuf;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::secret::Secret;

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Database {
	Postgres(Postgres),
	FileSystem(FileSystem),
}

impl Default for Database {
	fn default() -> Self {
		Self::FileSystem(FileSystem::default())
	}
}

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
pub struct Postgres {
	/// URL to connect to Postgres with
	///
	/// See: https://docs.rs/postgres/0.19.10/postgres/config/struct.Config.html#url
	pub url: Secret<String>,
}

impl Default for Postgres {
	fn default() -> Self {
		Self {
			url: Secret::new("postgresql://postgres:postgres@127.0.0.1:5432/postgres".into()),
		}
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
