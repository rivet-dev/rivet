use rivetkit::prelude::*;
use serde::{Deserialize, Serialize};

pub const ACTOR_NAME: &str = "counter";

struct Counter;

// Persistent state that survives restarts: https://rivet.dev/docs/actors/state
#[derive(Default, Serialize, Deserialize)]
struct CounterState {
	count: i64,
}

// Callable functions from clients: https://rivet.dev/docs/actors/actions
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
enum CounterAction {
	// Single positional argument arrives as a one-element tuple.
	Increment((i64,)),
	GetCount,
}

impl Actor for Counter {
	type Input = ();
	type ConnParams = ();
	type ConnState = ();
	type Action = CounterAction;
}

pub fn registry() -> Registry {
	let mut registry = Registry::new();
	registry.register::<Counter, _, _>(ACTOR_NAME, run);
	registry
}

async fn run(mut start: Start<Counter>) -> Result<()> {
	let ctx = start.ctx.clone();
	let mut state: CounterState = start.snapshot.decode_or_default()?;

	while let Some(event) = start.events.recv().await {
		match event {
			Event::Action(action) => match action.decode() {
				Ok(CounterAction::Increment((amount,))) => {
					state.count += amount;
					// Send events to all connected clients: https://rivet.dev/docs/actors/events
					ctx.broadcast("newCount", &state.count)?;
					ctx.request_save(RequestSaveOpts::default());
					action.ok(&state.count);
				}
				Ok(CounterAction::GetCount) => action.ok(&state.count),
				Err(error) => action.err(error),
			},
			// Persist the latest state whenever the runtime asks.
			Event::SerializeState(serialize) => serialize.save(&state),
			Event::ConnOpen(conn) => conn.accept(()),
			Event::Subscribe(subscribe) => subscribe.allow(),
			Event::ConnClosed(_) => {}
			Event::Http(http) => http.reply_status(404),
			Event::WebSocketOpen(ws) => ws.reject(anyhow!("websockets not supported")),
			Event::QueueSend(queue) => queue.err(anyhow!("queues not supported")),
			Event::Sleep(sleep) => sleep.ok(),
			Event::Destroy(destroy) => destroy.ok(),
		}
	}

	Ok(())
}
