/// Encode custom bytes into a UUID v4 format
///
/// This function takes up to 14 bytes of custom data and encodes them into a UUID v4
/// format, completely skipping the version and variant bytes to avoid any data loss:
/// - Bytes 0-5: data[0..6]
/// - Byte 6: Version byte (0x40 = version 4), NOT used for custom data
/// - Byte 7: data[6]
/// - Byte 8: Variant byte (0x80 = variant), NOT used for custom data
/// - Bytes 9-15: data[7..14]
///
/// # Arguments
/// * `data` - Slice of bytes to encode (max 14 bytes). If less than 14 bytes, remaining bytes are zeroed.
///
/// # Returns
/// A 16-byte array representing a UUID v4
///
/// # Panics
/// Panics if data length exceeds 14 bytes
pub fn encode_bytes_to_uuid(data: &[u8]) -> [u8; 16] {
	assert!(data.len() <= 14, "data must be at most 14 bytes");

	let mut uuid = [0u8; 16];

	// Bytes 0-5: First 6 bytes of custom data
	let copy_len = data.len().min(6);
	uuid[..copy_len].copy_from_slice(&data[..copy_len]);

	// Byte 6: Version byte (0x40 = version 4) - NO custom data
	uuid[6] = 0x40;

	// Byte 7: Next byte of custom data (data[6])
	if data.len() > 6 {
		uuid[7] = data[6];
	}

	// Byte 8: Variant byte (0x80 = variant) - NO custom data
	uuid[8] = 0x80;

	// Bytes 9-15: Remaining custom data (data[7..14])
	if data.len() > 7 {
		let remaining_len = (data.len() - 7).min(7);
		uuid[9..9 + remaining_len].copy_from_slice(&data[7..7 + remaining_len]);
	}

	uuid
}

/// Decode custom bytes from a UUID v4 format
///
/// This function extracts the custom data bytes from a UUID v4, completely
/// skipping the version and variant bytes:
/// - Bytes 0-5: uuid[0..6]
/// - Byte 6: UUID version byte - SKIPPED
/// - Byte 7: uuid[7] -> data[6]
/// - Byte 8: UUID variant byte - SKIPPED
/// - Bytes 9-15: uuid[9..16] -> data[7..14]
///
/// # Arguments
/// * `uuid` - 16-byte UUID array
///
/// # Returns
/// A 14-byte array containing the extracted custom data
pub fn decode_bytes_from_uuid(uuid: &[u8; 16]) -> [u8; 14] {
	let mut data = [0u8; 14];

	// Bytes 0-5: First 6 bytes of custom data
	data[..6].copy_from_slice(&uuid[..6]);

	// Byte 6: Skip UUID version byte, take uuid[7]
	data[6] = uuid[7];

	// Bytes 7-13: Take uuid[9..16] (skip variant byte at uuid[8])
	data[7..].copy_from_slice(&uuid[9..16]);

	data
}

#[cfg(test)]
mod tests {
	use super::*;
	use uuid::Uuid;

	#[test]
	fn test_encode_decode_roundtrip() {
		// Test with 14 bytes of custom data
		let original = [
			0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0xFF, 0x08, 0xFF, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E,
		];

		let uuid_bytes = encode_bytes_to_uuid(&original);
		let decoded = decode_bytes_from_uuid(&uuid_bytes);

		// All bytes should match exactly (no masking/loss)
		assert_eq!(decoded, original);
	}

	#[test]
	fn test_uuid_version_bits() {
		let data = [0xFF; 14];
		let uuid_bytes = encode_bytes_to_uuid(&data);
		let uuid = Uuid::from_bytes(uuid_bytes);

		// Validate UUID version is 4
		assert_eq!(uuid.get_version_num(), 4);

		// Byte 6 should be exactly 0x40 (version 4, no custom data)
		assert_eq!(uuid_bytes[6], 0x40);
	}

	#[test]
	fn test_uuid_variant_bits() {
		let data = [0xFF; 14];
		let uuid_bytes = encode_bytes_to_uuid(&data);
		let uuid = Uuid::from_bytes(uuid_bytes);

		// Validate UUID variant is RFC4122
		assert_eq!(uuid.get_variant(), uuid::Variant::RFC4122);

		// Byte 8 should be exactly 0x80 (variant, no custom data)
		assert_eq!(uuid_bytes[8], 0x80);
	}

	#[test]
	fn test_encode_partial_data() {
		// Test with less than 14 bytes
		let data = [0xAA, 0xBB, 0xCC];
		let uuid_bytes = encode_bytes_to_uuid(&data);
		let uuid = Uuid::from_bytes(uuid_bytes);

		assert_eq!(uuid_bytes[0], 0xAA);
		assert_eq!(uuid_bytes[1], 0xBB);
		assert_eq!(uuid_bytes[2], 0xCC);
		assert_eq!(uuid_bytes[3], 0x00); // Zeroed

		// Validate UUID version and variant
		assert_eq!(uuid.get_version_num(), 4);
		assert_eq!(uuid.get_variant(), uuid::Variant::RFC4122);
	}

	#[test]
	fn test_encode_empty_data() {
		let data = [];
		let uuid_bytes = encode_bytes_to_uuid(&data);
		let uuid = Uuid::from_bytes(uuid_bytes);

		// Validate UUID version and variant
		assert_eq!(uuid.get_version_num(), 4);
		assert_eq!(uuid.get_variant(), uuid::Variant::RFC4122);

		// All other bytes should be zero
		assert_eq!(uuid_bytes[0..6], [0; 6]);
		assert_eq!(uuid_bytes[9..15], [0; 6]);
		assert_eq!(uuid_bytes[15], 0x00); // Last byte unused
	}

	#[test]
	fn test_decode_skips_version_variant_bytes() {
		// Create a UUID with specific values
		let mut uuid = [0u8; 16];
		uuid[0] = 0x01;
		uuid[6] = 0x4F; // Version byte - should be IGNORED
		uuid[7] = 0xAA; // This should become data[6]
		uuid[8] = 0xBF; // Variant byte - should be IGNORED
		uuid[9] = 0xBB; // This should become data[7]

		let decoded = decode_bytes_from_uuid(&uuid);

		assert_eq!(decoded[0], 0x01);
		assert_eq!(decoded[6], 0xAA); // From uuid[7]
		assert_eq!(decoded[7], 0xBB); // From uuid[9]
	}

	#[test]
	#[should_panic(expected = "data must be at most 14 bytes")]
	fn test_encode_too_much_data() {
		let data = [0xFF; 15];
		encode_bytes_to_uuid(&data);
	}

	#[test]
	fn test_all_zeros() {
		let data = [0x00; 14];
		let uuid_bytes = encode_bytes_to_uuid(&data);
		let uuid = Uuid::from_bytes(uuid_bytes);
		let decoded = decode_bytes_from_uuid(&uuid_bytes);

		// All custom data should be zero
		assert_eq!(decoded, [0; 14]);

		// Validate UUID version and variant
		assert_eq!(uuid.get_version_num(), 4);
		assert_eq!(uuid.get_variant(), uuid::Variant::RFC4122);
	}

	#[test]
	fn test_all_ones() {
		let data = [0xFF; 14];
		let uuid_bytes = encode_bytes_to_uuid(&data);
		let uuid = Uuid::from_bytes(uuid_bytes);
		let decoded = decode_bytes_from_uuid(&uuid_bytes);

		// All custom data should be preserved exactly
		assert_eq!(decoded, [0xFF; 14]);

		// Validate UUID version and variant
		assert_eq!(uuid.get_version_num(), 4);
		assert_eq!(uuid.get_variant(), uuid::Variant::RFC4122);
	}
}
