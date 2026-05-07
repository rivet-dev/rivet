use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::Mutex as StdMutex;

use super::*;
use depot_client_types::{HEAD_FENCE_MISMATCH_CODE, HEAD_FENCE_MISMATCH_GROUP};
use rivet_envoy_client::config::{
	BoxFuture as EnvoyBoxFuture, EnvoyCallbacks, EnvoyConfig, HttpRequest, HttpResponse,
	WebSocketHandler, WebSocketSender,
};
use rivet_envoy_client::context::{SharedContext, WsTxMessage};
use rivet_envoy_client::envoy::ToEnvoyMessage;
use rivet_envoy_client::handle::EnvoyHandle;
use tokio::sync::{Mutex as AsyncMutex, mpsc};
use tracing::field::{Field, Visit};
use tracing::{Event, Subscriber};
use tracing_subscriber::layer::{Context as LayerContext, Layer};
use tracing_subscriber::prelude::*;
use tracing_subscriber::registry::Registry;

#[derive(Clone, Debug, Default)]
struct SqliteOperationLog {
	level: Option<tracing::Level>,
	message: Option<String>,
	actor_id: Option<String>,
	generation: Option<String>,
	backend: Option<String>,
	operation: Option<String>,
	sql: Option<String>,
	binding_count: Option<u64>,
	group: Option<String>,
	code: Option<String>,
	error_message: Option<String>,
}

#[derive(Clone)]
struct SqliteOperationLogLayer {
	records: Arc<StdMutex<Vec<SqliteOperationLog>>>,
}

#[derive(Default)]
struct SqliteOperationLogVisitor {
	record: SqliteOperationLog,
}

impl Visit for SqliteOperationLogVisitor {
	fn record_str(&mut self, field: &Field, value: &str) {
		match field.name() {
			"message" => self.record.message = Some(value.to_owned()),
			"actor_id" => self.record.actor_id = Some(value.to_owned()),
			"backend" => self.record.backend = Some(value.to_owned()),
			"operation" => self.record.operation = Some(value.to_owned()),
			"sql" => self.record.sql = Some(value.to_owned()),
			"group" => self.record.group = Some(value.to_owned()),
			"code" => self.record.code = Some(value.to_owned()),
			"error_message" => self.record.error_message = Some(value.to_owned()),
			_ => {}
		}
	}

	fn record_u64(&mut self, field: &Field, value: u64) {
		if field.name() == "binding_count" {
			self.record.binding_count = Some(value);
		}
	}

	fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
		match field.name() {
			"message" => {
				self.record.message = Some(format!("{value:?}").trim_matches('"').to_owned());
			}
			"generation" => self.record.generation = Some(format!("{value:?}")),
			"backend" => self.record.backend = Some(format!("{value:?}")),
			"error_message" => {
				self.record.error_message = Some(format!("{value:?}").trim_matches('"').to_owned());
			}
			_ => {}
		}
	}
}

impl<S> Layer<S> for SqliteOperationLogLayer
where
	S: Subscriber,
{
	fn on_event(&self, event: &Event<'_>, _ctx: LayerContext<'_, S>) {
		let mut visitor = SqliteOperationLogVisitor::default();
		event.record(&mut visitor);
		visitor.record.level = Some(*event.metadata().level());
		self.records
			.lock()
			.expect("sqlite operation log lock poisoned")
			.push(visitor.record);
	}
}

struct IdleEnvoyCallbacks;

impl EnvoyCallbacks for IdleEnvoyCallbacks {
	fn on_actor_start(
		&self,
		_handle: EnvoyHandle,
		_actor_id: String,
		_generation: u32,
		_config: protocol::ActorConfig,
		_preloaded_kv: Option<protocol::PreloadedKv>,
	) -> EnvoyBoxFuture<anyhow::Result<()>> {
		Box::pin(async { Ok(()) })
	}

	fn on_shutdown(&self) {}

	fn fetch(
		&self,
		_handle: EnvoyHandle,
		_actor_id: String,
		_gateway_id: protocol::GatewayId,
		_request_id: protocol::RequestId,
		_request: HttpRequest,
	) -> EnvoyBoxFuture<anyhow::Result<HttpResponse>> {
		Box::pin(async { unreachable!("sqlite tests do not fetch") })
	}

	fn websocket(
		&self,
		_handle: EnvoyHandle,
		_actor_id: String,
		_gateway_id: protocol::GatewayId,
		_request_id: protocol::RequestId,
		_request: HttpRequest,
		_path: String,
		_headers: HashMap<String, String>,
		_is_hibernatable: bool,
		_is_restoring_hibernatable: bool,
		_sender: WebSocketSender,
	) -> EnvoyBoxFuture<anyhow::Result<WebSocketHandler>> {
		Box::pin(async { unreachable!("sqlite tests do not open websockets") })
	}

	fn can_hibernate(
		&self,
		_actor_id: &str,
		_gateway_id: &protocol::GatewayId,
		_request_id: &protocol::RequestId,
		_request: &HttpRequest,
	) -> EnvoyBoxFuture<anyhow::Result<bool>> {
		Box::pin(async { Ok(false) })
	}
}

fn test_envoy_handle() -> (EnvoyHandle, mpsc::UnboundedReceiver<ToEnvoyMessage>) {
	let (envoy_tx, envoy_rx) = mpsc::unbounded_channel();
	let shared = Arc::new(SharedContext {
		config: EnvoyConfig {
			version: 1,
			endpoint: "http://127.0.0.1:1".to_string(),
			token: None,
			namespace: "test".to_string(),
			pool_name: "test".to_string(),
			prepopulate_actor_names: HashMap::new(),
			metadata: None,
			not_global: true,
			debug_latency_ms: None,
			callbacks: Arc::new(IdleEnvoyCallbacks),
		},
		envoy_key: "test-envoy".to_string(),
		envoy_tx,
		actors: Default::default(),
		actors_notify: Arc::new(tokio::sync::Notify::new()),
		live_tunnel_requests: Default::default(),
		pending_hibernation_restores: Default::default(),
		ws_tx: Arc::new(AsyncMutex::new(None::<mpsc::UnboundedSender<WsTxMessage>>)),
		protocol_metadata: Arc::new(AsyncMutex::new(None)),
		shutting_down: AtomicBool::new(false),
		stopped_tx: tokio::sync::watch::channel(true).0,
	});

	(EnvoyHandle::from_shared(shared), envoy_rx)
}

#[test]
fn remote_backend_requires_declared_database_and_capability() {
	assert_eq!(
		select_sqlite_backend(true, true),
		SqliteBackend::RemoteEnvoy
	);

	#[cfg(feature = "sqlite-local")]
	{
		assert_eq!(
			select_sqlite_backend(true, false),
			SqliteBackend::LocalNative
		);
		assert_eq!(
			select_sqlite_backend(false, true),
			SqliteBackend::LocalNative
		);
	}

	#[cfg(not(feature = "sqlite-local"))]
	{
		assert_eq!(
			select_sqlite_backend(true, false),
			SqliteBackend::Unavailable
		);
		assert_eq!(
			select_sqlite_backend(false, true),
			SqliteBackend::Unavailable
		);
	}
}

#[test]
fn protocol_conversion_preserves_bind_and_result_values() {
	let params = protocol_bind_params(vec![
		BindParam::Null,
		BindParam::Integer(7),
		BindParam::Float(1.5),
		BindParam::Text("hello".to_owned()),
		BindParam::Blob(vec![1, 2, 3]),
	]);

	assert!(matches!(
		params[0],
		protocol::SqliteBindParam::SqliteValueNull
	));
	assert!(matches!(
		params[1],
		protocol::SqliteBindParam::SqliteValueInteger(protocol::SqliteValueInteger { value: 7 })
	));
	assert!(matches!(
		params[2],
		protocol::SqliteBindParam::SqliteValueFloat(protocol::SqliteValueFloat { value })
			if f64::from_bits(u64::from_be_bytes(value)) == 1.5
	));
	assert!(matches!(
		&params[3],
		protocol::SqliteBindParam::SqliteValueText(protocol::SqliteValueText { value })
			if value == "hello"
	));
	assert!(matches!(
		&params[4],
		protocol::SqliteBindParam::SqliteValueBlob(protocol::SqliteValueBlob { value })
			if value == &vec![1, 2, 3]
	));

	let result = execute_result_from_protocol(protocol::SqliteExecuteResult {
		columns: vec!["id".to_owned(), "score".to_owned()],
		rows: vec![vec![
			protocol::SqliteColumnValue::SqliteValueInteger(protocol::SqliteValueInteger {
				value: 9,
			}),
			protocol::SqliteColumnValue::SqliteValueFloat(protocol::SqliteValueFloat {
				value: 2.25_f64.to_bits().to_be_bytes(),
			}),
		]],
		changes: 3,
		last_insert_row_id: Some(11),
	});

	assert_eq!(result.columns, vec!["id", "score"]);
	assert_eq!(
		result.rows,
		vec![vec![ColumnValue::Integer(9), ColumnValue::Float(2.25)]]
	);
	assert_eq!(result.changes, 3);
	assert_eq!(result.last_insert_row_id, Some(11));
}

#[test]
fn remote_protocol_compatibility_errors_become_remote_unavailable() {
	let err = anyhow::anyhow!(protocol::versioned::ProtocolCompatibilityError {
		feature: protocol::versioned::ProtocolCompatibilityFeature::RemoteSqliteExecution,
		direction: protocol::versioned::ProtocolCompatibilityDirection::ToRivet,
		required_version: 4,
		target_version: 3,
	});

	let mapped = remote_request_error(err);
	let structured = rivet_error::RivetError::extract(&mapped);
	assert_eq!(structured.group(), "sqlite");
	assert_eq!(structured.code(), "remote_unavailable");
}

#[test]
fn remote_lost_response_errors_become_indeterminate_result() {
	let err = anyhow::anyhow!(
		rivet_envoy_client::utils::RemoteSqliteIndeterminateResultError {
			operation: "execute",
		}
	);

	let mapped = remote_request_error(err);
	let structured = rivet_error::RivetError::extract(&mapped);
	assert_eq!(structured.group(), "sqlite");
	assert_eq!(structured.code(), "remote_indeterminate_result");
}

#[tokio::test]
async fn remote_execute_logs_operation_context_at_source() {
	let (handle, envoy_rx) = test_envoy_handle();
	drop(envoy_rx);
	let db = SqliteDb::new_with_remote_sqlite(
		handle,
		"actor-sqlite-log",
		Some("user/1".to_owned()),
		Some(7),
		true,
		true,
	);
	let records = Arc::new(StdMutex::new(Vec::new()));
	let subscriber = Registry::default().with(SqliteOperationLogLayer {
		records: records.clone(),
	});
	let _guard = tracing::subscriber::set_default(subscriber);

	let result = db
		.execute(
			"SELECT ?",
			Some(vec![BindParam::Integer(1), BindParam::Text("two".to_owned())]),
		)
		.await;

	assert!(result.is_err());
	let actor_specifier = rivet_error::RivetError::extract(&result.err().unwrap())
		.actor()
		.cloned();
	assert_eq!(
		actor_specifier,
		Some(rivet_error::ActorSpecifier::new("actor-sqlite-log", 7).with_key("user/1"))
	);
	let logs = records
		.lock()
		.expect("sqlite operation log lock poisoned")
		.clone();
	assert!(
		logs.iter().any(|log| {
			log.level == Some(tracing::Level::ERROR)
				&& log.message.as_deref() == Some("sqlite operation failed")
				&& log.actor_id.as_deref() == Some("actor-sqlite-log")
				&& log.generation.as_deref() == Some("Some(7)")
				&& log.backend.as_deref() == Some("RemoteEnvoy")
				&& log.operation.as_deref() == Some("execute")
				&& log.sql.as_deref() == Some("SELECT ?")
				&& log.binding_count == Some(2)
				&& log.group.as_deref() == Some("core")
				&& log.code.as_deref() == Some("internal_error")
				&& log.error_message.as_deref() == Some("An internal error occurred")
		}),
		"expected source sqlite operation log with actor id and generation; logs={logs:?}"
	);
}

#[test]
fn remote_head_fence_mismatch_stops_actor_once() {
	let (handle, mut envoy_rx) = test_envoy_handle();
	let db = SqliteDb::new_with_remote_sqlite(handle, "actor-a", None, Some(7), true, true);

	let mapped = db.remote_sqlite_error_response(protocol::SqliteErrorResponse {
		group: HEAD_FENCE_MISMATCH_GROUP.to_string(),
		code: HEAD_FENCE_MISMATCH_CODE.to_string(),
		message: "head fence mismatch in remote sqlite".to_string(),
	});
	let structured = rivet_error::RivetError::extract(&mapped);
	assert_eq!(structured.group(), "sqlite");
	assert_eq!(structured.code(), "closed");

	match envoy_rx.try_recv().expect("missing stop actor intent") {
		ToEnvoyMessage::ActorIntent {
			actor_id,
			generation,
			intent,
			error,
		} => {
			assert_eq!(actor_id, "actor-a");
			assert_eq!(generation, Some(7));
			assert!(matches!(intent, protocol::ActorIntent::ActorIntentStop));
			assert!(
				error
					.expect("missing stop reason")
					.contains("remote sqlite fatal storage error")
			);
		}
		_ => panic!("expected stop actor intent"),
	}

	let _ = db.remote_sqlite_error_response(protocol::SqliteErrorResponse {
		group: HEAD_FENCE_MISMATCH_GROUP.to_string(),
		code: HEAD_FENCE_MISMATCH_CODE.to_string(),
		message: "second head fence mismatch".to_string(),
	});
	assert!(envoy_rx.try_recv().is_err());
}
