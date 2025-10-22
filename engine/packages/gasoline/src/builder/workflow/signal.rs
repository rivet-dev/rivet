use std::{fmt::Display, time::Instant};

use anyhow::Result;
use rivet_metrics::KeyValue;
use rivet_util::Id;
use serde::Serialize;

use crate::{
	builder::BuilderError, ctx::WorkflowCtx, error::WorkflowError, history::cursor::HistoryResult,
	metrics, signal::Signal, workflow::Workflow,
};

pub struct SignalBuilder<'a, T: Signal + Serialize> {
	ctx: &'a mut WorkflowCtx,
	version: usize,

	body: T,
	to_workflow_name: Option<&'static str>,
	to_workflow_id: Option<Id>,
	tags: serde_json::Map<String, serde_json::Value>,
	error: Option<BuilderError>,
}

impl<'a, T: Signal + Serialize> SignalBuilder<'a, T> {
	pub(crate) fn new(ctx: &'a mut WorkflowCtx, version: usize, body: T) -> Self {
		SignalBuilder {
			ctx,
			version,

			body,
			to_workflow_name: None,
			to_workflow_id: None,
			tags: serde_json::Map::new(),
			error: None,
		}
	}

	pub fn to_workflow_id(mut self, workflow_id: Id) -> Self {
		if self.error.is_some() {
			return self;
		}

		self.to_workflow_id = Some(workflow_id);

		self
	}

	pub fn to_workflow<W: Workflow>(mut self) -> Self {
		if self.error.is_some() {
			return self;
		}

		self.to_workflow_name = Some(W::NAME);

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

	#[tracing::instrument(skip_all, fields(signal_name=T::NAME, signal_id))]
	pub async fn send(self) -> Result<Id> {
		self.ctx.check_stop()?;

		if let Some(err) = self.error {
			return Err(err.into());
		}

		// Error for version mismatch. This is done in the builder instead of in `VersionedWorkflowCtx` to
		// defer the error.
		self.ctx.compare_version("signal", self.version)?;

		let history_res = self
			.ctx
			.cursor()
			.compare_signal_send(self.version, T::NAME)?;
		let location = self.ctx.cursor().current_location_for(&history_res);

		// Signal sent before
		let signal_id = if let HistoryResult::Event(signal) = history_res {
			tracing::debug!("replaying signal dispatch",);

			signal.signal_id
		}
		// Send signal
		else {
			let signal_id = Id::new_v1(self.ctx.config().dc_label());
			let start_instant = Instant::now();

			// Serialize input
			let input_val = serde_json::value::to_raw_value(&self.body)
				.map_err(WorkflowError::SerializeSignalBody)?;

			match (
				self.to_workflow_name,
				self.to_workflow_id,
				self.tags.is_empty(),
			) {
				(Some(workflow_name), None, _) => {
					tracing::debug!(
						to_workflow_name=%workflow_name,
						"dispatching signal via workflow name and tags"
					);

					let workflow_id = self
						.ctx
						.db()
						.find_workflow(workflow_name, &serde_json::Value::Object(self.tags))
						.await?
						.ok_or(WorkflowError::WorkflowNotFound)?;

					self.ctx
						.db()
						.publish_signal_from_workflow(
							self.ctx.workflow_id(),
							&location,
							self.version,
							self.ctx.ray_id(),
							workflow_id,
							signal_id,
							T::NAME,
							&input_val,
							self.ctx.loop_location(),
						)
						.await?;
				}
				(None, Some(workflow_id), true) => {
					tracing::debug!(
						to_workflow_id=%workflow_id,
						"dispatching signal via workflow id"
					);

					self.ctx
						.db()
						.publish_signal_from_workflow(
							self.ctx.workflow_id(),
							&location,
							self.version,
							self.ctx.ray_id(),
							workflow_id,
							signal_id,
							T::NAME,
							&input_val,
							self.ctx.loop_location(),
						)
						.await?;
				}
				(None, None, false) => {
					return Err(BuilderError::InvalidSignalSend(
						"must provide workflow when using tags",
					)
					.into());
				}
				(Some(_), Some(_), _) => {
					return Err(BuilderError::InvalidSignalSend(
						"cannot provide both workflow and workflow id",
					)
					.into());
				}
				(None, Some(_), false) => {
					return Err(BuilderError::InvalidSignalSend(
						"cannot provide tags if providing a workflow id",
					)
					.into());
				}
				(None, None, true) => {
					return Err(BuilderError::InvalidSignalSend(
						"no workflow, workflow id, or tags provided",
					)
					.into());
				}
			}

			let dt = start_instant.elapsed().as_secs_f64();
			metrics::SIGNAL_SEND_DURATION.record(
				dt,
				&[
					KeyValue::new("workflow_name", self.ctx.name().to_string()),
					KeyValue::new("signal_name", T::NAME),
				],
			);
			metrics::SIGNAL_PUBLISHED.add(
				1,
				&[
					KeyValue::new("workflow_name", self.ctx.name().to_string()),
					KeyValue::new("signal_name", T::NAME),
				],
			);

			signal_id
		};

		tracing::Span::current().record("signal_id", signal_id.to_string());

		// Move to next event
		self.ctx.cursor_mut().update(&location);

		Ok(signal_id)
	}
}
