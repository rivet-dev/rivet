use chirp_workflow::prelude::*;

pub mod ops;
pub mod utils;
pub mod workflows;

pub fn registry() -> WorkflowResult<Registry> {
	use workflows::*;

	let mut registry = Registry::new();
	registry.register_workflow::<ws::Workflow>()?;

	Ok(registry)
}
