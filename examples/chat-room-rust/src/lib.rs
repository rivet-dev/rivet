use std::time::{SystemTime, UNIX_EPOCH};
use std::{future::Future, pin::Pin, sync::Arc};

use async_trait::async_trait;
use rivetkit::prelude::*;
use rivetkit::{Action, BindParam, ColumnValue, Event, Handles, action};
use serde::{Deserialize, Serialize};

pub const ACTOR_NAME: &str = "chatRoom";

type BoxFuture<T> = Pin<Box<dyn Future<Output = Result<T>> + Send>>;

pub struct ChatRoom {
	started_at_ms: i64,
}

#[derive(Default, Serialize, Deserialize)]
pub struct ChatRoomState {
	pub sent_count: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Message {
	pub sender: String,
	pub text: String,
	pub timestamp: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SendMessage {
	pub sender: String,
	pub text: String,
}

impl Action for SendMessage {
	type Output = Message;

	const NAME: &'static str = "sendMessage";
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetHistory;

impl Action for GetHistory {
	type Output = Vec<Message>;

	const NAME: &'static str = "getHistory";
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetStats;

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoomStats {
	pub sent_count: u64,
	pub started_at_ms: i64,
}

impl Action for GetStats {
	type Output = RoomStats;

	const NAME: &'static str = "getStats";
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NewMessage {
	pub message: Message,
}

impl Event for NewMessage {
	const NAME: &'static str = "newMessage";
}

#[async_trait]
impl Actor for ChatRoom {
	type State = ChatRoomState;
	type Input = ();
	type Actions = (SendMessage, GetHistory, GetStats);
	type Events = (NewMessage,);
	type Queue = ();
	type ConnParams = ();
	type ConnState = ();
	type Action = action::Raw;

	const HAS_DATABASE: bool = true;

	async fn create_state(_ctx: &Ctx<Self>, _input: Self::Input) -> Result<Self::State> {
		Ok(ChatRoomState::default())
	}

	async fn create(ctx: &Ctx<Self>) -> Result<Self> {
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
		Ok(Self {
			started_at_ms: now_ms(),
		})
	}
}

impl Handles<SendMessage> for ChatRoom {
	type Future = BoxFuture<Message>;

	fn handle(self: Arc<Self>, ctx: Ctx<Self>, action: SendMessage) -> Self::Future {
		Box::pin(async move {
			let message = send_message(&ctx, action.sender, action.text).await?;
			ctx.state_mut().sent_count += 1;
			ctx.emit(NewMessage {
				message: message.clone(),
			})?;
			Ok(message)
		})
	}
}

impl Handles<GetHistory> for ChatRoom {
	type Future = BoxFuture<Vec<Message>>;

	fn handle(self: Arc<Self>, ctx: Ctx<Self>, _action: GetHistory) -> Self::Future {
		Box::pin(async move { get_history(&ctx).await })
	}
}

impl Handles<GetStats> for ChatRoom {
	type Future = BoxFuture<RoomStats>;

	fn handle(self: Arc<Self>, ctx: Ctx<Self>, _action: GetStats) -> Self::Future {
		Box::pin(async move {
			Ok(RoomStats {
				sent_count: ctx.state().sent_count,
				started_at_ms: self.started_at_ms,
			})
		})
	}
}

pub fn registry() -> Registry {
	let mut registry = Registry::new();
	registry.register_actor::<ChatRoom>(ACTOR_NAME);
	registry
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
