use chirp_workflow::prelude::*;

pub mod ops;
pub mod types;
pub mod util;
pub mod workers;
pub mod workflows;

pub fn registry() -> WorkflowResult<Registry> {
	use workflows::*;

	let mut registry = Registry::new();
	registry.register_workflow::<server::Workflow>()?;
	registry.register_workflow::<server::nomad::Workflow>()?;
	registry.register_workflow::<server::nomad::destroy::Workflow>()?;
	registry.register_workflow::<server::nomad::alloc_plan::Workflow>()?;
	registry.register_workflow::<server::nomad::alloc_update::Workflow>()?;
	registry.register_workflow::<server::nomad::eval_update::Workflow>()?;
	registry.register_workflow::<server::nomad::eval_update::Workflow>()?;
	// registry.register_workflow::<server::pegboard::Workflow>()?;

	Ok(registry)
}
