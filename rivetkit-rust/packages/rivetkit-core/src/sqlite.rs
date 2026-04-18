use std::collections::HashSet;
use std::io::Cursor;

use anyhow::{Context, Result, anyhow, bail};
use rivet_envoy_client::handle::EnvoyHandle;
use rivet_envoy_client::protocol;
use serde::Serialize;
use serde_json::{Map as JsonMap, Value as JsonValue};

#[cfg(feature = "sqlite")]
pub use rivetkit_sqlite::query::{BindParam, ColumnValue, ExecResult, QueryResult};
#[cfg(feature = "sqlite")]
use rivetkit_sqlite::{
	database::{NativeDatabaseHandle, open_database_from_envoy},
	query::{exec_statements, execute_statement, query_statement},
	v2::vfs::SqliteVfsMetricsSnapshot,
};

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
#[derive(Clone, Debug, PartialEq)]
pub enum ColumnValue {
	Null,
	Integer(i64),
	Float(f64),
	Text(String),
	Blob(Vec<u8>),
}

#[cfg(not(feature = "sqlite"))]
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
	pub schema_version: u32,
	pub startup_data: Option<protocol::SqliteStartupData>,
}

#[derive(Clone, Default)]
pub struct SqliteDb {
	handle: Option<EnvoyHandle>,
	actor_id: Option<String>,
	schema_version: Option<u32>,
	startup_data: Option<protocol::SqliteStartupData>,
	#[cfg(feature = "sqlite")]
	db: std::sync::Arc<std::sync::Mutex<Option<NativeDatabaseHandle>>>,
}

impl SqliteDb {
	pub fn new(
		handle: EnvoyHandle,
		actor_id: impl Into<String>,
		schema_version: u32,
		startup_data: Option<protocol::SqliteStartupData>,
	) -> Self {
		Self {
			handle: Some(handle),
			actor_id: Some(actor_id.into()),
			schema_version: Some(schema_version),
			startup_data,
			#[cfg(feature = "sqlite")]
			db: Default::default(),
		}
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

	pub async fn open(&self, preloaded_entries: Vec<(Vec<u8>, Vec<u8>)>) -> Result<()> {
		#[cfg(feature = "sqlite")]
		{
			let config = self.runtime_config()?;
			let db = self.db.clone();
			let rt_handle = tokio::runtime::Handle::try_current()
				.context("open sqlite database requires a tokio runtime")?;

			tokio::task::spawn_blocking(move || {
				let mut guard = db
					.lock()
					.map_err(|_| anyhow!("sqlite database mutex poisoned"))?;
				if guard.is_some() {
					return Ok(());
				}

				let native_db = open_database_from_envoy(
					config.handle,
					config.actor_id,
					config.schema_version,
					config.startup_data,
					preloaded_entries,
					rt_handle,
				)?;
				*guard = Some(native_db);
				Ok(())
			})
			.await
			.context("join sqlite open task")?
		}

		#[cfg(not(feature = "sqlite"))]
		{
			let _ = preloaded_entries;
			bail!("actor database is not available because rivetkit-core was built without the `sqlite` feature")
		}
	}

	pub async fn exec(&self, sql: impl Into<String>) -> Result<QueryResult> {
		#[cfg(feature = "sqlite")]
		{
			self.open(Vec::new()).await?;
			let sql = sql.into();
			let db = self.db.clone();
			tokio::task::spawn_blocking(move || {
				let guard = db
					.lock()
					.map_err(|_| anyhow!("sqlite database mutex poisoned"))?;
				let native_db = guard
					.as_ref()
					.ok_or_else(|| anyhow!("sqlite database is closed"))?;
				exec_statements(native_db.as_ptr(), &sql)
			})
			.await
			.context("join sqlite exec task")?
		}

		#[cfg(not(feature = "sqlite"))]
		{
			let _ = sql;
			bail!("actor database is not available because rivetkit-core was built without the `sqlite` feature")
		}
	}

	pub async fn query(
		&self,
		sql: impl Into<String>,
		params: Option<Vec<BindParam>>,
	) -> Result<QueryResult> {
		#[cfg(feature = "sqlite")]
		{
			self.open(Vec::new()).await?;
			let sql = sql.into();
			let db = self.db.clone();
			tokio::task::spawn_blocking(move || {
				let guard = db
					.lock()
					.map_err(|_| anyhow!("sqlite database mutex poisoned"))?;
				let native_db = guard
					.as_ref()
					.ok_or_else(|| anyhow!("sqlite database is closed"))?;
				query_statement(native_db.as_ptr(), &sql, params.as_deref())
			})
			.await
			.context("join sqlite query task")?
		}

		#[cfg(not(feature = "sqlite"))]
		{
			let _ = (sql, params);
			bail!("actor database is not available because rivetkit-core was built without the `sqlite` feature")
		}
	}

	pub async fn run(
		&self,
		sql: impl Into<String>,
		params: Option<Vec<BindParam>>,
	) -> Result<ExecResult> {
		#[cfg(feature = "sqlite")]
		{
			self.open(Vec::new()).await?;
			let sql = sql.into();
			let db = self.db.clone();
			tokio::task::spawn_blocking(move || {
				let guard = db
					.lock()
					.map_err(|_| anyhow!("sqlite database mutex poisoned"))?;
				let native_db = guard
					.as_ref()
					.ok_or_else(|| anyhow!("sqlite database is closed"))?;
				execute_statement(native_db.as_ptr(), &sql, params.as_deref())
			})
			.await
			.context("join sqlite run task")?
		}

		#[cfg(not(feature = "sqlite"))]
		{
			let _ = (sql, params);
			bail!("actor database is not available because rivetkit-core was built without the `sqlite` feature")
		}
	}

	pub async fn close(&self) -> Result<()> {
		#[cfg(feature = "sqlite")]
		{
			let db = self.db.clone();
			tokio::task::spawn_blocking(move || {
				let mut guard = db
					.lock()
					.map_err(|_| anyhow!("sqlite database mutex poisoned"))?;
				guard.take();
				Ok(())
			})
			.await
			.context("join sqlite close task")?
		}

		#[cfg(not(feature = "sqlite"))]
		{
			Ok(())
		}
	}

	pub(crate) async fn cleanup(&self) -> Result<()> {
		self.close().await
	}

	pub fn take_last_kv_error(&self) -> Option<String> {
		#[cfg(feature = "sqlite")]
		{
			self.db.lock().ok().and_then(|guard| {
				guard
					.as_ref()
					.and_then(NativeDatabaseHandle::take_last_kv_error)
			})
		}

		#[cfg(not(feature = "sqlite"))]
		{
			None
		}
	}

	pub fn metrics(&self) -> Option<SqliteVfsMetricsSnapshot> {
		#[cfg(feature = "sqlite")]
		{
			self.db.lock().ok().and_then(|guard| {
				guard
					.as_ref()
					.and_then(NativeDatabaseHandle::sqlite_vfs_metrics)
			})
		}

		#[cfg(not(feature = "sqlite"))]
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
				.ok_or_else(|| anyhow!("sqlite actor id is not configured"))?,
			schema_version: self
				.schema_version
				.ok_or_else(|| anyhow!("sqlite schema version is not configured"))?,
			startup_data: self.startup_data.clone(),
		})
	}

	pub(crate) async fn query_rows_cbor(&self, sql: &str, params: Option<&[u8]>) -> Result<Vec<u8>> {
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

	fn handle(&self) -> Result<EnvoyHandle> {
		self.handle
			.clone()
			.ok_or_else(|| anyhow!("sqlite handle is not configured"))
	}
}

impl std::fmt::Debug for SqliteDb {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("SqliteDb")
			.field("configured", &self.handle.is_some())
			.field("actor_id", &self.actor_id)
			.field("schema_version", &self.schema_version)
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
						.ok_or_else(|| anyhow!("missing bind parameter: {name}"))
						.and_then(json_to_bind_param)
				})
				.collect::<Result<Vec<_>>>()
				.map(Some)
		}
		JsonValue::Null => Ok(None),
		other => bail!(
			"sqlite bind params must be an array or object, got {}",
			json_type_name(&other)
		),
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
			value
				.as_f64()
				.map(BindParam::Float)
				.ok_or_else(|| anyhow!("unsupported sqlite number bind parameter"))
		}
		JsonValue::String(value) => Ok(BindParam::Text(value.clone())),
		other => bail!(
			"unsupported sqlite bind parameter type: {}",
			json_type_name(other)
		),
	}
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
		ColumnValue::Blob(value) => JsonValue::Array(
			value
				.iter()
				.map(|byte| JsonValue::from(*byte))
				.collect(),
		),
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
