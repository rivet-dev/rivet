use napi::bindgen_prelude::Buffer;
use napi_derive::napi;
use rivetkit_core::sqlite::{
	BindParam, ColumnValue, QueryResult as CoreQueryResult, SqliteDb as CoreSqliteDb,
};

use crate::{NapiInvalidArgument, napi_anyhow_error};
#[napi]
#[derive(Clone)]
pub struct JsNativeDatabase {
	db: CoreSqliteDb,
	actor_id: Option<String>,
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
}

#[napi]
impl JsNativeDatabase {
	#[napi]
	pub fn take_last_kv_error(&self) -> Option<String> {
		self.db.take_last_kv_error()
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
