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
