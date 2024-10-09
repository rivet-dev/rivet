use chirp_workflow::prelude::*;

pub mod ops;
pub mod protocol;
pub mod types;
pub mod utils;
pub mod workflows;

pub fn registry() -> WorkflowResult<Registry> {
	use workflows::*;

	let mut registry = Registry::new();
	registry.register_workflow::<client::Workflow>()?;
	registry.register_workflow::<datacenter::Workflow>()?;

	Ok(registry)
}
