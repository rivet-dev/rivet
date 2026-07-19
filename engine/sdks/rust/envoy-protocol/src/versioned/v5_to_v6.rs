// from: v5.bare, to: v6.bare

#![allow(dead_code, unused_variables)]

use anyhow::Result;

use crate::generated::{v5, v6};

pub fn convert_kv_metadata_v5_to_v6(x: v5::KvMetadata) -> Result<v6::KvMetadata> {
	Ok(v6::KvMetadata {
		version: x.version,
		update_ts: x.update_ts,
	})
}

pub fn convert_kv_list_range_query_v5_to_v6(x: v5::KvListRangeQuery) -> Result<v6::KvListRangeQuery> {
	Ok(v6::KvListRangeQuery {
		start: x.start,
		end: x.end,
		exclusive: x.exclusive,
	})
}

pub fn convert_kv_list_prefix_query_v5_to_v6(x: v5::KvListPrefixQuery) -> Result<v6::KvListPrefixQuery> {
	Ok(v6::KvListPrefixQuery {
		key: x.key,
	})
}

pub fn convert_kv_list_query_v5_to_v6(x: v5::KvListQuery) -> Result<v6::KvListQuery> {
	Ok(match x {
		v5::KvListQuery::KvListAllQuery => v6::KvListQuery::KvListAllQuery,
		v5::KvListQuery::KvListRangeQuery(v) => v6::KvListQuery::KvListRangeQuery(convert_kv_list_range_query_v5_to_v6(v)?),
		v5::KvListQuery::KvListPrefixQuery(v) => v6::KvListQuery::KvListPrefixQuery(convert_kv_list_prefix_query_v5_to_v6(v)?),
	})
}

pub fn convert_kv_get_request_v5_to_v6(x: v5::KvGetRequest) -> Result<v6::KvGetRequest> {
	Ok(v6::KvGetRequest {
		keys: x.keys,
	})
}

pub fn convert_kv_list_request_v5_to_v6(x: v5::KvListRequest) -> Result<v6::KvListRequest> {
	Ok(v6::KvListRequest {
		query: convert_kv_list_query_v5_to_v6(x.query)?,
		reverse: x.reverse,
		limit: x.limit,
	})
}

pub fn convert_kv_put_request_v5_to_v6(x: v5::KvPutRequest) -> Result<v6::KvPutRequest> {
	Ok(v6::KvPutRequest {
		keys: x.keys,
		values: x.values,
	})
}

pub fn convert_kv_delete_request_v5_to_v6(x: v5::KvDeleteRequest) -> Result<v6::KvDeleteRequest> {
	Ok(v6::KvDeleteRequest {
		keys: x.keys,
	})
}

pub fn convert_kv_delete_range_request_v5_to_v6(x: v5::KvDeleteRangeRequest) -> Result<v6::KvDeleteRangeRequest> {
	Ok(v6::KvDeleteRangeRequest {
		start: x.start,
		end: x.end,
	})
}

pub fn convert_kv_error_response_v5_to_v6(x: v5::KvErrorResponse) -> Result<v6::KvErrorResponse> {
	Ok(v6::KvErrorResponse {
		message: x.message,
	})
}

pub fn convert_kv_get_response_v5_to_v6(x: v5::KvGetResponse) -> Result<v6::KvGetResponse> {
	Ok(v6::KvGetResponse {
		keys: x.keys,
		values: x.values,
		metadata: x.metadata.into_iter().map(|v| convert_kv_metadata_v5_to_v6(v)).collect::<Result<Vec<_>>>()?,
	})
}

pub fn convert_kv_list_response_v5_to_v6(x: v5::KvListResponse) -> Result<v6::KvListResponse> {
	Ok(v6::KvListResponse {
		keys: x.keys,
		values: x.values,
		metadata: x.metadata.into_iter().map(|v| convert_kv_metadata_v5_to_v6(v)).collect::<Result<Vec<_>>>()?,
	})
}

pub fn convert_kv_request_data_v5_to_v6(x: v5::KvRequestData) -> Result<v6::KvRequestData> {
	Ok(match x {
		v5::KvRequestData::KvGetRequest(v) => v6::KvRequestData::KvGetRequest(convert_kv_get_request_v5_to_v6(v)?),
		v5::KvRequestData::KvListRequest(v) => v6::KvRequestData::KvListRequest(convert_kv_list_request_v5_to_v6(v)?),
		v5::KvRequestData::KvPutRequest(v) => v6::KvRequestData::KvPutRequest(convert_kv_put_request_v5_to_v6(v)?),
		v5::KvRequestData::KvDeleteRequest(v) => v6::KvRequestData::KvDeleteRequest(convert_kv_delete_request_v5_to_v6(v)?),
		v5::KvRequestData::KvDeleteRangeRequest(v) => v6::KvRequestData::KvDeleteRangeRequest(convert_kv_delete_range_request_v5_to_v6(v)?),
		v5::KvRequestData::KvDropRequest => v6::KvRequestData::KvDropRequest,
	})
}

pub fn convert_kv_response_data_v5_to_v6(x: v5::KvResponseData) -> Result<v6::KvResponseData> {
	Ok(match x {
		v5::KvResponseData::KvErrorResponse(v) => v6::KvResponseData::KvErrorResponse(convert_kv_error_response_v5_to_v6(v)?),
		v5::KvResponseData::KvGetResponse(v) => v6::KvResponseData::KvGetResponse(convert_kv_get_response_v5_to_v6(v)?),
		v5::KvResponseData::KvListResponse(v) => v6::KvResponseData::KvListResponse(convert_kv_list_response_v5_to_v6(v)?),
		v5::KvResponseData::KvPutResponse => v6::KvResponseData::KvPutResponse,
		v5::KvResponseData::KvDeleteResponse => v6::KvResponseData::KvDeleteResponse,
		v5::KvResponseData::KvDropResponse => v6::KvResponseData::KvDropResponse,
	})
}

pub fn convert_sqlite_dirty_page_v5_to_v6(x: v5::SqliteDirtyPage) -> Result<v6::SqliteDirtyPage> {
	Ok(v6::SqliteDirtyPage {
		pgno: x.pgno,
		bytes: x.bytes,
	})
}

pub fn convert_sqlite_fetched_page_v5_to_v6(x: v5::SqliteFetchedPage) -> Result<v6::SqliteFetchedPage> {
	Ok(v6::SqliteFetchedPage {
		pgno: x.pgno,
		bytes: x.bytes,
	})
}

pub fn convert_sqlite_get_pages_request_v5_to_v6(x: v5::SqliteGetPagesRequest) -> Result<v6::SqliteGetPagesRequest> {
	Ok(v6::SqliteGetPagesRequest {
		actor_id: x.actor_id,
		pgnos: x.pgnos,
		expected_generation: x.expected_generation,
		expected_head_txid: x.expected_head_txid,
	})
}

pub fn convert_sqlite_get_pages_ok_v5_to_v6(x: v5::SqliteGetPagesOk) -> Result<v6::SqliteGetPagesOk> {
	Ok(v6::SqliteGetPagesOk {
		pages: x.pages.into_iter().map(|v| convert_sqlite_fetched_page_v5_to_v6(v)).collect::<Result<Vec<_>>>()?,
		head_txid: x.head_txid,
	})
}

pub fn convert_sqlite_error_response_v5_to_v6(x: v5::SqliteErrorResponse) -> Result<v6::SqliteErrorResponse> {
	Ok(v6::SqliteErrorResponse {
		group: x.group,
		code: x.code,
		message: x.message,
	})
}

pub fn convert_sqlite_get_pages_response_v5_to_v6(x: v5::SqliteGetPagesResponse) -> Result<v6::SqliteGetPagesResponse> {
	Ok(match x {
		v5::SqliteGetPagesResponse::SqliteGetPagesOk(v) => v6::SqliteGetPagesResponse::SqliteGetPagesOk(convert_sqlite_get_pages_ok_v5_to_v6(v)?),
		v5::SqliteGetPagesResponse::SqliteErrorResponse(v) => v6::SqliteGetPagesResponse::SqliteErrorResponse(convert_sqlite_error_response_v5_to_v6(v)?),
	})
}

pub fn convert_sqlite_commit_request_v5_to_v6(x: v5::SqliteCommitRequest) -> Result<v6::SqliteCommitRequest> {
	Ok(v6::SqliteCommitRequest {
		actor_id: x.actor_id,
		dirty_pages: x.dirty_pages.into_iter().map(|v| convert_sqlite_dirty_page_v5_to_v6(v)).collect::<Result<Vec<_>>>()?,
		db_size_pages: x.db_size_pages,
		now_ms: x.now_ms,
		expected_generation: x.expected_generation,
		expected_head_txid: x.expected_head_txid,
	})
}

pub fn convert_sqlite_commit_ok_v5_to_v6(x: v5::SqliteCommitOk) -> Result<v6::SqliteCommitOk> {
	Ok(v6::SqliteCommitOk {
		head_txid: x.head_txid,
	})
}

pub fn convert_sqlite_commit_response_v5_to_v6(x: v5::SqliteCommitResponse) -> Result<v6::SqliteCommitResponse> {
	Ok(match x {
		v5::SqliteCommitResponse::SqliteCommitOk(v) => v6::SqliteCommitResponse::SqliteCommitOk(convert_sqlite_commit_ok_v5_to_v6(v)?),
		v5::SqliteCommitResponse::SqliteErrorResponse(v) => v6::SqliteCommitResponse::SqliteErrorResponse(convert_sqlite_error_response_v5_to_v6(v)?),
	})
}

pub fn convert_sqlite_value_integer_v5_to_v6(x: v5::SqliteValueInteger) -> Result<v6::SqliteValueInteger> {
	Ok(v6::SqliteValueInteger {
		value: x.value,
	})
}

pub fn convert_sqlite_value_float_v5_to_v6(x: v5::SqliteValueFloat) -> Result<v6::SqliteValueFloat> {
	Ok(v6::SqliteValueFloat {
		value: x.value,
	})
}

pub fn convert_sqlite_value_text_v5_to_v6(x: v5::SqliteValueText) -> Result<v6::SqliteValueText> {
	Ok(v6::SqliteValueText {
		value: x.value,
	})
}

pub fn convert_sqlite_value_blob_v5_to_v6(x: v5::SqliteValueBlob) -> Result<v6::SqliteValueBlob> {
	Ok(v6::SqliteValueBlob {
		value: x.value,
	})
}

pub fn convert_sqlite_bind_param_v5_to_v6(x: v5::SqliteBindParam) -> Result<v6::SqliteBindParam> {
	Ok(match x {
		v5::SqliteBindParam::SqliteValueNull => v6::SqliteBindParam::SqliteValueNull,
		v5::SqliteBindParam::SqliteValueInteger(v) => v6::SqliteBindParam::SqliteValueInteger(convert_sqlite_value_integer_v5_to_v6(v)?),
		v5::SqliteBindParam::SqliteValueFloat(v) => v6::SqliteBindParam::SqliteValueFloat(convert_sqlite_value_float_v5_to_v6(v)?),
		v5::SqliteBindParam::SqliteValueText(v) => v6::SqliteBindParam::SqliteValueText(convert_sqlite_value_text_v5_to_v6(v)?),
		v5::SqliteBindParam::SqliteValueBlob(v) => v6::SqliteBindParam::SqliteValueBlob(convert_sqlite_value_blob_v5_to_v6(v)?),
	})
}

pub fn convert_sqlite_column_value_v5_to_v6(x: v5::SqliteColumnValue) -> Result<v6::SqliteColumnValue> {
	Ok(match x {
		v5::SqliteColumnValue::SqliteValueNull => v6::SqliteColumnValue::SqliteValueNull,
		v5::SqliteColumnValue::SqliteValueInteger(v) => v6::SqliteColumnValue::SqliteValueInteger(convert_sqlite_value_integer_v5_to_v6(v)?),
		v5::SqliteColumnValue::SqliteValueFloat(v) => v6::SqliteColumnValue::SqliteValueFloat(convert_sqlite_value_float_v5_to_v6(v)?),
		v5::SqliteColumnValue::SqliteValueText(v) => v6::SqliteColumnValue::SqliteValueText(convert_sqlite_value_text_v5_to_v6(v)?),
		v5::SqliteColumnValue::SqliteValueBlob(v) => v6::SqliteColumnValue::SqliteValueBlob(convert_sqlite_value_blob_v5_to_v6(v)?),
	})
}

pub fn convert_sqlite_query_result_v5_to_v6(x: v5::SqliteQueryResult) -> Result<v6::SqliteQueryResult> {
	Ok(v6::SqliteQueryResult {
		columns: x.columns,
		rows: x.rows.into_iter().map(|v| v.into_iter().map(|v| convert_sqlite_column_value_v5_to_v6(v)).collect::<Result<Vec<_>>>()).collect::<Result<Vec<_>>>()?,
	})
}

pub fn convert_sqlite_execute_result_v5_to_v6(x: v5::SqliteExecuteResult) -> Result<v6::SqliteExecuteResult> {
	Ok(v6::SqliteExecuteResult {
		columns: x.columns,
		rows: x.rows.into_iter().map(|v| v.into_iter().map(|v| convert_sqlite_column_value_v5_to_v6(v)).collect::<Result<Vec<_>>>()).collect::<Result<Vec<_>>>()?,
		changes: x.changes,
		last_insert_row_id: x.last_insert_row_id,
	})
}

pub fn convert_sqlite_exec_request_v5_to_v6(x: v5::SqliteExecRequest) -> Result<v6::SqliteExecRequest> {
	Ok(v6::SqliteExecRequest {
		namespace_id: x.namespace_id,
		actor_id: x.actor_id,
		generation: x.generation,
		sql: x.sql,
	})
}

pub fn convert_sqlite_execute_request_v5_to_v6(x: v5::SqliteExecuteRequest) -> Result<v6::SqliteExecuteRequest> {
	Ok(v6::SqliteExecuteRequest {
		namespace_id: x.namespace_id,
		actor_id: x.actor_id,
		generation: x.generation,
		sql: x.sql,
		params: x.params.map(|v| v.into_iter().map(|v| convert_sqlite_bind_param_v5_to_v6(v)).collect::<Result<Vec<_>>>()).transpose()?,
	})
}

pub fn convert_sqlite_exec_ok_v5_to_v6(x: v5::SqliteExecOk) -> Result<v6::SqliteExecOk> {
	Ok(v6::SqliteExecOk {
		result: convert_sqlite_query_result_v5_to_v6(x.result)?,
	})
}

pub fn convert_sqlite_execute_ok_v5_to_v6(x: v5::SqliteExecuteOk) -> Result<v6::SqliteExecuteOk> {
	Ok(v6::SqliteExecuteOk {
		result: convert_sqlite_execute_result_v5_to_v6(x.result)?,
	})
}

pub fn convert_sqlite_exec_response_v5_to_v6(x: v5::SqliteExecResponse) -> Result<v6::SqliteExecResponse> {
	Ok(match x {
		v5::SqliteExecResponse::SqliteExecOk(v) => v6::SqliteExecResponse::SqliteExecOk(convert_sqlite_exec_ok_v5_to_v6(v)?),
		v5::SqliteExecResponse::SqliteErrorResponse(v) => v6::SqliteExecResponse::SqliteErrorResponse(convert_sqlite_error_response_v5_to_v6(v)?),
	})
}

pub fn convert_sqlite_execute_response_v5_to_v6(x: v5::SqliteExecuteResponse) -> Result<v6::SqliteExecuteResponse> {
	Ok(match x {
		v5::SqliteExecuteResponse::SqliteExecuteOk(v) => v6::SqliteExecuteResponse::SqliteExecuteOk(convert_sqlite_execute_ok_v5_to_v6(v)?),
		v5::SqliteExecuteResponse::SqliteErrorResponse(v) => v6::SqliteExecuteResponse::SqliteErrorResponse(convert_sqlite_error_response_v5_to_v6(v)?),
	})
}

pub fn convert_stop_code_v5_to_v6(x: v5::StopCode) -> Result<v6::StopCode> {
	Ok(match x {
		v5::StopCode::Ok => v6::StopCode::Ok,
		v5::StopCode::Error => v6::StopCode::Error,
	})
}

pub fn convert_actor_name_v5_to_v6(x: v5::ActorName) -> Result<v6::ActorName> {
	Ok(v6::ActorName {
		metadata: x.metadata,
	})
}

pub fn convert_actor_config_v5_to_v6(x: v5::ActorConfig) -> Result<v6::ActorConfig> {
	Ok(v6::ActorConfig {
		name: x.name,
		key: x.key,
		create_ts: x.create_ts,
		input: x.input,
	})
}

pub fn convert_actor_checkpoint_v5_to_v6(x: v5::ActorCheckpoint) -> Result<v6::ActorCheckpoint> {
	Ok(v6::ActorCheckpoint {
		actor_id: x.actor_id,
		generation: x.generation,
		index: x.index,
	})
}

pub fn convert_actor_intent_v5_to_v6(x: v5::ActorIntent) -> Result<v6::ActorIntent> {
	Ok(match x {
		v5::ActorIntent::ActorIntentSleep => v6::ActorIntent::ActorIntentSleep,
		v5::ActorIntent::ActorIntentStop => v6::ActorIntent::ActorIntentStop,
	})
}

pub fn convert_actor_state_stopped_v5_to_v6(x: v5::ActorStateStopped) -> Result<v6::ActorStateStopped> {
	Ok(v6::ActorStateStopped {
		code: convert_stop_code_v5_to_v6(x.code)?,
		message: x.message,
	})
}

pub fn convert_actor_state_v5_to_v6(x: v5::ActorState) -> Result<v6::ActorState> {
	Ok(match x {
		v5::ActorState::ActorStateRunning => v6::ActorState::ActorStateRunning,
		v5::ActorState::ActorStateStopped(v) => v6::ActorState::ActorStateStopped(convert_actor_state_stopped_v5_to_v6(v)?),
	})
}

pub fn convert_event_actor_intent_v5_to_v6(x: v5::EventActorIntent) -> Result<v6::EventActorIntent> {
	Ok(v6::EventActorIntent {
		intent: convert_actor_intent_v5_to_v6(x.intent)?,
	})
}

pub fn convert_event_actor_state_update_v5_to_v6(x: v5::EventActorStateUpdate) -> Result<v6::EventActorStateUpdate> {
	Ok(v6::EventActorStateUpdate {
		state: convert_actor_state_v5_to_v6(x.state)?,
	})
}

pub fn convert_event_actor_set_alarm_v5_to_v6(x: v5::EventActorSetAlarm) -> Result<v6::EventActorSetAlarm> {
	Ok(v6::EventActorSetAlarm {
		alarm_ts: x.alarm_ts,
	})
}

pub fn convert_event_v5_to_v6(x: v5::Event) -> Result<v6::Event> {
	Ok(match x {
		v5::Event::EventActorIntent(v) => v6::Event::EventActorIntent(convert_event_actor_intent_v5_to_v6(v)?),
		v5::Event::EventActorStateUpdate(v) => v6::Event::EventActorStateUpdate(convert_event_actor_state_update_v5_to_v6(v)?),
		v5::Event::EventActorSetAlarm(v) => v6::Event::EventActorSetAlarm(convert_event_actor_set_alarm_v5_to_v6(v)?),
	})
}

pub fn convert_event_wrapper_v5_to_v6(x: v5::EventWrapper) -> Result<v6::EventWrapper> {
	Ok(v6::EventWrapper {
		checkpoint: convert_actor_checkpoint_v5_to_v6(x.checkpoint)?,
		inner: convert_event_v5_to_v6(x.inner)?,
	})
}

pub fn convert_preloaded_kv_entry_v5_to_v6(x: v5::PreloadedKvEntry) -> Result<v6::PreloadedKvEntry> {
	Ok(v6::PreloadedKvEntry {
		key: x.key,
		value: x.value,
		metadata: convert_kv_metadata_v5_to_v6(x.metadata)?,
	})
}

pub fn convert_preloaded_kv_v5_to_v6(x: v5::PreloadedKv) -> Result<v6::PreloadedKv> {
	Ok(v6::PreloadedKv {
		entries: x.entries.into_iter().map(|v| convert_preloaded_kv_entry_v5_to_v6(v)).collect::<Result<Vec<_>>>()?,
		requested_get_keys: x.requested_get_keys,
		requested_prefixes: x.requested_prefixes,
	})
}

pub fn convert_hibernating_request_v5_to_v6(x: v5::HibernatingRequest) -> Result<v6::HibernatingRequest> {
	Ok(v6::HibernatingRequest {
		gateway_id: x.gateway_id,
		request_id: x.request_id,
	})
}

pub fn convert_command_start_actor_v5_to_v6(x: v5::CommandStartActor) -> Result<v6::CommandStartActor> {
	Ok(v6::CommandStartActor {
		config: convert_actor_config_v5_to_v6(x.config)?,
		hibernating_requests: x.hibernating_requests.into_iter().map(|v| convert_hibernating_request_v5_to_v6(v)).collect::<Result<Vec<_>>>()?,
		preloaded_kv: x.preloaded_kv.map(|v| convert_preloaded_kv_v5_to_v6(v)).transpose()?,
	})
}

pub fn convert_stop_actor_reason_v5_to_v6(x: v5::StopActorReason) -> Result<v6::StopActorReason> {
	Ok(match x {
		v5::StopActorReason::SleepIntent => v6::StopActorReason::SleepIntent,
		v5::StopActorReason::StopIntent => v6::StopActorReason::StopIntent,
		v5::StopActorReason::Destroy => v6::StopActorReason::Destroy,
		v5::StopActorReason::GoingAway => v6::StopActorReason::GoingAway,
		v5::StopActorReason::Lost => v6::StopActorReason::Lost,
	})
}

pub fn convert_command_stop_actor_v5_to_v6(x: v5::CommandStopActor) -> Result<v6::CommandStopActor> {
	Ok(v6::CommandStopActor {
		reason: convert_stop_actor_reason_v5_to_v6(x.reason)?,
	})
}

pub fn convert_command_v5_to_v6(x: v5::Command) -> Result<v6::Command> {
	Ok(match x {
		v5::Command::CommandStartActor(v) => v6::Command::CommandStartActor(convert_command_start_actor_v5_to_v6(v)?),
		v5::Command::CommandStopActor(v) => v6::Command::CommandStopActor(convert_command_stop_actor_v5_to_v6(v)?),
	})
}

pub fn convert_command_wrapper_v5_to_v6(x: v5::CommandWrapper) -> Result<v6::CommandWrapper> {
	Ok(v6::CommandWrapper {
		checkpoint: convert_actor_checkpoint_v5_to_v6(x.checkpoint)?,
		inner: convert_command_v5_to_v6(x.inner)?,
	})
}

pub fn convert_actor_command_key_data_v5_to_v6(x: v5::ActorCommandKeyData) -> Result<v6::ActorCommandKeyData> {
	Ok(match x {
		v5::ActorCommandKeyData::CommandStartActor(v) => v6::ActorCommandKeyData::CommandStartActor(convert_command_start_actor_v5_to_v6(v)?),
		v5::ActorCommandKeyData::CommandStopActor(v) => v6::ActorCommandKeyData::CommandStopActor(convert_command_stop_actor_v5_to_v6(v)?),
	})
}

pub fn convert_message_id_v5_to_v6(x: v5::MessageId) -> Result<v6::MessageId> {
	Ok(v6::MessageId {
		gateway_id: x.gateway_id,
		request_id: x.request_id,
		message_index: x.message_index,
	})
}

pub fn convert_to_envoy_request_start_v5_to_v6(x: v5::ToEnvoyRequestStart) -> Result<v6::ToEnvoyRequestStart> {
	Ok(v6::ToEnvoyRequestStart {
		actor_id: x.actor_id,
		method: x.method,
		path: x.path,
		headers: x.headers,
		body: x.body,
		stream: x.stream,
	})
}

pub fn convert_to_envoy_request_chunk_v5_to_v6(x: v5::ToEnvoyRequestChunk) -> Result<v6::ToEnvoyRequestChunk> {
	Ok(v6::ToEnvoyRequestChunk {
		body: x.body,
		finish: x.finish,
	})
}

pub fn convert_to_rivet_response_start_v5_to_v6(x: v5::ToRivetResponseStart) -> Result<v6::ToRivetResponseStart> {
	Ok(v6::ToRivetResponseStart {
		status: x.status,
		headers: x.headers,
		body: x.body,
		stream: x.stream,
	})
}

pub fn convert_to_rivet_response_chunk_v5_to_v6(x: v5::ToRivetResponseChunk) -> Result<v6::ToRivetResponseChunk> {
	Ok(v6::ToRivetResponseChunk {
		body: x.body,
		finish: x.finish,
	})
}

pub fn convert_to_envoy_web_socket_open_v5_to_v6(x: v5::ToEnvoyWebSocketOpen) -> Result<v6::ToEnvoyWebSocketOpen> {
	Ok(v6::ToEnvoyWebSocketOpen {
		actor_id: x.actor_id,
		path: x.path,
		headers: x.headers,
	})
}

pub fn convert_to_envoy_web_socket_message_v5_to_v6(x: v5::ToEnvoyWebSocketMessage) -> Result<v6::ToEnvoyWebSocketMessage> {
	Ok(v6::ToEnvoyWebSocketMessage {
		data: x.data,
		binary: x.binary,
	})
}

pub fn convert_to_envoy_web_socket_close_v5_to_v6(x: v5::ToEnvoyWebSocketClose) -> Result<v6::ToEnvoyWebSocketClose> {
	Ok(v6::ToEnvoyWebSocketClose {
		code: x.code,
		reason: x.reason,
	})
}

pub fn convert_to_rivet_web_socket_open_v5_to_v6(x: v5::ToRivetWebSocketOpen) -> Result<v6::ToRivetWebSocketOpen> {
	Ok(v6::ToRivetWebSocketOpen {
		can_hibernate: x.can_hibernate,
	})
}

pub fn convert_to_rivet_web_socket_message_v5_to_v6(x: v5::ToRivetWebSocketMessage) -> Result<v6::ToRivetWebSocketMessage> {
	Ok(v6::ToRivetWebSocketMessage {
		data: x.data,
		binary: x.binary,
	})
}

pub fn convert_to_rivet_web_socket_message_ack_v5_to_v6(x: v5::ToRivetWebSocketMessageAck) -> Result<v6::ToRivetWebSocketMessageAck> {
	Ok(v6::ToRivetWebSocketMessageAck {
		index: x.index,
	})
}

pub fn convert_to_rivet_web_socket_close_v5_to_v6(x: v5::ToRivetWebSocketClose) -> Result<v6::ToRivetWebSocketClose> {
	Ok(v6::ToRivetWebSocketClose {
		code: x.code,
		reason: x.reason,
		hibernate: x.hibernate,
	})
}

pub fn convert_to_rivet_tunnel_message_kind_v5_to_v6(x: v5::ToRivetTunnelMessageKind) -> Result<v6::ToRivetTunnelMessageKind> {
	Ok(match x {
		v5::ToRivetTunnelMessageKind::ToRivetResponseStart(v) => v6::ToRivetTunnelMessageKind::ToRivetResponseStart(convert_to_rivet_response_start_v5_to_v6(v)?),
		v5::ToRivetTunnelMessageKind::ToRivetResponseChunk(v) => v6::ToRivetTunnelMessageKind::ToRivetResponseChunk(convert_to_rivet_response_chunk_v5_to_v6(v)?),
		v5::ToRivetTunnelMessageKind::ToRivetResponseAbort => v6::ToRivetTunnelMessageKind::ToRivetResponseAbort,
		v5::ToRivetTunnelMessageKind::ToRivetWebSocketOpen(v) => v6::ToRivetTunnelMessageKind::ToRivetWebSocketOpen(convert_to_rivet_web_socket_open_v5_to_v6(v)?),
		v5::ToRivetTunnelMessageKind::ToRivetWebSocketMessage(v) => v6::ToRivetTunnelMessageKind::ToRivetWebSocketMessage(convert_to_rivet_web_socket_message_v5_to_v6(v)?),
		v5::ToRivetTunnelMessageKind::ToRivetWebSocketMessageAck(v) => v6::ToRivetTunnelMessageKind::ToRivetWebSocketMessageAck(convert_to_rivet_web_socket_message_ack_v5_to_v6(v)?),
		v5::ToRivetTunnelMessageKind::ToRivetWebSocketClose(v) => v6::ToRivetTunnelMessageKind::ToRivetWebSocketClose(convert_to_rivet_web_socket_close_v5_to_v6(v)?),
	})
}

pub fn convert_to_rivet_tunnel_message_v5_to_v6(x: v5::ToRivetTunnelMessage) -> Result<v6::ToRivetTunnelMessage> {
	Ok(v6::ToRivetTunnelMessage {
		message_id: convert_message_id_v5_to_v6(x.message_id)?,
		message_kind: convert_to_rivet_tunnel_message_kind_v5_to_v6(x.message_kind)?,
	})
}

pub fn convert_to_envoy_tunnel_message_kind_v5_to_v6(x: v5::ToEnvoyTunnelMessageKind) -> Result<v6::ToEnvoyTunnelMessageKind> {
	Ok(match x {
		v5::ToEnvoyTunnelMessageKind::ToEnvoyRequestStart(v) => v6::ToEnvoyTunnelMessageKind::ToEnvoyRequestStart(convert_to_envoy_request_start_v5_to_v6(v)?),
		v5::ToEnvoyTunnelMessageKind::ToEnvoyRequestChunk(v) => v6::ToEnvoyTunnelMessageKind::ToEnvoyRequestChunk(convert_to_envoy_request_chunk_v5_to_v6(v)?),
		v5::ToEnvoyTunnelMessageKind::ToEnvoyRequestAbort => v6::ToEnvoyTunnelMessageKind::ToEnvoyRequestAbort,
		v5::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketOpen(v) => v6::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketOpen(convert_to_envoy_web_socket_open_v5_to_v6(v)?),
		v5::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketMessage(v) => v6::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketMessage(convert_to_envoy_web_socket_message_v5_to_v6(v)?),
		v5::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketClose(v) => v6::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketClose(convert_to_envoy_web_socket_close_v5_to_v6(v)?),
	})
}

pub fn convert_to_envoy_tunnel_message_v5_to_v6(x: v5::ToEnvoyTunnelMessage) -> Result<v6::ToEnvoyTunnelMessage> {
	Ok(v6::ToEnvoyTunnelMessage {
		message_id: convert_message_id_v5_to_v6(x.message_id)?,
		message_kind: convert_to_envoy_tunnel_message_kind_v5_to_v6(x.message_kind)?,
	})
}

pub fn convert_to_envoy_ping_v5_to_v6(x: v5::ToEnvoyPing) -> Result<v6::ToEnvoyPing> {
	Ok(v6::ToEnvoyPing {
		ts: x.ts,
	})
}

pub fn convert_to_rivet_metadata_v5_to_v6(x: v5::ToRivetMetadata) -> Result<v6::ToRivetMetadata> {
	Ok(v6::ToRivetMetadata {
		prepopulate_actor_names: x.prepopulate_actor_names.map(|v| v.into_iter().map(|(k, v)| -> Result<_> { Ok((k, convert_actor_name_v5_to_v6(v)?)) }).collect::<Result<_>>()).transpose()?,
		metadata: x.metadata,
	})
}

pub fn convert_to_rivet_events_v5_to_v6(x: v5::ToRivetEvents) -> Result<v6::ToRivetEvents> {
	Ok(x.into_iter().map(|v| convert_event_wrapper_v5_to_v6(v)).collect::<Result<Vec<_>>>()?)
}

pub fn convert_to_rivet_ack_commands_v5_to_v6(x: v5::ToRivetAckCommands) -> Result<v6::ToRivetAckCommands> {
	Ok(v6::ToRivetAckCommands {
		last_command_checkpoints: x.last_command_checkpoints.into_iter().map(|v| convert_actor_checkpoint_v5_to_v6(v)).collect::<Result<Vec<_>>>()?,
	})
}

pub fn convert_to_rivet_pong_v5_to_v6(x: v5::ToRivetPong) -> Result<v6::ToRivetPong> {
	Ok(v6::ToRivetPong {
		ts: x.ts,
	})
}

pub fn convert_to_rivet_kv_request_v5_to_v6(x: v5::ToRivetKvRequest) -> Result<v6::ToRivetKvRequest> {
	Ok(v6::ToRivetKvRequest {
		actor_id: x.actor_id,
		request_id: x.request_id,
		data: convert_kv_request_data_v5_to_v6(x.data)?,
	})
}

pub fn convert_to_rivet_sqlite_get_pages_request_v5_to_v6(x: v5::ToRivetSqliteGetPagesRequest) -> Result<v6::ToRivetSqliteGetPagesRequest> {
	Ok(v6::ToRivetSqliteGetPagesRequest {
		request_id: x.request_id,
		data: convert_sqlite_get_pages_request_v5_to_v6(x.data)?,
	})
}

pub fn convert_to_rivet_sqlite_commit_request_v5_to_v6(x: v5::ToRivetSqliteCommitRequest) -> Result<v6::ToRivetSqliteCommitRequest> {
	Ok(v6::ToRivetSqliteCommitRequest {
		request_id: x.request_id,
		data: convert_sqlite_commit_request_v5_to_v6(x.data)?,
	})
}

pub fn convert_to_rivet_sqlite_exec_request_v5_to_v6(x: v5::ToRivetSqliteExecRequest) -> Result<v6::ToRivetSqliteExecRequest> {
	Ok(v6::ToRivetSqliteExecRequest {
		request_id: x.request_id,
		data: convert_sqlite_exec_request_v5_to_v6(x.data)?,
	})
}

pub fn convert_to_rivet_sqlite_execute_request_v5_to_v6(x: v5::ToRivetSqliteExecuteRequest) -> Result<v6::ToRivetSqliteExecuteRequest> {
	Ok(v6::ToRivetSqliteExecuteRequest {
		request_id: x.request_id,
		data: convert_sqlite_execute_request_v5_to_v6(x.data)?,
	})
}

pub fn convert_to_rivet_v5_to_v6(x: v5::ToRivet) -> Result<v6::ToRivet> {
	Ok(match x {
		v5::ToRivet::ToRivetMetadata(v) => v6::ToRivet::ToRivetMetadata(convert_to_rivet_metadata_v5_to_v6(v)?),
		v5::ToRivet::ToRivetEvents(v) => v6::ToRivet::ToRivetEvents(convert_to_rivet_events_v5_to_v6(v)?),
		v5::ToRivet::ToRivetAckCommands(v) => v6::ToRivet::ToRivetAckCommands(convert_to_rivet_ack_commands_v5_to_v6(v)?),
		v5::ToRivet::ToRivetStopping => v6::ToRivet::ToRivetStopping,
		v5::ToRivet::ToRivetPong(v) => v6::ToRivet::ToRivetPong(convert_to_rivet_pong_v5_to_v6(v)?),
		v5::ToRivet::ToRivetKvRequest(v) => v6::ToRivet::ToRivetKvRequest(convert_to_rivet_kv_request_v5_to_v6(v)?),
		v5::ToRivet::ToRivetTunnelMessage(v) => v6::ToRivet::ToRivetTunnelMessage(convert_to_rivet_tunnel_message_v5_to_v6(v)?),
		v5::ToRivet::ToRivetSqliteGetPagesRequest(v) => v6::ToRivet::ToRivetSqliteGetPagesRequest(convert_to_rivet_sqlite_get_pages_request_v5_to_v6(v)?),
		v5::ToRivet::ToRivetSqliteCommitRequest(v) => v6::ToRivet::ToRivetSqliteCommitRequest(convert_to_rivet_sqlite_commit_request_v5_to_v6(v)?),
		v5::ToRivet::ToRivetSqliteExecRequest(v) => v6::ToRivet::ToRivetSqliteExecRequest(convert_to_rivet_sqlite_exec_request_v5_to_v6(v)?),
		v5::ToRivet::ToRivetSqliteExecuteRequest(v) => v6::ToRivet::ToRivetSqliteExecuteRequest(convert_to_rivet_sqlite_execute_request_v5_to_v6(v)?),
	})
}

pub fn convert_protocol_metadata_v5_to_v6(x: v5::ProtocolMetadata) -> Result<v6::ProtocolMetadata> {
	Ok(v6::ProtocolMetadata {
		envoy_lost_threshold: x.envoy_lost_threshold,
		actor_stop_threshold: x.actor_stop_threshold,
		max_response_payload_size: x.max_response_payload_size,
	})
}

pub fn convert_to_envoy_init_v5_to_v6(x: v5::ToEnvoyInit) -> Result<v6::ToEnvoyInit> {
	Ok(v6::ToEnvoyInit {
		metadata: convert_protocol_metadata_v5_to_v6(x.metadata)?,
	})
}

pub fn convert_to_envoy_commands_v5_to_v6(x: v5::ToEnvoyCommands) -> Result<v6::ToEnvoyCommands> {
	Ok(x.into_iter().map(|v| convert_command_wrapper_v5_to_v6(v)).collect::<Result<Vec<_>>>()?)
}

pub fn convert_to_envoy_ack_events_v5_to_v6(x: v5::ToEnvoyAckEvents) -> Result<v6::ToEnvoyAckEvents> {
	Ok(v6::ToEnvoyAckEvents {
		last_event_checkpoints: x.last_event_checkpoints.into_iter().map(|v| convert_actor_checkpoint_v5_to_v6(v)).collect::<Result<Vec<_>>>()?,
	})
}

pub fn convert_to_envoy_kv_response_v5_to_v6(x: v5::ToEnvoyKvResponse) -> Result<v6::ToEnvoyKvResponse> {
	Ok(v6::ToEnvoyKvResponse {
		request_id: x.request_id,
		data: convert_kv_response_data_v5_to_v6(x.data)?,
	})
}

pub fn convert_to_envoy_sqlite_get_pages_response_v5_to_v6(x: v5::ToEnvoySqliteGetPagesResponse) -> Result<v6::ToEnvoySqliteGetPagesResponse> {
	Ok(v6::ToEnvoySqliteGetPagesResponse {
		request_id: x.request_id,
		data: convert_sqlite_get_pages_response_v5_to_v6(x.data)?,
	})
}

pub fn convert_to_envoy_sqlite_commit_response_v5_to_v6(x: v5::ToEnvoySqliteCommitResponse) -> Result<v6::ToEnvoySqliteCommitResponse> {
	Ok(v6::ToEnvoySqliteCommitResponse {
		request_id: x.request_id,
		data: convert_sqlite_commit_response_v5_to_v6(x.data)?,
	})
}

pub fn convert_to_envoy_sqlite_exec_response_v5_to_v6(x: v5::ToEnvoySqliteExecResponse) -> Result<v6::ToEnvoySqliteExecResponse> {
	Ok(v6::ToEnvoySqliteExecResponse {
		request_id: x.request_id,
		data: convert_sqlite_exec_response_v5_to_v6(x.data)?,
	})
}

pub fn convert_to_envoy_sqlite_execute_response_v5_to_v6(x: v5::ToEnvoySqliteExecuteResponse) -> Result<v6::ToEnvoySqliteExecuteResponse> {
	Ok(v6::ToEnvoySqliteExecuteResponse {
		request_id: x.request_id,
		data: convert_sqlite_execute_response_v5_to_v6(x.data)?,
	})
}

pub fn convert_to_envoy_v5_to_v6(x: v5::ToEnvoy) -> Result<v6::ToEnvoy> {
	Ok(match x {
		v5::ToEnvoy::ToEnvoyInit(v) => v6::ToEnvoy::ToEnvoyInit(convert_to_envoy_init_v5_to_v6(v)?),
		v5::ToEnvoy::ToEnvoyCommands(v) => v6::ToEnvoy::ToEnvoyCommands(convert_to_envoy_commands_v5_to_v6(v)?),
		v5::ToEnvoy::ToEnvoyAckEvents(v) => v6::ToEnvoy::ToEnvoyAckEvents(convert_to_envoy_ack_events_v5_to_v6(v)?),
		v5::ToEnvoy::ToEnvoyKvResponse(v) => v6::ToEnvoy::ToEnvoyKvResponse(convert_to_envoy_kv_response_v5_to_v6(v)?),
		v5::ToEnvoy::ToEnvoyTunnelMessage(v) => v6::ToEnvoy::ToEnvoyTunnelMessage(convert_to_envoy_tunnel_message_v5_to_v6(v)?),
		v5::ToEnvoy::ToEnvoyPing(v) => v6::ToEnvoy::ToEnvoyPing(convert_to_envoy_ping_v5_to_v6(v)?),
		v5::ToEnvoy::ToEnvoySqliteGetPagesResponse(v) => v6::ToEnvoy::ToEnvoySqliteGetPagesResponse(convert_to_envoy_sqlite_get_pages_response_v5_to_v6(v)?),
		v5::ToEnvoy::ToEnvoySqliteCommitResponse(v) => v6::ToEnvoy::ToEnvoySqliteCommitResponse(convert_to_envoy_sqlite_commit_response_v5_to_v6(v)?),
		v5::ToEnvoy::ToEnvoySqliteExecResponse(v) => v6::ToEnvoy::ToEnvoySqliteExecResponse(convert_to_envoy_sqlite_exec_response_v5_to_v6(v)?),
		v5::ToEnvoy::ToEnvoySqliteExecuteResponse(v) => v6::ToEnvoy::ToEnvoySqliteExecuteResponse(convert_to_envoy_sqlite_execute_response_v5_to_v6(v)?),
	})
}

pub fn convert_to_envoy_conn_ping_v5_to_v6(x: v5::ToEnvoyConnPing) -> Result<v6::ToEnvoyConnPing> {
	Ok(v6::ToEnvoyConnPing {
		gateway_id: x.gateway_id,
		request_id: x.request_id,
		ts: x.ts,
	})
}

pub fn convert_to_envoy_conn_v5_to_v6(x: v5::ToEnvoyConn) -> Result<v6::ToEnvoyConn> {
	Ok(match x {
		v5::ToEnvoyConn::ToEnvoyConnPing(v) => v6::ToEnvoyConn::ToEnvoyConnPing(convert_to_envoy_conn_ping_v5_to_v6(v)?),
		v5::ToEnvoyConn::ToEnvoyConnClose => v6::ToEnvoyConn::ToEnvoyConnClose,
		v5::ToEnvoyConn::ToEnvoyCommands(v) => v6::ToEnvoyConn::ToEnvoyCommands(convert_to_envoy_commands_v5_to_v6(v)?),
		v5::ToEnvoyConn::ToEnvoyAckEvents(v) => v6::ToEnvoyConn::ToEnvoyAckEvents(convert_to_envoy_ack_events_v5_to_v6(v)?),
		v5::ToEnvoyConn::ToEnvoyTunnelMessage(v) => v6::ToEnvoyConn::ToEnvoyTunnelMessage(convert_to_envoy_tunnel_message_v5_to_v6(v)?),
	})
}

pub fn convert_to_gateway_pong_v5_to_v6(x: v5::ToGatewayPong) -> Result<v6::ToGatewayPong> {
	Ok(v6::ToGatewayPong {
		request_id: x.request_id,
		ts: x.ts,
	})
}

pub fn convert_to_gateway_v5_to_v6(x: v5::ToGateway) -> Result<v6::ToGateway> {
	Ok(match x {
		v5::ToGateway::ToGatewayPong(v) => v6::ToGateway::ToGatewayPong(convert_to_gateway_pong_v5_to_v6(v)?),
		v5::ToGateway::ToRivetTunnelMessage(v) => v6::ToGateway::ToRivetTunnelMessage(convert_to_rivet_tunnel_message_v5_to_v6(v)?),
	})
}

pub fn convert_to_outbound_actor_start_v5_to_v6(x: v5::ToOutboundActorStart) -> Result<v6::ToOutboundActorStart> {
	Ok(v6::ToOutboundActorStart {
		namespace_id: x.namespace_id,
		pool_name: x.pool_name,
		checkpoint: convert_actor_checkpoint_v5_to_v6(x.checkpoint)?,
		actor_config: convert_actor_config_v5_to_v6(x.actor_config)?,
	})
}

pub fn convert_to_outbound_v5_to_v6(x: v5::ToOutbound) -> Result<v6::ToOutbound> {
	Ok(match x {
		v5::ToOutbound::ToOutboundActorStart(v) => v6::ToOutbound::ToOutboundActorStart(convert_to_outbound_actor_start_v5_to_v6(v)?),
	})
}
