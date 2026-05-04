use std::collections::HashSet;
use std::io::Cursor;
#[cfg(feature = "sqlite-local")]
use std::sync::{
	Arc,
	atomic::{AtomicBool, Ordering},
};

use anyhow::{Context, Result};
pub use depot_client_types::{BindParam, ColumnValue, ExecResult, ExecuteResult, QueryResult};
#[cfg(feature = "sqlite-local")]
use parking_lot::Mutex;
use rivet_envoy_client::protocol;
use rivet_envoy_client::{handle::EnvoyHandle, utils::RemoteSqliteIndeterminateResultError};
use serde::Serialize;
use serde_json::{Map as JsonMap, Value as JsonValue};
#[cfg(feature = "sqlite-local")]
use tokio::sync::Mutex as AsyncMutex;
#[cfg(feature = "sqlite-local")]
use tokio::task::JoinHandle;

#[cfg(feature = "sqlite-local")]
mod envoy_sqlite_transport;

#[cfg(feature = "sqlite-local")]
use crate::error::ActorLifecycle;
use crate::error::SqliteRuntimeError;
#[cfg(feature = "sqlite-local")]
use crate::runtime::RuntimeSpawner;

#[cfg(feature = "sqlite-local")]
use depot_client::{
	database::{NativeDatabaseHandle, open_database_from_transport},
	vfs::{SqliteVfsMetrics, SqliteVfsMetricsSnapshot},
	worker::{
		SQLITE_WORKER_QUEUE_CAPACITY, SqliteWorkerCloseTimeoutError, SqliteWorkerClosingError,
		SqliteWorkerDeadError, SqliteWorkerOverloadedError,
	},
};
#[cfg(feature = "sqlite-local")]
use envoy_sqlite_transport::EnvoySqliteTransport;

#[cfg(not(feature = "sqlite-local"))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SqliteVfsMetricsSnapshot {
	pub request_build_ns: u64,
	pub serialize_ns: u64,
	pub transport_ns: u64,
	pub state_update_ns: u64,
	pub total_ns: u64,
	pub commit_count: u64,
}

#[derive(Clone)]
pub struct SqliteRuntimeConfig {
	pub handle: EnvoyHandle,
	pub actor_id: String,
	pub generation: Option<u64>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SqliteBackend {
	LocalNative,
	RemoteEnvoy,
	Unavailable,
}

impl Default for SqliteBackend {
	fn default() -> Self {
		Self::Unavailable
	}
}

#[derive(Clone, Default)]
pub struct SqliteDb {
	handle: Option<EnvoyHandle>,
	actor_id: Option<String>,
	generation: Option<u64>,
	backend: SqliteBackend,
	/// Mirrors the user's actor-config `db({...})` declaration. The envoy
	/// always sets up sqlite storage under the hood, so handle/actor_id are
	/// not a reliable signal for whether the user opted in; this flag is.
	enabled: bool,
	#[cfg(feature = "sqlite-local")]
	// Forced-sync: native SQLite handles are used inside spawn_blocking and
	// synchronous diagnostic accessors.
	db: Arc<Mutex<Option<NativeDatabaseHandle>>>,
	#[cfg(feature = "sqlite-local")]
	open_lock: Arc<AsyncMutex<()>>,
	#[cfg(feature = "sqlite-local")]
	worker_failure_task: Arc<Mutex<Option<JoinHandle<()>>>>,
	#[cfg(feature = "sqlite-local")]
	worker_fatal_reported: Arc<AtomicBool>,
	#[cfg(feature = "sqlite-local")]
	vfs_metrics: Option<Arc<dyn SqliteVfsMetrics>>,
}

impl SqliteDb {
	pub fn new(handle: EnvoyHandle, actor_id: impl Into<String>, enabled: bool) -> Self {
		Self::new_with_remote_sqlite(handle, actor_id, None, enabled, false)
	}

	pub fn new_with_remote_sqlite(
		handle: EnvoyHandle,
		actor_id: impl Into<String>,
		generation: Option<u64>,
		enabled: bool,
		remote_sqlite: bool,
	) -> Self {
		Self {
			handle: Some(handle),
			actor_id: Some(actor_id.into()),
			generation,
			backend: select_sqlite_backend(enabled, remote_sqlite),
			enabled,
			#[cfg(feature = "sqlite-local")]
			db: Default::default(),
			#[cfg(feature = "sqlite-local")]
			open_lock: Default::default(),
			#[cfg(feature = "sqlite-local")]
			worker_failure_task: Default::default(),
			#[cfg(feature = "sqlite-local")]
			worker_fatal_reported: Default::default(),
			#[cfg(feature = "sqlite-local")]
			vfs_metrics: None,
		}
	}

	#[cfg(feature = "sqlite-local")]
	pub(crate) fn set_vfs_metrics(&mut self, metrics: Arc<dyn SqliteVfsMetrics>) {
		self.vfs_metrics = Some(metrics);
	}

	pub fn is_enabled(&self) -> bool {
		self.enabled
	}

	pub fn backend(&self) -> SqliteBackend {
		self.backend
	}

	pub async fn get_pages(
		&self,
		request: protocol::SqliteGetPagesRequest,
	) -> Result<protocol::SqliteGetPagesResponse> {
		self.handle()?.sqlite_get_pages(request).await
	}

	pub async fn commit(
		&self,
		request: protocol::SqliteCommitRequest,
	) -> Result<protocol::SqliteCommitResponse> {
		self.handle()?.sqlite_commit(request).await
	}

	pub async fn open(&self) -> Result<()> {
		match self.backend {
			SqliteBackend::LocalNative => {
				#[cfg(feature = "sqlite-local")]
				{
					let _open_guard = self.open_lock.lock().await;
					if self.db.lock().is_some() {
						return Ok(());
					}

					let config = self.runtime_config()?;
					let vfs_metrics = self.vfs_metrics.clone();
					let rt_handle = tokio::runtime::Handle::try_current()
						.context("open sqlite database requires a tokio runtime")?;

					let native_db = open_database_from_transport(
						Arc::new(EnvoySqliteTransport::new(config.handle.clone())),
						config.actor_id.clone(),
						config
							.generation
							.ok_or_else(|| sqlite_not_configured("generation"))?,
						rt_handle,
						vfs_metrics,
					)
					.await?;
					self.worker_fatal_reported.store(false, Ordering::Release);
					self.start_worker_failure_monitor(native_db.clone(), config);
					*self.db.lock() = Some(native_db);
					Ok(())
				}

				#[cfg(not(feature = "sqlite-local"))]
				{
					Err(SqliteRuntimeError::Unavailable.build())
				}
			}
			SqliteBackend::RemoteEnvoy => {
				self.remote_config()?;
				Ok(())
			}
			SqliteBackend::Unavailable => Err(SqliteRuntimeError::Unavailable.build()),
		}
	}

	#[cfg(feature = "sqlite-local")]
	async fn local_exec(&self, sql: String) -> Result<QueryResult> {
		self.open().await?;
		self.map_local_worker_result(self.native_db_handle()?.exec(sql).await)
	}

	#[cfg(not(feature = "sqlite-local"))]
	async fn local_exec(&self, _sql: String) -> Result<QueryResult> {
		Err(SqliteRuntimeError::Unavailable.build())
	}

	#[cfg(feature = "sqlite-local")]
	async fn local_query(
		&self,
		sql: String,
		params: Option<Vec<BindParam>>,
	) -> Result<QueryResult> {
		self.open().await?;
		self.map_local_worker_result(self.native_db_handle()?.query(sql, params).await)
	}

	#[cfg(not(feature = "sqlite-local"))]
	async fn local_query(
		&self,
		_sql: String,
		_params: Option<Vec<BindParam>>,
	) -> Result<QueryResult> {
		Err(SqliteRuntimeError::Unavailable.build())
	}

	#[cfg(feature = "sqlite-local")]
	async fn local_run(&self, sql: String, params: Option<Vec<BindParam>>) -> Result<ExecResult> {
		self.open().await?;
		self.map_local_worker_result(self.native_db_handle()?.run(sql, params).await)
	}

	#[cfg(not(feature = "sqlite-local"))]
	async fn local_run(&self, _sql: String, _params: Option<Vec<BindParam>>) -> Result<ExecResult> {
		Err(SqliteRuntimeError::Unavailable.build())
	}

	#[cfg(feature = "sqlite-local")]
	async fn local_execute(
		&self,
		sql: String,
		params: Option<Vec<BindParam>>,
	) -> Result<ExecuteResult> {
		self.open().await?;
		self.map_local_worker_result(self.native_db_handle()?.execute(sql, params).await)
	}

	#[cfg(not(feature = "sqlite-local"))]
	async fn local_execute(
		&self,
		_sql: String,
		_params: Option<Vec<BindParam>>,
	) -> Result<ExecuteResult> {
		Err(SqliteRuntimeError::Unavailable.build())
	}

	pub async fn exec(&self, sql: impl Into<String>) -> Result<QueryResult> {
		let sql = sql.into();
		match self.backend {
			SqliteBackend::LocalNative => self.local_exec(sql).await,
			SqliteBackend::RemoteEnvoy => self.remote_exec(sql).await,
			SqliteBackend::Unavailable => Err(SqliteRuntimeError::Unavailable.build()),
		}
	}

	pub async fn query(
		&self,
		sql: impl Into<String>,
		params: Option<Vec<BindParam>>,
	) -> Result<QueryResult> {
		let sql = sql.into();
		match self.backend {
			SqliteBackend::LocalNative => self.local_query(sql, params).await,
			SqliteBackend::RemoteEnvoy => {
				Ok(self.remote_execute(sql, params).await?.into_query_result())
			}
			SqliteBackend::Unavailable => Err(SqliteRuntimeError::Unavailable.build()),
		}
	}

	pub async fn run(
		&self,
		sql: impl Into<String>,
		params: Option<Vec<BindParam>>,
	) -> Result<ExecResult> {
		let sql = sql.into();
		match self.backend {
			SqliteBackend::LocalNative => self.local_run(sql, params).await,
			SqliteBackend::RemoteEnvoy => {
				Ok(self.remote_execute(sql, params).await?.into_exec_result())
			}
			SqliteBackend::Unavailable => Err(SqliteRuntimeError::Unavailable.build()),
		}
	}

	pub async fn execute(
		&self,
		sql: impl Into<String>,
		params: Option<Vec<BindParam>>,
	) -> Result<ExecuteResult> {
		let sql = sql.into();
		match self.backend {
			SqliteBackend::LocalNative => self.local_execute(sql, params).await,
			SqliteBackend::RemoteEnvoy => self.remote_execute(sql, params).await,
			SqliteBackend::Unavailable => Err(SqliteRuntimeError::Unavailable.build()),
		}
	}

	pub async fn close(&self) -> Result<()> {
		match self.backend {
			SqliteBackend::LocalNative => {
				#[cfg(feature = "sqlite-local")]
				{
					let native_db = self.db.lock().take();
					if let Some(native_db) = native_db {
						let result = self.map_local_worker_result(native_db.close().await);
						self.abort_worker_failure_monitor();
						result?;
					}
				}
				Ok(())
			}
			SqliteBackend::RemoteEnvoy | SqliteBackend::Unavailable => Ok(()),
		}
	}

	pub(crate) async fn cleanup(&self) -> Result<()> {
		self.close().await
	}

	pub fn take_last_kv_error(&self) -> Option<String> {
		if self.backend != SqliteBackend::LocalNative {
			return None;
		}

		#[cfg(feature = "sqlite-local")]
		{
			return self
				.db
				.lock()
				.as_ref()
				.and_then(NativeDatabaseHandle::take_last_kv_error);
		}

		#[cfg(not(feature = "sqlite-local"))]
		None
	}

	#[cfg(feature = "sqlite-local")]
	fn native_db_handle(&self) -> Result<NativeDatabaseHandle> {
		self.db
			.lock()
			.as_ref()
			.cloned()
			.ok_or_else(|| SqliteRuntimeError::Closed.build())
	}

	#[cfg(feature = "sqlite-local")]
	fn map_local_worker_result<T>(&self, result: Result<T>) -> Result<T> {
		match result {
			Ok(value) => Ok(value),
			Err(error) => {
				if is_fatal_worker_error(&error) {
					self.report_worker_fatal(&error);
				}
				Err(map_local_worker_error(error))
			}
		}
	}

	#[cfg(feature = "sqlite-local")]
	fn report_worker_fatal(&self, error: &anyhow::Error) {
		let Ok(config) = self.runtime_config() else {
			return;
		};
		report_sqlite_worker_fatal(
			&self.worker_fatal_reported,
			config,
			format!("sqlite worker failed: {error}"),
		);
	}

	#[cfg(feature = "sqlite-local")]
	fn start_worker_failure_monitor(
		&self,
		native_db: NativeDatabaseHandle,
		config: SqliteRuntimeConfig,
	) {
		self.abort_worker_failure_monitor();
		let reported = Arc::clone(&self.worker_fatal_reported);
		let task = RuntimeSpawner::spawn(async move {
			if native_db.wait_for_worker_failure().await {
				report_sqlite_worker_fatal(
					&reported,
					config,
					"sqlite worker thread stopped unexpectedly".to_string(),
				);
			}
		});
		*self.worker_failure_task.lock() = Some(task);
	}

	#[cfg(feature = "sqlite-local")]
	fn abort_worker_failure_monitor(&self) {
		if let Some(task) = self.worker_failure_task.lock().take() {
			task.abort();
		}
	}

	pub fn metrics(&self) -> Option<SqliteVfsMetricsSnapshot> {
		#[cfg(feature = "sqlite-local")]
		{
			self.db
				.lock()
				.as_ref()
				.map(NativeDatabaseHandle::sqlite_vfs_metrics)
		}

		#[cfg(not(feature = "sqlite-local"))]
		{
			None
		}
	}

	pub fn runtime_config(&self) -> Result<SqliteRuntimeConfig> {
		Ok(SqliteRuntimeConfig {
			handle: self.handle()?,
			actor_id: self
				.actor_id
				.clone()
				.ok_or_else(|| sqlite_not_configured("actor id"))?,
			generation: self.generation,
		})
	}

	fn remote_config(&self) -> Result<RemoteSqliteConfig> {
		let config = self.runtime_config()?;
		let generation = config
			.generation
			.ok_or_else(|| sqlite_not_configured("generation"))?;
		Ok(RemoteSqliteConfig {
			namespace_id: config.handle.namespace().to_owned(),
			handle: config.handle,
			actor_id: config.actor_id,
			generation,
		})
	}

	async fn remote_exec(&self, sql: String) -> Result<QueryResult> {
		let config = self.remote_config()?;
		let response = config
			.handle
			.remote_sqlite_exec(protocol::SqliteExecRequest {
				namespace_id: config.namespace_id,
				actor_id: config.actor_id,
				generation: config.generation,
				sql,
			})
			.await
			.map_err(remote_request_error)?;

		match response {
			protocol::SqliteExecResponse::SqliteExecOk(ok) => {
				Ok(query_result_from_protocol(ok.result))
			}
			protocol::SqliteExecResponse::SqliteErrorResponse(error) => {
				Err(remote_sqlite_error_response(error.message))
			}
		}
	}

	async fn remote_execute(
		&self,
		sql: String,
		params: Option<Vec<BindParam>>,
	) -> Result<ExecuteResult> {
		let config = self.remote_config()?;
		let response = config
			.handle
			.remote_sqlite_execute(protocol::SqliteExecuteRequest {
				namespace_id: config.namespace_id,
				actor_id: config.actor_id,
				generation: config.generation,
				sql,
				params: params.map(protocol_bind_params),
			})
			.await
			.map_err(remote_request_error)?;

		match response {
			protocol::SqliteExecuteResponse::SqliteExecuteOk(ok) => {
				Ok(execute_result_from_protocol(ok.result))
			}
			protocol::SqliteExecuteResponse::SqliteErrorResponse(error) => {
				Err(remote_sqlite_error_response(error.message))
			}
		}
	}

	pub(crate) async fn query_rows_cbor(
		&self,
		sql: &str,
		params: Option<&[u8]>,
	) -> Result<Vec<u8>> {
		let bind_params = bind_params_from_cbor(sql, params)?;
		let result = self.query(sql.to_owned(), bind_params).await?;
		encode_json_as_cbor(&query_result_to_json_rows(&result))
	}

	pub(crate) async fn exec_rows_cbor(&self, sql: &str) -> Result<Vec<u8>> {
		let result = self.exec(sql.to_owned()).await?;
		encode_json_as_cbor(&query_result_to_json_rows(&result))
	}

	pub(crate) async fn run_cbor(&self, sql: &str, params: Option<&[u8]>) -> Result<ExecResult> {
		let bind_params = bind_params_from_cbor(sql, params)?;
		self.run(sql.to_owned(), bind_params).await
	}

	pub(crate) async fn execute_rows_cbor(
		&self,
		sql: &str,
		params: Option<&[u8]>,
	) -> Result<Vec<u8>> {
		let bind_params = bind_params_from_cbor(sql, params)?;
		let result = self.execute(sql.to_owned(), bind_params).await?;
		encode_json_as_cbor(&query_result_to_json_rows(&QueryResult {
			columns: result.columns,
			rows: result.rows,
		}))
	}

	fn handle(&self) -> Result<EnvoyHandle> {
		self.handle
			.clone()
			.ok_or_else(|| sqlite_not_configured("handle"))
	}
}

#[cfg(feature = "sqlite-local")]
fn report_sqlite_worker_fatal(reported: &AtomicBool, config: SqliteRuntimeConfig, message: String) {
	if reported.swap(true, Ordering::AcqRel) {
		return;
	}
	// A dead worker means SQLite's sole native connection is no longer a valid
	// actor subsystem. Core reports that through envoy lifecycle instead of
	// letting the actor continue to serve requests with a broken database.
	config.handle.stop_actor(
		config.actor_id,
		config
			.generation
			.and_then(|generation| generation.try_into().ok()),
		Some(message),
	);
}

struct RemoteSqliteConfig {
	handle: EnvoyHandle,
	namespace_id: String,
	actor_id: String,
	generation: u64,
}

fn select_sqlite_backend(enabled: bool, remote_sqlite: bool) -> SqliteBackend {
	if enabled && remote_sqlite {
		return SqliteBackend::RemoteEnvoy;
	}

	#[cfg(feature = "sqlite-local")]
	{
		SqliteBackend::LocalNative
	}

	#[cfg(not(feature = "sqlite-local"))]
	{
		SqliteBackend::Unavailable
	}
}

#[cfg(feature = "sqlite-local")]
fn is_fatal_worker_error(error: &anyhow::Error) -> bool {
	error.downcast_ref::<SqliteWorkerDeadError>().is_some()
		|| error
			.downcast_ref::<SqliteWorkerCloseTimeoutError>()
			.is_some()
}

#[cfg(feature = "sqlite-local")]
fn map_local_worker_error(error: anyhow::Error) -> anyhow::Error {
	if error
		.downcast_ref::<SqliteWorkerOverloadedError>()
		.is_some()
	{
		return ActorLifecycle::Overloaded {
			channel: "sqlite_worker".to_string(),
			capacity: SQLITE_WORKER_QUEUE_CAPACITY,
			operation: "execute sqlite command".to_string(),
		}
		.build();
	}

	if error.downcast_ref::<SqliteWorkerClosingError>().is_some()
		|| error.downcast_ref::<SqliteWorkerDeadError>().is_some()
	{
		return SqliteRuntimeError::Closed.build();
	}

	error
}

fn protocol_bind_params(params: Vec<BindParam>) -> Vec<protocol::SqliteBindParam> {
	params.into_iter().map(protocol_bind_param).collect()
}

fn protocol_bind_param(param: BindParam) -> protocol::SqliteBindParam {
	match param {
		BindParam::Null => protocol::SqliteBindParam::SqliteValueNull,
		BindParam::Integer(value) => {
			protocol::SqliteBindParam::SqliteValueInteger(protocol::SqliteValueInteger { value })
		}
		BindParam::Float(value) => {
			protocol::SqliteBindParam::SqliteValueFloat(protocol::SqliteValueFloat {
				value: value.to_bits().to_be_bytes(),
			})
		}
		BindParam::Text(value) => {
			protocol::SqliteBindParam::SqliteValueText(protocol::SqliteValueText { value })
		}
		BindParam::Blob(value) => {
			protocol::SqliteBindParam::SqliteValueBlob(protocol::SqliteValueBlob { value })
		}
	}
}

fn query_result_from_protocol(result: protocol::SqliteQueryResult) -> QueryResult {
	QueryResult {
		columns: result.columns,
		rows: result
			.rows
			.into_iter()
			.map(|row| row.into_iter().map(column_value_from_protocol).collect())
			.collect(),
	}
}

fn execute_result_from_protocol(result: protocol::SqliteExecuteResult) -> ExecuteResult {
	ExecuteResult {
		columns: result.columns,
		rows: result
			.rows
			.into_iter()
			.map(|row| row.into_iter().map(column_value_from_protocol).collect())
			.collect(),
		changes: result.changes,
		last_insert_row_id: result.last_insert_row_id,
	}
}

fn column_value_from_protocol(value: protocol::SqliteColumnValue) -> ColumnValue {
	match value {
		protocol::SqliteColumnValue::SqliteValueNull => ColumnValue::Null,
		protocol::SqliteColumnValue::SqliteValueInteger(value) => ColumnValue::Integer(value.value),
		protocol::SqliteColumnValue::SqliteValueFloat(value) => {
			ColumnValue::Float(f64::from_bits(u64::from_be_bytes(value.value)))
		}
		protocol::SqliteColumnValue::SqliteValueText(value) => ColumnValue::Text(value.value),
		protocol::SqliteColumnValue::SqliteValueBlob(value) => ColumnValue::Blob(value.value),
	}
}

fn remote_request_error(error: anyhow::Error) -> anyhow::Error {
	if let Some(indeterminate) = error.downcast_ref::<RemoteSqliteIndeterminateResultError>() {
		return SqliteRuntimeError::RemoteIndeterminateResult {
			operation: indeterminate.operation.to_owned(),
		}
		.build();
	}

	if let Some(compatibility) =
		error.downcast_ref::<protocol::versioned::ProtocolCompatibilityError>()
	{
		if compatibility.feature
			== protocol::versioned::ProtocolCompatibilityFeature::RemoteSqliteExecution
		{
			return SqliteRuntimeError::RemoteUnavailable {
				reason: compatibility.to_string(),
			}
			.build();
		}
	}

	error
}

fn remote_sqlite_error_response(message: String) -> anyhow::Error {
	if message.contains("unavailable") || message.contains("unsupported") {
		return SqliteRuntimeError::RemoteUnavailable { reason: message }.build();
	}

	SqliteRuntimeError::RemoteExecutionFailed { message }.build()
}
impl std::fmt::Debug for SqliteDb {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("SqliteDb")
			.field("configured", &self.handle.is_some())
			.field("actor_id", &self.actor_id)
			.finish()
	}
}

fn bind_params_from_cbor(sql: &str, params: Option<&[u8]>) -> Result<Option<Vec<BindParam>>> {
	let Some(params) = params else {
		return Ok(None);
	};
	if params.is_empty() {
		return Ok(None);
	}

	let value = ciborium::from_reader::<JsonValue, _>(Cursor::new(params))
		.context("decode sqlite bind params as cbor json")?;
	match value {
		JsonValue::Array(values) => values
			.iter()
			.map(json_to_bind_param)
			.collect::<Result<Vec<_>>>()
			.map(Some),
		JsonValue::Object(properties) => {
			let ordered_names = extract_named_sqlite_parameters(sql);
			if ordered_names.is_empty() {
				return properties
					.values()
					.map(json_to_bind_param)
					.collect::<Result<Vec<_>>>()
					.map(Some);
			}

			ordered_names
				.iter()
				.map(|name| {
					get_named_sqlite_binding(&properties, name)
						.ok_or_else(|| {
							SqliteRuntimeError::InvalidBindParameter {
								name: name.clone(),
								reason: "missing parameter".to_owned(),
							}
							.build()
						})
						.and_then(json_to_bind_param)
				})
				.collect::<Result<Vec<_>>>()
				.map(Some)
		}
		JsonValue::Null => Ok(None),
		other => Err(SqliteRuntimeError::InvalidBindParameter {
			name: "params".to_owned(),
			reason: format!("expected array or object, got {}", json_type_name(&other)),
		}
		.build()),
	}
}

fn json_to_bind_param(value: &JsonValue) -> Result<BindParam> {
	match value {
		JsonValue::Null => Ok(BindParam::Null),
		JsonValue::Bool(value) => Ok(BindParam::Integer(i64::from(*value))),
		JsonValue::Number(value) => {
			if let Some(value) = value.as_i64() {
				return Ok(BindParam::Integer(value));
			}
			if let Some(value) = value.as_u64() {
				let value = i64::try_from(value)
					.context("sqlite integer bind parameter exceeds i64 range")?;
				return Ok(BindParam::Integer(value));
			}
			value.as_f64().map(BindParam::Float).ok_or_else(|| {
				SqliteRuntimeError::InvalidBindParameter {
					name: "number".to_owned(),
					reason: "unsupported numeric value".to_owned(),
				}
				.build()
			})
		}
		JsonValue::String(value) => Ok(BindParam::Text(value.clone())),
		other => Err(SqliteRuntimeError::InvalidBindParameter {
			name: "value".to_owned(),
			reason: format!("unsupported type {}", json_type_name(other)),
		}
		.build()),
	}
}

fn sqlite_not_configured(component: &str) -> anyhow::Error {
	SqliteRuntimeError::NotConfigured {
		component: component.to_owned(),
	}
	.build()
}

fn extract_named_sqlite_parameters(sql: &str) -> Vec<String> {
	let mut ordered_names = Vec::new();
	let mut seen = HashSet::new();
	let bytes = sql.as_bytes();
	let mut idx = 0;

	while idx < bytes.len() {
		let byte = bytes[idx];
		if !matches!(byte, b':' | b'@' | b'$') {
			idx += 1;
			continue;
		}

		let start = idx;
		idx += 1;
		if idx >= bytes.len() || !is_sqlite_param_start(bytes[idx]) {
			continue;
		}
		idx += 1;
		while idx < bytes.len() && is_sqlite_param_continue(bytes[idx]) {
			idx += 1;
		}

		let name = &sql[start..idx];
		if seen.insert(name.to_owned()) {
			ordered_names.push(name.to_owned());
		}
	}

	ordered_names
}

fn is_sqlite_param_start(byte: u8) -> bool {
	byte == b'_' || byte.is_ascii_alphabetic()
}

fn is_sqlite_param_continue(byte: u8) -> bool {
	byte == b'_' || byte.is_ascii_alphanumeric()
}

fn get_named_sqlite_binding<'a>(
	bindings: &'a JsonMap<String, JsonValue>,
	name: &str,
) -> Option<&'a JsonValue> {
	if let Some(value) = bindings.get(name) {
		return Some(value);
	}

	let bare_name = name.get(1..)?;
	if let Some(value) = bindings.get(bare_name) {
		return Some(value);
	}

	for prefix in [":", "@", "$"] {
		let candidate = format!("{prefix}{bare_name}");
		if let Some(value) = bindings.get(&candidate) {
			return Some(value);
		}
	}

	None
}

fn query_result_to_json_rows(result: &QueryResult) -> JsonValue {
	JsonValue::Array(
		result
			.rows
			.iter()
			.map(|row| {
				let mut object = JsonMap::new();
				for (index, column) in result.columns.iter().enumerate() {
					let value = row
						.get(index)
						.map(column_value_to_json)
						.unwrap_or(JsonValue::Null);
					object.insert(column.clone(), value);
				}
				JsonValue::Object(object)
			})
			.collect(),
	)
}

fn column_value_to_json(value: &ColumnValue) -> JsonValue {
	match value {
		ColumnValue::Null => JsonValue::Null,
		ColumnValue::Integer(value) => JsonValue::from(*value),
		ColumnValue::Float(value) => JsonValue::from(*value),
		ColumnValue::Text(value) => JsonValue::String(value.clone()),
		ColumnValue::Blob(value) => {
			JsonValue::Array(value.iter().map(|byte| JsonValue::from(*byte)).collect())
		}
	}
}

fn encode_json_as_cbor(value: &impl Serialize) -> Result<Vec<u8>> {
	let mut encoded = Vec::new();
	ciborium::into_writer(value, &mut encoded).context("encode sqlite rows as cbor")?;
	Ok(encoded)
}

fn json_type_name(value: &JsonValue) -> &'static str {
	match value {
		JsonValue::Null => "null",
		JsonValue::Bool(_) => "boolean",
		JsonValue::Number(_) => "number",
		JsonValue::String(_) => "string",
		JsonValue::Array(_) => "array",
		JsonValue::Object(_) => "object",
	}
}

#[cfg(test)]
#[path = "../../tests/sqlite.rs"]
mod tests;
