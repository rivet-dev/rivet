use std::{ops::Deref, sync::Arc};

use foundationdb as fdb;
use global_error::GlobalResult;
use rivet_config::Config;
use service_discovery::ServiceDiscovery;
use tokio::fs;

use crate::Error;

#[derive(Clone)]
pub struct FdbPool {
	db: Arc<fdb::Database>,
	_sd: Option<Arc<ServiceDiscovery>>,
	// Prevent dropping temp file
	_connection_file: Arc<tempfile::NamedTempFile>,
}

impl Deref for FdbPool {
	type Target = Arc<fdb::Database>;

	fn deref(&self) -> &Self::Target {
		&self.db
	}
}

#[tracing::instrument(skip(config))]
pub async fn setup(config: Config) -> Result<FdbPool, Error> {
	let temp_file = tempfile::NamedTempFile::new().map_err(Error::BuildFdbConnectionFile)?;
	let temp_path = temp_file.path().to_path_buf();

	let fdb_config = config.server().map_err(Error::Global)?.foundationdb.clone();

	let sd = match &fdb_config.addresses {
		rivet_config::config::Addresses::Dynamic { fetch_endpoint } => {
			let sd = ServiceDiscovery::new(fetch_endpoint.clone());

			sd.start(move |servers| {
				let temp_path = temp_path.clone();
				let fdb_config = fdb_config.clone();
				async move {
					let joined = servers
						.into_iter()
						.filter_map(|server| server.lan_ip)
						.map(|lan_ip| format!("{lan_ip}:{}", fdb_config.port()))
						.collect::<Vec<_>>()
						.join(",");
					let connection = format!(
						"{cluster_description}:{cluster_id}@{joined}",
						cluster_description = fdb_config.cluster_description,
						cluster_id = fdb_config.cluster_id,
					);

					fs::write(temp_path, connection.as_bytes()).await?;

					GlobalResult::Ok(())
				}
			});

			Some(sd)
		}
		rivet_config::config::Addresses::Static(addresses) => {
			let joined = addresses.join(",");
			let connection = format!(
				"{cluster_description}:{cluster_id}@{joined}",
				cluster_description = fdb_config.cluster_description,
				cluster_id = fdb_config.cluster_id,
			);

			fs::write(temp_path, connection.as_bytes())
				.await
				.map_err(Error::BuildFdbConnectionFile)?;

			None
		}
	};

	// Start network
	fdb_util::init(temp_file.path());

	let fdb_handle = fdb_util::handle(&temp_file.path()).map_err(Error::BuildFdb)?;

	tracing::debug!(config_file_path=%temp_file.path().display(), "fdb started");

	Ok(FdbPool {
		db: Arc::new(fdb_handle),
		_sd: sd,
		_connection_file: Arc::new(temp_file),
	})
}
