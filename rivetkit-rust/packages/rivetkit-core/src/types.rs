use serde::{Deserialize, Serialize};

pub type ActorKey = Vec<ActorKeySegment>;
pub type ConnId = String;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ActorKeySegment {
	String(String),
	Number(f64),
}

pub fn format_actor_key(key: &ActorKey) -> String {
	key.iter()
		.map(|segment| match segment {
			ActorKeySegment::String(value) => escape_actor_key_segment(value),
			ActorKeySegment::Number(value) => escape_actor_key_segment(&value.to_string()),
		})
		.collect::<Vec<_>>()
		.join("/")
}

fn escape_actor_key_segment(segment: &str) -> String {
	if segment.is_empty() {
		return "\\0".to_owned();
	}

	let mut escaped = String::with_capacity(segment.len());
	for ch in segment.chars() {
		match ch {
			'\\' | '/' => {
				escaped.push('\\');
				escaped.push(ch);
			}
			_ => escaped.push(ch),
		}
	}
	escaped
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
