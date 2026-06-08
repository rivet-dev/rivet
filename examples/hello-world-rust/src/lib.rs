use std::{future::Future, pin::Pin, sync::Arc};

use async_trait::async_trait;
use rivetkit::prelude::*;
use rivetkit::{Action, Event, Handles, action};
use serde::{Deserialize, Serialize};

pub const ACTOR_NAME: &str = "counter";

type BoxFuture<T> = Pin<Box<dyn Future<Output = Result<T>> + Send>>;

pub struct Counter;

#[derive(Default, Serialize, Deserialize)]
pub struct CounterState {
	pub count: i64,
}

#[derive(Default, Serialize, Deserialize)]
pub struct CounterConnParams {
	pub label: String,
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct CounterConnState {
	pub label: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Increment {
	pub amount: i64,
}

impl Action for Increment {
	type Output = i64;

	const NAME: &'static str = "increment";
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetCount;

impl Action for GetCount {
	type Output = i64;

	const NAME: &'static str = "getCount";
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetConnLabel;

impl Action for GetConnLabel {
	type Output = String;

	const NAME: &'static str = "getConnLabel";
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NewCount {
	pub count: i64,
}

impl Event for NewCount {
	const NAME: &'static str = "newCount";
}

#[async_trait]
impl Actor for Counter {
	type State = CounterState;
	type Input = ();
	type Actions = (Increment, GetCount, GetConnLabel);
	type Events = (NewCount,);
	type Queue = ();
	type ConnParams = CounterConnParams;
	type ConnState = CounterConnState;
	type Action = action::Raw;

	async fn create_state(_ctx: &Ctx<Self>, _input: Self::Input) -> Result<Self::State> {
		Ok(CounterState::default())
	}

	async fn create(_ctx: &Ctx<Self>) -> Result<Self> {
		Ok(Self)
	}

	async fn create_conn_state(
		self: Arc<Self>,
		_ctx: Ctx<Self>,
		params: Self::ConnParams,
	) -> Result<Self::ConnState> {
		Ok(CounterConnState {
			label: params.label,
		})
	}
}

impl Handles<Increment> for Counter {
	type Future = BoxFuture<i64>;

	fn handle(self: Arc<Self>, ctx: Ctx<Self>, action: Increment) -> Self::Future {
		Box::pin(async move {
			let count = {
				let mut state = ctx.state_mut();
				state.count += action.amount;
				state.count
			};
			ctx.emit(NewCount { count })?;
			Ok(count)
		})
	}
}

impl Handles<GetCount> for Counter {
	type Future = BoxFuture<i64>;

	fn handle(self: Arc<Self>, ctx: Ctx<Self>, _action: GetCount) -> Self::Future {
		Box::pin(async move { Ok(ctx.state().count) })
	}
}

impl Handles<GetConnLabel> for Counter {
	type Future = BoxFuture<String>;

	fn handle(self: Arc<Self>, ctx: Ctx<Self>, _action: GetConnLabel) -> Self::Future {
		Box::pin(async move {
			Ok(ctx
				.conn()
				.map(|conn| conn.state())
				.transpose()?
				.map(|state| state.label)
				.unwrap_or_default())
		})
	}
}

pub fn registry() -> Registry {
	let mut registry = Registry::new();
	registry.register_actor::<Counter>(ACTOR_NAME);
	registry
}
