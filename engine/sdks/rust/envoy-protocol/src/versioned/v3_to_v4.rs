// @generated initial scaffold by scripts/vbare-gen-converters
// from: v3.bare, to: v4.bare
// Replace each todo!() with the migration semantics, then drop the @generated marker.

#![allow(dead_code, unused_variables)]

use anyhow::Result;

use crate::generated::{v3, v4};

pub fn convert_kv_metadata_v3_to_v4(x: v3::KvMetadata) -> Result<v4::KvMetadata> {
	Ok(v4::KvMetadata {
		version: x.version,
		update_ts: x.update_ts,
	})
}

pub fn convert_kv_list_range_query_v3_to_v4(
	x: v3::KvListRangeQuery,
) -> Result<v4::KvListRangeQuery> {
	Ok(v4::KvListRangeQuery {
		start: x.start,
		end: x.end,
		exclusive: x.exclusive,
	})
}

pub fn convert_kv_list_prefix_query_v3_to_v4(
	x: v3::KvListPrefixQuery,
) -> Result<v4::KvListPrefixQuery> {
	Ok(v4::KvListPrefixQuery { key: x.key })
}

pub fn convert_kv_list_query_v3_to_v4(x: v3::KvListQuery) -> Result<v4::KvListQuery> {
	Ok(match x {
		v3::KvListQuery::KvListAllQuery => v4::KvListQuery::KvListAllQuery,
		v3::KvListQuery::KvListRangeQuery(v) => {
			v4::KvListQuery::KvListRangeQuery(convert_kv_list_range_query_v3_to_v4(v)?)
		}
		v3::KvListQuery::KvListPrefixQuery(v) => {
			v4::KvListQuery::KvListPrefixQuery(convert_kv_list_prefix_query_v3_to_v4(v)?)
		}
	})
}

pub fn convert_kv_get_request_v3_to_v4(x: v3::KvGetRequest) -> Result<v4::KvGetRequest> {
	Ok(v4::KvGetRequest { keys: x.keys })
}

pub fn convert_kv_list_request_v3_to_v4(x: v3::KvListRequest) -> Result<v4::KvListRequest> {
	Ok(v4::KvListRequest {
		query: convert_kv_list_query_v3_to_v4(x.query)?,
		reverse: x.reverse,
		limit: x.limit,
	})
}

pub fn convert_kv_put_request_v3_to_v4(x: v3::KvPutRequest) -> Result<v4::KvPutRequest> {
	Ok(v4::KvPutRequest {
		keys: x.keys,
		values: x.values,
	})
}

pub fn convert_kv_delete_request_v3_to_v4(x: v3::KvDeleteRequest) -> Result<v4::KvDeleteRequest> {
	Ok(v4::KvDeleteRequest { keys: x.keys })
}

pub fn convert_kv_delete_range_request_v3_to_v4(
	x: v3::KvDeleteRangeRequest,
) -> Result<v4::KvDeleteRangeRequest> {
	Ok(v4::KvDeleteRangeRequest {
		start: x.start,
		end: x.end,
	})
}

pub fn convert_kv_error_response_v3_to_v4(x: v3::KvErrorResponse) -> Result<v4::KvErrorResponse> {
	Ok(v4::KvErrorResponse { message: x.message })
}

pub fn convert_kv_get_response_v3_to_v4(x: v3::KvGetResponse) -> Result<v4::KvGetResponse> {
	Ok(v4::KvGetResponse {
		keys: x.keys,
		values: x.values,
		metadata: x
			.metadata
			.into_iter()
			.map(|v| convert_kv_metadata_v3_to_v4(v))
			.collect::<Result<Vec<_>>>()?,
	})
}

pub fn convert_kv_list_response_v3_to_v4(x: v3::KvListResponse) -> Result<v4::KvListResponse> {
	Ok(v4::KvListResponse {
		keys: x.keys,
		values: x.values,
		metadata: x
			.metadata
			.into_iter()
			.map(|v| convert_kv_metadata_v3_to_v4(v))
			.collect::<Result<Vec<_>>>()?,
	})
}

pub fn convert_kv_request_data_v3_to_v4(x: v3::KvRequestData) -> Result<v4::KvRequestData> {
	Ok(match x {
		v3::KvRequestData::KvGetRequest(v) => {
			v4::KvRequestData::KvGetRequest(convert_kv_get_request_v3_to_v4(v)?)
		}
		v3::KvRequestData::KvListRequest(v) => {
			v4::KvRequestData::KvListRequest(convert_kv_list_request_v3_to_v4(v)?)
		}
		v3::KvRequestData::KvPutRequest(v) => {
			v4::KvRequestData::KvPutRequest(convert_kv_put_request_v3_to_v4(v)?)
		}
		v3::KvRequestData::KvDeleteRequest(v) => {
			v4::KvRequestData::KvDeleteRequest(convert_kv_delete_request_v3_to_v4(v)?)
		}
		v3::KvRequestData::KvDeleteRangeRequest(v) => {
			v4::KvRequestData::KvDeleteRangeRequest(convert_kv_delete_range_request_v3_to_v4(v)?)
		}
		v3::KvRequestData::KvDropRequest => v4::KvRequestData::KvDropRequest,
	})
}

pub fn convert_kv_response_data_v3_to_v4(x: v3::KvResponseData) -> Result<v4::KvResponseData> {
	Ok(match x {
		v3::KvResponseData::KvErrorResponse(v) => {
			v4::KvResponseData::KvErrorResponse(convert_kv_error_response_v3_to_v4(v)?)
		}
		v3::KvResponseData::KvGetResponse(v) => {
			v4::KvResponseData::KvGetResponse(convert_kv_get_response_v3_to_v4(v)?)
		}
		v3::KvResponseData::KvListResponse(v) => {
			v4::KvResponseData::KvListResponse(convert_kv_list_response_v3_to_v4(v)?)
		}
		v3::KvResponseData::KvPutResponse => v4::KvResponseData::KvPutResponse,
		v3::KvResponseData::KvDeleteResponse => v4::KvResponseData::KvDeleteResponse,
		v3::KvResponseData::KvDropResponse => v4::KvResponseData::KvDropResponse,
	})
}

pub fn convert_sqlite_dirty_page_v3_to_v4(x: v3::SqliteDirtyPage) -> Result<v4::SqliteDirtyPage> {
	Ok(v4::SqliteDirtyPage {
		pgno: x.pgno,
		bytes: x.bytes,
	})
}

pub fn convert_sqlite_fetched_page_v3_to_v4(
	x: v3::SqliteFetchedPage,
) -> Result<v4::SqliteFetchedPage> {
	Ok(v4::SqliteFetchedPage {
		pgno: x.pgno,
		bytes: x.bytes,
	})
}

pub fn convert_sqlite_get_pages_request_v3_to_v4(
	x: v3::SqliteGetPagesRequest,
) -> Result<v4::SqliteGetPagesRequest> {
	Ok(v4::SqliteGetPagesRequest {
		actor_id: x.actor_id,
		pgnos: x.pgnos,
		expected_generation: x.expected_generation,
		expected_head_txid: x.expected_head_txid,
	})
}

pub fn convert_sqlite_get_pages_ok_v3_to_v4(
	x: v3::SqliteGetPagesOk,
) -> Result<v4::SqliteGetPagesOk> {
	Ok(v4::SqliteGetPagesOk {
		pages: x
			.pages
			.into_iter()
			.map(|v| convert_sqlite_fetched_page_v3_to_v4(v))
			.collect::<Result<Vec<_>>>()?,
	})
}

pub fn convert_sqlite_error_response_v3_to_v4(
	x: v3::SqliteErrorResponse,
) -> Result<v4::SqliteErrorResponse> {
	Ok(v4::SqliteErrorResponse { message: x.message })
}

pub fn convert_sqlite_get_pages_response_v3_to_v4(
	x: v3::SqliteGetPagesResponse,
) -> Result<v4::SqliteGetPagesResponse> {
	Ok(match x {
		v3::SqliteGetPagesResponse::SqliteGetPagesOk(v) => {
			v4::SqliteGetPagesResponse::SqliteGetPagesOk(convert_sqlite_get_pages_ok_v3_to_v4(v)?)
		}
		v3::SqliteGetPagesResponse::SqliteErrorResponse(v) => {
			v4::SqliteGetPagesResponse::SqliteErrorResponse(convert_sqlite_error_response_v3_to_v4(
				v,
			)?)
		}
	})
}

pub fn convert_sqlite_commit_request_v3_to_v4(
	x: v3::SqliteCommitRequest,
) -> Result<v4::SqliteCommitRequest> {
	Ok(v4::SqliteCommitRequest {
		actor_id: x.actor_id,
		dirty_pages: x
			.dirty_pages
			.into_iter()
			.map(|v| convert_sqlite_dirty_page_v3_to_v4(v))
			.collect::<Result<Vec<_>>>()?,
		db_size_pages: x.db_size_pages,
		now_ms: x.now_ms,
		expected_generation: x.expected_generation,
		expected_head_txid: x.expected_head_txid,
	})
}

pub fn convert_sqlite_commit_response_v3_to_v4(
	x: v3::SqliteCommitResponse,
) -> Result<v4::SqliteCommitResponse> {
	Ok(match x {
		v3::SqliteCommitResponse::SqliteCommitOk => v4::SqliteCommitResponse::SqliteCommitOk,
		v3::SqliteCommitResponse::SqliteErrorResponse(v) => {
			v4::SqliteCommitResponse::SqliteErrorResponse(convert_sqlite_error_response_v3_to_v4(
				v,
			)?)
		}
	})
}

pub fn convert_stop_code_v3_to_v4(x: v3::StopCode) -> Result<v4::StopCode> {
	Ok(match x {
		v3::StopCode::Ok => v4::StopCode::Ok,
		v3::StopCode::Error => v4::StopCode::Error,
	})
}

pub fn convert_actor_name_v3_to_v4(x: v3::ActorName) -> Result<v4::ActorName> {
	Ok(v4::ActorName {
		metadata: x.metadata,
	})
}

pub fn convert_actor_config_v3_to_v4(x: v3::ActorConfig) -> Result<v4::ActorConfig> {
	Ok(v4::ActorConfig {
		name: x.name,
		key: x.key,
		create_ts: x.create_ts,
		input: x.input,
	})
}

pub fn convert_actor_checkpoint_v3_to_v4(x: v3::ActorCheckpoint) -> Result<v4::ActorCheckpoint> {
	Ok(v4::ActorCheckpoint {
		actor_id: x.actor_id,
		generation: x.generation,
		index: x.index,
	})
}

pub fn convert_actor_intent_v3_to_v4(x: v3::ActorIntent) -> Result<v4::ActorIntent> {
	Ok(match x {
		v3::ActorIntent::ActorIntentSleep => v4::ActorIntent::ActorIntentSleep,
		v3::ActorIntent::ActorIntentStop => v4::ActorIntent::ActorIntentStop,
	})
}

pub fn convert_actor_state_stopped_v3_to_v4(
	x: v3::ActorStateStopped,
) -> Result<v4::ActorStateStopped> {
	Ok(v4::ActorStateStopped {
		code: convert_stop_code_v3_to_v4(x.code)?,
		message: x.message,
	})
}

pub fn convert_actor_state_v3_to_v4(x: v3::ActorState) -> Result<v4::ActorState> {
	Ok(match x {
		v3::ActorState::ActorStateRunning => v4::ActorState::ActorStateRunning,
		v3::ActorState::ActorStateStopped(v) => {
			v4::ActorState::ActorStateStopped(convert_actor_state_stopped_v3_to_v4(v)?)
		}
	})
}

pub fn convert_event_actor_intent_v3_to_v4(
	x: v3::EventActorIntent,
) -> Result<v4::EventActorIntent> {
	Ok(v4::EventActorIntent {
		intent: convert_actor_intent_v3_to_v4(x.intent)?,
	})
}

pub fn convert_event_actor_state_update_v3_to_v4(
	x: v3::EventActorStateUpdate,
) -> Result<v4::EventActorStateUpdate> {
	Ok(v4::EventActorStateUpdate {
		state: convert_actor_state_v3_to_v4(x.state)?,
	})
}

pub fn convert_event_actor_set_alarm_v3_to_v4(
	x: v3::EventActorSetAlarm,
) -> Result<v4::EventActorSetAlarm> {
	Ok(v4::EventActorSetAlarm {
		alarm_ts: x.alarm_ts,
	})
}

pub fn convert_event_v3_to_v4(x: v3::Event) -> Result<v4::Event> {
	Ok(match x {
		v3::Event::EventActorIntent(v) => {
			v4::Event::EventActorIntent(convert_event_actor_intent_v3_to_v4(v)?)
		}
		v3::Event::EventActorStateUpdate(v) => {
			v4::Event::EventActorStateUpdate(convert_event_actor_state_update_v3_to_v4(v)?)
		}
		v3::Event::EventActorSetAlarm(v) => {
			v4::Event::EventActorSetAlarm(convert_event_actor_set_alarm_v3_to_v4(v)?)
		}
	})
}

pub fn convert_event_wrapper_v3_to_v4(x: v3::EventWrapper) -> Result<v4::EventWrapper> {
	Ok(v4::EventWrapper {
		checkpoint: convert_actor_checkpoint_v3_to_v4(x.checkpoint)?,
		inner: convert_event_v3_to_v4(x.inner)?,
	})
}

pub fn convert_preloaded_kv_entry_v3_to_v4(
	x: v3::PreloadedKvEntry,
) -> Result<v4::PreloadedKvEntry> {
	Ok(v4::PreloadedKvEntry {
		key: x.key,
		value: x.value,
		metadata: convert_kv_metadata_v3_to_v4(x.metadata)?,
	})
}

pub fn convert_preloaded_kv_v3_to_v4(x: v3::PreloadedKv) -> Result<v4::PreloadedKv> {
	Ok(v4::PreloadedKv {
		entries: x
			.entries
			.into_iter()
			.map(|v| convert_preloaded_kv_entry_v3_to_v4(v))
			.collect::<Result<Vec<_>>>()?,
		requested_get_keys: x.requested_get_keys,
		requested_prefixes: x.requested_prefixes,
	})
}

pub fn convert_hibernating_request_v3_to_v4(
	x: v3::HibernatingRequest,
) -> Result<v4::HibernatingRequest> {
	Ok(v4::HibernatingRequest {
		gateway_id: x.gateway_id,
		request_id: x.request_id,
	})
}

pub fn convert_command_start_actor_v3_to_v4(
	x: v3::CommandStartActor,
) -> Result<v4::CommandStartActor> {
	Ok(v4::CommandStartActor {
		config: convert_actor_config_v3_to_v4(x.config)?,
		hibernating_requests: x
			.hibernating_requests
			.into_iter()
			.map(|v| convert_hibernating_request_v3_to_v4(v))
			.collect::<Result<Vec<_>>>()?,
		preloaded_kv: x
			.preloaded_kv
			.map(|v| convert_preloaded_kv_v3_to_v4(v))
			.transpose()?,
	})
}

pub fn convert_stop_actor_reason_v3_to_v4(x: v3::StopActorReason) -> Result<v4::StopActorReason> {
	Ok(match x {
		v3::StopActorReason::SleepIntent => v4::StopActorReason::SleepIntent,
		v3::StopActorReason::StopIntent => v4::StopActorReason::StopIntent,
		v3::StopActorReason::Destroy => v4::StopActorReason::Destroy,
		v3::StopActorReason::GoingAway => v4::StopActorReason::GoingAway,
		v3::StopActorReason::Lost => v4::StopActorReason::Lost,
	})
}

pub fn convert_command_stop_actor_v3_to_v4(
	x: v3::CommandStopActor,
) -> Result<v4::CommandStopActor> {
	Ok(v4::CommandStopActor {
		reason: convert_stop_actor_reason_v3_to_v4(x.reason)?,
	})
}

pub fn convert_command_v3_to_v4(x: v3::Command) -> Result<v4::Command> {
	Ok(match x {
		v3::Command::CommandStartActor(v) => {
			v4::Command::CommandStartActor(convert_command_start_actor_v3_to_v4(v)?)
		}
		v3::Command::CommandStopActor(v) => {
			v4::Command::CommandStopActor(convert_command_stop_actor_v3_to_v4(v)?)
		}
	})
}

pub fn convert_command_wrapper_v3_to_v4(x: v3::CommandWrapper) -> Result<v4::CommandWrapper> {
	Ok(v4::CommandWrapper {
		checkpoint: convert_actor_checkpoint_v3_to_v4(x.checkpoint)?,
		inner: convert_command_v3_to_v4(x.inner)?,
	})
}

pub fn convert_actor_command_key_data_v3_to_v4(
	x: v3::ActorCommandKeyData,
) -> Result<v4::ActorCommandKeyData> {
	Ok(match x {
		v3::ActorCommandKeyData::CommandStartActor(v) => {
			v4::ActorCommandKeyData::CommandStartActor(convert_command_start_actor_v3_to_v4(v)?)
		}
		v3::ActorCommandKeyData::CommandStopActor(v) => {
			v4::ActorCommandKeyData::CommandStopActor(convert_command_stop_actor_v3_to_v4(v)?)
		}
	})
}

pub fn convert_message_id_v3_to_v4(x: v3::MessageId) -> Result<v4::MessageId> {
	Ok(v4::MessageId {
		gateway_id: x.gateway_id,
		request_id: x.request_id,
		message_index: x.message_index,
	})
}

pub fn convert_to_envoy_request_start_v3_to_v4(
	x: v3::ToEnvoyRequestStart,
) -> Result<v4::ToEnvoyRequestStart> {
	Ok(v4::ToEnvoyRequestStart {
		actor_id: x.actor_id,
		method: x.method,
		path: x.path,
		headers: x.headers,
		body: x.body,
		stream: x.stream,
	})
}

pub fn convert_to_envoy_request_chunk_v3_to_v4(
	x: v3::ToEnvoyRequestChunk,
) -> Result<v4::ToEnvoyRequestChunk> {
	Ok(v4::ToEnvoyRequestChunk {
		body: x.body,
		finish: x.finish,
	})
}

pub fn convert_to_rivet_response_start_v3_to_v4(
	x: v3::ToRivetResponseStart,
) -> Result<v4::ToRivetResponseStart> {
	Ok(v4::ToRivetResponseStart {
		status: x.status,
		headers: x.headers,
		body: x.body,
		stream: x.stream,
	})
}

pub fn convert_to_rivet_response_chunk_v3_to_v4(
	x: v3::ToRivetResponseChunk,
) -> Result<v4::ToRivetResponseChunk> {
	Ok(v4::ToRivetResponseChunk {
		body: x.body,
		finish: x.finish,
	})
}

pub fn convert_to_envoy_web_socket_open_v3_to_v4(
	x: v3::ToEnvoyWebSocketOpen,
) -> Result<v4::ToEnvoyWebSocketOpen> {
	Ok(v4::ToEnvoyWebSocketOpen {
		actor_id: x.actor_id,
		path: x.path,
		headers: x.headers,
	})
}

pub fn convert_to_envoy_web_socket_message_v3_to_v4(
	x: v3::ToEnvoyWebSocketMessage,
) -> Result<v4::ToEnvoyWebSocketMessage> {
	Ok(v4::ToEnvoyWebSocketMessage {
		data: x.data,
		binary: x.binary,
	})
}

pub fn convert_to_envoy_web_socket_close_v3_to_v4(
	x: v3::ToEnvoyWebSocketClose,
) -> Result<v4::ToEnvoyWebSocketClose> {
	Ok(v4::ToEnvoyWebSocketClose {
		code: x.code,
		reason: x.reason,
	})
}

pub fn convert_to_rivet_web_socket_open_v3_to_v4(
	x: v3::ToRivetWebSocketOpen,
) -> Result<v4::ToRivetWebSocketOpen> {
	Ok(v4::ToRivetWebSocketOpen {
		can_hibernate: x.can_hibernate,
	})
}

pub fn convert_to_rivet_web_socket_message_v3_to_v4(
	x: v3::ToRivetWebSocketMessage,
) -> Result<v4::ToRivetWebSocketMessage> {
	Ok(v4::ToRivetWebSocketMessage {
		data: x.data,
		binary: x.binary,
	})
}

pub fn convert_to_rivet_web_socket_message_ack_v3_to_v4(
	x: v3::ToRivetWebSocketMessageAck,
) -> Result<v4::ToRivetWebSocketMessageAck> {
	Ok(v4::ToRivetWebSocketMessageAck { index: x.index })
}

pub fn convert_to_rivet_web_socket_close_v3_to_v4(
	x: v3::ToRivetWebSocketClose,
) -> Result<v4::ToRivetWebSocketClose> {
	Ok(v4::ToRivetWebSocketClose {
		code: x.code,
		reason: x.reason,
		hibernate: x.hibernate,
	})
}

pub fn convert_to_rivet_tunnel_message_kind_v3_to_v4(
	x: v3::ToRivetTunnelMessageKind,
) -> Result<v4::ToRivetTunnelMessageKind> {
	Ok(match x {
		v3::ToRivetTunnelMessageKind::ToRivetResponseStart(v) => {
			v4::ToRivetTunnelMessageKind::ToRivetResponseStart(
				convert_to_rivet_response_start_v3_to_v4(v)?,
			)
		}
		v3::ToRivetTunnelMessageKind::ToRivetResponseChunk(v) => {
			v4::ToRivetTunnelMessageKind::ToRivetResponseChunk(
				convert_to_rivet_response_chunk_v3_to_v4(v)?,
			)
		}
		v3::ToRivetTunnelMessageKind::ToRivetResponseAbort => {
			v4::ToRivetTunnelMessageKind::ToRivetResponseAbort
		}
		v3::ToRivetTunnelMessageKind::ToRivetWebSocketOpen(v) => {
			v4::ToRivetTunnelMessageKind::ToRivetWebSocketOpen(
				convert_to_rivet_web_socket_open_v3_to_v4(v)?,
			)
		}
		v3::ToRivetTunnelMessageKind::ToRivetWebSocketMessage(v) => {
			v4::ToRivetTunnelMessageKind::ToRivetWebSocketMessage(
				convert_to_rivet_web_socket_message_v3_to_v4(v)?,
			)
		}
		v3::ToRivetTunnelMessageKind::ToRivetWebSocketMessageAck(v) => {
			v4::ToRivetTunnelMessageKind::ToRivetWebSocketMessageAck(
				convert_to_rivet_web_socket_message_ack_v3_to_v4(v)?,
			)
		}
		v3::ToRivetTunnelMessageKind::ToRivetWebSocketClose(v) => {
			v4::ToRivetTunnelMessageKind::ToRivetWebSocketClose(
				convert_to_rivet_web_socket_close_v3_to_v4(v)?,
			)
		}
	})
}

pub fn convert_to_rivet_tunnel_message_v3_to_v4(
	x: v3::ToRivetTunnelMessage,
) -> Result<v4::ToRivetTunnelMessage> {
	Ok(v4::ToRivetTunnelMessage {
		message_id: convert_message_id_v3_to_v4(x.message_id)?,
		message_kind: convert_to_rivet_tunnel_message_kind_v3_to_v4(x.message_kind)?,
	})
}

pub fn convert_to_envoy_tunnel_message_kind_v3_to_v4(
	x: v3::ToEnvoyTunnelMessageKind,
) -> Result<v4::ToEnvoyTunnelMessageKind> {
	Ok(match x {
		v3::ToEnvoyTunnelMessageKind::ToEnvoyRequestStart(v) => {
			v4::ToEnvoyTunnelMessageKind::ToEnvoyRequestStart(
				convert_to_envoy_request_start_v3_to_v4(v)?,
			)
		}
		v3::ToEnvoyTunnelMessageKind::ToEnvoyRequestChunk(v) => {
			v4::ToEnvoyTunnelMessageKind::ToEnvoyRequestChunk(
				convert_to_envoy_request_chunk_v3_to_v4(v)?,
			)
		}
		v3::ToEnvoyTunnelMessageKind::ToEnvoyRequestAbort => {
			v4::ToEnvoyTunnelMessageKind::ToEnvoyRequestAbort
		}
		v3::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketOpen(v) => {
			v4::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketOpen(
				convert_to_envoy_web_socket_open_v3_to_v4(v)?,
			)
		}
		v3::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketMessage(v) => {
			v4::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketMessage(
				convert_to_envoy_web_socket_message_v3_to_v4(v)?,
			)
		}
		v3::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketClose(v) => {
			v4::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketClose(
				convert_to_envoy_web_socket_close_v3_to_v4(v)?,
			)
		}
	})
}

pub fn convert_to_envoy_tunnel_message_v3_to_v4(
	x: v3::ToEnvoyTunnelMessage,
) -> Result<v4::ToEnvoyTunnelMessage> {
	Ok(v4::ToEnvoyTunnelMessage {
		message_id: convert_message_id_v3_to_v4(x.message_id)?,
		message_kind: convert_to_envoy_tunnel_message_kind_v3_to_v4(x.message_kind)?,
	})
}

pub fn convert_to_envoy_ping_v3_to_v4(x: v3::ToEnvoyPing) -> Result<v4::ToEnvoyPing> {
	Ok(v4::ToEnvoyPing { ts: x.ts })
}

pub fn convert_to_rivet_metadata_v3_to_v4(x: v3::ToRivetMetadata) -> Result<v4::ToRivetMetadata> {
	Ok(v4::ToRivetMetadata {
		prepopulate_actor_names: x
			.prepopulate_actor_names
			.map(|v| {
				v.into_iter()
					.map(|(k, v)| -> Result<_> { Ok((k, convert_actor_name_v3_to_v4(v)?)) })
					.collect::<Result<_>>()
			})
			.transpose()?,
		metadata: x.metadata,
	})
}

pub fn convert_to_rivet_events_v3_to_v4(x: v3::ToRivetEvents) -> Result<v4::ToRivetEvents> {
	Ok(x.into_iter()
		.map(|v| convert_event_wrapper_v3_to_v4(v))
		.collect::<Result<Vec<_>>>()?)
}

pub fn convert_to_rivet_ack_commands_v3_to_v4(
	x: v3::ToRivetAckCommands,
) -> Result<v4::ToRivetAckCommands> {
	Ok(v4::ToRivetAckCommands {
		last_command_checkpoints: x
			.last_command_checkpoints
			.into_iter()
			.map(|v| convert_actor_checkpoint_v3_to_v4(v))
			.collect::<Result<Vec<_>>>()?,
	})
}

pub fn convert_to_rivet_pong_v3_to_v4(x: v3::ToRivetPong) -> Result<v4::ToRivetPong> {
	Ok(v4::ToRivetPong { ts: x.ts })
}

pub fn convert_to_rivet_kv_request_v3_to_v4(
	x: v3::ToRivetKvRequest,
) -> Result<v4::ToRivetKvRequest> {
	Ok(v4::ToRivetKvRequest {
		actor_id: x.actor_id,
		request_id: x.request_id,
		data: convert_kv_request_data_v3_to_v4(x.data)?,
	})
}

pub fn convert_to_rivet_sqlite_get_pages_request_v3_to_v4(
	x: v3::ToRivetSqliteGetPagesRequest,
) -> Result<v4::ToRivetSqliteGetPagesRequest> {
	Ok(v4::ToRivetSqliteGetPagesRequest {
		request_id: x.request_id,
		data: convert_sqlite_get_pages_request_v3_to_v4(x.data)?,
	})
}

pub fn convert_to_rivet_sqlite_commit_request_v3_to_v4(
	x: v3::ToRivetSqliteCommitRequest,
) -> Result<v4::ToRivetSqliteCommitRequest> {
	Ok(v4::ToRivetSqliteCommitRequest {
		request_id: x.request_id,
		data: convert_sqlite_commit_request_v3_to_v4(x.data)?,
	})
}

pub fn convert_to_rivet_v3_to_v4(x: v3::ToRivet) -> Result<v4::ToRivet> {
	Ok(match x {
		v3::ToRivet::ToRivetMetadata(v) => {
			v4::ToRivet::ToRivetMetadata(convert_to_rivet_metadata_v3_to_v4(v)?)
		}
		v3::ToRivet::ToRivetEvents(v) => {
			v4::ToRivet::ToRivetEvents(convert_to_rivet_events_v3_to_v4(v)?)
		}
		v3::ToRivet::ToRivetAckCommands(v) => {
			v4::ToRivet::ToRivetAckCommands(convert_to_rivet_ack_commands_v3_to_v4(v)?)
		}
		v3::ToRivet::ToRivetStopping => v4::ToRivet::ToRivetStopping,
		v3::ToRivet::ToRivetPong(v) => v4::ToRivet::ToRivetPong(convert_to_rivet_pong_v3_to_v4(v)?),
		v3::ToRivet::ToRivetKvRequest(v) => {
			v4::ToRivet::ToRivetKvRequest(convert_to_rivet_kv_request_v3_to_v4(v)?)
		}
		v3::ToRivet::ToRivetTunnelMessage(v) => {
			v4::ToRivet::ToRivetTunnelMessage(convert_to_rivet_tunnel_message_v3_to_v4(v)?)
		}
		v3::ToRivet::ToRivetSqliteGetPagesRequest(v) => v4::ToRivet::ToRivetSqliteGetPagesRequest(
			convert_to_rivet_sqlite_get_pages_request_v3_to_v4(v)?,
		),
		v3::ToRivet::ToRivetSqliteCommitRequest(v) => v4::ToRivet::ToRivetSqliteCommitRequest(
			convert_to_rivet_sqlite_commit_request_v3_to_v4(v)?,
		),
	})
}

pub fn convert_protocol_metadata_v3_to_v4(x: v3::ProtocolMetadata) -> Result<v4::ProtocolMetadata> {
	Ok(v4::ProtocolMetadata {
		envoy_lost_threshold: x.envoy_lost_threshold,
		actor_stop_threshold: x.actor_stop_threshold,
		max_response_payload_size: x.max_response_payload_size,
	})
}

pub fn convert_to_envoy_init_v3_to_v4(x: v3::ToEnvoyInit) -> Result<v4::ToEnvoyInit> {
	Ok(v4::ToEnvoyInit {
		metadata: convert_protocol_metadata_v3_to_v4(x.metadata)?,
	})
}

pub fn convert_to_envoy_commands_v3_to_v4(x: v3::ToEnvoyCommands) -> Result<v4::ToEnvoyCommands> {
	Ok(x.into_iter()
		.map(|v| convert_command_wrapper_v3_to_v4(v))
		.collect::<Result<Vec<_>>>()?)
}

pub fn convert_to_envoy_ack_events_v3_to_v4(
	x: v3::ToEnvoyAckEvents,
) -> Result<v4::ToEnvoyAckEvents> {
	Ok(v4::ToEnvoyAckEvents {
		last_event_checkpoints: x
			.last_event_checkpoints
			.into_iter()
			.map(|v| convert_actor_checkpoint_v3_to_v4(v))
			.collect::<Result<Vec<_>>>()?,
	})
}

pub fn convert_to_envoy_kv_response_v3_to_v4(
	x: v3::ToEnvoyKvResponse,
) -> Result<v4::ToEnvoyKvResponse> {
	Ok(v4::ToEnvoyKvResponse {
		request_id: x.request_id,
		data: convert_kv_response_data_v3_to_v4(x.data)?,
	})
}

pub fn convert_to_envoy_sqlite_get_pages_response_v3_to_v4(
	x: v3::ToEnvoySqliteGetPagesResponse,
) -> Result<v4::ToEnvoySqliteGetPagesResponse> {
	Ok(v4::ToEnvoySqliteGetPagesResponse {
		request_id: x.request_id,
		data: convert_sqlite_get_pages_response_v3_to_v4(x.data)?,
	})
}

pub fn convert_to_envoy_sqlite_commit_response_v3_to_v4(
	x: v3::ToEnvoySqliteCommitResponse,
) -> Result<v4::ToEnvoySqliteCommitResponse> {
	Ok(v4::ToEnvoySqliteCommitResponse {
		request_id: x.request_id,
		data: convert_sqlite_commit_response_v3_to_v4(x.data)?,
	})
}

pub fn convert_to_envoy_v3_to_v4(x: v3::ToEnvoy) -> Result<v4::ToEnvoy> {
	Ok(match x {
		v3::ToEnvoy::ToEnvoyInit(v) => v4::ToEnvoy::ToEnvoyInit(convert_to_envoy_init_v3_to_v4(v)?),
		v3::ToEnvoy::ToEnvoyCommands(v) => {
			v4::ToEnvoy::ToEnvoyCommands(convert_to_envoy_commands_v3_to_v4(v)?)
		}
		v3::ToEnvoy::ToEnvoyAckEvents(v) => {
			v4::ToEnvoy::ToEnvoyAckEvents(convert_to_envoy_ack_events_v3_to_v4(v)?)
		}
		v3::ToEnvoy::ToEnvoyKvResponse(v) => {
			v4::ToEnvoy::ToEnvoyKvResponse(convert_to_envoy_kv_response_v3_to_v4(v)?)
		}
		v3::ToEnvoy::ToEnvoyTunnelMessage(v) => {
			v4::ToEnvoy::ToEnvoyTunnelMessage(convert_to_envoy_tunnel_message_v3_to_v4(v)?)
		}
		v3::ToEnvoy::ToEnvoyPing(v) => v4::ToEnvoy::ToEnvoyPing(convert_to_envoy_ping_v3_to_v4(v)?),
		v3::ToEnvoy::ToEnvoySqliteGetPagesResponse(v) => {
			v4::ToEnvoy::ToEnvoySqliteGetPagesResponse(
				convert_to_envoy_sqlite_get_pages_response_v3_to_v4(v)?,
			)
		}
		v3::ToEnvoy::ToEnvoySqliteCommitResponse(v) => v4::ToEnvoy::ToEnvoySqliteCommitResponse(
			convert_to_envoy_sqlite_commit_response_v3_to_v4(v)?,
		),
	})
}

pub fn convert_to_envoy_conn_ping_v3_to_v4(x: v3::ToEnvoyConnPing) -> Result<v4::ToEnvoyConnPing> {
	Ok(v4::ToEnvoyConnPing {
		gateway_id: x.gateway_id,
		request_id: x.request_id,
		ts: x.ts,
	})
}

pub fn convert_to_envoy_conn_v3_to_v4(x: v3::ToEnvoyConn) -> Result<v4::ToEnvoyConn> {
	Ok(match x {
		v3::ToEnvoyConn::ToEnvoyConnPing(v) => {
			v4::ToEnvoyConn::ToEnvoyConnPing(convert_to_envoy_conn_ping_v3_to_v4(v)?)
		}
		v3::ToEnvoyConn::ToEnvoyConnClose => v4::ToEnvoyConn::ToEnvoyConnClose,
		v3::ToEnvoyConn::ToEnvoyCommands(v) => {
			v4::ToEnvoyConn::ToEnvoyCommands(convert_to_envoy_commands_v3_to_v4(v)?)
		}
		v3::ToEnvoyConn::ToEnvoyAckEvents(v) => {
			v4::ToEnvoyConn::ToEnvoyAckEvents(convert_to_envoy_ack_events_v3_to_v4(v)?)
		}
		v3::ToEnvoyConn::ToEnvoyTunnelMessage(v) => {
			v4::ToEnvoyConn::ToEnvoyTunnelMessage(convert_to_envoy_tunnel_message_v3_to_v4(v)?)
		}
	})
}

pub fn convert_to_gateway_pong_v3_to_v4(x: v3::ToGatewayPong) -> Result<v4::ToGatewayPong> {
	Ok(v4::ToGatewayPong {
		request_id: x.request_id,
		ts: x.ts,
	})
}

pub fn convert_to_gateway_v3_to_v4(x: v3::ToGateway) -> Result<v4::ToGateway> {
	Ok(match x {
		v3::ToGateway::ToGatewayPong(v) => {
			v4::ToGateway::ToGatewayPong(convert_to_gateway_pong_v3_to_v4(v)?)
		}
		v3::ToGateway::ToRivetTunnelMessage(v) => {
			v4::ToGateway::ToRivetTunnelMessage(convert_to_rivet_tunnel_message_v3_to_v4(v)?)
		}
	})
}

pub fn convert_to_outbound_actor_start_v3_to_v4(
	x: v3::ToOutboundActorStart,
) -> Result<v4::ToOutboundActorStart> {
	Ok(v4::ToOutboundActorStart {
		namespace_id: x.namespace_id,
		pool_name: x.pool_name,
		checkpoint: convert_actor_checkpoint_v3_to_v4(x.checkpoint)?,
		actor_config: convert_actor_config_v3_to_v4(x.actor_config)?,
	})
}

pub fn convert_to_outbound_v3_to_v4(x: v3::ToOutbound) -> Result<v4::ToOutbound> {
	Ok(match x {
		v3::ToOutbound::ToOutboundActorStart(v) => {
			v4::ToOutbound::ToOutboundActorStart(convert_to_outbound_actor_start_v3_to_v4(v)?)
		}
	})
}
