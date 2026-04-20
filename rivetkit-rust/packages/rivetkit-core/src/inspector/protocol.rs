use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

const EMBEDDED_VERSION_LEN: usize = 2;

pub(crate) const CURRENT_VERSION: u16 = 4;
const SUPPORTED_VERSIONS: &[u16] = &[1, 2, 3, 4];
const MAX_QUEUE_STATUS_LIMIT: u32 = 200;
const WORKFLOW_HISTORY_DROPPED_ERROR: &str = "inspector.workflow_history_dropped";
const QUEUE_DROPPED_ERROR: &str = "inspector.queue_dropped";
const TRACE_DROPPED_ERROR: &str = "inspector.trace_dropped";
const DATABASE_DROPPED_ERROR: &str = "inspector.database_dropped";

mod bare_uint {
	use serde::{Deserialize, Deserializer, Serialize, Serializer};

	pub fn serialize<S>(value: &u64, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		serde_bare::Uint(*value).serialize(serializer)
	}

	pub fn deserialize<'de, D>(deserializer: D) -> Result<u64, D::Error>
	where
		D: Deserializer<'de>,
	{
		let serde_bare::Uint(value) = serde_bare::Uint::deserialize(deserializer)?;
		Ok(value)
	}
}

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
	#[serde(with = "serde_bytes")]
	pub state: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct IdRequest {
	#[serde(with = "bare_uint")]
	pub id: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ActionRequest {
	#[serde(with = "bare_uint")]
	pub id: u64,
	pub name: String,
	#[serde(with = "serde_bytes")]
	pub args: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct TraceQueryRequest {
	#[serde(with = "bare_uint")]
	pub id: u64,
	#[serde(with = "bare_uint")]
	pub start_ms: u64,
	#[serde(with = "bare_uint")]
	pub end_ms: u64,
	#[serde(with = "bare_uint")]
	pub limit: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct QueueRequest {
	#[serde(with = "bare_uint")]
	pub id: u64,
	#[serde(with = "bare_uint")]
	pub limit: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct WorkflowReplayRequest {
	#[serde(with = "bare_uint")]
	pub id: u64,
	pub entry_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct DatabaseTableRowsRequest {
	#[serde(with = "bare_uint")]
	pub id: u64,
	pub table: String,
	#[serde(with = "bare_uint")]
	pub limit: u64,
	#[serde(with = "bare_uint")]
	pub offset: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ConnectionDetails {
	pub id: String,
	#[serde(with = "serde_bytes")]
	pub details: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct InitMessage {
	pub connections: Vec<ConnectionDetails>,
	#[serde(with = "serde_bytes")]
	pub state: Option<Vec<u8>>,
	pub is_state_enabled: bool,
	pub rpcs: Vec<String>,
	pub is_database_enabled: bool,
	#[serde(with = "bare_uint")]
	pub queue_size: u64,
	#[serde(with = "serde_bytes")]
	pub workflow_history: Option<Vec<u8>>,
	#[serde(rename = "isWorkflowEnabled")]
	pub workflow_supported: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ConnectionsResponse {
	#[serde(with = "bare_uint")]
	pub rid: u64,
	pub connections: Vec<ConnectionDetails>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct StateResponse {
	#[serde(with = "bare_uint")]
	pub rid: u64,
	#[serde(with = "serde_bytes")]
	pub state: Option<Vec<u8>>,
	pub is_state_enabled: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ActionResponse {
	#[serde(with = "bare_uint")]
	pub rid: u64,
	#[serde(with = "serde_bytes")]
	pub output: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct TraceQueryResponse {
	#[serde(with = "bare_uint")]
	pub rid: u64,
	#[serde(with = "serde_bytes")]
	pub payload: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct QueueMessageSummary {
	#[serde(with = "bare_uint")]
	pub id: u64,
	pub name: String,
	#[serde(with = "bare_uint")]
	pub created_at_ms: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct QueueStatus {
	#[serde(with = "bare_uint")]
	pub size: u64,
	#[serde(with = "bare_uint")]
	pub max_size: u64,
	pub messages: Vec<QueueMessageSummary>,
	pub truncated: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct QueueResponse {
	#[serde(with = "bare_uint")]
	pub rid: u64,
	pub status: QueueStatus,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct WorkflowHistoryResponse {
	#[serde(with = "bare_uint")]
	pub rid: u64,
	#[serde(with = "serde_bytes")]
	pub history: Option<Vec<u8>>,
	#[serde(rename = "isWorkflowEnabled")]
	pub workflow_supported: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct WorkflowReplayResponse {
	#[serde(with = "bare_uint")]
	pub rid: u64,
	#[serde(with = "serde_bytes")]
	pub history: Option<Vec<u8>>,
	#[serde(rename = "isWorkflowEnabled")]
	pub workflow_supported: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct DatabaseSchemaResponse {
	#[serde(with = "bare_uint")]
	pub rid: u64,
	#[serde(with = "serde_bytes")]
	pub schema: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct DatabaseTableRowsResponse {
	#[serde(with = "bare_uint")]
	pub rid: u64,
	#[serde(with = "serde_bytes")]
	pub result: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct StateUpdated {
	#[serde(with = "serde_bytes")]
	pub state: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct QueueUpdated {
	#[serde(with = "bare_uint")]
	pub queue_size: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct WorkflowHistoryUpdated {
	#[serde(with = "serde_bytes")]
	pub history: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct RpcsListResponse {
	#[serde(with = "bare_uint")]
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

#[derive(Debug, Serialize)]
struct V1InitMessageEncode {
	pub connections: Vec<ConnectionDetails>,
	pub events: Vec<()>,
	#[serde(with = "serde_bytes")]
	pub state: Option<Vec<u8>>,
	pub is_state_enabled: bool,
	pub rpcs: Vec<String>,
	pub is_database_enabled: bool,
}

pub(crate) fn decode_client_message(payload: &[u8]) -> Result<ClientMessage> {
	let (version, body) = split_version(payload)?;
	decode_client_payload(body, version)
}

pub(crate) fn encode_server_message(message: &ServerMessage) -> Result<Vec<u8>> {
	encode_server_payload_with_embedded_version(message, CURRENT_VERSION)
}

pub(crate) fn clamp_queue_limit(limit: u64) -> u32 {
	limit.min(u64::from(MAX_QUEUE_STATUS_LIMIT)) as u32
}

pub(crate) fn decode_client_payload(payload: &[u8], version: u16) -> Result<ClientMessage> {
	let Some((&tag, body)) = payload.split_first() else {
		bail!("inspector websocket payload was empty");
	};

	match version {
		1 => decode_v1_message(tag, body),
		2 => decode_v2_message(tag, body),
		3 => decode_v3_message(tag, body),
		4 => decode_v4_message(tag, body),
		_ => unsupported_version(version),
	}
}

pub(crate) fn encode_client_payload_current(message: &ClientMessage) -> Result<Vec<u8>> {
	let (tag, payload) = match message {
		ClientMessage::PatchState(payload) => (0, encode_payload(payload, "patch state request")?),
		ClientMessage::StateRequest(payload) => (1, encode_payload(payload, "state request")?),
		ClientMessage::ConnectionsRequest(payload) => {
			(2, encode_payload(payload, "connections request")?)
		}
		ClientMessage::ActionRequest(payload) => (3, encode_payload(payload, "action request")?),
		ClientMessage::RpcsListRequest(payload) => {
			(4, encode_payload(payload, "rpcs list request")?)
		}
		ClientMessage::TraceQueryRequest(payload) => {
			(5, encode_payload(payload, "trace query request")?)
		}
		ClientMessage::QueueRequest(payload) => (6, encode_payload(payload, "queue request")?),
		ClientMessage::WorkflowHistoryRequest(payload) => {
			(7, encode_payload(payload, "workflow history request")?)
		}
		ClientMessage::WorkflowReplayRequest(payload) => {
			(8, encode_payload(payload, "workflow replay request")?)
		}
		ClientMessage::DatabaseSchemaRequest(payload) => {
			(9, encode_payload(payload, "database schema request")?)
		}
		ClientMessage::DatabaseTableRowsRequest(payload) => {
			(10, encode_payload(payload, "database table rows request")?)
		}
	};

	Ok(encode_tagged_payload(tag, payload))
}

pub(crate) fn decode_current_server_payload(payload: &[u8]) -> Result<ServerMessage> {
	let Some((&tag, body)) = payload.split_first() else {
		bail!("inspector websocket payload was empty");
	};

	match tag {
		0 => decode_payload(body, "state response").map(ServerMessage::StateResponse),
		1 => decode_payload(body, "connections response").map(ServerMessage::ConnectionsResponse),
		2 => decode_payload(body, "action response").map(ServerMessage::ActionResponse),
		3 => decode_payload(body, "connections updated").map(ServerMessage::ConnectionsUpdated),
		4 => decode_payload(body, "queue updated").map(ServerMessage::QueueUpdated),
		5 => decode_payload(body, "state updated").map(ServerMessage::StateUpdated),
		6 => decode_payload(body, "workflow history updated")
			.map(ServerMessage::WorkflowHistoryUpdated),
		7 => decode_payload(body, "rpcs list response").map(ServerMessage::RpcsListResponse),
		8 => decode_payload(body, "trace query response").map(ServerMessage::TraceQueryResponse),
		9 => decode_payload(body, "queue response").map(ServerMessage::QueueResponse),
		10 => decode_payload(body, "workflow history response")
			.map(ServerMessage::WorkflowHistoryResponse),
		11 => decode_payload(body, "workflow replay response")
			.map(ServerMessage::WorkflowReplayResponse),
		12 => decode_payload(body, "error response").map(ServerMessage::Error),
		13 => decode_payload(body, "init message").map(ServerMessage::Init),
		14 => decode_payload(body, "database schema response")
			.map(ServerMessage::DatabaseSchemaResponse),
		15 => decode_payload(body, "database table rows response")
			.map(ServerMessage::DatabaseTableRowsResponse),
		_ => bail!("unknown inspector v4 response tag {tag}"),
	}
}

pub(crate) fn encode_server_payload(message: &ServerMessage, version: u16) -> Result<Vec<u8>> {
	match version {
		1 => encode_v1_server_message(message),
		2 => encode_v2_server_message(message),
		3 => encode_v3_server_message(message),
		4 => encode_v4_server_message(message),
		_ => unsupported_version(version),
	}
}

fn encode_server_payload_with_embedded_version(
	message: &ServerMessage,
	version: u16,
) -> Result<Vec<u8>> {
	let mut encoded = Vec::new();
	encoded.extend_from_slice(&version.to_le_bytes());
	encoded.extend_from_slice(&encode_server_payload(message, version)?);
	Ok(encoded)
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

fn unsupported_version<T>(version: u16) -> Result<T> {
	bail!(
		"unsupported inspector websocket version {version}; expected one of {:?}",
		SUPPORTED_VERSIONS
	);
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

fn encode_v1_server_message(message: &ServerMessage) -> Result<Vec<u8>> {
	let (tag, payload) = match message {
		ServerMessage::StateResponse(payload) => (0, encode_payload(payload, "state response")?),
		ServerMessage::ConnectionsResponse(payload) => {
			(1, encode_payload(payload, "connections response")?)
		}
		ServerMessage::ActionResponse(payload) => (3, encode_payload(payload, "action response")?),
		ServerMessage::ConnectionsUpdated(payload) => {
			(4, encode_payload(payload, "connections updated")?)
		}
		ServerMessage::StateUpdated(payload) => (6, encode_payload(payload, "state updated")?),
		ServerMessage::RpcsListResponse(payload) => {
			(7, encode_payload(payload, "rpcs list response")?)
		}
		ServerMessage::Error(payload) => (8, encode_payload(payload, "error response")?),
		ServerMessage::Init(payload) => (
			9,
			encode_payload(
				&V1InitMessageEncode {
					connections: payload.connections.clone(),
					events: Vec::new(),
					state: payload.state.clone(),
					is_state_enabled: payload.is_state_enabled,
					rpcs: payload.rpcs.clone(),
					is_database_enabled: payload.is_database_enabled,
				},
				"init message",
			)?,
		),
		ServerMessage::QueueUpdated(_) | ServerMessage::QueueResponse(_) => {
			encode_v1_error(QUEUE_DROPPED_ERROR)?
		}
		ServerMessage::WorkflowHistoryUpdated(_)
		| ServerMessage::WorkflowHistoryResponse(_)
		| ServerMessage::WorkflowReplayResponse(_) => {
			encode_v1_error(WORKFLOW_HISTORY_DROPPED_ERROR)?
		}
		ServerMessage::TraceQueryResponse(_) => encode_v1_error(TRACE_DROPPED_ERROR)?,
		ServerMessage::DatabaseSchemaResponse(_)
		| ServerMessage::DatabaseTableRowsResponse(_) => {
			encode_v1_error(DATABASE_DROPPED_ERROR)?
		}
	};

	Ok(encode_tagged_payload(tag, payload))
}

fn encode_v2_server_message(message: &ServerMessage) -> Result<Vec<u8>> {
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
		ServerMessage::Error(payload) => (11, encode_payload(payload, "error response")?),
		ServerMessage::Init(payload) => (12, encode_payload(payload, "init message")?),
		ServerMessage::WorkflowReplayResponse(_) => {
			encode_v2_error(WORKFLOW_HISTORY_DROPPED_ERROR)?
		}
		ServerMessage::DatabaseSchemaResponse(_)
		| ServerMessage::DatabaseTableRowsResponse(_) => {
			encode_v2_error(DATABASE_DROPPED_ERROR)?
		}
	};

	Ok(encode_tagged_payload(tag, payload))
}

fn encode_v3_server_message(message: &ServerMessage) -> Result<Vec<u8>> {
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
		ServerMessage::Error(payload) => (11, encode_payload(payload, "error response")?),
		ServerMessage::Init(payload) => (12, encode_payload(payload, "init message")?),
		ServerMessage::DatabaseSchemaResponse(payload) => {
			(13, encode_payload(payload, "database schema response")?)
		}
		ServerMessage::DatabaseTableRowsResponse(payload) => {
			(14, encode_payload(payload, "database table rows response")?)
		}
		ServerMessage::WorkflowReplayResponse(_) => {
			encode_v3_error(WORKFLOW_HISTORY_DROPPED_ERROR)?
		}
	};

	Ok(encode_tagged_payload(tag, payload))
}

fn encode_v4_server_message(message: &ServerMessage) -> Result<Vec<u8>> {
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

	Ok(encode_tagged_payload(tag, payload))
}

fn encode_v1_error(message: &str) -> Result<(u8, Vec<u8>)> {
	Ok((8, encode_payload(&dropped_error(message), "error response")?))
}

fn encode_v2_error(message: &str) -> Result<(u8, Vec<u8>)> {
	Ok((11, encode_payload(&dropped_error(message), "error response")?))
}

fn encode_v3_error(message: &str) -> Result<(u8, Vec<u8>)> {
	Ok((11, encode_payload(&dropped_error(message), "error response")?))
}

fn dropped_error(message: &str) -> ErrorMessage {
	ErrorMessage {
		message: message.to_owned(),
	}
}

fn encode_tagged_payload(tag: u8, payload: Vec<u8>) -> Vec<u8> {
	let mut encoded = Vec::with_capacity(1 + payload.len());
	encoded.push(tag);
	encoded.extend_from_slice(&payload);
	encoded
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
