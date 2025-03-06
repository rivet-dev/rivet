use std::fmt::Display;

use global_error::{GlobalError, GlobalResult};
use serde::Serialize;
use uuid::Uuid;

use crate::{
	builder::BuilderError, db::DatabaseHandle, error::WorkflowError, metrics, signal::Signal,
	workflow::Workflow,
};

pub struct SignalBuilder<T: Signal + Serialize> {
	db: DatabaseHandle,
	ray_id: Uuid,
	body: T,
	to_workflow_name: Option<&'static str>,
	to_workflow_id: Option<Uuid>,
	tags: serde_json::Map<String, serde_json::Value>,
	error: Option<BuilderError>,
}

impl<T: Signal + Serialize> SignalBuilder<T> {
	pub(crate) fn new(db: DatabaseHandle, ray_id: Uuid, body: T) -> Self {
		SignalBuilder {
			db,
			ray_id,
			body,
			to_workflow_name: None,
			to_workflow_id: None,
			tags: serde_json::Map::new(),
			error: None,
		}
	}

	pub fn to_workflow_id(mut self, workflow_id: Uuid) -> Self {
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

	pub async fn send(self) -> GlobalResult<Uuid> {
		if let Some(err) = self.error {
			return Err(err.into());
		}

		let signal_id = Uuid::new_v4();

		// Serialize input
		let input_val = serde_json::value::to_raw_value(&self.body)
			.map_err(WorkflowError::SerializeSignalBody)
			.map_err(GlobalError::raw)?;

		match (
			self.to_workflow_name,
			self.to_workflow_id,
			self.tags.is_empty(),
		) {
			(Some(workflow_name), None, _) => {
				tracing::debug!(
					signal_name=%T::NAME,
					to_workflow_name=%workflow_name,
					tags=?self.tags,
					%signal_id,
					"dispatching signal via workflow name and tags"
				);

				let workflow_id = self
					.db
					.find_workflow(workflow_name, &serde_json::Value::Object(self.tags))
					.await?
					.ok_or(WorkflowError::WorkflowNotFound)
					.map_err(GlobalError::raw)?;

				self.db
					.publish_signal(self.ray_id, workflow_id, signal_id, T::NAME, &input_val)
					.await
					.map_err(GlobalError::raw)?;
			}
			(None, Some(workflow_id), true) => {
				tracing::debug!(signal_name=%T::NAME, to_workflow_id=%workflow_id, %signal_id, "dispatching signal via workflow id");

				self.db
					.publish_signal(self.ray_id, workflow_id, signal_id, T::NAME, &input_val)
					.await
					.map_err(GlobalError::raw)?;
			}
			(None, None, false) => {
				tracing::debug!(signal_name=%T::NAME, tags=?self.tags, %signal_id, "dispatching tagged signal");

				self.db
					.publish_tagged_signal(
						self.ray_id,
						&serde_json::Value::Object(self.tags),
						signal_id,
						T::NAME,
						&input_val,
					)
					.await
					.map_err(GlobalError::raw)?;
			}
			(Some(_), Some(_), _) => {
				return Err(BuilderError::InvalidSignalSend(
					"cannot provide both workflow and workflow id",
				)
				.into())
			}
			(None, Some(_), false) => {
				return Err(BuilderError::InvalidSignalSend(
					"cannot provide tags if providing a workflow id",
				)
				.into())
			}
			(None, None, true) => {
				return Err(BuilderError::InvalidSignalSend(
					"no workflow, workflow id, or tags provided",
				)
				.into())
			}
		}

		metrics::SIGNAL_PUBLISHED
			.with_label_values(&[T::NAME])
			.inc();

		Ok(signal_id)
	}
}
