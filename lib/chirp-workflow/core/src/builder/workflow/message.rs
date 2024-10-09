use std::fmt::Display;

use global_error::{GlobalError, GlobalResult};
use serde::Serialize;

use crate::{
	builder::BuilderError, ctx::WorkflowCtx, error::WorkflowError, history::cursor::HistoryResult,
	message::Message,
};

pub struct MessageBuilder<'a, M: Message> {
	ctx: &'a mut WorkflowCtx,
	version: usize,

	body: M,
	tags: serde_json::Map<String, serde_json::Value>,
	wait: bool,
	error: Option<BuilderError>,
}

impl<'a, M: Message> MessageBuilder<'a, M> {
	pub(crate) fn new(ctx: &'a mut WorkflowCtx, version: usize, body: M) -> Self {
		MessageBuilder {
			ctx,
			version,

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

		// Error for version mismatch. This is done in the builder instead of in `VersionedWorkflowCtx` to
		// defer the error.
		self.ctx
			.compare_version("message", self.version)
			.map_err(GlobalError::raw)?;

		let history_res = self
			.ctx
			.cursor()
			.compare_msg(self.version, M::NAME)
			.map_err(GlobalError::raw)?;
		let location = self.ctx.cursor().current_location_for(&history_res);

		// Message sent before
		if let HistoryResult::Event(_) = history_res {
			tracing::debug!(name=%self.ctx.name(), id=%self.ctx.workflow_id(), msg_name=%M::NAME, "replaying message dispatch");
		}
		// Send message
		else {
			tracing::info!(name=%self.ctx.name(), id=%self.ctx.workflow_id(), msg_name=%M::NAME, tags=?self.tags, "dispatching message");

			// Serialize body
			let body_val = serde_json::value::to_raw_value(&self.body)
				.map_err(WorkflowError::SerializeMessageBody)
				.map_err(GlobalError::raw)?;
			let tags = serde_json::Value::Object(self.tags);
			let tags2 = tags.clone();

			let (msg, write) = tokio::join!(
				async {
					self.ctx
						.db()
						.commit_workflow_message_send_event(
							self.ctx.workflow_id(),
							&location,
							self.version,
							&tags,
							M::NAME,
							&body_val,
							self.ctx.loop_location(),
						)
						.await
				},
				async {
					if self.wait {
						self.ctx.msg_ctx().message_wait(tags2, self.body).await
					} else {
						self.ctx.msg_ctx().message(tags2, self.body).await
					}
				},
			);

			msg.map_err(GlobalError::raw)?;
			write.map_err(GlobalError::raw)?;
		}

		// Move to next event
		self.ctx.cursor_mut().update(&location);

		Ok(())
	}
}
