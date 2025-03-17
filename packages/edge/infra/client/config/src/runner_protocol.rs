use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
pub enum ToRunner {
	Start {
		actor_id: Uuid,
		generation: u32,
	},
	Signal {
		actor_id: Uuid,
		generation: u32,
		signal: i32,
		persist_storage: bool,
	},
	// Kills the runner process
	Terminate,
}
