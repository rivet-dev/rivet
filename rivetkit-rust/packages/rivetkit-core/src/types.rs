use rivetkit_shared_types::actor_key;
use serde::{Deserialize, Serialize};

pub type ActorKey = Vec<ActorKeySegment>;
pub type ConnId = String;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ActorKeySegment {
	String(String),
	Number(f64),
}

/// The actor key in the slash form RivetKit shows users and the actor REST API returns. A
/// number segment is written as its decimal string.
pub fn format_actor_key(key: &ActorKey) -> String {
	let parts: Vec<String> = key
		.iter()
		.map(|segment| match segment {
			ActorKeySegment::String(value) => value.clone(),
			ActorKeySegment::Number(value) => value.to_string(),
		})
		.collect();
	actor_key::slash::serialize(&parts)
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum WsMessage {
	Text(String),
	Binary(Vec<u8>),
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaveStateOpts {
	pub immediate: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListOpts {
	pub reverse: bool,
	pub limit: Option<u32>,
}
