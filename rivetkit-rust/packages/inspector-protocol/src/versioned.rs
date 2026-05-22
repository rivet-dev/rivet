use anyhow::{Result, bail};
use serde_bare::Uint;
use vbare::OwnedVersionedData;

use crate::generated::{v1, v2, v3, v4};

const WORKFLOW_HISTORY_DROPPED_ERROR: &str = "inspector.workflow_history_dropped";
const QUEUE_DROPPED_ERROR: &str = "inspector.queue_dropped";
const TRACE_DROPPED_ERROR: &str = "inspector.trace_dropped";
const DATABASE_DROPPED_ERROR: &str = "inspector.database_dropped";

pub enum ToServer {
	V1(v1::ToServer),
	V2(v2::ToServer),
	V3(v3::ToServer),
	V4(v4::ToServer),
}

impl OwnedVersionedData for ToServer {
	type Latest = v4::ToServer;

	fn wrap_latest(latest: Self::Latest) -> Self {
		Self::V4(latest)
	}

	fn unwrap_latest(self) -> Result<Self::Latest> {
		match self {
			Self::V4(data) => Ok(data),
			_ => bail!("version not latest"),
		}
	}

	fn deserialize_version(payload: &[u8], version: u16) -> Result<Self> {
		match version {
			1 => Ok(Self::V1(serde_bare::from_slice(payload)?)),
			2 => Ok(Self::V2(serde_bare::from_slice(payload)?)),
			3 => Ok(Self::V3(serde_bare::from_slice(payload)?)),
			4 => Ok(Self::V4(serde_bare::from_slice(payload)?)),
			_ => bail!("invalid inspector protocol version for ToServer: {version}"),
		}
	}

	fn serialize_version(self, version: u16) -> Result<Vec<u8>> {
		match (self, version) {
			(Self::V1(data), 1) => serde_bare::to_vec(&data).map_err(Into::into),
			(Self::V2(data), 2) => serde_bare::to_vec(&data).map_err(Into::into),
			(Self::V3(data), 3) => serde_bare::to_vec(&data).map_err(Into::into),
			(Self::V4(data), 4) => serde_bare::to_vec(&data).map_err(Into::into),
			(_, version) => bail!("unexpected inspector protocol version for ToServer: {version}"),
		}
	}

	fn deserialize_converters() -> Vec<impl Fn(Self) -> Result<Self>> {
		vec![Self::v1_to_v2, Self::v2_to_v3, Self::v3_to_v4]
	}

	fn serialize_converters() -> Vec<impl Fn(Self) -> Result<Self>> {
		vec![Self::v4_to_v3, Self::v3_to_v2, Self::v2_to_v1]
	}
}

impl ToServer {
	fn v1_to_v2(self) -> Result<Self> {
		let Self::V1(data) = self else {
			bail!("expected inspector protocol v1 ToServer")
		};

		let body = match data.body {
			v1::ToServerBody::PatchStateRequest(req) => {
				v2::ToServerBody::PatchStateRequest(req.into())
			}
			v1::ToServerBody::StateRequest(req) => {
				v2::ToServerBody::StateRequest(req.into())
			}
			v1::ToServerBody::ConnectionsRequest(req) => {
				v2::ToServerBody::ConnectionsRequest(req.into())
			}
			v1::ToServerBody::ActionRequest(req) => {
				v2::ToServerBody::ActionRequest(req.into())
			}
			v1::ToServerBody::RpcsListRequest(req) => {
				v2::ToServerBody::RpcsListRequest(req.into())
			}
			v1::ToServerBody::EventsRequest(_) | v1::ToServerBody::ClearEventsRequest(_) => {
				bail!("cannot convert inspector v1 events requests to v2")
			}
		};

		Ok(Self::V2(v2::ToServer { body }))
	}

	fn v2_to_v3(self) -> Result<Self> {
		let Self::V2(data) = self else {
			bail!("expected inspector protocol v2 ToServer")
		};
		Ok(Self::V3(data.into()))
	}

	fn v3_to_v4(self) -> Result<Self> {
		let Self::V3(data) = self else {
			bail!("expected inspector protocol v3 ToServer")
		};

		let body = match data.body {
			v3::ToServerBody::PatchStateRequest(req) => {
				v4::ToServerBody::PatchStateRequest(req.into())
			}
			v3::ToServerBody::StateRequest(req) => {
				v4::ToServerBody::StateRequest(req.into())
			}
			v3::ToServerBody::ConnectionsRequest(req) => {
				v4::ToServerBody::ConnectionsRequest(req.into())
			}
			v3::ToServerBody::ActionRequest(req) => {
				v4::ToServerBody::ActionRequest(req.into())
			}
			v3::ToServerBody::RpcsListRequest(req) => {
				v4::ToServerBody::RpcsListRequest(req.into())
			}
			v3::ToServerBody::TraceQueryRequest(req) => {
				v4::ToServerBody::TraceQueryRequest(req.into())
			}
			v3::ToServerBody::QueueRequest(req) => {
				v4::ToServerBody::QueueRequest(req.into())
			}
			v3::ToServerBody::WorkflowHistoryRequest(req) => {
				v4::ToServerBody::WorkflowHistoryRequest(req.into())
			}
			v3::ToServerBody::DatabaseSchemaRequest(req) => {
				v4::ToServerBody::DatabaseSchemaRequest(req.into())
			}
			v3::ToServerBody::DatabaseTableRowsRequest(req) => {
				v4::ToServerBody::DatabaseTableRowsRequest(req.into())
			}
		};

		Ok(Self::V4(v4::ToServer { body }))
	}

	fn v4_to_v3(self) -> Result<Self> {
		let Self::V4(data) = self else {
			bail!("expected inspector protocol v4 ToServer")
		};

		let body = match data.body {
			v4::ToServerBody::PatchStateRequest(req) => {
				v3::ToServerBody::PatchStateRequest(req.into())
			}
			v4::ToServerBody::StateRequest(req) => {
				v3::ToServerBody::StateRequest(req.into())
			}
			v4::ToServerBody::ConnectionsRequest(req) => {
				v3::ToServerBody::ConnectionsRequest(req.into())
			}
			v4::ToServerBody::ActionRequest(req) => {
				v3::ToServerBody::ActionRequest(req.into())
			}
			v4::ToServerBody::RpcsListRequest(req) => {
				v3::ToServerBody::RpcsListRequest(req.into())
			}
			v4::ToServerBody::TraceQueryRequest(req) => {
				v3::ToServerBody::TraceQueryRequest(req.into())
			}
			v4::ToServerBody::QueueRequest(req) => {
				v3::ToServerBody::QueueRequest(req.into())
			}
			v4::ToServerBody::WorkflowHistoryRequest(req) => {
				v3::ToServerBody::WorkflowHistoryRequest(req.into())
			}
			v4::ToServerBody::WorkflowReplayRequest(_) => {
				bail!("cannot convert inspector v4 workflow replay requests to v3")
			}
			v4::ToServerBody::DatabaseSchemaRequest(req) => {
				v3::ToServerBody::DatabaseSchemaRequest(req.into())
			}
			v4::ToServerBody::DatabaseTableRowsRequest(req) => {
				v3::ToServerBody::DatabaseTableRowsRequest(req.into())
			}
		};

		Ok(Self::V3(v3::ToServer { body }))
	}

	fn v3_to_v2(self) -> Result<Self> {
		let Self::V3(data) = self else {
			bail!("expected inspector protocol v3 ToServer")
		};

		let body = match data.body {
			v3::ToServerBody::PatchStateRequest(req) => {
				v2::ToServerBody::PatchStateRequest(req.into())
			}
			v3::ToServerBody::StateRequest(req) => {
				v2::ToServerBody::StateRequest(req.into())
			}
			v3::ToServerBody::ConnectionsRequest(req) => {
				v2::ToServerBody::ConnectionsRequest(req.into())
			}
			v3::ToServerBody::ActionRequest(req) => {
				v2::ToServerBody::ActionRequest(req.into())
			}
			v3::ToServerBody::RpcsListRequest(req) => {
				v2::ToServerBody::RpcsListRequest(req.into())
			}
			v3::ToServerBody::TraceQueryRequest(req) => {
				v2::ToServerBody::TraceQueryRequest(req.into())
			}
			v3::ToServerBody::QueueRequest(req) => {
				v2::ToServerBody::QueueRequest(req.into())
			}
			v3::ToServerBody::WorkflowHistoryRequest(req) => {
				v2::ToServerBody::WorkflowHistoryRequest(req.into())
			}
			v3::ToServerBody::DatabaseSchemaRequest(_)
			| v3::ToServerBody::DatabaseTableRowsRequest(_) => {
				bail!("cannot convert inspector v3 database requests to v2")
			}
		};

		Ok(Self::V2(v2::ToServer { body }))
	}

	fn v2_to_v1(self) -> Result<Self> {
		let Self::V2(data) = self else {
			bail!("expected inspector protocol v2 ToServer")
		};

		let body = match data.body {
			v2::ToServerBody::PatchStateRequest(req) => {
				v1::ToServerBody::PatchStateRequest(req.into())
			}
			v2::ToServerBody::StateRequest(req) => {
				v1::ToServerBody::StateRequest(req.into())
			}
			v2::ToServerBody::ConnectionsRequest(req) => {
				v1::ToServerBody::ConnectionsRequest(req.into())
			}
			v2::ToServerBody::ActionRequest(req) => {
				v1::ToServerBody::ActionRequest(req.into())
			}
			v2::ToServerBody::RpcsListRequest(req) => {
				v1::ToServerBody::RpcsListRequest(req.into())
			}
			v2::ToServerBody::TraceQueryRequest(_)
			| v2::ToServerBody::QueueRequest(_)
			| v2::ToServerBody::WorkflowHistoryRequest(_) => {
				bail!("cannot convert inspector v2 queue/trace/workflow requests to v1")
			}
		};

		Ok(Self::V1(v1::ToServer { body }))
	}
}

pub enum ToClient {
	V1(v1::ToClient),
	V2(v2::ToClient),
	V3(v3::ToClient),
	V4(v4::ToClient),
}

impl OwnedVersionedData for ToClient {
	type Latest = v4::ToClient;

	fn wrap_latest(latest: Self::Latest) -> Self {
		Self::V4(latest)
	}

	fn unwrap_latest(self) -> Result<Self::Latest> {
		match self {
			Self::V4(data) => Ok(data),
			_ => bail!("version not latest"),
		}
	}

	fn deserialize_version(payload: &[u8], version: u16) -> Result<Self> {
		match version {
			1 => Ok(Self::V1(serde_bare::from_slice(payload)?)),
			2 => Ok(Self::V2(serde_bare::from_slice(payload)?)),
			3 => Ok(Self::V3(serde_bare::from_slice(payload)?)),
			4 => Ok(Self::V4(serde_bare::from_slice(payload)?)),
			_ => bail!("invalid inspector protocol version for ToClient: {version}"),
		}
	}

	fn serialize_version(self, version: u16) -> Result<Vec<u8>> {
		match (self, version) {
			(Self::V1(data), 1) => serde_bare::to_vec(&data).map_err(Into::into),
			(Self::V2(data), 2) => serde_bare::to_vec(&data).map_err(Into::into),
			(Self::V3(data), 3) => serde_bare::to_vec(&data).map_err(Into::into),
			(Self::V4(data), 4) => serde_bare::to_vec(&data).map_err(Into::into),
			(_, version) => bail!("unexpected inspector protocol version for ToClient: {version}"),
		}
	}

	fn deserialize_converters() -> Vec<impl Fn(Self) -> Result<Self>> {
		vec![Self::v1_to_v2, Self::v2_to_v3, Self::v3_to_v4]
	}

	fn serialize_converters() -> Vec<impl Fn(Self) -> Result<Self>> {
		vec![Self::v4_to_v3, Self::v3_to_v2, Self::v2_to_v1]
	}
}

impl ToClient {
	fn v1_to_v2(self) -> Result<Self> {
		let Self::V1(data) = self else {
			bail!("expected inspector protocol v1 ToClient")
		};

		let body = match data.body {
			v1::ToClientBody::StateResponse(resp) => {
				v2::ToClientBody::StateResponse(resp.into())
			}
			v1::ToClientBody::ConnectionsResponse(resp) => {
				v2::ToClientBody::ConnectionsResponse(resp.into())
			}
			v1::ToClientBody::ActionResponse(resp) => {
				v2::ToClientBody::ActionResponse(resp.into())
			}
			v1::ToClientBody::RpcsListResponse(resp) => {
				v2::ToClientBody::RpcsListResponse(resp.into())
			}
			v1::ToClientBody::ConnectionsUpdated(update) => {
				v2::ToClientBody::ConnectionsUpdated(update.into())
			}
			v1::ToClientBody::StateUpdated(update) => {
				v2::ToClientBody::StateUpdated(update.into())
			}
			v1::ToClientBody::Error(error) => v2::ToClientBody::Error(error.into()),
			v1::ToClientBody::Init(init) => v2::ToClientBody::Init(v2::Init {
				connections: convert_vec(init.connections),
				state: init.state,
				is_state_enabled: init.is_state_enabled,
				rpcs: init.rpcs,
				is_database_enabled: init.is_database_enabled,
				queue_size: Uint(0),
				workflow_history: None,
				is_workflow_enabled: false,
			}),
			v1::ToClientBody::EventsResponse(_) | v1::ToClientBody::EventsUpdated(_) => {
				bail!("cannot convert inspector v1 events responses to v2")
			}
		};

		Ok(Self::V2(v2::ToClient { body }))
	}

	fn v2_to_v3(self) -> Result<Self> {
		let Self::V2(data) = self else {
			bail!("expected inspector protocol v2 ToClient")
		};
		Ok(Self::V3(data.into()))
	}

	fn v3_to_v4(self) -> Result<Self> {
		let Self::V3(data) = self else {
			bail!("expected inspector protocol v3 ToClient")
		};

		let body = match data.body {
			v3::ToClientBody::StateResponse(resp) => {
				v4::ToClientBody::StateResponse(resp.into())
			}
			v3::ToClientBody::ConnectionsResponse(resp) => {
				v4::ToClientBody::ConnectionsResponse(resp.into())
			}
			v3::ToClientBody::ActionResponse(resp) => {
				v4::ToClientBody::ActionResponse(resp.into())
			}
			v3::ToClientBody::ConnectionsUpdated(update) => {
				v4::ToClientBody::ConnectionsUpdated(update.into())
			}
			v3::ToClientBody::QueueUpdated(update) => {
				v4::ToClientBody::QueueUpdated(update.into())
			}
			v3::ToClientBody::StateUpdated(update) => {
				v4::ToClientBody::StateUpdated(update.into())
			}
			v3::ToClientBody::WorkflowHistoryUpdated(update) => {
				v4::ToClientBody::WorkflowHistoryUpdated(update.into())
			}
			v3::ToClientBody::RpcsListResponse(resp) => {
				v4::ToClientBody::RpcsListResponse(resp.into())
			}
			v3::ToClientBody::TraceQueryResponse(resp) => {
				v4::ToClientBody::TraceQueryResponse(resp.into())
			}
			v3::ToClientBody::QueueResponse(resp) => {
				v4::ToClientBody::QueueResponse(resp.into())
			}
			v3::ToClientBody::WorkflowHistoryResponse(resp) => {
				v4::ToClientBody::WorkflowHistoryResponse(resp.into())
			}
			v3::ToClientBody::Error(error) => v4::ToClientBody::Error(error.into()),
			v3::ToClientBody::Init(init) => v4::ToClientBody::Init(init.into()),
			v3::ToClientBody::DatabaseSchemaResponse(resp) => {
				v4::ToClientBody::DatabaseSchemaResponse(resp.into())
			}
			v3::ToClientBody::DatabaseTableRowsResponse(resp) => {
				v4::ToClientBody::DatabaseTableRowsResponse(resp.into())
			}
		};

		Ok(Self::V4(v4::ToClient { body }))
	}

	fn v4_to_v3(self) -> Result<Self> {
		let Self::V4(data) = self else {
			bail!("expected inspector protocol v4 ToClient")
		};

		let body = match data.body {
			v4::ToClientBody::StateResponse(resp) => {
				v3::ToClientBody::StateResponse(resp.into())
			}
			v4::ToClientBody::ConnectionsResponse(resp) => {
				v3::ToClientBody::ConnectionsResponse(resp.into())
			}
			v4::ToClientBody::ActionResponse(resp) => {
				v3::ToClientBody::ActionResponse(resp.into())
			}
			v4::ToClientBody::ConnectionsUpdated(update) => {
				v3::ToClientBody::ConnectionsUpdated(update.into())
			}
			v4::ToClientBody::QueueUpdated(update) => {
				v3::ToClientBody::QueueUpdated(update.into())
			}
			v4::ToClientBody::StateUpdated(update) => {
				v3::ToClientBody::StateUpdated(update.into())
			}
			v4::ToClientBody::WorkflowHistoryUpdated(update) => {
				v3::ToClientBody::WorkflowHistoryUpdated(update.into())
			}
			v4::ToClientBody::RpcsListResponse(resp) => {
				v3::ToClientBody::RpcsListResponse(resp.into())
			}
			v4::ToClientBody::TraceQueryResponse(resp) => {
				v3::ToClientBody::TraceQueryResponse(resp.into())
			}
			v4::ToClientBody::QueueResponse(resp) => {
				v3::ToClientBody::QueueResponse(resp.into())
			}
			v4::ToClientBody::WorkflowHistoryResponse(resp) => {
				v3::ToClientBody::WorkflowHistoryResponse(resp.into())
			}
			v4::ToClientBody::WorkflowReplayResponse(_) => {
				v3::ToClientBody::Error(dropped_error(WORKFLOW_HISTORY_DROPPED_ERROR).into())
			}
			v4::ToClientBody::Error(error) => v3::ToClientBody::Error(error.into()),
			v4::ToClientBody::Init(init) => v3::ToClientBody::Init(init.into()),
			v4::ToClientBody::DatabaseSchemaResponse(resp) => {
				v3::ToClientBody::DatabaseSchemaResponse(resp.into())
			}
			v4::ToClientBody::DatabaseTableRowsResponse(resp) => {
				v3::ToClientBody::DatabaseTableRowsResponse(resp.into())
			}
		};

		Ok(Self::V3(v3::ToClient { body }))
	}

	fn v3_to_v2(self) -> Result<Self> {
		let Self::V3(data) = self else {
			bail!("expected inspector protocol v3 ToClient")
		};

		let body = match data.body {
			v3::ToClientBody::StateResponse(resp) => {
				v2::ToClientBody::StateResponse(resp.into())
			}
			v3::ToClientBody::ConnectionsResponse(resp) => {
				v2::ToClientBody::ConnectionsResponse(resp.into())
			}
			v3::ToClientBody::ActionResponse(resp) => {
				v2::ToClientBody::ActionResponse(resp.into())
			}
			v3::ToClientBody::ConnectionsUpdated(update) => {
				v2::ToClientBody::ConnectionsUpdated(update.into())
			}
			v3::ToClientBody::QueueUpdated(update) => {
				v2::ToClientBody::QueueUpdated(update.into())
			}
			v3::ToClientBody::StateUpdated(update) => {
				v2::ToClientBody::StateUpdated(update.into())
			}
			v3::ToClientBody::WorkflowHistoryUpdated(update) => {
				v2::ToClientBody::WorkflowHistoryUpdated(update.into())
			}
			v3::ToClientBody::RpcsListResponse(resp) => {
				v2::ToClientBody::RpcsListResponse(resp.into())
			}
			v3::ToClientBody::TraceQueryResponse(resp) => {
				v2::ToClientBody::TraceQueryResponse(resp.into())
			}
			v3::ToClientBody::QueueResponse(resp) => {
				v2::ToClientBody::QueueResponse(resp.into())
			}
			v3::ToClientBody::WorkflowHistoryResponse(resp) => {
				v2::ToClientBody::WorkflowHistoryResponse(resp.into())
			}
			v3::ToClientBody::Error(error) => v2::ToClientBody::Error(error.into()),
			v3::ToClientBody::Init(init) => v2::ToClientBody::Init(init.into()),
			v3::ToClientBody::DatabaseSchemaResponse(_)
			| v3::ToClientBody::DatabaseTableRowsResponse(_) => {
				v2::ToClientBody::Error(dropped_error(DATABASE_DROPPED_ERROR))
			}
		};

		Ok(Self::V2(v2::ToClient { body }))
	}

	fn v2_to_v1(self) -> Result<Self> {
		let Self::V2(data) = self else {
			bail!("expected inspector protocol v2 ToClient")
		};

		let body = match data.body {
			v2::ToClientBody::StateResponse(resp) => {
				v1::ToClientBody::StateResponse(resp.into())
			}
			v2::ToClientBody::ConnectionsResponse(resp) => {
				v1::ToClientBody::ConnectionsResponse(resp.into())
			}
			v2::ToClientBody::ActionResponse(resp) => {
				v1::ToClientBody::ActionResponse(resp.into())
			}
			v2::ToClientBody::ConnectionsUpdated(update) => {
				v1::ToClientBody::ConnectionsUpdated(update.into())
			}
			v2::ToClientBody::StateUpdated(update) => {
				v1::ToClientBody::StateUpdated(update.into())
			}
			v2::ToClientBody::RpcsListResponse(resp) => {
				v1::ToClientBody::RpcsListResponse(resp.into())
			}
			v2::ToClientBody::Error(error) => v1::ToClientBody::Error(error.into()),
			v2::ToClientBody::Init(init) => v1::ToClientBody::Init(v1::Init {
				connections: init.connections.into_iter().map(Into::into).collect(),
				events: Vec::new(),
				state: init.state,
				is_state_enabled: init.is_state_enabled,
				rpcs: init.rpcs,
				is_database_enabled: init.is_database_enabled,
			}),
			v2::ToClientBody::QueueUpdated(_) | v2::ToClientBody::QueueResponse(_) => {
				v1::ToClientBody::Error(dropped_error(QUEUE_DROPPED_ERROR).into())
			}
			v2::ToClientBody::WorkflowHistoryUpdated(_)
			| v2::ToClientBody::WorkflowHistoryResponse(_) => {
				v1::ToClientBody::Error(dropped_error(WORKFLOW_HISTORY_DROPPED_ERROR).into())
			}
			v2::ToClientBody::TraceQueryResponse(_) => {
				v1::ToClientBody::Error(dropped_error(TRACE_DROPPED_ERROR).into())
			}
		};

		Ok(Self::V1(v1::ToClient { body }))
	}
}

fn convert_vec<From, To>(values: Vec<From>) -> Vec<To>
where
	From: Into<To>,
{
	values.into_iter().map(Into::into).collect()
}

macro_rules! impl_same_fields_pair {
	($left:ident, $right:ident, $ty:ident { $($field:ident),+ $(,)? }) => {
		impl From<$left::$ty> for $right::$ty {
			fn from(value: $left::$ty) -> Self {
				Self {
					$($field: value.$field),+
				}
			}
		}

		impl From<$right::$ty> for $left::$ty {
			fn from(value: $right::$ty) -> Self {
				Self {
					$($field: value.$field),+
				}
			}
		}
	};
}

macro_rules! impl_connection_list_pair {
	($left:ident, $right:ident, $ty:ident) => {
		impl From<$left::$ty> for $right::$ty {
			fn from(value: $left::$ty) -> Self {
				Self {
					connections: convert_vec(value.connections),
				}
			}
		}

		impl From<$right::$ty> for $left::$ty {
			fn from(value: $right::$ty) -> Self {
				Self {
					connections: convert_vec(value.connections),
				}
			}
		}
	};
}

macro_rules! impl_connections_response_pair {
	($left:ident, $right:ident) => {
		impl From<$left::ConnectionsResponse> for $right::ConnectionsResponse {
			fn from(value: $left::ConnectionsResponse) -> Self {
				Self {
					rid: value.rid,
					connections: convert_vec(value.connections),
				}
			}
		}

		impl From<$right::ConnectionsResponse> for $left::ConnectionsResponse {
			fn from(value: $right::ConnectionsResponse) -> Self {
				Self {
					rid: value.rid,
					connections: convert_vec(value.connections),
				}
			}
		}
	};
}

macro_rules! impl_queue_status_pair {
	($left:ident, $right:ident) => {
		impl From<$left::QueueStatus> for $right::QueueStatus {
			fn from(value: $left::QueueStatus) -> Self {
				Self {
					size: value.size,
					max_size: value.max_size,
					messages: convert_vec(value.messages),
					truncated: value.truncated,
				}
			}
		}

		impl From<$right::QueueStatus> for $left::QueueStatus {
			fn from(value: $right::QueueStatus) -> Self {
				Self {
					size: value.size,
					max_size: value.max_size,
					messages: convert_vec(value.messages),
					truncated: value.truncated,
				}
			}
		}
	};
}

macro_rules! impl_queue_response_pair {
	($left:ident, $right:ident) => {
		impl From<$left::QueueResponse> for $right::QueueResponse {
			fn from(value: $left::QueueResponse) -> Self {
				Self {
					rid: value.rid,
					status: value.status.into(),
				}
			}
		}

		impl From<$right::QueueResponse> for $left::QueueResponse {
			fn from(value: $right::QueueResponse) -> Self {
				Self {
					rid: value.rid,
					status: value.status.into(),
				}
			}
		}
	};
}

macro_rules! impl_init_pair {
	($left:ident, $right:ident) => {
		impl From<$left::Init> for $right::Init {
			fn from(value: $left::Init) -> Self {
				Self {
					connections: convert_vec(value.connections),
					state: value.state,
					is_state_enabled: value.is_state_enabled,
					rpcs: value.rpcs,
					is_database_enabled: value.is_database_enabled,
					queue_size: value.queue_size,
					workflow_history: value.workflow_history,
					is_workflow_enabled: value.is_workflow_enabled,
				}
			}
		}

		impl From<$right::Init> for $left::Init {
			fn from(value: $right::Init) -> Self {
				Self {
					connections: convert_vec(value.connections),
					state: value.state,
					is_state_enabled: value.is_state_enabled,
					rpcs: value.rpcs,
					is_database_enabled: value.is_database_enabled,
					queue_size: value.queue_size,
					workflow_history: value.workflow_history,
					is_workflow_enabled: value.is_workflow_enabled,
				}
			}
		}
	};
}

macro_rules! impl_common_actor_pair {
	($left:ident, $right:ident) => {
		impl_same_fields_pair!($left, $right, PatchStateRequest { state });
		impl_same_fields_pair!($left, $right, ActionRequest { id, name, args });
		impl_same_fields_pair!($left, $right, StateRequest { id });
		impl_same_fields_pair!($left, $right, ConnectionsRequest { id });
		impl_same_fields_pair!($left, $right, RpcsListRequest { id });
		impl_same_fields_pair!($left, $right, Connection { id, details });
		impl_connections_response_pair!($left, $right);
		impl_connection_list_pair!($left, $right, ConnectionsUpdated);
		impl_same_fields_pair!($left, $right, StateResponse {
			rid,
			state,
			is_state_enabled,
		});
		impl_same_fields_pair!($left, $right, ActionResponse { rid, output });
		impl_same_fields_pair!($left, $right, StateUpdated { state });
		impl_same_fields_pair!($left, $right, RpcsListResponse { rid, rpcs });
		impl_same_fields_pair!($left, $right, Error { message });
	};
}

macro_rules! impl_queue_workflow_pair {
	($left:ident, $right:ident) => {
		impl_same_fields_pair!($left, $right, TraceQueryRequest {
			id,
			start_ms,
			end_ms,
			limit,
		});
		impl_same_fields_pair!($left, $right, TraceQueryResponse { rid, payload });
		impl_same_fields_pair!($left, $right, QueueRequest { id, limit });
		impl_same_fields_pair!($left, $right, QueueMessageSummary {
			id,
			name,
			created_at_ms,
		});
		impl_queue_status_pair!($left, $right);
		impl_queue_response_pair!($left, $right);
		impl_same_fields_pair!($left, $right, QueueUpdated { queue_size });
		impl_same_fields_pair!($left, $right, WorkflowHistoryRequest { id });
		impl_same_fields_pair!($left, $right, WorkflowHistoryResponse {
			rid,
			history,
			is_workflow_enabled,
		});
		impl_same_fields_pair!($left, $right, WorkflowHistoryUpdated { history });
		impl_init_pair!($left, $right);
	};
}

macro_rules! impl_database_pair {
	($left:ident, $right:ident) => {
		impl_same_fields_pair!($left, $right, DatabaseSchemaRequest { id });
		impl_same_fields_pair!($left, $right, DatabaseSchemaResponse { rid, schema });
		impl_same_fields_pair!($left, $right, DatabaseTableRowsRequest {
			id,
			table,
			limit,
			offset,
		});
		impl_same_fields_pair!($left, $right, DatabaseTableRowsResponse { rid, result });
	};
}

impl_common_actor_pair!(v1, v2);
impl_common_actor_pair!(v2, v3);
impl_common_actor_pair!(v3, v4);
impl_queue_workflow_pair!(v2, v3);
impl_queue_workflow_pair!(v3, v4);
impl_database_pair!(v3, v4);

impl From<v2::ToServerBody> for v3::ToServerBody {
	fn from(value: v2::ToServerBody) -> Self {
		match value {
			v2::ToServerBody::PatchStateRequest(req) => Self::PatchStateRequest(req.into()),
			v2::ToServerBody::StateRequest(req) => Self::StateRequest(req.into()),
			v2::ToServerBody::ConnectionsRequest(req) => Self::ConnectionsRequest(req.into()),
			v2::ToServerBody::ActionRequest(req) => Self::ActionRequest(req.into()),
			v2::ToServerBody::RpcsListRequest(req) => Self::RpcsListRequest(req.into()),
			v2::ToServerBody::TraceQueryRequest(req) => Self::TraceQueryRequest(req.into()),
			v2::ToServerBody::QueueRequest(req) => Self::QueueRequest(req.into()),
			v2::ToServerBody::WorkflowHistoryRequest(req) => {
				Self::WorkflowHistoryRequest(req.into())
			}
		}
	}
}

impl From<v2::ToServer> for v3::ToServer {
	fn from(value: v2::ToServer) -> Self {
		Self {
			body: value.body.into(),
		}
	}
}

impl From<v2::ToClientBody> for v3::ToClientBody {
	fn from(value: v2::ToClientBody) -> Self {
		match value {
			v2::ToClientBody::StateResponse(resp) => Self::StateResponse(resp.into()),
			v2::ToClientBody::ConnectionsResponse(resp) => Self::ConnectionsResponse(resp.into()),
			v2::ToClientBody::ActionResponse(resp) => Self::ActionResponse(resp.into()),
			v2::ToClientBody::ConnectionsUpdated(update) => {
				Self::ConnectionsUpdated(update.into())
			}
			v2::ToClientBody::QueueUpdated(update) => Self::QueueUpdated(update.into()),
			v2::ToClientBody::StateUpdated(update) => Self::StateUpdated(update.into()),
			v2::ToClientBody::WorkflowHistoryUpdated(update) => {
				Self::WorkflowHistoryUpdated(update.into())
			}
			v2::ToClientBody::RpcsListResponse(resp) => Self::RpcsListResponse(resp.into()),
			v2::ToClientBody::TraceQueryResponse(resp) => Self::TraceQueryResponse(resp.into()),
			v2::ToClientBody::QueueResponse(resp) => Self::QueueResponse(resp.into()),
			v2::ToClientBody::WorkflowHistoryResponse(resp) => {
				Self::WorkflowHistoryResponse(resp.into())
			}
			v2::ToClientBody::Error(error) => Self::Error(error.into()),
			v2::ToClientBody::Init(init) => Self::Init(init.into()),
		}
	}
}

impl From<v2::ToClient> for v3::ToClient {
	fn from(value: v2::ToClient) -> Self {
		Self {
			body: value.body.into(),
		}
	}
}

fn dropped_error(message: &str) -> v2::Error {
	v2::Error {
		message: message.to_owned(),
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn v3_database_schema_request_keeps_meaning_when_upgrading_to_v4() {
		let request = ToServer::V3(v3::ToServer {
			body: v3::ToServerBody::DatabaseSchemaRequest(v3::DatabaseSchemaRequest {
				id: Uint(7),
			}),
		});

		let ToServer::V4(upgraded) = ToServer::v3_to_v4(request).unwrap() else {
			panic!("expected v4 request")
		};

		assert!(matches!(
			upgraded.body,
			v4::ToServerBody::DatabaseSchemaRequest(v4::DatabaseSchemaRequest { id }) if id == Uint(7)
		));
	}

	#[test]
	fn v4_workflow_replay_response_downgrades_to_v3_error() {
		let response = ToClient::V4(v4::ToClient {
			body: v4::ToClientBody::WorkflowReplayResponse(v4::WorkflowReplayResponse {
				rid: Uint(11),
				history: Some(b"workflow".to_vec()),
				is_workflow_enabled: true,
			}),
		});

		let ToClient::V3(downgraded) = ToClient::v4_to_v3(response).unwrap() else {
			panic!("expected v3 response")
		};

		assert_eq!(
			downgraded.body,
			v3::ToClientBody::Error(v3::Error {
				message: WORKFLOW_HISTORY_DROPPED_ERROR.to_owned(),
			})
		);
	}
}
