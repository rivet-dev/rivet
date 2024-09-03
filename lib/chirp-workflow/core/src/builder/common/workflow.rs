use std::fmt::Display;

use global_error::{GlobalError, GlobalResult};
use serde::Serialize;
use uuid::Uuid;

use crate::{
	builder::BuilderError,
	ctx::common,
	db::DatabaseHandle,
	error::WorkflowError,
	workflow::{Workflow, WorkflowInput},
};

pub struct WorkflowBuilder<I: WorkflowInput> {
	db: DatabaseHandle,
	ray_id: Uuid,
	input: I,
	tags: serde_json::Map<String, serde_json::Value>,
	error: Option<BuilderError>,
}

impl<I: WorkflowInput> WorkflowBuilder<I>
where
	<I as WorkflowInput>::Workflow: Workflow<Input = I>,
{
	pub(crate) fn new(db: DatabaseHandle, ray_id: Uuid, input: I) -> Self {
		WorkflowBuilder {
			db,
			ray_id,
			input,
			tags: serde_json::Map::new(),
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

	pub async fn dispatch(self) -> GlobalResult<Uuid> {
		if let Some(err) = self.error {
			return Err(err.into());
		}

		let workflow_name = I::Workflow::NAME;
		let workflow_id = Uuid::new_v4();

		let no_tags = self.tags.is_empty();
		let tags = serde_json::Value::Object(self.tags);
		let tags = if no_tags { None } else { Some(&tags) };

		tracing::info!(
			%workflow_name,
			%workflow_id,
			?tags,
			input=?self.input,
			"dispatching workflow"
		);

		// Serialize input
		let input_val = serde_json::to_value(&self.input)
			.map_err(WorkflowError::SerializeWorkflowOutput)
			.map_err(GlobalError::raw)?;

		self.db
			.dispatch_workflow(self.ray_id, workflow_id, &workflow_name, tags, input_val)
			.await
			.map_err(GlobalError::raw)?;

		Ok(workflow_id)
	}

	pub async fn output(
		self,
	) -> GlobalResult<<<I as WorkflowInput>::Workflow as Workflow>::Output> {
		let db = self.db.clone();

		let workflow_id = self.dispatch().await?;
		common::wait_for_workflow::<I::Workflow>(&db, workflow_id).await
	}
}
