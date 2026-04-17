use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use anyhow::{Context, Result};
use ciborium::{de::from_reader, ser::into_writer};
use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::actor::Actor;
use crate::context::{ConnCtx, Ctx};
use rivetkit_core::{
	ActionRequest, ActorFactory, ActorInstanceCallbacks, FactoryRequest,
	OnBeforeConnectRequest, OnConnectRequest, OnDestroyRequest,
	OnDisconnectRequest, OnRequestRequest, OnSleepRequest, OnStateChangeRequest,
	OnWakeRequest, OnWebSocketRequest, RunRequest,
};

type BridgeFuture<T> = Pin<Box<dyn Future<Output = Result<T>> + Send + 'static>>;
pub(crate) type TypedAction<A> =
	Arc<dyn Fn(Arc<A>, Ctx<A>, Vec<u8>) -> BridgeFuture<Vec<u8>> + Send + Sync>;
pub(crate) type TypedActionMap<A> = HashMap<String, TypedAction<A>>;

const CBOR_NULL: &[u8] = &[0xf6];

pub(crate) fn build_action<A, Args, Ret, F, Fut>(handler: F) -> TypedAction<A>
where
	A: Actor,
	Args: DeserializeOwned + Send + 'static,
	Ret: Serialize + Send + 'static,
	F: Fn(Arc<A>, Ctx<A>, Args) -> Fut + Send + Sync + 'static,
	Fut: Future<Output = Result<Ret>> + Send + 'static,
{
	let handler = Arc::new(handler);
	Arc::new(move |actor, ctx, raw_args| {
		let handler = Arc::clone(&handler);
		Box::pin(async move {
			let args = deserialize_cbor::<Args>(&raw_args)
				.context("deserialize action arguments from CBOR")?;
			let output = handler(actor, ctx, args).await?;
			serialize_cbor(&output).context("serialize action output to CBOR")
		})
	})
}

pub(crate) fn build_factory<A>(actions: TypedActionMap<A>) -> ActorFactory
where
	A: Actor,
{
	let actions = Arc::new(actions);

	ActorFactory::new(A::config(), move |request| {
		let actions = Arc::clone(&actions);
		Box::pin(async move { create_callbacks::<A>(request, actions).await })
	})
}

async fn create_callbacks<A>(
	request: FactoryRequest,
	actions: Arc<TypedActionMap<A>>,
) -> Result<ActorInstanceCallbacks>
where
	A: Actor,
{
	let input = deserialize_input::<A::Input>(request.input.as_deref())?;
	let ctx = Ctx::<A>::new_bootstrap(request.ctx.clone());

	if request.is_new {
		let state = A::create_state(&ctx, &input)
			.await
			.context("create typed actor state")?;
		ctx.set_state(&state);
	}

	let vars = Arc::new(create_vars::<A>(&ctx).await?);
	ctx.initialize_vars(vars);

	let actor = Arc::new(
		A::on_create(&ctx, &input)
			.await
			.context("construct typed actor instance")?,
	);

	let mut callbacks = ActorInstanceCallbacks::default();
	callbacks.on_wake = Some(wrap_lifecycle({
		let actor = Arc::clone(&actor);
		let ctx = ctx.clone();
		move |_request: OnWakeRequest| {
			let actor = Arc::clone(&actor);
			let ctx = ctx.clone();
			async move { actor.on_wake(&ctx).await }
		}
	}));
	callbacks.on_sleep = Some(wrap_lifecycle({
		let actor = Arc::clone(&actor);
		let ctx = ctx.clone();
		move |_request: OnSleepRequest| {
			let actor = Arc::clone(&actor);
			let ctx = ctx.clone();
			async move { actor.on_sleep(&ctx).await }
		}
	}));
	callbacks.on_destroy = Some(wrap_lifecycle({
		let actor = Arc::clone(&actor);
		let ctx = ctx.clone();
		move |_request: OnDestroyRequest| {
			let actor = Arc::clone(&actor);
			let ctx = ctx.clone();
			async move { actor.on_destroy(&ctx).await }
		}
	}));
	callbacks.on_state_change = Some(wrap_lifecycle({
		let actor = Arc::clone(&actor);
		let ctx = ctx.clone();
		move |request: OnStateChangeRequest| {
			let actor = Arc::clone(&actor);
			let ctx = ctx.clone();
			async move {
				let _ = request.new_state;
				ctx.invalidate_state_cache();
				actor.on_state_change(&ctx).await
			}
		}
	}));
	callbacks.on_request = Some(Box::new({
		let actor = Arc::clone(&actor);
		let ctx = ctx.clone();
		move |request: OnRequestRequest| {
			let actor = Arc::clone(&actor);
			let ctx = ctx.clone();
			Box::pin(async move { actor.on_request(&ctx, request.request).await })
		}
	}));
	callbacks.on_websocket = Some(wrap_lifecycle({
		let actor = Arc::clone(&actor);
		let ctx = ctx.clone();
		move |request: OnWebSocketRequest| {
			let actor = Arc::clone(&actor);
			let ctx = ctx.clone();
			async move { actor.on_websocket(&ctx, request.ws).await }
		}
	}));
	callbacks.on_before_connect = Some(wrap_lifecycle({
		let actor = Arc::clone(&actor);
		let ctx = ctx.clone();
		move |request: OnBeforeConnectRequest| {
			let actor = Arc::clone(&actor);
			let ctx = ctx.clone();
			async move {
				let params = deserialize_cbor::<A::ConnParams>(&request.params)
					.context("deserialize connection params from CBOR")?;
				actor.on_before_connect(&ctx, &params).await
			}
		}
	}));
	callbacks.on_connect = Some(wrap_lifecycle({
		let actor = Arc::clone(&actor);
		let ctx = ctx.clone();
		move |request: OnConnectRequest| {
			let actor = Arc::clone(&actor);
			let ctx = ctx.clone();
			async move { actor.on_connect(&ctx, ConnCtx::new(request.conn)).await }
		}
	}));
	callbacks.on_disconnect = Some(wrap_lifecycle({
		let actor = Arc::clone(&actor);
		let ctx = ctx.clone();
		move |request: OnDisconnectRequest| {
			let actor = Arc::clone(&actor);
			let ctx = ctx.clone();
			async move { actor.on_disconnect(&ctx, ConnCtx::new(request.conn)).await }
		}
	}));
	callbacks.run = Some(wrap_lifecycle({
		let actor = Arc::clone(&actor);
		let ctx = ctx.clone();
		move |_request: RunRequest| {
			let actor = Arc::clone(&actor);
			let ctx = ctx.clone();
			async move { actor.run(&ctx).await }
		}
	}));

	for (name, action) in actions.iter() {
		callbacks.actions.insert(
			name.clone(),
			Box::new({
				let actor = Arc::clone(&actor);
				let ctx = ctx.clone();
				let action = Arc::clone(action);
				move |request: ActionRequest| {
					let actor = Arc::clone(&actor);
					let ctx = ctx.clone();
					let action = Arc::clone(&action);
					Box::pin(async move {
						let _ = (request.ctx, request.conn, request.name);
						action(actor, ctx, request.args).await
					})
				}
			}),
		);
	}

	Ok(callbacks)
}

fn wrap_lifecycle<T, F, Fut>(callback: F) -> Box<dyn Fn(T) -> BridgeFuture<()> + Send + Sync>
where
	T: Send + 'static,
	F: Fn(T) -> Fut + Send + Sync + 'static,
	Fut: Future<Output = Result<()>> + Send + 'static,
{
	Box::new(move |request| Box::pin(callback(request)))
}

async fn create_vars<A>(ctx: &Ctx<A>) -> Result<A::Vars>
where
	A: Actor,
{
	if TypeId::of::<A::Vars>() == TypeId::of::<()>() {
		return downcast_unit::<A::Vars>()
			.context("construct unit typed actor vars");
	}

	A::create_vars(ctx)
		.await
		.context("create typed actor vars")
}

fn deserialize_input<T>(bytes: Option<&[u8]>) -> Result<T>
where
	T: DeserializeOwned,
{
	let bytes = bytes.unwrap_or(CBOR_NULL);
	deserialize_cbor(bytes).context("deserialize actor input from CBOR")
}

fn serialize_cbor<T>(value: &T) -> Result<Vec<u8>>
where
	T: Serialize,
{
	let mut bytes = Vec::new();
	into_writer(value, &mut bytes)?;
	Ok(bytes)
}

fn deserialize_cbor<T>(bytes: &[u8]) -> Result<T>
where
	T: DeserializeOwned,
{
	Ok(from_reader(bytes)?)
}

fn downcast_unit<T>() -> Result<T>
where
	T: 'static,
{
	let value: Box<dyn Any> = Box::new(());
	Ok(*value
		.downcast::<T>()
		.map_err(|_| anyhow::anyhow!("failed to downcast unit vars"))?)
}

#[cfg(test)]
mod tests {
	use std::sync::Arc;
	use std::sync::atomic::{AtomicUsize, Ordering};

	use anyhow::Result;
	use async_trait::async_trait;
	use http::{Request, Response};
	use serde::{Deserialize, Serialize};

	use super::{TypedActionMap, build_action, build_factory};
	use crate::actor::Actor;
	use crate::context::Ctx;
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
				wake_count: AtomicUsize::new(0),
			})
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
			_request: Request<Vec<u8>>,
		) -> Result<Response<Vec<u8>>> {
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
			_request: Request<Vec<u8>>,
		) -> Result<Response<Vec<u8>>> {
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
		assert!(callbacks.on_sleep.is_some());
		assert!(callbacks.on_destroy.is_some());
		assert!(callbacks.on_state_change.is_some());
		assert!(callbacks.on_request.is_some());
		assert!(callbacks.on_before_connect.is_some());
		assert!(callbacks.on_connect.is_some());
		assert!(callbacks.on_disconnect.is_some());
		assert!(callbacks.run.is_some());
		assert!(callbacks.actions.contains_key("increment"));

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
}
