use std::collections::HashSet;
use std::io::Cursor;
#[cfg(feature = "sqlite")]
use std::sync::Arc;
#[cfg(feature = "sqlite")]
use std::time::Duration;

use anyhow::{Context, Result};
#[cfg(feature = "sqlite")]
use parking_lot::Mutex;
use rivet_envoy_client::handle::EnvoyHandle;
use rivet_envoy_client::protocol;
use serde::Serialize;
use serde_json::{Map as JsonMap, Value as JsonValue};
#[cfg(feature = "sqlite")]
use tokio::task::JoinHandle;
#[cfg(feature = "sqlite")]
use tokio::sync::Mutex as AsyncMutex;
#[cfg(feature = "sqlite")]
use tokio::time::{interval, timeout};
#[cfg(feature = "sqlite")]
use tracing::Instrument;

use crate::error::SqliteRuntimeError;

#[cfg(feature = "sqlite")]
pub use rivetkit_sqlite::query::{
	BindParam, ColumnValue, ExecResult, ExecuteResult, ExecuteRoute, QueryResult,
};
#[cfg(feature = "sqlite")]
use rivetkit_sqlite::{
	database::{NativeDatabaseHandle, open_database_from_envoy},
	optimization_flags::sqlite_optimization_flags,
	vfs::{SqliteVfsMetrics, VfsPreloadHintSnapshot},
};

#[cfg(feature = "sqlite")]
const PRELOAD_HINT_FLUSH_INTERVAL: Duration = Duration::from_secs(30);
#[cfg(feature = "sqlite")]
const PRELOAD_HINT_FLUSH_TIMEOUT: Duration = Duration::from_secs(5);

#[cfg(not(feature = "sqlite"))]
#[derive(Clone, Debug, PartialEq)]
pub enum BindParam {
	Null,
	Integer(i64),
	Float(f64),
	Text(String),
	Blob(Vec<u8>),
}

#[cfg(not(feature = "sqlite"))]
#[derive(Clone, Debug, PartialEq)]
pub struct ExecResult {
	pub changes: i64,
}

#[cfg(not(feature = "sqlite"))]
#[derive(Clone, Debug, PartialEq)]
pub struct QueryResult {
	pub columns: Vec<String>,
	pub rows: Vec<Vec<ColumnValue>>,
}

#[cfg(not(feature = "sqlite"))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExecuteRoute {
	Read,
	Write,
	WriteFallback,
}

#[cfg(not(feature = "sqlite"))]
#[derive(Clone, Debug, PartialEq)]
pub struct ExecuteResult {
	pub columns: Vec<String>,
	pub rows: Vec<Vec<ColumnValue>>,
	pub changes: i64,
	pub last_insert_row_id: Option<i64>,
	pub route: ExecuteRoute,
}

#[cfg(not(feature = "sqlite"))]
#[derive(Clone, Debug, PartialEq)]
pub enum ColumnValue {
	Null,
	Integer(i64),
	Float(f64),
	Text(String),
	Blob(Vec<u8>),
}

#[derive(Clone)]
pub struct SqliteRuntimeConfig {
	pub handle: EnvoyHandle,
	pub actor_id: String,
	pub startup_data: Option<protocol::SqliteStartupData>,
}

#[derive(Clone, Default)]
pub struct SqliteDb {
	handle: Option<EnvoyHandle>,
	actor_id: Option<String>,
	startup_data: Option<protocol::SqliteStartupData>,
	/// Mirrors the user's actor-config `db({...})` declaration. The envoy
	/// always sets up sqlite storage under the hood, so handle/actor_id are
	/// not a reliable signal for whether the user opted in; this flag is.
	enabled: bool,
	#[cfg(feature = "sqlite")]
	// Forced-sync: native SQLite handles are read from synchronous diagnostic
	// accessors and closed from cleanup paths.
	db: Arc<Mutex<Option<NativeDatabaseHandle>>>,
	#[cfg(feature = "sqlite")]
	open_lock: Arc<AsyncMutex<()>>,
	#[cfg(feature = "sqlite")]
	// Forced-sync: the background task is spawned and aborted from sync cleanup
	// paths around the native database handle.
	preload_hint_flush_task: Arc<Mutex<Option<JoinHandle<()>>>>,
	#[cfg(feature = "sqlite")]
	vfs_metrics: Option<Arc<dyn SqliteVfsMetrics>>,
}

impl SqliteDb {
	pub fn new(
		handle: EnvoyHandle,
		actor_id: impl Into<String>,
		startup_data: Option<protocol::SqliteStartupData>,
		enabled: bool,
	) -> Self {
		Self {
			handle: Some(handle),
			actor_id: Some(actor_id.into()),
			startup_data,
			enabled,
			#[cfg(feature = "sqlite")]
			db: Default::default(),
			#[cfg(feature = "sqlite")]
			open_lock: Default::default(),
			#[cfg(feature = "sqlite")]
			preload_hint_flush_task: Default::default(),
			#[cfg(feature = "sqlite")]
			vfs_metrics: None,
		}
	}

	#[cfg(feature = "sqlite")]
	pub(crate) fn set_vfs_metrics(&mut self, metrics: Arc<dyn SqliteVfsMetrics>) {
		self.vfs_metrics = Some(metrics);
	}

	pub fn is_enabled(&self) -> bool {
		self.enabled
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

	pub async fn commit_stage_begin(
		&self,
		request: protocol::SqliteCommitStageBeginRequest,
	) -> Result<protocol::SqliteCommitStageBeginResponse> {
		self.handle()?.sqlite_commit_stage_begin(request).await
	}

	pub async fn commit_stage(
		&self,
		request: protocol::SqliteCommitStageRequest,
	) -> Result<protocol::SqliteCommitStageResponse> {
		self.handle()?.sqlite_commit_stage(request).await
	}

	pub fn commit_stage_fire_and_forget(
		&self,
		request: protocol::SqliteCommitStageRequest,
	) -> Result<()> {
		self.handle()?.sqlite_commit_stage_fire_and_forget(request)
	}

	pub async fn commit_finalize(
		&self,
		request: protocol::SqliteCommitFinalizeRequest,
	) -> Result<protocol::SqliteCommitFinalizeResponse> {
		self.handle()?.sqlite_commit_finalize(request).await
	}

	pub async fn open(&self) -> Result<()> {
		#[cfg(feature = "sqlite")]
		{
			let _open_guard = self.open_lock.lock().await;
			if self.db.lock().is_some() {
				return Ok(());
			}

			let config = self.runtime_config()?;
			let vfs_metrics = self.vfs_metrics.clone();
			let rt_handle = tokio::runtime::Handle::try_current()
				.context("open sqlite database requires a tokio runtime")?;

			let native_db = open_database_from_envoy(
				config.handle,
				config.actor_id,
				config.startup_data,
				rt_handle,
				vfs_metrics,
			)
			.await?;
			*self.db.lock() = Some(native_db);
			self.ensure_preload_hint_flush_task()?;
			Ok(())
		}

		#[cfg(not(feature = "sqlite"))]
		{
			Err(SqliteRuntimeError::Unavailable.build())
		}
	}

	pub async fn exec(&self, sql: impl Into<String>) -> Result<QueryResult> {
		#[cfg(feature = "sqlite")]
		{
			self.open().await?;
			let sql = sql.into();
			self.native_db_handle()?.exec(sql).await
		}

		#[cfg(not(feature = "sqlite"))]
		{
			let _ = sql;
			Err(SqliteRuntimeError::Unavailable.build())
		}
	}

	pub async fn query(
		&self,
		sql: impl Into<String>,
		params: Option<Vec<BindParam>>,
	) -> Result<QueryResult> {
		#[cfg(feature = "sqlite")]
		{
			self.open().await?;
			let sql = sql.into();
			self.native_db_handle()?.query(sql, params).await
		}

		#[cfg(not(feature = "sqlite"))]
		{
			let _ = (sql, params);
			Err(SqliteRuntimeError::Unavailable.build())
		}
	}

	pub async fn run(
		&self,
		sql: impl Into<String>,
		params: Option<Vec<BindParam>>,
	) -> Result<ExecResult> {
		#[cfg(feature = "sqlite")]
		{
			self.open().await?;
			let sql = sql.into();
			self.native_db_handle()?.run(sql, params).await
		}

		#[cfg(not(feature = "sqlite"))]
		{
			let _ = (sql, params);
			Err(SqliteRuntimeError::Unavailable.build())
		}
	}

	pub async fn execute(
		&self,
		sql: impl Into<String>,
		params: Option<Vec<BindParam>>,
	) -> Result<ExecuteResult> {
		#[cfg(feature = "sqlite")]
		{
			self.open().await?;
			let sql = sql.into();
			self.native_db_handle()?.execute(sql, params).await
		}

		#[cfg(not(feature = "sqlite"))]
		{
			let _ = (sql, params);
			Err(SqliteRuntimeError::Unavailable.build())
		}
	}

	pub async fn close(&self) -> Result<()> {
		#[cfg(feature = "sqlite")]
		{
			self.stop_preload_hint_flush_task();
			let native_db = self.db.lock().take();
			if let Some(native_db) = native_db {
				native_db.close().await?;
			}
			Ok(())
		}

		#[cfg(not(feature = "sqlite"))]
		{
			Ok(())
		}
	}

	pub(crate) async fn cleanup(&self) -> Result<()> {
		#[cfg(feature = "sqlite")]
		{
			self.stop_preload_hint_flush_task();
			self.flush_preload_hints_before_close().await;
		}
		self.close().await
	}

	#[cfg(feature = "sqlite")]
	fn ensure_preload_hint_flush_task(&self) -> Result<()> {
		if !sqlite_optimization_flags().preload_hint_flush {
			return Ok(());
		}

		let config = self.runtime_config()?;
		let Some(generation) = config.startup_data.as_ref().map(|data| data.generation) else {
			return Ok(());
		};
		if self.db.lock().is_none() {
			return Ok(());
		}

		let mut task_guard = self.preload_hint_flush_task.lock();
		if task_guard.is_some() {
			return Ok(());
		}

		let db = self.db.clone();
		let handle = config.handle;
		let actor_id = config.actor_id;
		*task_guard = Some(tokio::spawn(
			async move {
				let mut tick = interval(PRELOAD_HINT_FLUSH_INTERVAL);
				tick.tick().await;
				loop {
					tick.tick().await;
					flush_preload_hints_best_effort(
						db.clone(),
						handle.clone(),
						actor_id.clone(),
						generation,
						"periodic",
					)
					.await;
				}
			}
			.in_current_span(),
		));
		Ok(())
	}

	#[cfg(feature = "sqlite")]
	fn stop_preload_hint_flush_task(&self) {
		if let Some(task) = self.preload_hint_flush_task.lock().take() {
			task.abort();
		}
	}

	#[cfg(feature = "sqlite")]
	async fn flush_preload_hints_before_close(&self) {
		if !sqlite_optimization_flags().preload_hint_flush {
			return;
		}

		let Ok(config) = self.runtime_config() else {
			return;
		};
		let Some(generation) = config.startup_data.as_ref().map(|data| data.generation) else {
			return;
		};

		enqueue_preload_hint_flush_best_effort(
			self.db.clone(),
			config.handle,
			config.actor_id,
			generation,
		)
		.await;
	}

	pub fn take_last_kv_error(&self) -> Option<String> {
		#[cfg(feature = "sqlite")]
		{
			self.db
				.lock()
				.as_ref()
				.and_then(NativeDatabaseHandle::take_last_kv_error)
		}

		#[cfg(not(feature = "sqlite"))]
		{
			None
		}
	}

	#[cfg(feature = "sqlite")]
	fn native_db_handle(&self) -> Result<NativeDatabaseHandle> {
		self.db
			.lock()
			.as_ref()
			.cloned()
			.ok_or_else(|| SqliteRuntimeError::Closed.build())
	}

	pub fn runtime_config(&self) -> Result<SqliteRuntimeConfig> {
		Ok(SqliteRuntimeConfig {
			handle: self.handle()?,
			actor_id: self
				.actor_id
				.clone()
				.ok_or_else(|| sqlite_not_configured("actor id"))?,
			startup_data: self.startup_data.clone(),
		})
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

#[cfg(feature = "sqlite")]
async fn enqueue_preload_hint_flush_best_effort(
	db: Arc<Mutex<Option<NativeDatabaseHandle>>>,
	handle: EnvoyHandle,
	actor_id: String,
	generation: u64,
) {
	let snapshot = match snapshot_preload_hints(db).await {
		Ok(Some(snapshot)) => snapshot,
		Ok(None) => return,
		Err(error) => {
			tracing::warn!(
				actor_id = %actor_id,
				?error,
				reason = "shutdown",
				"sqlite preload hint snapshot failed"
			);
			return;
		}
	};
	if snapshot.pgnos.is_empty() && snapshot.ranges.is_empty() {
		return;
	}

	let hint_count = snapshot.pgnos.len() + snapshot.ranges.len();
	let request = protocol::SqlitePersistPreloadHintsRequest {
		actor_id: actor_id.clone(),
		generation,
		hints: protocol_preload_hints(snapshot),
	};
	match handle.sqlite_persist_preload_hints_fire_and_forget(request) {
		Ok(()) => {
			tracing::debug!(
				actor_id = %actor_id,
				generation,
				reason = "shutdown",
				hint_count,
				"sqlite preload hint flush queued"
			);
		}
		Err(error) => {
			tracing::warn!(
				actor_id = %actor_id,
				generation,
				reason = "shutdown",
				hint_count,
				?error,
				"sqlite preload hint flush queue failed"
			);
		}
	}
}

#[cfg(feature = "sqlite")]
async fn flush_preload_hints_best_effort(
	db: Arc<Mutex<Option<NativeDatabaseHandle>>>,
	handle: EnvoyHandle,
	actor_id: String,
	generation: u64,
	reason: &'static str,
) {
	let snapshot = match snapshot_preload_hints(db).await {
		Ok(Some(snapshot)) => snapshot,
		Ok(None) => return,
		Err(error) => {
			tracing::warn!(
				actor_id = %actor_id,
				?error,
				reason,
				"sqlite preload hint snapshot failed"
			);
			return;
		}
	};
	if snapshot.pgnos.is_empty() && snapshot.ranges.is_empty() {
		return;
	}

	let hint_count = snapshot.pgnos.len() + snapshot.ranges.len();
	let request = protocol::SqlitePersistPreloadHintsRequest {
		actor_id: actor_id.clone(),
		generation,
		hints: protocol_preload_hints(snapshot),
	};
	let response = timeout(
		PRELOAD_HINT_FLUSH_TIMEOUT,
		handle.sqlite_persist_preload_hints(request),
	)
	.await;
	match response {
		Ok(Ok(protocol::SqlitePersistPreloadHintsResponse::SqlitePersistPreloadHintsOk)) => {
			tracing::debug!(
				actor_id = %actor_id,
				generation,
				reason,
				hint_count,
				"sqlite preload hints flushed"
			);
		}
		Ok(Ok(protocol::SqlitePersistPreloadHintsResponse::SqliteFenceMismatch(mismatch))) => {
			tracing::debug!(
				actor_id = %actor_id,
				generation,
				reason,
				hint_count,
				fence_reason = %mismatch.reason,
				"sqlite preload hint flush skipped after fence mismatch"
			);
		}
		Ok(Ok(protocol::SqlitePersistPreloadHintsResponse::SqliteErrorResponse(error))) => {
			tracing::warn!(
				actor_id = %actor_id,
				generation,
				reason,
				hint_count,
				error = %error.message,
				"sqlite preload hint flush failed"
			);
		}
		Ok(Err(error)) => {
			tracing::warn!(
				actor_id = %actor_id,
				generation,
				reason,
				hint_count,
				?error,
				"sqlite preload hint flush failed"
			);
		}
		Err(_) => {
			tracing::warn!(
				actor_id = %actor_id,
				generation,
				reason,
				hint_count,
				timeout_ms = PRELOAD_HINT_FLUSH_TIMEOUT.as_millis() as u64,
				"sqlite preload hint flush timed out"
			);
		}
	}
}

#[cfg(feature = "sqlite")]
async fn snapshot_preload_hints(
	db: Arc<Mutex<Option<NativeDatabaseHandle>>>,
) -> Result<Option<VfsPreloadHintSnapshot>> {
	tokio::task::spawn_blocking(move || {
		let guard = db.lock();
		Ok(guard.as_ref().map(NativeDatabaseHandle::snapshot_preload_hints))
	})
	.await
	.context("join sqlite preload hint snapshot task")?
}

#[cfg(feature = "sqlite")]
fn protocol_preload_hints(snapshot: VfsPreloadHintSnapshot) -> protocol::SqlitePreloadHints {
	protocol::SqlitePreloadHints {
		pgnos: snapshot.pgnos,
		ranges: snapshot
			.ranges
			.into_iter()
			.map(|range| protocol::SqlitePreloadHintRange {
				start_pgno: range.start_pgno,
				page_count: range.page_count,
			})
			.collect(),
	}
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
