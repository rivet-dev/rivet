use std::sync::Arc;

use anyhow::*;
use rivet_config::Config;
use tokio_util::sync::{CancellationToken, DropGuard};

use crate::{ClickHousePool, Error, UdbPool, UpsPool};

// TODO: Automatically shutdown all pools on drop
pub(crate) struct PoolsInner {
	pub(crate) _guard: DropGuard,
	pub(crate) ups: Option<UpsPool>,
	pub(crate) clickhouse: Option<clickhouse::Client>,
	pub(crate) udb: Option<UdbPool>,
	pub(crate) kafka_producer: Option<rdkafka::producer::FutureProducer>,
}

#[derive(Clone)]
pub struct Pools(Arc<PoolsInner>);

impl Pools {
	#[tracing::instrument(skip(config))]
	pub async fn new(config: Config) -> Result<Pools> {
		// TODO: Choose client name for this service
		let client_name = "rivet";
		let token = CancellationToken::new();

		let (ups, udb) = tokio::try_join!(
			crate::db::ups::setup(&config, client_name),
			crate::db::udb::setup(&config),
		)?;
		let clickhouse = crate::db::clickhouse::setup(&config)?;

		let kafka_producer = crate::db::kafka::setup(&config)?;

		let pool = Pools(Arc::new(PoolsInner {
			_guard: token.clone().drop_guard(),
			ups: Some(ups),
			clickhouse,
			udb,
			kafka_producer,
		}));

		Ok(pool)
	}

	// Only for tests
	#[tracing::instrument(skip(config))]
	pub async fn test(config: Config) -> Result<Pools> {
		// TODO: Choose client name for this service
		let client_name = "rivet";
		let token = CancellationToken::new();

		let (ups, udb) = tokio::try_join!(
			crate::db::ups::setup(&config, client_name),
			crate::db::udb::setup(&config),
		)?;

		let pool = Pools(Arc::new(PoolsInner {
			_guard: token.clone().drop_guard(),
			ups: Some(ups),
			clickhouse: None,
			udb,
			kafka_producer: None,
		}));

		Ok(pool)
	}

	// MARK: Getters
	pub fn ups_option(&self) -> &Option<UpsPool> {
		&self.0.ups
	}

	// MARK: Pool lookups
	pub fn ups(&self) -> Result<UpsPool> {
		self.0.ups.clone().ok_or(Error::MissingUpsPool.into())
	}

	pub fn clickhouse(&self) -> Result<ClickHousePool> {
		self.0.clickhouse.clone().context("missing clickhouse pool")
	}

	pub fn udb(&self) -> Result<UdbPool> {
		self.0.udb.clone().ok_or(Error::MissingUdbPool.into())
	}

	pub fn kafka(&self) -> Result<rdkafka::producer::FutureProducer> {
		self.0
			.kafka_producer
			.clone()
			.ok_or(Error::MissingKafkaPool.into())
	}
}
