use std::{ops::Deref, path::Path, sync::Arc};

use anyhow::Context;
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
pub async fn setup(config: Config) -> Result<Option<FdbPool>, Error> {
	let temp_file = tempfile::NamedTempFile::new().map_err(Error::BuildFdbConnectionFile)?;
	let temp_path = temp_file.path().to_path_buf();

	let Some(fdb_config) = config.server().map_err(Error::Global)?.foundationdb.clone() else {
		return Ok(None);
	};

	let sd = match &fdb_config.addresses {
		rivet_config::config::Addresses::Dynamic { fetch_endpoint } => {
			let sd = ServiceDiscovery::new(fetch_endpoint.clone());

			// Initial fetch
			let servers = sd
				.fetch()
				.await
				.context("failed to fetch services")
				.map_err(Error::BuildFdb)?;
			let joined = servers
				.into_iter()
				.filter_map(|server| server.lan_ip)
				.map(|lan_ip| format!("{lan_ip}:{}", fdb_config.port()))
				.collect::<Vec<_>>()
				.join(",");
			write_connection_file(&fdb_config, &temp_path, &joined)
				.await
				.map_err(Error::BuildFdbConnectionFile)?;

			// Start periodic fetch
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

					write_connection_file(&fdb_config, &temp_path, &joined).await?;

					GlobalResult::Ok(())
				}
			});

			Some(sd)
		}
		rivet_config::config::Addresses::Static(addresses) => {
			let joined = addresses.join(",");
			write_connection_file(&fdb_config, &temp_path, &joined)
				.await
				.map_err(Error::BuildFdbConnectionFile)?;

			None
		}
	};

	// Start network
	fdb_util::init(temp_file.path());

	let fdb_handle = fdb_util::handle(&temp_file.path()).map_err(Error::BuildFdb)?;

	tracing::debug!(config_file_path=%temp_file.path().display(), "fdb started");

	Ok(Some(FdbPool {
		db: Arc::new(fdb_handle),
		_sd: sd,
		_connection_file: Arc::new(temp_file),
	}))
}

async fn write_connection_file(
	fdb_config: &rivet_config::config::FoundationDb,
	temp_path: &Path,
	joined: &str,
) -> Result<(), std::io::Error> {
	let connection = format!(
		"{cluster_description}:{cluster_id}@{joined}",
		cluster_description = fdb_config.cluster_description,
		cluster_id = fdb_config.cluster_id,
	);

	fs::write(temp_path, connection.as_bytes()).await?;

	Ok(())
}
