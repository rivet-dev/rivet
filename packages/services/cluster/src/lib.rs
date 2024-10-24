use chirp_workflow::prelude::*;

pub mod ops;
pub mod types;
pub mod util;
pub mod workflows;

pub fn registry() -> WorkflowResult<Registry> {
	use workflows::*;

	let mut registry = Registry::new();
	registry.register_workflow::<cluster::Workflow>()?;
	registry.register_workflow::<datacenter::Workflow>()?;
	registry.register_workflow::<datacenter::scale::Workflow>()?;
	registry.register_workflow::<datacenter::tls_issue::Workflow>()?;
	registry.register_workflow::<server::Workflow>()?;
	registry.register_workflow::<server::install::Workflow>()?;
	registry.register_workflow::<server::dns_create::Workflow>()?;
	registry.register_workflow::<server::dns_delete::Workflow>()?;
	registry.register_workflow::<server::drain::Workflow>()?;
	registry.register_workflow::<server::undrain::Workflow>()?;
	registry.register_workflow::<prebake::Workflow>()?;

	Ok(registry)
}
