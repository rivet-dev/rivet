use anyhow::{Result, bail};
use serde::{Serialize, de::DeserializeOwned};
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
				v2::ToServerBody::PatchStateRequest(transcode_version(req)?)
			}
			v1::ToServerBody::StateRequest(req) => {
				v2::ToServerBody::StateRequest(transcode_version(req)?)
			}
			v1::ToServerBody::ConnectionsRequest(req) => {
				v2::ToServerBody::ConnectionsRequest(transcode_version(req)?)
			}
			v1::ToServerBody::ActionRequest(req) => {
				v2::ToServerBody::ActionRequest(transcode_version(req)?)
			}
			v1::ToServerBody::RpcsListRequest(req) => {
				v2::ToServerBody::RpcsListRequest(transcode_version(req)?)
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
		Ok(Self::V3(transcode_version(data)?))
	}

	fn v3_to_v4(self) -> Result<Self> {
		let Self::V3(data) = self else {
			bail!("expected inspector protocol v3 ToServer")
		};

		let body = match data.body {
			v3::ToServerBody::PatchStateRequest(req) => {
				v4::ToServerBody::PatchStateRequest(transcode_version(req)?)
			}
			v3::ToServerBody::StateRequest(req) => {
				v4::ToServerBody::StateRequest(transcode_version(req)?)
			}
			v3::ToServerBody::ConnectionsRequest(req) => {
				v4::ToServerBody::ConnectionsRequest(transcode_version(req)?)
			}
			v3::ToServerBody::ActionRequest(req) => {
				v4::ToServerBody::ActionRequest(transcode_version(req)?)
			}
			v3::ToServerBody::RpcsListRequest(req) => {
				v4::ToServerBody::RpcsListRequest(transcode_version(req)?)
			}
			v3::ToServerBody::TraceQueryRequest(req) => {
				v4::ToServerBody::TraceQueryRequest(transcode_version(req)?)
			}
			v3::ToServerBody::QueueRequest(req) => {
				v4::ToServerBody::QueueRequest(transcode_version(req)?)
			}
			v3::ToServerBody::WorkflowHistoryRequest(req) => {
				v4::ToServerBody::WorkflowHistoryRequest(transcode_version(req)?)
			}
			v3::ToServerBody::DatabaseSchemaRequest(req) => {
				v4::ToServerBody::DatabaseSchemaRequest(transcode_version(req)?)
			}
			v3::ToServerBody::DatabaseTableRowsRequest(req) => {
				v4::ToServerBody::DatabaseTableRowsRequest(transcode_version(req)?)
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
				v3::ToServerBody::PatchStateRequest(transcode_version(req)?)
			}
			v4::ToServerBody::StateRequest(req) => {
				v3::ToServerBody::StateRequest(transcode_version(req)?)
			}
			v4::ToServerBody::ConnectionsRequest(req) => {
				v3::ToServerBody::ConnectionsRequest(transcode_version(req)?)
			}
			v4::ToServerBody::ActionRequest(req) => {
				v3::ToServerBody::ActionRequest(transcode_version(req)?)
			}
			v4::ToServerBody::RpcsListRequest(req) => {
				v3::ToServerBody::RpcsListRequest(transcode_version(req)?)
			}
			v4::ToServerBody::TraceQueryRequest(req) => {
				v3::ToServerBody::TraceQueryRequest(transcode_version(req)?)
			}
			v4::ToServerBody::QueueRequest(req) => {
				v3::ToServerBody::QueueRequest(transcode_version(req)?)
			}
			v4::ToServerBody::WorkflowHistoryRequest(req) => {
				v3::ToServerBody::WorkflowHistoryRequest(transcode_version(req)?)
			}
			v4::ToServerBody::WorkflowReplayRequest(_) => {
				bail!("cannot convert inspector v4 workflow replay requests to v3")
			}
			v4::ToServerBody::DatabaseSchemaRequest(req) => {
				v3::ToServerBody::DatabaseSchemaRequest(transcode_version(req)?)
			}
			v4::ToServerBody::DatabaseTableRowsRequest(req) => {
				v3::ToServerBody::DatabaseTableRowsRequest(transcode_version(req)?)
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
				v2::ToServerBody::PatchStateRequest(transcode_version(req)?)
			}
			v3::ToServerBody::StateRequest(req) => {
				v2::ToServerBody::StateRequest(transcode_version(req)?)
			}
			v3::ToServerBody::ConnectionsRequest(req) => {
				v2::ToServerBody::ConnectionsRequest(transcode_version(req)?)
			}
			v3::ToServerBody::ActionRequest(req) => {
				v2::ToServerBody::ActionRequest(transcode_version(req)?)
			}
			v3::ToServerBody::RpcsListRequest(req) => {
				v2::ToServerBody::RpcsListRequest(transcode_version(req)?)
			}
			v3::ToServerBody::TraceQueryRequest(req) => {
				v2::ToServerBody::TraceQueryRequest(transcode_version(req)?)
			}
			v3::ToServerBody::QueueRequest(req) => {
				v2::ToServerBody::QueueRequest(transcode_version(req)?)
			}
			v3::ToServerBody::WorkflowHistoryRequest(req) => {
				v2::ToServerBody::WorkflowHistoryRequest(transcode_version(req)?)
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
				v1::ToServerBody::PatchStateRequest(transcode_version(req)?)
			}
			v2::ToServerBody::StateRequest(req) => {
				v1::ToServerBody::StateRequest(transcode_version(req)?)
			}
			v2::ToServerBody::ConnectionsRequest(req) => {
				v1::ToServerBody::ConnectionsRequest(transcode_version(req)?)
			}
			v2::ToServerBody::ActionRequest(req) => {
				v1::ToServerBody::ActionRequest(transcode_version(req)?)
			}
			v2::ToServerBody::RpcsListRequest(req) => {
				v1::ToServerBody::RpcsListRequest(transcode_version(req)?)
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
				v2::ToClientBody::StateResponse(transcode_version(resp)?)
			}
			v1::ToClientBody::ConnectionsResponse(resp) => {
				v2::ToClientBody::ConnectionsResponse(transcode_version(resp)?)
			}
			v1::ToClientBody::ActionResponse(resp) => {
				v2::ToClientBody::ActionResponse(transcode_version(resp)?)
			}
			v1::ToClientBody::RpcsListResponse(resp) => {
				v2::ToClientBody::RpcsListResponse(transcode_version(resp)?)
			}
			v1::ToClientBody::ConnectionsUpdated(update) => {
				v2::ToClientBody::ConnectionsUpdated(transcode_version(update)?)
			}
			v1::ToClientBody::StateUpdated(update) => {
				v2::ToClientBody::StateUpdated(transcode_version(update)?)
			}
			v1::ToClientBody::Error(error) => v2::ToClientBody::Error(transcode_version(error)?),
			v1::ToClientBody::Init(init) => v2::ToClientBody::Init(v2::Init {
				connections: transcode_version(init.connections)?,
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
		Ok(Self::V3(transcode_version(data)?))
	}

	fn v3_to_v4(self) -> Result<Self> {
		let Self::V3(data) = self else {
			bail!("expected inspector protocol v3 ToClient")
		};

		let body = match data.body {
			v3::ToClientBody::StateResponse(resp) => {
				v4::ToClientBody::StateResponse(transcode_version(resp)?)
			}
			v3::ToClientBody::ConnectionsResponse(resp) => {
				v4::ToClientBody::ConnectionsResponse(transcode_version(resp)?)
			}
			v3::ToClientBody::ActionResponse(resp) => {
				v4::ToClientBody::ActionResponse(transcode_version(resp)?)
			}
			v3::ToClientBody::ConnectionsUpdated(update) => {
				v4::ToClientBody::ConnectionsUpdated(transcode_version(update)?)
			}
			v3::ToClientBody::QueueUpdated(update) => {
				v4::ToClientBody::QueueUpdated(transcode_version(update)?)
			}
			v3::ToClientBody::StateUpdated(update) => {
				v4::ToClientBody::StateUpdated(transcode_version(update)?)
			}
			v3::ToClientBody::WorkflowHistoryUpdated(update) => {
				v4::ToClientBody::WorkflowHistoryUpdated(transcode_version(update)?)
			}
			v3::ToClientBody::RpcsListResponse(resp) => {
				v4::ToClientBody::RpcsListResponse(transcode_version(resp)?)
			}
			v3::ToClientBody::TraceQueryResponse(resp) => {
				v4::ToClientBody::TraceQueryResponse(transcode_version(resp)?)
			}
			v3::ToClientBody::QueueResponse(resp) => {
				v4::ToClientBody::QueueResponse(transcode_version(resp)?)
			}
			v3::ToClientBody::WorkflowHistoryResponse(resp) => {
				v4::ToClientBody::WorkflowHistoryResponse(transcode_version(resp)?)
			}
			v3::ToClientBody::Error(error) => v4::ToClientBody::Error(transcode_version(error)?),
			v3::ToClientBody::Init(init) => v4::ToClientBody::Init(transcode_version(init)?),
			v3::ToClientBody::DatabaseSchemaResponse(resp) => {
				v4::ToClientBody::DatabaseSchemaResponse(transcode_version(resp)?)
			}
			v3::ToClientBody::DatabaseTableRowsResponse(resp) => {
				v4::ToClientBody::DatabaseTableRowsResponse(transcode_version(resp)?)
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
				v3::ToClientBody::StateResponse(transcode_version(resp)?)
			}
			v4::ToClientBody::ConnectionsResponse(resp) => {
				v3::ToClientBody::ConnectionsResponse(transcode_version(resp)?)
			}
			v4::ToClientBody::ActionResponse(resp) => {
				v3::ToClientBody::ActionResponse(transcode_version(resp)?)
			}
			v4::ToClientBody::ConnectionsUpdated(update) => {
				v3::ToClientBody::ConnectionsUpdated(transcode_version(update)?)
			}
			v4::ToClientBody::QueueUpdated(update) => {
				v3::ToClientBody::QueueUpdated(transcode_version(update)?)
			}
			v4::ToClientBody::StateUpdated(update) => {
				v3::ToClientBody::StateUpdated(transcode_version(update)?)
			}
			v4::ToClientBody::WorkflowHistoryUpdated(update) => {
				v3::ToClientBody::WorkflowHistoryUpdated(transcode_version(update)?)
			}
			v4::ToClientBody::RpcsListResponse(resp) => {
				v3::ToClientBody::RpcsListResponse(transcode_version(resp)?)
			}
			v4::ToClientBody::TraceQueryResponse(resp) => {
				v3::ToClientBody::TraceQueryResponse(transcode_version(resp)?)
			}
			v4::ToClientBody::QueueResponse(resp) => {
				v3::ToClientBody::QueueResponse(transcode_version(resp)?)
			}
			v4::ToClientBody::WorkflowHistoryResponse(resp) => {
				v3::ToClientBody::WorkflowHistoryResponse(transcode_version(resp)?)
			}
			v4::ToClientBody::WorkflowReplayResponse(_) => v3::ToClientBody::Error(
				transcode_version(dropped_error(WORKFLOW_HISTORY_DROPPED_ERROR))?,
			),
			v4::ToClientBody::Error(error) => v3::ToClientBody::Error(transcode_version(error)?),
			v4::ToClientBody::Init(init) => v3::ToClientBody::Init(transcode_version(init)?),
			v4::ToClientBody::DatabaseSchemaResponse(resp) => {
				v3::ToClientBody::DatabaseSchemaResponse(transcode_version(resp)?)
			}
			v4::ToClientBody::DatabaseTableRowsResponse(resp) => {
				v3::ToClientBody::DatabaseTableRowsResponse(transcode_version(resp)?)
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
				v2::ToClientBody::StateResponse(transcode_version(resp)?)
			}
			v3::ToClientBody::ConnectionsResponse(resp) => {
				v2::ToClientBody::ConnectionsResponse(transcode_version(resp)?)
			}
			v3::ToClientBody::ActionResponse(resp) => {
				v2::ToClientBody::ActionResponse(transcode_version(resp)?)
			}
			v3::ToClientBody::ConnectionsUpdated(update) => {
				v2::ToClientBody::ConnectionsUpdated(transcode_version(update)?)
			}
			v3::ToClientBody::QueueUpdated(update) => {
				v2::ToClientBody::QueueUpdated(transcode_version(update)?)
			}
			v3::ToClientBody::StateUpdated(update) => {
				v2::ToClientBody::StateUpdated(transcode_version(update)?)
			}
			v3::ToClientBody::WorkflowHistoryUpdated(update) => {
				v2::ToClientBody::WorkflowHistoryUpdated(transcode_version(update)?)
			}
			v3::ToClientBody::RpcsListResponse(resp) => {
				v2::ToClientBody::RpcsListResponse(transcode_version(resp)?)
			}
			v3::ToClientBody::TraceQueryResponse(resp) => {
				v2::ToClientBody::TraceQueryResponse(transcode_version(resp)?)
			}
			v3::ToClientBody::QueueResponse(resp) => {
				v2::ToClientBody::QueueResponse(transcode_version(resp)?)
			}
			v3::ToClientBody::WorkflowHistoryResponse(resp) => {
				v2::ToClientBody::WorkflowHistoryResponse(transcode_version(resp)?)
			}
			v3::ToClientBody::Error(error) => v2::ToClientBody::Error(transcode_version(error)?),
			v3::ToClientBody::Init(init) => v2::ToClientBody::Init(transcode_version(init)?),
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
				v1::ToClientBody::StateResponse(transcode_version(resp)?)
			}
			v2::ToClientBody::ConnectionsResponse(resp) => {
				v1::ToClientBody::ConnectionsResponse(transcode_version(resp)?)
			}
			v2::ToClientBody::ActionResponse(resp) => {
				v1::ToClientBody::ActionResponse(transcode_version(resp)?)
			}
			v2::ToClientBody::ConnectionsUpdated(update) => {
				v1::ToClientBody::ConnectionsUpdated(transcode_version(update)?)
			}
			v2::ToClientBody::StateUpdated(update) => {
				v1::ToClientBody::StateUpdated(transcode_version(update)?)
			}
			v2::ToClientBody::RpcsListResponse(resp) => {
				v1::ToClientBody::RpcsListResponse(transcode_version(resp)?)
			}
			v2::ToClientBody::Error(error) => v1::ToClientBody::Error(transcode_version(error)?),
			v2::ToClientBody::Init(init) => v1::ToClientBody::Init(v1::Init {
				connections: transcode_version(init.connections)?,
				events: Vec::new(),
				state: init.state,
				is_state_enabled: init.is_state_enabled,
				rpcs: init.rpcs,
				is_database_enabled: init.is_database_enabled,
			}),
			v2::ToClientBody::QueueUpdated(_) | v2::ToClientBody::QueueResponse(_) => {
				v1::ToClientBody::Error(transcode_version(dropped_error(QUEUE_DROPPED_ERROR))?)
			}
			v2::ToClientBody::WorkflowHistoryUpdated(_)
			| v2::ToClientBody::WorkflowHistoryResponse(_) => v1::ToClientBody::Error(transcode_version(
				dropped_error(WORKFLOW_HISTORY_DROPPED_ERROR),
			)?),
			v2::ToClientBody::TraceQueryResponse(_) => {
				v1::ToClientBody::Error(transcode_version(dropped_error(TRACE_DROPPED_ERROR))?)
			}
		};

		Ok(Self::V1(v1::ToClient { body }))
	}
}

fn dropped_error(message: &str) -> v2::Error {
	v2::Error {
		message: message.to_owned(),
	}
}

fn transcode_version<From, To>(data: From) -> Result<To>
where
	From: Serialize,
	To: DeserializeOwned,
{
	let encoded = serde_bare::to_vec(&data)?;
	serde_bare::from_slice(&encoded).map_err(Into::into)
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
