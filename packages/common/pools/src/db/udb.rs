use std::{ops::Deref, sync::Arc};

use anyhow::*;
use rivet_config::{Config, config};
use universaldb as udb;

#[derive(Clone)]
pub struct UdbPool {
	db: udb::Database,
}

impl Deref for UdbPool {
	type Target = udb::Database;

	fn deref(&self) -> &Self::Target {
		&self.db
	}
}

#[tracing::instrument(skip(config))]
pub async fn setup(config: Config) -> Result<Option<UdbPool>> {
	let db_driver = match config.database() {
		config::Database::Postgres(pg) => {
			Arc::new(udb::driver::PostgresDatabaseDriver::new(pg.url.read().clone()).await?)
				as udb::DatabaseDriverHandle
		}
		config::Database::FileSystem(fs) => {
			Arc::new(udb::driver::RocksDbDatabaseDriver::new(fs.path.clone()).await?)
				as udb::DatabaseDriverHandle
		}
	};

	tracing::debug!("udb started");

	Ok(Some(UdbPool {
		db: udb::Database::new(db_driver),
	}))
}
