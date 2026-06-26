use std::{ops::Deref, sync::Arc, time::Duration};
use std::result::Result::{Err, Ok};

use anyhow::*;
use async_trait::async_trait;
use rivet_config::{Config, config};
use tokio::task::JoinHandle;
use universaldb::driver::slatedb::{
	SlateDbForwardingHandler, SlateDbForwardingServerHandle, SlateDbForwardingTransport,
};
use universalpubsub as ups;
use ups::NextOutput;
use uuid::Uuid;

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

#[tracing::instrument(skip(config, pubsub))]
pub async fn setup(config: &Config, pubsub: Option<ups::PubSub>, node_id: Uuid) -> Result<Option<UdbPool>> {
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
		config::Database::SlateDb(cfg) => {
			let lease = cfg.lease.as_ref().map(|lease| {
				let mut config = universaldb::driver::slatedb::SlateDbLeaseConfig::default();
				if let Some(ttl_ms) = lease.ttl_ms {
					config.ttl_ms = ttl_ms;
				}
				if let Some(heartbeat_ms) = lease.heartbeat_ms {
					config.heartbeat_ms = heartbeat_ms;
				}
				config.nats_subject.clone_from(&lease.nats_subject);
				config
			});
			let slatedb_config = universaldb::driver::slatedb::SlateDbConfig {
				object_store_url: cfg.object_store_url.clone(),
				path: cfg.path.clone(),
				lease,
			};
			if slatedb_config.lease.is_some() {
				let pubsub = pubsub.context("SlateDB lease forwarding requires pubsub pool")?;
				let transport = Arc::new(UpsSlateDbForwardingTransport { pubsub });
				universaldb::driver::SlateDbDatabaseDriver::new_managed(
					slatedb_config,
					transport,
					node_id,
				)
				.await?
			} else {
				Arc::new(universaldb::driver::SlateDbDatabaseDriver::new(slatedb_config).await?)
					as universaldb::DatabaseDriverHandle
			}
		}
	};

	tracing::debug!("udb started");

	Ok(Some(UdbPool {
		db: universaldb::Database::new(db_driver),
	}))
}

struct UpsSlateDbForwardingTransport {
	pubsub: ups::PubSub,
}

#[async_trait]
impl SlateDbForwardingTransport for UpsSlateDbForwardingTransport {
	async fn request(
		&self,
		subject: &str,
		payload: &[u8],
		timeout: Duration,
	) -> Result<Option<Vec<u8>>> {
		match self
			.pubsub
			.request_with_timeout(subject, payload, timeout)
			.await?
		{
			NextOutput::Message(message) => Ok(Some(message.payload)),
			NextOutput::Unsubscribed | NextOutput::NoResponders => Ok(None),
		}
	}

	async fn serve(
		&self,
		subject: String,
		handler: Arc<dyn SlateDbForwardingHandler>,
	) -> Result<Box<dyn SlateDbForwardingServerHandle>> {
		let pubsub = self.pubsub.clone();
		let handle = tokio::spawn(async move {
			let mut subscriber = match pubsub.subscribe(&subject).await {
				Ok(subscriber) => subscriber,
				Err(error) => {
					tracing::warn!(?error, subject = %subject, "failed to subscribe SlateDB forwarding server");
					return;
				}
			};

			loop {
				let output = match subscriber.next().await {
					Ok(output) => output,
					Err(error) => {
						tracing::warn!(?error, subject = %subject, "SlateDB forwarding subscriber failed");
						break;
					}
				};

				let message = match output {
					NextOutput::Message(message) => message,
					NextOutput::Unsubscribed => break,
					NextOutput::NoResponders => continue,
				};
				let handler = handler.clone();
				tokio::spawn(async move {
					match handler.handle(message.payload.clone()).await {
						Ok(response) => {
							if let Err(error) = message.reply(&response).await {
								tracing::debug!(?error, "failed to reply to SlateDB forwarding request");
							}
						}
						Err(error) => {
							tracing::warn!(?error, "failed to handle SlateDB forwarding request");
						}
					}
				});
			}
		});

		Ok(Box::new(UpsSlateDbForwardingServerHandle { handle }))
	}
}

struct UpsSlateDbForwardingServerHandle {
	handle: JoinHandle<()>,
}

impl SlateDbForwardingServerHandle for UpsSlateDbForwardingServerHandle {}

impl Drop for UpsSlateDbForwardingServerHandle {
	fn drop(&mut self) {
		self.handle.abort();
	}
}
