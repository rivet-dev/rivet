use gas::prelude::*;

pub mod consts;
pub mod errors;
pub mod http_client;
pub mod http_routes;
pub mod keys;
pub mod ops;
pub mod replica;
pub mod types;
pub mod utils;
pub mod workflows;

pub use epoxy_protocol::protocol;

pub fn registry() -> WorkflowResult<Registry> {
	use workflows::*;

	let mut registry = Registry::new();
	registry.register_workflow::<coordinator::Workflow>()?;
	registry.register_workflow::<replica::Workflow>()?;

	Ok(registry)
}
