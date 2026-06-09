// @generated initial scaffold by scripts/vbare-gen-converters
// from: v3.bare, to: v2.bare
// Replace each todo!() with the migration semantics, then drop the @generated marker.

#![allow(dead_code, unused_variables)]

use anyhow::{Result, bail};

use crate::generated::{v2, v3};

pub fn convert_kv_metadata_v3_to_v2(x: v3::KvMetadata) -> Result<v2::KvMetadata> {
	Ok(v2::KvMetadata {
		version: x.version,
		update_ts: x.update_ts,
	})
}

pub fn convert_kv_list_range_query_v3_to_v2(
	x: v3::KvListRangeQuery,
) -> Result<v2::KvListRangeQuery> {
	Ok(v2::KvListRangeQuery {
		start: x.start,
		end: x.end,
		exclusive: x.exclusive,
	})
}

pub fn convert_kv_list_prefix_query_v3_to_v2(
	x: v3::KvListPrefixQuery,
) -> Result<v2::KvListPrefixQuery> {
	Ok(v2::KvListPrefixQuery { key: x.key })
}

pub fn convert_kv_list_query_v3_to_v2(x: v3::KvListQuery) -> Result<v2::KvListQuery> {
	Ok(match x {
		v3::KvListQuery::KvListAllQuery => v2::KvListQuery::KvListAllQuery,
		v3::KvListQuery::KvListRangeQuery(v) => {
			v2::KvListQuery::KvListRangeQuery(convert_kv_list_range_query_v3_to_v2(v)?)
		}
		v3::KvListQuery::KvListPrefixQuery(v) => {
			v2::KvListQuery::KvListPrefixQuery(convert_kv_list_prefix_query_v3_to_v2(v)?)
		}
	})
}

pub fn convert_kv_get_request_v3_to_v2(x: v3::KvGetRequest) -> Result<v2::KvGetRequest> {
	Ok(v2::KvGetRequest { keys: x.keys })
}

pub fn convert_kv_list_request_v3_to_v2(x: v3::KvListRequest) -> Result<v2::KvListRequest> {
	Ok(v2::KvListRequest {
		query: convert_kv_list_query_v3_to_v2(x.query)?,
		reverse: x.reverse,
		limit: x.limit,
	})
}

pub fn convert_kv_put_request_v3_to_v2(x: v3::KvPutRequest) -> Result<v2::KvPutRequest> {
	Ok(v2::KvPutRequest {
		keys: x.keys,
		values: x.values,
	})
}

pub fn convert_kv_delete_request_v3_to_v2(x: v3::KvDeleteRequest) -> Result<v2::KvDeleteRequest> {
	Ok(v2::KvDeleteRequest { keys: x.keys })
}

pub fn convert_kv_delete_range_request_v3_to_v2(
	x: v3::KvDeleteRangeRequest,
) -> Result<v2::KvDeleteRangeRequest> {
	Ok(v2::KvDeleteRangeRequest {
		start: x.start,
		end: x.end,
	})
}

pub fn convert_kv_error_response_v3_to_v2(x: v3::KvErrorResponse) -> Result<v2::KvErrorResponse> {
	Ok(v2::KvErrorResponse { message: x.message })
}

pub fn convert_kv_get_response_v3_to_v2(x: v3::KvGetResponse) -> Result<v2::KvGetResponse> {
	Ok(v2::KvGetResponse {
		keys: x.keys,
		values: x.values,
		metadata: x
			.metadata
			.into_iter()
			.map(|v| convert_kv_metadata_v3_to_v2(v))
			.collect::<Result<Vec<_>>>()?,
	})
}

pub fn convert_kv_list_response_v3_to_v2(x: v3::KvListResponse) -> Result<v2::KvListResponse> {
	Ok(v2::KvListResponse {
		keys: x.keys,
		values: x.values,
		metadata: x
			.metadata
			.into_iter()
			.map(|v| convert_kv_metadata_v3_to_v2(v))
			.collect::<Result<Vec<_>>>()?,
	})
}

pub fn convert_kv_request_data_v3_to_v2(x: v3::KvRequestData) -> Result<v2::KvRequestData> {
	Ok(match x {
		v3::KvRequestData::KvGetRequest(v) => {
			v2::KvRequestData::KvGetRequest(convert_kv_get_request_v3_to_v2(v)?)
		}
		v3::KvRequestData::KvListRequest(v) => {
			v2::KvRequestData::KvListRequest(convert_kv_list_request_v3_to_v2(v)?)
		}
		v3::KvRequestData::KvPutRequest(v) => {
			v2::KvRequestData::KvPutRequest(convert_kv_put_request_v3_to_v2(v)?)
		}
		v3::KvRequestData::KvDeleteRequest(v) => {
			v2::KvRequestData::KvDeleteRequest(convert_kv_delete_request_v3_to_v2(v)?)
		}
		v3::KvRequestData::KvDeleteRangeRequest(v) => {
			v2::KvRequestData::KvDeleteRangeRequest(convert_kv_delete_range_request_v3_to_v2(v)?)
		}
		v3::KvRequestData::KvDropRequest => v2::KvRequestData::KvDropRequest,
	})
}

pub fn convert_kv_response_data_v3_to_v2(x: v3::KvResponseData) -> Result<v2::KvResponseData> {
	Ok(match x {
		v3::KvResponseData::KvErrorResponse(v) => {
			v2::KvResponseData::KvErrorResponse(convert_kv_error_response_v3_to_v2(v)?)
		}
		v3::KvResponseData::KvGetResponse(v) => {
			v2::KvResponseData::KvGetResponse(convert_kv_get_response_v3_to_v2(v)?)
		}
		v3::KvResponseData::KvListResponse(v) => {
			v2::KvResponseData::KvListResponse(convert_kv_list_response_v3_to_v2(v)?)
		}
		v3::KvResponseData::KvPutResponse => v2::KvResponseData::KvPutResponse,
		v3::KvResponseData::KvDeleteResponse => v2::KvResponseData::KvDeleteResponse,
		v3::KvResponseData::KvDropResponse => v2::KvResponseData::KvDropResponse,
	})
}

pub fn convert_sqlite_dirty_page_v3_to_v2(x: v3::SqliteDirtyPage) -> Result<v2::SqliteDirtyPage> {
	Ok(v2::SqliteDirtyPage {
		pgno: x.pgno,
		bytes: x.bytes,
	})
}

pub fn convert_sqlite_fetched_page_v3_to_v2(
	x: v3::SqliteFetchedPage,
) -> Result<v2::SqliteFetchedPage> {
	Ok(v2::SqliteFetchedPage {
		pgno: x.pgno,
		bytes: x.bytes,
	})
}

pub fn convert_sqlite_get_pages_request_v3_to_v2(
	x: v3::SqliteGetPagesRequest,
) -> Result<v2::SqliteGetPagesRequest> {
	Ok(v2::SqliteGetPagesRequest {
		actor_id: x.actor_id,
		generation: todo!(),
		pgnos: x.pgnos,
	})
}

pub fn convert_sqlite_get_pages_ok_v3_to_v2(
	x: v3::SqliteGetPagesOk,
) -> Result<v2::SqliteGetPagesOk> {
	Ok(v2::SqliteGetPagesOk {
		pages: x
			.pages
			.into_iter()
			.map(|v| convert_sqlite_fetched_page_v3_to_v2(v))
			.collect::<Result<Vec<_>>>()?,
		meta: todo!(),
	})
}

pub fn convert_sqlite_error_response_v3_to_v2(
	x: v3::SqliteErrorResponse,
) -> Result<v2::SqliteErrorResponse> {
	Ok(v2::SqliteErrorResponse { message: x.message })
}

pub fn convert_sqlite_get_pages_response_v3_to_v2(
	x: v3::SqliteGetPagesResponse,
) -> Result<v2::SqliteGetPagesResponse> {
	Ok(match x {
		v3::SqliteGetPagesResponse::SqliteGetPagesOk(v) => {
			v2::SqliteGetPagesResponse::SqliteGetPagesOk(convert_sqlite_get_pages_ok_v3_to_v2(v)?)
		}
		v3::SqliteGetPagesResponse::SqliteErrorResponse(v) => {
			v2::SqliteGetPagesResponse::SqliteErrorResponse(convert_sqlite_error_response_v3_to_v2(
				v,
			)?)
		}
	})
}

pub fn convert_sqlite_commit_request_v3_to_v2(
	x: v3::SqliteCommitRequest,
) -> Result<v2::SqliteCommitRequest> {
	Ok(v2::SqliteCommitRequest {
		actor_id: x.actor_id,
		generation: todo!(),
		expected_head_txid: todo!(),
		dirty_pages: x
			.dirty_pages
			.into_iter()
			.map(|v| convert_sqlite_dirty_page_v3_to_v2(v))
			.collect::<Result<Vec<_>>>()?,
		new_db_size_pages: todo!(),
	})
}

pub fn convert_sqlite_commit_response_v3_to_v2(
	x: v3::SqliteCommitResponse,
) -> Result<v2::SqliteCommitResponse> {
	Ok(match x {
		v3::SqliteCommitResponse::SqliteCommitOk => todo!(),
		v3::SqliteCommitResponse::SqliteErrorResponse(v) => {
			v2::SqliteCommitResponse::SqliteErrorResponse(convert_sqlite_error_response_v3_to_v2(
				v,
			)?)
		}
	})
}

pub fn convert_stop_code_v3_to_v2(x: v3::StopCode) -> Result<v2::StopCode> {
	Ok(match x {
		v3::StopCode::Ok => v2::StopCode::Ok,
		v3::StopCode::Error => v2::StopCode::Error,
	})
}

pub fn convert_actor_name_v3_to_v2(x: v3::ActorName) -> Result<v2::ActorName> {
	Ok(v2::ActorName {
		metadata: x.metadata,
	})
}

pub fn convert_actor_config_v3_to_v2(x: v3::ActorConfig) -> Result<v2::ActorConfig> {
	Ok(v2::ActorConfig {
		name: x.name,
		key: x.key,
		create_ts: x.create_ts,
		input: x.input,
	})
}

pub fn convert_actor_checkpoint_v3_to_v2(x: v3::ActorCheckpoint) -> Result<v2::ActorCheckpoint> {
	Ok(v2::ActorCheckpoint {
		actor_id: x.actor_id,
		generation: x.generation,
		index: x.index,
	})
}

pub fn convert_actor_intent_v3_to_v2(x: v3::ActorIntent) -> Result<v2::ActorIntent> {
	Ok(match x {
		v3::ActorIntent::ActorIntentSleep => v2::ActorIntent::ActorIntentSleep,
		v3::ActorIntent::ActorIntentStop => v2::ActorIntent::ActorIntentStop,
	})
}

pub fn convert_actor_state_stopped_v3_to_v2(
	x: v3::ActorStateStopped,
) -> Result<v2::ActorStateStopped> {
	Ok(v2::ActorStateStopped {
		code: convert_stop_code_v3_to_v2(x.code)?,
		message: x.message,
	})
}

pub fn convert_actor_state_v3_to_v2(x: v3::ActorState) -> Result<v2::ActorState> {
	Ok(match x {
		v3::ActorState::ActorStateRunning => v2::ActorState::ActorStateRunning,
		v3::ActorState::ActorStateStopped(v) => {
			v2::ActorState::ActorStateStopped(convert_actor_state_stopped_v3_to_v2(v)?)
		}
	})
}

pub fn convert_event_actor_intent_v3_to_v2(
	x: v3::EventActorIntent,
) -> Result<v2::EventActorIntent> {
	Ok(v2::EventActorIntent {
		intent: convert_actor_intent_v3_to_v2(x.intent)?,
	})
}

pub fn convert_event_actor_state_update_v3_to_v2(
	x: v3::EventActorStateUpdate,
) -> Result<v2::EventActorStateUpdate> {
	Ok(v2::EventActorStateUpdate {
		state: convert_actor_state_v3_to_v2(x.state)?,
	})
}

pub fn convert_event_actor_set_alarm_v3_to_v2(
	x: v3::EventActorSetAlarm,
) -> Result<v2::EventActorSetAlarm> {
	Ok(v2::EventActorSetAlarm {
		alarm_ts: x.alarm_ts,
	})
}

pub fn convert_event_v3_to_v2(x: v3::Event) -> Result<v2::Event> {
	Ok(match x {
		v3::Event::EventActorIntent(v) => {
			v2::Event::EventActorIntent(convert_event_actor_intent_v3_to_v2(v)?)
		}
		v3::Event::EventActorStateUpdate(v) => {
			v2::Event::EventActorStateUpdate(convert_event_actor_state_update_v3_to_v2(v)?)
		}
		v3::Event::EventActorSetAlarm(v) => {
			v2::Event::EventActorSetAlarm(convert_event_actor_set_alarm_v3_to_v2(v)?)
		}
	})
}

pub fn convert_event_wrapper_v3_to_v2(x: v3::EventWrapper) -> Result<v2::EventWrapper> {
	Ok(v2::EventWrapper {
		checkpoint: convert_actor_checkpoint_v3_to_v2(x.checkpoint)?,
		inner: convert_event_v3_to_v2(x.inner)?,
	})
}

pub fn convert_preloaded_kv_entry_v3_to_v2(
	x: v3::PreloadedKvEntry,
) -> Result<v2::PreloadedKvEntry> {
	Ok(v2::PreloadedKvEntry {
		key: x.key,
		value: x.value,
		metadata: convert_kv_metadata_v3_to_v2(x.metadata)?,
	})
}

pub fn convert_preloaded_kv_v3_to_v2(x: v3::PreloadedKv) -> Result<v2::PreloadedKv> {
	Ok(v2::PreloadedKv {
		entries: x
			.entries
			.into_iter()
			.map(|v| convert_preloaded_kv_entry_v3_to_v2(v))
			.collect::<Result<Vec<_>>>()?,
		requested_get_keys: x.requested_get_keys,
		requested_prefixes: x.requested_prefixes,
	})
}

pub fn convert_hibernating_request_v3_to_v2(
	x: v3::HibernatingRequest,
) -> Result<v2::HibernatingRequest> {
	Ok(v2::HibernatingRequest {
		gateway_id: x.gateway_id,
		request_id: x.request_id,
	})
}

pub fn convert_command_start_actor_v3_to_v2(
	x: v3::CommandStartActor,
) -> Result<v2::CommandStartActor> {
	Ok(v2::CommandStartActor {
		config: convert_actor_config_v3_to_v2(x.config)?,
		hibernating_requests: x
			.hibernating_requests
			.into_iter()
			.map(|v| convert_hibernating_request_v3_to_v2(v))
			.collect::<Result<Vec<_>>>()?,
		preloaded_kv: x
			.preloaded_kv
			.map(|v| convert_preloaded_kv_v3_to_v2(v))
			.transpose()?,
		// v3 dropped the field; downgrade leaves it absent on the v2 wire.
		sqlite_startup_data: None,
	})
}

pub fn convert_stop_actor_reason_v3_to_v2(x: v3::StopActorReason) -> Result<v2::StopActorReason> {
	Ok(match x {
		v3::StopActorReason::SleepIntent => v2::StopActorReason::SleepIntent,
		v3::StopActorReason::StopIntent => v2::StopActorReason::StopIntent,
		v3::StopActorReason::Destroy => v2::StopActorReason::Destroy,
		v3::StopActorReason::GoingAway => v2::StopActorReason::GoingAway,
		v3::StopActorReason::Lost => v2::StopActorReason::Lost,
	})
}

pub fn convert_command_stop_actor_v3_to_v2(
	x: v3::CommandStopActor,
) -> Result<v2::CommandStopActor> {
	Ok(v2::CommandStopActor {
		reason: convert_stop_actor_reason_v3_to_v2(x.reason)?,
	})
}

pub fn convert_command_v3_to_v2(x: v3::Command) -> Result<v2::Command> {
	Ok(match x {
		v3::Command::CommandStartActor(v) => {
			v2::Command::CommandStartActor(convert_command_start_actor_v3_to_v2(v)?)
		}
		v3::Command::CommandStopActor(v) => {
			v2::Command::CommandStopActor(convert_command_stop_actor_v3_to_v2(v)?)
		}
	})
}

pub fn convert_command_wrapper_v3_to_v2(x: v3::CommandWrapper) -> Result<v2::CommandWrapper> {
	Ok(v2::CommandWrapper {
		checkpoint: convert_actor_checkpoint_v3_to_v2(x.checkpoint)?,
		inner: convert_command_v3_to_v2(x.inner)?,
	})
}

pub fn convert_actor_command_key_data_v3_to_v2(
	x: v3::ActorCommandKeyData,
) -> Result<v2::ActorCommandKeyData> {
	Ok(match x {
		v3::ActorCommandKeyData::CommandStartActor(v) => {
			v2::ActorCommandKeyData::CommandStartActor(convert_command_start_actor_v3_to_v2(v)?)
		}
		v3::ActorCommandKeyData::CommandStopActor(v) => {
			v2::ActorCommandKeyData::CommandStopActor(convert_command_stop_actor_v3_to_v2(v)?)
		}
	})
}

pub fn convert_message_id_v3_to_v2(x: v3::MessageId) -> Result<v2::MessageId> {
	Ok(v2::MessageId {
		gateway_id: x.gateway_id,
		request_id: x.request_id,
		message_index: x.message_index,
	})
}

pub fn convert_to_envoy_request_start_v3_to_v2(
	x: v3::ToEnvoyRequestStart,
) -> Result<v2::ToEnvoyRequestStart> {
	Ok(v2::ToEnvoyRequestStart {
		actor_id: x.actor_id,
		method: x.method,
		path: x.path,
		headers: x.headers,
		body: x.body,
		stream: x.stream,
	})
}

pub fn convert_to_envoy_request_chunk_v3_to_v2(
	x: v3::ToEnvoyRequestChunk,
) -> Result<v2::ToEnvoyRequestChunk> {
	Ok(v2::ToEnvoyRequestChunk {
		body: x.body,
		finish: x.finish,
	})
}

pub fn convert_to_rivet_response_start_v3_to_v2(
	x: v3::ToRivetResponseStart,
) -> Result<v2::ToRivetResponseStart> {
	Ok(v2::ToRivetResponseStart {
		status: x.status,
		headers: x.headers,
		body: x.body,
		stream: x.stream,
	})
}

pub fn convert_to_rivet_response_chunk_v3_to_v2(
	x: v3::ToRivetResponseChunk,
) -> Result<v2::ToRivetResponseChunk> {
	Ok(v2::ToRivetResponseChunk {
		body: x.body,
		finish: x.finish,
	})
}

pub fn convert_to_envoy_web_socket_open_v3_to_v2(
	x: v3::ToEnvoyWebSocketOpen,
) -> Result<v2::ToEnvoyWebSocketOpen> {
	Ok(v2::ToEnvoyWebSocketOpen {
		actor_id: x.actor_id,
		path: x.path,
		headers: x.headers,
	})
}

pub fn convert_to_envoy_web_socket_message_v3_to_v2(
	x: v3::ToEnvoyWebSocketMessage,
) -> Result<v2::ToEnvoyWebSocketMessage> {
	Ok(v2::ToEnvoyWebSocketMessage {
		data: x.data,
		binary: x.binary,
	})
}

pub fn convert_to_envoy_web_socket_close_v3_to_v2(
	x: v3::ToEnvoyWebSocketClose,
) -> Result<v2::ToEnvoyWebSocketClose> {
	Ok(v2::ToEnvoyWebSocketClose {
		code: x.code,
		reason: x.reason,
	})
}

pub fn convert_to_rivet_web_socket_open_v3_to_v2(
	x: v3::ToRivetWebSocketOpen,
) -> Result<v2::ToRivetWebSocketOpen> {
	Ok(v2::ToRivetWebSocketOpen {
		can_hibernate: x.can_hibernate,
	})
}

pub fn convert_to_rivet_web_socket_message_v3_to_v2(
	x: v3::ToRivetWebSocketMessage,
) -> Result<v2::ToRivetWebSocketMessage> {
	Ok(v2::ToRivetWebSocketMessage {
		data: x.data,
		binary: x.binary,
	})
}

pub fn convert_to_rivet_web_socket_message_ack_v3_to_v2(
	x: v3::ToRivetWebSocketMessageAck,
) -> Result<v2::ToRivetWebSocketMessageAck> {
	Ok(v2::ToRivetWebSocketMessageAck { index: x.index })
}

pub fn convert_to_rivet_web_socket_close_v3_to_v2(
	x: v3::ToRivetWebSocketClose,
) -> Result<v2::ToRivetWebSocketClose> {
	Ok(v2::ToRivetWebSocketClose {
		code: x.code,
		reason: x.reason,
		hibernate: x.hibernate,
	})
}

pub fn convert_to_rivet_tunnel_message_kind_v3_to_v2(
	x: v3::ToRivetTunnelMessageKind,
) -> Result<v2::ToRivetTunnelMessageKind> {
	Ok(match x {
		v3::ToRivetTunnelMessageKind::ToRivetResponseStart(v) => {
			v2::ToRivetTunnelMessageKind::ToRivetResponseStart(
				convert_to_rivet_response_start_v3_to_v2(v)?,
			)
		}
		v3::ToRivetTunnelMessageKind::ToRivetResponseChunk(v) => {
			v2::ToRivetTunnelMessageKind::ToRivetResponseChunk(
				convert_to_rivet_response_chunk_v3_to_v2(v)?,
			)
		}
		v3::ToRivetTunnelMessageKind::ToRivetResponseAbort => {
			v2::ToRivetTunnelMessageKind::ToRivetResponseAbort
		}
		v3::ToRivetTunnelMessageKind::ToRivetWebSocketOpen(v) => {
			v2::ToRivetTunnelMessageKind::ToRivetWebSocketOpen(
				convert_to_rivet_web_socket_open_v3_to_v2(v)?,
			)
		}
		v3::ToRivetTunnelMessageKind::ToRivetWebSocketMessage(v) => {
			v2::ToRivetTunnelMessageKind::ToRivetWebSocketMessage(
				convert_to_rivet_web_socket_message_v3_to_v2(v)?,
			)
		}
		v3::ToRivetTunnelMessageKind::ToRivetWebSocketMessageAck(v) => {
			v2::ToRivetTunnelMessageKind::ToRivetWebSocketMessageAck(
				convert_to_rivet_web_socket_message_ack_v3_to_v2(v)?,
			)
		}
		v3::ToRivetTunnelMessageKind::ToRivetWebSocketClose(v) => {
			v2::ToRivetTunnelMessageKind::ToRivetWebSocketClose(
				convert_to_rivet_web_socket_close_v3_to_v2(v)?,
			)
		}
	})
}

pub fn convert_to_rivet_tunnel_message_v3_to_v2(
	x: v3::ToRivetTunnelMessage,
) -> Result<v2::ToRivetTunnelMessage> {
	Ok(v2::ToRivetTunnelMessage {
		message_id: convert_message_id_v3_to_v2(x.message_id)?,
		message_kind: convert_to_rivet_tunnel_message_kind_v3_to_v2(x.message_kind)?,
	})
}

pub fn convert_to_envoy_tunnel_message_kind_v3_to_v2(
	x: v3::ToEnvoyTunnelMessageKind,
) -> Result<v2::ToEnvoyTunnelMessageKind> {
	Ok(match x {
		v3::ToEnvoyTunnelMessageKind::ToEnvoyRequestStart(v) => {
			v2::ToEnvoyTunnelMessageKind::ToEnvoyRequestStart(
				convert_to_envoy_request_start_v3_to_v2(v)?,
			)
		}
		v3::ToEnvoyTunnelMessageKind::ToEnvoyRequestChunk(v) => {
			v2::ToEnvoyTunnelMessageKind::ToEnvoyRequestChunk(
				convert_to_envoy_request_chunk_v3_to_v2(v)?,
			)
		}
		v3::ToEnvoyTunnelMessageKind::ToEnvoyRequestAbort => {
			v2::ToEnvoyTunnelMessageKind::ToEnvoyRequestAbort
		}
		v3::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketOpen(v) => {
			v2::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketOpen(
				convert_to_envoy_web_socket_open_v3_to_v2(v)?,
			)
		}
		v3::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketMessage(v) => {
			v2::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketMessage(
				convert_to_envoy_web_socket_message_v3_to_v2(v)?,
			)
		}
		v3::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketClose(v) => {
			v2::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketClose(
				convert_to_envoy_web_socket_close_v3_to_v2(v)?,
			)
		}
	})
}

pub fn convert_to_envoy_tunnel_message_v3_to_v2(
	x: v3::ToEnvoyTunnelMessage,
) -> Result<v2::ToEnvoyTunnelMessage> {
	Ok(v2::ToEnvoyTunnelMessage {
		message_id: convert_message_id_v3_to_v2(x.message_id)?,
		message_kind: convert_to_envoy_tunnel_message_kind_v3_to_v2(x.message_kind)?,
	})
}

pub fn convert_to_envoy_ping_v3_to_v2(x: v3::ToEnvoyPing) -> Result<v2::ToEnvoyPing> {
	Ok(v2::ToEnvoyPing { ts: x.ts })
}

pub fn convert_to_rivet_metadata_v3_to_v2(x: v3::ToRivetMetadata) -> Result<v2::ToRivetMetadata> {
	Ok(v2::ToRivetMetadata {
		prepopulate_actor_names: x
			.prepopulate_actor_names
			.map(|v| {
				v.into_iter()
					.map(|(k, v)| -> Result<_> { Ok((k, convert_actor_name_v3_to_v2(v)?)) })
					.collect::<Result<_>>()
			})
			.transpose()?,
		metadata: x.metadata,
	})
}

pub fn convert_to_rivet_events_v3_to_v2(x: v3::ToRivetEvents) -> Result<v2::ToRivetEvents> {
	Ok(x.into_iter()
		.map(|v| convert_event_wrapper_v3_to_v2(v))
		.collect::<Result<Vec<_>>>()?)
}

pub fn convert_to_rivet_ack_commands_v3_to_v2(
	x: v3::ToRivetAckCommands,
) -> Result<v2::ToRivetAckCommands> {
	Ok(v2::ToRivetAckCommands {
		last_command_checkpoints: x
			.last_command_checkpoints
			.into_iter()
			.map(|v| convert_actor_checkpoint_v3_to_v2(v))
			.collect::<Result<Vec<_>>>()?,
	})
}

pub fn convert_to_rivet_pong_v3_to_v2(x: v3::ToRivetPong) -> Result<v2::ToRivetPong> {
	Ok(v2::ToRivetPong { ts: x.ts })
}

pub fn convert_to_rivet_kv_request_v3_to_v2(
	x: v3::ToRivetKvRequest,
) -> Result<v2::ToRivetKvRequest> {
	Ok(v2::ToRivetKvRequest {
		actor_id: x.actor_id,
		request_id: x.request_id,
		data: convert_kv_request_data_v3_to_v2(x.data)?,
	})
}

pub fn convert_to_rivet_sqlite_get_pages_request_v3_to_v2(
	x: v3::ToRivetSqliteGetPagesRequest,
) -> Result<v2::ToRivetSqliteGetPagesRequest> {
	Ok(v2::ToRivetSqliteGetPagesRequest {
		request_id: x.request_id,
		data: convert_sqlite_get_pages_request_v3_to_v2(x.data)?,
	})
}

pub fn convert_to_rivet_sqlite_commit_request_v3_to_v2(
	x: v3::ToRivetSqliteCommitRequest,
) -> Result<v2::ToRivetSqliteCommitRequest> {
	Ok(v2::ToRivetSqliteCommitRequest {
		request_id: x.request_id,
		data: convert_sqlite_commit_request_v3_to_v2(x.data)?,
	})
}

pub fn convert_to_rivet_v3_to_v2(x: v3::ToRivet) -> Result<v2::ToRivet> {
	Ok(match x {
		v3::ToRivet::ToRivetMetadata(v) => {
			v2::ToRivet::ToRivetMetadata(convert_to_rivet_metadata_v3_to_v2(v)?)
		}
		v3::ToRivet::ToRivetEvents(v) => {
			v2::ToRivet::ToRivetEvents(convert_to_rivet_events_v3_to_v2(v)?)
		}
		v3::ToRivet::ToRivetAckCommands(v) => {
			v2::ToRivet::ToRivetAckCommands(convert_to_rivet_ack_commands_v3_to_v2(v)?)
		}
		v3::ToRivet::ToRivetStopping => v2::ToRivet::ToRivetStopping,
		v3::ToRivet::ToRivetPong(v) => v2::ToRivet::ToRivetPong(convert_to_rivet_pong_v3_to_v2(v)?),
		v3::ToRivet::ToRivetKvRequest(v) => {
			v2::ToRivet::ToRivetKvRequest(convert_to_rivet_kv_request_v3_to_v2(v)?)
		}
		v3::ToRivet::ToRivetTunnelMessage(v) => {
			v2::ToRivet::ToRivetTunnelMessage(convert_to_rivet_tunnel_message_v3_to_v2(v)?)
		}
		v3::ToRivet::ToRivetSqliteGetPagesRequest(_)
		| v3::ToRivet::ToRivetSqliteCommitRequest(_) => {
			bail!("stateless sqlite requests require envoy-protocol v3")
		}
	})
}

pub fn convert_protocol_metadata_v3_to_v2(x: v3::ProtocolMetadata) -> Result<v2::ProtocolMetadata> {
	Ok(v2::ProtocolMetadata {
		envoy_lost_threshold: x.envoy_lost_threshold,
		actor_stop_threshold: x.actor_stop_threshold,
		max_response_payload_size: x.max_response_payload_size,
	})
}

pub fn convert_to_envoy_init_v3_to_v2(x: v3::ToEnvoyInit) -> Result<v2::ToEnvoyInit> {
	Ok(v2::ToEnvoyInit {
		metadata: convert_protocol_metadata_v3_to_v2(x.metadata)?,
	})
}

pub fn convert_to_envoy_commands_v3_to_v2(x: v3::ToEnvoyCommands) -> Result<v2::ToEnvoyCommands> {
	Ok(x.into_iter()
		.map(|v| convert_command_wrapper_v3_to_v2(v))
		.collect::<Result<Vec<_>>>()?)
}

pub fn convert_to_envoy_ack_events_v3_to_v2(
	x: v3::ToEnvoyAckEvents,
) -> Result<v2::ToEnvoyAckEvents> {
	Ok(v2::ToEnvoyAckEvents {
		last_event_checkpoints: x
			.last_event_checkpoints
			.into_iter()
			.map(|v| convert_actor_checkpoint_v3_to_v2(v))
			.collect::<Result<Vec<_>>>()?,
	})
}

pub fn convert_to_envoy_kv_response_v3_to_v2(
	x: v3::ToEnvoyKvResponse,
) -> Result<v2::ToEnvoyKvResponse> {
	Ok(v2::ToEnvoyKvResponse {
		request_id: x.request_id,
		data: convert_kv_response_data_v3_to_v2(x.data)?,
	})
}

pub fn convert_to_envoy_sqlite_get_pages_response_v3_to_v2(
	x: v3::ToEnvoySqliteGetPagesResponse,
) -> Result<v2::ToEnvoySqliteGetPagesResponse> {
	Ok(v2::ToEnvoySqliteGetPagesResponse {
		request_id: x.request_id,
		data: convert_sqlite_get_pages_response_v3_to_v2(x.data)?,
	})
}

pub fn convert_to_envoy_sqlite_commit_response_v3_to_v2(
	x: v3::ToEnvoySqliteCommitResponse,
) -> Result<v2::ToEnvoySqliteCommitResponse> {
	Ok(v2::ToEnvoySqliteCommitResponse {
		request_id: x.request_id,
		data: convert_sqlite_commit_response_v3_to_v2(x.data)?,
	})
}

pub fn convert_to_envoy_v3_to_v2(x: v3::ToEnvoy) -> Result<v2::ToEnvoy> {
	Ok(match x {
		v3::ToEnvoy::ToEnvoyInit(v) => v2::ToEnvoy::ToEnvoyInit(convert_to_envoy_init_v3_to_v2(v)?),
		v3::ToEnvoy::ToEnvoyCommands(v) => {
			v2::ToEnvoy::ToEnvoyCommands(convert_to_envoy_commands_v3_to_v2(v)?)
		}
		v3::ToEnvoy::ToEnvoyAckEvents(v) => {
			v2::ToEnvoy::ToEnvoyAckEvents(convert_to_envoy_ack_events_v3_to_v2(v)?)
		}
		v3::ToEnvoy::ToEnvoyKvResponse(v) => {
			v2::ToEnvoy::ToEnvoyKvResponse(convert_to_envoy_kv_response_v3_to_v2(v)?)
		}
		v3::ToEnvoy::ToEnvoyTunnelMessage(v) => {
			v2::ToEnvoy::ToEnvoyTunnelMessage(convert_to_envoy_tunnel_message_v3_to_v2(v)?)
		}
		v3::ToEnvoy::ToEnvoyPing(v) => v2::ToEnvoy::ToEnvoyPing(convert_to_envoy_ping_v3_to_v2(v)?),
		v3::ToEnvoy::ToEnvoySqliteGetPagesResponse(_)
		| v3::ToEnvoy::ToEnvoySqliteCommitResponse(_) => {
			bail!("stateless sqlite responses require envoy-protocol v3")
		}
	})
}

pub fn convert_to_envoy_conn_ping_v3_to_v2(x: v3::ToEnvoyConnPing) -> Result<v2::ToEnvoyConnPing> {
	Ok(v2::ToEnvoyConnPing {
		gateway_id: x.gateway_id,
		request_id: x.request_id,
		ts: x.ts,
	})
}

pub fn convert_to_envoy_conn_v3_to_v2(x: v3::ToEnvoyConn) -> Result<v2::ToEnvoyConn> {
	Ok(match x {
		v3::ToEnvoyConn::ToEnvoyConnPing(v) => {
			v2::ToEnvoyConn::ToEnvoyConnPing(convert_to_envoy_conn_ping_v3_to_v2(v)?)
		}
		v3::ToEnvoyConn::ToEnvoyConnClose => v2::ToEnvoyConn::ToEnvoyConnClose,
		v3::ToEnvoyConn::ToEnvoyCommands(v) => {
			v2::ToEnvoyConn::ToEnvoyCommands(convert_to_envoy_commands_v3_to_v2(v)?)
		}
		v3::ToEnvoyConn::ToEnvoyAckEvents(v) => {
			v2::ToEnvoyConn::ToEnvoyAckEvents(convert_to_envoy_ack_events_v3_to_v2(v)?)
		}
		v3::ToEnvoyConn::ToEnvoyTunnelMessage(v) => {
			v2::ToEnvoyConn::ToEnvoyTunnelMessage(convert_to_envoy_tunnel_message_v3_to_v2(v)?)
		}
	})
}

pub fn convert_to_gateway_pong_v3_to_v2(x: v3::ToGatewayPong) -> Result<v2::ToGatewayPong> {
	Ok(v2::ToGatewayPong {
		request_id: x.request_id,
		ts: x.ts,
	})
}

pub fn convert_to_gateway_v3_to_v2(x: v3::ToGateway) -> Result<v2::ToGateway> {
	Ok(match x {
		v3::ToGateway::ToGatewayPong(v) => {
			v2::ToGateway::ToGatewayPong(convert_to_gateway_pong_v3_to_v2(v)?)
		}
		v3::ToGateway::ToRivetTunnelMessage(v) => {
			v2::ToGateway::ToRivetTunnelMessage(convert_to_rivet_tunnel_message_v3_to_v2(v)?)
		}
	})
}

pub fn convert_to_outbound_actor_start_v3_to_v2(
	x: v3::ToOutboundActorStart,
) -> Result<v2::ToOutboundActorStart> {
	Ok(v2::ToOutboundActorStart {
		namespace_id: x.namespace_id,
		pool_name: x.pool_name,
		checkpoint: convert_actor_checkpoint_v3_to_v2(x.checkpoint)?,
		actor_config: convert_actor_config_v3_to_v2(x.actor_config)?,
	})
}

pub fn convert_to_outbound_v3_to_v2(x: v3::ToOutbound) -> Result<v2::ToOutbound> {
	Ok(match x {
		v3::ToOutbound::ToOutboundActorStart(v) => {
			v2::ToOutbound::ToOutboundActorStart(convert_to_outbound_actor_start_v3_to_v2(v)?)
		}
	})
}
