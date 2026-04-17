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
#[path = "../tests/modules/bridge.rs"]
mod tests;
