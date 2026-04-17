use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

const EMBEDDED_VERSION_LEN: usize = 2;

pub(crate) const CURRENT_VERSION: u16 = 4;
const SUPPORTED_VERSIONS: &[u16] = &[1, 2, 3, 4];
const MAX_QUEUE_STATUS_LIMIT: u32 = 200;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ClientMessage {
	PatchState(PatchStateRequest),
	StateRequest(IdRequest),
	ConnectionsRequest(IdRequest),
	ActionRequest(ActionRequest),
	RpcsListRequest(IdRequest),
	TraceQueryRequest(TraceQueryRequest),
	QueueRequest(QueueRequest),
	WorkflowHistoryRequest(IdRequest),
	WorkflowReplayRequest(WorkflowReplayRequest),
	DatabaseSchemaRequest(IdRequest),
	DatabaseTableRowsRequest(DatabaseTableRowsRequest),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ServerMessage {
	StateResponse(StateResponse),
	ConnectionsResponse(ConnectionsResponse),
	ActionResponse(ActionResponse),
	ConnectionsUpdated(ConnectionsUpdated),
	QueueUpdated(QueueUpdated),
	StateUpdated(StateUpdated),
	WorkflowHistoryUpdated(WorkflowHistoryUpdated),
	RpcsListResponse(RpcsListResponse),
	TraceQueryResponse(TraceQueryResponse),
	QueueResponse(QueueResponse),
	WorkflowHistoryResponse(WorkflowHistoryResponse),
	WorkflowReplayResponse(WorkflowReplayResponse),
	Error(ErrorMessage),
	Init(InitMessage),
	DatabaseSchemaResponse(DatabaseSchemaResponse),
	DatabaseTableRowsResponse(DatabaseTableRowsResponse),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct PatchStateRequest {
	pub state: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct IdRequest {
	pub id: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ActionRequest {
	pub id: u64,
	pub name: String,
	pub args: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct TraceQueryRequest {
	pub id: u64,
	pub start_ms: u64,
	pub end_ms: u64,
	pub limit: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct QueueRequest {
	pub id: u64,
	pub limit: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct WorkflowReplayRequest {
	pub id: u64,
	pub entry_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct DatabaseTableRowsRequest {
	pub id: u64,
	pub table: String,
	pub limit: u64,
	pub offset: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ConnectionDetails {
	pub id: String,
	pub details: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct InitMessage {
	pub connections: Vec<ConnectionDetails>,
	pub state: Option<Vec<u8>>,
	pub is_state_enabled: bool,
	pub rpcs: Vec<String>,
	pub is_database_enabled: bool,
	pub queue_size: u64,
	pub workflow_history: Option<Vec<u8>>,
	pub is_workflow_enabled: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ConnectionsResponse {
	pub rid: u64,
	pub connections: Vec<ConnectionDetails>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct StateResponse {
	pub rid: u64,
	pub state: Option<Vec<u8>>,
	pub is_state_enabled: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ActionResponse {
	pub rid: u64,
	pub output: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct TraceQueryResponse {
	pub rid: u64,
	pub payload: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct QueueMessageSummary {
	pub id: u64,
	pub name: String,
	pub created_at_ms: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct QueueStatus {
	pub size: u64,
	pub max_size: u64,
	pub messages: Vec<QueueMessageSummary>,
	pub truncated: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct QueueResponse {
	pub rid: u64,
	pub status: QueueStatus,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct WorkflowHistoryResponse {
	pub rid: u64,
	pub history: Option<Vec<u8>>,
	pub is_workflow_enabled: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct WorkflowReplayResponse {
	pub rid: u64,
	pub history: Option<Vec<u8>>,
	pub is_workflow_enabled: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct DatabaseSchemaResponse {
	pub rid: u64,
	pub schema: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct DatabaseTableRowsResponse {
	pub rid: u64,
	pub result: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct StateUpdated {
	pub state: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct QueueUpdated {
	pub queue_size: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct WorkflowHistoryUpdated {
	pub history: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct RpcsListResponse {
	pub rid: u64,
	pub rpcs: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ConnectionsUpdated {
	pub connections: Vec<ConnectionDetails>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ErrorMessage {
	pub message: String,
}

pub(crate) fn decode_client_message(payload: &[u8]) -> Result<ClientMessage> {
	let (version, body) = split_version(payload)?;
	let Some((&tag, body)) = body.split_first() else {
		bail!("inspector websocket payload was empty");
	};

	match version {
		1 => decode_v1_message(tag, body),
		2 => decode_v2_message(tag, body),
		3 => decode_v3_message(tag, body),
		4 => decode_v4_message(tag, body),
		_ => bail!("unsupported inspector websocket version {version}"),
	}
}

pub(crate) fn encode_server_message(message: &ServerMessage) -> Result<Vec<u8>> {
	let mut encoded = Vec::new();
	encoded.extend_from_slice(&CURRENT_VERSION.to_le_bytes());
	let (tag, payload) = match message {
		ServerMessage::StateResponse(payload) => (0, encode_payload(payload, "state response")?),
		ServerMessage::ConnectionsResponse(payload) => {
			(1, encode_payload(payload, "connections response")?)
		}
		ServerMessage::ActionResponse(payload) => (2, encode_payload(payload, "action response")?),
		ServerMessage::ConnectionsUpdated(payload) => {
			(3, encode_payload(payload, "connections updated")?)
		}
		ServerMessage::QueueUpdated(payload) => (4, encode_payload(payload, "queue updated")?),
		ServerMessage::StateUpdated(payload) => (5, encode_payload(payload, "state updated")?),
		ServerMessage::WorkflowHistoryUpdated(payload) => {
			(6, encode_payload(payload, "workflow history updated")?)
		}
		ServerMessage::RpcsListResponse(payload) => {
			(7, encode_payload(payload, "rpcs list response")?)
		}
		ServerMessage::TraceQueryResponse(payload) => {
			(8, encode_payload(payload, "trace query response")?)
		}
		ServerMessage::QueueResponse(payload) => (9, encode_payload(payload, "queue response")?),
		ServerMessage::WorkflowHistoryResponse(payload) => {
			(10, encode_payload(payload, "workflow history response")?)
		}
		ServerMessage::WorkflowReplayResponse(payload) => {
			(11, encode_payload(payload, "workflow replay response")?)
		}
		ServerMessage::Error(payload) => (12, encode_payload(payload, "error response")?),
		ServerMessage::Init(payload) => (13, encode_payload(payload, "init message")?),
		ServerMessage::DatabaseSchemaResponse(payload) => {
			(14, encode_payload(payload, "database schema response")?)
		}
		ServerMessage::DatabaseTableRowsResponse(payload) => {
			(15, encode_payload(payload, "database table rows response")?)
		}
	};
	encoded.push(tag);
	encoded.extend_from_slice(&payload);
	Ok(encoded)
}

pub(crate) fn clamp_queue_limit(limit: u64) -> u32 {
	limit.min(u64::from(MAX_QUEUE_STATUS_LIMIT)) as u32
}

fn split_version(payload: &[u8]) -> Result<(u16, &[u8])> {
	if payload.len() < EMBEDDED_VERSION_LEN {
		bail!("inspector websocket payload too short for embedded version");
	}

	let version = u16::from_le_bytes([payload[0], payload[1]]);
	if !SUPPORTED_VERSIONS.contains(&version) {
		bail!(
			"unsupported inspector websocket version {version}; expected one of {:?}",
			SUPPORTED_VERSIONS
		);
	}

	Ok((version, &payload[EMBEDDED_VERSION_LEN..]))
}

fn decode_v1_message(tag: u8, body: &[u8]) -> Result<ClientMessage> {
	match tag {
		0 => decode_payload(body, "patch state request").map(ClientMessage::PatchState),
		1 => decode_payload(body, "state request").map(ClientMessage::StateRequest),
		2 => decode_payload(body, "connections request").map(ClientMessage::ConnectionsRequest),
		3 => decode_payload(body, "action request").map(ClientMessage::ActionRequest),
		4 | 5 => bail!("Cannot convert events requests to v2"),
		6 => decode_payload(body, "rpcs list request").map(ClientMessage::RpcsListRequest),
		_ => bail!("unknown inspector v1 request tag {tag}"),
	}
}

fn decode_v2_message(tag: u8, body: &[u8]) -> Result<ClientMessage> {
	match tag {
		0 => decode_payload(body, "patch state request").map(ClientMessage::PatchState),
		1 => decode_payload(body, "state request").map(ClientMessage::StateRequest),
		2 => decode_payload(body, "connections request").map(ClientMessage::ConnectionsRequest),
		3 => decode_payload(body, "action request").map(ClientMessage::ActionRequest),
		4 => decode_payload(body, "rpcs list request").map(ClientMessage::RpcsListRequest),
		5 => decode_payload(body, "trace query request").map(ClientMessage::TraceQueryRequest),
		6 => decode_payload(body, "queue request").map(ClientMessage::QueueRequest),
		7 => decode_payload(body, "workflow history request")
			.map(ClientMessage::WorkflowHistoryRequest),
		_ => bail!("unknown inspector v2 request tag {tag}"),
	}
}

fn decode_v3_message(tag: u8, body: &[u8]) -> Result<ClientMessage> {
	match tag {
		0 => decode_payload(body, "patch state request").map(ClientMessage::PatchState),
		1 => decode_payload(body, "state request").map(ClientMessage::StateRequest),
		2 => decode_payload(body, "connections request").map(ClientMessage::ConnectionsRequest),
		3 => decode_payload(body, "action request").map(ClientMessage::ActionRequest),
		4 => decode_payload(body, "rpcs list request").map(ClientMessage::RpcsListRequest),
		5 => decode_payload(body, "trace query request").map(ClientMessage::TraceQueryRequest),
		6 => decode_payload(body, "queue request").map(ClientMessage::QueueRequest),
		7 => decode_payload(body, "workflow history request")
			.map(ClientMessage::WorkflowHistoryRequest),
		8 => decode_payload(body, "database schema request")
			.map(ClientMessage::DatabaseSchemaRequest),
		9 => decode_payload(body, "database table rows request")
			.map(ClientMessage::DatabaseTableRowsRequest),
		_ => bail!("unknown inspector v3 request tag {tag}"),
	}
}

fn decode_v4_message(tag: u8, body: &[u8]) -> Result<ClientMessage> {
	match tag {
		0 => decode_payload(body, "patch state request").map(ClientMessage::PatchState),
		1 => decode_payload(body, "state request").map(ClientMessage::StateRequest),
		2 => decode_payload(body, "connections request").map(ClientMessage::ConnectionsRequest),
		3 => decode_payload(body, "action request").map(ClientMessage::ActionRequest),
		4 => decode_payload(body, "rpcs list request").map(ClientMessage::RpcsListRequest),
		5 => decode_payload(body, "trace query request").map(ClientMessage::TraceQueryRequest),
		6 => decode_payload(body, "queue request").map(ClientMessage::QueueRequest),
		7 => decode_payload(body, "workflow history request")
			.map(ClientMessage::WorkflowHistoryRequest),
		8 => decode_payload(body, "workflow replay request")
			.map(ClientMessage::WorkflowReplayRequest),
		9 => decode_payload(body, "database schema request")
			.map(ClientMessage::DatabaseSchemaRequest),
		10 => decode_payload(body, "database table rows request")
			.map(ClientMessage::DatabaseTableRowsRequest),
		_ => bail!("unknown inspector v4 request tag {tag}"),
	}
}

fn decode_payload<T>(payload: &[u8], label: &str) -> Result<T>
where
	T: for<'de> Deserialize<'de>,
{
	serde_bare::from_slice(payload).with_context(|| format!("decode inspector {label}"))
}

fn encode_payload<T>(payload: &T, label: &str) -> Result<Vec<u8>>
where
	T: Serialize,
{
	serde_bare::to_vec(payload).with_context(|| format!("encode inspector {label}"))
}
