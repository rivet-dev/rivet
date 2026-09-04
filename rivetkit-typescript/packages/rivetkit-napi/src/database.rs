use std::{future::Future, time::Duration};

use crate::actor_context::{StateDeltaPayload, state_deltas_from_payload};
use napi::JsObject;
use napi::bindgen_prelude::{Buffer, Env};
use napi_derive::napi;
use rivetkit_core::ActorStateTransaction as CoreActorStateTransaction;
use rivetkit_core::sqlite::{
	BindParam, CallMode, ColumnValue, ExecuteResult as CoreExecuteResult,
	QueryResult as CoreQueryResult, SqliteBatchStatement as CoreSqliteBatchStatement,
	SqliteDb as CoreSqliteDb, SqliteTransaction as CoreSqliteTransaction, TransactionOrigin,
};

use crate::{NapiInvalidArgument, napi_anyhow_error};
#[napi]
#[derive(Clone)]
pub struct JsNativeDatabase {
	db: CoreSqliteDb,
	actor_id: Option<String>,
}

#[napi]
#[derive(Clone)]
pub struct JsSqliteTransaction {
	transaction: CoreSqliteTransaction,
}
#[napi]
#[derive(Clone)]
pub struct JsActorStateTransaction {
	transaction: CoreActorStateTransaction,
}

impl JsActorStateTransaction {
	pub(crate) fn new(transaction: CoreActorStateTransaction) -> Self {
		Self { transaction }
	}
}

impl JsNativeDatabase {
	pub(crate) fn new(db: CoreSqliteDb, actor_id: Option<String>) -> Self {
		tracing::debug!(
			class = "JsNativeDatabase",
			actor_id = actor_id.as_deref().unwrap_or("<unknown>"),
			"constructed napi class"
		);
		Self { db, actor_id }
	}
}

impl Drop for JsNativeDatabase {
	fn drop(&mut self) {
		tracing::debug!(
			class = "JsNativeDatabase",
			actor_id = self.actor_id.as_deref().unwrap_or("<unknown>"),
			"dropped napi class"
		);
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
	pub readonly: Option<bool>,
}

#[napi(object)]
pub struct NativeExecuteResult {
	pub columns: Vec<String>,
	pub rows: Vec<Vec<serde_json::Value>>,
	pub changes: i64,
	pub last_insert_row_id: Option<i64>,
	pub readonly: Option<bool>,
	pub commit_seq: Option<f64>,
}

#[napi(object)]
pub struct JsSqliteBatchStatement {
	pub sql: String,
	pub params: Option<Vec<JsBindParam>>,
}

#[napi(object)]
pub struct JsSqliteVfsMetrics {
	pub request_build_ns: f64,
	pub serialize_ns: f64,
	pub transport_ns: f64,
	pub state_update_ns: f64,
	pub total_ns: f64,
	pub commit_count: f64,
	pub page_cache_entries: f64,
	pub page_cache_weighted_size: f64,
	pub page_cache_capacity_pages: f64,
	pub write_buffer_dirty_pages: f64,
	pub db_size_pages: f64,
}

#[napi]
impl JsNativeDatabase {
	#[napi]
	pub fn take_last_kv_error(&self) -> Option<String> {
		self.db.take_last_kv_error()
	}

	#[napi]
	pub fn metrics(&self) -> Option<JsSqliteVfsMetrics> {
		self.db.metrics().map(|metrics| JsSqliteVfsMetrics {
			request_build_ns: metrics.request_build_ns as f64,
			serialize_ns: metrics.serialize_ns as f64,
			transport_ns: metrics.transport_ns as f64,
			state_update_ns: metrics.state_update_ns as f64,
			total_ns: metrics.total_ns as f64,
			commit_count: metrics.commit_count as f64,
			page_cache_entries: metrics.page_cache_entries as f64,
			page_cache_weighted_size: metrics.page_cache_weighted_size as f64,
			page_cache_capacity_pages: metrics.page_cache_capacity_pages as f64,
			write_buffer_dirty_pages: metrics.write_buffer_dirty_pages as f64,
			db_size_pages: metrics.db_size_pages as f64,
		})
	}

	#[napi]
	pub fn commit_seq(&self) -> f64 {
		self.db.commit_seq() as f64
	}

	#[napi]
	pub fn flushed_seq(&self) -> f64 {
		self.db.flushed_seq() as f64
	}

	#[napi]
	pub fn flush_error(&self) -> Option<String> {
		self.db.flush_error()
	}

	#[napi]
	pub fn supports_sync_metadata(&self) -> bool {
		self.db.backend() == rivetkit_core::sqlite::SqliteBackend::LocalNative
	}

	#[napi]
	pub async fn wait_for_flush(&self, seq: f64) -> napi::Result<()> {
		if !seq.is_finite() || seq < 0.0 || seq.fract() != 0.0 || seq > 9_007_199_254_740_991.0 {
			return Err(napi_anyhow_error(
				NapiInvalidArgument {
					argument: "seq".to_owned(),
					reason: "must be a non-negative safe integer".to_owned(),
				}
				.build(),
			));
		}
		self.db
			.wait_for_flush(seq as u64)
			.await
			.map_err(crate::napi_anyhow_error)
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
	pub async fn execute(
		&self,
		sql: String,
		params: Option<Vec<JsBindParam>>,
	) -> napi::Result<NativeExecuteResult> {
		let params = params.map(js_bind_params_to_core).transpose()?;
		let result = self
			.db
			.execute(sql, params)
			.await
			.map_err(crate::napi_anyhow_error)?;
		Ok(core_execute_result_to_js(result))
	}

	#[napi]
	pub fn execute_sync(
		&self,
		sql: String,
		params: Option<Vec<JsBindParam>>,
	) -> napi::Result<NativeExecuteResult> {
		let params = params.map(js_bind_params_to_core).transpose()?;
		let db = self.db.clone();
		wait_for_runtime(async move {
			db.execute_with_call_mode(sql, params, CallMode::SyncBlocking)
				.await
		})
		.map(core_execute_result_to_js)
	}

	#[napi]
	pub async fn execute_batch(
		&self,
		statements: Vec<JsSqliteBatchStatement>,
	) -> napi::Result<Vec<NativeExecuteResult>> {
		let statements = js_batch_statements_to_core(statements)?;
		let results = self
			.db
			.execute_batch(statements)
			.await
			.map_err(crate::napi_anyhow_error)?;
		Ok(results.into_iter().map(core_execute_result_to_js).collect())
	}

	#[napi]
	pub async fn exec(&self, sql: String) -> napi::Result<QueryResult> {
		let result = self.db.exec(sql).await.map_err(crate::napi_anyhow_error)?;
		Ok(core_query_result_to_js(result))
	}

	#[napi]
	pub fn exec_sync(&self, sql: String) -> napi::Result<QueryResult> {
		let db = self.db.clone();
		wait_for_runtime(async move { db.exec_with_call_mode(sql, CallMode::SyncBlocking).await })
			.map(core_query_result_to_js)
	}

	#[napi]
	pub async fn close(&self) -> napi::Result<()> {
		self.db.close().await.map_err(crate::napi_anyhow_error)
	}

	#[napi(ts_return_type = "Promise<JsSqliteTransaction>")]
	pub fn begin_transaction(
		&self,
		env: Env,
		timeout_ms: Option<f64>,
		name: Option<String>,
	) -> napi::Result<JsObject> {
		let timeout = timeout_ms.map(transaction_timeout).transpose()?;
		let reservation = self.db.reserve_bridge_transaction();
		let db = self.db.clone();
		env.spawn_future(async move {
			let transaction = db
				.begin_reserved_bridge_transaction(reservation, name, timeout)
				.await
				.map_err(crate::napi_anyhow_error)?;
			Ok(JsSqliteTransaction { transaction })
		})
	}

	#[napi]
	pub fn begin_transaction_sync(
		&self,
		timeout_ms: Option<f64>,
		name: Option<String>,
	) -> napi::Result<JsSqliteTransaction> {
		let timeout = timeout_ms.map(transaction_timeout).transpose()?;
		let db = self.db.clone();
		let transaction = wait_for_runtime(async move {
			db.begin_named_transaction_with_mode(
				name.as_deref(),
				timeout,
				TransactionOrigin::Bridge,
				CallMode::SyncBlocking,
			)
			.await
		})?;
		Ok(JsSqliteTransaction { transaction })
	}
}

#[napi]
impl JsSqliteTransaction {
	#[napi]
	pub async fn execute(
		&self,
		sql: String,
		params: Option<Vec<JsBindParam>>,
	) -> napi::Result<NativeExecuteResult> {
		let params = params.map(js_bind_params_to_core).transpose()?;
		self.transaction
			.execute(sql, params)
			.await
			.map(core_execute_result_to_js)
			.map_err(crate::napi_anyhow_error)
	}

	#[napi]
	pub fn execute_sync(
		&self,
		sql: String,
		params: Option<Vec<JsBindParam>>,
	) -> napi::Result<NativeExecuteResult> {
		let params = params.map(js_bind_params_to_core).transpose()?;
		let transaction = self.transaction.clone();
		wait_for_runtime(async move { transaction.execute(sql, params).await })
			.map(core_execute_result_to_js)
	}

	#[napi]
	pub async fn exec(&self, sql: String) -> napi::Result<QueryResult> {
		self.transaction
			.exec(sql)
			.await
			.map(core_query_result_to_js)
			.map_err(crate::napi_anyhow_error)
	}

	#[napi]
	pub fn exec_sync(&self, sql: String) -> napi::Result<QueryResult> {
		let transaction = self.transaction.clone();
		wait_for_runtime(async move { transaction.exec(sql).await }).map(core_query_result_to_js)
	}

	#[napi]
	pub async fn commit(&self) -> napi::Result<Option<f64>> {
		self.transaction
			.commit()
			.await
			.map(|seq| seq.map(|seq| seq as f64))
			.map_err(crate::napi_anyhow_error)
	}

	#[napi]
	pub fn commit_sync(&self) -> napi::Result<Option<f64>> {
		let transaction = self.transaction.clone();
		wait_for_runtime(async move { transaction.commit().await })
			.map(|seq| seq.map(|seq| seq as f64))
	}

	#[napi]
	pub async fn rollback(&self) -> napi::Result<()> {
		self.transaction
			.rollback()
			.await
			.map_err(crate::napi_anyhow_error)
	}

	#[napi]
	pub fn rollback_sync(&self) -> napi::Result<()> {
		let transaction = self.transaction.clone();
		wait_for_runtime(async move { transaction.rollback().await })
	}
}

#[napi]
impl JsActorStateTransaction {
	#[napi]
	pub async fn execute(
		&self,
		sql: String,
		params: Option<Vec<JsBindParam>>,
	) -> napi::Result<NativeExecuteResult> {
		let params = params.map(js_bind_params_to_core).transpose()?;
		self.transaction
			.execute(sql, params)
			.await
			.map(core_execute_result_to_js)
			.map_err(crate::napi_anyhow_error)
	}

	#[napi]
	pub fn execute_sync(
		&self,
		sql: String,
		params: Option<Vec<JsBindParam>>,
	) -> napi::Result<NativeExecuteResult> {
		let params = params.map(js_bind_params_to_core).transpose()?;
		let transaction = self.transaction.clone();
		wait_for_runtime(async move { transaction.execute(sql, params).await })
			.map(core_execute_result_to_js)
	}

	#[napi]
	pub async fn commit(&self, payload: StateDeltaPayload) -> napi::Result<()> {
		self.transaction
			.commit(state_deltas_from_payload(payload))
			.await
			.map_err(crate::napi_anyhow_error)
	}

	#[napi]
	pub async fn rollback(&self) -> napi::Result<()> {
		self.transaction
			.rollback()
			.await
			.map_err(crate::napi_anyhow_error)
	}
}

fn wait_for_runtime<T, F>(future: F) -> napi::Result<T>
where
	F: Future<Output = anyhow::Result<T>>,
{
	let runtime = tokio::runtime::Handle::try_current().map_err(|error| {
		napi_anyhow_error(
			crate::NapiInvalidState {
				state: "runtime".to_owned(),
				reason: format!("cannot run synchronous SQLite operation: {error}"),
			}
			.build(),
		)
	})?;
	// NAPI-RS enters its multithreaded runtime before invoking synchronous exports.
	tokio::task::block_in_place(|| runtime.block_on(future)).map_err(crate::napi_anyhow_error)
}

pub(crate) fn transaction_timeout(timeout_ms: f64) -> napi::Result<Duration> {
	if !timeout_ms.is_finite() || timeout_ms <= 0.0 {
		return Err(napi_anyhow_error(
			NapiInvalidArgument {
				argument: "timeout".to_owned(),
				reason: "must be a positive finite number of milliseconds".to_owned(),
			}
			.build(),
		));
	}
	Duration::try_from_secs_f64(timeout_ms / 1_000.0).map_err(|_| {
		napi_anyhow_error(
			NapiInvalidArgument {
				argument: "timeout".to_owned(),
				reason: "is too large to represent".to_owned(),
			}
			.build(),
		)
	})
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
			other => Err(napi_anyhow_error(
				NapiInvalidArgument {
					argument: "kind".to_owned(),
					reason: format!("unsupported bind param kind `{other}`"),
				}
				.build(),
			)),
		})
		.collect()
}

fn js_batch_statements_to_core(
	statements: Vec<JsSqliteBatchStatement>,
) -> napi::Result<Vec<CoreSqliteBatchStatement>> {
	statements
		.into_iter()
		.map(|statement| {
			let params = statement.params.map(js_bind_params_to_core).transpose()?;
			Ok(CoreSqliteBatchStatement {
				sql: statement.sql,
				params,
			})
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
		readonly: result.readonly,
	}
}

fn core_execute_result_to_js(result: CoreExecuteResult) -> NativeExecuteResult {
	NativeExecuteResult {
		columns: result.columns,
		rows: result
			.rows
			.into_iter()
			.map(|row| row.into_iter().map(column_value_to_json).collect())
			.collect(),
		changes: result.changes,
		last_insert_row_id: result.last_insert_row_id,
		readonly: result.readonly,
		commit_seq: result.commit_seq.map(|seq| seq as f64),
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

#[cfg(test)]
mod tests {
	#[test]
	fn synchronous_wait_uses_the_active_multithreaded_runtime() {
		let runtime = tokio::runtime::Builder::new_multi_thread()
			.enable_all()
			.build()
			.expect("runtime should build");
		let _guard = runtime.enter();

		let result = super::wait_for_runtime(async { Ok::<_, anyhow::Error>(42) })
			.expect("future should complete");

		assert_eq!(result, 42);
	}
}
