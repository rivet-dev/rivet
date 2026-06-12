// @generated initial scaffold by scripts/vbare-gen-converters
// from: v5.bare, to: v4.bare
// Replace each todo!() with the migration semantics, then drop the @generated marker.

#![allow(dead_code, unused_variables)]

use anyhow::Result;

use crate::generated::{v4, v5};

pub fn convert_kv_metadata_v5_to_v4(x: v5::KvMetadata) -> Result<v4::KvMetadata> {
	Ok(v4::KvMetadata {
		version: x.version,
		update_ts: x.update_ts,
	})
}

pub fn convert_kv_list_range_query_v5_to_v4(
	x: v5::KvListRangeQuery,
) -> Result<v4::KvListRangeQuery> {
	Ok(v4::KvListRangeQuery {
		start: x.start,
		end: x.end,
		exclusive: x.exclusive,
	})
}

pub fn convert_kv_list_prefix_query_v5_to_v4(
	x: v5::KvListPrefixQuery,
) -> Result<v4::KvListPrefixQuery> {
	Ok(v4::KvListPrefixQuery { key: x.key })
}

pub fn convert_kv_list_query_v5_to_v4(x: v5::KvListQuery) -> Result<v4::KvListQuery> {
	Ok(match x {
		v5::KvListQuery::KvListAllQuery => v4::KvListQuery::KvListAllQuery,
		v5::KvListQuery::KvListRangeQuery(v) => {
			v4::KvListQuery::KvListRangeQuery(convert_kv_list_range_query_v5_to_v4(v)?)
		}
		v5::KvListQuery::KvListPrefixQuery(v) => {
			v4::KvListQuery::KvListPrefixQuery(convert_kv_list_prefix_query_v5_to_v4(v)?)
		}
	})
}

pub fn convert_kv_get_request_v5_to_v4(x: v5::KvGetRequest) -> Result<v4::KvGetRequest> {
	Ok(v4::KvGetRequest { keys: x.keys })
}

pub fn convert_kv_list_request_v5_to_v4(x: v5::KvListRequest) -> Result<v4::KvListRequest> {
	Ok(v4::KvListRequest {
		query: convert_kv_list_query_v5_to_v4(x.query)?,
		reverse: x.reverse,
		limit: x.limit,
	})
}

pub fn convert_kv_put_request_v5_to_v4(x: v5::KvPutRequest) -> Result<v4::KvPutRequest> {
	Ok(v4::KvPutRequest {
		keys: x.keys,
		values: x.values,
	})
}

pub fn convert_kv_delete_request_v5_to_v4(x: v5::KvDeleteRequest) -> Result<v4::KvDeleteRequest> {
	Ok(v4::KvDeleteRequest { keys: x.keys })
}

pub fn convert_kv_delete_range_request_v5_to_v4(
	x: v5::KvDeleteRangeRequest,
) -> Result<v4::KvDeleteRangeRequest> {
	Ok(v4::KvDeleteRangeRequest {
		start: x.start,
		end: x.end,
	})
}

pub fn convert_kv_error_response_v5_to_v4(x: v5::KvErrorResponse) -> Result<v4::KvErrorResponse> {
	Ok(v4::KvErrorResponse { message: x.message })
}

pub fn convert_kv_get_response_v5_to_v4(x: v5::KvGetResponse) -> Result<v4::KvGetResponse> {
	Ok(v4::KvGetResponse {
		keys: x.keys,
		values: x.values,
		metadata: x
			.metadata
			.into_iter()
			.map(|v| convert_kv_metadata_v5_to_v4(v))
			.collect::<Result<Vec<_>>>()?,
	})
}

pub fn convert_kv_list_response_v5_to_v4(x: v5::KvListResponse) -> Result<v4::KvListResponse> {
	Ok(v4::KvListResponse {
		keys: x.keys,
		values: x.values,
		metadata: x
			.metadata
			.into_iter()
			.map(|v| convert_kv_metadata_v5_to_v4(v))
			.collect::<Result<Vec<_>>>()?,
	})
}

pub fn convert_kv_request_data_v5_to_v4(x: v5::KvRequestData) -> Result<v4::KvRequestData> {
	Ok(match x {
		v5::KvRequestData::KvGetRequest(v) => {
			v4::KvRequestData::KvGetRequest(convert_kv_get_request_v5_to_v4(v)?)
		}
		v5::KvRequestData::KvListRequest(v) => {
			v4::KvRequestData::KvListRequest(convert_kv_list_request_v5_to_v4(v)?)
		}
		v5::KvRequestData::KvPutRequest(v) => {
			v4::KvRequestData::KvPutRequest(convert_kv_put_request_v5_to_v4(v)?)
		}
		v5::KvRequestData::KvDeleteRequest(v) => {
			v4::KvRequestData::KvDeleteRequest(convert_kv_delete_request_v5_to_v4(v)?)
		}
		v5::KvRequestData::KvDeleteRangeRequest(v) => {
			v4::KvRequestData::KvDeleteRangeRequest(convert_kv_delete_range_request_v5_to_v4(v)?)
		}
		v5::KvRequestData::KvDropRequest => v4::KvRequestData::KvDropRequest,
	})
}

pub fn convert_kv_response_data_v5_to_v4(x: v5::KvResponseData) -> Result<v4::KvResponseData> {
	Ok(match x {
		v5::KvResponseData::KvErrorResponse(v) => {
			v4::KvResponseData::KvErrorResponse(convert_kv_error_response_v5_to_v4(v)?)
		}
		v5::KvResponseData::KvGetResponse(v) => {
			v4::KvResponseData::KvGetResponse(convert_kv_get_response_v5_to_v4(v)?)
		}
		v5::KvResponseData::KvListResponse(v) => {
			v4::KvResponseData::KvListResponse(convert_kv_list_response_v5_to_v4(v)?)
		}
		v5::KvResponseData::KvPutResponse => v4::KvResponseData::KvPutResponse,
		v5::KvResponseData::KvDeleteResponse => v4::KvResponseData::KvDeleteResponse,
		v5::KvResponseData::KvDropResponse => v4::KvResponseData::KvDropResponse,
	})
}

pub fn convert_sqlite_dirty_page_v5_to_v4(x: v5::SqliteDirtyPage) -> Result<v4::SqliteDirtyPage> {
	Ok(v4::SqliteDirtyPage {
		pgno: x.pgno,
		bytes: x.bytes,
	})
}

pub fn convert_sqlite_fetched_page_v5_to_v4(
	x: v5::SqliteFetchedPage,
) -> Result<v4::SqliteFetchedPage> {
	Ok(v4::SqliteFetchedPage {
		pgno: x.pgno,
		bytes: x.bytes,
	})
}

pub fn convert_sqlite_get_pages_request_v5_to_v4(
	x: v5::SqliteGetPagesRequest,
) -> Result<v4::SqliteGetPagesRequest> {
	Ok(v4::SqliteGetPagesRequest {
		actor_id: x.actor_id,
		pgnos: x.pgnos,
		expected_generation: x.expected_generation,
		expected_head_txid: x.expected_head_txid,
	})
}

pub fn convert_sqlite_get_pages_ok_v5_to_v4(
	x: v5::SqliteGetPagesOk,
) -> Result<v4::SqliteGetPagesOk> {
	Ok(v4::SqliteGetPagesOk {
		pages: x
			.pages
			.into_iter()
			.map(|v| convert_sqlite_fetched_page_v5_to_v4(v))
			.collect::<Result<Vec<_>>>()?,
	})
}

pub fn convert_sqlite_error_response_v5_to_v4(
	x: v5::SqliteErrorResponse,
) -> Result<v4::SqliteErrorResponse> {
	Ok(v4::SqliteErrorResponse { message: x.message })
}

pub fn convert_sqlite_get_pages_response_v5_to_v4(
	x: v5::SqliteGetPagesResponse,
) -> Result<v4::SqliteGetPagesResponse> {
	Ok(match x {
		v5::SqliteGetPagesResponse::SqliteGetPagesOk(v) => {
			v4::SqliteGetPagesResponse::SqliteGetPagesOk(convert_sqlite_get_pages_ok_v5_to_v4(v)?)
		}
		v5::SqliteGetPagesResponse::SqliteErrorResponse(v) => {
			v4::SqliteGetPagesResponse::SqliteErrorResponse(convert_sqlite_error_response_v5_to_v4(
				v,
			)?)
		}
	})
}

pub fn convert_sqlite_commit_request_v5_to_v4(
	x: v5::SqliteCommitRequest,
) -> Result<v4::SqliteCommitRequest> {
	Ok(v4::SqliteCommitRequest {
		actor_id: x.actor_id,
		dirty_pages: x
			.dirty_pages
			.into_iter()
			.map(|v| convert_sqlite_dirty_page_v5_to_v4(v))
			.collect::<Result<Vec<_>>>()?,
		db_size_pages: x.db_size_pages,
		now_ms: x.now_ms,
		expected_generation: x.expected_generation,
		expected_head_txid: x.expected_head_txid,
	})
}

pub fn convert_sqlite_commit_response_v5_to_v4(
	x: v5::SqliteCommitResponse,
) -> Result<v4::SqliteCommitResponse> {
	Ok(match x {
		v5::SqliteCommitResponse::SqliteCommitOk(_) => {
			// v4 had no head_txid; drop the field on the way down.
			v4::SqliteCommitResponse::SqliteCommitOk
		}
		v5::SqliteCommitResponse::SqliteErrorResponse(v) => {
			v4::SqliteCommitResponse::SqliteErrorResponse(convert_sqlite_error_response_v5_to_v4(
				v,
			)?)
		}
	})
}

pub fn convert_sqlite_value_integer_v5_to_v4(
	x: v5::SqliteValueInteger,
) -> Result<v4::SqliteValueInteger> {
	Ok(v4::SqliteValueInteger { value: x.value })
}

pub fn convert_sqlite_value_float_v5_to_v4(
	x: v5::SqliteValueFloat,
) -> Result<v4::SqliteValueFloat> {
	Ok(v4::SqliteValueFloat { value: x.value })
}

pub fn convert_sqlite_value_text_v5_to_v4(x: v5::SqliteValueText) -> Result<v4::SqliteValueText> {
	Ok(v4::SqliteValueText { value: x.value })
}

pub fn convert_sqlite_value_blob_v5_to_v4(x: v5::SqliteValueBlob) -> Result<v4::SqliteValueBlob> {
	Ok(v4::SqliteValueBlob { value: x.value })
}

pub fn convert_sqlite_bind_param_v5_to_v4(x: v5::SqliteBindParam) -> Result<v4::SqliteBindParam> {
	Ok(match x {
		v5::SqliteBindParam::SqliteValueNull => v4::SqliteBindParam::SqliteValueNull,
		v5::SqliteBindParam::SqliteValueInteger(v) => {
			v4::SqliteBindParam::SqliteValueInteger(convert_sqlite_value_integer_v5_to_v4(v)?)
		}
		v5::SqliteBindParam::SqliteValueFloat(v) => {
			v4::SqliteBindParam::SqliteValueFloat(convert_sqlite_value_float_v5_to_v4(v)?)
		}
		v5::SqliteBindParam::SqliteValueText(v) => {
			v4::SqliteBindParam::SqliteValueText(convert_sqlite_value_text_v5_to_v4(v)?)
		}
		v5::SqliteBindParam::SqliteValueBlob(v) => {
			v4::SqliteBindParam::SqliteValueBlob(convert_sqlite_value_blob_v5_to_v4(v)?)
		}
	})
}

pub fn convert_sqlite_column_value_v5_to_v4(
	x: v5::SqliteColumnValue,
) -> Result<v4::SqliteColumnValue> {
	Ok(match x {
		v5::SqliteColumnValue::SqliteValueNull => v4::SqliteColumnValue::SqliteValueNull,
		v5::SqliteColumnValue::SqliteValueInteger(v) => {
			v4::SqliteColumnValue::SqliteValueInteger(convert_sqlite_value_integer_v5_to_v4(v)?)
		}
		v5::SqliteColumnValue::SqliteValueFloat(v) => {
			v4::SqliteColumnValue::SqliteValueFloat(convert_sqlite_value_float_v5_to_v4(v)?)
		}
		v5::SqliteColumnValue::SqliteValueText(v) => {
			v4::SqliteColumnValue::SqliteValueText(convert_sqlite_value_text_v5_to_v4(v)?)
		}
		v5::SqliteColumnValue::SqliteValueBlob(v) => {
			v4::SqliteColumnValue::SqliteValueBlob(convert_sqlite_value_blob_v5_to_v4(v)?)
		}
	})
}

pub fn convert_sqlite_query_result_v5_to_v4(
	x: v5::SqliteQueryResult,
) -> Result<v4::SqliteQueryResult> {
	Ok(v4::SqliteQueryResult {
		columns: x.columns,
		rows: x
			.rows
			.into_iter()
			.map(|v| {
				v.into_iter()
					.map(|v| convert_sqlite_column_value_v5_to_v4(v))
					.collect::<Result<Vec<_>>>()
			})
			.collect::<Result<Vec<_>>>()?,
	})
}

pub fn convert_sqlite_execute_result_v5_to_v4(
	x: v5::SqliteExecuteResult,
) -> Result<v4::SqliteExecuteResult> {
	Ok(v4::SqliteExecuteResult {
		columns: x.columns,
		rows: x
			.rows
			.into_iter()
			.map(|v| {
				v.into_iter()
					.map(|v| convert_sqlite_column_value_v5_to_v4(v))
					.collect::<Result<Vec<_>>>()
			})
			.collect::<Result<Vec<_>>>()?,
		changes: x.changes,
		last_insert_row_id: x.last_insert_row_id,
	})
}

pub fn convert_sqlite_exec_request_v5_to_v4(
	x: v5::SqliteExecRequest,
) -> Result<v4::SqliteExecRequest> {
	Ok(v4::SqliteExecRequest {
		namespace_id: x.namespace_id,
		actor_id: x.actor_id,
		generation: x.generation,
		sql: x.sql,
	})
}

pub fn convert_sqlite_execute_request_v5_to_v4(
	x: v5::SqliteExecuteRequest,
) -> Result<v4::SqliteExecuteRequest> {
	Ok(v4::SqliteExecuteRequest {
		namespace_id: x.namespace_id,
		actor_id: x.actor_id,
		generation: x.generation,
		sql: x.sql,
		params: x
			.params
			.map(|v| {
				v.into_iter()
					.map(|v| convert_sqlite_bind_param_v5_to_v4(v))
					.collect::<Result<Vec<_>>>()
			})
			.transpose()?,
	})
}

pub fn convert_sqlite_exec_ok_v5_to_v4(x: v5::SqliteExecOk) -> Result<v4::SqliteExecOk> {
	Ok(v4::SqliteExecOk {
		result: convert_sqlite_query_result_v5_to_v4(x.result)?,
	})
}

pub fn convert_sqlite_execute_ok_v5_to_v4(x: v5::SqliteExecuteOk) -> Result<v4::SqliteExecuteOk> {
	Ok(v4::SqliteExecuteOk {
		result: convert_sqlite_execute_result_v5_to_v4(x.result)?,
	})
}

pub fn convert_sqlite_exec_response_v5_to_v4(
	x: v5::SqliteExecResponse,
) -> Result<v4::SqliteExecResponse> {
	Ok(match x {
		v5::SqliteExecResponse::SqliteExecOk(v) => {
			v4::SqliteExecResponse::SqliteExecOk(convert_sqlite_exec_ok_v5_to_v4(v)?)
		}
		v5::SqliteExecResponse::SqliteErrorResponse(v) => {
			v4::SqliteExecResponse::SqliteErrorResponse(convert_sqlite_error_response_v5_to_v4(v)?)
		}
	})
}

pub fn convert_sqlite_execute_response_v5_to_v4(
	x: v5::SqliteExecuteResponse,
) -> Result<v4::SqliteExecuteResponse> {
	Ok(match x {
		v5::SqliteExecuteResponse::SqliteExecuteOk(v) => {
			v4::SqliteExecuteResponse::SqliteExecuteOk(convert_sqlite_execute_ok_v5_to_v4(v)?)
		}
		v5::SqliteExecuteResponse::SqliteErrorResponse(v) => {
			v4::SqliteExecuteResponse::SqliteErrorResponse(convert_sqlite_error_response_v5_to_v4(
				v,
			)?)
		}
	})
}

pub fn convert_stop_code_v5_to_v4(x: v5::StopCode) -> Result<v4::StopCode> {
	Ok(match x {
		v5::StopCode::Ok => v4::StopCode::Ok,
		v5::StopCode::Error => v4::StopCode::Error,
	})
}

pub fn convert_actor_name_v5_to_v4(x: v5::ActorName) -> Result<v4::ActorName> {
	Ok(v4::ActorName {
		metadata: x.metadata,
	})
}

pub fn convert_actor_config_v5_to_v4(x: v5::ActorConfig) -> Result<v4::ActorConfig> {
	Ok(v4::ActorConfig {
		name: x.name,
		key: x.key,
		create_ts: x.create_ts,
		input: x.input,
	})
}

pub fn convert_actor_checkpoint_v5_to_v4(x: v5::ActorCheckpoint) -> Result<v4::ActorCheckpoint> {
	Ok(v4::ActorCheckpoint {
		actor_id: x.actor_id,
		generation: x.generation,
		index: x.index,
	})
}

pub fn convert_actor_intent_v5_to_v4(x: v5::ActorIntent) -> Result<v4::ActorIntent> {
	Ok(match x {
		v5::ActorIntent::ActorIntentSleep => v4::ActorIntent::ActorIntentSleep,
		v5::ActorIntent::ActorIntentStop => v4::ActorIntent::ActorIntentStop,
	})
}

pub fn convert_actor_state_stopped_v5_to_v4(
	x: v5::ActorStateStopped,
) -> Result<v4::ActorStateStopped> {
	Ok(v4::ActorStateStopped {
		code: convert_stop_code_v5_to_v4(x.code)?,
		message: x.message,
	})
}

pub fn convert_actor_state_v5_to_v4(x: v5::ActorState) -> Result<v4::ActorState> {
	Ok(match x {
		v5::ActorState::ActorStateRunning => v4::ActorState::ActorStateRunning,
		v5::ActorState::ActorStateStopped(v) => {
			v4::ActorState::ActorStateStopped(convert_actor_state_stopped_v5_to_v4(v)?)
		}
	})
}

pub fn convert_event_actor_intent_v5_to_v4(
	x: v5::EventActorIntent,
) -> Result<v4::EventActorIntent> {
	Ok(v4::EventActorIntent {
		intent: convert_actor_intent_v5_to_v4(x.intent)?,
	})
}

pub fn convert_event_actor_state_update_v5_to_v4(
	x: v5::EventActorStateUpdate,
) -> Result<v4::EventActorStateUpdate> {
	Ok(v4::EventActorStateUpdate {
		state: convert_actor_state_v5_to_v4(x.state)?,
	})
}

pub fn convert_event_actor_set_alarm_v5_to_v4(
	x: v5::EventActorSetAlarm,
) -> Result<v4::EventActorSetAlarm> {
	Ok(v4::EventActorSetAlarm {
		alarm_ts: x.alarm_ts,
	})
}

pub fn convert_event_v5_to_v4(x: v5::Event) -> Result<v4::Event> {
	Ok(match x {
		v5::Event::EventActorIntent(v) => {
			v4::Event::EventActorIntent(convert_event_actor_intent_v5_to_v4(v)?)
		}
		v5::Event::EventActorStateUpdate(v) => {
			v4::Event::EventActorStateUpdate(convert_event_actor_state_update_v5_to_v4(v)?)
		}
		v5::Event::EventActorSetAlarm(v) => {
			v4::Event::EventActorSetAlarm(convert_event_actor_set_alarm_v5_to_v4(v)?)
		}
	})
}

pub fn convert_event_wrapper_v5_to_v4(x: v5::EventWrapper) -> Result<v4::EventWrapper> {
	Ok(v4::EventWrapper {
		checkpoint: convert_actor_checkpoint_v5_to_v4(x.checkpoint)?,
		inner: convert_event_v5_to_v4(x.inner)?,
	})
}

pub fn convert_preloaded_kv_entry_v5_to_v4(
	x: v5::PreloadedKvEntry,
) -> Result<v4::PreloadedKvEntry> {
	Ok(v4::PreloadedKvEntry {
		key: x.key,
		value: x.value,
		metadata: convert_kv_metadata_v5_to_v4(x.metadata)?,
	})
}

pub fn convert_preloaded_kv_v5_to_v4(x: v5::PreloadedKv) -> Result<v4::PreloadedKv> {
	Ok(v4::PreloadedKv {
		entries: x
			.entries
			.into_iter()
			.map(|v| convert_preloaded_kv_entry_v5_to_v4(v))
			.collect::<Result<Vec<_>>>()?,
		requested_get_keys: x.requested_get_keys,
		requested_prefixes: x.requested_prefixes,
	})
}

pub fn convert_hibernating_request_v5_to_v4(
	x: v5::HibernatingRequest,
) -> Result<v4::HibernatingRequest> {
	Ok(v4::HibernatingRequest {
		gateway_id: x.gateway_id,
		request_id: x.request_id,
	})
}

pub fn convert_command_start_actor_v5_to_v4(
	x: v5::CommandStartActor,
) -> Result<v4::CommandStartActor> {
	Ok(v4::CommandStartActor {
		config: convert_actor_config_v5_to_v4(x.config)?,
		hibernating_requests: x
			.hibernating_requests
			.into_iter()
			.map(|v| convert_hibernating_request_v5_to_v4(v))
			.collect::<Result<Vec<_>>>()?,
		preloaded_kv: x
			.preloaded_kv
			.map(|v| convert_preloaded_kv_v5_to_v4(v))
			.transpose()?,
	})
}

pub fn convert_stop_actor_reason_v5_to_v4(x: v5::StopActorReason) -> Result<v4::StopActorReason> {
	Ok(match x {
		v5::StopActorReason::SleepIntent => v4::StopActorReason::SleepIntent,
		v5::StopActorReason::StopIntent => v4::StopActorReason::StopIntent,
		v5::StopActorReason::Destroy => v4::StopActorReason::Destroy,
		v5::StopActorReason::GoingAway => v4::StopActorReason::GoingAway,
		v5::StopActorReason::Lost => v4::StopActorReason::Lost,
	})
}

pub fn convert_command_stop_actor_v5_to_v4(
	x: v5::CommandStopActor,
) -> Result<v4::CommandStopActor> {
	Ok(v4::CommandStopActor {
		reason: convert_stop_actor_reason_v5_to_v4(x.reason)?,
	})
}

pub fn convert_command_v5_to_v4(x: v5::Command) -> Result<v4::Command> {
	Ok(match x {
		v5::Command::CommandStartActor(v) => {
			v4::Command::CommandStartActor(convert_command_start_actor_v5_to_v4(v)?)
		}
		v5::Command::CommandStopActor(v) => {
			v4::Command::CommandStopActor(convert_command_stop_actor_v5_to_v4(v)?)
		}
	})
}

pub fn convert_command_wrapper_v5_to_v4(x: v5::CommandWrapper) -> Result<v4::CommandWrapper> {
	Ok(v4::CommandWrapper {
		checkpoint: convert_actor_checkpoint_v5_to_v4(x.checkpoint)?,
		inner: convert_command_v5_to_v4(x.inner)?,
	})
}

pub fn convert_actor_command_key_data_v5_to_v4(
	x: v5::ActorCommandKeyData,
) -> Result<v4::ActorCommandKeyData> {
	Ok(match x {
		v5::ActorCommandKeyData::CommandStartActor(v) => {
			v4::ActorCommandKeyData::CommandStartActor(convert_command_start_actor_v5_to_v4(v)?)
		}
		v5::ActorCommandKeyData::CommandStopActor(v) => {
			v4::ActorCommandKeyData::CommandStopActor(convert_command_stop_actor_v5_to_v4(v)?)
		}
	})
}

pub fn convert_message_id_v5_to_v4(x: v5::MessageId) -> Result<v4::MessageId> {
	Ok(v4::MessageId {
		gateway_id: x.gateway_id,
		request_id: x.request_id,
		message_index: x.message_index,
	})
}

pub fn convert_to_envoy_request_start_v5_to_v4(
	x: v5::ToEnvoyRequestStart,
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

pub fn convert_to_envoy_request_chunk_v5_to_v4(
	x: v5::ToEnvoyRequestChunk,
) -> Result<v4::ToEnvoyRequestChunk> {
	Ok(v4::ToEnvoyRequestChunk {
		body: x.body,
		finish: x.finish,
	})
}

pub fn convert_to_rivet_response_start_v5_to_v4(
	x: v5::ToRivetResponseStart,
) -> Result<v4::ToRivetResponseStart> {
	Ok(v4::ToRivetResponseStart {
		status: x.status,
		headers: x.headers,
		body: x.body,
		stream: x.stream,
	})
}

pub fn convert_to_rivet_response_chunk_v5_to_v4(
	x: v5::ToRivetResponseChunk,
) -> Result<v4::ToRivetResponseChunk> {
	Ok(v4::ToRivetResponseChunk {
		body: x.body,
		finish: x.finish,
	})
}

pub fn convert_to_envoy_web_socket_open_v5_to_v4(
	x: v5::ToEnvoyWebSocketOpen,
) -> Result<v4::ToEnvoyWebSocketOpen> {
	Ok(v4::ToEnvoyWebSocketOpen {
		actor_id: x.actor_id,
		path: x.path,
		headers: x.headers,
	})
}

pub fn convert_to_envoy_web_socket_message_v5_to_v4(
	x: v5::ToEnvoyWebSocketMessage,
) -> Result<v4::ToEnvoyWebSocketMessage> {
	Ok(v4::ToEnvoyWebSocketMessage {
		data: x.data,
		binary: x.binary,
	})
}

pub fn convert_to_envoy_web_socket_close_v5_to_v4(
	x: v5::ToEnvoyWebSocketClose,
) -> Result<v4::ToEnvoyWebSocketClose> {
	Ok(v4::ToEnvoyWebSocketClose {
		code: x.code,
		reason: x.reason,
	})
}

pub fn convert_to_rivet_web_socket_open_v5_to_v4(
	x: v5::ToRivetWebSocketOpen,
) -> Result<v4::ToRivetWebSocketOpen> {
	Ok(v4::ToRivetWebSocketOpen {
		can_hibernate: x.can_hibernate,
	})
}

pub fn convert_to_rivet_web_socket_message_v5_to_v4(
	x: v5::ToRivetWebSocketMessage,
) -> Result<v4::ToRivetWebSocketMessage> {
	Ok(v4::ToRivetWebSocketMessage {
		data: x.data,
		binary: x.binary,
	})
}

pub fn convert_to_rivet_web_socket_message_ack_v5_to_v4(
	x: v5::ToRivetWebSocketMessageAck,
) -> Result<v4::ToRivetWebSocketMessageAck> {
	Ok(v4::ToRivetWebSocketMessageAck { index: x.index })
}

pub fn convert_to_rivet_web_socket_close_v5_to_v4(
	x: v5::ToRivetWebSocketClose,
) -> Result<v4::ToRivetWebSocketClose> {
	Ok(v4::ToRivetWebSocketClose {
		code: x.code,
		reason: x.reason,
		hibernate: x.hibernate,
	})
}

pub fn convert_to_rivet_tunnel_message_kind_v5_to_v4(
	x: v5::ToRivetTunnelMessageKind,
) -> Result<v4::ToRivetTunnelMessageKind> {
	Ok(match x {
		v5::ToRivetTunnelMessageKind::ToRivetResponseStart(v) => {
			v4::ToRivetTunnelMessageKind::ToRivetResponseStart(
				convert_to_rivet_response_start_v5_to_v4(v)?,
			)
		}
		v5::ToRivetTunnelMessageKind::ToRivetResponseChunk(v) => {
			v4::ToRivetTunnelMessageKind::ToRivetResponseChunk(
				convert_to_rivet_response_chunk_v5_to_v4(v)?,
			)
		}
		v5::ToRivetTunnelMessageKind::ToRivetResponseAbort => {
			v4::ToRivetTunnelMessageKind::ToRivetResponseAbort
		}
		v5::ToRivetTunnelMessageKind::ToRivetWebSocketOpen(v) => {
			v4::ToRivetTunnelMessageKind::ToRivetWebSocketOpen(
				convert_to_rivet_web_socket_open_v5_to_v4(v)?,
			)
		}
		v5::ToRivetTunnelMessageKind::ToRivetWebSocketMessage(v) => {
			v4::ToRivetTunnelMessageKind::ToRivetWebSocketMessage(
				convert_to_rivet_web_socket_message_v5_to_v4(v)?,
			)
		}
		v5::ToRivetTunnelMessageKind::ToRivetWebSocketMessageAck(v) => {
			v4::ToRivetTunnelMessageKind::ToRivetWebSocketMessageAck(
				convert_to_rivet_web_socket_message_ack_v5_to_v4(v)?,
			)
		}
		v5::ToRivetTunnelMessageKind::ToRivetWebSocketClose(v) => {
			v4::ToRivetTunnelMessageKind::ToRivetWebSocketClose(
				convert_to_rivet_web_socket_close_v5_to_v4(v)?,
			)
		}
	})
}

pub fn convert_to_rivet_tunnel_message_v5_to_v4(
	x: v5::ToRivetTunnelMessage,
) -> Result<v4::ToRivetTunnelMessage> {
	Ok(v4::ToRivetTunnelMessage {
		message_id: convert_message_id_v5_to_v4(x.message_id)?,
		message_kind: convert_to_rivet_tunnel_message_kind_v5_to_v4(x.message_kind)?,
	})
}

pub fn convert_to_envoy_tunnel_message_kind_v5_to_v4(
	x: v5::ToEnvoyTunnelMessageKind,
) -> Result<v4::ToEnvoyTunnelMessageKind> {
	Ok(match x {
		v5::ToEnvoyTunnelMessageKind::ToEnvoyRequestStart(v) => {
			v4::ToEnvoyTunnelMessageKind::ToEnvoyRequestStart(
				convert_to_envoy_request_start_v5_to_v4(v)?,
			)
		}
		v5::ToEnvoyTunnelMessageKind::ToEnvoyRequestChunk(v) => {
			v4::ToEnvoyTunnelMessageKind::ToEnvoyRequestChunk(
				convert_to_envoy_request_chunk_v5_to_v4(v)?,
			)
		}
		v5::ToEnvoyTunnelMessageKind::ToEnvoyRequestAbort => {
			v4::ToEnvoyTunnelMessageKind::ToEnvoyRequestAbort
		}
		v5::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketOpen(v) => {
			v4::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketOpen(
				convert_to_envoy_web_socket_open_v5_to_v4(v)?,
			)
		}
		v5::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketMessage(v) => {
			v4::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketMessage(
				convert_to_envoy_web_socket_message_v5_to_v4(v)?,
			)
		}
		v5::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketClose(v) => {
			v4::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketClose(
				convert_to_envoy_web_socket_close_v5_to_v4(v)?,
			)
		}
	})
}

pub fn convert_to_envoy_tunnel_message_v5_to_v4(
	x: v5::ToEnvoyTunnelMessage,
) -> Result<v4::ToEnvoyTunnelMessage> {
	Ok(v4::ToEnvoyTunnelMessage {
		message_id: convert_message_id_v5_to_v4(x.message_id)?,
		message_kind: convert_to_envoy_tunnel_message_kind_v5_to_v4(x.message_kind)?,
	})
}

pub fn convert_to_envoy_ping_v5_to_v4(x: v5::ToEnvoyPing) -> Result<v4::ToEnvoyPing> {
	Ok(v4::ToEnvoyPing { ts: x.ts })
}

pub fn convert_to_rivet_metadata_v5_to_v4(x: v5::ToRivetMetadata) -> Result<v4::ToRivetMetadata> {
	Ok(v4::ToRivetMetadata {
		prepopulate_actor_names: x
			.prepopulate_actor_names
			.map(|v| {
				v.into_iter()
					.map(|(k, v)| -> Result<_> { Ok((k, convert_actor_name_v5_to_v4(v)?)) })
					.collect::<Result<_>>()
			})
			.transpose()?,
		metadata: x.metadata,
	})
}

pub fn convert_to_rivet_events_v5_to_v4(x: v5::ToRivetEvents) -> Result<v4::ToRivetEvents> {
	Ok(x.into_iter()
		.map(|v| convert_event_wrapper_v5_to_v4(v))
		.collect::<Result<Vec<_>>>()?)
}

pub fn convert_to_rivet_ack_commands_v5_to_v4(
	x: v5::ToRivetAckCommands,
) -> Result<v4::ToRivetAckCommands> {
	Ok(v4::ToRivetAckCommands {
		last_command_checkpoints: x
			.last_command_checkpoints
			.into_iter()
			.map(|v| convert_actor_checkpoint_v5_to_v4(v))
			.collect::<Result<Vec<_>>>()?,
	})
}

pub fn convert_to_rivet_pong_v5_to_v4(x: v5::ToRivetPong) -> Result<v4::ToRivetPong> {
	Ok(v4::ToRivetPong { ts: x.ts })
}

pub fn convert_to_rivet_kv_request_v5_to_v4(
	x: v5::ToRivetKvRequest,
) -> Result<v4::ToRivetKvRequest> {
	Ok(v4::ToRivetKvRequest {
		actor_id: x.actor_id,
		request_id: x.request_id,
		data: convert_kv_request_data_v5_to_v4(x.data)?,
	})
}

pub fn convert_to_rivet_sqlite_get_pages_request_v5_to_v4(
	x: v5::ToRivetSqliteGetPagesRequest,
) -> Result<v4::ToRivetSqliteGetPagesRequest> {
	Ok(v4::ToRivetSqliteGetPagesRequest {
		request_id: x.request_id,
		data: convert_sqlite_get_pages_request_v5_to_v4(x.data)?,
	})
}

pub fn convert_to_rivet_sqlite_commit_request_v5_to_v4(
	x: v5::ToRivetSqliteCommitRequest,
) -> Result<v4::ToRivetSqliteCommitRequest> {
	Ok(v4::ToRivetSqliteCommitRequest {
		request_id: x.request_id,
		data: convert_sqlite_commit_request_v5_to_v4(x.data)?,
	})
}

pub fn convert_to_rivet_sqlite_exec_request_v5_to_v4(
	x: v5::ToRivetSqliteExecRequest,
) -> Result<v4::ToRivetSqliteExecRequest> {
	Ok(v4::ToRivetSqliteExecRequest {
		request_id: x.request_id,
		data: convert_sqlite_exec_request_v5_to_v4(x.data)?,
	})
}

pub fn convert_to_rivet_sqlite_execute_request_v5_to_v4(
	x: v5::ToRivetSqliteExecuteRequest,
) -> Result<v4::ToRivetSqliteExecuteRequest> {
	Ok(v4::ToRivetSqliteExecuteRequest {
		request_id: x.request_id,
		data: convert_sqlite_execute_request_v5_to_v4(x.data)?,
	})
}

pub fn convert_to_rivet_v5_to_v4(x: v5::ToRivet) -> Result<v4::ToRivet> {
	Ok(match x {
		v5::ToRivet::ToRivetMetadata(v) => {
			v4::ToRivet::ToRivetMetadata(convert_to_rivet_metadata_v5_to_v4(v)?)
		}
		v5::ToRivet::ToRivetEvents(v) => {
			v4::ToRivet::ToRivetEvents(convert_to_rivet_events_v5_to_v4(v)?)
		}
		v5::ToRivet::ToRivetAckCommands(v) => {
			v4::ToRivet::ToRivetAckCommands(convert_to_rivet_ack_commands_v5_to_v4(v)?)
		}
		v5::ToRivet::ToRivetStopping => v4::ToRivet::ToRivetStopping,
		v5::ToRivet::ToRivetPong(v) => v4::ToRivet::ToRivetPong(convert_to_rivet_pong_v5_to_v4(v)?),
		v5::ToRivet::ToRivetKvRequest(v) => {
			v4::ToRivet::ToRivetKvRequest(convert_to_rivet_kv_request_v5_to_v4(v)?)
		}
		v5::ToRivet::ToRivetTunnelMessage(v) => {
			v4::ToRivet::ToRivetTunnelMessage(convert_to_rivet_tunnel_message_v5_to_v4(v)?)
		}
		v5::ToRivet::ToRivetSqliteGetPagesRequest(v) => v4::ToRivet::ToRivetSqliteGetPagesRequest(
			convert_to_rivet_sqlite_get_pages_request_v5_to_v4(v)?,
		),
		v5::ToRivet::ToRivetSqliteCommitRequest(v) => v4::ToRivet::ToRivetSqliteCommitRequest(
			convert_to_rivet_sqlite_commit_request_v5_to_v4(v)?,
		),
		v5::ToRivet::ToRivetSqliteExecRequest(v) => {
			v4::ToRivet::ToRivetSqliteExecRequest(convert_to_rivet_sqlite_exec_request_v5_to_v4(v)?)
		}
		v5::ToRivet::ToRivetSqliteExecuteRequest(v) => v4::ToRivet::ToRivetSqliteExecuteRequest(
			convert_to_rivet_sqlite_execute_request_v5_to_v4(v)?,
		),
	})
}

pub fn convert_protocol_metadata_v5_to_v4(x: v5::ProtocolMetadata) -> Result<v4::ProtocolMetadata> {
	Ok(v4::ProtocolMetadata {
		envoy_lost_threshold: x.envoy_lost_threshold,
		actor_stop_threshold: x.actor_stop_threshold,
		max_response_payload_size: x.max_response_payload_size,
	})
}

pub fn convert_to_envoy_init_v5_to_v4(x: v5::ToEnvoyInit) -> Result<v4::ToEnvoyInit> {
	Ok(v4::ToEnvoyInit {
		metadata: convert_protocol_metadata_v5_to_v4(x.metadata)?,
	})
}

pub fn convert_to_envoy_commands_v5_to_v4(x: v5::ToEnvoyCommands) -> Result<v4::ToEnvoyCommands> {
	Ok(x.into_iter()
		.map(|v| convert_command_wrapper_v5_to_v4(v))
		.collect::<Result<Vec<_>>>()?)
}

pub fn convert_to_envoy_ack_events_v5_to_v4(
	x: v5::ToEnvoyAckEvents,
) -> Result<v4::ToEnvoyAckEvents> {
	Ok(v4::ToEnvoyAckEvents {
		last_event_checkpoints: x
			.last_event_checkpoints
			.into_iter()
			.map(|v| convert_actor_checkpoint_v5_to_v4(v))
			.collect::<Result<Vec<_>>>()?,
	})
}

pub fn convert_to_envoy_kv_response_v5_to_v4(
	x: v5::ToEnvoyKvResponse,
) -> Result<v4::ToEnvoyKvResponse> {
	Ok(v4::ToEnvoyKvResponse {
		request_id: x.request_id,
		data: convert_kv_response_data_v5_to_v4(x.data)?,
	})
}

pub fn convert_to_envoy_sqlite_get_pages_response_v5_to_v4(
	x: v5::ToEnvoySqliteGetPagesResponse,
) -> Result<v4::ToEnvoySqliteGetPagesResponse> {
	Ok(v4::ToEnvoySqliteGetPagesResponse {
		request_id: x.request_id,
		data: convert_sqlite_get_pages_response_v5_to_v4(x.data)?,
	})
}

pub fn convert_to_envoy_sqlite_commit_response_v5_to_v4(
	x: v5::ToEnvoySqliteCommitResponse,
) -> Result<v4::ToEnvoySqliteCommitResponse> {
	Ok(v4::ToEnvoySqliteCommitResponse {
		request_id: x.request_id,
		data: convert_sqlite_commit_response_v5_to_v4(x.data)?,
	})
}

pub fn convert_to_envoy_sqlite_exec_response_v5_to_v4(
	x: v5::ToEnvoySqliteExecResponse,
) -> Result<v4::ToEnvoySqliteExecResponse> {
	Ok(v4::ToEnvoySqliteExecResponse {
		request_id: x.request_id,
		data: convert_sqlite_exec_response_v5_to_v4(x.data)?,
	})
}

pub fn convert_to_envoy_sqlite_execute_response_v5_to_v4(
	x: v5::ToEnvoySqliteExecuteResponse,
) -> Result<v4::ToEnvoySqliteExecuteResponse> {
	Ok(v4::ToEnvoySqliteExecuteResponse {
		request_id: x.request_id,
		data: convert_sqlite_execute_response_v5_to_v4(x.data)?,
	})
}

pub fn convert_to_envoy_v5_to_v4(x: v5::ToEnvoy) -> Result<v4::ToEnvoy> {
	Ok(match x {
		v5::ToEnvoy::ToEnvoyInit(v) => v4::ToEnvoy::ToEnvoyInit(convert_to_envoy_init_v5_to_v4(v)?),
		v5::ToEnvoy::ToEnvoyCommands(v) => {
			v4::ToEnvoy::ToEnvoyCommands(convert_to_envoy_commands_v5_to_v4(v)?)
		}
		v5::ToEnvoy::ToEnvoyAckEvents(v) => {
			v4::ToEnvoy::ToEnvoyAckEvents(convert_to_envoy_ack_events_v5_to_v4(v)?)
		}
		v5::ToEnvoy::ToEnvoyKvResponse(v) => {
			v4::ToEnvoy::ToEnvoyKvResponse(convert_to_envoy_kv_response_v5_to_v4(v)?)
		}
		v5::ToEnvoy::ToEnvoyTunnelMessage(v) => {
			v4::ToEnvoy::ToEnvoyTunnelMessage(convert_to_envoy_tunnel_message_v5_to_v4(v)?)
		}
		v5::ToEnvoy::ToEnvoyPing(v) => v4::ToEnvoy::ToEnvoyPing(convert_to_envoy_ping_v5_to_v4(v)?),
		v5::ToEnvoy::ToEnvoySqliteGetPagesResponse(v) => {
			v4::ToEnvoy::ToEnvoySqliteGetPagesResponse(
				convert_to_envoy_sqlite_get_pages_response_v5_to_v4(v)?,
			)
		}
		v5::ToEnvoy::ToEnvoySqliteCommitResponse(v) => v4::ToEnvoy::ToEnvoySqliteCommitResponse(
			convert_to_envoy_sqlite_commit_response_v5_to_v4(v)?,
		),
		v5::ToEnvoy::ToEnvoySqliteExecResponse(v) => v4::ToEnvoy::ToEnvoySqliteExecResponse(
			convert_to_envoy_sqlite_exec_response_v5_to_v4(v)?,
		),
		v5::ToEnvoy::ToEnvoySqliteExecuteResponse(v) => v4::ToEnvoy::ToEnvoySqliteExecuteResponse(
			convert_to_envoy_sqlite_execute_response_v5_to_v4(v)?,
		),
	})
}

pub fn convert_to_envoy_conn_ping_v5_to_v4(x: v5::ToEnvoyConnPing) -> Result<v4::ToEnvoyConnPing> {
	Ok(v4::ToEnvoyConnPing {
		gateway_id: x.gateway_id,
		request_id: x.request_id,
		ts: x.ts,
	})
}

pub fn convert_to_envoy_conn_v5_to_v4(x: v5::ToEnvoyConn) -> Result<v4::ToEnvoyConn> {
	Ok(match x {
		v5::ToEnvoyConn::ToEnvoyConnPing(v) => {
			v4::ToEnvoyConn::ToEnvoyConnPing(convert_to_envoy_conn_ping_v5_to_v4(v)?)
		}
		v5::ToEnvoyConn::ToEnvoyConnClose => v4::ToEnvoyConn::ToEnvoyConnClose,
		v5::ToEnvoyConn::ToEnvoyCommands(v) => {
			v4::ToEnvoyConn::ToEnvoyCommands(convert_to_envoy_commands_v5_to_v4(v)?)
		}
		v5::ToEnvoyConn::ToEnvoyAckEvents(v) => {
			v4::ToEnvoyConn::ToEnvoyAckEvents(convert_to_envoy_ack_events_v5_to_v4(v)?)
		}
		v5::ToEnvoyConn::ToEnvoyTunnelMessage(v) => {
			v4::ToEnvoyConn::ToEnvoyTunnelMessage(convert_to_envoy_tunnel_message_v5_to_v4(v)?)
		}
	})
}

pub fn convert_to_gateway_pong_v5_to_v4(x: v5::ToGatewayPong) -> Result<v4::ToGatewayPong> {
	Ok(v4::ToGatewayPong {
		request_id: x.request_id,
		ts: x.ts,
	})
}

pub fn convert_to_gateway_v5_to_v4(x: v5::ToGateway) -> Result<v4::ToGateway> {
	Ok(match x {
		v5::ToGateway::ToGatewayPong(v) => {
			v4::ToGateway::ToGatewayPong(convert_to_gateway_pong_v5_to_v4(v)?)
		}
		v5::ToGateway::ToRivetTunnelMessage(v) => {
			v4::ToGateway::ToRivetTunnelMessage(convert_to_rivet_tunnel_message_v5_to_v4(v)?)
		}
	})
}

pub fn convert_to_outbound_actor_start_v5_to_v4(
	x: v5::ToOutboundActorStart,
) -> Result<v4::ToOutboundActorStart> {
	Ok(v4::ToOutboundActorStart {
		namespace_id: x.namespace_id,
		pool_name: x.pool_name,
		checkpoint: convert_actor_checkpoint_v5_to_v4(x.checkpoint)?,
		actor_config: convert_actor_config_v5_to_v4(x.actor_config)?,
	})
}

pub fn convert_to_outbound_v5_to_v4(x: v5::ToOutbound) -> Result<v4::ToOutbound> {
	Ok(match x {
		v5::ToOutbound::ToOutboundActorStart(v) => {
			v4::ToOutbound::ToOutboundActorStart(convert_to_outbound_actor_start_v5_to_v4(v)?)
		}
	})
}
