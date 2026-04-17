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
		use std::process::Stdio;
		use std::sync::Arc;
		use std::sync::atomic::{AtomicBool, Ordering};

		use anyhow::Result;
		use futures::future::BoxFuture;
		use rivet_envoy_client::config::HttpRequest;
		use rivet_envoy_client::protocol;
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
			let mut registry = CoreRegistry::new();
			registry.register("counter", factory);
			registry.into_dispatcher()
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
