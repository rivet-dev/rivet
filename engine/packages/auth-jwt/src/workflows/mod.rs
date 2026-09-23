pub mod key_rotation;

use gas::prelude::*;

pub fn registry() -> WorkflowResult<Registry> {
	let mut registry = Registry::new();
	registry.register_workflow::<key_rotation::Workflow>()?;
	Ok(registry)
}
