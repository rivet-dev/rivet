use gas::prelude::*;

pub mod errors;
pub mod keys;
mod metrics;
pub mod ops;
pub mod pubsub_subjects;
pub mod utils;
pub mod workflows;

pub fn registry() -> WorkflowResult<Registry> {
	use workflows::*;

	let mut registry = Registry::new();
	registry.register_workflow::<actor::Workflow>()?;
	registry.register_workflow::<runner::Workflow>()?;
	registry.register_workflow::<runner2::Workflow>()?;
	registry.register_workflow::<serverless::pool::Workflow>()?;
	registry.register_workflow::<serverless::runner::Workflow>()?;
	registry.register_workflow::<serverless::connection::Workflow>()?;

	Ok(registry)
}
