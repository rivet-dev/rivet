use gas::prelude::*;

pub mod keys;
pub mod ops;
pub mod workflows;

pub fn registry() -> WorkflowResult<Registry> {
	use workflows::*;

	let mut registry = Registry::new();
	registry.register_workflow::<ping::Workflow>()?;

	Ok(registry)
}
