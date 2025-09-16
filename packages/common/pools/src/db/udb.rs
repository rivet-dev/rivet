use std::{ops::Deref, sync::Arc};

use anyhow::*;
use rivet_config::{Config, config};

#[derive(Clone)]
pub struct UdbPool {
	db: universaldb::Database,
}

impl Deref for UdbPool {
	type Target = universaldb::Database;

	fn deref(&self) -> &Self::Target {
		&self.db
	}
}

#[tracing::instrument(skip(config))]
pub async fn setup(config: Config) -> Result<Option<UdbPool>> {
	let db_driver = match config.database() {
		config::Database::Postgres(pg) => {
			Arc::new(universaldb::driver::PostgresDatabaseDriver::new(pg.url.read().clone()).await?)
				as universaldb::DatabaseDriverHandle
		}
		config::Database::FileSystem(fs) => {
			Arc::new(universaldb::driver::RocksDbDatabaseDriver::new(fs.path.clone()).await?)
				as universaldb::DatabaseDriverHandle
		}
	};

	tracing::debug!("udb started");

	Ok(Some(UdbPool {
		db: universaldb::Database::new(db_driver),
	}))
}
