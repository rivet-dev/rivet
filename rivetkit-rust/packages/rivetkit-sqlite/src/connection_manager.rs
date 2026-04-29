use anyhow::{Result, anyhow};
use libsqlite3_sys::{
	SQLITE_OPEN_CREATE, SQLITE_OPEN_READONLY, SQLITE_OPEN_READWRITE, sqlite3,
	sqlite3_get_autocommit,
};
use tokio::sync::{Mutex, Notify};

use crate::vfs::{NativeConnection, NativeVfsHandle, open_connection};

const DEFAULT_MAX_READERS: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeConnectionManagerConfig {
	pub max_readers: usize,
}

impl Default for NativeConnectionManagerConfig {
	fn default() -> Self {
		Self {
			max_readers: DEFAULT_MAX_READERS,
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
	state: Mutex<NativeConnectionManagerState>,
	changed: Notify,
}

struct NativeConnectionManagerState {
	vfs: Option<NativeVfsHandle>,
	mode: NativeConnectionManagerMode,
	idle_readers: Vec<NativeConnection>,
	idle_writer: Option<NativeConnection>,
	active_readers: usize,
	open_readers: usize,
	pending_writers: usize,
	active_writer: bool,
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
		Self {
			inner: std::sync::Arc::new(NativeConnectionManagerInner {
				file_name: file_name.into(),
				config,
				state: Mutex::new(NativeConnectionManagerState {
					vfs: Some(vfs),
					mode: NativeConnectionManagerMode::Closed,
					idle_readers: Vec::new(),
					idle_writer: None,
					active_readers: 0,
					open_readers: 0,
					pending_writers: 0,
					active_writer: false,
				}),
				changed: Notify::new(),
			}),
		}
	}

	pub async fn acquire_read(&self) -> Result<NativeReadConnectionLease> {
		if self.inner.config.max_readers == 0 {
			return Err(anyhow!("sqlite read connection manager has no reader slots"));
		}

		loop {
			let notified = self.inner.changed.notified();
			let open_result = {
				let mut state = self.inner.state.lock().await;
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
					state.refresh_mode();
					return Ok(NativeReadConnectionLease {
						manager: self.clone(),
						connection: Some(connection),
						newly_opened: false,
					});
				} else if state.open_readers < self.inner.config.max_readers {
					state.active_readers += 1;
					state.open_readers += 1;
					state.mode = NativeConnectionManagerMode::ReadMode;
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
						state.refresh_mode();
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
					state.mode = NativeConnectionManagerMode::WriteMode;
					if let Some(connection) = state.idle_writer.take() {
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
						return Ok(NativeWriteConnectionLease {
							manager: self.clone(),
							connection: Some(connection),
							newly_opened: true,
						});
					}
					Err(err) => {
						let mut state = self.inner.state.lock().await;
						state.active_writer = false;
						state.refresh_mode();
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
			std::mem::take(&mut state.idle_readers)
		};
		drop(idle_readers);

		loop {
			let notified = self.inner.changed.notified();
			let vfs = {
				let mut state = self.inner.state.lock().await;
				if state.active_readers == 0 && !state.active_writer {
					state.mode = NativeConnectionManagerMode::Closed;
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
				state.idle_readers.push(connection);
				state.refresh_mode();
				None
			} else {
				state.open_readers = state.open_readers.saturating_sub(1);
				state.refresh_mode();
				Some(connection)
			}
		};
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
			.is_some_and(|connection| unsafe { sqlite3_get_autocommit(connection.as_ptr()) == 0 });
		let close_connection = {
			let mut state = self.manager.inner.state.lock().await;
			state.active_writer = false;
			if keep_writer_open
				&& state.vfs.is_some()
				&& !matches!(state.mode, NativeConnectionManagerMode::Closing)
			{
				state.idle_writer = connection;
				state.mode = NativeConnectionManagerMode::WriteMode;
				None
			} else {
				state.refresh_mode();
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
	fn refresh_mode(&mut self) {
		if matches!(self.mode, NativeConnectionManagerMode::Closing) {
			return;
		}
		self.mode = if self.active_writer {
			NativeConnectionManagerMode::WriteMode
		} else if self.idle_writer.is_some() {
			NativeConnectionManagerMode::WriteMode
		} else if self.active_readers > 0 || self.open_readers > 0 {
			NativeConnectionManagerMode::ReadMode
		} else {
			NativeConnectionManagerMode::Closed
		};
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
}
