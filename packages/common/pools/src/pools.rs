use std::sync::Arc;

use anyhow::*;
use clickhouse_inserter::ClickHouseInserterHandle;
use rivet_config::Config;
use tokio_util::sync::{CancellationToken, DropGuard};

use crate::{ClickHousePool, Error, UdbPool, UpsPool};

// TODO: Automatically shutdown all pools on drop
pub(crate) struct PoolsInner {
	pub(crate) _guard: DropGuard,
	pub(crate) nats: Option<UpsPool>,
	pub(crate) clickhouse: Option<clickhouse::Client>,
	pub(crate) clickhouse_inserter: Option<ClickHouseInserterHandle>,
	pub(crate) udb: Option<UdbPool>,
	clickhouse_enabled: bool,
}

#[derive(Clone)]
pub struct Pools(Arc<PoolsInner>);

impl Pools {
	#[tracing::instrument(skip(config))]
	pub async fn new(config: Config) -> Result<Pools> {
		// TODO: Choose client name for this service
		let client_name = "rivet".to_string();
		let token = CancellationToken::new();

		let (nats, udb) = tokio::try_join!(
			crate::db::ups::setup(config.clone(), client_name.clone()),
			crate::db::udb::setup(config.clone()),
		)?;
		let clickhouse = crate::db::clickhouse::setup(config.clone())?;

		// Create the ClickHouse inserter if vector is enabled
		let clickhouse_inserter = if let Some(vector_http) = config.vector_http().as_ref() {
			let inserter =
				clickhouse_inserter::create_inserter(&vector_http.host, vector_http.port)?;
			Some(inserter)
		} else {
			None
		};

		let pool = Pools(Arc::new(PoolsInner {
			_guard: token.clone().drop_guard(),
			nats: Some(nats),
			clickhouse,
			clickhouse_inserter,
			udb,
			clickhouse_enabled: config.clickhouse().is_some(),
		}));

		Ok(pool)
	}

	// Only for tests
	#[tracing::instrument(skip(config))]
	pub async fn test(config: Config) -> Result<Pools> {
		// TODO: Choose client name for this service
		let client_name = "rivet".to_string();
		let token = CancellationToken::new();

		let (nats, udb) = tokio::try_join!(
			crate::db::ups::setup(config.clone(), client_name.clone()),
			crate::db::udb::setup(config.clone()),
		)?;

		// Test setup doesn't use ClickHouse inserter
		let pool = Pools(Arc::new(PoolsInner {
			_guard: token.clone().drop_guard(),
			nats: Some(nats),
			clickhouse: None,
			clickhouse_inserter: None,
			udb,
			clickhouse_enabled: config.clickhouse().is_some(),
		}));

		Ok(pool)
	}

	// MARK: Getters
	pub fn nats_option(&self) -> &Option<UpsPool> {
		&self.0.nats
	}

	// MARK: Pool lookups
	pub fn ups(&self) -> Result<UpsPool> {
		self.0.nats.clone().ok_or(Error::MissingNatsPool.into())
	}

	pub fn clickhouse_enabled(&self) -> bool {
		self.0.clickhouse_enabled
	}

	pub fn clickhouse(&self) -> Result<ClickHousePool> {
		ensure!(self.clickhouse_enabled(), "clickhouse disabled");

		self.0.clickhouse.clone().context("missing clickhouse pool")
	}

	pub fn clickhouse_inserter(&self) -> Result<ClickHouseInserterHandle> {
		ensure!(self.clickhouse_enabled(), "clickhouse disabled");

		self.0
			.clickhouse_inserter
			.clone()
			.context("missing clickhouse inserter")
	}

	pub fn udb(&self) -> Result<UdbPool> {
		self.0.udb.clone().ok_or(Error::MissingUdbPool.into())
	}
}
