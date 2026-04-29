use std::{
	sync::Arc,
	time::{Duration, Instant},
};

use anyhow::{Result, anyhow};
use libsqlite3_sys::{
	SQLITE_OPEN_CREATE, SQLITE_OPEN_READONLY, SQLITE_OPEN_READWRITE, sqlite3,
	sqlite3_get_autocommit,
};
use tokio::sync::{Mutex, Notify};

use crate::{
	optimization_flags::SqliteOptimizationFlags,
	vfs::{NativeConnection, NativeVfsHandle, SqliteVfsMetrics, open_connection},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeConnectionManagerConfig {
	pub read_pool_enabled: bool,
	pub max_readers: usize,
	pub idle_ttl: Duration,
}

impl Default for NativeConnectionManagerConfig {
	fn default() -> Self {
		Self::from_optimization_flags(SqliteOptimizationFlags::default())
	}
}

impl NativeConnectionManagerConfig {
	pub fn from_optimization_flags(flags: SqliteOptimizationFlags) -> Self {
		Self {
			read_pool_enabled: flags.sqlite_read_pool_enabled,
			max_readers: flags.sqlite_read_pool_max_readers,
			idle_ttl: Duration::from_millis(flags.sqlite_read_pool_idle_ttl_ms),
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeConnectionManagerMode {
	Closed,
	ReadMode,
	WriteMode,
	Closing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeConnectionManagerSnapshot {
	pub mode: NativeConnectionManagerMode,
	pub active_readers: usize,
	pub idle_readers: usize,
	pub open_readers: usize,
	pub pending_writers: usize,
	pub active_writer: bool,
}

#[derive(Clone)]
pub struct NativeConnectionManager {
	inner: std::sync::Arc<NativeConnectionManagerInner>,
}

struct NativeConnectionManagerInner {
	file_name: String,
	config: NativeConnectionManagerConfig,
	metrics: Option<Arc<dyn SqliteVfsMetrics>>,
	state: Mutex<NativeConnectionManagerState>,
	changed: Notify,
}

struct NativeConnectionManagerState {
	vfs: Option<NativeVfsHandle>,
	mode: NativeConnectionManagerMode,
	idle_readers: Vec<IdleReadConnection>,
	idle_writer: Option<NativeConnection>,
	active_readers: usize,
	open_readers: usize,
	pending_writers: usize,
	active_writer: bool,
	manual_transaction_started_at: Option<Instant>,
}

struct IdleReadConnection {
	connection: NativeConnection,
	idle_since: Instant,
}

#[must_use = "release the read connection lease when work is complete"]
pub struct NativeReadConnectionLease {
	manager: NativeConnectionManager,
	connection: Option<NativeConnection>,
	newly_opened: bool,
}

#[must_use = "release the write connection lease when work is complete"]
pub struct NativeWriteConnectionLease {
	manager: NativeConnectionManager,
	connection: Option<NativeConnection>,
	newly_opened: bool,
}

impl NativeConnectionManager {
	pub fn new(
		vfs: NativeVfsHandle,
		file_name: impl Into<String>,
		config: NativeConnectionManagerConfig,
	) -> Self {
		Self::new_with_metrics(vfs, file_name, config, None)
	}

	pub fn new_with_metrics(
		vfs: NativeVfsHandle,
		file_name: impl Into<String>,
		config: NativeConnectionManagerConfig,
		metrics: Option<Arc<dyn SqliteVfsMetrics>>,
	) -> Self {
		Self {
			inner: std::sync::Arc::new(NativeConnectionManagerInner {
				file_name: file_name.into(),
				config,
				metrics,
				state: Mutex::new(NativeConnectionManagerState {
					vfs: Some(vfs),
					mode: NativeConnectionManagerMode::Closed,
					idle_readers: Vec::new(),
					idle_writer: None,
					active_readers: 0,
					open_readers: 0,
					pending_writers: 0,
					active_writer: false,
					manual_transaction_started_at: None,
				}),
				changed: Notify::new(),
			}),
		}
	}

	pub fn read_pool_enabled(&self) -> bool {
		self.inner.config.read_pool_enabled
	}

	pub async fn acquire_read(&self) -> Result<NativeReadConnectionLease> {
		if !self.inner.config.read_pool_enabled {
			return Err(anyhow!("sqlite read connection pool is disabled"));
		}
		if self.inner.config.max_readers == 0 {
			return Err(anyhow!("sqlite read connection manager has no reader slots"));
		}

		let wait_started_at = Instant::now();
		loop {
			let notified = self.inner.changed.notified();
			let open_result = {
				let mut state = self.inner.state.lock().await;
				let closed_readers = state.prune_expired_readers(self.inner.config.idle_ttl);
				self.record_reader_closes(closed_readers);
				self.record_reader_gauges(&state);
				if state.vfs.is_none() {
					return Err(anyhow!("sqlite connection manager is closed"));
				}
				if matches!(state.mode, NativeConnectionManagerMode::Closing) {
					return Err(anyhow!("sqlite connection manager is closing"));
				}
				if state.pending_writers > 0
					|| matches!(state.mode, NativeConnectionManagerMode::WriteMode)
					|| state.active_writer
				{
					None
				} else if let Some(connection) = state.idle_readers.pop() {
					state.active_readers += 1;
					self.record_mode_transition(state.refresh_mode());
					self.record_reader_gauges(&state);
					self.observe_read_wait(wait_started_at.elapsed());
					return Ok(NativeReadConnectionLease {
						manager: self.clone(),
						connection: Some(connection.connection),
						newly_opened: false,
					});
				} else if state.open_readers < self.inner.config.max_readers {
					state.active_readers += 1;
					state.open_readers += 1;
					self.record_mode_transition(state.set_mode(NativeConnectionManagerMode::ReadMode));
					self.record_reader_gauges(&state);
					Some(
						state
							.vfs
							.as_ref()
							.expect("vfs checked above")
							.clone(),
					)
				} else {
					None
				}
			};

			if let Some(vfs) = open_result {
				let file_name = self.inner.file_name.clone();
				match tokio::task::spawn_blocking(move || {
					open_connection(vfs, &file_name, SQLITE_OPEN_READONLY)
				})
				.await?
				{
					Ok(connection) => {
						self.record_reader_open();
						self.observe_read_wait(wait_started_at.elapsed());
						return Ok(NativeReadConnectionLease {
							manager: self.clone(),
							connection: Some(connection),
							newly_opened: true,
						});
					}
					Err(err) => {
						let mut state = self.inner.state.lock().await;
						state.active_readers = state.active_readers.saturating_sub(1);
						state.open_readers = state.open_readers.saturating_sub(1);
						self.record_mode_transition(state.refresh_mode());
						self.record_reader_gauges(&state);
						self.inner.changed.notify_waiters();
						return Err(anyhow!("failed to open sqlite read connection: {err}"));
					}
				}
			}

			notified.await;
		}
	}

	pub async fn acquire_write(&self) -> Result<NativeWriteConnectionLease> {
		let mut pending_registered = false;
		let wait_started_at = Instant::now();

		loop {
			let notified = self.inner.changed.notified();
			let open_result = {
				let mut state = self.inner.state.lock().await;
				if !pending_registered {
					state.pending_writers += 1;
					pending_registered = true;
					self.inner.changed.notify_waiters();
				}
				if state.vfs.is_none() {
					state.pending_writers = state.pending_writers.saturating_sub(1);
					return Err(anyhow!("sqlite connection manager is closed"));
				}
				if matches!(state.mode, NativeConnectionManagerMode::Closing) {
					state.pending_writers = state.pending_writers.saturating_sub(1);
					self.inner.changed.notify_waiters();
					return Err(anyhow!("sqlite connection manager is closing"));
				}
				if state.active_readers == 0 && !state.active_writer {
					let idle_readers = std::mem::take(&mut state.idle_readers);
					state.open_readers = state.open_readers.saturating_sub(idle_readers.len());
					state.pending_writers = state.pending_writers.saturating_sub(1);
					state.active_writer = true;
					self.record_reader_closes(idle_readers.len());
					self.record_mode_transition(state.set_mode(NativeConnectionManagerMode::WriteMode));
					self.record_reader_gauges(&state);
					if let Some(connection) = state.idle_writer.take() {
						self.observe_write_wait(wait_started_at.elapsed());
						return Ok(NativeWriteConnectionLease {
							manager: self.clone(),
							connection: Some(connection),
							newly_opened: false,
						});
					}
					Some((
						state
							.vfs
							.as_ref()
							.expect("vfs checked above")
							.clone(),
						idle_readers,
					))
				} else {
					None
				}
			};

			if let Some((vfs, idle_readers)) = open_result {
				drop(idle_readers);
				let file_name = self.inner.file_name.clone();
				match tokio::task::spawn_blocking(move || {
					open_connection(
						vfs,
						&file_name,
						SQLITE_OPEN_READWRITE | SQLITE_OPEN_CREATE,
					)
				})
				.await?
				{
					Ok(connection) => {
						self.observe_write_wait(wait_started_at.elapsed());
						return Ok(NativeWriteConnectionLease {
							manager: self.clone(),
							connection: Some(connection),
							newly_opened: true,
						});
					}
					Err(err) => {
						let mut state = self.inner.state.lock().await;
						state.active_writer = false;
						self.record_mode_transition(state.refresh_mode());
						self.inner.changed.notify_waiters();
						return Err(anyhow!("failed to open sqlite write connection: {err}"));
					}
				}
			}

			notified.await;
		}
	}

	pub async fn with_read_connection<T, F>(
		&self,
		f: F,
	) -> Result<T>
	where
		T: Send + 'static,
		F: FnOnce(*mut sqlite3) -> Result<T> + Send + 'static,
	{
		self.with_read_connection_state(move |db, _newly_opened| f(db))
			.await
	}

	pub async fn with_read_connection_state<T, F>(
		&self,
		f: F,
	) -> Result<T>
	where
		T: Send + 'static,
		F: FnOnce(*mut sqlite3, bool) -> Result<T> + Send + 'static,
	{
		let mut lease = self.acquire_read().await?;
		let newly_opened = lease.newly_opened;
		let connection = lease
			.connection
			.take()
			.expect("read connection lease should hold a connection");
		let (connection, result) =
			tokio::task::spawn_blocking(move || {
				let result = f(connection.as_ptr(), newly_opened);
				(connection, result)
			})
			.await?;
		lease.connection = Some(connection);
		lease.release().await;
		result
	}

	pub async fn with_write_connection<T, F>(
		&self,
		f: F,
	) -> Result<T>
	where
		T: Send + 'static,
		F: FnOnce(*mut sqlite3) -> Result<T> + Send + 'static,
	{
		self.with_write_connection_state(move |db, _newly_opened| f(db))
			.await
	}

	pub async fn with_write_connection_state<T, F>(
		&self,
		f: F,
	) -> Result<T>
	where
		T: Send + 'static,
		F: FnOnce(*mut sqlite3, bool) -> Result<T> + Send + 'static,
	{
		let mut lease = self.acquire_write().await?;
		let newly_opened = lease.newly_opened;
		let connection = lease
			.connection
			.take()
			.expect("write connection lease should hold a connection");
		let (connection, result) =
			tokio::task::spawn_blocking(move || {
				let result = f(connection.as_ptr(), newly_opened);
				(connection, result)
			})
			.await?;
		lease.connection = Some(connection);
		lease.release().await;
		result
	}

	pub async fn close(&self) -> Result<()> {
		let idle_readers = {
			let mut state = self.inner.state.lock().await;
			if state.vfs.is_none() {
				return Ok(());
			}
			state.mode = NativeConnectionManagerMode::Closing;
			state.open_readers = state.open_readers.saturating_sub(state.idle_readers.len());
			self.inner.changed.notify_waiters();
			state.idle_writer.take();
			self.record_reader_closes(state.idle_readers.len());
			self.record_reader_gauges(&state);
			std::mem::take(&mut state.idle_readers)
		};
		drop(idle_readers);

		loop {
			let notified = self.inner.changed.notified();
			let vfs = {
				let mut state = self.inner.state.lock().await;
				if state.active_readers == 0 && !state.active_writer {
					self.record_mode_transition(state.set_mode(NativeConnectionManagerMode::Closed));
					state.vfs.take()
				} else {
					None
				}
			};

			if let Some(vfs) = vfs {
				drop(vfs);
				self.inner.changed.notify_waiters();
				return Ok(());
			}

			notified.await;
		}
	}

	pub async fn snapshot(&self) -> NativeConnectionManagerSnapshot {
		let state = self.inner.state.lock().await;
		state.snapshot()
	}

	#[cfg(test)]
	pub(crate) async fn wait_for_snapshot(
		&self,
		predicate: impl Fn(&NativeConnectionManagerSnapshot) -> bool,
	) -> NativeConnectionManagerSnapshot {
		loop {
			let notified = self.inner.changed.notified();
			let snapshot = self.snapshot().await;
			if predicate(&snapshot) {
				return snapshot;
			}
			notified.await;
		}
	}

	fn record_reader_gauges(&self, state: &NativeConnectionManagerState) {
		if let Some(metrics) = &self.inner.metrics {
			metrics.set_read_pool_active_readers(state.active_readers as u64);
			metrics.set_read_pool_idle_readers(state.idle_readers.len() as u64);
		}
	}

	fn record_reader_open(&self) {
		if let Some(metrics) = &self.inner.metrics {
			metrics.record_read_pool_reader_open();
		}
	}

	fn record_reader_closes(&self, count: usize) {
		if count == 0 {
			return;
		}
		if let Some(metrics) = &self.inner.metrics {
			metrics.record_read_pool_reader_close(count as u64);
		}
	}

	fn observe_read_wait(&self, duration: Duration) {
		if let Some(metrics) = &self.inner.metrics {
			metrics.observe_read_pool_read_wait(duration);
		}
	}

	fn observe_write_wait(&self, duration: Duration) {
		if let Some(metrics) = &self.inner.metrics {
			metrics.observe_read_pool_write_wait(duration);
		}
	}

	fn record_mode_transition(
		&self,
		transition: Option<(NativeConnectionManagerMode, NativeConnectionManagerMode)>,
	) {
		let Some((from, to)) = transition else {
			return;
		};
		if let Some(metrics) = &self.inner.metrics {
			metrics.record_read_pool_mode_transition(from.as_metric_label(), to.as_metric_label());
		}
	}

	fn observe_manual_transaction(&self, duration: Duration) {
		if let Some(metrics) = &self.inner.metrics {
			metrics.observe_read_pool_manual_transaction(duration);
		}
	}
}

impl NativeReadConnectionLease {
	pub fn as_ptr(&self) -> *mut sqlite3 {
		self.connection
			.as_ref()
			.expect("read connection lease should hold a connection")
			.as_ptr()
	}

	pub async fn release(mut self) {
		let Some(connection) = self.connection.take() else {
			return;
		};
		let idle_connection = {
			let mut state = self.manager.inner.state.lock().await;
			state.active_readers = state.active_readers.saturating_sub(1);
			if state.vfs.is_some()
				&& state.pending_writers == 0
				&& !matches!(state.mode, NativeConnectionManagerMode::Closing)
			{
				state.idle_readers.push(IdleReadConnection {
					connection,
					idle_since: Instant::now(),
				});
				self.manager.record_mode_transition(state.refresh_mode());
				self.manager.record_reader_gauges(&state);
				None
			} else {
				state.open_readers = state.open_readers.saturating_sub(1);
				self.manager.record_mode_transition(state.refresh_mode());
				self.manager.record_reader_gauges(&state);
				Some(connection)
			}
		};
		if idle_connection.is_some() {
			self.manager.record_reader_closes(1);
		}
		drop(idle_connection);
		self.manager.inner.changed.notify_waiters();
	}
}

impl Drop for NativeReadConnectionLease {
	fn drop(&mut self) {
		if self.connection.is_some() {
			tracing::warn!("sqlite read connection lease dropped without release");
		}
	}
}

impl NativeWriteConnectionLease {
	pub fn as_ptr(&self) -> *mut sqlite3 {
		self.connection
			.as_ref()
			.expect("write connection lease should hold a connection")
			.as_ptr()
	}

	pub fn newly_opened(&self) -> bool {
		self.newly_opened
	}

	pub async fn release(mut self) {
		let connection = self.connection.take();
		let keep_writer_open = connection
			.as_ref()
			.is_some_and(|connection| {
				!self.manager.inner.config.read_pool_enabled
					|| unsafe { sqlite3_get_autocommit(connection.as_ptr()) == 0 }
			});
		let close_connection = {
			let mut state = self.manager.inner.state.lock().await;
			state.active_writer = false;
			if keep_writer_open
				&& state.vfs.is_some()
				&& !matches!(state.mode, NativeConnectionManagerMode::Closing)
			{
				if state.manual_transaction_started_at.is_none()
					&& connection
						.as_ref()
						.is_some_and(|connection| unsafe {
							sqlite3_get_autocommit(connection.as_ptr()) == 0
						})
				{
					state.manual_transaction_started_at = Some(Instant::now());
				}
				state.idle_writer = connection;
				self.manager.record_mode_transition(
					state.set_mode(NativeConnectionManagerMode::WriteMode),
				);
				None
			} else {
				if let Some(started_at) = state.manual_transaction_started_at.take() {
					self.manager.observe_manual_transaction(started_at.elapsed());
				}
				self.manager.record_mode_transition(state.refresh_mode());
				connection
			}
		};
		drop(close_connection);
		self.manager.inner.changed.notify_waiters();
	}
}

impl Drop for NativeWriteConnectionLease {
	fn drop(&mut self) {
		if self.connection.is_some() {
			tracing::warn!("sqlite write connection lease dropped without release");
		}
	}
}

impl NativeConnectionManagerState {
	fn set_mode(
		&mut self,
		mode: NativeConnectionManagerMode,
	) -> Option<(NativeConnectionManagerMode, NativeConnectionManagerMode)> {
		let previous = self.mode;
		self.mode = mode;
		(previous != mode).then_some((previous, mode))
	}

	fn refresh_mode(&mut self) -> Option<(NativeConnectionManagerMode, NativeConnectionManagerMode)> {
		if matches!(self.mode, NativeConnectionManagerMode::Closing) {
			return None;
		}
		let mode = if self.active_writer {
			NativeConnectionManagerMode::WriteMode
		} else if self.idle_writer.is_some() {
			NativeConnectionManagerMode::WriteMode
		} else if self.active_readers > 0 || self.open_readers > 0 {
			NativeConnectionManagerMode::ReadMode
		} else {
			NativeConnectionManagerMode::Closed
		};
		self.set_mode(mode)
	}

	fn snapshot(&self) -> NativeConnectionManagerSnapshot {
		NativeConnectionManagerSnapshot {
			mode: self.mode,
			active_readers: self.active_readers,
			idle_readers: self.idle_readers.len(),
			open_readers: self.open_readers,
			pending_writers: self.pending_writers,
			active_writer: self.active_writer,
		}
	}

	fn prune_expired_readers(&mut self, idle_ttl: Duration) -> usize {
		let now = Instant::now();
		let before = self.idle_readers.len();
		self.idle_readers
			.retain(|reader| now.duration_since(reader.idle_since) < idle_ttl);
		let closed = before - self.idle_readers.len();
		self.open_readers = self.open_readers.saturating_sub(closed);
		if closed > 0 {
			self.refresh_mode();
		}
		closed
	}
}

impl NativeConnectionManagerMode {
	fn as_metric_label(self) -> &'static str {
		match self {
			Self::Closed => "closed",
			Self::ReadMode => "read",
			Self::WriteMode => "write",
			Self::Closing => "closing",
		}
	}
}
