//! Remote SQL runs on the actor VFS. Its pages go through the lane's page protocol state in
//! process, so remote SQL and the Envoy page path share one database view and one commit path.
use super::{IndeterminateResult, Lane, Pages, lane_transport::LaneTransport};
use anyhow::{Result, ensure};
use rivet_envoy_protocol as p;
use sqlite_embedded::{
	database::{NativeDatabaseHandle, open_database_from_transport_with_guard},
	guard::ConnectionGuard,
	query::{BindParam, ColumnValue, ExecuteResult, QueryResult, SqliteResultTooLargeError},
	vfs::SqliteTransactionTooLargeError,
	worker::SqlitePool,
};
use std::sync::{
	Arc, Weak,
	atomic::{AtomicBool, Ordering},
};
use tokio::sync::Mutex;

pub(super) struct Session {
	database: NativeDatabaseHandle,
	unknown_commit: Arc<AtomicBool>,
}

/// Opens the actor VFS over the lane's pages for `generation`.
#[allow(
	clippy::too_many_arguments,
	reason = "a session needs its environment, pages, lane, identity, generation, pool and guard"
)]
pub(super) async fn open_session(
	env: Arc<dyn rivet_env::Env>,
	pages: Arc<Mutex<Pages>>,
	lane: Weak<Lane>,
	vfs_name: String,
	actor_id: String,
	generation: u32,
	pool: &SqlitePool,
	guard: ConnectionGuard,
) -> Result<Session> {
	let unknown_commit = Arc::new(AtomicBool::new(false));
	let transport = Arc::new(LaneTransport::new(
		env,
		pages,
		lane,
		generation,
		unknown_commit.clone(),
	));
	// The actor VFS is a storage adapter: its SQLite worker threads drive the in-process
	// transport through a runtime handle, as the actor-side VFS does.
	#[allow(
		clippy::disallowed_methods,
		reason = "the actor VFS adapter needs a runtime handle for its worker threads"
	)]
	let runtime = tokio::runtime::Handle::current();
	let database = open_database_from_transport_with_guard(
		Some(pool),
		vfs_name,
		transport,
		actor_id,
		u64::from(generation),
		runtime,
		None,
		Some(guard),
	)
	.await?;
	Ok(Session {
		database,
		unknown_commit,
	})
}

/// Runs one remote SQL request. A fenced filesystem or undecided page commit makes the
/// result indeterminate, so the caller discards the session. Responses above
/// `max_response_bytes` logical bytes fail the request with `sqlite.response_too_large`. The
/// session's connection guard stops each statement as soon as its own result passes that cap,
/// and a batch is checked again after each statement.
pub(super) async fn execute(
	session: &Session,
	max_response_bytes: usize,
	fenced: impl Fn() -> bool,
	request: RemoteRequest,
) -> Result<p::ToEnvoy> {
	let reply = dispatch(&session.database, max_response_bytes, request).await?;
	ensure!(
		!fenced() && !session.unknown_commit.load(Ordering::Acquire),
		IndeterminateResult
	);
	Ok(reply)
}

pub(super) fn in_transaction(session: &Option<Session>) -> bool {
	session
		.as_ref()
		.is_some_and(|session| session.database.explicit_transaction_open())
}

/// Error group and code of a remote SQL response refused for passing the response byte cap.
const RESPONSE_TOO_LARGE_GROUP: &str = "sqlite";
const RESPONSE_TOO_LARGE_CODE: &str = "response_too_large";

fn error(error: anyhow::Error) -> p::SqliteErrorResponse {
	if error.is::<SqliteResultTooLargeError>() {
		return p::SqliteErrorResponse {
			group: RESPONSE_TOO_LARGE_GROUP.into(),
			code: RESPONSE_TOO_LARGE_CODE.into(),
			message: error.to_string().into(),
		};
	}

	if error.is::<SqliteTransactionTooLargeError>() {
		return p::SqliteErrorResponse {
			group: sqlite_types::TRANSACTION_TOO_LARGE_GROUP.into(),
			code: sqlite_types::TRANSACTION_TOO_LARGE_CODE.into(),
			message: error.to_string().into(),
		};
	}

	p::SqliteErrorResponse {
		group: "sqlite".into(),
		code: "query_error".into(),
		message: error.to_string().into(),
	}
}
/// A remote SQL message, split out of the Envoy's message union.
pub(super) enum RemoteRequest {
	Exec(p::ToRivetSqliteExecRequest),
	Execute(p::ToRivetSqliteExecuteRequest),
	ExecuteBatch(p::ToRivetSqliteExecuteBatchRequest),
}

impl RemoteRequest {
	/// Returns the remote SQL request in `message`, or `message` itself when it is not one.
	pub(super) fn from_message(message: p::ToRivet) -> std::result::Result<Self, p::ToRivet> {
		let message = if let p::ToRivet::ToRivetSqliteExecRequest(req) = message {
			return Ok(Self::Exec(req));
		} else {
			message
		};
		let message = if let p::ToRivet::ToRivetSqliteExecuteRequest(req) = message {
			return Ok(Self::Execute(req));
		} else {
			message
		};
		if let p::ToRivet::ToRivetSqliteExecuteBatchRequest(req) = message {
			return Ok(Self::ExecuteBatch(req));
		}
		Err(message)
	}
}

async fn dispatch(
	database: &NativeDatabaseHandle,
	max_response_bytes: usize,
	request: RemoteRequest,
) -> Result<p::ToEnvoy> {
	Ok(match request {
		RemoteRequest::Exec(req) => {
			let result = database
				.exec(req.data.sql.to_string())
				.await
				.map(query_result);
			p::ToEnvoy::ToEnvoySqliteExecResponse(p::ToEnvoySqliteExecResponse {
				request_id: req.request_id,
				data: match result {
					Ok(result) => p::SqliteExecResponse::SqliteExecOk(p::SqliteExecOk { result }),
					Err(e) => p::SqliteExecResponse::SqliteErrorResponse(error(e)),
				},
			})
		}
		RemoteRequest::Execute(req) => {
			let result = execute_one(database, req.data.sql.to_string(), req.data.params).await;
			p::ToEnvoy::ToEnvoySqliteExecuteResponse(p::ToEnvoySqliteExecuteResponse {
				request_id: req.request_id,
				data: match result {
					Ok(result) => {
						p::SqliteExecuteResponse::SqliteExecuteOk(p::SqliteExecuteOk { result })
					}
					Err(e) => p::SqliteExecuteResponse::SqliteErrorResponse(error(e)),
				},
			})
		}
		RemoteRequest::ExecuteBatch(req) => {
			let result = execute_batch(database, req.data.statements, max_response_bytes).await;
			p::ToEnvoy::ToEnvoySqliteExecuteBatchResponse(p::ToEnvoySqliteExecuteBatchResponse {
				request_id: req.request_id,
				data: match result {
					Ok(results) => p::SqliteExecuteBatchResponse::SqliteExecuteBatchOk(
						p::SqliteExecuteBatchOk { results },
					),
					Err(e) => p::SqliteExecuteBatchResponse::SqliteErrorResponse(error(e)),
				},
			})
		}
	})
}

/// Runs the statements in one savepoint, rolling all of them back when one fails.
async fn execute_batch(
	database: &NativeDatabaseHandle,
	statements: Vec<p::SqliteBatchStatement>,
	max_response_bytes: usize,
) -> Result<Vec<p::SqliteExecuteResult>> {
	database.exec("SAVEPOINT rivet_batch".into()).await?;
	let mut results = Vec::new();
	let mut bytes = 0usize;
	for statement in statements {
		let result = execute_one(database, statement.sql.to_string(), statement.params)
			.await
			.and_then(|result| {
				bytes += logical_bytes(&result.rows);
				if bytes > max_response_bytes {
					return Err(SqliteResultTooLargeError {
						max_bytes: max_response_bytes as u64,
					}
					.into());
				}
				Ok(result)
			});
		match result {
			Ok(result) => results.push(result),
			Err(error) => {
				database
					.exec("ROLLBACK TO rivet_batch; RELEASE rivet_batch".into())
					.await?;
				return Err(error);
			}
		}
	}
	database.exec("RELEASE rivet_batch".into()).await?;
	Ok(results)
}

async fn execute_one(
	database: &NativeDatabaseHandle,
	sql: String,
	params: Option<Vec<p::SqliteBindParam>>,
) -> Result<p::SqliteExecuteResult> {
	let params = params
		.unwrap_or_default()
		.into_iter()
		.map(bind_param)
		.collect();
	let result = database.execute(sql, Some(params)).await?;
	Ok(execute_result(result))
}

fn bind_param(value: p::SqliteBindParam) -> BindParam {
	match value {
		p::SqliteBindParam::SqliteValueNull => BindParam::Null,
		p::SqliteBindParam::SqliteValueInteger(v) => BindParam::Integer(v.value),
		p::SqliteBindParam::SqliteValueFloat(v) => BindParam::Float(f64::from_be_bytes(v.value)),
		p::SqliteBindParam::SqliteValueText(v) => BindParam::Text(v.value.to_string()),
		p::SqliteBindParam::SqliteValueBlob(v) => BindParam::Blob(v.value.to_vec()),
	}
}

fn column_value(value: ColumnValue) -> p::SqliteColumnValue {
	match value {
		ColumnValue::Null => p::SqliteColumnValue::SqliteValueNull,
		ColumnValue::Integer(value) => {
			p::SqliteColumnValue::SqliteValueInteger(p::SqliteValueInteger { value })
		}
		ColumnValue::Float(value) => p::SqliteColumnValue::SqliteValueFloat(p::SqliteValueFloat {
			value: value.to_be_bytes(),
		}),
		ColumnValue::Text(value) => p::SqliteColumnValue::SqliteValueText(p::SqliteValueText {
			value: value.into(),
		}),
		ColumnValue::Blob(value) => p::SqliteColumnValue::SqliteValueBlob(p::SqliteValueBlob {
			value: value.into(),
		}),
	}
}

/// Counts a response's logical size: every value takes 8 bytes plus its text or blob length.
fn logical_bytes(rows: &[Vec<p::SqliteColumnValue>]) -> usize {
	rows.iter()
		.flatten()
		.map(|value| {
			8 + match value {
				p::SqliteColumnValue::SqliteValueText(text) => text.value.len(),
				p::SqliteColumnValue::SqliteValueBlob(blob) => blob.value.len(),
				p::SqliteColumnValue::SqliteValueNull
				| p::SqliteColumnValue::SqliteValueInteger(_)
				| p::SqliteColumnValue::SqliteValueFloat(_) => 0,
			}
		})
		.sum()
}

fn rows(rows: Vec<Vec<ColumnValue>>) -> Vec<Vec<p::SqliteColumnValue>> {
	rows.into_iter()
		.map(|row| row.into_iter().map(column_value).collect())
		.collect()
}

fn query_result(result: QueryResult) -> p::SqliteQueryResult {
	p::SqliteQueryResult {
		columns: result.columns.into_iter().map(Into::into).collect(),
		rows: rows(result.rows),
	}
}

fn execute_result(result: ExecuteResult) -> p::SqliteExecuteResult {
	p::SqliteExecuteResult {
		columns: result.columns.into_iter().map(Into::into).collect(),
		rows: rows(result.rows),
		changes: result.changes,
		last_insert_row_id: result.last_insert_row_id,
	}
}
