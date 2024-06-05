use std::{collections::HashMap, future::Future, pin::Pin, sync::Arc};

use futures_util::FutureExt;

use crate::{Workflow, WorkflowCtx, WorkflowError, WorkflowResult};

pub type RegistryHandle = Arc<Registry>;

/// Contains a lookup map for workflow run handlers by workflow name.
pub struct Registry {
	pub(crate) workflows: HashMap<String, Arc<RegistryWorkflow>>,
}

impl Registry {
	pub fn new() -> Self {
		Registry {
			workflows: HashMap::new(),
		}
	}

	pub fn handle(self) -> RegistryHandle {
		Arc::new(self)
	}

	pub fn merge(mut self, registry: Registry) -> Registry {
		self.workflows.extend(registry.workflows.into_iter());

		self
	}

	pub fn register_workflow<W: Workflow>(&mut self) {
		self.workflows.insert(
			W::name().to_string(),
			Arc::new(RegistryWorkflow {
				run: |ctx| {
					async move {
						// Deserialize input
						let input = serde_json::from_str(&ctx.input)
							.map_err(WorkflowError::DeserializeWorkflowInput)?;

						// Run workflow
						let output = match W::run(ctx, &input).await {
							Ok(x) => x,
							Err(err) => {
								// Differentiate between WorkflowError and user error
								match err.downcast::<WorkflowError>() {
									Ok(err) => return Err(err),
									Err(err) => return Err(WorkflowError::WorkflowFailure(err)),
								}
							}
						};

						// Serialize output
						let output_str = serde_json::to_string(&output)
							.map_err(WorkflowError::SerializeWorkflowOutput)?;

						Ok(output_str)
					}
					.boxed()
				},
			}),
		);
	}

	pub fn get_workflow(&self, name: &str) -> WorkflowResult<&Arc<RegistryWorkflow>> {
		self.workflows
			.get(name)
			.ok_or(WorkflowError::WorkflowMissingFromRegistry(name.to_string()))
	}
}

pub struct RegistryWorkflow {
	pub run: for<'a> fn(
		&'a mut WorkflowCtx,
	) -> Pin<Box<dyn Future<Output = WorkflowResult<String>> + Send + 'a>>,
}
