use std::fmt::Display;

use global_error::GlobalResult;
use serde::Serialize;

use crate::{builder::BuilderError, ctx::MessageCtx, message::Message};

pub struct MessageBuilder<'a, M: Message> {
	msg_ctx: &'a MessageCtx,
	body: M,
	tags: serde_json::Map<String, serde_json::Value>,
	wait: bool,
	error: Option<BuilderError>,
}

impl<'a, M: Message> MessageBuilder<'a, M> {
	pub(crate) fn new(msg_ctx: &'a MessageCtx, body: M) -> Self {
		MessageBuilder {
			msg_ctx,
			body,
			tags: serde_json::Map::new(),
			wait: false,
			error: None,
		}
	}

	pub fn tags(mut self, tags: serde_json::Value) -> Self {
		if self.error.is_some() {
			return self;
		}

		match tags {
			serde_json::Value::Object(map) => {
				self.tags.extend(map);
			}
			_ => self.error = Some(BuilderError::TagsNotMap.into()),
		}

		self
	}

	pub fn tag(mut self, k: impl Display, v: impl Serialize) -> Self {
		if self.error.is_some() {
			return self;
		}

		match serde_json::to_value(&v) {
			Ok(v) => {
				self.tags.insert(k.to_string(), v);
			}
			Err(err) => self.error = Some(err.into()),
		}

		self
	}

	pub async fn wait(mut self) -> Self {
		if self.error.is_some() {
			return self;
		}

		self.wait = true;

		self
	}

	pub async fn send(self) -> GlobalResult<()> {
		if let Some(err) = self.error {
			return Err(err.into());
		}

		tracing::info!(msg_name=%M::NAME, tags=?self.tags, "dispatching message");

		let tags = serde_json::Value::Object(self.tags);

		if self.wait {
			self.msg_ctx.message_wait(tags, self.body).await?;
		} else {
			self.msg_ctx.message(tags, self.body).await?;
		}

		Ok(())
	}
}
