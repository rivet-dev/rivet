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
