use std::{fmt::Display, sync::Arc};

use global_error::{GlobalError, GlobalResult};
use serde::Serialize;
use uuid::Uuid;

use crate::{
	builder::BuilderError,
	ctx::WorkflowCtx,
	error::WorkflowError,
	workflow::{Workflow, WorkflowInput},
};

pub struct SubWorkflowBuilder<'a, I: WorkflowInput> {
	ctx: &'a mut WorkflowCtx,
	input: I,
	tags: serde_json::Map<String, serde_json::Value>,
	error: Option<GlobalError>,
}

impl<'a, I: WorkflowInput> SubWorkflowBuilder<'a, I>
where
	<I as WorkflowInput>::Workflow: Workflow<Input = I>,
{
	pub(crate) fn new(ctx: &'a mut WorkflowCtx, input: I) -> Self {
		SubWorkflowBuilder {
			ctx,
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
			return Err(err);
		}

		let sub_workflow_name = I::Workflow::NAME;
		let sub_workflow_id = Uuid::new_v4();

		let no_tags = self.tags.is_empty();
		let tags = serde_json::Value::Object(self.tags);
		let tags = if no_tags { None } else { Some(&tags) };

		tracing::info!(
			name=%self.ctx.name(),
			id=%self.ctx.workflow_id(),
			%sub_workflow_name,
			%sub_workflow_id,
			?tags,
			input=?self.input,
			"dispatching sub workflow"
		);

		// Serialize input
		let input_val = serde_json::to_value(&self.input)
			.map_err(WorkflowError::SerializeWorkflowOutput)
			.map_err(GlobalError::raw)?;

		self.ctx
			.db()
			.dispatch_sub_workflow(
				self.ctx.ray_id(),
				self.ctx.workflow_id(),
				self.ctx.full_location().as_ref(),
				sub_workflow_id,
				&sub_workflow_name,
				tags,
				input_val,
				self.ctx.loop_location(),
			)
			.await
			.map_err(GlobalError::raw)?;

		tracing::info!(
			name=%self.ctx.name(),
			id=%self.ctx.workflow_id(),
			%sub_workflow_name,
			?sub_workflow_id,
			"sub workflow dispatched"
		);

		Ok(sub_workflow_id)
	}

	pub async fn output(
		self,
	) -> GlobalResult<<<I as WorkflowInput>::Workflow as Workflow>::Output> {
		if let Some(err) = self.error {
			return Err(err);
		}

		let no_tags = self.tags.is_empty();
		let tags = serde_json::Value::Object(self.tags);
		let tags = if no_tags { None } else { Some(&tags) };

		// Lookup workflow
		let Ok(workflow) = self.ctx.registry().get_workflow(I::Workflow::NAME) else {
			tracing::warn!(
				name=%self.ctx.name(),
				id=%self.ctx.workflow_id(),
				sub_workflow_name=%I::Workflow::NAME,
				"sub workflow not found in current registry",
			);

			// TODO(RVT-3755): If a sub workflow is dispatched, then the worker is updated to include the sub
			// worker in the registry, this will diverge in history because it will try to run the sub worker
			// in-process during the replay
			// If the workflow isn't in the current registry, dispatch the workflow instead
			let sub_workflow_id = self.ctx.dispatch_workflow_inner(tags, self.input).await?;
			let output = self
				.ctx
				.wait_for_workflow::<I::Workflow>(sub_workflow_id)
				.await?;

			return Ok(output);
		};

		tracing::info!(name=%self.ctx.name(), id=%self.ctx.workflow_id(), sub_workflow_name=%I::Workflow::NAME, "running sub workflow");

		// TODO(RVT-3756): This is redundant with the deserialization in `workflow.run` in the registry
		// Create a new branched workflow context for the sub workflow
		let mut ctx = self
			.ctx
			.with_input(Arc::new(serde_json::to_value(&self.input)?));

		// Run workflow
		let output = (workflow.run)(&mut ctx).await.map_err(GlobalError::raw)?;

		// TODO: RVT-3756
		// Deserialize output
		let output = serde_json::from_value(output)
			.map_err(WorkflowError::DeserializeWorkflowOutput)
			.map_err(GlobalError::raw)?;

		self.ctx.inc_location();

		Ok(output)
	}
}
