//! Portable counter actor fixture.
//!
//! The public `counter_actor` function is the single actor source used by
//! native tests. The `extern "C"` exports wrap the same function in a
//! `DylibBackend` so the fixture also builds as a cdylib.

use std::collections::HashMap;
use std::ffi::c_void;
use std::io::Cursor;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Result;
use rivet_actor_plugin_abi::{
	self as abi, ConnInfo, DylibBackend, Event, KvEntry, KvListOpts, PortableActorCtx,
	RequestSaveOpts,
};
use serde_json::{Value as JsonValue, json};

pub async fn counter_actor(ctx: PortableActorCtx) -> Result<()> {
	counter_actor_with_factory(ctx, None).await
}

async fn counter_actor_with_factory(
	ctx: PortableActorCtx,
	factory_info: Option<FactoryInfo>,
) -> Result<()> {
	ctx.startup_ready(Ok(()))?;

	let mut count = read_count(&ctx.state()?);
	let mut conn_stats = ConnStats::default();
	let mut scheduled_count = 0i64;
	let mut double_reply_status = JsonValue::Null;
	let save_wait_status = Arc::new(Mutex::new(None::<JsonValue>));
	while let Some(event) = ctx.next_event().await? {
		match event {
			Event::Action { name, reply, .. } => match name.as_str() {
				"increment" => {
					count += 1;
					ctx.request_save(RequestSaveOpts::default())?;
					ctx.reply_ok(reply, encode_json(&json!(count)))?;
				}
				"save_direct" => {
					count += 1;
					ctx.save_state(encode_json(&json!({ "count": count })))
						.await?;
					ctx.reply_ok(reply, encode_json(&json!(count)))?;
				}
				"set_state_report" => {
					ctx.set_state(encode_json(&json!({ "count": 41 })))?;
					count = read_count(&ctx.state()?);
					ctx.reply_ok(reply, encode_json(&json!({ "count": count })))?;
				}
				"abort_snapshot" => {
					ctx.reply_ok(
						reply,
						encode_json(&json!({
							"aborted": ctx.actor_aborted()?,
						})),
					)?;
				}
				"wait_abort" => {
					ctx.wait_for_actor_abort().await?;
					ctx.reply_ok(
						reply,
						encode_json(&json!({
							"aborted": ctx.actor_aborted()?,
						})),
					)?;
				}
				"save_wait" => {
					count += 1;
					*save_wait_status.lock().expect("save_wait status lock") = None;
					let wait_ctx = ctx.clone();
					let status = save_wait_status.clone();
					thread::spawn(move || {
						let result = futures::executor::block_on(wait_ctx.request_save_and_wait(
							RequestSaveOpts {
								immediate: true,
								max_wait_ms: Some(1_000),
							},
						));
						*status.lock().expect("save_wait status lock") = Some(match result {
							Ok(()) => json!({ "done": true, "ok": true }),
							Err(error) => {
								json!({ "done": true, "ok": false, "error": format!("{error:#}") })
							}
						});
					});
					ctx.reply_ok(reply, encode_json(&json!(count)))?;
				}
				"save_wait_status" => {
					let status = save_wait_status
						.lock()
						.expect("save_wait status lock")
						.clone()
						.unwrap_or_else(|| json!({ "done": false }));
					ctx.reply_ok(reply, encode_json(&status))?;
				}
				"double_reply_probe" => {
					ctx.reply_ok(reply, encode_json(&json!({ "first": true })))?;
					let second = ctx.reply_ok(reply, encode_json(&json!({ "second": true })));
					double_reply_status = json!({
						"secondOk": second.is_ok(),
						"secondError": second.err().map(|error| format!("{error:#}")),
					});
				}
				"double_reply_status" => {
					ctx.reply_ok(reply, encode_json(&double_reply_status))?;
				}
				"drop_reply_probe" => {
					let _ = reply;
				}
				"reply_err_probe" => {
					ctx.reply_err(reply, "portable reply error")?;
				}
				"sleep_now" => {
					ctx.sleep()?;
					ctx.reply_ok(reply, encode_json(&json!({ "requested": true })))?;
				}
				"keep_awake_report" => {
					let before = ctx.keep_awake_count()?;
					let count_ctx = ctx.clone();
					let during = ctx
						.keep_awake(async move { count_ctx.keep_awake_count() })
						.await??;
					let after = ctx.keep_awake_count()?;
					ctx.reply_ok(
						reply,
						encode_json(&json!({
							"before": before,
							"during": during,
							"after": after,
						})),
					)?;
				}
				"fanout_alarm_report" => {
					let timestamp_ms = now_ms().saturating_add(250);
					ctx.broadcast(
						"portable-broadcast",
						encode_json(&json!({ "source": "portable-parity" })),
					)?;
					ctx.set_alarm(Some(timestamp_ms)).await?;
					ctx.set_alarm(None).await?;
					ctx.reply_ok(
						reply,
						encode_json(&json!({
							"broadcasted": true,
							"alarmTimestampFuture": timestamp_ms >= now_ms(),
							"alarmCleared": true,
						})),
					)?;
				}
				"sleep_marker" => {
					let marker = ctx.kv_get(b"portable-lifecycle/sleep".to_vec()).await?;
					ctx.reply_ok(
						reply,
						encode_json(&json!({
							"sleepCleanupObserved": marker.as_deref() == Some(b"done".as_slice()),
						})),
					)?;
				}
				"get" => {
					ctx.reply_ok(reply, encode_json(&json!(count)))?;
				}
				"factory_config_report" => {
					let info = factory_info.as_ref();
					ctx.reply_ok(
						reply,
						encode_json(&json!({
							"configJson": info.map(|info| info.config_json.as_str()).unwrap_or(""),
							"sidecarPath": info.map(|info| info.sidecar_path.as_str()).unwrap_or(""),
						})),
					)?;
				}
				"identity_report" => {
					ctx.reply_ok(
						reply,
						encode_json(&json!({
							"actorId": ctx.actor_id()?,
							"name": ctx.name()?,
							"key": ctx.key()?,
							"region": ctx.region()?,
							"input": ctx.input()?.map(|input| decode_json(&input)),
							"hasState": ctx.has_state()?,
						})),
					)?;
				}
				"conn_report" => {
					let conns = ctx.conn_list().await?;
					ctx.disconnect_conn("missing-conn-for-portable-parity")
						.await?;
					let send_result = conn_stats.last_open.as_ref().map(|conn| {
						ctx.send(conn.id.clone(), "portable-send", b"payload".to_vec())
					});
					ctx.reply_ok(reply, encode_json(&conn_stats.report(conns, send_result)))?;
				}
				"ack_invalid" => {
					let result = ctx.ack_hibernatable_websocket_message(
						b"bad".to_vec(),
						b"req1".to_vec(),
						7,
					);
					ctx.reply_ok(
						reply,
						encode_json(&json!({
							"ok": result.is_ok(),
							"error": result.err().map(|error| format!("{error:#}")),
						})),
					)?;
				}
				"kv_roundtrip" => {
					let report = kv_roundtrip(&ctx).await?;
					ctx.reply_ok(reply, encode_json(&report))?;
				}
				"sqlite_roundtrip" => {
					let report = sqlite_roundtrip(&ctx).await?;
					ctx.reply_ok(reply, encode_json(&report))?;
				}
				"schedule_once" => {
					ctx.after(250, "scheduled_increment", Vec::new()).await?;
					let pending = ctx.scheduled_events().await?;
					ctx.reply_ok(
						reply,
						encode_json(&json!({
							"pendingCount": pending.len(),
							"firstAction": pending.first().map(|event| event.action_name.as_str()),
						})),
					)?;
				}
				"schedule_at_once" => {
					let timestamp_ms = now_ms().saturating_add(250);
					ctx.at(timestamp_ms, "scheduled_increment", Vec::new())
						.await?;
					let pending = ctx.scheduled_events().await?;
					ctx.reply_ok(
						reply,
						encode_json(&json!({
							"pendingCount": pending.len(),
							"firstAction": pending.first().map(|event| event.action_name.as_str()),
							"firstTimestampAtOrAfter": pending
								.first()
								.map(|event| event.timestamp_ms >= timestamp_ms)
								.unwrap_or(false),
						})),
					)?;
				}
				"scheduled_increment" => {
					scheduled_count += 1;
					ctx.reply_ok(reply, encode_json(&json!(scheduled_count)))?;
				}
				"schedule_report" => {
					ctx.reply_ok(
						reply,
						encode_json(&json!({ "scheduledCount": scheduled_count })),
					)?;
				}
				_ => {
					ctx.reply_err(reply, format!("unknown action `{name}`"))?;
				}
			},
			Event::SerializeState { reply } => {
				ctx.reply_ok(reply, encode_json(&json!({ "count": count })))?;
			}
			Event::Sleep { reply } => {
				ctx.kv_put(b"portable-lifecycle/sleep".to_vec(), b"done".to_vec())
					.await?;
				ctx.reply_ok(reply, Vec::new())?;
			}
			Event::Destroy { reply } => {
				ctx.reply_ok(reply, Vec::new())?;
			}
			Event::ConnPreflight {
				conn,
				params,
				reply,
			} => {
				conn_stats.preflight_count += 1;
				conn_stats.last_preflight = Some(conn);
				conn_stats.last_preflight_params = Some(params);
				ctx.reply_ok(reply, Vec::new())?;
			}
			Event::ConnOpen { conn, reply } => {
				conn_stats.open_count += 1;
				conn_stats.last_open = Some(conn);
				ctx.reply_ok(reply, Vec::new())?;
			}
			Event::QueueSend {
				name,
				body,
				conn,
				request,
				wait,
				timeout_ms,
				reply,
			} => {
				let response = encode_json(&json!({
					"name": name,
					"body": decode_json(&body),
					"conn": conn_info_json(&conn),
					"request": decode_http_request(&request).ok().map(|request| http_request_json(&request)),
					"wait": wait,
					"timeoutMs": timeout_ms,
				}));
				ctx.reply_ok(
					reply,
					abi::encode_queue_send_response("completed", Some(response))?,
				)?;
			}
			Event::WebSocketOpen {
				conn,
				request,
				reply,
			} => {
				conn_stats.ws_open_count += 1;
				conn_stats.last_ws_open = Some(conn);
				conn_stats.last_ws_request = request
					.as_deref()
					.and_then(|request| decode_http_request(request).ok())
					.map(|request| http_request_json(&request));
				ctx.reply_ok(reply, Vec::new())?;
			}
			Event::Http { request, reply } => {
				let request = decode_http_request(&request)?;
				let body = decode_json(&request.body);
				ctx.reply_ok(
					reply,
					encode_http_response(&HttpResponseWire {
						status: 207,
						headers: HashMap::from([(
							"x-portable-fixture".to_owned(),
							"http".to_owned(),
						)]),
						body: encode_json(&json!({
							"method": request.method,
							"uri": request.uri,
							"body": body,
							"header": request.headers.get("x-portable-test").cloned(),
						})),
					})?,
				)?;
			}
			Event::Subscribe {
				conn,
				event_name,
				reply,
			} => {
				conn_stats.subscribe_count += 1;
				conn_stats.last_subscribe = Some(conn);
				conn_stats.last_subscribe_event_name = Some(event_name);
				ctx.reply_ok(reply, Vec::new())?;
			}
			Event::ConnClosed { conn } => {
				conn_stats.closed_count += 1;
				conn_stats.last_closed = Some(conn);
			}
		}
	}

	Ok(())
}

#[derive(Clone)]
struct FactoryInfo {
	config_json: String,
	sidecar_path: String,
}

#[derive(Default)]
struct ConnStats {
	preflight_count: u64,
	open_count: u64,
	closed_count: u64,
	subscribe_count: u64,
	ws_open_count: u64,
	last_preflight: Option<ConnInfo>,
	last_preflight_params: Option<Vec<u8>>,
	last_open: Option<ConnInfo>,
	last_closed: Option<ConnInfo>,
	last_subscribe: Option<ConnInfo>,
	last_subscribe_event_name: Option<String>,
	last_ws_open: Option<ConnInfo>,
	last_ws_request: Option<JsonValue>,
}

impl ConnStats {
	fn report(&self, conns: Vec<ConnInfo>, send_result: Option<Result<()>>) -> JsonValue {
		json!({
			"preflightCount": self.preflight_count,
			"openCount": self.open_count,
			"closedCount": self.closed_count,
			"subscribeCount": self.subscribe_count,
			"wsOpenCount": self.ws_open_count,
			"lastPreflight": self.last_preflight.as_ref().map(conn_info_json),
			"lastPreflightParams": self.last_preflight_params.as_ref().map(|params| decode_json(params)),
			"lastOpen": self.last_open.as_ref().map(conn_info_json),
			"lastClosed": self.last_closed.as_ref().map(conn_info_json),
			"lastSubscribe": self.last_subscribe.as_ref().map(conn_info_json),
			"lastSubscribeEventName": self.last_subscribe_event_name.as_deref(),
			"lastWsOpen": self.last_ws_open.as_ref().map(conn_info_json),
			"lastWsRequest": self.last_ws_request.as_ref(),
			"connList": conns.iter().map(conn_info_json).collect::<Vec<_>>(),
			"disconnectMissingOk": true,
			"sendOk": send_result.as_ref().is_some_and(Result::is_ok),
			"sendError": send_result.and_then(Result::err).map(|error| format!("{error:#}")),
		})
	}
}

fn conn_info_json(conn: &ConnInfo) -> JsonValue {
	json!({
		"id": conn.id,
		"params": decode_json(&conn.params),
		"state": conn.state,
		"isHibernatable": conn.is_hibernatable,
	})
}

fn http_request_json(request: &HttpRequestWire) -> JsonValue {
	json!({
		"method": request.method.as_str(),
		"uri": request.uri.as_str(),
		"headers": &request.headers,
		"body": decode_json(&request.body),
	})
}

async fn kv_roundtrip(ctx: &PortableActorCtx) -> Result<JsonValue> {
	let prefix = b"portable-kv/".to_vec();
	let end = b"portable-kv0".to_vec();
	ctx.kv_delete_range(prefix.clone(), end.clone()).await?;

	ctx.kv_put(b"portable-kv/a".to_vec(), b"one".to_vec())
		.await?;
	ctx.kv_batch_put(vec![
		KvEntry {
			key: b"portable-kv/b".to_vec(),
			value: b"two".to_vec(),
		},
		KvEntry {
			key: b"portable-kv/c".to_vec(),
			value: b"three".to_vec(),
		},
		KvEntry {
			key: b"portable-kv/other".to_vec(),
			value: b"other".to_vec(),
		},
	])
	.await?;

	ctx.kv_delete(b"portable-kv/c".to_vec()).await?;
	ctx.kv_batch_delete(vec![b"portable-kv/other".to_vec()])
		.await?;

	let got = ctx.kv_get(b"portable-kv/a".to_vec()).await?;
	let batch = ctx
		.kv_batch_get(vec![
			b"portable-kv/a".to_vec(),
			b"portable-kv/b".to_vec(),
			b"portable-kv/c".to_vec(),
		])
		.await?;
	let prefix_entries = ctx
		.kv_list_prefix(prefix.clone(), KvListOpts::default())
		.await?;
	let range_entries = ctx
		.kv_list_range(
			b"portable-kv/a".to_vec(),
			b"portable-kv/c".to_vec(),
			KvListOpts {
				reverse: true,
				limit: Some(1),
			},
		)
		.await?;

	ctx.kv_delete_range(prefix.clone(), end).await?;
	let after_delete = ctx.kv_list_prefix(prefix, KvListOpts::default()).await?;

	Ok(json!({
		"got": bytes_option_json(got),
		"batch": batch.into_iter().map(bytes_option_json).collect::<Vec<_>>(),
		"prefix": kv_entries_json(prefix_entries),
		"range": kv_entries_json(range_entries),
		"afterDelete": kv_entries_json(after_delete),
	}))
}

async fn sqlite_roundtrip(ctx: &PortableActorCtx) -> Result<JsonValue> {
	if !ctx.sql_is_enabled() {
		return Ok(json!({ "enabled": false }));
	}

	ctx.db_exec(
		"CREATE TABLE IF NOT EXISTS portable_sqlite_parity (
			id INTEGER PRIMARY KEY,
			value TEXT NOT NULL
		)",
	)
	.await?;
	ctx.db_run("DELETE FROM portable_sqlite_parity", None)
		.await?;
	ctx.db_run(
		"INSERT INTO portable_sqlite_parity (id, value) VALUES (?, ?)",
		Some(encode_json(&json!([1, "native-dylib-parity"]))),
	)
	.await?;
	ctx.db_run(
		"INSERT INTO portable_sqlite_parity (id, value) VALUES (?, ?)",
		Some(encode_json(&json!([2, "filtered-out"]))),
	)
	.await?;

	let rows = ctx
		.db_query(
			"SELECT id, value FROM portable_sqlite_parity WHERE id = ?",
			Some(encode_json(&json!([1]))),
		)
		.await?;
	let rows = decode_json(&rows);

	Ok(json!({
		"enabled": true,
		"rows": rows,
	}))
}

fn kv_entries_json(entries: Vec<KvEntry>) -> JsonValue {
	JsonValue::Array(
		entries
			.into_iter()
			.map(|entry| {
				json!({
					"key": bytes_json(entry.key),
					"value": bytes_json(entry.value),
				})
			})
			.collect(),
	)
}

fn bytes_option_json(bytes: Option<Vec<u8>>) -> JsonValue {
	bytes.map(bytes_json).unwrap_or(JsonValue::Null)
}

fn bytes_json(bytes: Vec<u8>) -> JsonValue {
	JsonValue::String(String::from_utf8_lossy(&bytes).into_owned())
}

fn encode_json(value: &JsonValue) -> Vec<u8> {
	let mut out = Vec::new();
	ciborium::into_writer(value, &mut out).expect("encode cbor json");
	out
}

fn decode_json(bytes: &[u8]) -> JsonValue {
	if bytes.is_empty() {
		return JsonValue::Null;
	}
	ciborium::from_reader(Cursor::new(bytes)).unwrap_or(JsonValue::Null)
}

#[derive(serde::Serialize, serde::Deserialize)]
struct HttpRequestWire {
	method: String,
	uri: String,
	headers: HashMap<String, String>,
	#[serde(with = "serde_bytes")]
	body: Vec<u8>,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct HttpResponseWire {
	status: u16,
	headers: HashMap<String, String>,
	#[serde(with = "serde_bytes")]
	body: Vec<u8>,
}

fn decode_http_request(bytes: &[u8]) -> Result<HttpRequestWire> {
	Ok(ciborium::from_reader(Cursor::new(bytes))?)
}

fn encode_http_response(response: &HttpResponseWire) -> Result<Vec<u8>> {
	let mut out = Vec::new();
	ciborium::into_writer(response, &mut out)?;
	Ok(out)
}

fn read_count(state: &[u8]) -> i64 {
	if state.is_empty() {
		return 0;
	}

	let value = decode_json(state);
	value
		.get("count")
		.and_then(JsonValue::as_i64)
		.unwrap_or_default()
}

fn now_ms() -> i64 {
	let millis = SystemTime::now()
		.duration_since(UNIX_EPOCH)
		.unwrap_or_default()
		.as_millis();
	i64::try_from(millis).unwrap_or(i64::MAX)
}

#[unsafe(no_mangle)]
pub extern "C" fn rivet_actor_abi_magic() -> u64 {
	abi::RIVET_ACTOR_ABI_MAGIC
}

#[unsafe(no_mangle)]
pub extern "C" fn rivet_actor_abi_version() -> u64 {
	abi::RIVET_ACTOR_ABI_VERSION
}

struct Plugin;
struct Factory {
	info: FactoryInfo,
}
struct Instance {
	join: Option<thread::JoinHandle<()>>,
}

#[unsafe(no_mangle)]
pub extern "C" fn rivet_actor_plugin_init(_out_err: *mut abi::OwnedBuf) -> *mut c_void {
	Box::into_raw(Box::new(Plugin)) as *mut c_void
}

#[unsafe(no_mangle)]
pub extern "C" fn rivet_actor_factory_new(
	_plugin: *mut c_void,
	config_json: abi::BorrowedBuf,
	sidecar_path: abi::BorrowedBuf,
	_out_err: *mut abi::OwnedBuf,
) -> *mut c_void {
	let info = FactoryInfo {
		config_json: String::from_utf8_lossy(unsafe { config_json.as_slice() }).into_owned(),
		sidecar_path: String::from_utf8_lossy(unsafe { sidecar_path.as_slice() }).into_owned(),
	};
	Box::into_raw(Box::new(Factory { info })) as *mut c_void
}

struct SendVtable(abi::HostVtable);
unsafe impl Send for SendVtable {}

struct SendPtr(*mut c_void);
unsafe impl Send for SendPtr {}

impl SendPtr {
	fn complete(self, done: abi::CompletionFn, result: abi::AbiResult) {
		done(self.0, result);
	}
}

#[unsafe(no_mangle)]
pub extern "C" fn rivet_actor_run(
	factory: *mut c_void,
	host: *const abi::HostVtable,
	done: abi::CompletionFn,
	user_data: *mut c_void,
) -> *mut c_void {
	let host = SendVtable(unsafe { *host });
	let user_data = SendPtr(user_data);
	let factory_info = unsafe { (*(factory as *const Factory)).info.clone() };
	let join = thread::spawn(move || {
		let ctx = unsafe { PortableActorCtx::new_dylib(DylibBackend::from_host_vtable(&host.0)) };
		let result =
			futures::executor::block_on(counter_actor_with_factory(ctx, Some(factory_info)));
		let abi_result = match result {
			Ok(()) => abi::AbiResult::ok(abi::OwnedBuf::empty()),
			Err(err) => {
				abi::AbiResult::err(abi::OwnedBuf::from_vec(format!("{err:#}").into_bytes()))
			}
		};
		user_data.complete(done, abi_result);
	});

	Box::into_raw(Box::new(Instance { join: Some(join) })) as *mut c_void
}

#[unsafe(no_mangle)]
pub extern "C" fn rivet_actor_cancel(_instance: *mut c_void) {}

#[unsafe(no_mangle)]
pub extern "C" fn rivet_actor_grace_deadline(_instance: *mut c_void) {}

#[unsafe(no_mangle)]
pub extern "C" fn rivet_actor_instance_free(instance: *mut c_void) {
	if !instance.is_null() {
		let mut instance = unsafe { Box::from_raw(instance as *mut Instance) };
		if let Some(join) = instance.join.take() {
			let _ = join.join();
		}
	}
}

#[unsafe(no_mangle)]
pub extern "C" fn rivet_actor_factory_free(factory: *mut c_void) {
	if !factory.is_null() {
		unsafe { drop(Box::from_raw(factory as *mut Factory)) };
	}
}

#[unsafe(no_mangle)]
pub extern "C" fn rivet_actor_plugin_shutdown(plugin: *mut c_void) {
	if !plugin.is_null() {
		unsafe { drop(Box::from_raw(plugin as *mut Plugin)) };
	}
}
