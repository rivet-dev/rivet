use std::sync::LazyLock;

use async_openai::{
	Client,
	config::OpenAIConfig,
	types::chat::{
		ChatCompletionRequestAssistantMessage, ChatCompletionRequestMessage,
		ChatCompletionRequestSystemMessage, ChatCompletionRequestUserMessage,
		ChatCompletionResponseStream, CreateChatCompletionRequestArgs,
	},
};
use rivetkit::prelude::*;
use rivetkit::{Action, Event};
use serde::{Deserialize, Serialize};

// The model the agent talks to

const MODEL: &str = "gpt-4o-mini";
const SYSTEM_PROMPT: &str =
	"You are a helpful assistant living inside a Rivet Actor. Keep replies short.";

/// Reads `OPENAI_API_KEY` and `OPENAI_BASE_URL` from the environment.
static OPENAI: LazyLock<Client<OpenAIConfig>> = LazyLock::new(Client::new);

// Messages, actions and events shared with clients

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Message {
	pub id: i64,
	pub role: String,
	pub content: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SendMessage {
	pub content: String,
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

/// Broadcast for each new message. The assistant's message starts empty and
/// fills in through `response` events.
#[derive(Debug, Serialize, Deserialize)]
pub struct MessageAdded(pub Message);

impl Event for MessageAdded {
	const NAME: &'static str = "messageAdded";
}

/// One streamed token of the assistant's reply. `content` is the reply so far,
/// and the last event has `done` set.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Response {
	pub message_id: i64,
	pub delta: String,
	pub content: String,
	pub done: bool,
}

impl Event for Response {
	const NAME: &'static str = "response";
}

/// Starts streaming the model's reply to `history`, with the system prompt first.
pub(crate) async fn stream_reply(history: &[Message]) -> Result<ChatCompletionResponseStream> {
	let mut messages: Vec<ChatCompletionRequestMessage> =
		vec![ChatCompletionRequestSystemMessage::from(SYSTEM_PROMPT).into()];
	for message in history {
		let content = message.content.clone();
		let message = match message.role.as_str() {
			"assistant" => ChatCompletionRequestAssistantMessage::from(content).into(),
			_ => ChatCompletionRequestUserMessage::from(content).into(),
		};
		messages.push(message);
	}

	let request = CreateChatCompletionRequestArgs::default()
		.model(MODEL)
		.messages(messages)
		.build()?;
	Ok(OPENAI.chat().create_stream(request).await?)
}
