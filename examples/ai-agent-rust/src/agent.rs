use std::sync::Arc;

use anyhow::Context as _;
use async_trait::async_trait;
use futures::StreamExt;
use rivetkit::prelude::*;
use rivetkit::{BindParam, ColumnValue, Handler, action};

use crate::helpers::{GetHistory, Message, MessageAdded, Response, SendMessage, stream_reply};

/// One actor per conversation. The history lives in the actor's SQLite
/// database, so it survives sleep and restarts.
pub struct Agent;

#[async_trait]
impl Actor for Agent {
	type Input = ();
	type Actions = (SendMessage, GetHistory);
	type Events = (MessageAdded, Response);
	type ConnParams = ();
	type Action = action::Raw;

	async fn create(ctx: &Ctx<Self>) -> Result<Self> {
		// Create the table that holds the conversation the first time this actor
		// starts
		ctx.sql()
			.execute(
				"CREATE TABLE IF NOT EXISTS messages (
					id INTEGER PRIMARY KEY AUTOINCREMENT,
					role TEXT NOT NULL,
					content TEXT NOT NULL
				)",
				None,
			)
			.await?;
		Ok(Self)
	}
}

impl Handler<GetHistory> for Agent {
	async fn handle(self: Arc<Self>, ctx: Ctx<Self>, _action: GetHistory) -> Result<Vec<Message>> {
		// Return the conversation in order, skipping replies whose stream failed
		// before any tokens arrived
		let rows = ctx
			.sql()
			.query(
				"SELECT id, role, content FROM messages WHERE content != '' ORDER BY id ASC",
				None,
			)
			.await?
			.rows;
		let mut history = Vec::with_capacity(rows.len());
		for row in rows {
			let [
				ColumnValue::Integer(id),
				ColumnValue::Text(role),
				ColumnValue::Text(content),
			] = row.as_slice()
			else {
				return Err(anyhow!("unexpected messages row: {row:?}"));
			};
			history.push(Message {
				id: *id,
				role: role.clone(),
				content: content.clone(),
			});
		}
		Ok(history)
	}
}

impl Handler<SendMessage> for Agent {
	async fn handle(self: Arc<Self>, ctx: Ctx<Self>, action: SendMessage) -> Result<Message> {
		// Reject empty messages before touching the database
		let content = action.content.trim().to_owned();
		if content.is_empty() {
			return Err(anyhow!("message content cannot be empty"));
		}

		// Save the user's message and broadcast it. It is saved before calling the
		// model so the model always sees the message it is replying to, even if
		// the actor restarts mid-call
		let inserted = ctx
			.sql()
			.execute(
				"INSERT INTO messages (role, content) VALUES ('user', ?)",
				Some(vec![BindParam::Text(content.clone())]),
			)
			.await?;
		ctx.emit(MessageAdded(Message {
			id: inserted
				.last_insert_row_id
				.context("insert returned no row id")?,
			role: "user".to_owned(),
			content,
		}))?;

		// Load the whole conversation. Sending the full history on every call is
		// what gives the agent memory. Empty replies are skipped because they come
		// from a stream that failed before any tokens arrived
		let rows = ctx
			.sql()
			.query(
				"SELECT id, role, content FROM messages WHERE content != '' ORDER BY id ASC",
				None,
			)
			.await?
			.rows;
		let mut history = Vec::with_capacity(rows.len());
		for row in rows {
			let [
				ColumnValue::Integer(id),
				ColumnValue::Text(role),
				ColumnValue::Text(content),
			] = row.as_slice()
			else {
				return Err(anyhow!("unexpected messages row: {row:?}"));
			};
			history.push(Message {
				id: *id,
				role: role.clone(),
				content: content.clone(),
			});
		}

		// Start streaming the model's reply
		let mut stream = stream_reply(&history).await?;

		// Add an empty assistant message for the reply to stream into
		let inserted = ctx
			.sql()
			.execute(
				"INSERT INTO messages (role, content) VALUES ('assistant', '')",
				None,
			)
			.await?;
		let message_id = inserted
			.last_insert_row_id
			.context("insert returned no row id")?;
		ctx.emit(MessageAdded(Message {
			id: message_id,
			role: "assistant".to_owned(),
			content: String::new(),
		}))?;

		// Broadcast each token as it arrives so clients can render the reply live
		let mut content = String::new();
		while let Some(chunk) = stream.next().await {
			let delta = chunk?
				.choices
				.into_iter()
				.next()
				.and_then(|choice| choice.delta.content);
			let Some(delta) = delta else {
				continue;
			};
			content.push_str(&delta);
			ctx.emit(Response {
				message_id,
				delta,
				content: content.clone(),
				done: false,
			})?;
		}

		// Save the finished reply and tell clients the stream is done
		ctx.sql()
			.execute(
				"UPDATE messages SET content = ? WHERE id = ?",
				Some(vec![
					BindParam::Text(content.clone()),
					BindParam::Integer(message_id),
				]),
			)
			.await?;
		ctx.emit(Response {
			message_id,
			delta: String::new(),
			content: content.clone(),
			done: true,
		})?;

		// Also return the reply for callers that do not subscribe to events
		Ok(Message {
			id: message_id,
			role: "assistant".to_owned(),
			content,
		})
	}
}
