pub mod generated;

// Re-export latest version.
pub use generated::v1::*;

pub const PROTOCOL_VERSION: u32 = 1;

/// Serialize a ToRivet message to BARE bytes.
pub fn encode_to_server(msg: &ToRivet) -> Result<Vec<u8>, serde_bare::error::Error> {
	serde_bare::to_vec(msg)
}

/// Deserialize a ToRivet message from BARE bytes.
pub fn decode_to_server(bytes: &[u8]) -> Result<ToRivet, serde_bare::error::Error> {
	serde_bare::from_slice(bytes)
}

/// Serialize a ToClient message to BARE bytes.
pub fn encode_to_client(msg: &ToClient) -> Result<Vec<u8>, serde_bare::error::Error> {
	serde_bare::to_vec(msg)
}

/// Deserialize a ToClient message from BARE bytes.
pub fn decode_to_client(bytes: &[u8]) -> Result<ToClient, serde_bare::error::Error> {
	serde_bare::from_slice(bytes)
}

#[cfg(test)]
mod tests {
	use super::*;

	// MARK: Round-trip tests

	#[test]
	fn round_trip_to_server_request_actor_open() {
		let msg = ToRivet::ToRivetRequest(ToRivetRequest {
			request_id: 1,
			actor_id: "abc".into(),
			data: RequestData::ActorOpenRequest,
		});
		let bytes = encode_to_server(&msg).unwrap();
		let decoded = decode_to_server(&bytes).unwrap();
		assert_eq!(msg, decoded);
	}

	#[test]
	fn round_trip_to_server_request_kv_get() {
		let msg = ToRivet::ToRivetRequest(ToRivetRequest {
			request_id: 3,
			actor_id: "actor1".into(),
			data: RequestData::KvGetRequest(KvGetRequest {
				keys: vec![vec![1, 2, 3], vec![4, 5]],
			}),
		});
		let bytes = encode_to_server(&msg).unwrap();
		let decoded = decode_to_server(&bytes).unwrap();
		assert_eq!(msg, decoded);
	}

	#[test]
	fn round_trip_to_client_response_error() {
		let msg = ToClient::ToClientResponse(ToClientResponse {
			request_id: 10,
			data: ResponseData::ErrorResponse(ErrorResponse {
				code: "actor_locked".into(),
				message: "actor is locked by another connection".into(),
			}),
		});
		let bytes = encode_to_client(&msg).unwrap();
		let decoded = decode_to_client(&bytes).unwrap();
		assert_eq!(msg, decoded);
	}

	#[test]
	fn round_trip_to_client_ping() {
		let msg = ToClient::ToClientPing(ToClientPing { ts: 9876543210 });
		let bytes = encode_to_client(&msg).unwrap();
		let decoded = decode_to_client(&bytes).unwrap();
		assert_eq!(msg, decoded);
	}

	#[test]
	fn round_trip_to_client_close() {
		let msg = ToClient::ToClientClose;
		let bytes = encode_to_client(&msg).unwrap();
		let decoded = decode_to_client(&bytes).unwrap();
		assert_eq!(msg, decoded);
	}

	// MARK: Cross-language byte compatibility tests

	#[test]
	fn bytes_to_server_request_actor_open() {
		let msg = ToRivet::ToRivetRequest(ToRivetRequest {
			request_id: 1,
			actor_id: "abc".into(),
			data: RequestData::ActorOpenRequest,
		});
		let bytes = encode_to_server(&msg).unwrap();
		assert_eq!(
			bytes,
			[0x00, 0x01, 0x00, 0x00, 0x00, 0x03, 0x61, 0x62, 0x63, 0x00]
		);
	}

	#[test]
	fn bytes_to_server_pong() {
		let msg = ToRivet::ToRivetPong(ToRivetPong { ts: 1234567890 });
		let bytes = encode_to_server(&msg).unwrap();
		assert_eq!(
			bytes,
			[0x01, 0xD2, 0x02, 0x96, 0x49, 0x00, 0x00, 0x00, 0x00]
		);
	}

	#[test]
	fn bytes_to_client_close() {
		let msg = ToClient::ToClientClose;
		let bytes = encode_to_client(&msg).unwrap();
		assert_eq!(bytes, [0x02]);
	}

	#[test]
	fn bytes_to_client_response_kv_get() {
		let msg = ToClient::ToClientResponse(ToClientResponse {
			request_id: 42,
			data: ResponseData::KvGetResponse(KvGetResponse {
				keys: vec![vec![1, 2]],
				values: vec![vec![3, 4, 5]],
			}),
		});
		let bytes = encode_to_client(&msg).unwrap();
		assert_eq!(
			bytes,
			[
				0x00, 0x2A, 0x00, 0x00, 0x00, 0x03, 0x01, 0x02, 0x01, 0x02, 0x01, 0x03, 0x03, 0x04,
				0x05
			]
		);
	}
}
