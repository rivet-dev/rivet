use super::*;

	impl RegistryDispatcher {
		async fn start_actor_for_test(
			&self,
			actor_id: &str,
			generation: u32,
			actor_name: &str,
			input: Option<Vec<u8>>,
		) -> anyhow::Result<()> {
			let factory = self
				.factories
				.get(actor_name)
				.cloned()
				.ok_or_else(|| anyhow::anyhow!("actor factory `{actor_name}` is not registered"))?;
			let ctx = ActorContext::new_runtime(
				actor_id.to_owned(),
				actor_name.to_owned(),
				actor_key_from_protocol(None),
				self.region.clone(),
				factory.config().clone(),
				crate::kv::tests::new_in_memory(),
				crate::sqlite::SqliteDb::default(),
			);
			self.start_actor(StartActorRequest {
				actor_id: actor_id.to_owned(),
				generation,
				actor_name: actor_name.to_owned(),
				input,
				preload_persisted_actor: None,
				ctx,
			})
			.await
		}

		async fn handle_websocket_for_test(&self, actor_id: &str) -> anyhow::Result<()> {
			let instance = self.active_actor(actor_id).await?;
			let Some(callback) = instance.callbacks.on_websocket.as_ref() else {
				return Ok(());
			};

			instance
				.ctx
				.with_websocket_callback(|| async {
					callback(OnWebSocketRequest {
						ctx: instance.ctx.clone(),
						ws: WebSocket::new(),
					})
					.await
				})
				.await
		}

		async fn stop_actor_for_test(
			&self,
			actor_id: &str,
			reason: protocol::StopActorReason,
		) -> anyhow::Result<()> {
			let instance = self.active_actor(actor_id).await?;
			let _ = self.active_instances.remove_async(actor_id).await;

			let lifecycle = ActorLifecycle;
			match reason {
				protocol::StopActorReason::SleepIntent => {
					lifecycle
						.shutdown_for_sleep(
							instance.ctx.clone(),
							instance.factory.as_ref(),
							instance.callbacks.clone(),
						)
						.await?;
				}
				_ => {
					lifecycle
						.shutdown_for_destroy(
							instance.ctx.clone(),
							instance.factory.as_ref(),
							instance.callbacks.clone(),
						)
						.await?;
				}
			}

			Ok(())
		}
	}

	mod moved_tests {
		use std::collections::HashMap;
		use std::io::Cursor;
		use std::process::Stdio;
		use std::sync::Arc;
		use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

		use anyhow::Result;
		use ciborium::{from_reader, into_writer};
		use futures::future::BoxFuture;
		use rivet_envoy_client::config::{HttpRequest, HttpResponse};
		use rivet_envoy_client::protocol;
		use serde_json::{Value as JsonValue, json};
		use tokio::io::AsyncWriteExt;
		use tokio::net::TcpListener;
		use tokio::process::Command;

		use super::{
			CoreRegistry, RegistryDispatcher, engine_health_url, terminate_engine_process,
			wait_for_engine_health,
		};
		use crate::actor::callbacks::{
			ActorInstanceCallbacks, LifecycleCallback, OnRequestRequest,
			OnWebSocketRequest, RequestCallback, Response,
		};
		use crate::actor::factory::{ActorFactory, FactoryRequest};
		use crate::ActorConfig;

		fn request_callback<F>(callback: F) -> RequestCallback
		where
			F: Fn(OnRequestRequest) -> BoxFuture<'static, Result<super::Response>>
				+ Send
				+ Sync
				+ 'static,
		{
			Box::new(callback)
		}

		fn lifecycle_callback<F, T>(callback: F) -> LifecycleCallback<T>
		where
			F: Fn(T) -> BoxFuture<'static, Result<()>> + Send + Sync + 'static,
			T: Send + 'static,
		{
			Box::new(callback)
		}

		fn factory<F>(build: F) -> ActorFactory
		where
			F: Fn(FactoryRequest) -> BoxFuture<'static, Result<ActorInstanceCallbacks>>
				+ Send
				+ Sync
				+ 'static,
		{
			ActorFactory::new(ActorConfig::default(), build)
		}

		fn dispatcher_for(factory: ActorFactory) -> Arc<RegistryDispatcher> {
			dispatcher_for_token(factory, None)
		}

		fn dispatcher_for_token(
			factory: ActorFactory,
			inspector_token: Option<&str>,
		) -> Arc<RegistryDispatcher> {
			let mut registry = CoreRegistry::new();
			registry.register("counter", factory);
			Arc::new(RegistryDispatcher {
				factories: registry.factories,
				active_instances: scc::HashMap::new(),
				region: String::new(),
				inspector_token: inspector_token.map(str::to_owned),
			})
		}

		fn encode_cbor(value: &impl serde::Serialize) -> Vec<u8> {
			let mut encoded = Vec::new();
			into_writer(value, &mut encoded).expect("encode test cbor");
			encoded
		}

		fn decode_json_body(response: &HttpResponse) -> JsonValue {
			serde_json::from_slice(
				response.body.as_ref().expect("response body should exist"),
			)
			.expect("response body should be valid json")
		}

		fn inspector_fixture_factory() -> ActorFactory {
			factory(|request| {
				Box::pin(async move {
					request.ctx.set_state(encode_cbor(&json!({ "count": 5 })));
					request
						.ctx
						.queue()
						.send("job", &encode_cbor(&json!({ "work": 1 })))
						.await?;

					let mut callbacks = ActorInstanceCallbacks::default();
					callbacks.on_request = Some(request_callback(|_request| {
						Box::pin(async move {
							let response = Response::from(
								http::Response::builder()
									.status(http::StatusCode::IM_A_TEAPOT)
									.body(b"wrong route".to_vec())
									.expect("build response"),
							);
							Ok(response)
						})
					}));
					callbacks.actions.insert(
						"increment".to_owned(),
						Box::new(|request| {
							Box::pin(async move {
								let args: Vec<i64> = from_reader(Cursor::new(request.args))
									.expect("decode action args");
								let state: JsonValue =
									from_reader(Cursor::new(request.ctx.state()))
										.expect("decode actor state");
								let next = state
									.get("count")
									.and_then(JsonValue::as_i64)
									.unwrap_or_default()
									+ args.first().copied().unwrap_or_default();
								request
									.ctx
									.set_state(encode_cbor(&json!({ "count": next })));
								Ok(encode_cbor(&json!(next)))
							})
						}),
					);
					Ok(callbacks)
				})
			})
		}

		fn workflow_inspector_fixture_factory(
			history_calls: Arc<AtomicUsize>,
			replay_calls: Arc<AtomicUsize>,
		) -> ActorFactory {
			factory(move |_request| {
				let history_calls = history_calls.clone();
				let replay_calls = replay_calls.clone();
				Box::pin(async move {
					let mut callbacks = ActorInstanceCallbacks::default();
					callbacks.get_workflow_history = Some(Box::new(move |_request| {
						let history_calls = history_calls.clone();
						Box::pin(async move {
							history_calls.fetch_add(1, Ordering::SeqCst);
							Ok(Some(encode_cbor(&json!({
								"nameRegistry": ["counter"],
								"entries": [{"id": "entry-1"}],
								"entryMetadata": {
									"entry-1": {"status": "completed"}
								},
							}))))
						})
					}));
					callbacks.replay_workflow = Some(Box::new(move |request| {
						let replay_calls = replay_calls.clone();
						Box::pin(async move {
							replay_calls.fetch_add(1, Ordering::SeqCst);
							Ok(Some(encode_cbor(&json!({
								"nameRegistry": ["counter"],
								"entries": [{"id": request.entry_id.unwrap_or_else(|| "root".to_owned())}],
								"entryMetadata": {},
							}))))
						})
					}));
					Ok(callbacks)
				})
			})
		}

		#[tokio::test]
		async fn dispatcher_routes_fetch_to_started_actor() {
			let dispatcher = dispatcher_for(factory(|_request| {
				Box::pin(async move {
					let mut callbacks = ActorInstanceCallbacks::default();
					callbacks.on_request = Some(request_callback(|request| {
						Box::pin(async move {
							let response = Response::from(
								http::Response::builder()
									.status(http::StatusCode::CREATED)
									.body(request.request.into_body())
									.expect("build response"),
							);
							Ok(response)
						})
					}));
					Ok(callbacks)
				})
			}));

			dispatcher
				.start_actor_for_test("actor-1", 1, "counter", Some(b"seed".to_vec()))
				.await
				.expect("start actor");

			let response = dispatcher
				.handle_fetch(
					"actor-1",
					HttpRequest {
						method: "POST".to_owned(),
						path: "/".to_owned(),
						headers: HashMap::new(),
						body: Some(b"ping".to_vec()),
						body_stream: None,
					},
				)
				.await
				.expect("fetch should succeed");

			assert_eq!(response.status, http::StatusCode::CREATED.as_u16());
			assert_eq!(response.body, Some(b"ping".to_vec()));
		}

		#[tokio::test]
		async fn dispatcher_serves_prometheus_metrics_before_actor_request_callback() {
			let dispatcher = dispatcher_for_token(
				factory(|_request| {
					Box::pin(async move {
						let mut callbacks = ActorInstanceCallbacks::default();
						callbacks.on_request = Some(request_callback(|_request| {
							Box::pin(async move {
								let response = Response::from(
									http::Response::builder()
										.status(http::StatusCode::IM_A_TEAPOT)
										.body(b"wrong route".to_vec())
										.expect("build response"),
								);
								Ok(response)
							})
						}));
						Ok(callbacks)
					})
				}),
				Some("token"),
			);

			dispatcher
				.start_actor_for_test("actor-1", 1, "counter", None)
				.await
				.expect("start actor");

			let response = dispatcher
				.handle_fetch(
					"actor-1",
					HttpRequest {
						method: "GET".to_owned(),
						path: "/metrics".to_owned(),
						headers: HashMap::from([(
							"authorization".to_owned(),
							"Bearer token".to_owned(),
						)]),
						body: None,
						body_stream: None,
					},
				)
				.await
				.expect("metrics fetch should succeed");

			assert_eq!(response.status, http::StatusCode::OK.as_u16());
			assert_eq!(
				response
					.headers
					.get(http::header::CONTENT_TYPE.as_str())
					.map(String::as_str),
				Some("text/plain; version=0.0.4")
			);
			let body = String::from_utf8(
				response.body.expect("metrics body should be present"),
			)
			.expect("metrics body should be utf-8");
			assert!(body.contains("total_startup_ms"));
			assert!(!body.contains("wrong route"));
		}

		#[tokio::test]
		async fn dispatcher_rejects_metrics_without_valid_token() {
			let dispatcher = dispatcher_for_token(
				factory(|_request| {
					Box::pin(async move { Ok(ActorInstanceCallbacks::default()) })
				}),
				Some("token"),
			);

			dispatcher
				.start_actor_for_test("actor-1", 1, "counter", None)
				.await
				.expect("start actor");

			let response = dispatcher
				.handle_fetch(
					"actor-1",
					HttpRequest {
						method: "GET".to_owned(),
						path: "/metrics".to_owned(),
						headers: HashMap::new(),
						body: None,
						body_stream: None,
					},
				)
				.await
				.expect("metrics fetch should succeed");

			assert_eq!(response.status, http::StatusCode::UNAUTHORIZED.as_u16());
		}

		#[tokio::test]
		async fn dispatcher_routes_inspector_state_before_actor_request_callback() {
			let dispatcher = dispatcher_for_token(inspector_fixture_factory(), Some("token"));

			dispatcher
				.start_actor_for_test("actor-1", 1, "counter", None)
				.await
				.expect("start actor");

			let response = dispatcher
				.handle_fetch(
					"actor-1",
					HttpRequest {
						method: "GET".to_owned(),
						path: "/inspector/state".to_owned(),
						headers: HashMap::from([(
							"authorization".to_owned(),
							"Bearer token".to_owned(),
						)]),
						body: None,
						body_stream: None,
					},
				)
				.await
				.expect("inspector state should succeed");

			assert_eq!(response.status, http::StatusCode::OK.as_u16());
			assert_eq!(
				decode_json_body(&response),
				json!({
					"state": { "count": 5 },
					"isStateEnabled": true,
				})
			);
		}

		#[tokio::test]
		async fn dispatcher_rejects_inspector_without_valid_token() {
			let dispatcher = dispatcher_for_token(inspector_fixture_factory(), Some("token"));

			dispatcher
				.start_actor_for_test("actor-1", 1, "counter", None)
				.await
				.expect("start actor");

			let response = dispatcher
				.handle_fetch(
					"actor-1",
					HttpRequest {
						method: "GET".to_owned(),
						path: "/inspector/state".to_owned(),
						headers: HashMap::from([(
							"authorization".to_owned(),
							"Bearer wrong-token".to_owned(),
						)]),
						body: None,
						body_stream: None,
					},
				)
				.await
				.expect("inspector auth response should succeed");

			assert_eq!(response.status, http::StatusCode::UNAUTHORIZED.as_u16());
			assert_eq!(
				decode_json_body(&response)
					.get("code")
					.and_then(JsonValue::as_str),
				Some("unauthorized")
			);
		}

		#[tokio::test]
		async fn dispatcher_patches_inspector_state_and_executes_action() {
			let dispatcher = dispatcher_for_token(inspector_fixture_factory(), Some("token"));

			dispatcher
				.start_actor_for_test("actor-1", 1, "counter", None)
				.await
				.expect("start actor");

			let patch_response = dispatcher
				.handle_fetch(
					"actor-1",
					HttpRequest {
						method: "PATCH".to_owned(),
						path: "/inspector/state".to_owned(),
						headers: HashMap::from([
							("authorization".to_owned(), "Bearer token".to_owned()),
							(
								"content-type".to_owned(),
								"application/json".to_owned(),
							),
						]),
						body: Some(br#"{"state":{"count":42}}"#.to_vec()),
						body_stream: None,
					},
				)
				.await
				.expect("inspector patch should succeed");
			assert_eq!(patch_response.status, http::StatusCode::OK.as_u16());
			assert_eq!(decode_json_body(&patch_response), json!({ "ok": true }));

			let action_response = dispatcher
				.handle_fetch(
					"actor-1",
					HttpRequest {
						method: "POST".to_owned(),
						path: "/inspector/action/increment".to_owned(),
						headers: HashMap::from([
							("authorization".to_owned(), "Bearer token".to_owned()),
							(
								"content-type".to_owned(),
								"application/json".to_owned(),
							),
						]),
						body: Some(br#"{"args":[5]}"#.to_vec()),
						body_stream: None,
					},
				)
				.await
				.expect("inspector action should succeed");

			assert_eq!(action_response.status, http::StatusCode::OK.as_u16());
			assert_eq!(
				decode_json_body(&action_response),
				json!({ "output": 47 })
			);
		}

		#[tokio::test]
		async fn dispatcher_returns_inspector_queue_and_summary_json() {
			let dispatcher = dispatcher_for_token(inspector_fixture_factory(), Some("token"));

			dispatcher
				.start_actor_for_test("actor-1", 1, "counter", None)
				.await
				.expect("start actor");

			let queue_response = dispatcher
				.handle_fetch(
					"actor-1",
					HttpRequest {
						method: "GET".to_owned(),
						path: "/inspector/queue?limit=10".to_owned(),
						headers: HashMap::from([(
							"authorization".to_owned(),
							"Bearer token".to_owned(),
						)]),
						body: None,
						body_stream: None,
					},
				)
				.await
				.expect("inspector queue should succeed");
			assert_eq!(queue_response.status, http::StatusCode::OK.as_u16());
			let queue_json = decode_json_body(&queue_response);
			assert_eq!(queue_json["size"], json!(1));
			assert_eq!(queue_json["maxSize"], json!(1000));
			assert_eq!(queue_json["truncated"], json!(false));
			assert_eq!(queue_json["messages"][0]["id"], json!(1));
			assert_eq!(queue_json["messages"][0]["name"], json!("job"));
			assert!(queue_json["messages"][0]["createdAtMs"].is_number());

			let summary_response = dispatcher
				.handle_fetch(
					"actor-1",
					HttpRequest {
						method: "GET".to_owned(),
						path: "/inspector/summary".to_owned(),
						headers: HashMap::from([(
							"authorization".to_owned(),
							"Bearer token".to_owned(),
						)]),
						body: None,
						body_stream: None,
					},
				)
				.await
				.expect("inspector summary should succeed");
			assert_eq!(summary_response.status, http::StatusCode::OK.as_u16());
			assert_eq!(
				decode_json_body(&summary_response),
				json!({
					"state": { "count": 5 },
					"isStateEnabled": true,
					"connections": [],
					"rpcs": ["increment"],
					"queueSize": 1,
					"isDatabaseEnabled": false,
					"isWorkflowEnabled": false,
					"workflowHistory": null,
				})
			);
		}

		#[tokio::test]
		async fn dispatcher_routes_workflow_inspector_requests_lazily() {
			let history_calls = Arc::new(AtomicUsize::new(0));
			let replay_calls = Arc::new(AtomicUsize::new(0));
			let dispatcher = dispatcher_for_token(
				workflow_inspector_fixture_factory(
					history_calls.clone(),
					replay_calls.clone(),
				),
				Some("token"),
			);

			dispatcher
				.start_actor_for_test("actor-1", 1, "counter", None)
				.await
				.expect("start actor");
			assert_eq!(history_calls.load(Ordering::SeqCst), 0);
			assert_eq!(replay_calls.load(Ordering::SeqCst), 0);

			let state_response = dispatcher
				.handle_fetch(
					"actor-1",
					HttpRequest {
						method: "GET".to_owned(),
						path: "/inspector/state".to_owned(),
						headers: HashMap::from([(
							"authorization".to_owned(),
							"Bearer token".to_owned(),
						)]),
						body: None,
						body_stream: None,
					},
				)
				.await
				.expect("state request should succeed");
			assert_eq!(state_response.status, http::StatusCode::OK.as_u16());
			assert_eq!(history_calls.load(Ordering::SeqCst), 0);
			assert_eq!(replay_calls.load(Ordering::SeqCst), 0);

			let history_response = dispatcher
				.handle_fetch(
					"actor-1",
					HttpRequest {
						method: "GET".to_owned(),
						path: "/inspector/workflow-history".to_owned(),
						headers: HashMap::from([(
							"authorization".to_owned(),
							"Bearer token".to_owned(),
						)]),
						body: None,
						body_stream: None,
					},
				)
				.await
				.expect("workflow history should succeed");
			assert_eq!(history_response.status, http::StatusCode::OK.as_u16());
			assert_eq!(
				decode_json_body(&history_response),
				json!({
					"history": {
						"nameRegistry": ["counter"],
						"entries": [{"id": "entry-1"}],
						"entryMetadata": {
							"entry-1": {"status": "completed"}
						},
					},
					"isWorkflowEnabled": true,
				})
			);
			assert_eq!(history_calls.load(Ordering::SeqCst), 1);
			assert_eq!(replay_calls.load(Ordering::SeqCst), 0);

			let replay_response = dispatcher
				.handle_fetch(
					"actor-1",
					HttpRequest {
						method: "POST".to_owned(),
						path: "/inspector/workflow/replay".to_owned(),
						headers: HashMap::from([
							("authorization".to_owned(), "Bearer token".to_owned()),
							(
								"content-type".to_owned(),
								"application/json".to_owned(),
							),
						]),
						body: Some(br#"{"entryId":"entry-9"}"#.to_vec()),
						body_stream: None,
					},
				)
				.await
				.expect("workflow replay should succeed");
			assert_eq!(replay_response.status, http::StatusCode::OK.as_u16());
			assert_eq!(
				decode_json_body(&replay_response),
				json!({
					"history": {
						"nameRegistry": ["counter"],
						"entries": [{"id": "entry-9"}],
						"entryMetadata": {},
					},
					"isWorkflowEnabled": true,
				})
			);
			assert_eq!(history_calls.load(Ordering::SeqCst), 1);
			assert_eq!(replay_calls.load(Ordering::SeqCst), 1);
		}

		#[tokio::test]
		async fn dispatcher_returns_null_workflow_payloads_without_callbacks() {
			let dispatcher = dispatcher_for_token(
				factory(|_request| {
					Box::pin(async move { Ok(ActorInstanceCallbacks::default()) })
				}),
				Some("token"),
			);

			dispatcher
				.start_actor_for_test("actor-1", 1, "counter", None)
				.await
				.expect("start actor");

			let history_response = dispatcher
				.handle_fetch(
					"actor-1",
					HttpRequest {
						method: "GET".to_owned(),
						path: "/inspector/workflow-history".to_owned(),
						headers: HashMap::from([(
							"authorization".to_owned(),
							"Bearer token".to_owned(),
						)]),
						body: None,
						body_stream: None,
					},
				)
				.await
				.expect("workflow history should succeed");
			assert_eq!(
				decode_json_body(&history_response),
				json!({
					"history": null,
					"isWorkflowEnabled": false,
				})
			);

			let replay_response = dispatcher
				.handle_fetch(
					"actor-1",
					HttpRequest {
						method: "POST".to_owned(),
						path: "/inspector/workflow/replay".to_owned(),
						headers: HashMap::from([
							("authorization".to_owned(), "Bearer token".to_owned()),
							(
								"content-type".to_owned(),
								"application/json".to_owned(),
							),
						]),
						body: Some(br#"{}"#.to_vec()),
						body_stream: None,
					},
				)
				.await
				.expect("workflow replay should succeed");
			assert_eq!(
				decode_json_body(&replay_response),
				json!({
					"history": null,
					"isWorkflowEnabled": false,
				})
			);
		}

		#[tokio::test]
		async fn dispatcher_routes_websocket_to_started_actor() {
			let invoked = Arc::new(AtomicBool::new(false));
			let invoked_clone = invoked.clone();
			let dispatcher = dispatcher_for(factory(move |_request| {
				let invoked = invoked_clone.clone();
				Box::pin(async move {
					let mut callbacks = ActorInstanceCallbacks::default();
					callbacks.on_websocket = Some(lifecycle_callback(
						move |_request: OnWebSocketRequest| {
							let invoked = invoked.clone();
							Box::pin(async move {
								invoked.store(true, Ordering::SeqCst);
								Ok(())
							})
						},
					));
					Ok(callbacks)
				})
			}));

			dispatcher
				.start_actor_for_test("actor-1", 1, "counter", None)
				.await
				.expect("start actor");
			dispatcher
				.handle_websocket_for_test("actor-1")
				.await
				.expect("websocket should succeed");

			assert!(invoked.load(Ordering::SeqCst));
		}

		#[tokio::test]
		async fn dispatcher_stops_actor_and_removes_it_from_active_map() {
			let dispatcher = dispatcher_for(factory(|_request| {
				Box::pin(async move { Ok(ActorInstanceCallbacks::default()) })
			}));

			dispatcher
				.start_actor_for_test("actor-1", 1, "counter", None)
				.await
				.expect("start actor");
			dispatcher
				.stop_actor_for_test("actor-1", protocol::StopActorReason::Destroy)
				.await
				.expect("stop actor");

			assert!(
				dispatcher
					.active_instances
					.get_async(&"actor-1".to_owned())
					.await
					.is_none()
			);
		}

		#[tokio::test]
		async fn dispatcher_returns_error_for_unknown_actor_fetch() {
			let dispatcher = dispatcher_for(factory(|_request| {
				Box::pin(async move { Ok(ActorInstanceCallbacks::default()) })
			}));

			let result = dispatcher
				.handle_fetch(
					"missing",
					HttpRequest {
						method: "GET".to_owned(),
						path: "/".to_owned(),
						headers: HashMap::new(),
						body: None,
						body_stream: None,
					},
				)
				.await;
			let error = match result {
				Ok(_) => panic!("missing actor should error"),
				Err(error) => error,
			};

			assert!(error.to_string().contains("missing"));
		}

		#[tokio::test]
		async fn engine_health_check_retries_until_success() {
			let listener = TcpListener::bind("127.0.0.1:0")
				.await
				.expect("bind health listener");
			let address = listener.local_addr().expect("health listener addr");
			let server = tokio::spawn(async move {
				for attempt in 0..3 {
					let (mut stream, _) =
						listener.accept().await.expect("accept health request");
					let mut request = [0u8; 1024];
					let _ = stream.readable().await;
					let _ = stream.try_read(&mut request);

					if attempt < 2 {
						stream
							.write_all(
								b"HTTP/1.1 503 Service Unavailable\r\ncontent-length: 0\r\n\r\n",
							)
							.await
							.expect("write unhealthy response");
					} else {
						stream
							.write_all(
								b"HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: 51\r\n\r\n{\"status\":\"ok\",\"runtime\":\"engine\",\"version\":\"test\"}",
							)
							.await
							.expect("write healthy response");
					}
				}
			});

			let health =
				wait_for_engine_health(&engine_health_url(&format!("http://{address}")))
					.await
					.expect("wait for engine health");
			server.await.expect("join health server");

			assert_eq!(health.runtime.as_deref(), Some("engine"));
			assert_eq!(health.version.as_deref(), Some("test"));
		}

		#[tokio::test]
		#[cfg(unix)]
		async fn terminate_engine_process_prefers_sigterm() {
			let mut child = Command::new("sh")
				.arg("-c")
				.arg("trap 'exit 0' TERM; while true; do sleep 1; done")
				.stdout(Stdio::null())
				.stderr(Stdio::null())
				.spawn()
				.expect("spawn looping shell");

			terminate_engine_process(&mut child)
				.await
				.expect("terminate child process");

			assert!(child.try_wait().expect("inspect child").is_some());
		}
	}
