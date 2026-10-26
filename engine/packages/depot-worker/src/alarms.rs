//! Portable alarm operations and limits, with native callback and I/O execution.

#[cfg(not(target_arch = "wasm32"))]
use crate::filesystem::store::FilesystemStore;
#[cfg(not(target_arch = "wasm32"))]
use anyhow::Context;
use anyhow::{Result, ensure};
use depot_types::{VolumeKey, VolumeLogKey};
use futures_util::future::BoxFuture;
use rivet_data::generated::{
	depot_alarm_v1::{AlarmCurrent, AlarmRetryRecord, AlarmTimeRecord},
	shared_depot_alarms_v1::{AlarmOwner, AlarmSummary},
};
#[cfg(not(target_arch = "wasm32"))]
use rivet_metrics::{GaugeGuardExt, IntGaugeGuard};
use serde::{Deserialize, Serialize};
#[cfg(not(target_arch = "wasm32"))]
use std::sync::{
	Arc,
	atomic::{AtomicBool, Ordering},
};
#[cfg(not(target_arch = "wasm32"))]
use tokio::sync::Semaphore;

/// Component-owned bounds; production startup resolves optional configuration overrides.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Limits {
	pub max_candidates: u32,
	pub max_operations: u32,
	pub max_callbacks: u32,
	pub page_size: u32,
	pub handler_timeout_ms: u64,
	pub io_timeout_ms: u64,
	pub retry_min_ms: u64,
	pub retry_max_ms: u64,
}

impl Default for Limits {
	fn default() -> Self {
		Self {
			max_candidates: 1024,
			max_operations: 16,
			max_callbacks: 128,
			page_size: 128,
			handler_timeout_ms: 30_000,
			io_timeout_ms: 5000,
			retry_min_ms: 500,
			retry_max_ms: 30_000,
		}
	}
}

impl Limits {
	/// Reject invalid bounds before a worker starts serving alarms.
	pub fn validate(&self) -> Result<()> {
		ensure!(
			self.max_candidates > 0
				&& self.max_operations > 0
				&& self.max_callbacks > 0
				&& (1..=128).contains(&self.page_size)
				&& self.handler_timeout_ms > 0
				&& self.io_timeout_ms > 0
				&& self.retry_min_ms > 0
				&& self.retry_max_ms >= self.retry_min_ms
				&& self.retry_max_ms <= i64::MAX as u64,
			"invalid depot.alarms limits"
		);
		Ok(())
	}
}

#[cfg(not(target_arch = "wasm32"))]
impl From<&rivet_config::config::depot::AlarmConfig> for Limits {
	fn from(config: &rivet_config::config::depot::AlarmConfig) -> Self {
		let defaults = Self::default();
		Self {
			max_candidates: config.max_candidates.unwrap_or(defaults.max_candidates),
			max_operations: config.max_operations.unwrap_or(defaults.max_operations),
			max_callbacks: config.max_callbacks.unwrap_or(defaults.max_callbacks),
			page_size: config.page_size.unwrap_or(defaults.page_size),
			handler_timeout_ms: config
				.handler_timeout_ms
				.unwrap_or(defaults.handler_timeout_ms),
			io_timeout_ms: config.io_timeout_ms.unwrap_or(defaults.io_timeout_ms),
			retry_min_ms: config.retry_min_ms.unwrap_or(defaults.retry_min_ms),
			retry_max_ms: config.retry_max_ms.unwrap_or(defaults.retry_max_ms),
		}
	}
}

#[derive(Clone, Debug)]
pub struct AlarmContext {
	pub revision: rivet_data::generated::shared_depot_alarms_v1::AlarmRevision,
	pub deadline_unix_ms: i64,
	pub owner: AlarmOwner,
}

pub trait DepotDriver: Send + Sync + 'static {
	/// May be dropped on timeout, replacement, shutdown, or tablet authority loss.
	fn on_alarm(&self, key: VolumeKey, alarm: AlarmContext) -> BoxFuture<'_, Result<()>>;
}

/// One worker owns the callback pool shared by all its tablet maintainers.
#[derive(Clone)]
#[cfg(not(target_arch = "wasm32"))]
pub struct AlarmDriver {
	driver: Arc<dyn DepotDriver>,
	pool: Arc<CallbackPool>,
}
#[cfg(not(target_arch = "wasm32"))]
struct CallbackPool {
	permits: Semaphore,
	warned: AtomicBool,
	capacity: u32,
	_capacity: IntGaugeGuard,
}
#[cfg(not(target_arch = "wasm32"))]
impl AlarmDriver {
	pub fn new(driver: Arc<dyn DepotDriver>, capacity: u32) -> Result<Self> {
		ensure!(
			capacity > 0 && capacity as usize <= Semaphore::MAX_PERMITS,
			"depot.alarms.max_callbacks is outside the semaphore capacity"
		);
		Ok(Self {
			driver,
			pool: Arc::new(CallbackPool {
				permits: Semaphore::new(capacity as usize),
				warned: AtomicBool::new(false),
				capacity,
				_capacity: crate::metrics::ALARM_CALLBACK_CAPACITY.add_guard(i64::from(capacity)),
			}),
		})
	}
}

/// Immutable work retained in the maintainer's operation registry across uncertain retries.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Request {
	Page {
		after: Option<bytes::Bytes>,
		epoch: u64,
		limit: usize,
	},
	Read {
		key: VolumeLogKey,
	},
	Call {
		key: VolumeLogKey,
		summary: AlarmSummary,
		attempt: u32,
	},
	Retry {
		key: VolumeLogKey,
		summary: AlarmSummary,
		record: AlarmRetryRecord,
	},
}
impl Request {
	pub(crate) fn key(&self) -> Option<&VolumeLogKey> {
		match self {
			Self::Page { .. } => None,
			Self::Read { key } | Self::Call { key, .. } | Self::Retry { key, .. } => Some(key),
		}
	}
	/// Projection reads and updates share admission with filesystem replay and retirement.
	pub(crate) fn uses_store(&self) -> bool {
		matches!(self, Self::Read { .. } | Self::Retry { .. })
	}
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Response {
	Page {
		rows: Vec<(bytes::Bytes, AlarmTimeRecord)>,
		done: bool,
	},
	Read {
		current: Option<AlarmCurrent>,
	},
	Called {
		busy: bool,
		randomness: u32,
	},
	Updated {
		current: Option<AlarmCurrent>,
	},
}

/// Executes one operation. Deadlines, retry policy and ownership decisions stay in the core.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) async fn execute(
	fs: &FilesystemStore,
	driver: &AlarmDriver,
	owner: AlarmOwner,
	request: Request,
	randomness: u32,
) -> Result<Response> {
	match request {
		Request::Page { after, limit, .. } => {
			let snapshot = fs.db.snapshot().await?;
			let (rows, done) = fs.alarm_page(&snapshot, after, limit).await?;
			crate::metrics::ALARM_DISCOVERY_ROWS.inc_by(rows.len() as u64);
			Ok(Response::Page { rows, done })
		}
		Request::Read { key } => Ok(Response::Read {
			current: fs.current_alarm(&key).await?,
		}),
		Request::Call { key, summary, .. } => {
			let Ok(_permit) = driver.pool.permits.try_acquire() else {
				if !driver.pool.warned.swap(true, Ordering::Relaxed) {
					tracing::warn!(
						capacity = driver.pool.capacity,
						"volume alarm callback pool is full; pending alarms retain their retry deadlines"
					);
				}
				return Ok(Response::Called {
					busy: true,
					randomness,
				});
			};
			let _active = crate::metrics::ALARM_CALLBACKS.inc_guard();
			let used = driver.pool.capacity as usize - driver.pool.permits.available_permits();
			if used * 10 >= driver.pool.capacity as usize * 9 {
				if !driver.pool.warned.swap(true, Ordering::Relaxed) {
					tracing::warn!(
						used,
						capacity = driver.pool.capacity,
						"volume alarm callback pool is near capacity; pending callbacks will back off at admission"
					);
				}
			} else if used * 10 < driver.pool.capacity as usize * 8 {
				driver.pool.warned.store(false, Ordering::Relaxed);
			}
			let revision = depot_types::alarm::revision(key.0.volume_incarnation, &summary)
				.context("callback without an alarm")?;
			let deadline_unix_ms = summary
				.pending
				.context("callback without a deadline")?
				.deadline_unix_ms;
			if let Err(error) = driver
				.driver
				.on_alarm(
					key.0.volume_key,
					AlarmContext {
						revision,
						deadline_unix_ms,
						owner,
					},
				)
				.await
			{
				tracing::warn!(%error, "volume alarm callback failed; the unacknowledged alarm will retry");
			}
			Ok(Response::Called {
				busy: false,
				randomness,
			})
		}
		Request::Retry {
			key,
			summary,
			record,
		} => {
			let current = fs
				.stage_alarm_retry(
					&key,
					summary,
					record.next_unix_ms,
					record.attempt,
					record.owner_id,
					record.next_boot_ms,
				)
				.await?;
			Ok(Response::Updated { current })
		}
	}
}
