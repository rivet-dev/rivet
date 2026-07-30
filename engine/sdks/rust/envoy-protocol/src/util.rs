use serde::{Deserialize, Serialize};

const ACTOR_ERROR_PREFIX: &str = "rivet-error:";

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ActorErrorEnvelope {
	pub group: String,
	pub code: String,
	pub message: String,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub metadata: Option<serde_json::Value>,
}

pub fn encode_actor_error(error: &ActorErrorEnvelope) -> Result<String, serde_json::Error> {
	Ok(format!(
		"{ACTOR_ERROR_PREFIX}{}",
		serde_json::to_string(error)?
	))
}

pub fn decode_actor_error(message: &str) -> Option<ActorErrorEnvelope> {
	let payload = message.strip_prefix(ACTOR_ERROR_PREFIX)?;
	serde_json::from_str(payload).ok()
}

/// Generate a new 4-byte gateway ID from a random u32
pub fn generate_gateway_id() -> crate::GatewayId {
	rand::random::<u32>().to_le_bytes()
}

/// Generate a new 4-byte request ID from a random u32
pub fn generate_request_id() -> crate::RequestId {
	rand::random::<u32>().to_le_bytes()
}

/// Convert a GatewayId to a hex string
pub fn id_to_string(gateway_id: &crate::GatewayId) -> String {
	hex::encode(gateway_id)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn actor_error_envelope_round_trips() {
		let error = ActorErrorEnvelope {
			group: "actor".to_owned(),
			code: "not_registered".to_owned(),
			message: "Actor factory 'removed' is not registered.".to_owned(),
			metadata: Some(serde_json::json!({ "actor_name": "removed" })),
		};

		let encoded = encode_actor_error(&error).expect("encode actor error");

		assert_eq!(decode_actor_error(&encoded), Some(error));
		assert_eq!(decode_actor_error("ordinary crash message"), None);
	}
}
