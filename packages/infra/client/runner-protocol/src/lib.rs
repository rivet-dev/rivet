use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
pub enum ToRunner {
	Start {
		actor_id: Uuid,
	},
	Signal {
		actor_id: Uuid,
		signal: i32,
		persist_state: bool,
	},
}
