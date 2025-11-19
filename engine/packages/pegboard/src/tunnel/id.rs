use anyhow::{Context, Result, ensure};
use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use rivet_runner_protocol as protocol;

// Type aliases for the message ID components
pub type GatewayId = [u8; 4];
pub type RequestId = [u8; 4];
pub type MessageIndex = u16;
pub type MessageId = [u8; 12];

/// Generate a new 4-byte gateway ID from a random u32
pub fn generate_gateway_id() -> GatewayId {
	rand::random::<u32>().to_le_bytes()
}

/// Build a MessageId from its components
pub fn build_message_id(
	gateway_id: GatewayId,
	request_id: RequestId,
	message_index: MessageIndex,
) -> Result<MessageId> {
	let parts = protocol::MessageIdParts {
		gateway_id,
		request_id,
		message_index,
	};

	// Serialize directly to a fixed-size buffer on the stack
	let mut message_id = [0u8; 12];
	let mut cursor = std::io::Cursor::new(&mut message_id[..]);
	serde_bare::to_writer(&mut cursor, &parts).context("failed to serialize message id parts")?;

	// Verify we wrote exactly 12 bytes
	let written = cursor.position() as usize;
	ensure!(
		written == 12,
		"message id serialization produced wrong size: expected 12 bytes, got {}",
		written
	);

	Ok(message_id)
}

/// Parse a MessageId into its components
pub fn parse_message_id(message_id: MessageId) -> Result<protocol::MessageIdParts> {
	serde_bare::from_slice(&message_id).context("failed to deserialize message id")
}

/// Convert a GatewayId to a base64 string
pub fn gateway_id_to_string(gateway_id: &GatewayId) -> String {
	BASE64.encode(gateway_id)
}

/// Parse a GatewayId from a base64 string
pub fn gateway_id_from_string(s: &str) -> Result<GatewayId> {
	let bytes = BASE64.decode(s).context("failed to decode base64")?;
	let gateway_id: GatewayId = bytes.try_into().map_err(|v: Vec<u8>| {
		anyhow::anyhow!(
			"invalid gateway id length: expected 4 bytes, got {}",
			v.len()
		)
	})?;
	Ok(gateway_id)
}

/// Generate a new 4-byte request ID from a random u32
pub fn generate_request_id() -> RequestId {
	rand::random::<u32>().to_le_bytes()
}

/// Convert a RequestId to a base64 string
pub fn request_id_to_string(request_id: &RequestId) -> String {
	BASE64.encode(request_id)
}

/// Parse a RequestId from a base64 string
pub fn request_id_from_string(s: &str) -> Result<RequestId> {
	let bytes = BASE64.decode(s).context("failed to decode base64")?;
	let request_id: RequestId = bytes.try_into().map_err(|v: Vec<u8>| {
		anyhow::anyhow!(
			"invalid request id length: expected 4 bytes, got {}",
			v.len()
		)
	})?;
	Ok(request_id)
}
