use gas::prelude::*;

pub mod workflows;

pub fn registry() -> WorkflowResult<Registry> {
	use workflows::*;

	let mut registry = Registry::new();
	registry.register_workflow::<pruner::Workflow>()?;

	Ok(registry)
}
