#[cfg(feature = "chirp")]
use chirp_workflow::prelude::*;

pub mod client_config;
mod metrics;
#[cfg(feature = "ops")]
pub mod ops;
pub mod protocol;
pub mod system_info;
#[cfg(feature = "workflows")]
pub mod workflows;

#[cfg(feature = "workflows")]
pub fn registry() -> WorkflowResult<Registry> {
	use workflows::*;

	let mut registry = Registry::new();
	registry.register_workflow::<client::Workflow>()?;
	registry.register_workflow::<datacenter::Workflow>()?;

	Ok(registry)
}
