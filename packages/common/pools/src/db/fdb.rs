use std::{ops::Deref, sync::Arc};

use foundationdb as fdb;
use rivet_config::Config;
use tokio::fs;

use crate::Error;

#[derive(Clone)]
pub struct FdbPool {
	db: Arc<fdb::Database>,
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
	let connection = &config.server().map_err(Error::Global)?.fdb.connection;

	let temp_file = tempfile::NamedTempFile::new().map_err(Error::BuildFdbConnectionFile)?;
	fs::write(temp_file.path(), connection.as_bytes())
		.await
		.map_err(Error::BuildFdbConnectionFile)?;

	// Start network
	fdb_util::init(temp_file.path());

	let fdb_handle = fdb_util::handle(&temp_file.path()).map_err(Error::BuildFdb)?;

	tracing::debug!(?connection, "fdb connected");

	Ok(FdbPool {
		db: Arc::new(fdb_handle),
		_connection_file: Arc::new(temp_file),
	})
}
