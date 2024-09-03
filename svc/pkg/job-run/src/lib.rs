use chirp_workflow::prelude::*;

pub mod ops;
pub mod util;
pub mod workers;
pub mod workflows;

pub fn registry() -> WorkflowResult<Registry> {
	use workflows::*;

	let mut registry = Registry::new();
	registry.register_workflow::<drain_all::Workflow>()?;

	Ok(registry)
}
