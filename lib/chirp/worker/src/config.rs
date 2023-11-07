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
		let worker_kind_name = env::var("CHIRP_WORKER_KIND")
			.map_err(|_| ManagerError::MissingEnvVar("CHIRP_WORKER_KIND".into()))?;
		let worker_kind = match worker_kind_name.as_str() {
			"rpc" => WorkerKind::Rpc {
				group: env::var("CHIRP_WORKER_RPC_GROUP")
					.map_err(|_| ManagerError::MissingEnvVar("CHIRP_WORKER_RPC_GROUP".into()))?,
			},
			"consumer" => WorkerKind::Consumer {
				topic: topic.into(),
				group: env::var("CHIRP_WORKER_CONSUMER_GROUP").map_err(|_| {
					ManagerError::MissingEnvVar("CHIRP_WORKER_CONSUMER_GROUP".into())
				})?,
			},
			_ => {
				return Err(ManagerError::InvalidEnvVar {
					key: "CHIRP_WORKER_KIND".to_owned(),
					message: "unknown worker kind".to_owned(),
				})
			}
		};

		Ok(Self {
			service_name: env::var("CHIRP_SERVICE_NAME")
				.map_err(|_| ManagerError::MissingEnvVar("CHIRP_SERVICE_NAME".into()))?,
			worker_instance: env::var("CHIRP_WORKER_INSTANCE")
				.map_err(|_| ManagerError::MissingEnvVar("CHIRP_WORKER_INSTANCE".into()))?,
			worker_source_hash: env::var("RIVET_SOURCE_HASH")
				.map_err(|_| ManagerError::MissingEnvVar("RIVET_SOURCE_HASH".into()))?,
			worker_kind,
		})
	}
}
