// Requires a running local engine from `./scripts/run/engine-rocksdb.sh`.
// This example is not part of CI.

use rivetkit::prelude::*;
use serde::{Deserialize, Serialize};

struct Chat;

#[derive(Default, Serialize, Deserialize)]
struct ChatState {
	messages: Vec<Message>,
}

#[derive(Clone, Serialize, Deserialize)]
struct Message {
	user: String,
	text: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
enum ChatAction {
	Send { text: String },
	History,
	Kick { user_id: String },
}

impl Actor for Chat {
	type Input = ();
	type ConnParams = String;
	type ConnState = String;
	type Action = ChatAction;
}

async fn run(mut start: Start<Chat>) -> Result<()> {
	let _ = start.input.decode_or_default()?;
	let ctx = start.ctx.clone();
	let mut state: ChatState = start.snapshot.decode_or_default()?;

	while let Some(event) = start.events.recv().await {
		match event {
			Event::Action(action) => match action.decode() {
				Ok(ChatAction::Send { text }) => {
					let user = action
						.conn()
						.and_then(|conn| conn.state().ok())
						.unwrap_or_else(|| "system".to_string());
					let message = Message {
						user: user.clone(),
						text: text.clone(),
					};
					state.messages.push(message.clone());
					ctx.broadcast("message", &message)?;
					ctx.request_save(RequestSaveOpts::default());
					action.ok(&());
				}
				Ok(ChatAction::History) => action.ok(&state.messages),
				Ok(ChatAction::Kick { user_id }) => {
					for conn in ctx.conns_vec() {
						if conn
							.state()
							.ok()
							.is_some_and(|state: String| state == user_id)
						{
							let _ = conn.disconnect(Some("kicked")).await;
						}
					}
					action.ok(&());
				}
				Err(error) => action.err(error),
			},
			Event::QueueSend(queue) => queue.err(anyhow!("no queue support")),
			Event::Http(http) => http.reply_status(404),
			Event::WebSocketOpen(ws) => ws.reject(anyhow!("no websocket support")),
			Event::ConnOpen(conn) => {
				let username = conn.params()?;
				conn.accept(username);
			}
			Event::ConnClosed(closed) => {
				let _ = closed.conn.id();
			}
			Event::Subscribe(subscribe) => subscribe.allow(),
			Event::SerializeState(serialize) => serialize.save(&state),
			Event::Sleep(sleep) => {
				save_chat_state(&ctx, &state).await?;
				sleep.ok();
			}
			Event::Destroy(destroy) => {
				save_chat_state(&ctx, &state).await?;
				destroy.ok();
			}
			Event::WorkflowHistory(history) => history.reply_raw(None),
			Event::WorkflowReplay(replay) => replay.reply_raw(None),
		}
	}

	Ok(())
}

async fn save_chat_state(ctx: &Ctx<Chat>, state: &ChatState) -> Result<()> {
	ctx.save_state(rivetkit::persist::state_deltas(state)?)
		.await
}

#[tokio::main]
async fn main() -> Result<()> {
	let mut registry = Registry::new();
	registry.register::<Chat, _, _>("chat", run);
	registry.serve().await
}
