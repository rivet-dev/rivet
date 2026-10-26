use anyhow::Result;
use vbare::Versioned;

use rivetkit_inspector_protocol::{self as wire, versioned};

pub(crate) use wire::{
	ActionResponse, Connection as ConnectionDetails, ConnectionsResponse, ConnectionsUpdated,
	DatabaseSchemaResponse, DatabaseTableRowsResponse, Error as ErrorMessage, Init as InitMessage,
	QueueMessageSummary, QueueResponse, QueueStatus, QueueUpdated, RpcsListResponse, Schedule,
	ScheduleDeleteResponse, ScheduleError, ScheduleFire, ScheduleHistoryResponse,
	SchedulesResponse, SchedulesUpdated, StateResponse, StateUpdated, TabConfigEntry,
	ToClientBody as ServerMessage, ToServerBody as ClientMessage, TraceQueryResponse,
	WorkflowHistoryResponse, WorkflowHistoryUpdated, WorkflowReplayResponse,
};

const MAX_QUEUE_STATUS_LIMIT: u32 = 200;
pub(crate) const PROTOCOL_VERSION: u16 = wire::PROTOCOL_VERSION;

pub(crate) fn decode_client_message(payload: &[u8]) -> Result<ClientMessage> {
	Ok(<versioned::ToServer as Versioned>::deserialize_with_embedded_version(payload)?.body)
}

pub(crate) fn encode_server_message_embedded(
	message: &ServerMessage,
	version: u16,
) -> Result<Vec<u8>> {
	versioned::ToClient::serialize_with_embedded_version(
		&wire::ToClient {
			body: message.clone(),
		},
		version,
	)
}

pub(crate) fn clamp_queue_limit(limit: u64) -> u32 {
	limit.min(u64::from(MAX_QUEUE_STATUS_LIMIT)) as u32
}

pub(crate) fn decode_client_payload(payload: &[u8], version: u16) -> Result<ClientMessage> {
	Ok(<versioned::ToServer as Versioned>::deserialize(payload, version)?.body)
}

pub(crate) fn encode_client_payload_current(message: &ClientMessage) -> Result<Vec<u8>> {
	versioned::ToServer::serialize(
		&wire::ToServer {
			body: message.clone(),
		},
		wire::PROTOCOL_VERSION,
	)
}

pub(crate) fn decode_current_server_payload(payload: &[u8]) -> Result<ServerMessage> {
	Ok(<versioned::ToClient as Versioned>::deserialize(payload, wire::PROTOCOL_VERSION)?.body)
}

pub(crate) fn encode_server_payload(message: &ServerMessage, version: u16) -> Result<Vec<u8>> {
	versioned::ToClient::serialize(
		&wire::ToClient {
			body: message.clone(),
		},
		version,
	)
}


