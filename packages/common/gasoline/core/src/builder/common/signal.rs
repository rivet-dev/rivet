use std::{fmt::Display, time::Instant};

use anyhow::Result;
use rivet_metrics::KeyValue;
use rivet_util::Id;
use serde::Serialize;

use crate::{
	builder::BuilderError, db::DatabaseHandle, error::WorkflowError, metrics, signal::Signal,
	workflow::Workflow,
};

pub struct SignalBuilder<T: Signal + Serialize> {
	db: DatabaseHandle,
	config: rivet_config::Config,
	ray_id: Id,
	body: T,
	to_workflow_name: Option<&'static str>,
	to_workflow_id: Option<Id>,
	tags: serde_json::Map<String, serde_json::Value>,
	error: Option<BuilderError>,
}

impl<T: Signal + Serialize> SignalBuilder<T> {
	pub(crate) fn new(
		db: DatabaseHandle,
		config: rivet_config::Config,
		ray_id: Id,
		body: T,
		from_workflow: bool,
	) -> Self {
		SignalBuilder {
			config,
			db,
			ray_id,
			body,
			to_workflow_name: None,
			to_workflow_id: None,
			tags: serde_json::Map::new(),
			error: from_workflow.then_some(BuilderError::CannotDispatchFromOpInWorkflow),
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
		if let Some(err) = self.error {
			return Err(err.into());
		}

		let signal_id = Id::new_v1(self.config.dc_label());
		let start_instant = Instant::now();

		tracing::Span::current().record("signal_id", signal_id.to_string());

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
					tags=?self.tags,
					"dispatching signal via workflow name and tags"
				);

				let workflow_id = self
					.db
					.find_workflow(workflow_name, &serde_json::Value::Object(self.tags))
					.await?
					.ok_or(WorkflowError::WorkflowNotFound)?;

				self.db
					.publish_signal(self.ray_id, workflow_id, signal_id, T::NAME, &input_val)
					.await?;
			}
			(None, Some(workflow_id), true) => {
				tracing::debug!(to_workflow_id=%workflow_id, "dispatching signal via workflow id");

				self.db
					.publish_signal(self.ray_id, workflow_id, signal_id, T::NAME, &input_val)
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
				KeyValue::new("workflow_name", ""),
				KeyValue::new("signal_name", T::NAME),
			],
		);
		metrics::SIGNAL_PUBLISHED.add(
			1,
			&[
				KeyValue::new("workflow_name", ""),
				KeyValue::new("signal_name", T::NAME),
			],
		);

		Ok(signal_id)
	}
}
