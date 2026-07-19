// from: v6.bare, to: v5.bare

#![allow(dead_code, unused_variables)]

use anyhow::Result;

use crate::generated::{v6, v5};
use super::{
	ProtocolCompatibilityDirection, ProtocolCompatibilityFeature, incompatible,
};

pub fn convert_kv_metadata_v6_to_v5(x: v6::KvMetadata) -> Result<v5::KvMetadata> {
	Ok(v5::KvMetadata {
		version: x.version,
		update_ts: x.update_ts,
	})
}

pub fn convert_kv_list_range_query_v6_to_v5(x: v6::KvListRangeQuery) -> Result<v5::KvListRangeQuery> {
	Ok(v5::KvListRangeQuery {
		start: x.start,
		end: x.end,
		exclusive: x.exclusive,
	})
}

pub fn convert_kv_list_prefix_query_v6_to_v5(x: v6::KvListPrefixQuery) -> Result<v5::KvListPrefixQuery> {
	Ok(v5::KvListPrefixQuery {
		key: x.key,
	})
}

pub fn convert_kv_list_query_v6_to_v5(x: v6::KvListQuery) -> Result<v5::KvListQuery> {
	Ok(match x {
		v6::KvListQuery::KvListAllQuery => v5::KvListQuery::KvListAllQuery,
		v6::KvListQuery::KvListRangeQuery(v) => v5::KvListQuery::KvListRangeQuery(convert_kv_list_range_query_v6_to_v5(v)?),
		v6::KvListQuery::KvListPrefixQuery(v) => v5::KvListQuery::KvListPrefixQuery(convert_kv_list_prefix_query_v6_to_v5(v)?),
	})
}

pub fn convert_kv_get_request_v6_to_v5(x: v6::KvGetRequest) -> Result<v5::KvGetRequest> {
	Ok(v5::KvGetRequest {
		keys: x.keys,
	})
}

pub fn convert_kv_list_request_v6_to_v5(x: v6::KvListRequest) -> Result<v5::KvListRequest> {
	Ok(v5::KvListRequest {
		query: convert_kv_list_query_v6_to_v5(x.query)?,
		reverse: x.reverse,
		limit: x.limit,
	})
}

pub fn convert_kv_put_request_v6_to_v5(x: v6::KvPutRequest) -> Result<v5::KvPutRequest> {
	Ok(v5::KvPutRequest {
		keys: x.keys,
		values: x.values,
	})
}

pub fn convert_kv_delete_request_v6_to_v5(x: v6::KvDeleteRequest) -> Result<v5::KvDeleteRequest> {
	Ok(v5::KvDeleteRequest {
		keys: x.keys,
	})
}

pub fn convert_kv_delete_range_request_v6_to_v5(x: v6::KvDeleteRangeRequest) -> Result<v5::KvDeleteRangeRequest> {
	Ok(v5::KvDeleteRangeRequest {
		start: x.start,
		end: x.end,
	})
}

pub fn convert_kv_error_response_v6_to_v5(x: v6::KvErrorResponse) -> Result<v5::KvErrorResponse> {
	Ok(v5::KvErrorResponse {
		message: x.message,
	})
}

pub fn convert_kv_get_response_v6_to_v5(x: v6::KvGetResponse) -> Result<v5::KvGetResponse> {
	Ok(v5::KvGetResponse {
		keys: x.keys,
		values: x.values,
		metadata: x.metadata.into_iter().map(|v| convert_kv_metadata_v6_to_v5(v)).collect::<Result<Vec<_>>>()?,
	})
}

pub fn convert_kv_list_response_v6_to_v5(x: v6::KvListResponse) -> Result<v5::KvListResponse> {
	Ok(v5::KvListResponse {
		keys: x.keys,
		values: x.values,
		metadata: x.metadata.into_iter().map(|v| convert_kv_metadata_v6_to_v5(v)).collect::<Result<Vec<_>>>()?,
	})
}

pub fn convert_kv_request_data_v6_to_v5(x: v6::KvRequestData) -> Result<v5::KvRequestData> {
	Ok(match x {
		v6::KvRequestData::KvGetRequest(v) => v5::KvRequestData::KvGetRequest(convert_kv_get_request_v6_to_v5(v)?),
		v6::KvRequestData::KvListRequest(v) => v5::KvRequestData::KvListRequest(convert_kv_list_request_v6_to_v5(v)?),
		v6::KvRequestData::KvPutRequest(v) => v5::KvRequestData::KvPutRequest(convert_kv_put_request_v6_to_v5(v)?),
		v6::KvRequestData::KvDeleteRequest(v) => v5::KvRequestData::KvDeleteRequest(convert_kv_delete_request_v6_to_v5(v)?),
		v6::KvRequestData::KvDeleteRangeRequest(v) => v5::KvRequestData::KvDeleteRangeRequest(convert_kv_delete_range_request_v6_to_v5(v)?),
		v6::KvRequestData::KvDropRequest => v5::KvRequestData::KvDropRequest,
	})
}

pub fn convert_kv_response_data_v6_to_v5(x: v6::KvResponseData) -> Result<v5::KvResponseData> {
	Ok(match x {
		v6::KvResponseData::KvErrorResponse(v) => v5::KvResponseData::KvErrorResponse(convert_kv_error_response_v6_to_v5(v)?),
		v6::KvResponseData::KvGetResponse(v) => v5::KvResponseData::KvGetResponse(convert_kv_get_response_v6_to_v5(v)?),
		v6::KvResponseData::KvListResponse(v) => v5::KvResponseData::KvListResponse(convert_kv_list_response_v6_to_v5(v)?),
		v6::KvResponseData::KvPutResponse => v5::KvResponseData::KvPutResponse,
		v6::KvResponseData::KvDeleteResponse => v5::KvResponseData::KvDeleteResponse,
		v6::KvResponseData::KvDropResponse => v5::KvResponseData::KvDropResponse,
	})
}

pub fn convert_sqlite_dirty_page_v6_to_v5(x: v6::SqliteDirtyPage) -> Result<v5::SqliteDirtyPage> {
	Ok(v5::SqliteDirtyPage {
		pgno: x.pgno,
		bytes: x.bytes,
	})
}

pub fn convert_sqlite_fetched_page_v6_to_v5(x: v6::SqliteFetchedPage) -> Result<v5::SqliteFetchedPage> {
	Ok(v5::SqliteFetchedPage {
		pgno: x.pgno,
		bytes: x.bytes,
	})
}

pub fn convert_sqlite_get_pages_request_v6_to_v5(x: v6::SqliteGetPagesRequest) -> Result<v5::SqliteGetPagesRequest> {
	Ok(v5::SqliteGetPagesRequest {
		actor_id: x.actor_id,
		pgnos: x.pgnos,
		expected_generation: x.expected_generation,
		expected_head_txid: x.expected_head_txid,
	})
}

pub fn convert_sqlite_get_pages_ok_v6_to_v5(x: v6::SqliteGetPagesOk) -> Result<v5::SqliteGetPagesOk> {
	Ok(v5::SqliteGetPagesOk {
		pages: x.pages.into_iter().map(|v| convert_sqlite_fetched_page_v6_to_v5(v)).collect::<Result<Vec<_>>>()?,
		head_txid: x.head_txid,
	})
}

pub fn convert_sqlite_error_response_v6_to_v5(x: v6::SqliteErrorResponse) -> Result<v5::SqliteErrorResponse> {
	Ok(v5::SqliteErrorResponse {
		group: x.group,
		code: x.code,
		message: x.message,
	})
}

pub fn convert_sqlite_get_pages_response_v6_to_v5(x: v6::SqliteGetPagesResponse) -> Result<v5::SqliteGetPagesResponse> {
	Ok(match x {
		v6::SqliteGetPagesResponse::SqliteGetPagesOk(v) => v5::SqliteGetPagesResponse::SqliteGetPagesOk(convert_sqlite_get_pages_ok_v6_to_v5(v)?),
		v6::SqliteGetPagesResponse::SqliteErrorResponse(v) => v5::SqliteGetPagesResponse::SqliteErrorResponse(convert_sqlite_error_response_v6_to_v5(v)?),
	})
}

pub fn convert_sqlite_commit_request_v6_to_v5(x: v6::SqliteCommitRequest) -> Result<v5::SqliteCommitRequest> {
	Ok(v5::SqliteCommitRequest {
		actor_id: x.actor_id,
		dirty_pages: x.dirty_pages.into_iter().map(|v| convert_sqlite_dirty_page_v6_to_v5(v)).collect::<Result<Vec<_>>>()?,
		db_size_pages: x.db_size_pages,
		now_ms: x.now_ms,
		expected_generation: x.expected_generation,
		expected_head_txid: x.expected_head_txid,
	})
}

pub fn convert_sqlite_commit_ok_v6_to_v5(x: v6::SqliteCommitOk) -> Result<v5::SqliteCommitOk> {
	Ok(v5::SqliteCommitOk {
		head_txid: x.head_txid,
	})
}

pub fn convert_sqlite_commit_response_v6_to_v5(x: v6::SqliteCommitResponse) -> Result<v5::SqliteCommitResponse> {
	Ok(match x {
		v6::SqliteCommitResponse::SqliteCommitOk(v) => v5::SqliteCommitResponse::SqliteCommitOk(convert_sqlite_commit_ok_v6_to_v5(v)?),
		v6::SqliteCommitResponse::SqliteErrorResponse(v) => v5::SqliteCommitResponse::SqliteErrorResponse(convert_sqlite_error_response_v6_to_v5(v)?),
	})
}

pub fn convert_sqlite_value_integer_v6_to_v5(x: v6::SqliteValueInteger) -> Result<v5::SqliteValueInteger> {
	Ok(v5::SqliteValueInteger {
		value: x.value,
	})
}

pub fn convert_sqlite_value_float_v6_to_v5(x: v6::SqliteValueFloat) -> Result<v5::SqliteValueFloat> {
	Ok(v5::SqliteValueFloat {
		value: x.value,
	})
}

pub fn convert_sqlite_value_text_v6_to_v5(x: v6::SqliteValueText) -> Result<v5::SqliteValueText> {
	Ok(v5::SqliteValueText {
		value: x.value,
	})
}

pub fn convert_sqlite_value_blob_v6_to_v5(x: v6::SqliteValueBlob) -> Result<v5::SqliteValueBlob> {
	Ok(v5::SqliteValueBlob {
		value: x.value,
	})
}

pub fn convert_sqlite_bind_param_v6_to_v5(x: v6::SqliteBindParam) -> Result<v5::SqliteBindParam> {
	Ok(match x {
		v6::SqliteBindParam::SqliteValueNull => v5::SqliteBindParam::SqliteValueNull,
		v6::SqliteBindParam::SqliteValueInteger(v) => v5::SqliteBindParam::SqliteValueInteger(convert_sqlite_value_integer_v6_to_v5(v)?),
		v6::SqliteBindParam::SqliteValueFloat(v) => v5::SqliteBindParam::SqliteValueFloat(convert_sqlite_value_float_v6_to_v5(v)?),
		v6::SqliteBindParam::SqliteValueText(v) => v5::SqliteBindParam::SqliteValueText(convert_sqlite_value_text_v6_to_v5(v)?),
		v6::SqliteBindParam::SqliteValueBlob(v) => v5::SqliteBindParam::SqliteValueBlob(convert_sqlite_value_blob_v6_to_v5(v)?),
	})
}

pub fn convert_sqlite_column_value_v6_to_v5(x: v6::SqliteColumnValue) -> Result<v5::SqliteColumnValue> {
	Ok(match x {
		v6::SqliteColumnValue::SqliteValueNull => v5::SqliteColumnValue::SqliteValueNull,
		v6::SqliteColumnValue::SqliteValueInteger(v) => v5::SqliteColumnValue::SqliteValueInteger(convert_sqlite_value_integer_v6_to_v5(v)?),
		v6::SqliteColumnValue::SqliteValueFloat(v) => v5::SqliteColumnValue::SqliteValueFloat(convert_sqlite_value_float_v6_to_v5(v)?),
		v6::SqliteColumnValue::SqliteValueText(v) => v5::SqliteColumnValue::SqliteValueText(convert_sqlite_value_text_v6_to_v5(v)?),
		v6::SqliteColumnValue::SqliteValueBlob(v) => v5::SqliteColumnValue::SqliteValueBlob(convert_sqlite_value_blob_v6_to_v5(v)?),
	})
}

pub fn convert_sqlite_query_result_v6_to_v5(x: v6::SqliteQueryResult) -> Result<v5::SqliteQueryResult> {
	Ok(v5::SqliteQueryResult {
		columns: x.columns,
		rows: x.rows.into_iter().map(|v| v.into_iter().map(|v| convert_sqlite_column_value_v6_to_v5(v)).collect::<Result<Vec<_>>>()).collect::<Result<Vec<_>>>()?,
	})
}

pub fn convert_sqlite_execute_result_v6_to_v5(x: v6::SqliteExecuteResult) -> Result<v5::SqliteExecuteResult> {
	Ok(v5::SqliteExecuteResult {
		columns: x.columns,
		rows: x.rows.into_iter().map(|v| v.into_iter().map(|v| convert_sqlite_column_value_v6_to_v5(v)).collect::<Result<Vec<_>>>()).collect::<Result<Vec<_>>>()?,
		changes: x.changes,
		last_insert_row_id: x.last_insert_row_id,
	})
}

pub fn convert_sqlite_exec_request_v6_to_v5(x: v6::SqliteExecRequest) -> Result<v5::SqliteExecRequest> {
	Ok(v5::SqliteExecRequest {
		namespace_id: x.namespace_id,
		actor_id: x.actor_id,
		generation: x.generation,
		sql: x.sql,
	})
}

pub fn convert_sqlite_execute_request_v6_to_v5(x: v6::SqliteExecuteRequest) -> Result<v5::SqliteExecuteRequest> {
	Ok(v5::SqliteExecuteRequest {
		namespace_id: x.namespace_id,
		actor_id: x.actor_id,
		generation: x.generation,
		sql: x.sql,
		params: x.params.map(|v| v.into_iter().map(|v| convert_sqlite_bind_param_v6_to_v5(v)).collect::<Result<Vec<_>>>()).transpose()?,
	})
}

pub fn convert_sqlite_exec_ok_v6_to_v5(x: v6::SqliteExecOk) -> Result<v5::SqliteExecOk> {
	Ok(v5::SqliteExecOk {
		result: convert_sqlite_query_result_v6_to_v5(x.result)?,
	})
}

pub fn convert_sqlite_execute_ok_v6_to_v5(x: v6::SqliteExecuteOk) -> Result<v5::SqliteExecuteOk> {
	Ok(v5::SqliteExecuteOk {
		result: convert_sqlite_execute_result_v6_to_v5(x.result)?,
	})
}

pub fn convert_sqlite_exec_response_v6_to_v5(x: v6::SqliteExecResponse) -> Result<v5::SqliteExecResponse> {
	Ok(match x {
		v6::SqliteExecResponse::SqliteExecOk(v) => v5::SqliteExecResponse::SqliteExecOk(convert_sqlite_exec_ok_v6_to_v5(v)?),
		v6::SqliteExecResponse::SqliteErrorResponse(v) => v5::SqliteExecResponse::SqliteErrorResponse(convert_sqlite_error_response_v6_to_v5(v)?),
	})
}

pub fn convert_sqlite_execute_response_v6_to_v5(x: v6::SqliteExecuteResponse) -> Result<v5::SqliteExecuteResponse> {
	Ok(match x {
		v6::SqliteExecuteResponse::SqliteExecuteOk(v) => v5::SqliteExecuteResponse::SqliteExecuteOk(convert_sqlite_execute_ok_v6_to_v5(v)?),
		v6::SqliteExecuteResponse::SqliteErrorResponse(v) => v5::SqliteExecuteResponse::SqliteErrorResponse(convert_sqlite_error_response_v6_to_v5(v)?),
	})
}

pub fn convert_stop_code_v6_to_v5(x: v6::StopCode) -> Result<v5::StopCode> {
	Ok(match x {
		v6::StopCode::Ok => v5::StopCode::Ok,
		v6::StopCode::Error => v5::StopCode::Error,
	})
}

pub fn convert_actor_name_v6_to_v5(x: v6::ActorName) -> Result<v5::ActorName> {
	Ok(v5::ActorName {
		metadata: x.metadata,
	})
}

pub fn convert_actor_config_v6_to_v5(x: v6::ActorConfig) -> Result<v5::ActorConfig> {
	Ok(v5::ActorConfig {
		name: x.name,
		key: x.key,
		create_ts: x.create_ts,
		input: x.input,
	})
}

pub fn convert_actor_checkpoint_v6_to_v5(x: v6::ActorCheckpoint) -> Result<v5::ActorCheckpoint> {
	Ok(v5::ActorCheckpoint {
		actor_id: x.actor_id,
		generation: x.generation,
		index: x.index,
	})
}

pub fn convert_actor_intent_v6_to_v5(x: v6::ActorIntent) -> Result<v5::ActorIntent> {
	Ok(match x {
		v6::ActorIntent::ActorIntentSleep => v5::ActorIntent::ActorIntentSleep,
		v6::ActorIntent::ActorIntentStop => v5::ActorIntent::ActorIntentStop,
	})
}

pub fn convert_actor_state_stopped_v6_to_v5(x: v6::ActorStateStopped) -> Result<v5::ActorStateStopped> {
	Ok(v5::ActorStateStopped {
		code: convert_stop_code_v6_to_v5(x.code)?,
		message: x.message,
	})
}

pub fn convert_actor_state_v6_to_v5(x: v6::ActorState) -> Result<v5::ActorState> {
	Ok(match x {
		v6::ActorState::ActorStateRunning => v5::ActorState::ActorStateRunning,
		v6::ActorState::ActorStateStopped(v) => v5::ActorState::ActorStateStopped(convert_actor_state_stopped_v6_to_v5(v)?),
	})
}

pub fn convert_event_actor_intent_v6_to_v5(x: v6::EventActorIntent) -> Result<v5::EventActorIntent> {
	Ok(v5::EventActorIntent {
		intent: convert_actor_intent_v6_to_v5(x.intent)?,
	})
}

pub fn convert_event_actor_state_update_v6_to_v5(x: v6::EventActorStateUpdate) -> Result<v5::EventActorStateUpdate> {
	Ok(v5::EventActorStateUpdate {
		state: convert_actor_state_v6_to_v5(x.state)?,
	})
}

pub fn convert_event_actor_set_alarm_v6_to_v5(x: v6::EventActorSetAlarm) -> Result<v5::EventActorSetAlarm> {
	Ok(v5::EventActorSetAlarm {
		alarm_ts: x.alarm_ts,
	})
}

pub fn convert_event_v6_to_v5(x: v6::Event) -> Result<v5::Event> {
	Ok(match x {
		v6::Event::EventActorIntent(v) => v5::Event::EventActorIntent(convert_event_actor_intent_v6_to_v5(v)?),
		v6::Event::EventActorStateUpdate(v) => v5::Event::EventActorStateUpdate(convert_event_actor_state_update_v6_to_v5(v)?),
		v6::Event::EventActorSetAlarm(v) => v5::Event::EventActorSetAlarm(convert_event_actor_set_alarm_v6_to_v5(v)?),
	})
}

pub fn convert_event_wrapper_v6_to_v5(x: v6::EventWrapper) -> Result<v5::EventWrapper> {
	Ok(v5::EventWrapper {
		checkpoint: convert_actor_checkpoint_v6_to_v5(x.checkpoint)?,
		inner: convert_event_v6_to_v5(x.inner)?,
	})
}

pub fn convert_preloaded_kv_entry_v6_to_v5(x: v6::PreloadedKvEntry) -> Result<v5::PreloadedKvEntry> {
	Ok(v5::PreloadedKvEntry {
		key: x.key,
		value: x.value,
		metadata: convert_kv_metadata_v6_to_v5(x.metadata)?,
	})
}

pub fn convert_preloaded_kv_v6_to_v5(x: v6::PreloadedKv) -> Result<v5::PreloadedKv> {
	Ok(v5::PreloadedKv {
		entries: x.entries.into_iter().map(|v| convert_preloaded_kv_entry_v6_to_v5(v)).collect::<Result<Vec<_>>>()?,
		requested_get_keys: x.requested_get_keys,
		requested_prefixes: x.requested_prefixes,
	})
}

pub fn convert_hibernating_request_v6_to_v5(x: v6::HibernatingRequest) -> Result<v5::HibernatingRequest> {
	Ok(v5::HibernatingRequest {
		gateway_id: x.gateway_id,
		request_id: x.request_id,
	})
}

pub fn convert_command_start_actor_v6_to_v5(x: v6::CommandStartActor) -> Result<v5::CommandStartActor> {
	Ok(v5::CommandStartActor {
		config: convert_actor_config_v6_to_v5(x.config)?,
		hibernating_requests: x.hibernating_requests.into_iter().map(|v| convert_hibernating_request_v6_to_v5(v)).collect::<Result<Vec<_>>>()?,
		preloaded_kv: x.preloaded_kv.map(|v| convert_preloaded_kv_v6_to_v5(v)).transpose()?,
	})
}

pub fn convert_stop_actor_reason_v6_to_v5(x: v6::StopActorReason) -> Result<v5::StopActorReason> {
	Ok(match x {
		v6::StopActorReason::SleepIntent => v5::StopActorReason::SleepIntent,
		v6::StopActorReason::StopIntent => v5::StopActorReason::StopIntent,
		v6::StopActorReason::Destroy => v5::StopActorReason::Destroy,
		v6::StopActorReason::GoingAway => v5::StopActorReason::GoingAway,
		v6::StopActorReason::Lost => v5::StopActorReason::Lost,
	})
}

pub fn convert_command_stop_actor_v6_to_v5(x: v6::CommandStopActor) -> Result<v5::CommandStopActor> {
	Ok(v5::CommandStopActor {
		reason: convert_stop_actor_reason_v6_to_v5(x.reason)?,
	})
}

pub fn convert_command_v6_to_v5(x: v6::Command) -> Result<v5::Command> {
	Ok(match x {
		v6::Command::CommandStartActor(v) => v5::Command::CommandStartActor(convert_command_start_actor_v6_to_v5(v)?),
		v6::Command::CommandStopActor(v) => v5::Command::CommandStopActor(convert_command_stop_actor_v6_to_v5(v)?),
	})
}

pub fn convert_command_wrapper_v6_to_v5(x: v6::CommandWrapper) -> Result<v5::CommandWrapper> {
	Ok(v5::CommandWrapper {
		checkpoint: convert_actor_checkpoint_v6_to_v5(x.checkpoint)?,
		inner: convert_command_v6_to_v5(x.inner)?,
	})
}

pub fn convert_actor_command_key_data_v6_to_v5(x: v6::ActorCommandKeyData) -> Result<v5::ActorCommandKeyData> {
	Ok(match x {
		v6::ActorCommandKeyData::CommandStartActor(v) => v5::ActorCommandKeyData::CommandStartActor(convert_command_start_actor_v6_to_v5(v)?),
		v6::ActorCommandKeyData::CommandStopActor(v) => v5::ActorCommandKeyData::CommandStopActor(convert_command_stop_actor_v6_to_v5(v)?),
	})
}

pub fn convert_message_id_v6_to_v5(x: v6::MessageId) -> Result<v5::MessageId> {
	Ok(v5::MessageId {
		gateway_id: x.gateway_id,
		request_id: x.request_id,
		message_index: x.message_index,
	})
}

pub fn convert_to_envoy_request_start_v6_to_v5(x: v6::ToEnvoyRequestStart) -> Result<v5::ToEnvoyRequestStart> {
	Ok(v5::ToEnvoyRequestStart {
		actor_id: x.actor_id,
		method: x.method,
		path: x.path,
		headers: x.headers,
		body: x.body,
		stream: x.stream,
	})
}

pub fn convert_to_envoy_request_chunk_v6_to_v5(x: v6::ToEnvoyRequestChunk) -> Result<v5::ToEnvoyRequestChunk> {
	Ok(v5::ToEnvoyRequestChunk {
		body: x.body,
		finish: x.finish,
	})
}

pub fn convert_to_rivet_response_start_v6_to_v5(x: v6::ToRivetResponseStart) -> Result<v5::ToRivetResponseStart> {
	Ok(v5::ToRivetResponseStart {
		status: x.status,
		headers: x.headers,
		body: x.body,
		stream: x.stream,
	})
}

pub fn convert_to_rivet_response_chunk_v6_to_v5(x: v6::ToRivetResponseChunk) -> Result<v5::ToRivetResponseChunk> {
	Ok(v5::ToRivetResponseChunk {
		body: x.body,
		finish: x.finish,
	})
}

pub fn convert_to_envoy_web_socket_open_v6_to_v5(x: v6::ToEnvoyWebSocketOpen) -> Result<v5::ToEnvoyWebSocketOpen> {
	Ok(v5::ToEnvoyWebSocketOpen {
		actor_id: x.actor_id,
		path: x.path,
		headers: x.headers,
	})
}

pub fn convert_to_envoy_web_socket_message_v6_to_v5(x: v6::ToEnvoyWebSocketMessage) -> Result<v5::ToEnvoyWebSocketMessage> {
	Ok(v5::ToEnvoyWebSocketMessage {
		data: x.data,
		binary: x.binary,
	})
}

pub fn convert_to_envoy_web_socket_close_v6_to_v5(x: v6::ToEnvoyWebSocketClose) -> Result<v5::ToEnvoyWebSocketClose> {
	Ok(v5::ToEnvoyWebSocketClose {
		code: x.code,
		reason: x.reason,
	})
}

pub fn convert_to_rivet_web_socket_open_v6_to_v5(x: v6::ToRivetWebSocketOpen) -> Result<v5::ToRivetWebSocketOpen> {
	Ok(v5::ToRivetWebSocketOpen {
		can_hibernate: x.can_hibernate,
	})
}

pub fn convert_to_rivet_web_socket_message_v6_to_v5(x: v6::ToRivetWebSocketMessage) -> Result<v5::ToRivetWebSocketMessage> {
	Ok(v5::ToRivetWebSocketMessage {
		data: x.data,
		binary: x.binary,
	})
}

pub fn convert_to_rivet_web_socket_message_ack_v6_to_v5(x: v6::ToRivetWebSocketMessageAck) -> Result<v5::ToRivetWebSocketMessageAck> {
	Ok(v5::ToRivetWebSocketMessageAck {
		index: x.index,
	})
}

pub fn convert_to_rivet_web_socket_close_v6_to_v5(x: v6::ToRivetWebSocketClose) -> Result<v5::ToRivetWebSocketClose> {
	Ok(v5::ToRivetWebSocketClose {
		code: x.code,
		reason: x.reason,
		hibernate: x.hibernate,
	})
}

pub fn convert_to_rivet_tunnel_message_kind_v6_to_v5(x: v6::ToRivetTunnelMessageKind) -> Result<v5::ToRivetTunnelMessageKind> {
	Ok(match x {
		v6::ToRivetTunnelMessageKind::ToRivetResponseStart(v) => v5::ToRivetTunnelMessageKind::ToRivetResponseStart(convert_to_rivet_response_start_v6_to_v5(v)?),
		v6::ToRivetTunnelMessageKind::ToRivetResponseChunk(v) => v5::ToRivetTunnelMessageKind::ToRivetResponseChunk(convert_to_rivet_response_chunk_v6_to_v5(v)?),
		v6::ToRivetTunnelMessageKind::ToRivetResponseAbort => v5::ToRivetTunnelMessageKind::ToRivetResponseAbort,
		v6::ToRivetTunnelMessageKind::ToRivetWebSocketOpen(v) => v5::ToRivetTunnelMessageKind::ToRivetWebSocketOpen(convert_to_rivet_web_socket_open_v6_to_v5(v)?),
		v6::ToRivetTunnelMessageKind::ToRivetWebSocketMessage(v) => v5::ToRivetTunnelMessageKind::ToRivetWebSocketMessage(convert_to_rivet_web_socket_message_v6_to_v5(v)?),
		v6::ToRivetTunnelMessageKind::ToRivetWebSocketMessageAck(v) => v5::ToRivetTunnelMessageKind::ToRivetWebSocketMessageAck(convert_to_rivet_web_socket_message_ack_v6_to_v5(v)?),
		v6::ToRivetTunnelMessageKind::ToRivetWebSocketClose(v) => v5::ToRivetTunnelMessageKind::ToRivetWebSocketClose(convert_to_rivet_web_socket_close_v6_to_v5(v)?),
	})
}

pub fn convert_to_rivet_tunnel_message_v6_to_v5(x: v6::ToRivetTunnelMessage) -> Result<v5::ToRivetTunnelMessage> {
	Ok(v5::ToRivetTunnelMessage {
		message_id: convert_message_id_v6_to_v5(x.message_id)?,
		message_kind: convert_to_rivet_tunnel_message_kind_v6_to_v5(x.message_kind)?,
	})
}

pub fn convert_to_envoy_tunnel_message_kind_v6_to_v5(x: v6::ToEnvoyTunnelMessageKind) -> Result<v5::ToEnvoyTunnelMessageKind> {
	Ok(match x {
		v6::ToEnvoyTunnelMessageKind::ToEnvoyRequestStart(v) => v5::ToEnvoyTunnelMessageKind::ToEnvoyRequestStart(convert_to_envoy_request_start_v6_to_v5(v)?),
		v6::ToEnvoyTunnelMessageKind::ToEnvoyRequestChunk(v) => v5::ToEnvoyTunnelMessageKind::ToEnvoyRequestChunk(convert_to_envoy_request_chunk_v6_to_v5(v)?),
		v6::ToEnvoyTunnelMessageKind::ToEnvoyRequestAbort => v5::ToEnvoyTunnelMessageKind::ToEnvoyRequestAbort,
		v6::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketOpen(v) => v5::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketOpen(convert_to_envoy_web_socket_open_v6_to_v5(v)?),
		v6::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketMessage(v) => v5::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketMessage(convert_to_envoy_web_socket_message_v6_to_v5(v)?),
		v6::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketClose(v) => v5::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketClose(convert_to_envoy_web_socket_close_v6_to_v5(v)?),
	})
}

pub fn convert_to_envoy_tunnel_message_v6_to_v5(x: v6::ToEnvoyTunnelMessage) -> Result<v5::ToEnvoyTunnelMessage> {
	Ok(v5::ToEnvoyTunnelMessage {
		message_id: convert_message_id_v6_to_v5(x.message_id)?,
		message_kind: convert_to_envoy_tunnel_message_kind_v6_to_v5(x.message_kind)?,
	})
}

pub fn convert_to_envoy_ping_v6_to_v5(x: v6::ToEnvoyPing) -> Result<v5::ToEnvoyPing> {
	Ok(v5::ToEnvoyPing {
		ts: x.ts,
	})
}

pub fn convert_to_rivet_metadata_v6_to_v5(x: v6::ToRivetMetadata) -> Result<v5::ToRivetMetadata> {
	Ok(v5::ToRivetMetadata {
		prepopulate_actor_names: x.prepopulate_actor_names.map(|v| v.into_iter().map(|(k, v)| -> Result<_> { Ok((k, convert_actor_name_v6_to_v5(v)?)) }).collect::<Result<_>>()).transpose()?,
		metadata: x.metadata,
	})
}

pub fn convert_to_rivet_events_v6_to_v5(x: v6::ToRivetEvents) -> Result<v5::ToRivetEvents> {
	Ok(x.into_iter().map(|v| convert_event_wrapper_v6_to_v5(v)).collect::<Result<Vec<_>>>()?)
}

pub fn convert_to_rivet_ack_commands_v6_to_v5(x: v6::ToRivetAckCommands) -> Result<v5::ToRivetAckCommands> {
	Ok(v5::ToRivetAckCommands {
		last_command_checkpoints: x.last_command_checkpoints.into_iter().map(|v| convert_actor_checkpoint_v6_to_v5(v)).collect::<Result<Vec<_>>>()?,
	})
}

pub fn convert_to_rivet_pong_v6_to_v5(x: v6::ToRivetPong) -> Result<v5::ToRivetPong> {
	Ok(v5::ToRivetPong {
		ts: x.ts,
	})
}

pub fn convert_to_rivet_kv_request_v6_to_v5(x: v6::ToRivetKvRequest) -> Result<v5::ToRivetKvRequest> {
	Ok(v5::ToRivetKvRequest {
		actor_id: x.actor_id,
		request_id: x.request_id,
		data: convert_kv_request_data_v6_to_v5(x.data)?,
	})
}

pub fn convert_to_rivet_sqlite_get_pages_request_v6_to_v5(x: v6::ToRivetSqliteGetPagesRequest) -> Result<v5::ToRivetSqliteGetPagesRequest> {
	Ok(v5::ToRivetSqliteGetPagesRequest {
		request_id: x.request_id,
		data: convert_sqlite_get_pages_request_v6_to_v5(x.data)?,
	})
}

pub fn convert_to_rivet_sqlite_commit_request_v6_to_v5(x: v6::ToRivetSqliteCommitRequest) -> Result<v5::ToRivetSqliteCommitRequest> {
	Ok(v5::ToRivetSqliteCommitRequest {
		request_id: x.request_id,
		data: convert_sqlite_commit_request_v6_to_v5(x.data)?,
	})
}

pub fn convert_to_rivet_sqlite_exec_request_v6_to_v5(x: v6::ToRivetSqliteExecRequest) -> Result<v5::ToRivetSqliteExecRequest> {
	Ok(v5::ToRivetSqliteExecRequest {
		request_id: x.request_id,
		data: convert_sqlite_exec_request_v6_to_v5(x.data)?,
	})
}

pub fn convert_to_rivet_sqlite_execute_request_v6_to_v5(x: v6::ToRivetSqliteExecuteRequest) -> Result<v5::ToRivetSqliteExecuteRequest> {
	Ok(v5::ToRivetSqliteExecuteRequest {
		request_id: x.request_id,
		data: convert_sqlite_execute_request_v6_to_v5(x.data)?,
	})
}

pub fn convert_to_rivet_v6_to_v5(x: v6::ToRivet) -> Result<v5::ToRivet> {
	Ok(match x {
		v6::ToRivet::ToRivetMetadata(v) => v5::ToRivet::ToRivetMetadata(convert_to_rivet_metadata_v6_to_v5(v)?),
		v6::ToRivet::ToRivetEvents(v) => v5::ToRivet::ToRivetEvents(convert_to_rivet_events_v6_to_v5(v)?),
		v6::ToRivet::ToRivetAckCommands(v) => v5::ToRivet::ToRivetAckCommands(convert_to_rivet_ack_commands_v6_to_v5(v)?),
		v6::ToRivet::ToRivetStopping => v5::ToRivet::ToRivetStopping,
		v6::ToRivet::ToRivetPong(v) => v5::ToRivet::ToRivetPong(convert_to_rivet_pong_v6_to_v5(v)?),
		v6::ToRivet::ToRivetKvRequest(v) => v5::ToRivet::ToRivetKvRequest(convert_to_rivet_kv_request_v6_to_v5(v)?),
		v6::ToRivet::ToRivetTunnelMessage(v) => v5::ToRivet::ToRivetTunnelMessage(convert_to_rivet_tunnel_message_v6_to_v5(v)?),
		v6::ToRivet::ToRivetSqliteGetPagesRequest(v) => v5::ToRivet::ToRivetSqliteGetPagesRequest(convert_to_rivet_sqlite_get_pages_request_v6_to_v5(v)?),
		v6::ToRivet::ToRivetSqliteCommitRequest(v) => v5::ToRivet::ToRivetSqliteCommitRequest(convert_to_rivet_sqlite_commit_request_v6_to_v5(v)?),
		v6::ToRivet::ToRivetSqliteExecRequest(v) => v5::ToRivet::ToRivetSqliteExecRequest(convert_to_rivet_sqlite_exec_request_v6_to_v5(v)?),
		v6::ToRivet::ToRivetSqliteExecuteRequest(v) => v5::ToRivet::ToRivetSqliteExecuteRequest(convert_to_rivet_sqlite_execute_request_v6_to_v5(v)?),
		v6::ToRivet::ToRivetSqliteExecuteBatchRequest(_) => {
			return Err(incompatible(
				ProtocolCompatibilityFeature::RemoteSqliteBatchExecution,
				ProtocolCompatibilityDirection::ToRivet,
				6,
				5,
			));
		}
	})
}

pub fn convert_protocol_metadata_v6_to_v5(x: v6::ProtocolMetadata) -> Result<v5::ProtocolMetadata> {
	Ok(v5::ProtocolMetadata {
		envoy_lost_threshold: x.envoy_lost_threshold,
		actor_stop_threshold: x.actor_stop_threshold,
		max_response_payload_size: x.max_response_payload_size,
	})
}

pub fn convert_to_envoy_init_v6_to_v5(x: v6::ToEnvoyInit) -> Result<v5::ToEnvoyInit> {
	Ok(v5::ToEnvoyInit {
		metadata: convert_protocol_metadata_v6_to_v5(x.metadata)?,
	})
}

pub fn convert_to_envoy_commands_v6_to_v5(x: v6::ToEnvoyCommands) -> Result<v5::ToEnvoyCommands> {
	Ok(x.into_iter().map(|v| convert_command_wrapper_v6_to_v5(v)).collect::<Result<Vec<_>>>()?)
}

pub fn convert_to_envoy_ack_events_v6_to_v5(x: v6::ToEnvoyAckEvents) -> Result<v5::ToEnvoyAckEvents> {
	Ok(v5::ToEnvoyAckEvents {
		last_event_checkpoints: x.last_event_checkpoints.into_iter().map(|v| convert_actor_checkpoint_v6_to_v5(v)).collect::<Result<Vec<_>>>()?,
	})
}

pub fn convert_to_envoy_kv_response_v6_to_v5(x: v6::ToEnvoyKvResponse) -> Result<v5::ToEnvoyKvResponse> {
	Ok(v5::ToEnvoyKvResponse {
		request_id: x.request_id,
		data: convert_kv_response_data_v6_to_v5(x.data)?,
	})
}

pub fn convert_to_envoy_sqlite_get_pages_response_v6_to_v5(x: v6::ToEnvoySqliteGetPagesResponse) -> Result<v5::ToEnvoySqliteGetPagesResponse> {
	Ok(v5::ToEnvoySqliteGetPagesResponse {
		request_id: x.request_id,
		data: convert_sqlite_get_pages_response_v6_to_v5(x.data)?,
	})
}

pub fn convert_to_envoy_sqlite_commit_response_v6_to_v5(x: v6::ToEnvoySqliteCommitResponse) -> Result<v5::ToEnvoySqliteCommitResponse> {
	Ok(v5::ToEnvoySqliteCommitResponse {
		request_id: x.request_id,
		data: convert_sqlite_commit_response_v6_to_v5(x.data)?,
	})
}

pub fn convert_to_envoy_sqlite_exec_response_v6_to_v5(x: v6::ToEnvoySqliteExecResponse) -> Result<v5::ToEnvoySqliteExecResponse> {
	Ok(v5::ToEnvoySqliteExecResponse {
		request_id: x.request_id,
		data: convert_sqlite_exec_response_v6_to_v5(x.data)?,
	})
}

pub fn convert_to_envoy_sqlite_execute_response_v6_to_v5(x: v6::ToEnvoySqliteExecuteResponse) -> Result<v5::ToEnvoySqliteExecuteResponse> {
	Ok(v5::ToEnvoySqliteExecuteResponse {
		request_id: x.request_id,
		data: convert_sqlite_execute_response_v6_to_v5(x.data)?,
	})
}

pub fn convert_to_envoy_v6_to_v5(x: v6::ToEnvoy) -> Result<v5::ToEnvoy> {
	Ok(match x {
		v6::ToEnvoy::ToEnvoyInit(v) => v5::ToEnvoy::ToEnvoyInit(convert_to_envoy_init_v6_to_v5(v)?),
		v6::ToEnvoy::ToEnvoyCommands(v) => v5::ToEnvoy::ToEnvoyCommands(convert_to_envoy_commands_v6_to_v5(v)?),
		v6::ToEnvoy::ToEnvoyAckEvents(v) => v5::ToEnvoy::ToEnvoyAckEvents(convert_to_envoy_ack_events_v6_to_v5(v)?),
		v6::ToEnvoy::ToEnvoyKvResponse(v) => v5::ToEnvoy::ToEnvoyKvResponse(convert_to_envoy_kv_response_v6_to_v5(v)?),
		v6::ToEnvoy::ToEnvoyTunnelMessage(v) => v5::ToEnvoy::ToEnvoyTunnelMessage(convert_to_envoy_tunnel_message_v6_to_v5(v)?),
		v6::ToEnvoy::ToEnvoyPing(v) => v5::ToEnvoy::ToEnvoyPing(convert_to_envoy_ping_v6_to_v5(v)?),
		v6::ToEnvoy::ToEnvoySqliteGetPagesResponse(v) => v5::ToEnvoy::ToEnvoySqliteGetPagesResponse(convert_to_envoy_sqlite_get_pages_response_v6_to_v5(v)?),
		v6::ToEnvoy::ToEnvoySqliteCommitResponse(v) => v5::ToEnvoy::ToEnvoySqliteCommitResponse(convert_to_envoy_sqlite_commit_response_v6_to_v5(v)?),
		v6::ToEnvoy::ToEnvoySqliteExecResponse(v) => v5::ToEnvoy::ToEnvoySqliteExecResponse(convert_to_envoy_sqlite_exec_response_v6_to_v5(v)?),
		v6::ToEnvoy::ToEnvoySqliteExecuteResponse(v) => v5::ToEnvoy::ToEnvoySqliteExecuteResponse(convert_to_envoy_sqlite_execute_response_v6_to_v5(v)?),
		v6::ToEnvoy::ToEnvoySqliteExecuteBatchResponse(_) => {
			return Err(incompatible(
				ProtocolCompatibilityFeature::RemoteSqliteBatchExecution,
				ProtocolCompatibilityDirection::ToEnvoy,
				6,
				5,
			));
		}
	})
}

pub fn convert_to_envoy_conn_ping_v6_to_v5(x: v6::ToEnvoyConnPing) -> Result<v5::ToEnvoyConnPing> {
	Ok(v5::ToEnvoyConnPing {
		gateway_id: x.gateway_id,
		request_id: x.request_id,
		ts: x.ts,
	})
}

pub fn convert_to_envoy_conn_v6_to_v5(x: v6::ToEnvoyConn) -> Result<v5::ToEnvoyConn> {
	Ok(match x {
		v6::ToEnvoyConn::ToEnvoyConnPing(v) => v5::ToEnvoyConn::ToEnvoyConnPing(convert_to_envoy_conn_ping_v6_to_v5(v)?),
		v6::ToEnvoyConn::ToEnvoyConnClose => v5::ToEnvoyConn::ToEnvoyConnClose,
		v6::ToEnvoyConn::ToEnvoyCommands(v) => v5::ToEnvoyConn::ToEnvoyCommands(convert_to_envoy_commands_v6_to_v5(v)?),
		v6::ToEnvoyConn::ToEnvoyAckEvents(v) => v5::ToEnvoyConn::ToEnvoyAckEvents(convert_to_envoy_ack_events_v6_to_v5(v)?),
		v6::ToEnvoyConn::ToEnvoyTunnelMessage(v) => v5::ToEnvoyConn::ToEnvoyTunnelMessage(convert_to_envoy_tunnel_message_v6_to_v5(v)?),
	})
}

pub fn convert_to_gateway_pong_v6_to_v5(x: v6::ToGatewayPong) -> Result<v5::ToGatewayPong> {
	Ok(v5::ToGatewayPong {
		request_id: x.request_id,
		ts: x.ts,
	})
}

pub fn convert_to_gateway_v6_to_v5(x: v6::ToGateway) -> Result<v5::ToGateway> {
	Ok(match x {
		v6::ToGateway::ToGatewayPong(v) => v5::ToGateway::ToGatewayPong(convert_to_gateway_pong_v6_to_v5(v)?),
		v6::ToGateway::ToRivetTunnelMessage(v) => v5::ToGateway::ToRivetTunnelMessage(convert_to_rivet_tunnel_message_v6_to_v5(v)?),
	})
}

pub fn convert_to_outbound_actor_start_v6_to_v5(x: v6::ToOutboundActorStart) -> Result<v5::ToOutboundActorStart> {
	Ok(v5::ToOutboundActorStart {
		namespace_id: x.namespace_id,
		pool_name: x.pool_name,
		checkpoint: convert_actor_checkpoint_v6_to_v5(x.checkpoint)?,
		actor_config: convert_actor_config_v6_to_v5(x.actor_config)?,
	})
}

pub fn convert_to_outbound_v6_to_v5(x: v6::ToOutbound) -> Result<v5::ToOutbound> {
	Ok(match x {
		v6::ToOutbound::ToOutboundActorStart(v) => v5::ToOutbound::ToOutboundActorStart(convert_to_outbound_actor_start_v6_to_v5(v)?),
	})
}
