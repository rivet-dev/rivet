use napi::bindgen_prelude::Buffer;
use napi_derive::napi;
use rivetkit_core::sqlite::{
	BindParam, ColumnValue, QueryResult as CoreQueryResult, SqliteDb as CoreSqliteDb,
	SqliteRuntimeConfig,
};

use crate::envoy_handle::JsEnvoyHandle;
#[napi]
#[derive(Clone)]
pub struct JsNativeDatabase {
	db: CoreSqliteDb,
}

impl JsNativeDatabase {
	fn new(db: CoreSqliteDb) -> Self {
		Self { db }
	}
}

#[napi(object)]
pub struct JsBindParam {
	pub kind: String,
	pub int_value: Option<i64>,
	pub float_value: Option<f64>,
	pub text_value: Option<String>,
	pub blob_value: Option<Buffer>,
}

#[napi(object)]
pub struct ExecuteResult {
	pub changes: i64,
}

#[napi(object)]
pub struct QueryResult {
	pub columns: Vec<String>,
	pub rows: Vec<Vec<serde_json::Value>>,
}

#[napi(object)]
pub struct JsSqliteVfsMetrics {
	pub request_build_ns: i64,
	pub serialize_ns: i64,
	pub transport_ns: i64,
	pub state_update_ns: i64,
	pub total_ns: i64,
	pub commit_count: i64,
}

#[napi]
impl JsNativeDatabase {
	#[napi]
	pub fn take_last_kv_error(&self) -> Option<String> {
		self.db.take_last_kv_error()
	}

	#[napi]
	pub fn get_sqlite_vfs_metrics(&self) -> Option<JsSqliteVfsMetrics> {
		self.db.metrics().map(|metrics| JsSqliteVfsMetrics {
			request_build_ns: u64_to_i64(metrics.request_build_ns),
			serialize_ns: u64_to_i64(metrics.serialize_ns),
			transport_ns: u64_to_i64(metrics.transport_ns),
			state_update_ns: u64_to_i64(metrics.state_update_ns),
			total_ns: u64_to_i64(metrics.total_ns),
			commit_count: u64_to_i64(metrics.commit_count),
		})
	}

	#[napi]
	pub async fn run(
		&self,
		sql: String,
		params: Option<Vec<JsBindParam>>,
	) -> napi::Result<ExecuteResult> {
		let params = params.map(js_bind_params_to_core).transpose()?;
		let result = self
			.db
			.run(sql, params)
			.await
			.map_err(crate::napi_anyhow_error)?;
		Ok(ExecuteResult {
			changes: result.changes,
		})
	}

	#[napi]
	pub async fn query(
		&self,
		sql: String,
		params: Option<Vec<JsBindParam>>,
	) -> napi::Result<QueryResult> {
		let params = params.map(js_bind_params_to_core).transpose()?;
		let result = self
			.db
			.query(sql, params)
			.await
			.map_err(crate::napi_anyhow_error)?;
		Ok(core_query_result_to_js(result))
	}

	#[napi]
	pub async fn exec(&self, sql: String) -> napi::Result<QueryResult> {
		let result = self.db.exec(sql).await.map_err(crate::napi_anyhow_error)?;
		Ok(core_query_result_to_js(result))
	}

	#[napi]
	pub async fn close(&self) -> napi::Result<()> {
		self.db.close().await.map_err(crate::napi_anyhow_error)
	}
}

fn js_bind_params_to_core(params: Vec<JsBindParam>) -> napi::Result<Vec<BindParam>> {
	params
		.into_iter()
		.map(|param| match param.kind.as_str() {
			"null" => Ok(BindParam::Null),
			"int" => Ok(BindParam::Integer(param.int_value.unwrap_or_default())),
			"float" => Ok(BindParam::Float(param.float_value.unwrap_or_default())),
			"text" => Ok(BindParam::Text(param.text_value.unwrap_or_default())),
			"blob" => Ok(BindParam::Blob(
				param
					.blob_value
					.map(|value| value.as_ref().to_vec())
					.unwrap_or_default(),
			)),
			other => Err(napi::Error::from_reason(format!(
				"unsupported bind param kind: {other}"
			))),
		})
		.collect()
}

fn core_query_result_to_js(result: CoreQueryResult) -> QueryResult {
	QueryResult {
		columns: result.columns,
		rows: result
			.rows
			.into_iter()
			.map(|row| row.into_iter().map(column_value_to_json).collect())
			.collect(),
	}
}

fn column_value_to_json(value: ColumnValue) -> serde_json::Value {
	match value {
		ColumnValue::Null => serde_json::Value::Null,
		ColumnValue::Integer(value) => serde_json::Value::from(value),
		ColumnValue::Float(value) => serde_json::Value::from(value),
		ColumnValue::Text(value) => serde_json::Value::String(value),
		ColumnValue::Blob(value) => {
			serde_json::Value::Array(value.into_iter().map(serde_json::Value::from).collect())
		}
	}
}

fn u64_to_i64(value: u64) -> i64 {
	value.min(i64::MAX as u64) as i64
}

pub(crate) async fn open_database_with_runtime_config(
	config: SqliteRuntimeConfig,
) -> napi::Result<JsNativeDatabase> {
	let SqliteRuntimeConfig {
		handle,
		actor_id,
		startup_data,
	} = config;
	let db = CoreSqliteDb::new(handle, actor_id, startup_data);
	db.open()
		.await
		.map_err(crate::napi_anyhow_error)?;
	Ok(JsNativeDatabase::new(db))
}

/// Open a native SQLite database backed by the envoy's KV channel.
#[napi]
pub async fn open_database_from_envoy(
	js_handle: &JsEnvoyHandle,
	actor_id: String,
) -> napi::Result<JsNativeDatabase> {
	let startup_data = js_handle.clone_sqlite_startup_data(&actor_id).await;

	open_database_with_runtime_config(
		SqliteRuntimeConfig {
			handle: js_handle.handle.clone(),
			actor_id,
			startup_data,
		},
	)
	.await
}
