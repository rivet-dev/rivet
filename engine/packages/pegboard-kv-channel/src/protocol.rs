//! BARE serialization types for the KV channel protocol v1.
//!
//! Types match engine/sdks/schemas/kv-channel-protocol/v1.bare exactly.
//! Variant order in enums must match the union tag order in the schema.

use serde::{Deserialize, Serialize};

pub const PROTOCOL_VERSION: u32 = 1;

pub type Id = String;
pub type KvKey = Vec<u8>;
pub type KvValue = Vec<u8>;

// MARK: KV

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KvGetRequest {
	pub keys: Vec<KvKey>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KvPutRequest {
	pub keys: Vec<KvKey>,
	pub values: Vec<KvValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KvDeleteRequest {
	pub keys: Vec<KvKey>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KvDeleteRangeRequest {
	pub start: KvKey,
	pub end: KvKey,
}

// MARK: Request/Response

/// Union tag order must match v1.bare RequestData union exactly.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RequestData {
	ActorOpenRequest,
	ActorCloseRequest,
	KvGetRequest(KvGetRequest),
	KvPutRequest(KvPutRequest),
	KvDeleteRequest(KvDeleteRequest),
	KvDeleteRangeRequest(KvDeleteRangeRequest),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {
	pub code: String,
	pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KvGetResponse {
	pub keys: Vec<KvKey>,
	pub values: Vec<KvValue>,
}

/// Union tag order must match v1.bare ResponseData union exactly.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResponseData {
	ErrorResponse(ErrorResponse),
	ActorOpenResponse,
	ActorCloseResponse,
	KvGetResponse(KvGetResponse),
	KvPutResponse,
	KvDeleteResponse,
}

// MARK: To Server

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToServerRequest {
	pub request_id: u32,
	pub actor_id: Id,
	pub data: RequestData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToServerPong {
	pub ts: i64,
}

/// Union tag order must match v1.bare ToServer union exactly.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ToServer {
	ToServerRequest(ToServerRequest),
	ToServerPong(ToServerPong),
}

// MARK: To Client

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToClientResponse {
	pub request_id: u32,
	pub data: ResponseData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToClientPing {
	pub ts: i64,
}

/// Union tag order must match v1.bare ToClient union exactly.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ToClient {
	ToClientResponse(ToClientResponse),
	ToClientPing(ToClientPing),
	ToClientClose,
}

pub fn encode_to_client(msg: &ToClient) -> anyhow::Result<Vec<u8>> {
	Ok(serde_bare::to_vec(msg)?)
}

pub fn decode_to_server(data: &[u8]) -> anyhow::Result<ToServer> {
	Ok(serde_bare::from_slice(data)?)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn round_trip_to_server_request() {
		let msg = ToServer::ToServerRequest(ToServerRequest {
			request_id: 42,
			actor_id: "test_actor".to_string(),
			data: RequestData::KvGetRequest(KvGetRequest {
				keys: vec![vec![1, 2, 3]],
			}),
		});
		let encoded = serde_bare::to_vec(&msg).unwrap();
		let decoded: ToServer = serde_bare::from_slice(&encoded).unwrap();
		match decoded {
			ToServer::ToServerRequest(req) => {
				assert_eq!(req.request_id, 42);
				assert_eq!(req.actor_id, "test_actor");
			}
			_ => panic!("wrong variant"),
		}
	}

	#[test]
	fn round_trip_to_client_response() {
		let msg = ToClient::ToClientResponse(ToClientResponse {
			request_id: 1,
			data: ResponseData::KvGetResponse(KvGetResponse {
				keys: vec![vec![0x08]],
				values: vec![vec![0xFF]],
			}),
		});
		let encoded = encode_to_client(&msg).unwrap();
		let decoded: ToClient = serde_bare::from_slice(&encoded).unwrap();
		match decoded {
			ToClient::ToClientResponse(resp) => assert_eq!(resp.request_id, 1),
			_ => panic!("wrong variant"),
		}
	}

	#[test]
	fn round_trip_to_client_ping() {
		let msg = ToClient::ToClientPing(ToClientPing { ts: 1234567890 });
		let encoded = encode_to_client(&msg).unwrap();
		let decoded: ToClient = serde_bare::from_slice(&encoded).unwrap();
		match decoded {
			ToClient::ToClientPing(ping) => assert_eq!(ping.ts, 1234567890),
			_ => panic!("wrong variant"),
		}
	}

	#[test]
	fn round_trip_to_client_close() {
		let msg = ToClient::ToClientClose;
		let encoded = encode_to_client(&msg).unwrap();
		let decoded: ToClient = serde_bare::from_slice(&encoded).unwrap();
		assert!(matches!(decoded, ToClient::ToClientClose));
	}

	#[test]
	fn round_trip_error_response() {
		let msg = ToClient::ToClientResponse(ToClientResponse {
			request_id: 5,
			data: ResponseData::ErrorResponse(ErrorResponse {
				code: "actor_locked".to_string(),
				message: "actor is locked by another connection".to_string(),
			}),
		});
		let encoded = encode_to_client(&msg).unwrap();
		let decoded: ToClient = serde_bare::from_slice(&encoded).unwrap();
		match decoded {
			ToClient::ToClientResponse(resp) => match resp.data {
				ResponseData::ErrorResponse(err) => {
					assert_eq!(err.code, "actor_locked");
				}
				_ => panic!("wrong response variant"),
			},
			_ => panic!("wrong variant"),
		}
	}
}
