// @generated initial scaffold by scripts/vbare-gen-converters
// from: v4.bare, to: v5.bare
// Replace each todo!() with the migration semantics, then drop the @generated marker.

#![allow(dead_code, unused_variables)]

use anyhow::Result;

use crate::generated::{v4, v5};

pub fn convert_kv_metadata_v4_to_v5(x: v4::KvMetadata) -> Result<v5::KvMetadata> {
	Ok(v5::KvMetadata {
		version: x.version,
		update_ts: x.update_ts,
	})
}

pub fn convert_kv_list_range_query_v4_to_v5(
	x: v4::KvListRangeQuery,
) -> Result<v5::KvListRangeQuery> {
	Ok(v5::KvListRangeQuery {
		start: x.start,
		end: x.end,
		exclusive: x.exclusive,
	})
}

pub fn convert_kv_list_prefix_query_v4_to_v5(
	x: v4::KvListPrefixQuery,
) -> Result<v5::KvListPrefixQuery> {
	Ok(v5::KvListPrefixQuery { key: x.key })
}

pub fn convert_kv_list_query_v4_to_v5(x: v4::KvListQuery) -> Result<v5::KvListQuery> {
	Ok(match x {
		v4::KvListQuery::KvListAllQuery => v5::KvListQuery::KvListAllQuery,
		v4::KvListQuery::KvListRangeQuery(v) => {
			v5::KvListQuery::KvListRangeQuery(convert_kv_list_range_query_v4_to_v5(v)?)
		}
		v4::KvListQuery::KvListPrefixQuery(v) => {
			v5::KvListQuery::KvListPrefixQuery(convert_kv_list_prefix_query_v4_to_v5(v)?)
		}
	})
}

pub fn convert_kv_get_request_v4_to_v5(x: v4::KvGetRequest) -> Result<v5::KvGetRequest> {
	Ok(v5::KvGetRequest { keys: x.keys })
}

pub fn convert_kv_list_request_v4_to_v5(x: v4::KvListRequest) -> Result<v5::KvListRequest> {
	Ok(v5::KvListRequest {
		query: convert_kv_list_query_v4_to_v5(x.query)?,
		reverse: x.reverse,
		limit: x.limit,
	})
}

pub fn convert_kv_put_request_v4_to_v5(x: v4::KvPutRequest) -> Result<v5::KvPutRequest> {
	Ok(v5::KvPutRequest {
		keys: x.keys,
		values: x.values,
	})
}

pub fn convert_kv_delete_request_v4_to_v5(x: v4::KvDeleteRequest) -> Result<v5::KvDeleteRequest> {
	Ok(v5::KvDeleteRequest { keys: x.keys })
}

pub fn convert_kv_delete_range_request_v4_to_v5(
	x: v4::KvDeleteRangeRequest,
) -> Result<v5::KvDeleteRangeRequest> {
	Ok(v5::KvDeleteRangeRequest {
		start: x.start,
		end: x.end,
	})
}

pub fn convert_kv_error_response_v4_to_v5(x: v4::KvErrorResponse) -> Result<v5::KvErrorResponse> {
	Ok(v5::KvErrorResponse { message: x.message })
}

pub fn convert_kv_get_response_v4_to_v5(x: v4::KvGetResponse) -> Result<v5::KvGetResponse> {
	Ok(v5::KvGetResponse {
		keys: x.keys,
		values: x.values,
		metadata: x
			.metadata
			.into_iter()
			.map(|v| convert_kv_metadata_v4_to_v5(v))
			.collect::<Result<Vec<_>>>()?,
	})
}

pub fn convert_kv_list_response_v4_to_v5(x: v4::KvListResponse) -> Result<v5::KvListResponse> {
	Ok(v5::KvListResponse {
		keys: x.keys,
		values: x.values,
		metadata: x
			.metadata
			.into_iter()
			.map(|v| convert_kv_metadata_v4_to_v5(v))
			.collect::<Result<Vec<_>>>()?,
	})
}

pub fn convert_kv_request_data_v4_to_v5(x: v4::KvRequestData) -> Result<v5::KvRequestData> {
	Ok(match x {
		v4::KvRequestData::KvGetRequest(v) => {
			v5::KvRequestData::KvGetRequest(convert_kv_get_request_v4_to_v5(v)?)
		}
		v4::KvRequestData::KvListRequest(v) => {
			v5::KvRequestData::KvListRequest(convert_kv_list_request_v4_to_v5(v)?)
		}
		v4::KvRequestData::KvPutRequest(v) => {
			v5::KvRequestData::KvPutRequest(convert_kv_put_request_v4_to_v5(v)?)
		}
		v4::KvRequestData::KvDeleteRequest(v) => {
			v5::KvRequestData::KvDeleteRequest(convert_kv_delete_request_v4_to_v5(v)?)
		}
		v4::KvRequestData::KvDeleteRangeRequest(v) => {
			v5::KvRequestData::KvDeleteRangeRequest(convert_kv_delete_range_request_v4_to_v5(v)?)
		}
		v4::KvRequestData::KvDropRequest => v5::KvRequestData::KvDropRequest,
	})
}

pub fn convert_kv_response_data_v4_to_v5(x: v4::KvResponseData) -> Result<v5::KvResponseData> {
	Ok(match x {
		v4::KvResponseData::KvErrorResponse(v) => {
			v5::KvResponseData::KvErrorResponse(convert_kv_error_response_v4_to_v5(v)?)
		}
		v4::KvResponseData::KvGetResponse(v) => {
			v5::KvResponseData::KvGetResponse(convert_kv_get_response_v4_to_v5(v)?)
		}
		v4::KvResponseData::KvListResponse(v) => {
			v5::KvResponseData::KvListResponse(convert_kv_list_response_v4_to_v5(v)?)
		}
		v4::KvResponseData::KvPutResponse => v5::KvResponseData::KvPutResponse,
		v4::KvResponseData::KvDeleteResponse => v5::KvResponseData::KvDeleteResponse,
		v4::KvResponseData::KvDropResponse => v5::KvResponseData::KvDropResponse,
	})
}

pub fn convert_sqlite_dirty_page_v4_to_v5(x: v4::SqliteDirtyPage) -> Result<v5::SqliteDirtyPage> {
	Ok(v5::SqliteDirtyPage {
		pgno: x.pgno,
		bytes: x.bytes,
	})
}

pub fn convert_sqlite_fetched_page_v4_to_v5(
	x: v4::SqliteFetchedPage,
) -> Result<v5::SqliteFetchedPage> {
	Ok(v5::SqliteFetchedPage {
		pgno: x.pgno,
		bytes: x.bytes,
	})
}

pub fn convert_sqlite_get_pages_request_v4_to_v5(
	x: v4::SqliteGetPagesRequest,
) -> Result<v5::SqliteGetPagesRequest> {
	Ok(v5::SqliteGetPagesRequest {
		actor_id: x.actor_id,
		pgnos: x.pgnos,
		expected_generation: x.expected_generation,
		expected_head_txid: x.expected_head_txid,
	})
}

pub fn convert_sqlite_get_pages_ok_v4_to_v5(
	x: v4::SqliteGetPagesOk,
) -> Result<v5::SqliteGetPagesOk> {
	Ok(v5::SqliteGetPagesOk {
		pages: x
			.pages
			.into_iter()
			.map(|v| convert_sqlite_fetched_page_v4_to_v5(v))
			.collect::<Result<Vec<_>>>()?,
		// v4 had no head_txid in the response; v5 callers treat None as "unknown".
		head_txid: None,
	})
}

pub fn convert_sqlite_error_response_v4_to_v5(
	x: v4::SqliteErrorResponse,
) -> Result<v5::SqliteErrorResponse> {
	Ok(v5::SqliteErrorResponse {
		// v4 errors were untyped strings; lift to the canonical RivetError shape.
		group: "core".to_string(),
		code: "internal_error".to_string(),
		message: x.message,
	})
}

pub fn convert_sqlite_get_pages_response_v4_to_v5(
	x: v4::SqliteGetPagesResponse,
) -> Result<v5::SqliteGetPagesResponse> {
	Ok(match x {
		v4::SqliteGetPagesResponse::SqliteGetPagesOk(v) => {
			v5::SqliteGetPagesResponse::SqliteGetPagesOk(convert_sqlite_get_pages_ok_v4_to_v5(v)?)
		}
		v4::SqliteGetPagesResponse::SqliteErrorResponse(v) => {
			v5::SqliteGetPagesResponse::SqliteErrorResponse(convert_sqlite_error_response_v4_to_v5(
				v,
			)?)
		}
	})
}

pub fn convert_sqlite_commit_request_v4_to_v5(
	x: v4::SqliteCommitRequest,
) -> Result<v5::SqliteCommitRequest> {
	Ok(v5::SqliteCommitRequest {
		actor_id: x.actor_id,
		dirty_pages: x
			.dirty_pages
			.into_iter()
			.map(|v| convert_sqlite_dirty_page_v4_to_v5(v))
			.collect::<Result<Vec<_>>>()?,
		db_size_pages: x.db_size_pages,
		now_ms: x.now_ms,
		expected_generation: x.expected_generation,
		expected_head_txid: x.expected_head_txid,
	})
}

pub fn convert_sqlite_commit_response_v4_to_v5(
	x: v4::SqliteCommitResponse,
) -> Result<v5::SqliteCommitResponse> {
	Ok(match x {
		v4::SqliteCommitResponse::SqliteCommitOk => {
			// v4's SqliteCommitOk was a void variant; v5 carries an optional head_txid.
			v5::SqliteCommitResponse::SqliteCommitOk(v5::SqliteCommitOk { head_txid: None })
		}
		v4::SqliteCommitResponse::SqliteErrorResponse(v) => {
			v5::SqliteCommitResponse::SqliteErrorResponse(convert_sqlite_error_response_v4_to_v5(
				v,
			)?)
		}
	})
}

pub fn convert_sqlite_value_integer_v4_to_v5(
	x: v4::SqliteValueInteger,
) -> Result<v5::SqliteValueInteger> {
	Ok(v5::SqliteValueInteger { value: x.value })
}

pub fn convert_sqlite_value_float_v4_to_v5(
	x: v4::SqliteValueFloat,
) -> Result<v5::SqliteValueFloat> {
	Ok(v5::SqliteValueFloat { value: x.value })
}

pub fn convert_sqlite_value_text_v4_to_v5(x: v4::SqliteValueText) -> Result<v5::SqliteValueText> {
	Ok(v5::SqliteValueText { value: x.value })
}

pub fn convert_sqlite_value_blob_v4_to_v5(x: v4::SqliteValueBlob) -> Result<v5::SqliteValueBlob> {
	Ok(v5::SqliteValueBlob { value: x.value })
}

pub fn convert_sqlite_bind_param_v4_to_v5(x: v4::SqliteBindParam) -> Result<v5::SqliteBindParam> {
	Ok(match x {
		v4::SqliteBindParam::SqliteValueNull => v5::SqliteBindParam::SqliteValueNull,
		v4::SqliteBindParam::SqliteValueInteger(v) => {
			v5::SqliteBindParam::SqliteValueInteger(convert_sqlite_value_integer_v4_to_v5(v)?)
		}
		v4::SqliteBindParam::SqliteValueFloat(v) => {
			v5::SqliteBindParam::SqliteValueFloat(convert_sqlite_value_float_v4_to_v5(v)?)
		}
		v4::SqliteBindParam::SqliteValueText(v) => {
			v5::SqliteBindParam::SqliteValueText(convert_sqlite_value_text_v4_to_v5(v)?)
		}
		v4::SqliteBindParam::SqliteValueBlob(v) => {
			v5::SqliteBindParam::SqliteValueBlob(convert_sqlite_value_blob_v4_to_v5(v)?)
		}
	})
}

pub fn convert_sqlite_column_value_v4_to_v5(
	x: v4::SqliteColumnValue,
) -> Result<v5::SqliteColumnValue> {
	Ok(match x {
		v4::SqliteColumnValue::SqliteValueNull => v5::SqliteColumnValue::SqliteValueNull,
		v4::SqliteColumnValue::SqliteValueInteger(v) => {
			v5::SqliteColumnValue::SqliteValueInteger(convert_sqlite_value_integer_v4_to_v5(v)?)
		}
		v4::SqliteColumnValue::SqliteValueFloat(v) => {
			v5::SqliteColumnValue::SqliteValueFloat(convert_sqlite_value_float_v4_to_v5(v)?)
		}
		v4::SqliteColumnValue::SqliteValueText(v) => {
			v5::SqliteColumnValue::SqliteValueText(convert_sqlite_value_text_v4_to_v5(v)?)
		}
		v4::SqliteColumnValue::SqliteValueBlob(v) => {
			v5::SqliteColumnValue::SqliteValueBlob(convert_sqlite_value_blob_v4_to_v5(v)?)
		}
	})
}

pub fn convert_sqlite_query_result_v4_to_v5(
	x: v4::SqliteQueryResult,
) -> Result<v5::SqliteQueryResult> {
	Ok(v5::SqliteQueryResult {
		columns: x.columns,
		rows: x
			.rows
			.into_iter()
			.map(|v| {
				v.into_iter()
					.map(|v| convert_sqlite_column_value_v4_to_v5(v))
					.collect::<Result<Vec<_>>>()
			})
			.collect::<Result<Vec<_>>>()?,
	})
}

pub fn convert_sqlite_execute_result_v4_to_v5(
	x: v4::SqliteExecuteResult,
) -> Result<v5::SqliteExecuteResult> {
	Ok(v5::SqliteExecuteResult {
		columns: x.columns,
		rows: x
			.rows
			.into_iter()
			.map(|v| {
				v.into_iter()
					.map(|v| convert_sqlite_column_value_v4_to_v5(v))
					.collect::<Result<Vec<_>>>()
			})
			.collect::<Result<Vec<_>>>()?,
		changes: x.changes,
		last_insert_row_id: x.last_insert_row_id,
	})
}

pub fn convert_sqlite_exec_request_v4_to_v5(
	x: v4::SqliteExecRequest,
) -> Result<v5::SqliteExecRequest> {
	Ok(v5::SqliteExecRequest {
		namespace_id: x.namespace_id,
		actor_id: x.actor_id,
		generation: x.generation,
		sql: x.sql,
	})
}

pub fn convert_sqlite_execute_request_v4_to_v5(
	x: v4::SqliteExecuteRequest,
) -> Result<v5::SqliteExecuteRequest> {
	Ok(v5::SqliteExecuteRequest {
		namespace_id: x.namespace_id,
		actor_id: x.actor_id,
		generation: x.generation,
		sql: x.sql,
		params: x
			.params
			.map(|v| {
				v.into_iter()
					.map(|v| convert_sqlite_bind_param_v4_to_v5(v))
					.collect::<Result<Vec<_>>>()
			})
			.transpose()?,
	})
}

pub fn convert_sqlite_exec_ok_v4_to_v5(x: v4::SqliteExecOk) -> Result<v5::SqliteExecOk> {
	Ok(v5::SqliteExecOk {
		result: convert_sqlite_query_result_v4_to_v5(x.result)?,
	})
}

pub fn convert_sqlite_execute_ok_v4_to_v5(x: v4::SqliteExecuteOk) -> Result<v5::SqliteExecuteOk> {
	Ok(v5::SqliteExecuteOk {
		result: convert_sqlite_execute_result_v4_to_v5(x.result)?,
	})
}

pub fn convert_sqlite_exec_response_v4_to_v5(
	x: v4::SqliteExecResponse,
) -> Result<v5::SqliteExecResponse> {
	Ok(match x {
		v4::SqliteExecResponse::SqliteExecOk(v) => {
			v5::SqliteExecResponse::SqliteExecOk(convert_sqlite_exec_ok_v4_to_v5(v)?)
		}
		v4::SqliteExecResponse::SqliteErrorResponse(v) => {
			v5::SqliteExecResponse::SqliteErrorResponse(convert_sqlite_error_response_v4_to_v5(v)?)
		}
	})
}

pub fn convert_sqlite_execute_response_v4_to_v5(
	x: v4::SqliteExecuteResponse,
) -> Result<v5::SqliteExecuteResponse> {
	Ok(match x {
		v4::SqliteExecuteResponse::SqliteExecuteOk(v) => {
			v5::SqliteExecuteResponse::SqliteExecuteOk(convert_sqlite_execute_ok_v4_to_v5(v)?)
		}
		v4::SqliteExecuteResponse::SqliteErrorResponse(v) => {
			v5::SqliteExecuteResponse::SqliteErrorResponse(convert_sqlite_error_response_v4_to_v5(
				v,
			)?)
		}
	})
}

pub fn convert_stop_code_v4_to_v5(x: v4::StopCode) -> Result<v5::StopCode> {
	Ok(match x {
		v4::StopCode::Ok => v5::StopCode::Ok,
		v4::StopCode::Error => v5::StopCode::Error,
	})
}

pub fn convert_actor_name_v4_to_v5(x: v4::ActorName) -> Result<v5::ActorName> {
	Ok(v5::ActorName {
		metadata: x.metadata,
	})
}

pub fn convert_actor_config_v4_to_v5(x: v4::ActorConfig) -> Result<v5::ActorConfig> {
	Ok(v5::ActorConfig {
		name: x.name,
		key: x.key,
		create_ts: x.create_ts,
		input: x.input,
	})
}

pub fn convert_actor_checkpoint_v4_to_v5(x: v4::ActorCheckpoint) -> Result<v5::ActorCheckpoint> {
	Ok(v5::ActorCheckpoint {
		actor_id: x.actor_id,
		generation: x.generation,
		index: x.index,
	})
}

pub fn convert_actor_intent_v4_to_v5(x: v4::ActorIntent) -> Result<v5::ActorIntent> {
	Ok(match x {
		v4::ActorIntent::ActorIntentSleep => v5::ActorIntent::ActorIntentSleep,
		v4::ActorIntent::ActorIntentStop => v5::ActorIntent::ActorIntentStop,
	})
}

pub fn convert_actor_state_stopped_v4_to_v5(
	x: v4::ActorStateStopped,
) -> Result<v5::ActorStateStopped> {
	Ok(v5::ActorStateStopped {
		code: convert_stop_code_v4_to_v5(x.code)?,
		message: x.message,
	})
}

pub fn convert_actor_state_v4_to_v5(x: v4::ActorState) -> Result<v5::ActorState> {
	Ok(match x {
		v4::ActorState::ActorStateRunning => v5::ActorState::ActorStateRunning,
		v4::ActorState::ActorStateStopped(v) => {
			v5::ActorState::ActorStateStopped(convert_actor_state_stopped_v4_to_v5(v)?)
		}
	})
}

pub fn convert_event_actor_intent_v4_to_v5(
	x: v4::EventActorIntent,
) -> Result<v5::EventActorIntent> {
	Ok(v5::EventActorIntent {
		intent: convert_actor_intent_v4_to_v5(x.intent)?,
	})
}

pub fn convert_event_actor_state_update_v4_to_v5(
	x: v4::EventActorStateUpdate,
) -> Result<v5::EventActorStateUpdate> {
	Ok(v5::EventActorStateUpdate {
		state: convert_actor_state_v4_to_v5(x.state)?,
	})
}

pub fn convert_event_actor_set_alarm_v4_to_v5(
	x: v4::EventActorSetAlarm,
) -> Result<v5::EventActorSetAlarm> {
	Ok(v5::EventActorSetAlarm {
		alarm_ts: x.alarm_ts,
	})
}

pub fn convert_event_v4_to_v5(x: v4::Event) -> Result<v5::Event> {
	Ok(match x {
		v4::Event::EventActorIntent(v) => {
			v5::Event::EventActorIntent(convert_event_actor_intent_v4_to_v5(v)?)
		}
		v4::Event::EventActorStateUpdate(v) => {
			v5::Event::EventActorStateUpdate(convert_event_actor_state_update_v4_to_v5(v)?)
		}
		v4::Event::EventActorSetAlarm(v) => {
			v5::Event::EventActorSetAlarm(convert_event_actor_set_alarm_v4_to_v5(v)?)
		}
	})
}

pub fn convert_event_wrapper_v4_to_v5(x: v4::EventWrapper) -> Result<v5::EventWrapper> {
	Ok(v5::EventWrapper {
		checkpoint: convert_actor_checkpoint_v4_to_v5(x.checkpoint)?,
		inner: convert_event_v4_to_v5(x.inner)?,
	})
}

pub fn convert_preloaded_kv_entry_v4_to_v5(
	x: v4::PreloadedKvEntry,
) -> Result<v5::PreloadedKvEntry> {
	Ok(v5::PreloadedKvEntry {
		key: x.key,
		value: x.value,
		metadata: convert_kv_metadata_v4_to_v5(x.metadata)?,
	})
}

pub fn convert_preloaded_kv_v4_to_v5(x: v4::PreloadedKv) -> Result<v5::PreloadedKv> {
	Ok(v5::PreloadedKv {
		entries: x
			.entries
			.into_iter()
			.map(|v| convert_preloaded_kv_entry_v4_to_v5(v))
			.collect::<Result<Vec<_>>>()?,
		requested_get_keys: x.requested_get_keys,
		requested_prefixes: x.requested_prefixes,
	})
}

pub fn convert_hibernating_request_v4_to_v5(
	x: v4::HibernatingRequest,
) -> Result<v5::HibernatingRequest> {
	Ok(v5::HibernatingRequest {
		gateway_id: x.gateway_id,
		request_id: x.request_id,
	})
}

pub fn convert_command_start_actor_v4_to_v5(
	x: v4::CommandStartActor,
) -> Result<v5::CommandStartActor> {
	Ok(v5::CommandStartActor {
		config: convert_actor_config_v4_to_v5(x.config)?,
		hibernating_requests: x
			.hibernating_requests
			.into_iter()
			.map(|v| convert_hibernating_request_v4_to_v5(v))
			.collect::<Result<Vec<_>>>()?,
		preloaded_kv: x
			.preloaded_kv
			.map(|v| convert_preloaded_kv_v4_to_v5(v))
			.transpose()?,
	})
}

pub fn convert_stop_actor_reason_v4_to_v5(x: v4::StopActorReason) -> Result<v5::StopActorReason> {
	Ok(match x {
		v4::StopActorReason::SleepIntent => v5::StopActorReason::SleepIntent,
		v4::StopActorReason::StopIntent => v5::StopActorReason::StopIntent,
		v4::StopActorReason::Destroy => v5::StopActorReason::Destroy,
		v4::StopActorReason::GoingAway => v5::StopActorReason::GoingAway,
		v4::StopActorReason::Lost => v5::StopActorReason::Lost,
	})
}

pub fn convert_command_stop_actor_v4_to_v5(
	x: v4::CommandStopActor,
) -> Result<v5::CommandStopActor> {
	Ok(v5::CommandStopActor {
		reason: convert_stop_actor_reason_v4_to_v5(x.reason)?,
	})
}

pub fn convert_command_v4_to_v5(x: v4::Command) -> Result<v5::Command> {
	Ok(match x {
		v4::Command::CommandStartActor(v) => {
			v5::Command::CommandStartActor(convert_command_start_actor_v4_to_v5(v)?)
		}
		v4::Command::CommandStopActor(v) => {
			v5::Command::CommandStopActor(convert_command_stop_actor_v4_to_v5(v)?)
		}
	})
}

pub fn convert_command_wrapper_v4_to_v5(x: v4::CommandWrapper) -> Result<v5::CommandWrapper> {
	Ok(v5::CommandWrapper {
		checkpoint: convert_actor_checkpoint_v4_to_v5(x.checkpoint)?,
		inner: convert_command_v4_to_v5(x.inner)?,
	})
}

pub fn convert_actor_command_key_data_v4_to_v5(
	x: v4::ActorCommandKeyData,
) -> Result<v5::ActorCommandKeyData> {
	Ok(match x {
		v4::ActorCommandKeyData::CommandStartActor(v) => {
			v5::ActorCommandKeyData::CommandStartActor(convert_command_start_actor_v4_to_v5(v)?)
		}
		v4::ActorCommandKeyData::CommandStopActor(v) => {
			v5::ActorCommandKeyData::CommandStopActor(convert_command_stop_actor_v4_to_v5(v)?)
		}
	})
}

pub fn convert_message_id_v4_to_v5(x: v4::MessageId) -> Result<v5::MessageId> {
	Ok(v5::MessageId {
		gateway_id: x.gateway_id,
		request_id: x.request_id,
		message_index: x.message_index,
	})
}

pub fn convert_to_envoy_request_start_v4_to_v5(
	x: v4::ToEnvoyRequestStart,
) -> Result<v5::ToEnvoyRequestStart> {
	Ok(v5::ToEnvoyRequestStart {
		actor_id: x.actor_id,
		method: x.method,
		path: x.path,
		headers: x.headers,
		body: x.body,
		stream: x.stream,
	})
}

pub fn convert_to_envoy_request_chunk_v4_to_v5(
	x: v4::ToEnvoyRequestChunk,
) -> Result<v5::ToEnvoyRequestChunk> {
	Ok(v5::ToEnvoyRequestChunk {
		body: x.body,
		finish: x.finish,
	})
}

pub fn convert_to_rivet_response_start_v4_to_v5(
	x: v4::ToRivetResponseStart,
) -> Result<v5::ToRivetResponseStart> {
	Ok(v5::ToRivetResponseStart {
		status: x.status,
		headers: x.headers,
		body: x.body,
		stream: x.stream,
	})
}

pub fn convert_to_rivet_response_chunk_v4_to_v5(
	x: v4::ToRivetResponseChunk,
) -> Result<v5::ToRivetResponseChunk> {
	Ok(v5::ToRivetResponseChunk {
		body: x.body,
		finish: x.finish,
	})
}

pub fn convert_to_envoy_web_socket_open_v4_to_v5(
	x: v4::ToEnvoyWebSocketOpen,
) -> Result<v5::ToEnvoyWebSocketOpen> {
	Ok(v5::ToEnvoyWebSocketOpen {
		actor_id: x.actor_id,
		path: x.path,
		headers: x.headers,
	})
}

pub fn convert_to_envoy_web_socket_message_v4_to_v5(
	x: v4::ToEnvoyWebSocketMessage,
) -> Result<v5::ToEnvoyWebSocketMessage> {
	Ok(v5::ToEnvoyWebSocketMessage {
		data: x.data,
		binary: x.binary,
	})
}

pub fn convert_to_envoy_web_socket_close_v4_to_v5(
	x: v4::ToEnvoyWebSocketClose,
) -> Result<v5::ToEnvoyWebSocketClose> {
	Ok(v5::ToEnvoyWebSocketClose {
		code: x.code,
		reason: x.reason,
	})
}

pub fn convert_to_rivet_web_socket_open_v4_to_v5(
	x: v4::ToRivetWebSocketOpen,
) -> Result<v5::ToRivetWebSocketOpen> {
	Ok(v5::ToRivetWebSocketOpen {
		can_hibernate: x.can_hibernate,
	})
}

pub fn convert_to_rivet_web_socket_message_v4_to_v5(
	x: v4::ToRivetWebSocketMessage,
) -> Result<v5::ToRivetWebSocketMessage> {
	Ok(v5::ToRivetWebSocketMessage {
		data: x.data,
		binary: x.binary,
	})
}

pub fn convert_to_rivet_web_socket_message_ack_v4_to_v5(
	x: v4::ToRivetWebSocketMessageAck,
) -> Result<v5::ToRivetWebSocketMessageAck> {
	Ok(v5::ToRivetWebSocketMessageAck { index: x.index })
}

pub fn convert_to_rivet_web_socket_close_v4_to_v5(
	x: v4::ToRivetWebSocketClose,
) -> Result<v5::ToRivetWebSocketClose> {
	Ok(v5::ToRivetWebSocketClose {
		code: x.code,
		reason: x.reason,
		hibernate: x.hibernate,
	})
}

pub fn convert_to_rivet_tunnel_message_kind_v4_to_v5(
	x: v4::ToRivetTunnelMessageKind,
) -> Result<v5::ToRivetTunnelMessageKind> {
	Ok(match x {
		v4::ToRivetTunnelMessageKind::ToRivetResponseStart(v) => {
			v5::ToRivetTunnelMessageKind::ToRivetResponseStart(
				convert_to_rivet_response_start_v4_to_v5(v)?,
			)
		}
		v4::ToRivetTunnelMessageKind::ToRivetResponseChunk(v) => {
			v5::ToRivetTunnelMessageKind::ToRivetResponseChunk(
				convert_to_rivet_response_chunk_v4_to_v5(v)?,
			)
		}
		v4::ToRivetTunnelMessageKind::ToRivetResponseAbort => {
			v5::ToRivetTunnelMessageKind::ToRivetResponseAbort
		}
		v4::ToRivetTunnelMessageKind::ToRivetWebSocketOpen(v) => {
			v5::ToRivetTunnelMessageKind::ToRivetWebSocketOpen(
				convert_to_rivet_web_socket_open_v4_to_v5(v)?,
			)
		}
		v4::ToRivetTunnelMessageKind::ToRivetWebSocketMessage(v) => {
			v5::ToRivetTunnelMessageKind::ToRivetWebSocketMessage(
				convert_to_rivet_web_socket_message_v4_to_v5(v)?,
			)
		}
		v4::ToRivetTunnelMessageKind::ToRivetWebSocketMessageAck(v) => {
			v5::ToRivetTunnelMessageKind::ToRivetWebSocketMessageAck(
				convert_to_rivet_web_socket_message_ack_v4_to_v5(v)?,
			)
		}
		v4::ToRivetTunnelMessageKind::ToRivetWebSocketClose(v) => {
			v5::ToRivetTunnelMessageKind::ToRivetWebSocketClose(
				convert_to_rivet_web_socket_close_v4_to_v5(v)?,
			)
		}
	})
}

pub fn convert_to_rivet_tunnel_message_v4_to_v5(
	x: v4::ToRivetTunnelMessage,
) -> Result<v5::ToRivetTunnelMessage> {
	Ok(v5::ToRivetTunnelMessage {
		message_id: convert_message_id_v4_to_v5(x.message_id)?,
		message_kind: convert_to_rivet_tunnel_message_kind_v4_to_v5(x.message_kind)?,
	})
}

pub fn convert_to_envoy_tunnel_message_kind_v4_to_v5(
	x: v4::ToEnvoyTunnelMessageKind,
) -> Result<v5::ToEnvoyTunnelMessageKind> {
	Ok(match x {
		v4::ToEnvoyTunnelMessageKind::ToEnvoyRequestStart(v) => {
			v5::ToEnvoyTunnelMessageKind::ToEnvoyRequestStart(
				convert_to_envoy_request_start_v4_to_v5(v)?,
			)
		}
		v4::ToEnvoyTunnelMessageKind::ToEnvoyRequestChunk(v) => {
			v5::ToEnvoyTunnelMessageKind::ToEnvoyRequestChunk(
				convert_to_envoy_request_chunk_v4_to_v5(v)?,
			)
		}
		v4::ToEnvoyTunnelMessageKind::ToEnvoyRequestAbort => {
			v5::ToEnvoyTunnelMessageKind::ToEnvoyRequestAbort
		}
		v4::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketOpen(v) => {
			v5::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketOpen(
				convert_to_envoy_web_socket_open_v4_to_v5(v)?,
			)
		}
		v4::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketMessage(v) => {
			v5::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketMessage(
				convert_to_envoy_web_socket_message_v4_to_v5(v)?,
			)
		}
		v4::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketClose(v) => {
			v5::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketClose(
				convert_to_envoy_web_socket_close_v4_to_v5(v)?,
			)
		}
	})
}

pub fn convert_to_envoy_tunnel_message_v4_to_v5(
	x: v4::ToEnvoyTunnelMessage,
) -> Result<v5::ToEnvoyTunnelMessage> {
	Ok(v5::ToEnvoyTunnelMessage {
		message_id: convert_message_id_v4_to_v5(x.message_id)?,
		message_kind: convert_to_envoy_tunnel_message_kind_v4_to_v5(x.message_kind)?,
	})
}

pub fn convert_to_envoy_ping_v4_to_v5(x: v4::ToEnvoyPing) -> Result<v5::ToEnvoyPing> {
	Ok(v5::ToEnvoyPing { ts: x.ts })
}

pub fn convert_to_rivet_metadata_v4_to_v5(x: v4::ToRivetMetadata) -> Result<v5::ToRivetMetadata> {
	Ok(v5::ToRivetMetadata {
		prepopulate_actor_names: x
			.prepopulate_actor_names
			.map(|v| {
				v.into_iter()
					.map(|(k, v)| -> Result<_> { Ok((k, convert_actor_name_v4_to_v5(v)?)) })
					.collect::<Result<_>>()
			})
			.transpose()?,
		metadata: x.metadata,
	})
}

pub fn convert_to_rivet_events_v4_to_v5(x: v4::ToRivetEvents) -> Result<v5::ToRivetEvents> {
	Ok(x.into_iter()
		.map(|v| convert_event_wrapper_v4_to_v5(v))
		.collect::<Result<Vec<_>>>()?)
}

pub fn convert_to_rivet_ack_commands_v4_to_v5(
	x: v4::ToRivetAckCommands,
) -> Result<v5::ToRivetAckCommands> {
	Ok(v5::ToRivetAckCommands {
		last_command_checkpoints: x
			.last_command_checkpoints
			.into_iter()
			.map(|v| convert_actor_checkpoint_v4_to_v5(v))
			.collect::<Result<Vec<_>>>()?,
	})
}

pub fn convert_to_rivet_pong_v4_to_v5(x: v4::ToRivetPong) -> Result<v5::ToRivetPong> {
	Ok(v5::ToRivetPong { ts: x.ts })
}

pub fn convert_to_rivet_kv_request_v4_to_v5(
	x: v4::ToRivetKvRequest,
) -> Result<v5::ToRivetKvRequest> {
	Ok(v5::ToRivetKvRequest {
		actor_id: x.actor_id,
		request_id: x.request_id,
		data: convert_kv_request_data_v4_to_v5(x.data)?,
	})
}

pub fn convert_to_rivet_sqlite_get_pages_request_v4_to_v5(
	x: v4::ToRivetSqliteGetPagesRequest,
) -> Result<v5::ToRivetSqliteGetPagesRequest> {
	Ok(v5::ToRivetSqliteGetPagesRequest {
		request_id: x.request_id,
		data: convert_sqlite_get_pages_request_v4_to_v5(x.data)?,
	})
}

pub fn convert_to_rivet_sqlite_commit_request_v4_to_v5(
	x: v4::ToRivetSqliteCommitRequest,
) -> Result<v5::ToRivetSqliteCommitRequest> {
	Ok(v5::ToRivetSqliteCommitRequest {
		request_id: x.request_id,
		data: convert_sqlite_commit_request_v4_to_v5(x.data)?,
	})
}

pub fn convert_to_rivet_sqlite_exec_request_v4_to_v5(
	x: v4::ToRivetSqliteExecRequest,
) -> Result<v5::ToRivetSqliteExecRequest> {
	Ok(v5::ToRivetSqliteExecRequest {
		request_id: x.request_id,
		data: convert_sqlite_exec_request_v4_to_v5(x.data)?,
	})
}

pub fn convert_to_rivet_sqlite_execute_request_v4_to_v5(
	x: v4::ToRivetSqliteExecuteRequest,
) -> Result<v5::ToRivetSqliteExecuteRequest> {
	Ok(v5::ToRivetSqliteExecuteRequest {
		request_id: x.request_id,
		data: convert_sqlite_execute_request_v4_to_v5(x.data)?,
	})
}

pub fn convert_to_rivet_v4_to_v5(x: v4::ToRivet) -> Result<v5::ToRivet> {
	Ok(match x {
		v4::ToRivet::ToRivetMetadata(v) => {
			v5::ToRivet::ToRivetMetadata(convert_to_rivet_metadata_v4_to_v5(v)?)
		}
		v4::ToRivet::ToRivetEvents(v) => {
			v5::ToRivet::ToRivetEvents(convert_to_rivet_events_v4_to_v5(v)?)
		}
		v4::ToRivet::ToRivetAckCommands(v) => {
			v5::ToRivet::ToRivetAckCommands(convert_to_rivet_ack_commands_v4_to_v5(v)?)
		}
		v4::ToRivet::ToRivetStopping => v5::ToRivet::ToRivetStopping,
		v4::ToRivet::ToRivetPong(v) => v5::ToRivet::ToRivetPong(convert_to_rivet_pong_v4_to_v5(v)?),
		v4::ToRivet::ToRivetKvRequest(v) => {
			v5::ToRivet::ToRivetKvRequest(convert_to_rivet_kv_request_v4_to_v5(v)?)
		}
		v4::ToRivet::ToRivetTunnelMessage(v) => {
			v5::ToRivet::ToRivetTunnelMessage(convert_to_rivet_tunnel_message_v4_to_v5(v)?)
		}
		v4::ToRivet::ToRivetSqliteGetPagesRequest(v) => v5::ToRivet::ToRivetSqliteGetPagesRequest(
			convert_to_rivet_sqlite_get_pages_request_v4_to_v5(v)?,
		),
		v4::ToRivet::ToRivetSqliteCommitRequest(v) => v5::ToRivet::ToRivetSqliteCommitRequest(
			convert_to_rivet_sqlite_commit_request_v4_to_v5(v)?,
		),
		v4::ToRivet::ToRivetSqliteExecRequest(v) => {
			v5::ToRivet::ToRivetSqliteExecRequest(convert_to_rivet_sqlite_exec_request_v4_to_v5(v)?)
		}
		v4::ToRivet::ToRivetSqliteExecuteRequest(v) => v5::ToRivet::ToRivetSqliteExecuteRequest(
			convert_to_rivet_sqlite_execute_request_v4_to_v5(v)?,
		),
	})
}

pub fn convert_protocol_metadata_v4_to_v5(x: v4::ProtocolMetadata) -> Result<v5::ProtocolMetadata> {
	Ok(v5::ProtocolMetadata {
		envoy_lost_threshold: x.envoy_lost_threshold,
		actor_stop_threshold: x.actor_stop_threshold,
		max_response_payload_size: x.max_response_payload_size,
	})
}

pub fn convert_to_envoy_init_v4_to_v5(x: v4::ToEnvoyInit) -> Result<v5::ToEnvoyInit> {
	Ok(v5::ToEnvoyInit {
		metadata: convert_protocol_metadata_v4_to_v5(x.metadata)?,
	})
}

pub fn convert_to_envoy_commands_v4_to_v5(x: v4::ToEnvoyCommands) -> Result<v5::ToEnvoyCommands> {
	Ok(x.into_iter()
		.map(|v| convert_command_wrapper_v4_to_v5(v))
		.collect::<Result<Vec<_>>>()?)
}

pub fn convert_to_envoy_ack_events_v4_to_v5(
	x: v4::ToEnvoyAckEvents,
) -> Result<v5::ToEnvoyAckEvents> {
	Ok(v5::ToEnvoyAckEvents {
		last_event_checkpoints: x
			.last_event_checkpoints
			.into_iter()
			.map(|v| convert_actor_checkpoint_v4_to_v5(v))
			.collect::<Result<Vec<_>>>()?,
	})
}

pub fn convert_to_envoy_kv_response_v4_to_v5(
	x: v4::ToEnvoyKvResponse,
) -> Result<v5::ToEnvoyKvResponse> {
	Ok(v5::ToEnvoyKvResponse {
		request_id: x.request_id,
		data: convert_kv_response_data_v4_to_v5(x.data)?,
	})
}

pub fn convert_to_envoy_sqlite_get_pages_response_v4_to_v5(
	x: v4::ToEnvoySqliteGetPagesResponse,
) -> Result<v5::ToEnvoySqliteGetPagesResponse> {
	Ok(v5::ToEnvoySqliteGetPagesResponse {
		request_id: x.request_id,
		data: convert_sqlite_get_pages_response_v4_to_v5(x.data)?,
	})
}

pub fn convert_to_envoy_sqlite_commit_response_v4_to_v5(
	x: v4::ToEnvoySqliteCommitResponse,
) -> Result<v5::ToEnvoySqliteCommitResponse> {
	Ok(v5::ToEnvoySqliteCommitResponse {
		request_id: x.request_id,
		data: convert_sqlite_commit_response_v4_to_v5(x.data)?,
	})
}

pub fn convert_to_envoy_sqlite_exec_response_v4_to_v5(
	x: v4::ToEnvoySqliteExecResponse,
) -> Result<v5::ToEnvoySqliteExecResponse> {
	Ok(v5::ToEnvoySqliteExecResponse {
		request_id: x.request_id,
		data: convert_sqlite_exec_response_v4_to_v5(x.data)?,
	})
}

pub fn convert_to_envoy_sqlite_execute_response_v4_to_v5(
	x: v4::ToEnvoySqliteExecuteResponse,
) -> Result<v5::ToEnvoySqliteExecuteResponse> {
	Ok(v5::ToEnvoySqliteExecuteResponse {
		request_id: x.request_id,
		data: convert_sqlite_execute_response_v4_to_v5(x.data)?,
	})
}

pub fn convert_to_envoy_v4_to_v5(x: v4::ToEnvoy) -> Result<v5::ToEnvoy> {
	Ok(match x {
		v4::ToEnvoy::ToEnvoyInit(v) => v5::ToEnvoy::ToEnvoyInit(convert_to_envoy_init_v4_to_v5(v)?),
		v4::ToEnvoy::ToEnvoyCommands(v) => {
			v5::ToEnvoy::ToEnvoyCommands(convert_to_envoy_commands_v4_to_v5(v)?)
		}
		v4::ToEnvoy::ToEnvoyAckEvents(v) => {
			v5::ToEnvoy::ToEnvoyAckEvents(convert_to_envoy_ack_events_v4_to_v5(v)?)
		}
		v4::ToEnvoy::ToEnvoyKvResponse(v) => {
			v5::ToEnvoy::ToEnvoyKvResponse(convert_to_envoy_kv_response_v4_to_v5(v)?)
		}
		v4::ToEnvoy::ToEnvoyTunnelMessage(v) => {
			v5::ToEnvoy::ToEnvoyTunnelMessage(convert_to_envoy_tunnel_message_v4_to_v5(v)?)
		}
		v4::ToEnvoy::ToEnvoyPing(v) => v5::ToEnvoy::ToEnvoyPing(convert_to_envoy_ping_v4_to_v5(v)?),
		v4::ToEnvoy::ToEnvoySqliteGetPagesResponse(v) => {
			v5::ToEnvoy::ToEnvoySqliteGetPagesResponse(
				convert_to_envoy_sqlite_get_pages_response_v4_to_v5(v)?,
			)
		}
		v4::ToEnvoy::ToEnvoySqliteCommitResponse(v) => v5::ToEnvoy::ToEnvoySqliteCommitResponse(
			convert_to_envoy_sqlite_commit_response_v4_to_v5(v)?,
		),
		v4::ToEnvoy::ToEnvoySqliteExecResponse(v) => v5::ToEnvoy::ToEnvoySqliteExecResponse(
			convert_to_envoy_sqlite_exec_response_v4_to_v5(v)?,
		),
		v4::ToEnvoy::ToEnvoySqliteExecuteResponse(v) => v5::ToEnvoy::ToEnvoySqliteExecuteResponse(
			convert_to_envoy_sqlite_execute_response_v4_to_v5(v)?,
		),
	})
}

pub fn convert_to_envoy_conn_ping_v4_to_v5(x: v4::ToEnvoyConnPing) -> Result<v5::ToEnvoyConnPing> {
	Ok(v5::ToEnvoyConnPing {
		gateway_id: x.gateway_id,
		request_id: x.request_id,
		ts: x.ts,
	})
}

pub fn convert_to_envoy_conn_v4_to_v5(x: v4::ToEnvoyConn) -> Result<v5::ToEnvoyConn> {
	Ok(match x {
		v4::ToEnvoyConn::ToEnvoyConnPing(v) => {
			v5::ToEnvoyConn::ToEnvoyConnPing(convert_to_envoy_conn_ping_v4_to_v5(v)?)
		}
		v4::ToEnvoyConn::ToEnvoyConnClose => v5::ToEnvoyConn::ToEnvoyConnClose,
		v4::ToEnvoyConn::ToEnvoyCommands(v) => {
			v5::ToEnvoyConn::ToEnvoyCommands(convert_to_envoy_commands_v4_to_v5(v)?)
		}
		v4::ToEnvoyConn::ToEnvoyAckEvents(v) => {
			v5::ToEnvoyConn::ToEnvoyAckEvents(convert_to_envoy_ack_events_v4_to_v5(v)?)
		}
		v4::ToEnvoyConn::ToEnvoyTunnelMessage(v) => {
			v5::ToEnvoyConn::ToEnvoyTunnelMessage(convert_to_envoy_tunnel_message_v4_to_v5(v)?)
		}
	})
}

pub fn convert_to_gateway_pong_v4_to_v5(x: v4::ToGatewayPong) -> Result<v5::ToGatewayPong> {
	Ok(v5::ToGatewayPong {
		request_id: x.request_id,
		ts: x.ts,
	})
}

pub fn convert_to_gateway_v4_to_v5(x: v4::ToGateway) -> Result<v5::ToGateway> {
	Ok(match x {
		v4::ToGateway::ToGatewayPong(v) => {
			v5::ToGateway::ToGatewayPong(convert_to_gateway_pong_v4_to_v5(v)?)
		}
		v4::ToGateway::ToRivetTunnelMessage(v) => {
			v5::ToGateway::ToRivetTunnelMessage(convert_to_rivet_tunnel_message_v4_to_v5(v)?)
		}
	})
}

pub fn convert_to_outbound_actor_start_v4_to_v5(
	x: v4::ToOutboundActorStart,
) -> Result<v5::ToOutboundActorStart> {
	Ok(v5::ToOutboundActorStart {
		namespace_id: x.namespace_id,
		pool_name: x.pool_name,
		checkpoint: convert_actor_checkpoint_v4_to_v5(x.checkpoint)?,
		actor_config: convert_actor_config_v4_to_v5(x.actor_config)?,
	})
}

pub fn convert_to_outbound_v4_to_v5(x: v4::ToOutbound) -> Result<v5::ToOutbound> {
	Ok(match x {
		v4::ToOutbound::ToOutboundActorStart(v) => {
			v5::ToOutbound::ToOutboundActorStart(convert_to_outbound_actor_start_v4_to_v5(v)?)
		}
	})
}
