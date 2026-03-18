//! BARE serialization/deserialization for KV channel protocol messages.
//!
//! Implements all types from `engine/sdks/schemas/kv-channel-protocol/v1.bare`.
//! Uses `serde_bare` for encoding/decoding.
//!
//! The protocol defines ToServer (client -> server) and ToClient (server -> client)
//! union types for WebSocket binary frames.
//!
//! Enum variant order in each union must match the BARE schema tag order exactly.
//! serde_bare encodes enum discriminants positionally (varint of variant index).

use serde::{Deserialize, Serialize};

// MARK: Core

/// 30-character base36 string encoding from engine/packages/util-id/.
pub type Id = String;

// MARK: KV

pub type KvKey = Vec<u8>;
pub type KvValue = Vec<u8>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KvGetRequest {
    pub keys: Vec<KvKey>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KvPutRequest {
    pub keys: Vec<KvKey>,
    pub values: Vec<KvValue>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KvDeleteRequest {
    pub keys: Vec<KvKey>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KvDeleteRangeRequest {
    pub start: KvKey,
    pub end: KvKey,
}

// MARK: Request/Response

/// Union of all request types. Variant order matches v1.bare RequestData union.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RequestData {
    ActorOpenRequest,                               // tag 0 - void
    ActorCloseRequest,                              // tag 1 - void
    KvGetRequest(KvGetRequest),                     // tag 2
    KvPutRequest(KvPutRequest),                     // tag 3
    KvDeleteRequest(KvDeleteRequest),               // tag 4
    KvDeleteRangeRequest(KvDeleteRangeRequest),     // tag 5
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KvGetResponse {
    pub keys: Vec<KvKey>,
    pub values: Vec<KvValue>,
}

/// Union of all response types. Variant order matches v1.bare ResponseData union.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResponseData {
    ErrorResponse(ErrorResponse),       // tag 0
    ActorOpenResponse,                  // tag 1 - void
    ActorCloseResponse,                 // tag 2 - void
    KvGetResponse(KvGetResponse),       // tag 3
    KvPutResponse,                      // tag 4 - void
    KvDeleteResponse,                   // tag 5 - void
}

// MARK: To Server

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToServerRequest {
    pub request_id: u32,
    pub actor_id: Id,
    pub data: RequestData,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToServerPong {
    pub ts: i64,
}

/// Top-level client-to-server message. Variant order matches v1.bare ToServer union.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToServer {
    ToServerRequest(ToServerRequest),   // tag 0
    ToServerPong(ToServerPong),         // tag 1
}

// MARK: To Client

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToClientResponse {
    pub request_id: u32,
    pub data: ResponseData,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToClientPing {
    pub ts: i64,
}

/// Top-level server-to-client message. Variant order matches v1.bare ToClient union.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToClient {
    ToClientResponse(ToClientResponse), // tag 0
    ToClientPing(ToClientPing),         // tag 1
    ToClientClose,                      // tag 2 - void
}

// MARK: Encode/Decode

/// Serialize a ToServer message to BARE bytes.
pub fn encode_to_server(msg: &ToServer) -> Result<Vec<u8>, serde_bare::error::Error> {
    serde_bare::to_vec(msg)
}

/// Deserialize a ToServer message from BARE bytes.
pub fn decode_to_server(bytes: &[u8]) -> Result<ToServer, serde_bare::error::Error> {
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
        let msg = ToServer::ToServerRequest(ToServerRequest {
            request_id: 1,
            actor_id: "abc".into(),
            data: RequestData::ActorOpenRequest,
        });
        let bytes = encode_to_server(&msg).unwrap();
        let decoded = decode_to_server(&bytes).unwrap();
        assert_eq!(msg, decoded);
    }

    #[test]
    fn round_trip_to_server_request_actor_close() {
        let msg = ToServer::ToServerRequest(ToServerRequest {
            request_id: 2,
            actor_id: "test-actor-123".into(),
            data: RequestData::ActorCloseRequest,
        });
        let bytes = encode_to_server(&msg).unwrap();
        let decoded = decode_to_server(&bytes).unwrap();
        assert_eq!(msg, decoded);
    }

    #[test]
    fn round_trip_to_server_request_kv_get() {
        let msg = ToServer::ToServerRequest(ToServerRequest {
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
    fn round_trip_to_server_request_kv_put() {
        let msg = ToServer::ToServerRequest(ToServerRequest {
            request_id: 4,
            actor_id: "actor2".into(),
            data: RequestData::KvPutRequest(KvPutRequest {
                keys: vec![vec![0x08, 0x01, 0x01, 0x00]],
                values: vec![vec![0xFF; 4096]],
            }),
        });
        let bytes = encode_to_server(&msg).unwrap();
        let decoded = decode_to_server(&bytes).unwrap();
        assert_eq!(msg, decoded);
    }

    #[test]
    fn round_trip_to_server_request_kv_delete() {
        let msg = ToServer::ToServerRequest(ToServerRequest {
            request_id: 5,
            actor_id: "a".into(),
            data: RequestData::KvDeleteRequest(KvDeleteRequest {
                keys: vec![vec![1], vec![2], vec![3]],
            }),
        });
        let bytes = encode_to_server(&msg).unwrap();
        let decoded = decode_to_server(&bytes).unwrap();
        assert_eq!(msg, decoded);
    }

    #[test]
    fn round_trip_to_server_request_kv_delete_range() {
        let msg = ToServer::ToServerRequest(ToServerRequest {
            request_id: 6,
            actor_id: "actor3".into(),
            data: RequestData::KvDeleteRangeRequest(KvDeleteRangeRequest {
                start: vec![0x08, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00],
                end: vec![0x08, 0x01, 0x01, 0x01],
            }),
        });
        let bytes = encode_to_server(&msg).unwrap();
        let decoded = decode_to_server(&bytes).unwrap();
        assert_eq!(msg, decoded);
    }

    #[test]
    fn round_trip_to_server_pong() {
        let msg = ToServer::ToServerPong(ToServerPong { ts: 1234567890 });
        let bytes = encode_to_server(&msg).unwrap();
        let decoded = decode_to_server(&bytes).unwrap();
        assert_eq!(msg, decoded);
    }

    #[test]
    fn round_trip_to_server_pong_negative_ts() {
        let msg = ToServer::ToServerPong(ToServerPong { ts: -1 });
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
    fn round_trip_to_client_response_actor_open() {
        let msg = ToClient::ToClientResponse(ToClientResponse {
            request_id: 11,
            data: ResponseData::ActorOpenResponse,
        });
        let bytes = encode_to_client(&msg).unwrap();
        let decoded = decode_to_client(&bytes).unwrap();
        assert_eq!(msg, decoded);
    }

    #[test]
    fn round_trip_to_client_response_actor_close() {
        let msg = ToClient::ToClientResponse(ToClientResponse {
            request_id: 12,
            data: ResponseData::ActorCloseResponse,
        });
        let bytes = encode_to_client(&msg).unwrap();
        let decoded = decode_to_client(&bytes).unwrap();
        assert_eq!(msg, decoded);
    }

    #[test]
    fn round_trip_to_client_response_kv_get() {
        let msg = ToClient::ToClientResponse(ToClientResponse {
            request_id: 13,
            data: ResponseData::KvGetResponse(KvGetResponse {
                keys: vec![vec![1, 2], vec![3, 4]],
                values: vec![vec![10, 20], vec![30, 40, 50]],
            }),
        });
        let bytes = encode_to_client(&msg).unwrap();
        let decoded = decode_to_client(&bytes).unwrap();
        assert_eq!(msg, decoded);
    }

    #[test]
    fn round_trip_to_client_response_kv_get_empty() {
        let msg = ToClient::ToClientResponse(ToClientResponse {
            request_id: 14,
            data: ResponseData::KvGetResponse(KvGetResponse {
                keys: vec![],
                values: vec![],
            }),
        });
        let bytes = encode_to_client(&msg).unwrap();
        let decoded = decode_to_client(&bytes).unwrap();
        assert_eq!(msg, decoded);
    }

    #[test]
    fn round_trip_to_client_response_kv_put() {
        let msg = ToClient::ToClientResponse(ToClientResponse {
            request_id: 15,
            data: ResponseData::KvPutResponse,
        });
        let bytes = encode_to_client(&msg).unwrap();
        let decoded = decode_to_client(&bytes).unwrap();
        assert_eq!(msg, decoded);
    }

    #[test]
    fn round_trip_to_client_response_kv_delete() {
        let msg = ToClient::ToClientResponse(ToClientResponse {
            request_id: 16,
            data: ResponseData::KvDeleteResponse,
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
    //
    // These verify exact byte output matching the TypeScript implementation
    // in engine/sdks/typescript/kv-channel-protocol/src/index.ts.

    #[test]
    fn bytes_to_server_request_actor_open() {
        // ToServer { tag: "ToServerRequest", val: { requestId: 1, actorId: "abc", data: { tag: "ActorOpenRequest" } } }
        let msg = ToServer::ToServerRequest(ToServerRequest {
            request_id: 1,
            actor_id: "abc".into(),
            data: RequestData::ActorOpenRequest,
        });
        let bytes = encode_to_server(&msg).unwrap();
        // tag(0) + u32_le(1) + str("abc": len=3 + bytes) + tag(0)
        assert_eq!(
            bytes,
            [0x00, 0x01, 0x00, 0x00, 0x00, 0x03, 0x61, 0x62, 0x63, 0x00]
        );
    }

    #[test]
    fn bytes_to_server_pong() {
        // ToServer { tag: "ToServerPong", val: { ts: 1234567890n } }
        let msg = ToServer::ToServerPong(ToServerPong { ts: 1234567890 });
        let bytes = encode_to_server(&msg).unwrap();
        // tag(1) + i64_le(1234567890)
        assert_eq!(
            bytes,
            [0x01, 0xD2, 0x02, 0x96, 0x49, 0x00, 0x00, 0x00, 0x00]
        );
    }

    #[test]
    fn bytes_to_client_close() {
        // ToClient { tag: "ToClientClose", val: null }
        let msg = ToClient::ToClientClose;
        let bytes = encode_to_client(&msg).unwrap();
        // Just the tag byte for void type.
        assert_eq!(bytes, [0x02]);
    }

    #[test]
    fn bytes_to_client_ping() {
        let msg = ToClient::ToClientPing(ToClientPing { ts: 0 });
        let bytes = encode_to_client(&msg).unwrap();
        // tag(1) + i64_le(0)
        assert_eq!(
            bytes,
            [0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]
        );
    }

    #[test]
    fn bytes_to_client_response_kv_get() {
        // ToClient { tag: "ToClientResponse", val: { requestId: 42, data: { tag: "KvGetResponse", val: { keys: [[1,2]], values: [[3,4,5]] } } } }
        let msg = ToClient::ToClientResponse(ToClientResponse {
            request_id: 42,
            data: ResponseData::KvGetResponse(KvGetResponse {
                keys: vec![vec![1, 2]],
                values: vec![vec![3, 4, 5]],
            }),
        });
        let bytes = encode_to_client(&msg).unwrap();
        // tag(0) + u32_le(42) + tag(3) + varint(1) + varint(2) + [1,2] + varint(1) + varint(3) + [3,4,5]
        assert_eq!(
            bytes,
            [0x00, 0x2A, 0x00, 0x00, 0x00, 0x03, 0x01, 0x02, 0x01, 0x02, 0x01, 0x03, 0x03, 0x04, 0x05]
        );
    }

    #[test]
    fn bytes_to_client_response_actor_open_void() {
        let msg = ToClient::ToClientResponse(ToClientResponse {
            request_id: 0,
            data: ResponseData::ActorOpenResponse,
        });
        let bytes = encode_to_client(&msg).unwrap();
        // tag(0) + u32_le(0) + tag(1)
        assert_eq!(bytes, [0x00, 0x00, 0x00, 0x00, 0x00, 0x01]);
    }

    #[test]
    fn bytes_to_client_response_error() {
        let msg = ToClient::ToClientResponse(ToClientResponse {
            request_id: 7,
            data: ResponseData::ErrorResponse(ErrorResponse {
                code: "err".into(),
                message: "bad".into(),
            }),
        });
        let bytes = encode_to_client(&msg).unwrap();
        // tag(0) + u32_le(7) + tag(0) + str("err": 3+bytes) + str("bad": 3+bytes)
        assert_eq!(
            bytes,
            [0x00, 0x07, 0x00, 0x00, 0x00, 0x00, 0x03, 0x65, 0x72, 0x72, 0x03, 0x62, 0x61, 0x64]
        );
    }

    #[test]
    fn bytes_to_server_request_kv_delete_range() {
        let msg = ToServer::ToServerRequest(ToServerRequest {
            request_id: 0,
            actor_id: "x".into(),
            data: RequestData::KvDeleteRangeRequest(KvDeleteRangeRequest {
                start: vec![0x08, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00],
                end: vec![0x08, 0x01, 0x01, 0x01],
            }),
        });
        let bytes = encode_to_server(&msg).unwrap();
        // tag(0) + u32_le(0) + str("x": 1+byte) + tag(5) + data(start: 8+bytes) + data(end: 4+bytes)
        let expected: Vec<u8> = vec![
            0x00, // ToServer tag: ToServerRequest
            0x00, 0x00, 0x00, 0x00, // requestId: 0
            0x01, 0x78, // actorId: "x" (len=1 + 'x')
            0x05, // RequestData tag: KvDeleteRangeRequest
            0x08, 0x08, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, // start (len=8 + bytes)
            0x04, 0x08, 0x01, 0x01, 0x01, // end (len=4 + bytes)
        ];
        assert_eq!(bytes, expected);
    }

    // MARK: Decode error tests

    #[test]
    fn decode_to_server_invalid_tag() {
        // Tag byte 0xFF is not a valid ToServer variant.
        let result = decode_to_server(&[0xFF]);
        assert!(result.is_err());
    }

    #[test]
    fn decode_to_client_empty() {
        let result = decode_to_client(&[]);
        assert!(result.is_err());
    }

    #[test]
    fn decode_to_server_truncated() {
        // ToServerRequest tag but missing fields.
        let result = decode_to_server(&[0x00]);
        assert!(result.is_err());
    }
}
