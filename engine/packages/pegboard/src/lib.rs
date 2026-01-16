use gas::prelude::*;

pub mod actor_kv;
pub mod errors;
pub mod keys;
pub mod metrics;
pub mod ops;
pub mod pubsub_subjects;
pub mod utils;
pub mod workflows;

pub fn registry() -> WorkflowResult<Registry> {
	use workflows::*;

	let mut registry = Registry::new();
	registry.register_workflow::<actor::Workflow>()?;
	registry.register_workflow::<actor_runner_name_selector_backfill::Workflow>()?;
	registry.register_workflow::<runner::Workflow>()?;
	registry.register_workflow::<runner2::Workflow>()?;
	registry.register_workflow::<runner_pool::Workflow>()?;
	registry.register_workflow::<runner_pool_error_tracker::Workflow>()?;
	registry.register_workflow::<serverless::receiver::Workflow>()?;
	registry.register_workflow::<serverless::conn::Workflow>()?;
	registry.register_workflow::<serverless::backfill::Workflow>()?;

	Ok(registry)
}
