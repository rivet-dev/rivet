use chirp_workflow::prelude::*;

pub mod workflows;

pub fn registry() -> Registry {
	let mut registry = Registry::new();
	registry.register_workflow::<workflows::Test>();

	registry
}
