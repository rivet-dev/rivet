use std::{fmt::Display, time::Instant};

use anyhow::Result;
use rivet_metrics::KeyValue;
use serde::Serialize;

use crate::{
	builder::BuilderError, ctx::WorkflowCtx, error::WorkflowError, history::cursor::HistoryResult,
	message::Message, metrics,
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
			_ => self.error = Some(BuilderError::TagsNotMap),
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

	pub fn wait(mut self) -> Self {
		if self.error.is_some() {
			return self;
		}

		self.wait = true;

		self
	}

	#[tracing::instrument(skip_all, fields(message_name=M::NAME))]
	pub async fn send(self) -> Result<()> {
		self.ctx.check_stop()?;

		if let Some(err) = self.error {
			return Err(err.into());
		}

		// Error for version mismatch. This is done in the builder instead of in `VersionedWorkflowCtx` to
		// defer the error.
		self.ctx.compare_version("message", self.version)?;

		let history_res = self.ctx.cursor().compare_msg(self.version, M::NAME)?;
		let location = self.ctx.cursor().current_location_for(&history_res);

		// Message sent before
		if let HistoryResult::Event(_) = history_res {
			tracing::debug!("replaying message dispatch");
		}
		// Send message
		else {
			tracing::debug!(tags=?self.tags, "dispatching message");

			let start_instant = Instant::now();

			// Serialize body
			let body_val = serde_json::value::to_raw_value(&self.body)
				.map_err(WorkflowError::SerializeMessageBody)?;
			let tags = serde_json::Value::Object(self.tags);
			let tags2 = tags.clone();

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
				.await?;

			if self.wait {
				self.ctx.msg_ctx().message_wait(tags2, self.body).await?;
			} else {
				self.ctx.msg_ctx().message(tags2, self.body).await?;
			}

			let dt = start_instant.elapsed().as_secs_f64();
			metrics::MESSAGE_SEND_DURATION.record(
				dt,
				&[
					KeyValue::new("workflow_name", self.ctx.name().to_string()),
					KeyValue::new("message_name", M::NAME),
				],
			);
			metrics::MESSAGE_PUBLISHED.add(
				1,
				&[
					KeyValue::new("workflow_name", self.ctx.name().to_string()),
					KeyValue::new("message_name", M::NAME),
				],
			);
		}

		// Move to next event
		self.ctx.cursor_mut().update(&location);

		Ok(())
	}
}
