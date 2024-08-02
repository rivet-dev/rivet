use chirp_workflow::prelude::*;

pub mod workflows;

pub fn registry() -> Registry {
	use workflows::*;

	let mut registry = Registry::new();
	registry.register_workflow::<test::Test>();

	registry
}
