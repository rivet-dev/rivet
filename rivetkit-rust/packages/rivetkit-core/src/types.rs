use serde::{Deserialize, Serialize};

pub type ActorKey = Vec<ActorKeySegment>;
pub type ConnId = String;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ActorKeySegment {
	String(String),
	Number(f64),
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
