use chirp_workflow::prelude::*;

pub mod ops;
pub mod types;
pub mod util;
pub mod workflows;

pub fn registry() -> WorkflowResult<Registry> {
	use workflows::*;

	let mut registry = Registry::new();
	registry.register_workflow::<image::Workflow>()?;
	registry.register_workflow::<server::Workflow>()?;
	registry.register_workflow::<server::cleanup::Workflow>()?;

	Ok(registry)
}
