use std::time::{SystemTime, UNIX_EPOCH};

use rivetkit::prelude::*;
use rivetkit::{ActorConfig, BindParam, ColumnValue};
use serde::{Deserialize, Serialize};

pub const ACTOR_NAME: &str = "chatRoom";

struct ChatRoom;

#[derive(Clone, Serialize, Deserialize)]
struct Message {
	sender: String,
	text: String,
	timestamp: i64,
}

// Callable functions from clients: https://rivet.dev/docs/actors/actions
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
enum ChatRoomAction {
	SendMessage(String, String),
	GetHistory,
}

impl Actor for ChatRoom {
	type Input = ();
	type ConnParams = ();
	type ConnState = ();
	type Action = ChatRoomAction;
}

pub fn registry() -> Registry {
	let mut registry = Registry::new();
	// Declare a SQLite database for this actor: https://rivet.dev/docs/actors/sqlite
	registry.register_with::<ChatRoom, _, _>(
		ACTOR_NAME,
		ActorConfig {
			has_database: true,
			..Default::default()
		},
		run,
	);
	registry
}

async fn run(mut start: Start<ChatRoom>) -> Result<()> {
	let ctx = start.ctx.clone();

	// Create the messages table on wake.
	ctx.sql()
		.execute(
			"CREATE TABLE IF NOT EXISTS messages (
				id INTEGER PRIMARY KEY AUTOINCREMENT,
				sender TEXT NOT NULL,
				text TEXT NOT NULL,
				timestamp INTEGER NOT NULL
			)",
			None,
		)
		.await?;

	while let Some(event) = start.events.recv().await {
		match event {
			Event::Action(action) => match action.decode() {
				Ok(ChatRoomAction::SendMessage(sender, text)) => {
					match send_message(&ctx, sender, text).await {
						Ok(message) => {
							// Send events to all connected clients: https://rivet.dev/docs/actors/events
							ctx.broadcast("newMessage", &message)?;
							action.ok(&message);
						}
						Err(error) => action.err(error),
					}
				}
				Ok(ChatRoomAction::GetHistory) => match get_history(&ctx).await {
					Ok(messages) => action.ok(&messages),
					Err(error) => action.err(error),
				},
				Err(error) => action.err(error),
			},
			// Messages live in SQLite, so there is no actor state to serialize.
			Event::SerializeState(serialize) => serialize.skip(),
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

async fn send_message(ctx: &Ctx<ChatRoom>, sender: String, text: String) -> Result<Message> {
	let timestamp = now_ms();
	ctx.sql()
		.execute(
			"INSERT INTO messages (sender, text, timestamp) VALUES (?, ?, ?)",
			Some(vec![
				BindParam::Text(sender.clone()),
				BindParam::Text(text.clone()),
				BindParam::Integer(timestamp),
			]),
		)
		.await?;
	Ok(Message {
		sender,
		text,
		timestamp,
	})
}

async fn get_history(ctx: &Ctx<ChatRoom>) -> Result<Vec<Message>> {
	let result = ctx
		.sql()
		.query(
			"SELECT sender, text, timestamp FROM messages ORDER BY id ASC",
			None,
		)
		.await?;
	let mut messages = Vec::with_capacity(result.rows.len());
	for row in result.rows {
		messages.push(Message {
			sender: column_text(row.first())?,
			text: column_text(row.get(1))?,
			timestamp: column_int(row.get(2))?,
		});
	}
	Ok(messages)
}

fn column_text(value: Option<&ColumnValue>) -> Result<String> {
	match value {
		Some(ColumnValue::Text(text)) => Ok(text.clone()),
		other => Err(anyhow!("expected text column, got {other:?}")),
	}
}

fn column_int(value: Option<&ColumnValue>) -> Result<i64> {
	match value {
		Some(ColumnValue::Integer(int)) => Ok(*int),
		other => Err(anyhow!("expected integer column, got {other:?}")),
	}
}

fn now_ms() -> i64 {
	SystemTime::now()
		.duration_since(UNIX_EPOCH)
		.map(|d| d.as_millis() as i64)
		.unwrap_or_default()
}
