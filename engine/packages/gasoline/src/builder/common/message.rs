use std::time::Instant;

use anyhow::Result;

use crate::{
	builder::BuilderError, ctx::MessageCtx, message::Message, metrics, utils::topic::AsTopic,
};

pub struct MessageBuilder<M: Message> {
	msg_ctx: MessageCtx,
	body: M,
	topic: Option<String>,
	wait: bool,
	error: Option<BuilderError>,
}

impl<M: Message> MessageBuilder<M> {
	pub(crate) fn new(msg_ctx: MessageCtx, body: M) -> Self {
		MessageBuilder {
			msg_ctx,
			body,
			topic: None,
			wait: false,
			error: None,
		}
	}

	pub fn topic(mut self, topic: impl AsTopic) -> Self {
		if self.error.is_some() {
			return self;
		}

		self.topic = Some(topic.as_topic());

		self
	}

	pub fn wait(mut self) -> Self {
		if self.error.is_some() {
			return self;
		}

		self.wait = true;

		self
	}

	#[tracing::instrument(skip_all, fields(message_name=M::NAME))]
	pub async fn send(self) -> Result<()> {
		if let Some(err) = self.error {
			return Err(err.into());
		}

		tracing::debug!(topic=?self.topic, "dispatching message");

		let topic = self.topic.unwrap_or_else(|| "*".to_string());

		let start_instant = Instant::now();

		if self.wait {
			self.msg_ctx.message_wait(&topic, self.body).await?;
		} else {
			self.msg_ctx.message(&topic, self.body).await?;
		}

		let dt = start_instant.elapsed().as_secs_f64();
		metrics::MESSAGE_SEND_DURATION
			.with_label_values(&["", M::NAME])
			.observe(dt);
		metrics::MESSAGE_PUBLISHED
			.with_label_values(&["", M::NAME])
			.inc();

		Ok(())
	}
}
