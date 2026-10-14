pub mod key_rotation;
pub mod webhook_recovery;
pub mod webhook_rotation;

use gas::prelude::*;

pub fn registry() -> WorkflowResult<Registry> {
	let mut registry = Registry::new();
	registry.register_workflow::<key_rotation::Workflow>()?;
	registry.register_workflow::<webhook_rotation::Workflow>()?;
	registry.register_workflow::<webhook_recovery::Workflow>()?;
	Ok(registry)
}
