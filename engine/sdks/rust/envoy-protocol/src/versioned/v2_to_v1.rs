// @generated initial scaffold by scripts/vbare-gen-converters
// from: v2.bare, to: v1.bare
// Replace each todo!() with the migration semantics, then drop the @generated marker.

#![allow(dead_code, unused_variables)]

use anyhow::{Result, bail};

use crate::generated::{v1, v2};
use crate::versioned::{
	ProtocolCompatibilityDirection, ProtocolCompatibilityFeature, incompatible,
};

pub fn convert_kv_metadata_v2_to_v1(x: v2::KvMetadata) -> Result<v1::KvMetadata> {
	Ok(v1::KvMetadata {
		version: x.version,
		update_ts: x.update_ts,
	})
}

pub fn convert_kv_list_range_query_v2_to_v1(
	x: v2::KvListRangeQuery,
) -> Result<v1::KvListRangeQuery> {
	Ok(v1::KvListRangeQuery {
		start: x.start,
		end: x.end,
		exclusive: x.exclusive,
	})
}

pub fn convert_kv_list_prefix_query_v2_to_v1(
	x: v2::KvListPrefixQuery,
) -> Result<v1::KvListPrefixQuery> {
	Ok(v1::KvListPrefixQuery { key: x.key })
}

pub fn convert_kv_list_query_v2_to_v1(x: v2::KvListQuery) -> Result<v1::KvListQuery> {
	Ok(match x {
		v2::KvListQuery::KvListAllQuery => v1::KvListQuery::KvListAllQuery,
		v2::KvListQuery::KvListRangeQuery(v) => {
			v1::KvListQuery::KvListRangeQuery(convert_kv_list_range_query_v2_to_v1(v)?)
		}
		v2::KvListQuery::KvListPrefixQuery(v) => {
			v1::KvListQuery::KvListPrefixQuery(convert_kv_list_prefix_query_v2_to_v1(v)?)
		}
	})
}

pub fn convert_kv_get_request_v2_to_v1(x: v2::KvGetRequest) -> Result<v1::KvGetRequest> {
	Ok(v1::KvGetRequest { keys: x.keys })
}

pub fn convert_kv_list_request_v2_to_v1(x: v2::KvListRequest) -> Result<v1::KvListRequest> {
	Ok(v1::KvListRequest {
		query: convert_kv_list_query_v2_to_v1(x.query)?,
		reverse: x.reverse,
		limit: x.limit,
	})
}

pub fn convert_kv_put_request_v2_to_v1(x: v2::KvPutRequest) -> Result<v1::KvPutRequest> {
	Ok(v1::KvPutRequest {
		keys: x.keys,
		values: x.values,
	})
}

pub fn convert_kv_delete_request_v2_to_v1(x: v2::KvDeleteRequest) -> Result<v1::KvDeleteRequest> {
	Ok(v1::KvDeleteRequest { keys: x.keys })
}

pub fn convert_kv_delete_range_request_v2_to_v1(
	x: v2::KvDeleteRangeRequest,
) -> Result<v1::KvDeleteRangeRequest> {
	Ok(v1::KvDeleteRangeRequest {
		start: x.start,
		end: x.end,
	})
}

pub fn convert_kv_error_response_v2_to_v1(x: v2::KvErrorResponse) -> Result<v1::KvErrorResponse> {
	Ok(v1::KvErrorResponse { message: x.message })
}

pub fn convert_kv_get_response_v2_to_v1(x: v2::KvGetResponse) -> Result<v1::KvGetResponse> {
	Ok(v1::KvGetResponse {
		keys: x.keys,
		values: x.values,
		metadata: x
			.metadata
			.into_iter()
			.map(|v| convert_kv_metadata_v2_to_v1(v))
			.collect::<Result<Vec<_>>>()?,
	})
}

pub fn convert_kv_list_response_v2_to_v1(x: v2::KvListResponse) -> Result<v1::KvListResponse> {
	Ok(v1::KvListResponse {
		keys: x.keys,
		values: x.values,
		metadata: x
			.metadata
			.into_iter()
			.map(|v| convert_kv_metadata_v2_to_v1(v))
			.collect::<Result<Vec<_>>>()?,
	})
}

pub fn convert_kv_request_data_v2_to_v1(x: v2::KvRequestData) -> Result<v1::KvRequestData> {
	Ok(match x {
		v2::KvRequestData::KvGetRequest(v) => {
			v1::KvRequestData::KvGetRequest(convert_kv_get_request_v2_to_v1(v)?)
		}
		v2::KvRequestData::KvListRequest(v) => {
			v1::KvRequestData::KvListRequest(convert_kv_list_request_v2_to_v1(v)?)
		}
		v2::KvRequestData::KvPutRequest(v) => {
			v1::KvRequestData::KvPutRequest(convert_kv_put_request_v2_to_v1(v)?)
		}
		v2::KvRequestData::KvDeleteRequest(v) => {
			v1::KvRequestData::KvDeleteRequest(convert_kv_delete_request_v2_to_v1(v)?)
		}
		v2::KvRequestData::KvDeleteRangeRequest(v) => {
			v1::KvRequestData::KvDeleteRangeRequest(convert_kv_delete_range_request_v2_to_v1(v)?)
		}
		v2::KvRequestData::KvDropRequest => v1::KvRequestData::KvDropRequest,
	})
}

pub fn convert_kv_response_data_v2_to_v1(x: v2::KvResponseData) -> Result<v1::KvResponseData> {
	Ok(match x {
		v2::KvResponseData::KvErrorResponse(v) => {
			v1::KvResponseData::KvErrorResponse(convert_kv_error_response_v2_to_v1(v)?)
		}
		v2::KvResponseData::KvGetResponse(v) => {
			v1::KvResponseData::KvGetResponse(convert_kv_get_response_v2_to_v1(v)?)
		}
		v2::KvResponseData::KvListResponse(v) => {
			v1::KvResponseData::KvListResponse(convert_kv_list_response_v2_to_v1(v)?)
		}
		v2::KvResponseData::KvPutResponse => v1::KvResponseData::KvPutResponse,
		v2::KvResponseData::KvDeleteResponse => v1::KvResponseData::KvDeleteResponse,
		v2::KvResponseData::KvDropResponse => v1::KvResponseData::KvDropResponse,
	})
}

pub fn convert_stop_code_v2_to_v1(x: v2::StopCode) -> Result<v1::StopCode> {
	Ok(match x {
		v2::StopCode::Ok => v1::StopCode::Ok,
		v2::StopCode::Error => v1::StopCode::Error,
	})
}

pub fn convert_actor_name_v2_to_v1(x: v2::ActorName) -> Result<v1::ActorName> {
	Ok(v1::ActorName {
		metadata: x.metadata,
	})
}

pub fn convert_actor_config_v2_to_v1(x: v2::ActorConfig) -> Result<v1::ActorConfig> {
	Ok(v1::ActorConfig {
		name: x.name,
		key: x.key,
		create_ts: x.create_ts,
		input: x.input,
	})
}

pub fn convert_actor_checkpoint_v2_to_v1(x: v2::ActorCheckpoint) -> Result<v1::ActorCheckpoint> {
	Ok(v1::ActorCheckpoint {
		actor_id: x.actor_id,
		generation: x.generation,
		index: x.index,
	})
}

pub fn convert_actor_intent_v2_to_v1(x: v2::ActorIntent) -> Result<v1::ActorIntent> {
	Ok(match x {
		v2::ActorIntent::ActorIntentSleep => v1::ActorIntent::ActorIntentSleep,
		v2::ActorIntent::ActorIntentStop => v1::ActorIntent::ActorIntentStop,
	})
}

pub fn convert_actor_state_stopped_v2_to_v1(
	x: v2::ActorStateStopped,
) -> Result<v1::ActorStateStopped> {
	Ok(v1::ActorStateStopped {
		code: convert_stop_code_v2_to_v1(x.code)?,
		message: x.message,
	})
}

pub fn convert_actor_state_v2_to_v1(x: v2::ActorState) -> Result<v1::ActorState> {
	Ok(match x {
		v2::ActorState::ActorStateRunning => v1::ActorState::ActorStateRunning,
		v2::ActorState::ActorStateStopped(v) => {
			v1::ActorState::ActorStateStopped(convert_actor_state_stopped_v2_to_v1(v)?)
		}
	})
}

pub fn convert_event_actor_intent_v2_to_v1(
	x: v2::EventActorIntent,
) -> Result<v1::EventActorIntent> {
	Ok(v1::EventActorIntent {
		intent: convert_actor_intent_v2_to_v1(x.intent)?,
	})
}

pub fn convert_event_actor_state_update_v2_to_v1(
	x: v2::EventActorStateUpdate,
) -> Result<v1::EventActorStateUpdate> {
	Ok(v1::EventActorStateUpdate {
		state: convert_actor_state_v2_to_v1(x.state)?,
	})
}

pub fn convert_event_actor_set_alarm_v2_to_v1(
	x: v2::EventActorSetAlarm,
) -> Result<v1::EventActorSetAlarm> {
	Ok(v1::EventActorSetAlarm {
		alarm_ts: x.alarm_ts,
	})
}

pub fn convert_event_v2_to_v1(x: v2::Event) -> Result<v1::Event> {
	Ok(match x {
		v2::Event::EventActorIntent(v) => {
			v1::Event::EventActorIntent(convert_event_actor_intent_v2_to_v1(v)?)
		}
		v2::Event::EventActorStateUpdate(v) => {
			v1::Event::EventActorStateUpdate(convert_event_actor_state_update_v2_to_v1(v)?)
		}
		v2::Event::EventActorSetAlarm(v) => {
			v1::Event::EventActorSetAlarm(convert_event_actor_set_alarm_v2_to_v1(v)?)
		}
	})
}

pub fn convert_event_wrapper_v2_to_v1(x: v2::EventWrapper) -> Result<v1::EventWrapper> {
	Ok(v1::EventWrapper {
		checkpoint: convert_actor_checkpoint_v2_to_v1(x.checkpoint)?,
		inner: convert_event_v2_to_v1(x.inner)?,
	})
}

pub fn convert_preloaded_kv_entry_v2_to_v1(
	x: v2::PreloadedKvEntry,
) -> Result<v1::PreloadedKvEntry> {
	Ok(v1::PreloadedKvEntry {
		key: x.key,
		value: x.value,
		metadata: convert_kv_metadata_v2_to_v1(x.metadata)?,
	})
}

pub fn convert_preloaded_kv_v2_to_v1(x: v2::PreloadedKv) -> Result<v1::PreloadedKv> {
	Ok(v1::PreloadedKv {
		entries: x
			.entries
			.into_iter()
			.map(|v| convert_preloaded_kv_entry_v2_to_v1(v))
			.collect::<Result<Vec<_>>>()?,
		requested_get_keys: x.requested_get_keys,
		requested_prefixes: x.requested_prefixes,
	})
}

pub fn convert_hibernating_request_v2_to_v1(
	x: v2::HibernatingRequest,
) -> Result<v1::HibernatingRequest> {
	Ok(v1::HibernatingRequest {
		gateway_id: x.gateway_id,
		request_id: x.request_id,
	})
}

pub fn convert_command_start_actor_v2_to_v1(
	x: v2::CommandStartActor,
) -> Result<v1::CommandStartActor> {
	if x.sqlite_startup_data.is_some() {
		return Err(incompatible(
			ProtocolCompatibilityFeature::SqliteStartupData,
			ProtocolCompatibilityDirection::ToEnvoy,
			2,
			1,
		));
	}
	Ok(v1::CommandStartActor {
		config: convert_actor_config_v2_to_v1(x.config)?,
		hibernating_requests: x
			.hibernating_requests
			.into_iter()
			.map(|v| convert_hibernating_request_v2_to_v1(v))
			.collect::<Result<Vec<_>>>()?,
		preloaded_kv: x
			.preloaded_kv
			.map(|v| convert_preloaded_kv_v2_to_v1(v))
			.transpose()?,
	})
}

pub fn convert_stop_actor_reason_v2_to_v1(x: v2::StopActorReason) -> Result<v1::StopActorReason> {
	Ok(match x {
		v2::StopActorReason::SleepIntent => v1::StopActorReason::SleepIntent,
		v2::StopActorReason::StopIntent => v1::StopActorReason::StopIntent,
		v2::StopActorReason::Destroy => v1::StopActorReason::Destroy,
		v2::StopActorReason::GoingAway => v1::StopActorReason::GoingAway,
		v2::StopActorReason::Lost => v1::StopActorReason::Lost,
	})
}

pub fn convert_command_stop_actor_v2_to_v1(
	x: v2::CommandStopActor,
) -> Result<v1::CommandStopActor> {
	Ok(v1::CommandStopActor {
		reason: convert_stop_actor_reason_v2_to_v1(x.reason)?,
	})
}

pub fn convert_command_v2_to_v1(x: v2::Command) -> Result<v1::Command> {
	Ok(match x {
		v2::Command::CommandStartActor(v) => {
			v1::Command::CommandStartActor(convert_command_start_actor_v2_to_v1(v)?)
		}
		v2::Command::CommandStopActor(v) => {
			v1::Command::CommandStopActor(convert_command_stop_actor_v2_to_v1(v)?)
		}
	})
}

pub fn convert_command_wrapper_v2_to_v1(x: v2::CommandWrapper) -> Result<v1::CommandWrapper> {
	Ok(v1::CommandWrapper {
		checkpoint: convert_actor_checkpoint_v2_to_v1(x.checkpoint)?,
		inner: convert_command_v2_to_v1(x.inner)?,
	})
}

pub fn convert_actor_command_key_data_v2_to_v1(
	x: v2::ActorCommandKeyData,
) -> Result<v1::ActorCommandKeyData> {
	Ok(match x {
		v2::ActorCommandKeyData::CommandStartActor(v) => {
			v1::ActorCommandKeyData::CommandStartActor(convert_command_start_actor_v2_to_v1(v)?)
		}
		v2::ActorCommandKeyData::CommandStopActor(v) => {
			v1::ActorCommandKeyData::CommandStopActor(convert_command_stop_actor_v2_to_v1(v)?)
		}
	})
}

pub fn convert_message_id_v2_to_v1(x: v2::MessageId) -> Result<v1::MessageId> {
	Ok(v1::MessageId {
		gateway_id: x.gateway_id,
		request_id: x.request_id,
		message_index: x.message_index,
	})
}

pub fn convert_to_envoy_request_start_v2_to_v1(
	x: v2::ToEnvoyRequestStart,
) -> Result<v1::ToEnvoyRequestStart> {
	Ok(v1::ToEnvoyRequestStart {
		actor_id: x.actor_id,
		method: x.method,
		path: x.path,
		headers: x.headers,
		body: x.body,
		stream: x.stream,
	})
}

pub fn convert_to_envoy_request_chunk_v2_to_v1(
	x: v2::ToEnvoyRequestChunk,
) -> Result<v1::ToEnvoyRequestChunk> {
	Ok(v1::ToEnvoyRequestChunk {
		body: x.body,
		finish: x.finish,
	})
}

pub fn convert_to_rivet_response_start_v2_to_v1(
	x: v2::ToRivetResponseStart,
) -> Result<v1::ToRivetResponseStart> {
	Ok(v1::ToRivetResponseStart {
		status: x.status,
		headers: x.headers,
		body: x.body,
		stream: x.stream,
	})
}

pub fn convert_to_rivet_response_chunk_v2_to_v1(
	x: v2::ToRivetResponseChunk,
) -> Result<v1::ToRivetResponseChunk> {
	Ok(v1::ToRivetResponseChunk {
		body: x.body,
		finish: x.finish,
	})
}

pub fn convert_to_envoy_web_socket_open_v2_to_v1(
	x: v2::ToEnvoyWebSocketOpen,
) -> Result<v1::ToEnvoyWebSocketOpen> {
	Ok(v1::ToEnvoyWebSocketOpen {
		actor_id: x.actor_id,
		path: x.path,
		headers: x.headers,
	})
}

pub fn convert_to_envoy_web_socket_message_v2_to_v1(
	x: v2::ToEnvoyWebSocketMessage,
) -> Result<v1::ToEnvoyWebSocketMessage> {
	Ok(v1::ToEnvoyWebSocketMessage {
		data: x.data,
		binary: x.binary,
	})
}

pub fn convert_to_envoy_web_socket_close_v2_to_v1(
	x: v2::ToEnvoyWebSocketClose,
) -> Result<v1::ToEnvoyWebSocketClose> {
	Ok(v1::ToEnvoyWebSocketClose {
		code: x.code,
		reason: x.reason,
	})
}

pub fn convert_to_rivet_web_socket_open_v2_to_v1(
	x: v2::ToRivetWebSocketOpen,
) -> Result<v1::ToRivetWebSocketOpen> {
	Ok(v1::ToRivetWebSocketOpen {
		can_hibernate: x.can_hibernate,
	})
}

pub fn convert_to_rivet_web_socket_message_v2_to_v1(
	x: v2::ToRivetWebSocketMessage,
) -> Result<v1::ToRivetWebSocketMessage> {
	Ok(v1::ToRivetWebSocketMessage {
		data: x.data,
		binary: x.binary,
	})
}

pub fn convert_to_rivet_web_socket_message_ack_v2_to_v1(
	x: v2::ToRivetWebSocketMessageAck,
) -> Result<v1::ToRivetWebSocketMessageAck> {
	Ok(v1::ToRivetWebSocketMessageAck { index: x.index })
}

pub fn convert_to_rivet_web_socket_close_v2_to_v1(
	x: v2::ToRivetWebSocketClose,
) -> Result<v1::ToRivetWebSocketClose> {
	Ok(v1::ToRivetWebSocketClose {
		code: x.code,
		reason: x.reason,
		hibernate: x.hibernate,
	})
}

pub fn convert_to_rivet_tunnel_message_kind_v2_to_v1(
	x: v2::ToRivetTunnelMessageKind,
) -> Result<v1::ToRivetTunnelMessageKind> {
	Ok(match x {
		v2::ToRivetTunnelMessageKind::ToRivetResponseStart(v) => {
			v1::ToRivetTunnelMessageKind::ToRivetResponseStart(
				convert_to_rivet_response_start_v2_to_v1(v)?,
			)
		}
		v2::ToRivetTunnelMessageKind::ToRivetResponseChunk(v) => {
			v1::ToRivetTunnelMessageKind::ToRivetResponseChunk(
				convert_to_rivet_response_chunk_v2_to_v1(v)?,
			)
		}
		v2::ToRivetTunnelMessageKind::ToRivetResponseAbort => {
			v1::ToRivetTunnelMessageKind::ToRivetResponseAbort
		}
		v2::ToRivetTunnelMessageKind::ToRivetWebSocketOpen(v) => {
			v1::ToRivetTunnelMessageKind::ToRivetWebSocketOpen(
				convert_to_rivet_web_socket_open_v2_to_v1(v)?,
			)
		}
		v2::ToRivetTunnelMessageKind::ToRivetWebSocketMessage(v) => {
			v1::ToRivetTunnelMessageKind::ToRivetWebSocketMessage(
				convert_to_rivet_web_socket_message_v2_to_v1(v)?,
			)
		}
		v2::ToRivetTunnelMessageKind::ToRivetWebSocketMessageAck(v) => {
			v1::ToRivetTunnelMessageKind::ToRivetWebSocketMessageAck(
				convert_to_rivet_web_socket_message_ack_v2_to_v1(v)?,
			)
		}
		v2::ToRivetTunnelMessageKind::ToRivetWebSocketClose(v) => {
			v1::ToRivetTunnelMessageKind::ToRivetWebSocketClose(
				convert_to_rivet_web_socket_close_v2_to_v1(v)?,
			)
		}
	})
}

pub fn convert_to_rivet_tunnel_message_v2_to_v1(
	x: v2::ToRivetTunnelMessage,
) -> Result<v1::ToRivetTunnelMessage> {
	Ok(v1::ToRivetTunnelMessage {
		message_id: convert_message_id_v2_to_v1(x.message_id)?,
		message_kind: convert_to_rivet_tunnel_message_kind_v2_to_v1(x.message_kind)?,
	})
}

pub fn convert_to_envoy_tunnel_message_kind_v2_to_v1(
	x: v2::ToEnvoyTunnelMessageKind,
) -> Result<v1::ToEnvoyTunnelMessageKind> {
	Ok(match x {
		v2::ToEnvoyTunnelMessageKind::ToEnvoyRequestStart(v) => {
			v1::ToEnvoyTunnelMessageKind::ToEnvoyRequestStart(
				convert_to_envoy_request_start_v2_to_v1(v)?,
			)
		}
		v2::ToEnvoyTunnelMessageKind::ToEnvoyRequestChunk(v) => {
			v1::ToEnvoyTunnelMessageKind::ToEnvoyRequestChunk(
				convert_to_envoy_request_chunk_v2_to_v1(v)?,
			)
		}
		v2::ToEnvoyTunnelMessageKind::ToEnvoyRequestAbort => {
			v1::ToEnvoyTunnelMessageKind::ToEnvoyRequestAbort
		}
		v2::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketOpen(v) => {
			v1::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketOpen(
				convert_to_envoy_web_socket_open_v2_to_v1(v)?,
			)
		}
		v2::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketMessage(v) => {
			v1::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketMessage(
				convert_to_envoy_web_socket_message_v2_to_v1(v)?,
			)
		}
		v2::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketClose(v) => {
			v1::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketClose(
				convert_to_envoy_web_socket_close_v2_to_v1(v)?,
			)
		}
	})
}

pub fn convert_to_envoy_tunnel_message_v2_to_v1(
	x: v2::ToEnvoyTunnelMessage,
) -> Result<v1::ToEnvoyTunnelMessage> {
	Ok(v1::ToEnvoyTunnelMessage {
		message_id: convert_message_id_v2_to_v1(x.message_id)?,
		message_kind: convert_to_envoy_tunnel_message_kind_v2_to_v1(x.message_kind)?,
	})
}

pub fn convert_to_envoy_ping_v2_to_v1(x: v2::ToEnvoyPing) -> Result<v1::ToEnvoyPing> {
	Ok(v1::ToEnvoyPing { ts: x.ts })
}

pub fn convert_to_rivet_metadata_v2_to_v1(x: v2::ToRivetMetadata) -> Result<v1::ToRivetMetadata> {
	Ok(v1::ToRivetMetadata {
		prepopulate_actor_names: x
			.prepopulate_actor_names
			.map(|v| {
				v.into_iter()
					.map(|(k, v)| -> Result<_> { Ok((k, convert_actor_name_v2_to_v1(v)?)) })
					.collect::<Result<_>>()
			})
			.transpose()?,
		metadata: x.metadata,
	})
}

pub fn convert_to_rivet_events_v2_to_v1(x: v2::ToRivetEvents) -> Result<v1::ToRivetEvents> {
	Ok(x.into_iter()
		.map(|v| convert_event_wrapper_v2_to_v1(v))
		.collect::<Result<Vec<_>>>()?)
}

pub fn convert_to_rivet_ack_commands_v2_to_v1(
	x: v2::ToRivetAckCommands,
) -> Result<v1::ToRivetAckCommands> {
	Ok(v1::ToRivetAckCommands {
		last_command_checkpoints: x
			.last_command_checkpoints
			.into_iter()
			.map(|v| convert_actor_checkpoint_v2_to_v1(v))
			.collect::<Result<Vec<_>>>()?,
	})
}

pub fn convert_to_rivet_pong_v2_to_v1(x: v2::ToRivetPong) -> Result<v1::ToRivetPong> {
	Ok(v1::ToRivetPong { ts: x.ts })
}

pub fn convert_to_rivet_kv_request_v2_to_v1(
	x: v2::ToRivetKvRequest,
) -> Result<v1::ToRivetKvRequest> {
	Ok(v1::ToRivetKvRequest {
		actor_id: x.actor_id,
		request_id: x.request_id,
		data: convert_kv_request_data_v2_to_v1(x.data)?,
	})
}

pub fn convert_to_rivet_v2_to_v1(x: v2::ToRivet) -> Result<v1::ToRivet> {
	Ok(match x {
		v2::ToRivet::ToRivetMetadata(v) => {
			v1::ToRivet::ToRivetMetadata(convert_to_rivet_metadata_v2_to_v1(v)?)
		}
		v2::ToRivet::ToRivetEvents(v) => {
			v1::ToRivet::ToRivetEvents(convert_to_rivet_events_v2_to_v1(v)?)
		}
		v2::ToRivet::ToRivetAckCommands(v) => {
			v1::ToRivet::ToRivetAckCommands(convert_to_rivet_ack_commands_v2_to_v1(v)?)
		}
		v2::ToRivet::ToRivetStopping => v1::ToRivet::ToRivetStopping,
		v2::ToRivet::ToRivetPong(v) => v1::ToRivet::ToRivetPong(convert_to_rivet_pong_v2_to_v1(v)?),
		v2::ToRivet::ToRivetKvRequest(v) => {
			v1::ToRivet::ToRivetKvRequest(convert_to_rivet_kv_request_v2_to_v1(v)?)
		}
		v2::ToRivet::ToRivetTunnelMessage(v) => {
			v1::ToRivet::ToRivetTunnelMessage(convert_to_rivet_tunnel_message_v2_to_v1(v)?)
		}
		v2::ToRivet::ToRivetSqliteGetPagesRequest(_)
		| v2::ToRivet::ToRivetSqliteCommitRequest(_)
		| v2::ToRivet::ToRivetSqliteCommitStageBeginRequest(_)
		| v2::ToRivet::ToRivetSqliteCommitStageRequest(_)
		| v2::ToRivet::ToRivetSqliteCommitFinalizeRequest(_) => {
			bail!("sqlite requests require envoy-protocol v2")
		}
	})
}

pub fn convert_protocol_metadata_v2_to_v1(x: v2::ProtocolMetadata) -> Result<v1::ProtocolMetadata> {
	Ok(v1::ProtocolMetadata {
		envoy_lost_threshold: x.envoy_lost_threshold,
		actor_stop_threshold: x.actor_stop_threshold,
		max_response_payload_size: x.max_response_payload_size,
	})
}

pub fn convert_to_envoy_init_v2_to_v1(x: v2::ToEnvoyInit) -> Result<v1::ToEnvoyInit> {
	Ok(v1::ToEnvoyInit {
		metadata: convert_protocol_metadata_v2_to_v1(x.metadata)?,
	})
}

pub fn convert_to_envoy_commands_v2_to_v1(x: v2::ToEnvoyCommands) -> Result<v1::ToEnvoyCommands> {
	Ok(x.into_iter()
		.map(|v| convert_command_wrapper_v2_to_v1(v))
		.collect::<Result<Vec<_>>>()?)
}

pub fn convert_to_envoy_ack_events_v2_to_v1(
	x: v2::ToEnvoyAckEvents,
) -> Result<v1::ToEnvoyAckEvents> {
	Ok(v1::ToEnvoyAckEvents {
		last_event_checkpoints: x
			.last_event_checkpoints
			.into_iter()
			.map(|v| convert_actor_checkpoint_v2_to_v1(v))
			.collect::<Result<Vec<_>>>()?,
	})
}

pub fn convert_to_envoy_kv_response_v2_to_v1(
	x: v2::ToEnvoyKvResponse,
) -> Result<v1::ToEnvoyKvResponse> {
	Ok(v1::ToEnvoyKvResponse {
		request_id: x.request_id,
		data: convert_kv_response_data_v2_to_v1(x.data)?,
	})
}

pub fn convert_to_envoy_v2_to_v1(x: v2::ToEnvoy) -> Result<v1::ToEnvoy> {
	Ok(match x {
		v2::ToEnvoy::ToEnvoyInit(v) => v1::ToEnvoy::ToEnvoyInit(convert_to_envoy_init_v2_to_v1(v)?),
		v2::ToEnvoy::ToEnvoyCommands(v) => {
			v1::ToEnvoy::ToEnvoyCommands(convert_to_envoy_commands_v2_to_v1(v)?)
		}
		v2::ToEnvoy::ToEnvoyAckEvents(v) => {
			v1::ToEnvoy::ToEnvoyAckEvents(convert_to_envoy_ack_events_v2_to_v1(v)?)
		}
		v2::ToEnvoy::ToEnvoyKvResponse(v) => {
			v1::ToEnvoy::ToEnvoyKvResponse(convert_to_envoy_kv_response_v2_to_v1(v)?)
		}
		v2::ToEnvoy::ToEnvoyTunnelMessage(v) => {
			v1::ToEnvoy::ToEnvoyTunnelMessage(convert_to_envoy_tunnel_message_v2_to_v1(v)?)
		}
		v2::ToEnvoy::ToEnvoyPing(v) => v1::ToEnvoy::ToEnvoyPing(convert_to_envoy_ping_v2_to_v1(v)?),
		v2::ToEnvoy::ToEnvoySqliteGetPagesResponse(_)
		| v2::ToEnvoy::ToEnvoySqliteCommitResponse(_)
		| v2::ToEnvoy::ToEnvoySqliteCommitStageBeginResponse(_)
		| v2::ToEnvoy::ToEnvoySqliteCommitStageResponse(_)
		| v2::ToEnvoy::ToEnvoySqliteCommitFinalizeResponse(_) => {
			bail!("sqlite responses require envoy-protocol v2")
		}
	})
}

pub fn convert_to_envoy_conn_ping_v2_to_v1(x: v2::ToEnvoyConnPing) -> Result<v1::ToEnvoyConnPing> {
	Ok(v1::ToEnvoyConnPing {
		gateway_id: x.gateway_id,
		request_id: x.request_id,
		ts: x.ts,
	})
}

pub fn convert_to_envoy_conn_v2_to_v1(x: v2::ToEnvoyConn) -> Result<v1::ToEnvoyConn> {
	Ok(match x {
		v2::ToEnvoyConn::ToEnvoyConnPing(v) => {
			v1::ToEnvoyConn::ToEnvoyConnPing(convert_to_envoy_conn_ping_v2_to_v1(v)?)
		}
		v2::ToEnvoyConn::ToEnvoyConnClose => v1::ToEnvoyConn::ToEnvoyConnClose,
		v2::ToEnvoyConn::ToEnvoyCommands(v) => {
			v1::ToEnvoyConn::ToEnvoyCommands(convert_to_envoy_commands_v2_to_v1(v)?)
		}
		v2::ToEnvoyConn::ToEnvoyAckEvents(v) => {
			v1::ToEnvoyConn::ToEnvoyAckEvents(convert_to_envoy_ack_events_v2_to_v1(v)?)
		}
		v2::ToEnvoyConn::ToEnvoyTunnelMessage(v) => {
			v1::ToEnvoyConn::ToEnvoyTunnelMessage(convert_to_envoy_tunnel_message_v2_to_v1(v)?)
		}
	})
}

pub fn convert_to_gateway_pong_v2_to_v1(x: v2::ToGatewayPong) -> Result<v1::ToGatewayPong> {
	Ok(v1::ToGatewayPong {
		request_id: x.request_id,
		ts: x.ts,
	})
}

pub fn convert_to_gateway_v2_to_v1(x: v2::ToGateway) -> Result<v1::ToGateway> {
	Ok(match x {
		v2::ToGateway::ToGatewayPong(v) => {
			v1::ToGateway::ToGatewayPong(convert_to_gateway_pong_v2_to_v1(v)?)
		}
		v2::ToGateway::ToRivetTunnelMessage(v) => {
			v1::ToGateway::ToRivetTunnelMessage(convert_to_rivet_tunnel_message_v2_to_v1(v)?)
		}
	})
}

pub fn convert_to_outbound_actor_start_v2_to_v1(
	x: v2::ToOutboundActorStart,
) -> Result<v1::ToOutboundActorStart> {
	Ok(v1::ToOutboundActorStart {
		namespace_id: x.namespace_id,
		pool_name: x.pool_name,
		checkpoint: convert_actor_checkpoint_v2_to_v1(x.checkpoint)?,
		actor_config: convert_actor_config_v2_to_v1(x.actor_config)?,
	})
}

pub fn convert_to_outbound_v2_to_v1(x: v2::ToOutbound) -> Result<v1::ToOutbound> {
	Ok(match x {
		v2::ToOutbound::ToOutboundActorStart(v) => {
			v1::ToOutbound::ToOutboundActorStart(convert_to_outbound_actor_start_v2_to_v1(v)?)
		}
	})
}
