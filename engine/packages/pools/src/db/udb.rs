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
pub async fn setup(config: &Config) -> Result<Option<UdbPool>> {
	let db_driver = match config.database() {
		config::Database::Postgres(pg) => {
			let postgres_config = universaldb::driver::postgres::PostgresConfig {
				connection_string: pg.url.read().clone(),
				ssl_config: pg.ssl.as_ref().map(|ssl| {
					universaldb::driver::postgres::PostgresSslConfig {
						ssl_root_cert_path: ssl.root_cert_path.clone(),
						ssl_client_cert_path: ssl.client_cert_path.clone(),
						ssl_client_key_path: ssl.client_key_path.clone(),
					}
				}),
			};

			Arc::new(
				universaldb::driver::PostgresDatabaseDriver::new_with_config(postgres_config)
					.await?,
			) as universaldb::DatabaseDriverHandle
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
