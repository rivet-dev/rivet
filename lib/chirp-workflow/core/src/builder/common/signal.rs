use std::fmt::Display;

use global_error::{GlobalError, GlobalResult};
use serde::Serialize;
use uuid::Uuid;

use crate::{builder::BuilderError, db::DatabaseHandle, error::WorkflowError, signal::Signal};

pub struct SignalBuilder<T: Signal + Serialize> {
	db: DatabaseHandle,
	ray_id: Uuid,
	body: T,
	to_workflow_id: Option<Uuid>,
	tags: serde_json::Map<String, serde_json::Value>,
	error: Option<GlobalError>,
}

impl<T: Signal + Serialize> SignalBuilder<T> {
	pub(crate) fn new(db: DatabaseHandle, ray_id: Uuid, body: T) -> Self {
		SignalBuilder {
			db,
			ray_id,
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
			return Err(err);
		}

		let signal_id = Uuid::new_v4();

		// Serialize input
		let input_val = serde_json::to_value(&self.body)
			.map_err(WorkflowError::SerializeSignalBody)
			.map_err(GlobalError::raw)?;

		match (self.to_workflow_id, self.tags.is_empty()) {
			(Some(workflow_id), true) => {
				tracing::info!(signal_name=%T::NAME, to_workflow_id=%workflow_id, %signal_id, "dispatching signal");

				self.db
					.publish_signal(self.ray_id, workflow_id, signal_id, T::NAME, input_val)
					.await
					.map_err(GlobalError::raw)?;
			}
			(None, false) => {
				tracing::info!(signal_name=%T::NAME, tags=?self.tags, %signal_id, "dispatching tagged signal");

				self.db
					.publish_tagged_signal(
						self.ray_id,
						&serde_json::Value::Object(self.tags),
						signal_id,
						T::NAME,
						input_val,
					)
					.await
					.map_err(GlobalError::raw)?;
			}
			(Some(_), false) => return Err(BuilderError::WorkflowIdAndTags.into()),
			(None, true) => return Err(BuilderError::NoWorkflowIdOrTags.into()),
		}

		Ok(signal_id)
	}
}
