use anyhow::Result;
use serde_bare::Uint;
use vbare::OwnedVersionedData;

use rivetkit_inspector_protocol::{self as wire, versioned};

pub(crate) use wire::{
	ActionResponse, Connection as ConnectionDetails, ConnectionsResponse, ConnectionsUpdated,
	DatabaseSchemaResponse, DatabaseTableRowsResponse, Error as ErrorMessage, Init as InitMessage,
	QueueMessageSummary, QueueResponse, QueueStatus, QueueUpdated, RpcsListResponse, StateResponse,
	StateUpdated, ToClientBody as ServerMessage, ToServerBody as ClientMessage, TraceQueryResponse,
	WorkflowHistoryResponse, WorkflowHistoryUpdated, WorkflowReplayResponse,
};

const MAX_QUEUE_STATUS_LIMIT: u32 = 200;

pub(crate) fn decode_client_message(payload: &[u8]) -> Result<ClientMessage> {
	Ok(
		<versioned::ToServer as OwnedVersionedData>::deserialize_with_embedded_version(payload)?
			.body,
	)
}

pub(crate) fn encode_server_message(message: &ServerMessage) -> Result<Vec<u8>> {
	versioned::ToClient::wrap_latest(wire::ToClient {
		body: message.clone(),
	})
	.serialize_with_embedded_version(wire::PROTOCOL_VERSION)
}

pub(crate) fn clamp_queue_limit(limit: Uint) -> u32 {
	limit.0.min(u64::from(MAX_QUEUE_STATUS_LIMIT)) as u32
}

pub(crate) fn decode_client_payload(payload: &[u8], version: u16) -> Result<ClientMessage> {
	Ok(<versioned::ToServer as OwnedVersionedData>::deserialize(payload, version)?.body)
}

pub(crate) fn encode_client_payload_current(message: &ClientMessage) -> Result<Vec<u8>> {
	versioned::ToServer::wrap_latest(wire::ToServer {
		body: message.clone(),
	})
	.serialize(wire::PROTOCOL_VERSION)
}

pub(crate) fn decode_current_server_payload(payload: &[u8]) -> Result<ServerMessage> {
	Ok(
		<versioned::ToClient as OwnedVersionedData>::deserialize(payload, wire::PROTOCOL_VERSION)?
			.body,
	)
}

pub(crate) fn encode_server_payload(message: &ServerMessage, version: u16) -> Result<Vec<u8>> {
	versioned::ToClient::wrap_latest(wire::ToClient {
		body: message.clone(),
	})
	.serialize(version)
}
