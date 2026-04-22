use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use http::StatusCode;
use rivetkit::prelude::*;

const CBOR_NULL: &[u8] = &[0xf6];

#[derive(Clone, Serialize, Deserialize)]
struct CounterState {
	count: i64,
}

struct Counter {
	request_count: AtomicU64,
}

#[async_trait]
impl Actor for Counter {
	type State = CounterState;
	type ConnParams = ();
	type ConnState = ();
	type Input = ();
	type Vars = ();

	async fn create_state(_ctx: &Ctx<Self>, _input: &Self::Input) -> Result<Self::State> {
		Ok(CounterState { count: 0 })
	}

	async fn create_conn_state(
		self: &Arc<Self>,
		_ctx: &Ctx<Self>,
		_params: &Self::ConnParams,
	) -> Result<Self::ConnState> {
		let _ = self;
		Ok(())
	}

	async fn on_create(ctx: &Ctx<Self>, _input: &Self::Input) -> Result<Self> {
		initialize_schema(ctx).await?;
		Ok(Self {
			request_count: AtomicU64::new(0),
		})
	}

	async fn on_request(self: &Arc<Self>, ctx: &Ctx<Self>, _request: Request) -> Result<Response> {
		self.request_count.fetch_add(1, Ordering::Relaxed);
		let state = ctx.state();
		let body = format!("{{\"count\":{}}}", state.count).into_bytes();
		let response = http::Response::builder()
			.status(StatusCode::OK)
			.header("content-type", "application/json")
			.body(body)?;
		Ok(response.into())
	}

	async fn run(self: &Arc<Self>, ctx: &Ctx<Self>) -> Result<()> {
		let _ = self;

		loop {
			tokio::select! {
				_ = ctx.abort_signal().cancelled() => break,
				_ = tokio::time::sleep(Duration::from_secs(3600)) => {
					ctx.schedule().after(Duration::ZERO, "get_count", CBOR_NULL);
				}
			}
		}

		Ok(())
	}
}

impl Counter {
	async fn increment(self: Arc<Self>, ctx: Ctx<Self>, (amount,): (i64,)) -> Result<CounterState> {
		let _ = self;
		let mut state = (*ctx.state()).clone();
		state.count += amount;
		ctx.set_state(&state);
		ctx.broadcast("count_changed", &state);
		Ok(state)
	}

	async fn get_count(self: Arc<Self>, ctx: Ctx<Self>, _args: ()) -> Result<i64> {
		let _ = self;
		Ok(ctx.state().count)
	}
}

async fn initialize_schema(ctx: &Ctx<Counter>) -> Result<()> {
	// The public SQLite surface is still the low-level envoy page protocol.
	// Keep schema bootstrap isolated so this example can swap to a query helper later.
	let _ = (
		ctx.sql(),
		"CREATE TABLE IF NOT EXISTS log (id INTEGER PRIMARY KEY, action TEXT)",
	);
	Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
	let mut registry = Registry::new();
	registry
		.register::<Counter>("counter")
		.action("increment", Counter::increment)
		.action("get_count", Counter::get_count)
		.done();
	registry.serve().await
}
