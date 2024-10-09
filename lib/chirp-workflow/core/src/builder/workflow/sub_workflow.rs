use std::{fmt::Display, sync::Arc};

use global_error::{GlobalError, GlobalResult};
use serde::Serialize;
use uuid::Uuid;

use crate::{
	builder::BuilderError,
	ctx::WorkflowCtx,
	error::{WorkflowError, WorkflowResult},
	event::Event,
	workflow::{Workflow, WorkflowInput},
};

pub struct SubWorkflowBuilder<'a, I: WorkflowInput> {
	ctx: &'a mut WorkflowCtx,
	input: I,
	tags: serde_json::Map<String, serde_json::Value>,
	error: Option<BuilderError>,
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
			return Err(err.into());
		}

		let tags = if self.tags.is_empty() {
			None
		} else {
			Some(serde_json::Value::Object(self.tags))
		};

		Self::dispatch_workflow_inner(self.ctx, self.input, tags)
			.await
			.map_err(GlobalError::raw)
	}

	pub async fn output(
		self,
	) -> GlobalResult<<<I as WorkflowInput>::Workflow as Workflow>::Output> {
		if let Some(err) = self.error {
			return Err(err.into());
		}

		if !self.tags.is_empty() {
			return Err(
				BuilderError::TagsOnSubWorkflowOutputNotSupported(I::Workflow::NAME).into(),
			);
		}

		tracing::info!(name=%self.ctx.name(), id=%self.ctx.workflow_id(), sub_workflow_name=%I::Workflow::NAME, "running sub workflow");

		let mut ctx = self
			.ctx
			.with_input(Arc::new(serde_json::to_value(&self.input)?));

		// Run workflow
		let output =
			<<I as WorkflowInput>::Workflow as Workflow>::run(&mut ctx, &self.input).await?;

		self.ctx.inc_location();

		Ok(output)
	}

	async fn dispatch_workflow_inner(
		ctx: &mut WorkflowCtx,
		input: I,
		tags: Option<serde_json::Value>,
	) -> WorkflowResult<Uuid>
	where
		I: WorkflowInput,
		<I as WorkflowInput>::Workflow: Workflow<Input = I>,
	{
		let event = ctx.current_history_event();

		// Signal received before
		let id = if let Some(event) = event {
			// Validate history is consistent
			let Event::SubWorkflow(sub_workflow) = event else {
				return Err(WorkflowError::HistoryDiverged(format!(
					"expected {event} at {}, found sub workflow {}",
					ctx.loc(),
					I::Workflow::NAME
				)));
			};

			if sub_workflow.name != I::Workflow::NAME {
				return Err(WorkflowError::HistoryDiverged(format!(
					"expected {event} at {}, found sub_workflow {}",
					ctx.loc(),
					I::Workflow::NAME
				)));
			}

			tracing::debug!(
				name=%ctx.name(),
				id=%ctx.workflow_id(),
				sub_workflow_name=%sub_workflow.name,
				sub_workflow_id=%sub_workflow.sub_workflow_id,
				"replaying workflow dispatch"
			);

			sub_workflow.sub_workflow_id
		}
		// Dispatch new workflow
		else {
			let sub_workflow_name = I::Workflow::NAME;
			let sub_workflow_id = Uuid::new_v4();

			tracing::info!(
				name=%ctx.name(),
				id=%ctx.workflow_id(),
				%sub_workflow_name,
				%sub_workflow_id,
				?tags,
				?input,
				"dispatching sub workflow"
			);

			// Serialize input
			let input_val =
				serde_json::to_value(input).map_err(WorkflowError::SerializeWorkflowOutput)?;

			ctx.db()
				.dispatch_sub_workflow(
					ctx.ray_id(),
					ctx.workflow_id(),
					ctx.full_location().as_ref(),
					sub_workflow_id,
					&sub_workflow_name,
					tags.as_ref(),
					input_val,
					ctx.loop_location(),
				)
				.await?;

			tracing::info!(
				name=%ctx.name(),
				id=%ctx.workflow_id(),
				%sub_workflow_name,
				?sub_workflow_id,
				"sub workflow dispatched"
			);

			sub_workflow_id
		};

		// Move to next event
		ctx.inc_location();

		Ok(id)
	}
}
