use std::{fmt::Display, sync::Arc};

use global_error::{GlobalError, GlobalResult};
use serde::Serialize;
use uuid::Uuid;

use crate::{
	builder::BuilderError,
	ctx::WorkflowCtx,
	error::{WorkflowError, WorkflowResult},
	history::cursor::HistoryResult,
	workflow::{Workflow, WorkflowInput},
};

pub struct SubWorkflowBuilder<'a, I: WorkflowInput> {
	ctx: &'a mut WorkflowCtx,
	version: usize,

	input: I,
	tags: serde_json::Map<String, serde_json::Value>,
	error: Option<BuilderError>,
}

impl<'a, I: WorkflowInput> SubWorkflowBuilder<'a, I>
where
	<I as WorkflowInput>::Workflow: Workflow<Input = I>,
{
	pub(crate) fn new(ctx: &'a mut WorkflowCtx, version: usize, input: I) -> Self {
		SubWorkflowBuilder {
			ctx,
			version,

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

		// Error for version mismatch. This is done in the builder instead of in `VersionedWorkflowCtx` to
		// defer the error.
		self.ctx
			.compare_version("sub workflow", self.version)
			.map_err(GlobalError::raw)?;

		Self::dispatch_workflow_inner(self.ctx, self.version, self.input, tags)
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

		// Err for version mismatch
		self.ctx
			.compare_version("sub workflow", self.version)
			.map_err(GlobalError::raw)?;

		let input_val = serde_json::value::to_raw_value(&self.input)
			.map_err(WorkflowError::SerializeWorkflowInput)
			.map_err(GlobalError::raw)?;
		let mut branch = self
			.ctx
			.branch_inner(Arc::new(input_val), self.version, None)
			.await
			.map_err(GlobalError::raw)?;

		tracing::info!(name=%self.ctx.name(), id=%self.ctx.workflow_id(), sub_workflow_name=%I::Workflow::NAME, "running sub workflow");
		// Run workflow
		let output =
			<<I as WorkflowInput>::Workflow as Workflow>::run(&mut branch, &self.input).await?;

		// Validate no leftover events
		branch.cursor().check_clear().map_err(GlobalError::raw)?;

		// Move to next event
		self.ctx.cursor_mut().update(branch.cursor().root());

		Ok(output)
	}

	async fn dispatch_workflow_inner(
		ctx: &mut WorkflowCtx,
		version: usize,
		input: I,
		tags: Option<serde_json::Value>,
	) -> WorkflowResult<Uuid>
	where
		I: WorkflowInput,
		<I as WorkflowInput>::Workflow: Workflow<Input = I>,
	{
		let history_res = ctx
			.cursor()
			.compare_sub_workflow(version, I::Workflow::NAME)?;
		let location = ctx.cursor().current_location_for(&history_res);

		// Signal received before
		let id = if let HistoryResult::Event(sub_workflow) = history_res {
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
			let input_val = serde_json::value::to_raw_value(&input)
				.map_err(WorkflowError::SerializeWorkflowOutput)?;

			ctx.db()
				.dispatch_sub_workflow(
					ctx.ray_id(),
					ctx.workflow_id(),
					&location,
					version,
					sub_workflow_id,
					&sub_workflow_name,
					tags.as_ref(),
					&input_val,
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
		ctx.cursor_mut().update(&location);

		Ok(id)
	}
}
