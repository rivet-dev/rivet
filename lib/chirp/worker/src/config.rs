use std::{env, fmt::Debug};

use crate::error::ManagerError;

#[derive(Clone, Debug)]
pub struct Config {
	/// The name of the service running workers.
	pub service_name: String,

	/// Unique name to the instance of this process.
	pub worker_instance: String,

	/// Hash of the source that this Chirp worker is running. Useful for cache
	/// busting.
	pub worker_source_hash: String,

	pub worker_kind: WorkerKind,
}

#[derive(Clone, Debug)]
pub enum WorkerKind {
	Rpc { group: String },
	Consumer { topic: String, group: String },
}

impl Config {
	pub fn from_env(topic: &str) -> Result<Self, ManagerError> {
		let worker_kind = WorkerKind::Consumer {
			topic: topic.into(),
			group: rivet_env::service_name().to_string(),
		};

		Ok(Self {
			service_name: rivet_env::service_name().to_string(),
			worker_instance: rivet_env::service_instance().to_string(),
			worker_source_hash: rivet_env::source_hash().to_string(),
			worker_kind,
		})
	}
}
