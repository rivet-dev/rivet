use super::*;

	mod moved_tests {
		use std::sync::Arc;
		use std::sync::atomic::{AtomicUsize, Ordering};

		use anyhow::Result;
		use async_trait::async_trait;
		use rivet_error::RivetError;
		use serde::{Deserialize, Serialize};

		use super::{TypedActionMap, build_action, build_factory};
		use crate::actor::Actor;
		use crate::context::Ctx;
		use crate::{Request, Response};
		use rivetkit_core::{ActionRequest, ActorContext, ConnHandle, FactoryRequest};

		#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
		struct TestState {
			value: i64,
		}

		#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
		struct TestInput {
			start: i64,
		}

		#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
		struct TestParams {
			label: String,
		}

		#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
		struct TestConnState {
			count: usize,
		}

		#[derive(Debug)]
		struct TestVars {
			label: &'static str,
		}

		struct TestActor {
			migrate_count: AtomicUsize,
			wake_count: AtomicUsize,
		}

		struct UnitVarsActor;

		#[async_trait]
		impl Actor for TestActor {
			type State = TestState;
			type ConnParams = TestParams;
			type ConnState = TestConnState;
			type Input = TestInput;
			type Vars = TestVars;

			async fn create_state(
				_ctx: &Ctx<Self>,
				input: &Self::Input,
			) -> Result<Self::State> {
				Ok(TestState { value: input.start })
			}

			async fn create_vars(_ctx: &Ctx<Self>) -> Result<Self::Vars> {
				Ok(TestVars { label: "vars" })
			}

			async fn create_conn_state(
				self: &Arc<Self>,
				_ctx: &Ctx<Self>,
				_params: &Self::ConnParams,
			) -> Result<Self::ConnState> {
				let _ = self;
				Ok(TestConnState { count: 0 })
			}

			async fn on_create(_ctx: &Ctx<Self>, _input: &Self::Input) -> Result<Self> {
				Ok(Self {
					migrate_count: AtomicUsize::new(0),
					wake_count: AtomicUsize::new(0),
				})
			}

			async fn on_migrate(
				self: &Arc<Self>,
				ctx: &Ctx<Self>,
				is_new: bool,
			) -> Result<()> {
				assert_eq!(ctx.vars().label, "vars");
				assert!(is_new);
				self.migrate_count.fetch_add(1, Ordering::SeqCst);
				Ok(())
			}

			async fn on_wake(self: &Arc<Self>, ctx: &Ctx<Self>) -> Result<()> {
				assert_eq!(ctx.vars().label, "vars");
				self.wake_count.fetch_add(1, Ordering::SeqCst);
				Ok(())
			}

			async fn on_state_change(self: &Arc<Self>, ctx: &Ctx<Self>) -> Result<()> {
				let _ = self;
				assert!(ctx.state().value >= 0);
				Ok(())
			}

			async fn on_request(
				self: &Arc<Self>,
				ctx: &Ctx<Self>,
				_request: Request,
			) -> Result<Response> {
				let _ = self;
				Ok(Response::new(ctx.state().value.to_string().into_bytes()))
			}

			async fn on_before_connect(
				self: &Arc<Self>,
				ctx: &Ctx<Self>,
				params: &Self::ConnParams,
			) -> Result<()> {
				let _ = self;
				assert_eq!(ctx.vars().label, "vars");
				assert_eq!(params.label, "socket");
				Ok(())
			}

			async fn on_connect(
				self: &Arc<Self>,
				_ctx: &Ctx<Self>,
				conn: crate::context::ConnCtx<Self>,
			) -> Result<()> {
				let _ = self;
				assert_eq!(conn.state().count, 1);
				Ok(())
			}

			async fn on_disconnect(
				self: &Arc<Self>,
				_ctx: &Ctx<Self>,
				conn: crate::context::ConnCtx<Self>,
			) -> Result<()> {
				let _ = self;
				assert_eq!(conn.params().label, "socket");
				Ok(())
			}
		}

		impl TestActor {
			async fn increment(
				self: Arc<Self>,
				ctx: Ctx<Self>,
				(amount,): (i64,),
			) -> Result<TestState> {
				let _ = self;
				let mut state = (*ctx.state()).clone();
				state.value += amount;
				ctx.set_state(&state);
				Ok(state)
			}
		}

		#[async_trait]
		impl Actor for UnitVarsActor {
			type State = TestState;
			type ConnParams = ();
			type ConnState = ();
			type Input = ();
			type Vars = ();

			async fn create_state(
				_ctx: &Ctx<Self>,
				_input: &Self::Input,
			) -> Result<Self::State> {
				Ok(TestState { value: 0 })
			}

			async fn create_conn_state(
				self: &Arc<Self>,
				_ctx: &Ctx<Self>,
				_params: &Self::ConnParams,
			) -> Result<Self::ConnState> {
				let _ = self;
				Ok(())
			}

			async fn on_create(_ctx: &Ctx<Self>, _input: &Self::Input) -> Result<Self> {
				Ok(Self)
			}

			async fn on_request(
				self: &Arc<Self>,
				_ctx: &Ctx<Self>,
				_request: Request,
			) -> Result<Response> {
				let _ = self;
				Ok(Response::new(b"ok".to_vec()))
			}
		}

		#[tokio::test]
		async fn factory_builds_callbacks_and_serializes_actions() {
			let mut actions = TypedActionMap::<TestActor>::new();
			actions.insert(
				"increment".to_owned(),
				build_action(TestActor::increment),
			);
			let factory = build_factory::<TestActor>(actions);
			let input = super::serialize_cbor(&TestInput { start: 7 })
				.expect("test input should serialize");
			let ctx = ActorContext::new("actor-id", "test", Vec::new(), "local");
			let callbacks = factory
				.create(FactoryRequest {
					ctx: ctx.clone(),
					input: Some(input),
					is_new: true,
				})
				.await
				.expect("factory should build typed callbacks");

			assert!(callbacks.on_wake.is_some());
			assert!(callbacks.on_migrate.is_some());
			assert!(callbacks.on_sleep.is_some());
			assert!(callbacks.on_destroy.is_some());
			assert!(callbacks.on_state_change.is_some());
			assert!(callbacks.on_request.is_some());
			assert!(callbacks.on_before_connect.is_some());
			assert!(callbacks.on_connect.is_some());
			assert!(callbacks.on_disconnect.is_some());
			assert!(callbacks.run.is_some());
			assert!(callbacks.actions.contains_key("increment"));

			let migrate = callbacks
				.on_migrate
				.as_ref()
				.expect("on_migrate should be wired");
			migrate(rivetkit_core::OnMigrateRequest {
				ctx: ctx.clone(),
				is_new: true,
			})
			.await
			.expect("on_migrate should succeed");

			let wake = callbacks
				.on_wake
				.as_ref()
				.expect("on_wake should be wired");
			wake(rivetkit_core::OnWakeRequest { ctx: ctx.clone() })
				.await
				.expect("on_wake should succeed");

			let request = callbacks
				.on_request
				.as_ref()
				.expect("on_request should be wired");
			let response = request(rivetkit_core::OnRequestRequest {
				ctx: ctx.clone(),
				request: Request::new(Vec::new()),
			})
			.await
			.expect("on_request should succeed");
			assert_eq!(response.body(), b"7");

			let before_connect = callbacks
				.on_before_connect
				.as_ref()
				.expect("on_before_connect should be wired");
			before_connect(rivetkit_core::OnBeforeConnectRequest {
				ctx: ctx.clone(),
				params: super::serialize_cbor(&TestParams {
					label: "socket".to_owned(),
				})
				.expect("params should serialize"),
			})
			.await
			.expect("on_before_connect should succeed");

			let conn = ConnHandle::new(
				"conn-id",
				super::serialize_cbor(&TestParams {
					label: "socket".to_owned(),
				})
				.expect("params should serialize"),
				super::serialize_cbor(&TestConnState { count: 1 })
					.expect("conn state should serialize"),
				false,
			);
			callbacks
				.on_connect
				.as_ref()
				.expect("on_connect should be wired")(rivetkit_core::OnConnectRequest {
				ctx: ctx.clone(),
				conn: conn.clone(),
			})
			.await
			.expect("on_connect should succeed");
			callbacks
				.on_disconnect
				.as_ref()
				.expect("on_disconnect should be wired")(rivetkit_core::OnDisconnectRequest {
				ctx: ctx.clone(),
				conn: conn.clone(),
			})
			.await
			.expect("on_disconnect should succeed");

			let action = callbacks
				.actions
				.get("increment")
				.expect("increment action should be present");
			let output = action(ActionRequest {
				ctx: ctx.clone(),
				conn,
				name: "increment".to_owned(),
				args: super::serialize_cbor(&(5_i64,))
					.expect("action args should serialize"),
			})
			.await
			.expect("action should succeed");
			let output = super::deserialize_cbor::<TestState>(&output)
				.expect("action output should deserialize");
			assert_eq!(output.value, 12);
		}

		#[tokio::test]
		async fn factory_supports_unit_vars_without_create_vars_override() {
			let factory = build_factory::<UnitVarsActor>(TypedActionMap::new());
			let ctx = ActorContext::new("actor-id", "unit-vars", Vec::new(), "local");
			let callbacks = factory
				.create(FactoryRequest {
					ctx: ctx.clone(),
					input: None,
					is_new: true,
				})
				.await
				.expect("factory should build callbacks for unit vars");

			let response = callbacks
				.on_request
				.as_ref()
				.expect("on_request should be wired")(rivetkit_core::OnRequestRequest {
				ctx,
				request: Request::new(Vec::new()),
			})
			.await
			.expect("on_request should succeed");

			assert_eq!(response.body(), b"ok");
		}

		#[tokio::test]
		async fn factory_records_typed_startup_metrics() {
			let factory = build_factory::<TestActor>(TypedActionMap::new());
			let ctx = ActorContext::new("actor-id", "metrics", Vec::new(), "local");
			let input = super::serialize_cbor(&TestInput { start: 3 })
				.expect("test input should serialize");

			let _callbacks = factory
				.create(FactoryRequest {
					ctx: ctx.clone(),
					input: Some(input),
					is_new: true,
				})
				.await
				.expect("factory should build typed callbacks");

			let metrics = ctx.render_metrics().expect("render metrics");
			let create_state_line = metrics
				.lines()
				.find(|line: &&str| line.starts_with("create_state_ms"))
				.expect("create_state_ms line");
			let create_vars_line = metrics
				.lines()
				.find(|line: &&str| line.starts_with("create_vars_ms"))
				.expect("create_vars_ms line");

			assert!(!create_state_line.ends_with(" 0"));
			assert!(!create_vars_line.ends_with(" 0"));
		}

		#[tokio::test]
		async fn action_deserialization_failures_become_validation_errors() {
			let mut actions = TypedActionMap::<TestActor>::new();
			actions.insert(
				"increment".to_owned(),
				build_action(TestActor::increment),
			);
			let factory = build_factory::<TestActor>(actions);
			let callbacks = factory
				.create(FactoryRequest {
					ctx: ActorContext::new("actor-id", "test", Vec::new(), "local"),
					input: Some(
						super::serialize_cbor(&TestInput { start: 1 })
							.expect("test input should serialize"),
					),
					is_new: true,
				})
				.await
				.expect("factory should build typed callbacks");
			let action = callbacks
				.actions
				.get("increment")
				.expect("increment action should be present");
			let error = action(ActionRequest {
				ctx: ActorContext::new("actor-id", "test", Vec::new(), "local"),
				conn: ConnHandle::default(),
				name: "increment".to_owned(),
				args: vec![0xff],
			})
			.await
			.expect_err("invalid CBOR should fail");
			let error = RivetError::extract(&error);

			assert_eq!(error.group(), "actor");
			assert_eq!(error.code(), "validation_error");
			assert!(
				error.message().contains("action arguments"),
				"unexpected error message: {}",
				error.message(),
			);
		}

		#[tokio::test]
		async fn state_decode_failures_become_validation_errors() {
			let factory = build_factory::<TestActor>(TypedActionMap::new());
			let ctx = ActorContext::new("actor-id", "test", Vec::new(), "local");
			ctx.set_state(vec![0xff]);
			let callbacks = factory
				.create(FactoryRequest {
					ctx: ctx.clone(),
					input: Some(
						super::serialize_cbor(&TestInput { start: 0 })
							.expect("test input should serialize"),
					),
					is_new: false,
				})
				.await
				.expect("factory should build typed callbacks");
			let error = callbacks
				.on_request
				.as_ref()
				.expect("on_request should be wired")(rivetkit_core::OnRequestRequest {
				ctx,
				request: Request::new(Vec::new()),
			})
			.await
			.expect_err("invalid typed state should fail");
			let error = RivetError::extract(&error);

			assert_eq!(error.group(), "actor");
			assert_eq!(error.code(), "validation_error");
			assert!(
				error.message().contains("actor state"),
				"unexpected error message: {}",
				error.message(),
			);
		}
	}
