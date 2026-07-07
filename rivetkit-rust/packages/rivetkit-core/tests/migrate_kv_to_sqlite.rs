use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use anyhow::Result;
use rivet_envoy_client::config::{
	BoxFuture as EnvoyBoxFuture, EnvoyCallbacks, EnvoyConfig, HttpRequest, HttpResponse,
	WebSocketHandler, WebSocketSender,
};
use rivet_envoy_client::context::{SharedContext, WsTxMessage};
use rivet_envoy_client::envoy::ToEnvoyMessage;
use rivet_envoy_client::handle::EnvoyHandle;
use rivet_envoy_client::protocol;
use rivet_envoy_client::sqlite::{RemoteSqliteRequest, RemoteSqliteResponse};
use rusqlite::types::{Value, ValueRef};
use rusqlite::{Connection, params_from_iter};
use tokio::sync::{Mutex as AsyncMutex, mpsc};

use crate::actor::connection::{PersistedConnection, encode_persisted_connection};
use crate::actor::context::ActorContext;
use crate::actor::internal_schema;
use crate::actor::internal_storage::{self, InternalActorSnapshot};
use crate::actor::keys::{
	INSPECTOR_TOKEN_KEY, LAST_PUSHED_ALARM_KEY, PERSIST_DATA_KEY, QUEUE_METADATA_KEY,
	make_connection_key, make_prefixed_key, make_queue_message_key, make_traces_key,
	make_workflow_key,
};
use crate::actor::kv::Kv;
use crate::actor::queue::{
	PersistedQueueMessage, QueueMetadata, encode_queue_message, encode_queue_metadata,
};
use crate::actor::state::{
	PersistedActor, PersistedScheduleEvent, encode_last_pushed_alarm, encode_persisted_actor,
};
use crate::sqlite::{ColumnValue, SqliteDb};
use crate::types::{ActorKeySegment, ListOpts};

use super::import_core_state_if_needed;

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
		Box::pin(async { unreachable!("migration tests do not fetch") })
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
		Box::pin(async { unreachable!("migration tests do not open websockets") })
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
		last_ping_ts: std::sync::atomic::AtomicI64::new(i64::MAX),
		stopped_tx: tokio::sync::watch::channel(true).0,
	});

	(EnvoyHandle::from_shared(shared), envoy_rx)
}

fn sqlite_ctx(kv: Kv) -> (ActorContext, tokio::task::JoinHandle<()>) {
	let (handle, envoy_rx) = test_envoy_handle();
	let db = SqliteDb::new_with_remote_sqlite(
		handle,
		"actor-import",
		Some("user/1".to_owned()),
		Some(1),
		true,
		true,
	);
	let ctx = ActorContext::build(
		"actor-import".to_owned(),
		"actor".to_owned(),
		vec![ActorKeySegment::String("user/1".to_owned())],
		"local".to_owned(),
		Some(1),
		"test-envoy".to_owned(),
		Default::default(),
		kv,
		db,
	);
	(ctx, tokio::spawn(run_remote_sqlite(envoy_rx)))
}

async fn run_remote_sqlite(mut envoy_rx: mpsc::UnboundedReceiver<ToEnvoyMessage>) {
	let conn = Connection::open_in_memory().expect("sqlite harness should open");
	while let Some(message) = envoy_rx.recv().await {
		let ToEnvoyMessage::RemoteSqliteRequest {
			request,
			response_tx,
		} = message
		else {
			continue;
		};
		let RemoteSqliteRequest::Execute(request) = request else {
			panic!("migration test only expects remote execute requests");
		};
		let response = execute_remote_sql(&conn, &request.sql, request.params)
			.unwrap_or_else(|error| {
				protocol::SqliteExecuteResponse::SqliteErrorResponse(
					protocol::SqliteErrorResponse {
						group: "core".to_owned(),
						code: "internal_error".to_owned(),
						message: error.to_string(),
					},
				)
			});
		response_tx
			.send(Ok(RemoteSqliteResponse::Execute(response)))
			.expect("remote sqlite response receiver should still be alive");
	}
}

fn execute_remote_sql(
	conn: &Connection,
	sql: &str,
	params: Option<Vec<protocol::SqliteBindParam>>,
) -> rusqlite::Result<protocol::SqliteExecuteResponse> {
	let params = params
		.unwrap_or_default()
		.into_iter()
		.map(sqlite_value_from_protocol)
		.collect::<Vec<_>>();
	let mut statement = conn.prepare(sql)?;
	let column_count = statement.column_count();
	let columns = (0..column_count)
		.map(|index| statement.column_name(index).unwrap_or("").to_owned())
		.collect::<Vec<_>>();

	if column_count == 0 {
		let changes = statement.execute(params_from_iter(params.iter()))?;
		return Ok(sqlite_execute_response(protocol::SqliteExecuteResult {
			columns,
			rows: Vec::new(),
			changes: changes.try_into().unwrap_or(i64::MAX),
			last_insert_row_id: Some(conn.last_insert_rowid()),
		}));
	}

	let mut rows = statement.query(params_from_iter(params.iter()))?;
	let mut out_rows = Vec::new();
	while let Some(row) = rows.next()? {
		let mut out_row = Vec::with_capacity(column_count);
		for index in 0..column_count {
			out_row.push(sqlite_column_value_from_ref(row.get_ref(index)?));
		}
		out_rows.push(out_row);
	}

	Ok(sqlite_execute_response(protocol::SqliteExecuteResult {
		columns,
		rows: out_rows,
		changes: conn.changes().try_into().unwrap_or(i64::MAX),
		last_insert_row_id: Some(conn.last_insert_rowid()),
	}))
}

fn sqlite_value_from_protocol(value: protocol::SqliteBindParam) -> Value {
	match value {
		protocol::SqliteBindParam::SqliteValueNull => Value::Null,
		protocol::SqliteBindParam::SqliteValueInteger(value) => Value::Integer(value.value),
		protocol::SqliteBindParam::SqliteValueFloat(value) => {
			Value::Real(f64::from_bits(u64::from_be_bytes(value.value)))
		}
		protocol::SqliteBindParam::SqliteValueText(value) => Value::Text(value.value),
		protocol::SqliteBindParam::SqliteValueBlob(value) => Value::Blob(value.value),
	}
}

fn sqlite_column_value_from_ref(value: ValueRef<'_>) -> protocol::SqliteColumnValue {
	match value {
		ValueRef::Null => protocol::SqliteColumnValue::SqliteValueNull,
		ValueRef::Integer(value) => {
			protocol::SqliteColumnValue::SqliteValueInteger(protocol::SqliteValueInteger {
				value,
			})
		}
		ValueRef::Real(value) => {
			protocol::SqliteColumnValue::SqliteValueFloat(protocol::SqliteValueFloat {
				value: value.to_bits().to_be_bytes(),
			})
		}
		ValueRef::Text(value) => {
			protocol::SqliteColumnValue::SqliteValueText(protocol::SqliteValueText {
				value: String::from_utf8_lossy(value).into_owned(),
			})
		}
		ValueRef::Blob(value) => {
			protocol::SqliteColumnValue::SqliteValueBlob(protocol::SqliteValueBlob {
				value: value.to_vec(),
			})
		}
	}
}

fn sqlite_execute_response(
	result: protocol::SqliteExecuteResult,
) -> protocol::SqliteExecuteResponse {
	protocol::SqliteExecuteResponse::SqliteExecuteOk(protocol::SqliteExecuteOk { result })
}

#[tokio::test]
async fn marks_empty_legacy_import_done_without_runtime_rows() -> Result<()> {
	let kv = Kv::new_in_memory();
	let trace_key = make_traces_key(b"trace-only");
	kv.put(&trace_key, b"trace-value").await?;

	let (ctx, sqlite_task) = sqlite_ctx(kv);
	internal_schema::ensure_internal_schema(ctx.sql()).await?;
	import_core_state_if_needed(&ctx).await?;

	assert_eq!(
		internal_storage::load_meta_text(ctx.sql(), "kv_import_state").await?,
		Some("done".to_owned())
	);
	assert_eq!(internal_storage::load_actor_snapshot(ctx.sql()).await?, None);
	assert_eq!(
		internal_storage::load_queue_metadata(ctx.sql()).await?,
		QueueMetadata {
			next_id: 1,
			size: 0,
		}
	);
	let runtime_rows = ctx
		.sql()
		.query("SELECT COUNT(*) FROM _rivet_runtime", None)
		.await?;
	assert_eq!(
		runtime_rows.rows,
		vec![vec![ColumnValue::Integer(0)]],
		"fresh actors should not create _rivet_runtime rows during empty legacy import",
	);
	let user_rows = ctx
		.sql()
		.query("SELECT key, value FROM _rivet_user_kv", None)
		.await?;
	assert!(user_rows.rows.is_empty(), "trace records must not import as user KV");

	drop(ctx);
	sqlite_task.abort();
	Ok(())
}

#[tokio::test]
async fn imports_legacy_kv_snapshot_to_sqlite_once() -> Result<()> {
	let kv = Kv::new_in_memory();
	let actor = PersistedActor {
		input: Some(b"input".to_vec()),
		has_initialized: true,
		state: b"state-v1".to_vec(),
		scheduled_events: vec![PersistedScheduleEvent {
			event_id: "event-1".to_owned(),
			timestamp: 1234,
			action: "tick".to_owned(),
			args: Some(b"args".to_vec()),
		}],
	};
	let connection = PersistedConnection {
		id: "conn-1".to_owned(),
		parameters: b"params".to_vec(),
		state: b"conn-state".to_vec(),
		subscriptions: Vec::new(),
		gateway_id: [1, 2, 3, 4],
		request_id: [5, 6, 7, 8],
		server_message_index: 9,
		client_message_index: 10,
		request_path: "/socket".to_owned(),
		request_headers: HashMap::from([("x-test".to_owned(), "yes".to_owned())]),
	};
	let queue_message = PersistedQueueMessage {
		name: "job".to_owned(),
		body: b"body".to_vec(),
		created_at: 9876,
		failure_count: None,
		available_at: None,
		in_flight: None,
		in_flight_at: None,
	};
	let retry_queue_message = PersistedQueueMessage {
		name: "retry-job".to_owned(),
		body: b"retry-body".to_vec(),
		created_at: 9988,
		failure_count: Some(2),
		available_at: Some(9999),
		in_flight: Some(true),
		in_flight_at: Some(10000),
	};
	let retry_queue_message_key = make_queue_message_key(42);
	let retry_queue_message_value = encode_queue_message(&retry_queue_message)?;
	let workflow_key = make_workflow_key(b"wf-key");
	let user_key = make_prefixed_key(b"user-key");
	let trace_key = make_traces_key(b"trace-key");
	let legacy_entries = vec![
		(
			PERSIST_DATA_KEY.to_vec(),
			encode_persisted_actor(&actor)?,
		),
		(
			LAST_PUSHED_ALARM_KEY.to_vec(),
			encode_last_pushed_alarm(Some(5678))?,
		),
		(INSPECTOR_TOKEN_KEY.to_vec(), b"inspector-token".to_vec()),
		(
			make_connection_key("conn-1"),
			encode_persisted_connection(&connection)?,
		),
		(
			QUEUE_METADATA_KEY.to_vec(),
			encode_queue_metadata(&QueueMetadata {
				next_id: 41,
				size: 1,
			})?,
		),
		(
			make_queue_message_key(40),
			encode_queue_message(&queue_message)?,
		),
		(
			retry_queue_message_key.clone(),
			retry_queue_message_value.clone(),
		),
		(workflow_key.clone(), b"workflow-value".to_vec()),
		(user_key.clone(), b"user-value".to_vec()),
		(trace_key.clone(), b"trace-value".to_vec()),
	];
	let legacy_refs = legacy_entries
		.iter()
		.map(|(key, value)| (key.as_slice(), value.as_slice()))
		.collect::<Vec<_>>();
	kv.batch_put(&legacy_refs).await?;

	let (ctx, sqlite_task) = sqlite_ctx(kv.clone());
	internal_schema::ensure_internal_schema(ctx.sql()).await?;
	import_core_state_if_needed(&ctx).await?;

	assert_eq!(
		internal_storage::load_meta_text(ctx.sql(), "kv_import_state").await?,
		Some("done".to_owned())
	);
	assert_eq!(
		internal_storage::load_actor_snapshot(ctx.sql()).await?,
		Some(InternalActorSnapshot {
			actor: actor.clone(),
			last_pushed_alarm: Some(5678),
		})
	);
	assert_eq!(
		internal_storage::load_inspector_token(ctx.sql()).await?,
		Some("inspector-token".to_owned())
	);
	assert_eq!(
		internal_storage::load_connections(ctx.sql()).await?,
		vec![PersistedConnection {
			client_message_index: 0,
			..connection
		}]
	);
	assert_eq!(
		internal_storage::load_queue_metadata(ctx.sql()).await?,
		QueueMetadata {
			next_id: 43,
			size: 2,
		}
	);
	let queue_rows = internal_storage::load_queue_messages(ctx.sql()).await?;
	assert_eq!(queue_rows.len(), 2);
	assert_eq!(queue_rows[0].id, 40);
	assert_eq!(queue_rows[0].message, queue_message);
	assert_eq!(queue_rows[1].id, 42);
	assert_eq!(
		queue_rows[1].message,
		PersistedQueueMessage {
			failure_count: None,
			available_at: None,
			in_flight: None,
			in_flight_at: None,
			..retry_queue_message
		}
	);
	assert_eq!(
		internal_storage::user_kv_batch_get(ctx.sql(), &[retry_queue_message_key.as_slice()])
			.await?,
		vec![None],
		"legacy queue retry metadata stays only in the frozen legacy KV, not user KV",
	);

	let workflow_rows = ctx
		.sql()
		.query("SELECT key, value FROM _rivet_wf_kv", None)
		.await?;
	assert_eq!(
		workflow_rows.rows,
		vec![vec![
			ColumnValue::Blob(workflow_key),
			ColumnValue::Blob(b"workflow-value".to_vec())
		]]
	);
	assert_eq!(
		internal_storage::user_kv_batch_get(ctx.sql(), &[user_key.as_slice()]).await?,
		vec![Some(b"user-value".to_vec())],
		"legacy user KV keys must import verbatim, keeping the [4] prefix the TS runtime queries with",
	);
	assert_eq!(
		internal_storage::user_kv_batch_get(ctx.sql(), &[b"user-key"]).await?,
		vec![None],
		"stripped keys must not exist; the runtime passes keys through unchanged",
	);
	assert_eq!(
		ctx.kv_batch_get(&[user_key.as_slice()]).await?,
		vec![Some(b"user-value".to_vec())],
		"the runtime kv path used by the TS bridge must read migrated entries",
	);
	let trace_rows = ctx
		.sql()
		.query("SELECT key, value FROM _rivet_user_kv WHERE key = ?", Some(vec![
			crate::sqlite::BindParam::Blob(trace_key.clone()),
		]))
		.await?;
	assert!(trace_rows.rows.is_empty(), "legacy traces must not import");

	let legacy_after_import = kv
		.list_prefix(
			&[],
			ListOpts {
				reverse: false,
				limit: None,
			},
		)
		.await?;
	let mut sorted_legacy_entries = legacy_entries.clone();
	sorted_legacy_entries.sort_by(|left, right| left.0.cmp(&right.0));
	assert_eq!(legacy_after_import, sorted_legacy_entries);

	kv.put(PERSIST_DATA_KEY, &encode_persisted_actor(&PersistedActor {
		state: b"mutated-after-import".to_vec(),
		..actor
	})?)
	.await?;
	import_core_state_if_needed(&ctx).await?;
	assert_eq!(
		internal_storage::load_actor_snapshot(ctx.sql())
			.await?
			.expect("actor snapshot should remain imported")
			.actor
			.state,
		b"state-v1".to_vec(),
		"done import state must freeze the sqlite snapshot against later legacy kv changes",
	);

	drop(ctx);
	drop(kv);
	sqlite_task.abort();
	Ok(())
}

#[tokio::test]
async fn imports_subspaces_larger_than_the_backend_page_cap() -> Result<()> {
	let kv = Kv::new_in_memory();
	// Cap every listing below the entry count so the import must paginate
	// with a cursor, mirroring the engine's default 16,384-key listing cap.
	kv.test_set_list_limit_cap(7);

	let entry_count: usize = 20;
	let mut expected_user_entries = Vec::new();
	for index in 0..entry_count {
		let user_key = make_prefixed_key(format!("user-key-{index:04}").as_bytes());
		let value = format!("user-value-{index:04}").into_bytes();
		kv.put(&user_key, &value).await?;
		expected_user_entries.push((user_key, value));
	}
	for index in 0..entry_count {
		let workflow_key = make_workflow_key(format!("wf-key-{index:04}").as_bytes());
		kv.put(&workflow_key, format!("wf-value-{index:04}").as_bytes())
			.await?;
	}
	for index in 0..entry_count {
		kv.put(
			&make_queue_message_key(index as u64 + 1),
			&encode_queue_message(&PersistedQueueMessage {
				name: format!("job-{index:04}"),
				body: b"body".to_vec(),
				created_at: 1000 + index as i64,
				failure_count: None,
				available_at: None,
				in_flight: None,
				in_flight_at: None,
			})?,
		)
		.await?;
	}

	let (ctx, sqlite_task) = sqlite_ctx(kv.clone());
	internal_schema::ensure_internal_schema(ctx.sql()).await?;
	import_core_state_if_needed(&ctx).await?;

	assert_eq!(
		internal_storage::load_meta_text(ctx.sql(), "kv_import_state").await?,
		Some("done".to_owned())
	);
	let user_rows = ctx
		.sql()
		.query("SELECT COUNT(*) FROM _rivet_user_kv", None)
		.await?;
	assert_eq!(
		user_rows.rows,
		vec![vec![ColumnValue::Integer(entry_count as i64)]],
		"every user kv entry must import even when listings are capped below the total",
	);
	let expected_user_refs = expected_user_entries
		.iter()
		.map(|(key, _)| key.as_slice())
		.collect::<Vec<_>>();
	assert_eq!(
		internal_storage::user_kv_batch_get(ctx.sql(), &expected_user_refs).await?,
		expected_user_entries
			.iter()
			.map(|(_, value)| Some(value.clone()))
			.collect::<Vec<_>>(),
	);
	let workflow_rows = ctx
		.sql()
		.query("SELECT COUNT(*) FROM _rivet_wf_kv", None)
		.await?;
	assert_eq!(
		workflow_rows.rows,
		vec![vec![ColumnValue::Integer(entry_count as i64)]],
		"every workflow kv entry must import even when listings are capped below the total",
	);
	let queue_rows = internal_storage::load_queue_messages(ctx.sql()).await?;
	assert_eq!(
		queue_rows.len(),
		entry_count,
		"every queue message must import even when listings are capped below the total",
	);
	assert_eq!(
		internal_storage::load_queue_metadata(ctx.sql()).await?.next_id,
		entry_count as u64 + 1,
	);

	drop(ctx);
	drop(kv);
	sqlite_task.abort();
	Ok(())
}

#[tokio::test]
async fn interrupted_import_clears_partial_rows_and_reimports() -> Result<()> {
	let kv = Kv::new_in_memory();
	let actor = PersistedActor {
		input: None,
		has_initialized: true,
		state: b"legacy-state".to_vec(),
		scheduled_events: Vec::new(),
	};
	kv.put(PERSIST_DATA_KEY, &encode_persisted_actor(&actor)?)
		.await?;
	let user_key = make_prefixed_key(b"user-key");
	kv.put(&user_key, b"user-value").await?;

	let (ctx, sqlite_task) = sqlite_ctx(kv.clone());
	internal_schema::ensure_internal_schema(ctx.sql()).await?;

	// Simulate a crash mid-import: the importing marker is set and a stale
	// partial row exists that a fresh import would not produce.
	internal_storage::persist_meta_text(ctx.sql(), "kv_import_state", "importing").await?;
	internal_storage::user_kv_batch_put(ctx.sql(), &[(b"stale-partial-key".as_slice(), b"stale".as_slice())])
		.await?;
	internal_storage::persist_actor_snapshot(
		ctx.sql(),
		&PersistedActor {
			input: None,
			has_initialized: false,
			state: b"stale-partial-state".to_vec(),
			scheduled_events: Vec::new(),
		},
	)
	.await?;

	import_core_state_if_needed(&ctx).await?;

	assert_eq!(
		internal_storage::load_meta_text(ctx.sql(), "kv_import_state").await?,
		Some("done".to_owned())
	);
	assert_eq!(
		internal_storage::load_actor_snapshot(ctx.sql())
			.await?
			.expect("actor snapshot should import after interrupted import")
			.actor,
		actor,
		"interrupted imports must restart from scratch, replacing partial rows",
	);
	assert_eq!(
		internal_storage::user_kv_batch_get(
			ctx.sql(),
			&[user_key.as_slice(), b"stale-partial-key"],
		)
		.await?,
		vec![Some(b"user-value".to_vec()), None],
		"partial user kv rows must be cleared before the re-import",
	);
	let legacy_after_import = kv
		.list_prefix(
			&[],
			ListOpts {
				reverse: false,
				limit: None,
			},
		)
		.await?;
	assert_eq!(
		legacy_after_import.len(),
		2,
		"the frozen legacy kv must survive the interrupted import retry",
	);

	drop(ctx);
	drop(kv);
	sqlite_task.abort();
	Ok(())
}
