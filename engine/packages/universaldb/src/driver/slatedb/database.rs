use std::{
	path::Path,
	sync::{
		Arc,
		atomic::{AtomicBool, AtomicI32, AtomicU64, Ordering},
	},
	time::Duration,
};

use anyhow::{Context, Result};
use parking_lot::RwLock;
use slatedb::{
	Db,
	object_store::{ObjectStore, parse_url_opts, path::Path as ObjectStorePath},
};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use url::Url;
use uuid::Uuid;

use crate::{
	RetryableTransaction, Transaction,
	driver::{BoxFut, DatabaseDriver, DatabaseDriverHandle, Erased},
	error::DatabaseError,
	transaction::TXN_TIMEOUT,
	utils::{MaybeCommitted, calculate_tx_retry_backoff},
};

use super::{
	forwarding::{
		SlateDbForwardingClient, SlateDbForwardingDatabaseDriver, SlateDbForwardingServer,
		SlateDbForwardingTransport,
	},
	lease::{LeaseState, SlateDbLease},
	transaction::SlateDbTransactionDriver,
	transaction_conflict_tracker::TransactionConflictTracker,
};

#[derive(Debug, Clone)]
pub struct SlateDbConfig {
	pub object_store_url: String,
	pub path: Option<String>,
	pub lease: Option<SlateDbLeaseConfig>,
}

impl SlateDbConfig {
	pub fn new(object_store_url: String) -> Self {
		Self {
			object_store_url,
			path: None,
			lease: None,
		}
	}
}

#[derive(Debug, Clone)]
pub struct SlateDbLeaseConfig {
	pub ttl_ms: u64,
	pub heartbeat_ms: u64,
	pub nats_subject: Option<String>,
}

impl Default for SlateDbLeaseConfig {
	fn default() -> Self {
		Self {
			ttl_ms: 15_000,
			heartbeat_ms: 5_000,
			nats_subject: None,
		}
	}
}

pub struct SlateDbDatabaseDriver {
	db: Arc<Db>,
	max_retries: AtomicI32,
	txn_conflict_tracker: TransactionConflictTracker,
	commit_mutex: Arc<Mutex<()>>,
	last_applied_version: Arc<AtomicU64>,
	active: Option<Arc<AtomicBool>>,
}

pub(super) fn resolve_object_store(
	config: &SlateDbConfig,
) -> Result<(Arc<dyn ObjectStore>, ObjectStorePath)> {
	let url = Url::parse(&config.object_store_url).context("invalid SlateDB object store URL")?;
	let (store, parsed_path) =
		parse_url_opts(&url, std::env::vars()).context("failed to parse object store URL")?;
	let object_store: Arc<dyn ObjectStore> = Arc::from(store);
	let db_path = match &config.path {
		Some(path) => ObjectStorePath::parse(path).context("invalid SlateDB path")?,
		None => parsed_path,
	};

	Ok((object_store, db_path))
}

impl SlateDbDatabaseDriver {
	pub async fn new(config: SlateDbConfig) -> Result<Self> {
		tracing::info!(object_store_url=%config.object_store_url, "starting slatedb driver");

		let (object_store, db_path) = resolve_object_store(&config)?;
		Self::open_resolved(object_store, db_path, None).await
	}

	async fn open_resolved(
		object_store: Arc<dyn ObjectStore>,
		db_path: ObjectStorePath,
		active: Option<Arc<AtomicBool>>,
	) -> Result<Self> {
		let db = Db::open(db_path, object_store)
			.await
			.context("failed to open SlateDB")?;

		Ok(SlateDbDatabaseDriver {
			db: Arc::new(db),
			max_retries: AtomicI32::new(100),
			txn_conflict_tracker: TransactionConflictTracker::new(),
			commit_mutex: Arc::new(Mutex::new(())),
			last_applied_version: Arc::new(AtomicU64::new(0)),
			active,
		})
	}

	pub async fn new_managed(
		config: SlateDbConfig,
		transport: Arc<dyn SlateDbForwardingTransport>,
		node_id: Uuid,
	) -> Result<DatabaseDriverHandle> {
		let (object_store, db_path) = resolve_object_store(&config)?;
		Self::new_managed_with_object_store(config, object_store, db_path, transport, node_id).await
	}

	pub async fn new_managed_with_object_store(
		config: SlateDbConfig,
		object_store: Arc<dyn ObjectStore>,
		db_path: ObjectStorePath,
		transport: Arc<dyn SlateDbForwardingTransport>,
		node_id: Uuid,
	) -> Result<DatabaseDriverHandle> {
		let lease_config = config
			.lease
			.clone()
			.context("managed SlateDB driver requires lease config")?;
		let subject = forwarding_subject(&lease_config, &db_path);
		let lease = SlateDbLease::new(object_store.clone(), db_path.clone(), lease_config.clone());
		let forwarding_client =
			SlateDbForwardingClient::new(transport.clone(), Some(lease.clone()), subject.clone());
		let forwarding_driver = Arc::new(SlateDbForwardingDatabaseDriver::new(forwarding_client));
		let active = Arc::new(AtomicBool::new(false));
		let local = Arc::new(RwLock::new(None));
		let heartbeat_ms = lease_config.heartbeat_ms;

		let managed = Arc::new(SlateDbManagedDatabaseDriver {
			object_store: object_store.clone(),
			db_path: db_path.clone(),
			transport: transport.clone(),
			lease,
			lease_config,
			subject,
			node_id,
			active: active.clone(),
			// This is a forced-sync slot because DatabaseDriver::create_txn is synchronous.
			local: local.clone(),
			forwarding_driver,
			lease_task: RwLock::new(None),
		});

		managed.try_become_leader().await?;
		let task_driver = Arc::downgrade(&managed);
		let task = tokio::spawn(async move {
			let mut interval = tokio::time::interval(Duration::from_millis(heartbeat_ms.max(1)));
			interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

			loop {
				interval.tick().await;
				let Some(driver) = task_driver.upgrade() else {
					break;
				};
				if !driver.active.load(Ordering::Acquire)
					&& let Err(error) = driver.try_become_leader().await
				{
					tracing::warn!(?error, "failed to acquire SlateDB leader lease");
				}
			}
		});
		*managed.lease_task.write() = Some(task);

		Ok(managed as DatabaseDriverHandle)
	}

	pub async fn close(&self) -> Result<()> {
		self.db.close().await.context("failed to close SlateDB")?;
		Ok(())
	}

	pub(super) fn is_active(&self) -> bool {
		self.active
			.as_ref()
			.is_none_or(|active| active.load(Ordering::Acquire))
	}
}

impl DatabaseDriver for SlateDbDatabaseDriver {
	fn create_txn(&self) -> Result<Transaction> {
		if !self.is_active() {
			return Err(DatabaseError::NotCommitted.into());
		}

		Ok(Transaction::new(Arc::new(SlateDbTransactionDriver::new(
			self.db.clone(),
			self.txn_conflict_tracker.clone(),
			self.commit_mutex.clone(),
			self.last_applied_version.clone(),
			self.active.clone(),
		))))
	}

	fn run<'a>(
		&'a self,
		closure: Box<dyn Fn(RetryableTransaction) -> BoxFut<'a, Result<Erased>> + Send + Sync + 'a>,
	) -> BoxFut<'a, Result<Erased>> {
		Box::pin(async move {
			let mut maybe_committed = MaybeCommitted(false);
			let max_retries = self.max_retries.load(Ordering::SeqCst);

			for attempt in 0..max_retries {
				let tx = self.create_txn()?;
				let mut retryable = RetryableTransaction::new(tx);
				retryable.maybe_committed = maybe_committed;

				let error =
					match tokio::time::timeout(TXN_TIMEOUT, closure(retryable.clone())).await {
						Ok(Ok(res)) => match retryable.inner.driver.commit_ref().await {
							Ok(_) => return Ok(res),
							Err(e) => e,
						},
						Ok(Err(e)) => e,
						Err(_) => anyhow::Error::from(DatabaseError::TransactionTooOld),
					};

				let chain = error
					.chain()
					.find_map(|x| x.downcast_ref::<DatabaseError>());

				if let Some(db_error) = chain {
					if db_error.is_retryable() {
						if db_error.is_maybe_committed() {
							maybe_committed = MaybeCommitted(true);
						}

						let backoff_ms = calculate_tx_retry_backoff(attempt as usize);
						tokio::time::sleep(tokio::time::Duration::from_millis(backoff_ms)).await;
						continue;
					}
				}

				return Err(error);
			}

			Err(DatabaseError::MaxRetriesReached.into())
		})
	}

	fn txn_retry_limit(&self, limit: i32) -> Result<()> {
		self.max_retries.store(limit, Ordering::SeqCst);
		Ok(())
	}

	fn checkpoint(&self, _path: &Path) -> Result<()> {
		anyhow::bail!("checkpoint not supported by SlateDB driver")
	}
}

pub struct SlateDbManagedDatabaseDriver {
	object_store: Arc<dyn ObjectStore>,
	db_path: ObjectStorePath,
	transport: Arc<dyn SlateDbForwardingTransport>,
	lease: SlateDbLease,
	lease_config: SlateDbLeaseConfig,
	subject: String,
	node_id: Uuid,
	active: Arc<AtomicBool>,
	local: Arc<RwLock<Option<Arc<SlateDbDatabaseDriver>>>>,
	forwarding_driver: Arc<SlateDbForwardingDatabaseDriver>,
	lease_task: RwLock<Option<JoinHandle<()>>>,
}

impl SlateDbManagedDatabaseDriver {
	async fn try_become_leader(&self) -> Result<bool> {
		if self.active.load(Ordering::Acquire) {
			return Ok(true);
		}

		let Some(state) = self
			.lease
			.try_acquire(
				self.node_id.to_string(),
				self.subject.clone(),
				now_ms(),
			)
			.await?
		else {
			return Ok(false);
		};

		self.install_leader(state).await?;
		Ok(true)
	}

	async fn install_leader(&self, initial_state: LeaseState) -> Result<()> {
		let local_active = self.active.clone();
		let local_driver = Arc::new(
			SlateDbDatabaseDriver::open_resolved(
				self.object_store.clone(),
				self.db_path.clone(),
				Some(local_active.clone()),
			)
			.await?,
		);

		let server = SlateDbForwardingServer::spawn(
			self.transport.clone(),
			self.subject.clone(),
			local_driver.clone(),
		)
		.await?;

		*self.local.write() = Some(local_driver);
		local_active.store(true, Ordering::Release);
		let renew_driver = self.clone_for_task();
		tokio::spawn(async move {
			renew_driver.renew_loop(initial_state, server).await;
		});

		tracing::info!(
			leader_id = %self.node_id,
			subject = %self.subject,
			"acquired SlateDB leader lease"
		);

		Ok(())
	}

	fn clone_for_task(&self) -> SlateDbManagedTask {
		SlateDbManagedTask {
			lease: self.lease.clone(),
			lease_config: self.lease_config.clone(),
			active: self.active.clone(),
			local: self.local.clone(),
			node_id: self.node_id,
		}
	}

}

impl Drop for SlateDbManagedDatabaseDriver {
	fn drop(&mut self) {
		if let Some(task) = self.lease_task.write().take() {
			task.abort();
		}
	}
}

impl DatabaseDriver for SlateDbManagedDatabaseDriver {
	fn create_txn(&self) -> Result<Transaction> {
		if self.active.load(Ordering::Acquire)
			&& let Some(local) = self.local.read().clone()
		{
			return local.create_txn();
		}

		self.forwarding_driver.create_txn()
	}

	fn run<'a>(
		&'a self,
		closure: Box<dyn Fn(RetryableTransaction) -> BoxFut<'a, Result<Erased>> + Send + Sync + 'a>,
	) -> BoxFut<'a, Result<Erased>> {
		Box::pin(async move {
			let mut maybe_committed = MaybeCommitted(false);
			let max_retries = self.forwarding_driver.max_retries.load(Ordering::SeqCst);

			for attempt in 0..max_retries {
				let tx = self.create_txn()?;
				let mut retryable = RetryableTransaction::new(tx);
				retryable.maybe_committed = maybe_committed;

				let error =
					match tokio::time::timeout(TXN_TIMEOUT, closure(retryable.clone())).await {
						Ok(Ok(res)) => match retryable.inner.driver.commit_ref().await {
							Ok(_) => return Ok(res),
							Err(e) => e,
						},
						Ok(Err(e)) => e,
						Err(_) => anyhow::Error::from(DatabaseError::TransactionTooOld),
					};

				let chain = error
					.chain()
					.find_map(|x| x.downcast_ref::<DatabaseError>());

				if let Some(db_error) = chain
					&& db_error.is_retryable()
				{
					if db_error.is_maybe_committed() {
						maybe_committed = MaybeCommitted(true);
					}

					let backoff_ms = calculate_tx_retry_backoff(attempt as usize);
					tokio::time::sleep(tokio::time::Duration::from_millis(backoff_ms)).await;
					continue;
				}

				return Err(error);
			}

			Err(DatabaseError::MaxRetriesReached.into())
		})
	}

	fn txn_retry_limit(&self, limit: i32) -> Result<()> {
		self.forwarding_driver.txn_retry_limit(limit)?;
		if let Some(local) = self.local.read().as_ref() {
			local.txn_retry_limit(limit)?;
		}
		Ok(())
	}

	fn checkpoint(&self, _path: &Path) -> Result<()> {
		anyhow::bail!("checkpoint not supported by SlateDB driver")
	}
}

struct SlateDbManagedTask {
	lease: SlateDbLease,
	lease_config: SlateDbLeaseConfig,
	active: Arc<AtomicBool>,
	local: Arc<RwLock<Option<Arc<SlateDbDatabaseDriver>>>>,
	node_id: Uuid,
}

impl SlateDbManagedTask {
	async fn renew_loop(self, mut state: LeaseState, _server: SlateDbForwardingServer) {
		let mut interval = tokio::time::interval(Duration::from_millis(
			self.lease_config.heartbeat_ms.max(1),
		));
		interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

		loop {
			interval.tick().await;
			match self.lease.renew(&state, now_ms()).await {
				Ok(Some(new_state)) => {
					state = new_state;
				}
				Ok(None) => {
					tracing::warn!(leader_id = %self.node_id, "lost SlateDB leader lease");
					self.active.store(false, Ordering::Release);
					*self.local.write() = None;
					break;
				}
				Err(error) => {
					tracing::warn!(?error, leader_id = %self.node_id, "failed to renew SlateDB leader lease");
					self.active.store(false, Ordering::Release);
					*self.local.write() = None;
					break;
				}
			}
		}
	}
}

fn forwarding_subject(config: &SlateDbLeaseConfig, db_path: &ObjectStorePath) -> String {
	config.nats_subject.clone().unwrap_or_else(|| {
		let path = db_path.to_string();
		format!("udb.slatedb.{}", hex::encode(path.as_bytes()))
	})
}

fn now_ms() -> u64 {
	std::time::SystemTime::now()
		.duration_since(std::time::UNIX_EPOCH)
		.map(|duration| duration.as_millis().try_into().unwrap_or(u64::MAX))
		.unwrap_or(0)
}
