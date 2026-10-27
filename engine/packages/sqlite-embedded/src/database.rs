use std::sync::Arc;
use std::time::Instant;

use anyhow::{Result, anyhow};
use async_trait::async_trait;
use rivet_envoy_protocol as protocol;
use tokio::runtime::Handle;
use tokio::sync::watch;

use crate::{
	query::{BindParam, ExecResult, ExecuteResult, QueryResult},
	vfs::{
		CommitMode, DatabaseFailure, FlushError, InitialPages, NativeVfsHandle, SqliteOpenPhase,
		SqliteTransportHandle, SqliteVfs, SqliteVfsMetrics, SqliteVfsMetricsSnapshot, VfsConfig,
		fetch_initial_pages_for_registration,
	},
	worker::{
		SqlitePool, SqliteWorkerCloseTimeoutError, SqliteWorkerFatalError, SqliteWorkerHandle,
		SqliteWorkerResult,
	},
};

#[derive(Clone)]
pub struct NativeDatabaseHandle {
	vfs: NativeVfsHandle,
	worker: SqliteWorkerHandle,
}

pub fn vfs_name_for_actor_database(actor_id: &str, generation: u64) -> String {
	format!("envoy-sqlite-{actor_id}-g{generation}")
}

struct GenerationFencedTransport {
	inner: SqliteTransportHandle,
	generation: u64,
}

#[async_trait]
impl crate::vfs::SqliteTransport for GenerationFencedTransport {
	async fn get_pages(
		&self,
		mut request: protocol::SqliteGetPagesRequest,
	) -> Result<protocol::SqliteGetPagesResponse> {
		request.expected_generation.get_or_insert(self.generation);
		self.inner.get_pages(request).await
	}

	async fn commit(
		&self,
		mut request: protocol::SqliteCommitRequest,
	) -> Result<protocol::SqliteCommitResponse> {
		request.expected_generation.get_or_insert(self.generation);
		self.inner.commit(request).await
	}

	async fn commit_stage_begin(
		&self,
		mut request: protocol::SqliteCommitStageBeginRequest,
	) -> Result<protocol::SqliteCommitStageBeginResponse> {
		request.expected_generation.get_or_insert(self.generation);
		self.inner.commit_stage_begin(request).await
	}

	async fn commit_stage_segment(
		&self,
		mut request: protocol::SqliteCommitStageSegmentRequest,
	) -> Result<protocol::SqliteCommitStageSegmentResponse> {
		request.expected_generation.get_or_insert(self.generation);
		self.inner.commit_stage_segment(request).await
	}

	async fn commit_finalize(
		&self,
		mut request: protocol::SqliteCommitFinalizeRequest,
	) -> Result<protocol::SqliteCommitFinalizeResponse> {
		request.expected_generation.get_or_insert(self.generation);
		self.inner.commit_finalize(request).await
	}
}

pub async fn open_database_from_transport(
	transport: SqliteTransportHandle,
	actor_id: String,
	generation: u64,
	rt_handle: Handle,
	metrics: Option<Arc<dyn SqliteVfsMetrics>>,
) -> Result<NativeDatabaseHandle> {
	open_database_from_transport_on_pool(
		None,
		transport,
		actor_id,
		generation,
		rt_handle,
		metrics,
		CommitMode::Awaited,
	)
	.await
}

/// Opens the database on the runtime's shared worker pool, or on a dedicated single-thread
/// pool when `pool` is `None`.
pub async fn open_database_from_transport_on_pool(
	pool: Option<&SqlitePool>,
	transport: SqliteTransportHandle,
	actor_id: String,
	generation: u64,
	rt_handle: Handle,
	metrics: Option<Arc<dyn SqliteVfsMetrics>>,
	commit_mode: CommitMode,
) -> Result<NativeDatabaseHandle> {
	let vfs_name = vfs_name_for_actor_database(&actor_id, generation);
	open_database(
		pool,
		vfs_name,
		transport,
		actor_id,
		generation,
		rt_handle,
		metrics,
		None,
		Preload::Fetch,
		commit_mode,
	)
	.await
}

/// Opens the database of an actor whose start showed its storage was never written, on the
/// runtime's shared worker pool, without reading any page first: a new database has none.
///
/// Its first write carries no expected head, which storage accepts only while the database is
/// empty, so a database that turns out to hold pages fails that write with a head fence mismatch
/// instead of being overwritten.
pub async fn open_new_database_from_transport_on_pool(
	pool: Option<&SqlitePool>,
	transport: SqliteTransportHandle,
	actor_id: String,
	generation: u64,
	rt_handle: Handle,
	metrics: Option<Arc<dyn SqliteVfsMetrics>>,
	commit_mode: CommitMode,
) -> Result<NativeDatabaseHandle> {
	let vfs_name = vfs_name_for_actor_database(&actor_id, generation);
	open_database(
		pool,
		vfs_name,
		transport,
		actor_id,
		generation,
		rt_handle,
		metrics,
		None,
		Preload::Empty,
		commit_mode,
	)
	.await
}

/// Like [`open_database_from_transport_on_pool`], with `guard` installed on the connection for
/// SQL sent by an untrusted peer. `vfs_name` must be unique in the process while the database
/// is open: registering a second VFS under a live name replaces the first.
#[allow(clippy::too_many_arguments)]
pub async fn open_database_from_transport_with_guard(
	pool: Option<&SqlitePool>,
	vfs_name: String,
	transport: SqliteTransportHandle,
	actor_id: String,
	generation: u64,
	rt_handle: Handle,
	metrics: Option<Arc<dyn SqliteVfsMetrics>>,
	guard: Option<crate::guard::ConnectionGuard>,
) -> Result<NativeDatabaseHandle> {
	open_database(
		pool,
		vfs_name,
		transport,
		actor_id,
		generation,
		rt_handle,
		metrics,
		guard,
		Preload::Fetch,
		CommitMode::Awaited,
	)
	.await
}

/// How an open learns the database's first pages.
enum Preload {
	/// Read them from storage before registering the VFS.
	Fetch,
	/// The database is known to be new, so it starts empty with no read.
	Empty,
}

#[allow(clippy::too_many_arguments)]
async fn open_database(
	pool: Option<&SqlitePool>,
	vfs_name: String,
	transport: SqliteTransportHandle,
	actor_id: String,
	generation: u64,
	rt_handle: Handle,
	metrics: Option<Arc<dyn SqliteVfsMetrics>>,
	guard: Option<crate::guard::ConnectionGuard>,
	preload: Preload,
	commit_mode: CommitMode,
) -> Result<NativeDatabaseHandle> {
	let open_timer = SqliteOpenTimer::new(&metrics);
	// A database without a shared pool runs on its own single-thread pool, which also owns its
	// page budget.
	let owned_pool;
	let pool = match pool {
		Some(pool) => pool,
		None => {
			owned_pool = SqlitePool::new(1, None);
			&owned_pool
		}
	};
	let mut config = VfsConfig::default();
	config.page_budget = pool.page_budget().clone();
	config.commit_mode = commit_mode;
	let transport: SqliteTransportHandle = Arc::new(GenerationFencedTransport {
		inner: transport,
		generation,
	});
	let initial_pages = match preload {
		Preload::Fetch => {
			let preload_start = Instant::now();
			let preload_result = fetch_initial_pages_for_registration(
				transport.clone(),
				&actor_id,
				generation,
				&config,
			)
			.await
			.map_err(|err| anyhow!("failed to preload sqlite pages: {err}"));
			let initial_pages = observe_open_phase_result(
				&metrics,
				SqliteOpenPhase::InitialPreload,
				preload_start,
				preload_result,
			)?;
			record_startup_preload_pages(
				&metrics,
				u64::from(initial_pages.requested_page_count),
				initial_pages.pages.len() as u64,
			);
			initial_pages
		}
		Preload::Empty => InitialPages::default(),
	};

	let vfs_register_start = Instant::now();
	let vfs_result = SqliteVfs::register_with_transport_and_initial_pages(
		&vfs_name,
		transport,
		actor_id.clone(),
		rt_handle,
		config,
		initial_pages,
		metrics.clone(),
	)
	.map(Arc::new)
	.map_err(|err| anyhow!("failed to register sqlite VFS: {err}"));
	let vfs = observe_open_phase_result(
		&metrics,
		SqliteOpenPhase::VfsRegister,
		vfs_register_start,
		vfs_result,
	)?;

	let worker_ready_start = Instant::now();
	let worker_ready_result: Result<NativeDatabaseHandle> = async {
		let native_db = NativeDatabaseHandle {
			worker: SqliteWorkerHandle::start_on_with_guard(
				pool,
				vfs.clone(),
				actor_id,
				metrics.clone(),
				guard,
			)?,
			vfs,
		};
		native_db.initialize().await?;
		Ok(native_db)
	}
	.await;
	let native_db = observe_open_phase_result(
		&metrics,
		SqliteOpenPhase::WorkerReady,
		worker_ready_start,
		worker_ready_result,
	)?;
	open_timer.finish_success();
	Ok(native_db)
}

/// Records the total SQLite open metric when the open attempt leaves scope.
///
/// Individual open phases record their own durations. This guard owns the total
/// duration so early returns cannot forget the total error metric.
struct SqliteOpenTimer {
	metrics: Option<Arc<dyn SqliteVfsMetrics>>,
	started_at: Instant,
	finished: bool,
}

impl SqliteOpenTimer {
	fn new(metrics: &Option<Arc<dyn SqliteVfsMetrics>>) -> Self {
		Self {
			metrics: metrics.clone(),
			started_at: Instant::now(),
			finished: false,
		}
	}

	fn finish_success(mut self) {
		observe_open_phase(
			&self.metrics,
			SqliteOpenPhase::Total,
			"success",
			self.started_at,
		);
		self.finished = true;
	}
}

impl Drop for SqliteOpenTimer {
	fn drop(&mut self) {
		if self.finished {
			return;
		}

		observe_open_phase(
			&self.metrics,
			SqliteOpenPhase::Total,
			"error",
			self.started_at,
		);
	}
}

fn observe_open_phase_result<T, E>(
	metrics: &Option<Arc<dyn SqliteVfsMetrics>>,
	phase: SqliteOpenPhase,
	start: Instant,
	result: std::result::Result<T, E>,
) -> std::result::Result<T, E> {
	let outcome = if result.is_ok() { "success" } else { "error" };
	observe_open_phase(metrics, phase, outcome, start);
	result
}

fn observe_open_phase(
	metrics: &Option<Arc<dyn SqliteVfsMetrics>>,
	phase: SqliteOpenPhase,
	outcome: &'static str,
	start: Instant,
) {
	if let Some(metrics) = metrics {
		metrics.observe_open_phase(phase, outcome, start.elapsed().as_nanos() as u64);
	}
}

fn record_startup_preload_pages(
	metrics: &Option<Arc<dyn SqliteVfsMetrics>>,
	requested_pages: u64,
	loaded_pages: u64,
) {
	if let Some(metrics) = metrics {
		if requested_pages > 0 {
			metrics.record_startup_preload_pages("requested", requested_pages);
		}
		if loaded_pages > 0 {
			metrics.record_startup_preload_pages("loaded", loaded_pages);
		}
	}
}

impl NativeDatabaseHandle {
	pub fn new(vfs: NativeVfsHandle, file_name: String) -> Result<Self> {
		Self::new_with_metrics(vfs, file_name, None)
	}

	/// Runs the database on a dedicated single-thread pool.
	pub fn new_with_metrics(
		vfs: NativeVfsHandle,
		file_name: String,
		metrics: Option<Arc<dyn SqliteVfsMetrics>>,
	) -> Result<Self> {
		Ok(Self {
			worker: SqliteWorkerHandle::start(vfs.clone(), file_name, metrics)?,
			vfs,
		})
	}

	/// Runs the database on a shared worker pool.
	pub fn new_on_pool(
		pool: &SqlitePool,
		vfs: NativeVfsHandle,
		file_name: String,
		metrics: Option<Arc<dyn SqliteVfsMetrics>>,
	) -> Result<Self> {
		Ok(Self {
			worker: SqliteWorkerHandle::start_on(pool, vfs.clone(), file_name, metrics)?,
			vfs,
		})
	}

	pub async fn exec(&self, sql: String) -> Result<QueryResult> {
		self.check_fatal_error()?;
		self.map_worker_result(self.worker.exec(sql).await)
	}

	pub async fn exec_profiled(&self, sql: String) -> Result<SqliteWorkerResult<QueryResult>> {
		self.check_fatal_error()?;
		self.map_worker_result(self.worker.exec_profiled(sql).await)
	}

	/// Holds every later commit in memory until `uncork_commits` sends them to depot as one
	/// commit. None of the held transactions is durable until uncorking succeeds.
	pub async fn cork_commits(&self) -> Result<()> {
		self.check_fatal_error()?;
		self.map_worker_result(self.worker.cork_commits().await)
	}

	/// Sends the transactions committed since `cork_commits` to depot as one commit. Inside an
	/// explicit transaction it waits until the transaction ends, so the commit includes it. A
	/// deferred database never corks, so this waits for its held commits to become durable.
	pub async fn uncork_commits(&self) -> Result<()> {
		self.check_fatal_error()?;
		self.map_worker_result(self.worker.uncork_commits().await)?;
		if self.vfs.commit_mode() == CommitMode::Deferred {
			self.wait_for_flush(self.vfs.commit_seq()).await?;
		}
		Ok(())
	}

	pub async fn query(&self, sql: String, params: Option<Vec<BindParam>>) -> Result<QueryResult> {
		self.execute(sql, params).await.map(|result| QueryResult {
			columns: result.columns,
			rows: result.rows,
		})
	}

	pub async fn run(&self, sql: String, params: Option<Vec<BindParam>>) -> Result<ExecResult> {
		self.execute(sql, params).await.map(|result| ExecResult {
			changes: result.changes,
		})
	}

	pub async fn execute(
		&self,
		sql: String,
		params: Option<Vec<BindParam>>,
	) -> Result<ExecuteResult> {
		self.check_fatal_error()?;
		self.map_worker_result(self.worker.execute(sql, params).await)
	}

	pub async fn execute_profiled(
		&self,
		sql: String,
		params: Option<Vec<BindParam>>,
	) -> Result<SqliteWorkerResult<ExecuteResult>> {
		self.check_fatal_error()?;
		self.map_worker_result(self.worker.execute_profiled(sql, params).await)
	}

	/// Marks this database's later work as background work; see
	/// [`SqliteWorkerHandle::set_background`].
	pub fn set_background(&self, background: bool) {
		self.worker.set_background(background);
	}

	/// Runs `statements` in order as one worker command; see
	/// [`SqliteWorkerHandle::execute_batch`].
	pub async fn execute_batch(
		&self,
		statements: Vec<(String, Option<Vec<BindParam>>)>,
	) -> Result<Vec<ExecuteResult>> {
		self.check_fatal_error()?;
		self.map_worker_result(self.worker.execute_batch(statements).await)
	}

	/// Closes the worker, then waits for commits held in deferred mode to become durable.
	pub async fn close(&self) -> Result<()> {
		self.vfs.begin_close();
		let result = self.worker.close().await;
		self.finish_close(result).await
	}

	/// Closes the database without returning while its worker still owns SQLite.
	/// Any returned error is reported only after the worker thread has terminated.
	pub async fn close_and_wait(&self) -> Result<()> {
		self.vfs.begin_close();
		let result = self.worker.close_and_wait().await;
		self.finish_close(result).await
	}

	async fn finish_close(&self, worker_result: Result<()>) -> Result<()> {
		if worker_result.as_ref().err().is_some_and(|error| {
			error
				.downcast_ref::<SqliteWorkerCloseTimeoutError>()
				.is_some()
		}) {
			// The worker may still own SQLite and commit more, so held commits cannot drain.
			self.vfs
				.abort_flusher("sqlite worker close timed out")
				.await;
			return worker_result;
		}
		let flush_result = self.vfs.drain_flusher(self.vfs.close_flush_timeout()).await;
		match (self.map_worker_close_result(worker_result), flush_result) {
			(Err(error), _) => Err(error),
			(Ok(()), Err(error)) => Err(anyhow!(error)),
			(Ok(()), Ok(())) => Ok(()),
		}
	}

	fn map_worker_close_result(&self, result: Result<()>) -> Result<()> {
		match result {
			Ok(()) => Ok(()),
			// The caller uses this concrete outcome to retain the native handle for
			// a later strict join. Never hide it behind a previously recorded VFS
			// failure while the worker may still own SQLite.
			Err(error)
				if error
					.downcast_ref::<SqliteWorkerCloseTimeoutError>()
					.is_some() =>
			{
				Err(error)
			}
			Err(error) => Err(self.fatal_error().unwrap_or(error)),
		}
	}

	/// Resolves once the database stops serving its generation: its worker stopped, deferred
	/// commits can no longer become durable, or it closed.
	pub async fn wait_for_failure(&self) -> DatabaseFailure {
		tokio::select! {
			failure = self.vfs.wait_for_failure() => failure,
			failed = self.worker.wait_for_failure() => {
				if failed {
					DatabaseFailure::WorkerStopped
				} else {
					DatabaseFailure::Closed
				}
			}
		}
	}

	pub fn commit_mode(&self) -> CommitMode {
		self.vfs.commit_mode()
	}

	/// The newest local commit's sequence, which `wait_for_flush` takes as a ticket.
	pub fn commit_seq(&self) -> u64 {
		self.vfs.commit_seq()
	}

	/// Pages committed locally that are not durable yet.
	pub fn unflushed_pages(&self) -> usize {
		self.vfs.unflushed_pages()
	}

	/// Resolves once local commit `seq` and every earlier one are durable.
	pub async fn wait_for_flush(&self, seq: u64) -> std::result::Result<(), FlushError> {
		self.vfs.wait_for_flush(seq).await
	}

	/// Breaks the database: every later statement fails and flush waits reject. Commits held
	/// locally never become durable.
	pub fn break_database(&self, reason: String) {
		self.vfs.break_database(reason);
	}

	/// Returns whether the connection has an open explicit transaction, such as
	/// one started by a manual `BEGIN`, as of the last completed command.
	pub fn explicit_transaction_open(&self) -> bool {
		self.worker.explicit_transaction_open()
	}

	/// Subscribes to explicit transaction state changes. The value changes
	/// before the reply of the command that changed it is delivered.
	pub fn subscribe_explicit_transaction(&self) -> watch::Receiver<bool> {
		self.worker.subscribe_explicit_transaction()
	}

	pub fn take_last_kv_error(&self) -> Option<String> {
		self.vfs.take_last_error()
	}

	pub fn clone_fatal_error(&self) -> Option<String> {
		self.vfs.clone_fatal_error()
	}

	/// Reads back a file the connection wrote through its own VFS, such as a `VACUUM INTO` output.
	pub fn read_vfs_file(&self, path: &str) -> Option<Vec<u8>> {
		self.vfs.read_aux_file(path)
	}

	/// Drops a file the connection wrote through its own VFS, releasing the memory holding it.
	pub fn delete_vfs_file(&self, path: &str) {
		self.vfs.delete_aux_file(path);
	}

	pub fn sqlite_vfs_metrics(&self) -> SqliteVfsMetricsSnapshot {
		self.vfs.sqlite_vfs_metrics()
	}

	

	

	

	

	async fn initialize(&self) -> Result<()> {
		self.map_worker_result(self.worker.wait_ready().await)
	}

	fn check_fatal_error(&self) -> Result<()> {
		if let Some(error) = self.fatal_error() {
			return Err(error);
		}

		Ok(())
	}

	fn map_worker_result<T>(&self, result: Result<T>) -> Result<T> {
		match result {
			Ok(value) => {
				self.check_fatal_error()?;
				Ok(value)
			}
			Err(error) if error.is::<crate::vfs::SqliteCommitIndeterminateError>() => Err(error),
			Err(error)
				if error.is::<crate::vfs::SqliteTransactionTooLargeError>()
					&& self.fatal_error().is_none() =>
			{
				Err(error)
			}
			Err(error) => Err(self.fatal_error().unwrap_or(error)),
		}
	}

	fn fatal_error(&self) -> Option<anyhow::Error> {
		self.clone_fatal_error()
			.map(|message| SqliteWorkerFatalError::new(message).into())
	}
}


