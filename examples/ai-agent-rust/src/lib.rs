mod agent;
mod helpers;

use rivetkit::prelude::*;

pub use agent::Agent;
pub use helpers::{GetHistory, Message, MessageAdded, Response, SendMessage};

pub const ACTOR_NAME: &str = "agent";

pub fn registry() -> Registry {
	let mut registry = Registry::new();
	registry.register_actor::<Agent>(ACTOR_NAME);
	registry
}
