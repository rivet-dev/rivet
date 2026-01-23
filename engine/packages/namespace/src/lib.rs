use gas::prelude::*;

pub mod errors;
pub mod keys;
pub mod ops;
pub mod utils;
pub mod workflows;

pub fn registry() -> WorkflowResult<Registry> {
	use workflows::*;

	let mut registry = Registry::new();
	registry.register_workflow::<namespace::Workflow>()?;
	registry.register_workflow::<metrics_exporter::Workflow>()?;

	Ok(registry)
}
