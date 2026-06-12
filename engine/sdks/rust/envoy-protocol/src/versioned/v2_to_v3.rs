// @generated initial scaffold by scripts/vbare-gen-converters
// from: v2.bare, to: v3.bare
// Replace each todo!() with the migration semantics, then drop the @generated marker.

#![allow(dead_code, unused_variables)]

use anyhow::{Result, bail};

use crate::generated::{v2, v3};

pub fn convert_kv_metadata_v2_to_v3(x: v2::KvMetadata) -> Result<v3::KvMetadata> {
	Ok(v3::KvMetadata {
		version: x.version,
		update_ts: x.update_ts,
	})
}

pub fn convert_kv_list_range_query_v2_to_v3(
	x: v2::KvListRangeQuery,
) -> Result<v3::KvListRangeQuery> {
	Ok(v3::KvListRangeQuery {
		start: x.start,
		end: x.end,
		exclusive: x.exclusive,
	})
}

pub fn convert_kv_list_prefix_query_v2_to_v3(
	x: v2::KvListPrefixQuery,
) -> Result<v3::KvListPrefixQuery> {
	Ok(v3::KvListPrefixQuery { key: x.key })
}

pub fn convert_kv_list_query_v2_to_v3(x: v2::KvListQuery) -> Result<v3::KvListQuery> {
	Ok(match x {
		v2::KvListQuery::KvListAllQuery => v3::KvListQuery::KvListAllQuery,
		v2::KvListQuery::KvListRangeQuery(v) => {
			v3::KvListQuery::KvListRangeQuery(convert_kv_list_range_query_v2_to_v3(v)?)
		}
		v2::KvListQuery::KvListPrefixQuery(v) => {
			v3::KvListQuery::KvListPrefixQuery(convert_kv_list_prefix_query_v2_to_v3(v)?)
		}
	})
}

pub fn convert_kv_get_request_v2_to_v3(x: v2::KvGetRequest) -> Result<v3::KvGetRequest> {
	Ok(v3::KvGetRequest { keys: x.keys })
}

pub fn convert_kv_list_request_v2_to_v3(x: v2::KvListRequest) -> Result<v3::KvListRequest> {
	Ok(v3::KvListRequest {
		query: convert_kv_list_query_v2_to_v3(x.query)?,
		reverse: x.reverse,
		limit: x.limit,
	})
}

pub fn convert_kv_put_request_v2_to_v3(x: v2::KvPutRequest) -> Result<v3::KvPutRequest> {
	Ok(v3::KvPutRequest {
		keys: x.keys,
		values: x.values,
	})
}

pub fn convert_kv_delete_request_v2_to_v3(x: v2::KvDeleteRequest) -> Result<v3::KvDeleteRequest> {
	Ok(v3::KvDeleteRequest { keys: x.keys })
}

pub fn convert_kv_delete_range_request_v2_to_v3(
	x: v2::KvDeleteRangeRequest,
) -> Result<v3::KvDeleteRangeRequest> {
	Ok(v3::KvDeleteRangeRequest {
		start: x.start,
		end: x.end,
	})
}

pub fn convert_kv_error_response_v2_to_v3(x: v2::KvErrorResponse) -> Result<v3::KvErrorResponse> {
	Ok(v3::KvErrorResponse { message: x.message })
}

pub fn convert_kv_get_response_v2_to_v3(x: v2::KvGetResponse) -> Result<v3::KvGetResponse> {
	Ok(v3::KvGetResponse {
		keys: x.keys,
		values: x.values,
		metadata: x
			.metadata
			.into_iter()
			.map(|v| convert_kv_metadata_v2_to_v3(v))
			.collect::<Result<Vec<_>>>()?,
	})
}

pub fn convert_kv_list_response_v2_to_v3(x: v2::KvListResponse) -> Result<v3::KvListResponse> {
	Ok(v3::KvListResponse {
		keys: x.keys,
		values: x.values,
		metadata: x
			.metadata
			.into_iter()
			.map(|v| convert_kv_metadata_v2_to_v3(v))
			.collect::<Result<Vec<_>>>()?,
	})
}

pub fn convert_kv_request_data_v2_to_v3(x: v2::KvRequestData) -> Result<v3::KvRequestData> {
	Ok(match x {
		v2::KvRequestData::KvGetRequest(v) => {
			v3::KvRequestData::KvGetRequest(convert_kv_get_request_v2_to_v3(v)?)
		}
		v2::KvRequestData::KvListRequest(v) => {
			v3::KvRequestData::KvListRequest(convert_kv_list_request_v2_to_v3(v)?)
		}
		v2::KvRequestData::KvPutRequest(v) => {
			v3::KvRequestData::KvPutRequest(convert_kv_put_request_v2_to_v3(v)?)
		}
		v2::KvRequestData::KvDeleteRequest(v) => {
			v3::KvRequestData::KvDeleteRequest(convert_kv_delete_request_v2_to_v3(v)?)
		}
		v2::KvRequestData::KvDeleteRangeRequest(v) => {
			v3::KvRequestData::KvDeleteRangeRequest(convert_kv_delete_range_request_v2_to_v3(v)?)
		}
		v2::KvRequestData::KvDropRequest => v3::KvRequestData::KvDropRequest,
	})
}

pub fn convert_kv_response_data_v2_to_v3(x: v2::KvResponseData) -> Result<v3::KvResponseData> {
	Ok(match x {
		v2::KvResponseData::KvErrorResponse(v) => {
			v3::KvResponseData::KvErrorResponse(convert_kv_error_response_v2_to_v3(v)?)
		}
		v2::KvResponseData::KvGetResponse(v) => {
			v3::KvResponseData::KvGetResponse(convert_kv_get_response_v2_to_v3(v)?)
		}
		v2::KvResponseData::KvListResponse(v) => {
			v3::KvResponseData::KvListResponse(convert_kv_list_response_v2_to_v3(v)?)
		}
		v2::KvResponseData::KvPutResponse => v3::KvResponseData::KvPutResponse,
		v2::KvResponseData::KvDeleteResponse => v3::KvResponseData::KvDeleteResponse,
		v2::KvResponseData::KvDropResponse => v3::KvResponseData::KvDropResponse,
	})
}

pub fn convert_sqlite_dirty_page_v2_to_v3(x: v2::SqliteDirtyPage) -> Result<v3::SqliteDirtyPage> {
	Ok(v3::SqliteDirtyPage {
		pgno: x.pgno,
		bytes: x.bytes,
	})
}

pub fn convert_sqlite_fetched_page_v2_to_v3(
	x: v2::SqliteFetchedPage,
) -> Result<v3::SqliteFetchedPage> {
	Ok(v3::SqliteFetchedPage {
		pgno: x.pgno,
		bytes: x.bytes,
	})
}

pub fn convert_sqlite_get_pages_request_v2_to_v3(
	x: v2::SqliteGetPagesRequest,
) -> Result<v3::SqliteGetPagesRequest> {
	Ok(v3::SqliteGetPagesRequest {
		actor_id: x.actor_id,
		pgnos: x.pgnos,
		expected_generation: todo!(),
		expected_head_txid: todo!(),
	})
}

pub fn convert_sqlite_get_pages_ok_v2_to_v3(
	x: v2::SqliteGetPagesOk,
) -> Result<v3::SqliteGetPagesOk> {
	Ok(v3::SqliteGetPagesOk {
		pages: x
			.pages
			.into_iter()
			.map(|v| convert_sqlite_fetched_page_v2_to_v3(v))
			.collect::<Result<Vec<_>>>()?,
	})
}

pub fn convert_sqlite_error_response_v2_to_v3(
	x: v2::SqliteErrorResponse,
) -> Result<v3::SqliteErrorResponse> {
	Ok(v3::SqliteErrorResponse { message: x.message })
}

pub fn convert_sqlite_get_pages_response_v2_to_v3(
	x: v2::SqliteGetPagesResponse,
) -> Result<v3::SqliteGetPagesResponse> {
	Ok(match x {
		v2::SqliteGetPagesResponse::SqliteGetPagesOk(v) => {
			v3::SqliteGetPagesResponse::SqliteGetPagesOk(convert_sqlite_get_pages_ok_v2_to_v3(v)?)
		}
		v2::SqliteGetPagesResponse::SqliteFenceMismatch(_) => todo!(),
		v2::SqliteGetPagesResponse::SqliteErrorResponse(v) => {
			v3::SqliteGetPagesResponse::SqliteErrorResponse(convert_sqlite_error_response_v2_to_v3(
				v,
			)?)
		}
	})
}

pub fn convert_sqlite_commit_request_v2_to_v3(
	x: v2::SqliteCommitRequest,
) -> Result<v3::SqliteCommitRequest> {
	Ok(v3::SqliteCommitRequest {
		actor_id: x.actor_id,
		dirty_pages: x
			.dirty_pages
			.into_iter()
			.map(|v| convert_sqlite_dirty_page_v2_to_v3(v))
			.collect::<Result<Vec<_>>>()?,
		db_size_pages: todo!(),
		now_ms: todo!(),
		expected_generation: todo!(),
		expected_head_txid: todo!(),
	})
}

pub fn convert_sqlite_commit_response_v2_to_v3(
	x: v2::SqliteCommitResponse,
) -> Result<v3::SqliteCommitResponse> {
	Ok(match x {
		v2::SqliteCommitResponse::SqliteCommitOk(_) => todo!(),
		v2::SqliteCommitResponse::SqliteFenceMismatch(_) => todo!(),
		v2::SqliteCommitResponse::SqliteCommitTooLarge(_) => todo!(),
		v2::SqliteCommitResponse::SqliteErrorResponse(v) => {
			v3::SqliteCommitResponse::SqliteErrorResponse(convert_sqlite_error_response_v2_to_v3(
				v,
			)?)
		}
	})
}

pub fn convert_stop_code_v2_to_v3(x: v2::StopCode) -> Result<v3::StopCode> {
	Ok(match x {
		v2::StopCode::Ok => v3::StopCode::Ok,
		v2::StopCode::Error => v3::StopCode::Error,
	})
}

pub fn convert_actor_name_v2_to_v3(x: v2::ActorName) -> Result<v3::ActorName> {
	Ok(v3::ActorName {
		metadata: x.metadata,
	})
}

pub fn convert_actor_config_v2_to_v3(x: v2::ActorConfig) -> Result<v3::ActorConfig> {
	Ok(v3::ActorConfig {
		name: x.name,
		key: x.key,
		create_ts: x.create_ts,
		input: x.input,
	})
}

pub fn convert_actor_checkpoint_v2_to_v3(x: v2::ActorCheckpoint) -> Result<v3::ActorCheckpoint> {
	Ok(v3::ActorCheckpoint {
		actor_id: x.actor_id,
		generation: x.generation,
		index: x.index,
	})
}

pub fn convert_actor_intent_v2_to_v3(x: v2::ActorIntent) -> Result<v3::ActorIntent> {
	Ok(match x {
		v2::ActorIntent::ActorIntentSleep => v3::ActorIntent::ActorIntentSleep,
		v2::ActorIntent::ActorIntentStop => v3::ActorIntent::ActorIntentStop,
	})
}

pub fn convert_actor_state_stopped_v2_to_v3(
	x: v2::ActorStateStopped,
) -> Result<v3::ActorStateStopped> {
	Ok(v3::ActorStateStopped {
		code: convert_stop_code_v2_to_v3(x.code)?,
		message: x.message,
	})
}

pub fn convert_actor_state_v2_to_v3(x: v2::ActorState) -> Result<v3::ActorState> {
	Ok(match x {
		v2::ActorState::ActorStateRunning => v3::ActorState::ActorStateRunning,
		v2::ActorState::ActorStateStopped(v) => {
			v3::ActorState::ActorStateStopped(convert_actor_state_stopped_v2_to_v3(v)?)
		}
	})
}

pub fn convert_event_actor_intent_v2_to_v3(
	x: v2::EventActorIntent,
) -> Result<v3::EventActorIntent> {
	Ok(v3::EventActorIntent {
		intent: convert_actor_intent_v2_to_v3(x.intent)?,
	})
}

pub fn convert_event_actor_state_update_v2_to_v3(
	x: v2::EventActorStateUpdate,
) -> Result<v3::EventActorStateUpdate> {
	Ok(v3::EventActorStateUpdate {
		state: convert_actor_state_v2_to_v3(x.state)?,
	})
}

pub fn convert_event_actor_set_alarm_v2_to_v3(
	x: v2::EventActorSetAlarm,
) -> Result<v3::EventActorSetAlarm> {
	Ok(v3::EventActorSetAlarm {
		alarm_ts: x.alarm_ts,
	})
}

pub fn convert_event_v2_to_v3(x: v2::Event) -> Result<v3::Event> {
	Ok(match x {
		v2::Event::EventActorIntent(v) => {
			v3::Event::EventActorIntent(convert_event_actor_intent_v2_to_v3(v)?)
		}
		v2::Event::EventActorStateUpdate(v) => {
			v3::Event::EventActorStateUpdate(convert_event_actor_state_update_v2_to_v3(v)?)
		}
		v2::Event::EventActorSetAlarm(v) => {
			v3::Event::EventActorSetAlarm(convert_event_actor_set_alarm_v2_to_v3(v)?)
		}
	})
}

pub fn convert_event_wrapper_v2_to_v3(x: v2::EventWrapper) -> Result<v3::EventWrapper> {
	Ok(v3::EventWrapper {
		checkpoint: convert_actor_checkpoint_v2_to_v3(x.checkpoint)?,
		inner: convert_event_v2_to_v3(x.inner)?,
	})
}

pub fn convert_preloaded_kv_entry_v2_to_v3(
	x: v2::PreloadedKvEntry,
) -> Result<v3::PreloadedKvEntry> {
	Ok(v3::PreloadedKvEntry {
		key: x.key,
		value: x.value,
		metadata: convert_kv_metadata_v2_to_v3(x.metadata)?,
	})
}

pub fn convert_preloaded_kv_v2_to_v3(x: v2::PreloadedKv) -> Result<v3::PreloadedKv> {
	Ok(v3::PreloadedKv {
		entries: x
			.entries
			.into_iter()
			.map(|v| convert_preloaded_kv_entry_v2_to_v3(v))
			.collect::<Result<Vec<_>>>()?,
		requested_get_keys: x.requested_get_keys,
		requested_prefixes: x.requested_prefixes,
	})
}

pub fn convert_hibernating_request_v2_to_v3(
	x: v2::HibernatingRequest,
) -> Result<v3::HibernatingRequest> {
	Ok(v3::HibernatingRequest {
		gateway_id: x.gateway_id,
		request_id: x.request_id,
	})
}

pub fn convert_command_start_actor_v2_to_v3(
	x: v2::CommandStartActor,
) -> Result<v3::CommandStartActor> {
	Ok(v3::CommandStartActor {
		config: convert_actor_config_v2_to_v3(x.config)?,
		hibernating_requests: x
			.hibernating_requests
			.into_iter()
			.map(|v| convert_hibernating_request_v2_to_v3(v))
			.collect::<Result<Vec<_>>>()?,
		preloaded_kv: x
			.preloaded_kv
			.map(|v| convert_preloaded_kv_v2_to_v3(v))
			.transpose()?,
	})
}

pub fn convert_stop_actor_reason_v2_to_v3(x: v2::StopActorReason) -> Result<v3::StopActorReason> {
	Ok(match x {
		v2::StopActorReason::SleepIntent => v3::StopActorReason::SleepIntent,
		v2::StopActorReason::StopIntent => v3::StopActorReason::StopIntent,
		v2::StopActorReason::Destroy => v3::StopActorReason::Destroy,
		v2::StopActorReason::GoingAway => v3::StopActorReason::GoingAway,
		v2::StopActorReason::Lost => v3::StopActorReason::Lost,
	})
}

pub fn convert_command_stop_actor_v2_to_v3(
	x: v2::CommandStopActor,
) -> Result<v3::CommandStopActor> {
	Ok(v3::CommandStopActor {
		reason: convert_stop_actor_reason_v2_to_v3(x.reason)?,
	})
}

pub fn convert_command_v2_to_v3(x: v2::Command) -> Result<v3::Command> {
	Ok(match x {
		v2::Command::CommandStartActor(v) => {
			v3::Command::CommandStartActor(convert_command_start_actor_v2_to_v3(v)?)
		}
		v2::Command::CommandStopActor(v) => {
			v3::Command::CommandStopActor(convert_command_stop_actor_v2_to_v3(v)?)
		}
	})
}

pub fn convert_command_wrapper_v2_to_v3(x: v2::CommandWrapper) -> Result<v3::CommandWrapper> {
	Ok(v3::CommandWrapper {
		checkpoint: convert_actor_checkpoint_v2_to_v3(x.checkpoint)?,
		inner: convert_command_v2_to_v3(x.inner)?,
	})
}

pub fn convert_actor_command_key_data_v2_to_v3(
	x: v2::ActorCommandKeyData,
) -> Result<v3::ActorCommandKeyData> {
	Ok(match x {
		v2::ActorCommandKeyData::CommandStartActor(v) => {
			v3::ActorCommandKeyData::CommandStartActor(convert_command_start_actor_v2_to_v3(v)?)
		}
		v2::ActorCommandKeyData::CommandStopActor(v) => {
			v3::ActorCommandKeyData::CommandStopActor(convert_command_stop_actor_v2_to_v3(v)?)
		}
	})
}

pub fn convert_message_id_v2_to_v3(x: v2::MessageId) -> Result<v3::MessageId> {
	Ok(v3::MessageId {
		gateway_id: x.gateway_id,
		request_id: x.request_id,
		message_index: x.message_index,
	})
}

pub fn convert_to_envoy_request_start_v2_to_v3(
	x: v2::ToEnvoyRequestStart,
) -> Result<v3::ToEnvoyRequestStart> {
	Ok(v3::ToEnvoyRequestStart {
		actor_id: x.actor_id,
		method: x.method,
		path: x.path,
		headers: x.headers,
		body: x.body,
		stream: x.stream,
	})
}

pub fn convert_to_envoy_request_chunk_v2_to_v3(
	x: v2::ToEnvoyRequestChunk,
) -> Result<v3::ToEnvoyRequestChunk> {
	Ok(v3::ToEnvoyRequestChunk {
		body: x.body,
		finish: x.finish,
	})
}

pub fn convert_to_rivet_response_start_v2_to_v3(
	x: v2::ToRivetResponseStart,
) -> Result<v3::ToRivetResponseStart> {
	Ok(v3::ToRivetResponseStart {
		status: x.status,
		headers: x.headers,
		body: x.body,
		stream: x.stream,
	})
}

pub fn convert_to_rivet_response_chunk_v2_to_v3(
	x: v2::ToRivetResponseChunk,
) -> Result<v3::ToRivetResponseChunk> {
	Ok(v3::ToRivetResponseChunk {
		body: x.body,
		finish: x.finish,
	})
}

pub fn convert_to_envoy_web_socket_open_v2_to_v3(
	x: v2::ToEnvoyWebSocketOpen,
) -> Result<v3::ToEnvoyWebSocketOpen> {
	Ok(v3::ToEnvoyWebSocketOpen {
		actor_id: x.actor_id,
		path: x.path,
		headers: x.headers,
	})
}

pub fn convert_to_envoy_web_socket_message_v2_to_v3(
	x: v2::ToEnvoyWebSocketMessage,
) -> Result<v3::ToEnvoyWebSocketMessage> {
	Ok(v3::ToEnvoyWebSocketMessage {
		data: x.data,
		binary: x.binary,
	})
}

pub fn convert_to_envoy_web_socket_close_v2_to_v3(
	x: v2::ToEnvoyWebSocketClose,
) -> Result<v3::ToEnvoyWebSocketClose> {
	Ok(v3::ToEnvoyWebSocketClose {
		code: x.code,
		reason: x.reason,
	})
}

pub fn convert_to_rivet_web_socket_open_v2_to_v3(
	x: v2::ToRivetWebSocketOpen,
) -> Result<v3::ToRivetWebSocketOpen> {
	Ok(v3::ToRivetWebSocketOpen {
		can_hibernate: x.can_hibernate,
	})
}

pub fn convert_to_rivet_web_socket_message_v2_to_v3(
	x: v2::ToRivetWebSocketMessage,
) -> Result<v3::ToRivetWebSocketMessage> {
	Ok(v3::ToRivetWebSocketMessage {
		data: x.data,
		binary: x.binary,
	})
}

pub fn convert_to_rivet_web_socket_message_ack_v2_to_v3(
	x: v2::ToRivetWebSocketMessageAck,
) -> Result<v3::ToRivetWebSocketMessageAck> {
	Ok(v3::ToRivetWebSocketMessageAck { index: x.index })
}

pub fn convert_to_rivet_web_socket_close_v2_to_v3(
	x: v2::ToRivetWebSocketClose,
) -> Result<v3::ToRivetWebSocketClose> {
	Ok(v3::ToRivetWebSocketClose {
		code: x.code,
		reason: x.reason,
		hibernate: x.hibernate,
	})
}

pub fn convert_to_rivet_tunnel_message_kind_v2_to_v3(
	x: v2::ToRivetTunnelMessageKind,
) -> Result<v3::ToRivetTunnelMessageKind> {
	Ok(match x {
		v2::ToRivetTunnelMessageKind::ToRivetResponseStart(v) => {
			v3::ToRivetTunnelMessageKind::ToRivetResponseStart(
				convert_to_rivet_response_start_v2_to_v3(v)?,
			)
		}
		v2::ToRivetTunnelMessageKind::ToRivetResponseChunk(v) => {
			v3::ToRivetTunnelMessageKind::ToRivetResponseChunk(
				convert_to_rivet_response_chunk_v2_to_v3(v)?,
			)
		}
		v2::ToRivetTunnelMessageKind::ToRivetResponseAbort => {
			v3::ToRivetTunnelMessageKind::ToRivetResponseAbort
		}
		v2::ToRivetTunnelMessageKind::ToRivetWebSocketOpen(v) => {
			v3::ToRivetTunnelMessageKind::ToRivetWebSocketOpen(
				convert_to_rivet_web_socket_open_v2_to_v3(v)?,
			)
		}
		v2::ToRivetTunnelMessageKind::ToRivetWebSocketMessage(v) => {
			v3::ToRivetTunnelMessageKind::ToRivetWebSocketMessage(
				convert_to_rivet_web_socket_message_v2_to_v3(v)?,
			)
		}
		v2::ToRivetTunnelMessageKind::ToRivetWebSocketMessageAck(v) => {
			v3::ToRivetTunnelMessageKind::ToRivetWebSocketMessageAck(
				convert_to_rivet_web_socket_message_ack_v2_to_v3(v)?,
			)
		}
		v2::ToRivetTunnelMessageKind::ToRivetWebSocketClose(v) => {
			v3::ToRivetTunnelMessageKind::ToRivetWebSocketClose(
				convert_to_rivet_web_socket_close_v2_to_v3(v)?,
			)
		}
	})
}

pub fn convert_to_rivet_tunnel_message_v2_to_v3(
	x: v2::ToRivetTunnelMessage,
) -> Result<v3::ToRivetTunnelMessage> {
	Ok(v3::ToRivetTunnelMessage {
		message_id: convert_message_id_v2_to_v3(x.message_id)?,
		message_kind: convert_to_rivet_tunnel_message_kind_v2_to_v3(x.message_kind)?,
	})
}

pub fn convert_to_envoy_tunnel_message_kind_v2_to_v3(
	x: v2::ToEnvoyTunnelMessageKind,
) -> Result<v3::ToEnvoyTunnelMessageKind> {
	Ok(match x {
		v2::ToEnvoyTunnelMessageKind::ToEnvoyRequestStart(v) => {
			v3::ToEnvoyTunnelMessageKind::ToEnvoyRequestStart(
				convert_to_envoy_request_start_v2_to_v3(v)?,
			)
		}
		v2::ToEnvoyTunnelMessageKind::ToEnvoyRequestChunk(v) => {
			v3::ToEnvoyTunnelMessageKind::ToEnvoyRequestChunk(
				convert_to_envoy_request_chunk_v2_to_v3(v)?,
			)
		}
		v2::ToEnvoyTunnelMessageKind::ToEnvoyRequestAbort => {
			v3::ToEnvoyTunnelMessageKind::ToEnvoyRequestAbort
		}
		v2::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketOpen(v) => {
			v3::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketOpen(
				convert_to_envoy_web_socket_open_v2_to_v3(v)?,
			)
		}
		v2::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketMessage(v) => {
			v3::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketMessage(
				convert_to_envoy_web_socket_message_v2_to_v3(v)?,
			)
		}
		v2::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketClose(v) => {
			v3::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketClose(
				convert_to_envoy_web_socket_close_v2_to_v3(v)?,
			)
		}
	})
}

pub fn convert_to_envoy_tunnel_message_v2_to_v3(
	x: v2::ToEnvoyTunnelMessage,
) -> Result<v3::ToEnvoyTunnelMessage> {
	Ok(v3::ToEnvoyTunnelMessage {
		message_id: convert_message_id_v2_to_v3(x.message_id)?,
		message_kind: convert_to_envoy_tunnel_message_kind_v2_to_v3(x.message_kind)?,
	})
}

pub fn convert_to_envoy_ping_v2_to_v3(x: v2::ToEnvoyPing) -> Result<v3::ToEnvoyPing> {
	Ok(v3::ToEnvoyPing { ts: x.ts })
}

pub fn convert_to_rivet_metadata_v2_to_v3(x: v2::ToRivetMetadata) -> Result<v3::ToRivetMetadata> {
	Ok(v3::ToRivetMetadata {
		prepopulate_actor_names: x
			.prepopulate_actor_names
			.map(|v| {
				v.into_iter()
					.map(|(k, v)| -> Result<_> { Ok((k, convert_actor_name_v2_to_v3(v)?)) })
					.collect::<Result<_>>()
			})
			.transpose()?,
		metadata: x.metadata,
	})
}

pub fn convert_to_rivet_events_v2_to_v3(x: v2::ToRivetEvents) -> Result<v3::ToRivetEvents> {
	Ok(x.into_iter()
		.map(|v| convert_event_wrapper_v2_to_v3(v))
		.collect::<Result<Vec<_>>>()?)
}

pub fn convert_to_rivet_ack_commands_v2_to_v3(
	x: v2::ToRivetAckCommands,
) -> Result<v3::ToRivetAckCommands> {
	Ok(v3::ToRivetAckCommands {
		last_command_checkpoints: x
			.last_command_checkpoints
			.into_iter()
			.map(|v| convert_actor_checkpoint_v2_to_v3(v))
			.collect::<Result<Vec<_>>>()?,
	})
}

pub fn convert_to_rivet_pong_v2_to_v3(x: v2::ToRivetPong) -> Result<v3::ToRivetPong> {
	Ok(v3::ToRivetPong { ts: x.ts })
}

pub fn convert_to_rivet_kv_request_v2_to_v3(
	x: v2::ToRivetKvRequest,
) -> Result<v3::ToRivetKvRequest> {
	Ok(v3::ToRivetKvRequest {
		actor_id: x.actor_id,
		request_id: x.request_id,
		data: convert_kv_request_data_v2_to_v3(x.data)?,
	})
}

pub fn convert_to_rivet_sqlite_get_pages_request_v2_to_v3(
	x: v2::ToRivetSqliteGetPagesRequest,
) -> Result<v3::ToRivetSqliteGetPagesRequest> {
	Ok(v3::ToRivetSqliteGetPagesRequest {
		request_id: x.request_id,
		data: convert_sqlite_get_pages_request_v2_to_v3(x.data)?,
	})
}

pub fn convert_to_rivet_sqlite_commit_request_v2_to_v3(
	x: v2::ToRivetSqliteCommitRequest,
) -> Result<v3::ToRivetSqliteCommitRequest> {
	Ok(v3::ToRivetSqliteCommitRequest {
		request_id: x.request_id,
		data: convert_sqlite_commit_request_v2_to_v3(x.data)?,
	})
}

pub fn convert_to_rivet_v2_to_v3(x: v2::ToRivet) -> Result<v3::ToRivet> {
	Ok(match x {
		v2::ToRivet::ToRivetMetadata(v) => {
			v3::ToRivet::ToRivetMetadata(convert_to_rivet_metadata_v2_to_v3(v)?)
		}
		v2::ToRivet::ToRivetEvents(v) => {
			v3::ToRivet::ToRivetEvents(convert_to_rivet_events_v2_to_v3(v)?)
		}
		v2::ToRivet::ToRivetAckCommands(v) => {
			v3::ToRivet::ToRivetAckCommands(convert_to_rivet_ack_commands_v2_to_v3(v)?)
		}
		v2::ToRivet::ToRivetStopping => v3::ToRivet::ToRivetStopping,
		v2::ToRivet::ToRivetPong(v) => v3::ToRivet::ToRivetPong(convert_to_rivet_pong_v2_to_v3(v)?),
		v2::ToRivet::ToRivetKvRequest(v) => {
			v3::ToRivet::ToRivetKvRequest(convert_to_rivet_kv_request_v2_to_v3(v)?)
		}
		v2::ToRivet::ToRivetTunnelMessage(v) => {
			v3::ToRivet::ToRivetTunnelMessage(convert_to_rivet_tunnel_message_v2_to_v3(v)?)
		}
		v2::ToRivet::ToRivetSqliteGetPagesRequest(_)
		| v2::ToRivet::ToRivetSqliteCommitRequest(_)
		| v2::ToRivet::ToRivetSqliteCommitStageBeginRequest(_)
		| v2::ToRivet::ToRivetSqliteCommitStageRequest(_)
		| v2::ToRivet::ToRivetSqliteCommitFinalizeRequest(_) => {
			bail!("legacy sqlite requests require envoy-protocol v2")
		}
	})
}

pub fn convert_protocol_metadata_v2_to_v3(x: v2::ProtocolMetadata) -> Result<v3::ProtocolMetadata> {
	Ok(v3::ProtocolMetadata {
		envoy_lost_threshold: x.envoy_lost_threshold,
		actor_stop_threshold: x.actor_stop_threshold,
		max_response_payload_size: x.max_response_payload_size,
	})
}

pub fn convert_to_envoy_init_v2_to_v3(x: v2::ToEnvoyInit) -> Result<v3::ToEnvoyInit> {
	Ok(v3::ToEnvoyInit {
		metadata: convert_protocol_metadata_v2_to_v3(x.metadata)?,
	})
}

pub fn convert_to_envoy_commands_v2_to_v3(x: v2::ToEnvoyCommands) -> Result<v3::ToEnvoyCommands> {
	Ok(x.into_iter()
		.map(|v| convert_command_wrapper_v2_to_v3(v))
		.collect::<Result<Vec<_>>>()?)
}

pub fn convert_to_envoy_ack_events_v2_to_v3(
	x: v2::ToEnvoyAckEvents,
) -> Result<v3::ToEnvoyAckEvents> {
	Ok(v3::ToEnvoyAckEvents {
		last_event_checkpoints: x
			.last_event_checkpoints
			.into_iter()
			.map(|v| convert_actor_checkpoint_v2_to_v3(v))
			.collect::<Result<Vec<_>>>()?,
	})
}

pub fn convert_to_envoy_kv_response_v2_to_v3(
	x: v2::ToEnvoyKvResponse,
) -> Result<v3::ToEnvoyKvResponse> {
	Ok(v3::ToEnvoyKvResponse {
		request_id: x.request_id,
		data: convert_kv_response_data_v2_to_v3(x.data)?,
	})
}

pub fn convert_to_envoy_sqlite_get_pages_response_v2_to_v3(
	x: v2::ToEnvoySqliteGetPagesResponse,
) -> Result<v3::ToEnvoySqliteGetPagesResponse> {
	Ok(v3::ToEnvoySqliteGetPagesResponse {
		request_id: x.request_id,
		data: convert_sqlite_get_pages_response_v2_to_v3(x.data)?,
	})
}

pub fn convert_to_envoy_sqlite_commit_response_v2_to_v3(
	x: v2::ToEnvoySqliteCommitResponse,
) -> Result<v3::ToEnvoySqliteCommitResponse> {
	Ok(v3::ToEnvoySqliteCommitResponse {
		request_id: x.request_id,
		data: convert_sqlite_commit_response_v2_to_v3(x.data)?,
	})
}

pub fn convert_to_envoy_v2_to_v3(x: v2::ToEnvoy) -> Result<v3::ToEnvoy> {
	Ok(match x {
		v2::ToEnvoy::ToEnvoyInit(v) => v3::ToEnvoy::ToEnvoyInit(convert_to_envoy_init_v2_to_v3(v)?),
		v2::ToEnvoy::ToEnvoyCommands(v) => {
			v3::ToEnvoy::ToEnvoyCommands(convert_to_envoy_commands_v2_to_v3(v)?)
		}
		v2::ToEnvoy::ToEnvoyAckEvents(v) => {
			v3::ToEnvoy::ToEnvoyAckEvents(convert_to_envoy_ack_events_v2_to_v3(v)?)
		}
		v2::ToEnvoy::ToEnvoyKvResponse(v) => {
			v3::ToEnvoy::ToEnvoyKvResponse(convert_to_envoy_kv_response_v2_to_v3(v)?)
		}
		v2::ToEnvoy::ToEnvoyTunnelMessage(v) => {
			v3::ToEnvoy::ToEnvoyTunnelMessage(convert_to_envoy_tunnel_message_v2_to_v3(v)?)
		}
		v2::ToEnvoy::ToEnvoyPing(v) => v3::ToEnvoy::ToEnvoyPing(convert_to_envoy_ping_v2_to_v3(v)?),
		v2::ToEnvoy::ToEnvoySqliteGetPagesResponse(_)
		| v2::ToEnvoy::ToEnvoySqliteCommitResponse(_)
		| v2::ToEnvoy::ToEnvoySqliteCommitStageBeginResponse(_)
		| v2::ToEnvoy::ToEnvoySqliteCommitStageResponse(_)
		| v2::ToEnvoy::ToEnvoySqliteCommitFinalizeResponse(_) => {
			bail!("legacy sqlite responses require envoy-protocol v2")
		}
	})
}

pub fn convert_to_envoy_conn_ping_v2_to_v3(x: v2::ToEnvoyConnPing) -> Result<v3::ToEnvoyConnPing> {
	Ok(v3::ToEnvoyConnPing {
		gateway_id: x.gateway_id,
		request_id: x.request_id,
		ts: x.ts,
	})
}

pub fn convert_to_envoy_conn_v2_to_v3(x: v2::ToEnvoyConn) -> Result<v3::ToEnvoyConn> {
	Ok(match x {
		v2::ToEnvoyConn::ToEnvoyConnPing(v) => {
			v3::ToEnvoyConn::ToEnvoyConnPing(convert_to_envoy_conn_ping_v2_to_v3(v)?)
		}
		v2::ToEnvoyConn::ToEnvoyConnClose => v3::ToEnvoyConn::ToEnvoyConnClose,
		v2::ToEnvoyConn::ToEnvoyCommands(v) => {
			v3::ToEnvoyConn::ToEnvoyCommands(convert_to_envoy_commands_v2_to_v3(v)?)
		}
		v2::ToEnvoyConn::ToEnvoyAckEvents(v) => {
			v3::ToEnvoyConn::ToEnvoyAckEvents(convert_to_envoy_ack_events_v2_to_v3(v)?)
		}
		v2::ToEnvoyConn::ToEnvoyTunnelMessage(v) => {
			v3::ToEnvoyConn::ToEnvoyTunnelMessage(convert_to_envoy_tunnel_message_v2_to_v3(v)?)
		}
	})
}

pub fn convert_to_gateway_pong_v2_to_v3(x: v2::ToGatewayPong) -> Result<v3::ToGatewayPong> {
	Ok(v3::ToGatewayPong {
		request_id: x.request_id,
		ts: x.ts,
	})
}

pub fn convert_to_gateway_v2_to_v3(x: v2::ToGateway) -> Result<v3::ToGateway> {
	Ok(match x {
		v2::ToGateway::ToGatewayPong(v) => {
			v3::ToGateway::ToGatewayPong(convert_to_gateway_pong_v2_to_v3(v)?)
		}
		v2::ToGateway::ToRivetTunnelMessage(v) => {
			v3::ToGateway::ToRivetTunnelMessage(convert_to_rivet_tunnel_message_v2_to_v3(v)?)
		}
	})
}

pub fn convert_to_outbound_actor_start_v2_to_v3(
	x: v2::ToOutboundActorStart,
) -> Result<v3::ToOutboundActorStart> {
	Ok(v3::ToOutboundActorStart {
		namespace_id: x.namespace_id,
		pool_name: x.pool_name,
		checkpoint: convert_actor_checkpoint_v2_to_v3(x.checkpoint)?,
		actor_config: convert_actor_config_v2_to_v3(x.actor_config)?,
	})
}

pub fn convert_to_outbound_v2_to_v3(x: v2::ToOutbound) -> Result<v3::ToOutbound> {
	Ok(match x {
		v2::ToOutbound::ToOutboundActorStart(v) => {
			v3::ToOutbound::ToOutboundActorStart(convert_to_outbound_actor_start_v2_to_v3(v)?)
		}
	})
}
