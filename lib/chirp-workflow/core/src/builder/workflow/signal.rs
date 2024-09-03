use std::fmt::Display;

use global_error::{GlobalError, GlobalResult};
use serde::Serialize;
use uuid::Uuid;

use crate::{
	builder::BuilderError, ctx::WorkflowCtx, error::WorkflowError, event::Event, signal::Signal,
};

pub struct SignalBuilder<'a, T: Signal + Serialize> {
	ctx: &'a mut WorkflowCtx,
	body: T,
	to_workflow_id: Option<Uuid>,
	tags: serde_json::Map<String, serde_json::Value>,
	error: Option<BuilderError>,
}

impl<'a, T: Signal + Serialize> SignalBuilder<'a, T> {
	pub(crate) fn new(ctx: &'a mut WorkflowCtx, body: T) -> Self {
		SignalBuilder {
			ctx,
			body,
			to_workflow_id: None,
			tags: serde_json::Map::new(),
			error: None,
		}
	}

	pub fn to_workflow(mut self, workflow_id: Uuid) -> Self {
		if self.error.is_some() {
			return self;
		}

		self.to_workflow_id = Some(workflow_id);

		self
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

	pub async fn send(self) -> GlobalResult<Uuid> {
		if let Some(err) = self.error {
			return Err(err.into());
		}

		let event = self.ctx.current_history_event();

		// Signal sent before
		if let Some(event) = event {
			// Validate history is consistent
			let Event::SignalSend(signal) = event else {
				return Err(WorkflowError::HistoryDiverged(format!(
					"expected {event} at {}, found signal send {}",
					self.ctx.loc(),
					T::NAME
				)))
				.map_err(GlobalError::raw);
			};

			if signal.name != T::NAME {
				return Err(WorkflowError::HistoryDiverged(format!(
					"expected {event} at {}, found signal send {}",
					self.ctx.loc(),
					T::NAME
				)))
				.map_err(GlobalError::raw);
			}

			tracing::debug!(name=%self.ctx.name(), id=%self.ctx.workflow_id(), signal_name=%signal.name, signal_id=%signal.signal_id, "replaying signal dispatch");

			Ok(signal.signal_id)
		}
		// Send signal
		else {
			let signal_id = Uuid::new_v4();

			// Serialize input
			let input_val = serde_json::to_value(&self.body)
				.map_err(WorkflowError::SerializeSignalBody)
				.map_err(GlobalError::raw)?;

			match (self.to_workflow_id, self.tags.is_empty()) {
				(Some(workflow_id), true) => {
					tracing::info!(name=%self.ctx.name(), id=%self.ctx.workflow_id(), signal_name=%T::NAME, to_workflow_id=%workflow_id, %signal_id, "dispatching signal");

					self.ctx
						.db()
						.publish_signal_from_workflow(
							self.ctx.workflow_id(),
							self.ctx.full_location().as_ref(),
							self.ctx.ray_id(),
							workflow_id,
							signal_id,
							T::NAME,
							input_val,
							self.ctx.loop_location(),
						)
						.await
						.map_err(GlobalError::raw)?;
				}
				(None, false) => {
					tracing::info!(name=%self.ctx.name(), id=%self.ctx.workflow_id(), signal_name=%T::NAME, tags=?self.tags, %signal_id, "dispatching tagged signal");

					self.ctx
						.db()
						.publish_tagged_signal_from_workflow(
							self.ctx.workflow_id(),
							self.ctx.full_location().as_ref(),
							self.ctx.ray_id(),
							&serde_json::Value::Object(self.tags),
							signal_id,
							T::NAME,
							input_val,
							self.ctx.loop_location(),
						)
						.await
						.map_err(GlobalError::raw)?;
				}
				(Some(_), false) => return Err(BuilderError::WorkflowIdAndTags.into()),
				(None, true) => return Err(BuilderError::NoWorkflowIdOrTags.into()),
			}

			// Move to next event
			self.ctx.inc_location();

			Ok(signal_id)
		}
	}
}
