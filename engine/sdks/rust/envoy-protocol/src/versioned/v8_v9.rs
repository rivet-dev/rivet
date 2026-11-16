//! Converters between envoy protocol v8 and v9. v9 adds the optional individual actor fork to
//! the start command, the engine-internal SQLite tail proof request, and the volume group on
//! commands and gateway requests, which v8 cannot carry; every other type is unchanged and is
//! copied field by field.

use anyhow::Result;

use crate::generated::{v8, v9};

pub fn convert_kv_metadata_v8_to_v9(x: v8::KvMetadata) -> Result<v9::KvMetadata> {
	Ok(v9::KvMetadata {
		version: x.version,
		update_ts: x.update_ts,
	})
}

pub fn convert_kv_list_range_query_v8_to_v9(
	x: v8::KvListRangeQuery,
) -> Result<v9::KvListRangeQuery> {
	Ok(v9::KvListRangeQuery {
		start: x.start,
		end: x.end,
		exclusive: x.exclusive,
	})
}

pub fn convert_kv_list_prefix_query_v8_to_v9(
	x: v8::KvListPrefixQuery,
) -> Result<v9::KvListPrefixQuery> {
	Ok(v9::KvListPrefixQuery { key: x.key })
}

pub fn convert_kv_list_query_v8_to_v9(x: v8::KvListQuery) -> Result<v9::KvListQuery> {
	Ok(match x {
		v8::KvListQuery::KvListAllQuery => v9::KvListQuery::KvListAllQuery,
		v8::KvListQuery::KvListRangeQuery(v) => {
			v9::KvListQuery::KvListRangeQuery(convert_kv_list_range_query_v8_to_v9(v)?)
		}
		v8::KvListQuery::KvListPrefixQuery(v) => {
			v9::KvListQuery::KvListPrefixQuery(convert_kv_list_prefix_query_v8_to_v9(v)?)
		}
	})
}

pub fn convert_kv_get_request_v8_to_v9(x: v8::KvGetRequest) -> Result<v9::KvGetRequest> {
	Ok(v9::KvGetRequest { keys: x.keys })
}

pub fn convert_kv_list_request_v8_to_v9(x: v8::KvListRequest) -> Result<v9::KvListRequest> {
	Ok(v9::KvListRequest {
		query: convert_kv_list_query_v8_to_v9(x.query)?,
		reverse: x.reverse,
		limit: x.limit,
	})
}

pub fn convert_kv_put_request_v8_to_v9(x: v8::KvPutRequest) -> Result<v9::KvPutRequest> {
	Ok(v9::KvPutRequest {
		keys: x.keys,
		values: x.values,
	})
}

pub fn convert_kv_delete_request_v8_to_v9(x: v8::KvDeleteRequest) -> Result<v9::KvDeleteRequest> {
	Ok(v9::KvDeleteRequest { keys: x.keys })
}

pub fn convert_kv_delete_range_request_v8_to_v9(
	x: v8::KvDeleteRangeRequest,
) -> Result<v9::KvDeleteRangeRequest> {
	Ok(v9::KvDeleteRangeRequest {
		start: x.start,
		end: x.end,
	})
}

pub fn convert_kv_error_response_v8_to_v9(x: v8::KvErrorResponse) -> Result<v9::KvErrorResponse> {
	Ok(v9::KvErrorResponse { message: x.message })
}

pub fn convert_kv_get_response_v8_to_v9(x: v8::KvGetResponse) -> Result<v9::KvGetResponse> {
	Ok(v9::KvGetResponse {
		keys: x.keys,
		values: x.values,
		metadata: x
			.metadata
			.into_iter()
			.map(|v| convert_kv_metadata_v8_to_v9(v))
			.collect::<Result<Vec<_>>>()?,
	})
}

pub fn convert_kv_list_response_v8_to_v9(x: v8::KvListResponse) -> Result<v9::KvListResponse> {
	Ok(v9::KvListResponse {
		keys: x.keys,
		values: x.values,
		metadata: x
			.metadata
			.into_iter()
			.map(|v| convert_kv_metadata_v8_to_v9(v))
			.collect::<Result<Vec<_>>>()?,
	})
}

pub fn convert_kv_request_data_v8_to_v9(x: v8::KvRequestData) -> Result<v9::KvRequestData> {
	Ok(match x {
		v8::KvRequestData::KvGetRequest(v) => {
			v9::KvRequestData::KvGetRequest(convert_kv_get_request_v8_to_v9(v)?)
		}
		v8::KvRequestData::KvListRequest(v) => {
			v9::KvRequestData::KvListRequest(convert_kv_list_request_v8_to_v9(v)?)
		}
		v8::KvRequestData::KvPutRequest(v) => {
			v9::KvRequestData::KvPutRequest(convert_kv_put_request_v8_to_v9(v)?)
		}
		v8::KvRequestData::KvDeleteRequest(v) => {
			v9::KvRequestData::KvDeleteRequest(convert_kv_delete_request_v8_to_v9(v)?)
		}
		v8::KvRequestData::KvDeleteRangeRequest(v) => {
			v9::KvRequestData::KvDeleteRangeRequest(convert_kv_delete_range_request_v8_to_v9(v)?)
		}
		v8::KvRequestData::KvDropRequest => v9::KvRequestData::KvDropRequest,
	})
}

pub fn convert_kv_response_data_v8_to_v9(x: v8::KvResponseData) -> Result<v9::KvResponseData> {
	Ok(match x {
		v8::KvResponseData::KvErrorResponse(v) => {
			v9::KvResponseData::KvErrorResponse(convert_kv_error_response_v8_to_v9(v)?)
		}
		v8::KvResponseData::KvGetResponse(v) => {
			v9::KvResponseData::KvGetResponse(convert_kv_get_response_v8_to_v9(v)?)
		}
		v8::KvResponseData::KvListResponse(v) => {
			v9::KvResponseData::KvListResponse(convert_kv_list_response_v8_to_v9(v)?)
		}
		v8::KvResponseData::KvPutResponse => v9::KvResponseData::KvPutResponse,
		v8::KvResponseData::KvDeleteResponse => v9::KvResponseData::KvDeleteResponse,
		v8::KvResponseData::KvDropResponse => v9::KvResponseData::KvDropResponse,
	})
}

pub fn convert_sqlite_dirty_page_v8_to_v9(x: v8::SqliteDirtyPage) -> Result<v9::SqliteDirtyPage> {
	Ok(v9::SqliteDirtyPage {
		pgno: x.pgno,
		bytes: x.bytes,
	})
}

pub fn convert_sqlite_fetched_page_v8_to_v9(
	x: v8::SqliteFetchedPage,
) -> Result<v9::SqliteFetchedPage> {
	Ok(v9::SqliteFetchedPage {
		pgno: x.pgno,
		bytes: x.bytes,
	})
}

pub fn convert_sqlite_get_pages_request_v8_to_v9(
	x: v8::SqliteGetPagesRequest,
) -> Result<v9::SqliteGetPagesRequest> {
	Ok(v9::SqliteGetPagesRequest {
		actor_id: x.actor_id,
		pgnos: x.pgnos,
		expected_generation: x.expected_generation,
		expected_head_txid: x.expected_head_txid,
	})
}

pub fn convert_sqlite_get_pages_ok_v8_to_v9(
	x: v8::SqliteGetPagesOk,
) -> Result<v9::SqliteGetPagesOk> {
	Ok(v9::SqliteGetPagesOk {
		pages: x
			.pages
			.into_iter()
			.map(|v| convert_sqlite_fetched_page_v8_to_v9(v))
			.collect::<Result<Vec<_>>>()?,
		head_txid: x.head_txid,
	})
}

pub fn convert_sqlite_error_response_v8_to_v9(
	x: v8::SqliteErrorResponse,
) -> Result<v9::SqliteErrorResponse> {
	Ok(v9::SqliteErrorResponse {
		group: x.group,
		code: x.code,
		message: x.message,
	})
}

pub fn convert_sqlite_get_pages_response_v8_to_v9(
	x: v8::SqliteGetPagesResponse,
) -> Result<v9::SqliteGetPagesResponse> {
	Ok(match x {
		v8::SqliteGetPagesResponse::SqliteGetPagesOk(v) => {
			v9::SqliteGetPagesResponse::SqliteGetPagesOk(convert_sqlite_get_pages_ok_v8_to_v9(v)?)
		}
		v8::SqliteGetPagesResponse::SqliteErrorResponse(v) => {
			v9::SqliteGetPagesResponse::SqliteErrorResponse(convert_sqlite_error_response_v8_to_v9(
				v,
			)?)
		}
	})
}

pub fn convert_sqlite_commit_request_v8_to_v9(
	x: v8::SqliteCommitRequest,
) -> Result<v9::SqliteCommitRequest> {
	Ok(v9::SqliteCommitRequest {
		actor_id: x.actor_id,
		dirty_pages: x
			.dirty_pages
			.into_iter()
			.map(|v| convert_sqlite_dirty_page_v8_to_v9(v))
			.collect::<Result<Vec<_>>>()?,
		db_size_pages: x.db_size_pages,
		now_ms: x.now_ms,
		expected_generation: x.expected_generation,
		expected_head_txid: x.expected_head_txid,
	})
}

pub fn convert_sqlite_commit_ok_v8_to_v9(x: v8::SqliteCommitOk) -> Result<v9::SqliteCommitOk> {
	Ok(v9::SqliteCommitOk {
		head_txid: x.head_txid,
	})
}

pub fn convert_sqlite_commit_response_v8_to_v9(
	x: v8::SqliteCommitResponse,
) -> Result<v9::SqliteCommitResponse> {
	Ok(match x {
		v8::SqliteCommitResponse::SqliteCommitOk(v) => {
			v9::SqliteCommitResponse::SqliteCommitOk(convert_sqlite_commit_ok_v8_to_v9(v)?)
		}
		v8::SqliteCommitResponse::SqliteErrorResponse(v) => {
			v9::SqliteCommitResponse::SqliteErrorResponse(convert_sqlite_error_response_v8_to_v9(
				v,
			)?)
		}
	})
}

pub fn convert_sqlite_commit_stage_begin_request_v8_to_v9(
	x: v8::SqliteCommitStageBeginRequest,
) -> Result<v9::SqliteCommitStageBeginRequest> {
	Ok(v9::SqliteCommitStageBeginRequest {
		actor_id: x.actor_id,
		expected_generation: x.expected_generation,
		expected_head_txid: x.expected_head_txid,
	})
}

pub fn convert_sqlite_commit_stage_begin_ok_v8_to_v9(
	x: v8::SqliteCommitStageBeginOk,
) -> Result<v9::SqliteCommitStageBeginOk> {
	Ok(v9::SqliteCommitStageBeginOk { txid: x.txid })
}

pub fn convert_sqlite_commit_stage_begin_response_v8_to_v9(
	x: v8::SqliteCommitStageBeginResponse,
) -> Result<v9::SqliteCommitStageBeginResponse> {
	Ok(match x {
		v8::SqliteCommitStageBeginResponse::SqliteCommitStageBeginOk(v) => {
			v9::SqliteCommitStageBeginResponse::SqliteCommitStageBeginOk(
				convert_sqlite_commit_stage_begin_ok_v8_to_v9(v)?,
			)
		}
		v8::SqliteCommitStageBeginResponse::SqliteErrorResponse(v) => {
			v9::SqliteCommitStageBeginResponse::SqliteErrorResponse(
				convert_sqlite_error_response_v8_to_v9(v)?,
			)
		}
	})
}

pub fn convert_sqlite_commit_stage_segment_request_v8_to_v9(
	x: v8::SqliteCommitStageSegmentRequest,
) -> Result<v9::SqliteCommitStageSegmentRequest> {
	Ok(v9::SqliteCommitStageSegmentRequest {
		actor_id: x.actor_id,
		expected_generation: x.expected_generation,
		txid: x.txid,
		first_pgno: x.first_pgno,
		dirty_pages: x
			.dirty_pages
			.into_iter()
			.map(|v| convert_sqlite_dirty_page_v8_to_v9(v))
			.collect::<Result<Vec<_>>>()?,
	})
}

pub fn convert_sqlite_commit_stage_segment_ok_v8_to_v9(
	x: v8::SqliteCommitStageSegmentOk,
) -> Result<v9::SqliteCommitStageSegmentOk> {
	Ok(v9::SqliteCommitStageSegmentOk {
		staged_bytes: x.staged_bytes,
	})
}

pub fn convert_sqlite_commit_stage_segment_response_v8_to_v9(
	x: v8::SqliteCommitStageSegmentResponse,
) -> Result<v9::SqliteCommitStageSegmentResponse> {
	Ok(match x {
		v8::SqliteCommitStageSegmentResponse::SqliteCommitStageSegmentOk(v) => {
			v9::SqliteCommitStageSegmentResponse::SqliteCommitStageSegmentOk(
				convert_sqlite_commit_stage_segment_ok_v8_to_v9(v)?,
			)
		}
		v8::SqliteCommitStageSegmentResponse::SqliteErrorResponse(v) => {
			v9::SqliteCommitStageSegmentResponse::SqliteErrorResponse(
				convert_sqlite_error_response_v8_to_v9(v)?,
			)
		}
	})
}

pub fn convert_sqlite_commit_finalize_request_v8_to_v9(
	x: v8::SqliteCommitFinalizeRequest,
) -> Result<v9::SqliteCommitFinalizeRequest> {
	Ok(v9::SqliteCommitFinalizeRequest {
		actor_id: x.actor_id,
		expected_generation: x.expected_generation,
		txid: x.txid,
		new_db_size_pages: x.new_db_size_pages,
		now_ms: x.now_ms,
		segment_first_pgnos: x.segment_first_pgnos,
	})
}

pub fn convert_sqlite_commit_finalize_ok_v8_to_v9(
	x: v8::SqliteCommitFinalizeOk,
) -> Result<v9::SqliteCommitFinalizeOk> {
	Ok(v9::SqliteCommitFinalizeOk {
		head_txid: x.head_txid,
	})
}

pub fn convert_sqlite_commit_finalize_response_v8_to_v9(
	x: v8::SqliteCommitFinalizeResponse,
) -> Result<v9::SqliteCommitFinalizeResponse> {
	Ok(match x {
		v8::SqliteCommitFinalizeResponse::SqliteCommitFinalizeOk(v) => {
			v9::SqliteCommitFinalizeResponse::SqliteCommitFinalizeOk(
				convert_sqlite_commit_finalize_ok_v8_to_v9(v)?,
			)
		}
		v8::SqliteCommitFinalizeResponse::SqliteErrorResponse(v) => {
			v9::SqliteCommitFinalizeResponse::SqliteErrorResponse(
				convert_sqlite_error_response_v8_to_v9(v)?,
			)
		}
	})
}

pub fn convert_sqlite_value_integer_v8_to_v9(
	x: v8::SqliteValueInteger,
) -> Result<v9::SqliteValueInteger> {
	Ok(v9::SqliteValueInteger { value: x.value })
}

pub fn convert_sqlite_value_float_v8_to_v9(
	x: v8::SqliteValueFloat,
) -> Result<v9::SqliteValueFloat> {
	Ok(v9::SqliteValueFloat { value: x.value })
}

pub fn convert_sqlite_value_text_v8_to_v9(x: v8::SqliteValueText) -> Result<v9::SqliteValueText> {
	Ok(v9::SqliteValueText { value: x.value })
}

pub fn convert_sqlite_value_blob_v8_to_v9(x: v8::SqliteValueBlob) -> Result<v9::SqliteValueBlob> {
	Ok(v9::SqliteValueBlob { value: x.value })
}

pub fn convert_sqlite_bind_param_v8_to_v9(x: v8::SqliteBindParam) -> Result<v9::SqliteBindParam> {
	Ok(match x {
		v8::SqliteBindParam::SqliteValueNull => v9::SqliteBindParam::SqliteValueNull,
		v8::SqliteBindParam::SqliteValueInteger(v) => {
			v9::SqliteBindParam::SqliteValueInteger(convert_sqlite_value_integer_v8_to_v9(v)?)
		}
		v8::SqliteBindParam::SqliteValueFloat(v) => {
			v9::SqliteBindParam::SqliteValueFloat(convert_sqlite_value_float_v8_to_v9(v)?)
		}
		v8::SqliteBindParam::SqliteValueText(v) => {
			v9::SqliteBindParam::SqliteValueText(convert_sqlite_value_text_v8_to_v9(v)?)
		}
		v8::SqliteBindParam::SqliteValueBlob(v) => {
			v9::SqliteBindParam::SqliteValueBlob(convert_sqlite_value_blob_v8_to_v9(v)?)
		}
	})
}

pub fn convert_sqlite_column_value_v8_to_v9(
	x: v8::SqliteColumnValue,
) -> Result<v9::SqliteColumnValue> {
	Ok(match x {
		v8::SqliteColumnValue::SqliteValueNull => v9::SqliteColumnValue::SqliteValueNull,
		v8::SqliteColumnValue::SqliteValueInteger(v) => {
			v9::SqliteColumnValue::SqliteValueInteger(convert_sqlite_value_integer_v8_to_v9(v)?)
		}
		v8::SqliteColumnValue::SqliteValueFloat(v) => {
			v9::SqliteColumnValue::SqliteValueFloat(convert_sqlite_value_float_v8_to_v9(v)?)
		}
		v8::SqliteColumnValue::SqliteValueText(v) => {
			v9::SqliteColumnValue::SqliteValueText(convert_sqlite_value_text_v8_to_v9(v)?)
		}
		v8::SqliteColumnValue::SqliteValueBlob(v) => {
			v9::SqliteColumnValue::SqliteValueBlob(convert_sqlite_value_blob_v8_to_v9(v)?)
		}
	})
}

pub fn convert_sqlite_query_result_v8_to_v9(
	x: v8::SqliteQueryResult,
) -> Result<v9::SqliteQueryResult> {
	Ok(v9::SqliteQueryResult {
		columns: x.columns,
		rows: x
			.rows
			.into_iter()
			.map(|v| {
				v.into_iter()
					.map(|v| convert_sqlite_column_value_v8_to_v9(v))
					.collect::<Result<Vec<_>>>()
			})
			.collect::<Result<Vec<_>>>()?,
	})
}

pub fn convert_sqlite_execute_result_v8_to_v9(
	x: v8::SqliteExecuteResult,
) -> Result<v9::SqliteExecuteResult> {
	Ok(v9::SqliteExecuteResult {
		columns: x.columns,
		rows: x
			.rows
			.into_iter()
			.map(|v| {
				v.into_iter()
					.map(|v| convert_sqlite_column_value_v8_to_v9(v))
					.collect::<Result<Vec<_>>>()
			})
			.collect::<Result<Vec<_>>>()?,
		changes: x.changes,
		last_insert_row_id: x.last_insert_row_id,
	})
}

pub fn convert_sqlite_exec_request_v8_to_v9(
	x: v8::SqliteExecRequest,
) -> Result<v9::SqliteExecRequest> {
	Ok(v9::SqliteExecRequest {
		namespace_id: x.namespace_id,
		actor_id: x.actor_id,
		generation: x.generation,
		sql: x.sql,
	})
}

pub fn convert_sqlite_execute_request_v8_to_v9(
	x: v8::SqliteExecuteRequest,
) -> Result<v9::SqliteExecuteRequest> {
	Ok(v9::SqliteExecuteRequest {
		namespace_id: x.namespace_id,
		actor_id: x.actor_id,
		generation: x.generation,
		sql: x.sql,
		params: x
			.params
			.map(|v| {
				v.into_iter()
					.map(|v| convert_sqlite_bind_param_v8_to_v9(v))
					.collect::<Result<Vec<_>>>()
			})
			.transpose()?,
	})
}

pub fn convert_sqlite_batch_statement_v8_to_v9(
	x: v8::SqliteBatchStatement,
) -> Result<v9::SqliteBatchStatement> {
	Ok(v9::SqliteBatchStatement {
		sql: x.sql,
		params: x
			.params
			.map(|v| {
				v.into_iter()
					.map(|v| convert_sqlite_bind_param_v8_to_v9(v))
					.collect::<Result<Vec<_>>>()
			})
			.transpose()?,
	})
}

pub fn convert_sqlite_execute_batch_request_v8_to_v9(
	x: v8::SqliteExecuteBatchRequest,
) -> Result<v9::SqliteExecuteBatchRequest> {
	Ok(v9::SqliteExecuteBatchRequest {
		namespace_id: x.namespace_id,
		actor_id: x.actor_id,
		generation: x.generation,
		statements: x
			.statements
			.into_iter()
			.map(|v| convert_sqlite_batch_statement_v8_to_v9(v))
			.collect::<Result<Vec<_>>>()?,
	})
}

pub fn convert_sqlite_exec_ok_v8_to_v9(x: v8::SqliteExecOk) -> Result<v9::SqliteExecOk> {
	Ok(v9::SqliteExecOk {
		result: convert_sqlite_query_result_v8_to_v9(x.result)?,
	})
}

pub fn convert_sqlite_execute_ok_v8_to_v9(x: v8::SqliteExecuteOk) -> Result<v9::SqliteExecuteOk> {
	Ok(v9::SqliteExecuteOk {
		result: convert_sqlite_execute_result_v8_to_v9(x.result)?,
	})
}

pub fn convert_sqlite_execute_batch_ok_v8_to_v9(
	x: v8::SqliteExecuteBatchOk,
) -> Result<v9::SqliteExecuteBatchOk> {
	Ok(v9::SqliteExecuteBatchOk {
		results: x
			.results
			.into_iter()
			.map(|v| convert_sqlite_execute_result_v8_to_v9(v))
			.collect::<Result<Vec<_>>>()?,
	})
}

pub fn convert_sqlite_exec_response_v8_to_v9(
	x: v8::SqliteExecResponse,
) -> Result<v9::SqliteExecResponse> {
	Ok(match x {
		v8::SqliteExecResponse::SqliteExecOk(v) => {
			v9::SqliteExecResponse::SqliteExecOk(convert_sqlite_exec_ok_v8_to_v9(v)?)
		}
		v8::SqliteExecResponse::SqliteErrorResponse(v) => {
			v9::SqliteExecResponse::SqliteErrorResponse(convert_sqlite_error_response_v8_to_v9(v)?)
		}
	})
}

pub fn convert_sqlite_execute_response_v8_to_v9(
	x: v8::SqliteExecuteResponse,
) -> Result<v9::SqliteExecuteResponse> {
	Ok(match x {
		v8::SqliteExecuteResponse::SqliteExecuteOk(v) => {
			v9::SqliteExecuteResponse::SqliteExecuteOk(convert_sqlite_execute_ok_v8_to_v9(v)?)
		}
		v8::SqliteExecuteResponse::SqliteErrorResponse(v) => {
			v9::SqliteExecuteResponse::SqliteErrorResponse(convert_sqlite_error_response_v8_to_v9(
				v,
			)?)
		}
	})
}

pub fn convert_sqlite_execute_batch_response_v8_to_v9(
	x: v8::SqliteExecuteBatchResponse,
) -> Result<v9::SqliteExecuteBatchResponse> {
	Ok(match x {
		v8::SqliteExecuteBatchResponse::SqliteExecuteBatchOk(v) => {
			v9::SqliteExecuteBatchResponse::SqliteExecuteBatchOk(
				convert_sqlite_execute_batch_ok_v8_to_v9(v)?,
			)
		}
		v8::SqliteExecuteBatchResponse::SqliteErrorResponse(v) => {
			v9::SqliteExecuteBatchResponse::SqliteErrorResponse(
				convert_sqlite_error_response_v8_to_v9(v)?,
			)
		}
	})
}

pub fn convert_stop_code_v8_to_v9(x: v8::StopCode) -> Result<v9::StopCode> {
	Ok(match x {
		v8::StopCode::Ok => v9::StopCode::Ok,
		v8::StopCode::Error => v9::StopCode::Error,
	})
}

pub fn convert_actor_name_v8_to_v9(x: v8::ActorName) -> Result<v9::ActorName> {
	Ok(v9::ActorName {
		metadata: x.metadata,
	})
}

pub fn convert_actor_config_v8_to_v9(x: v8::ActorConfig) -> Result<v9::ActorConfig> {
	Ok(v9::ActorConfig {
		name: x.name,
		key: x.key,
		create_ts: x.create_ts,
		input: x.input,
	})
}

pub fn convert_actor_checkpoint_v8_to_v9(x: v8::ActorCheckpoint) -> Result<v9::ActorCheckpoint> {
	Ok(v9::ActorCheckpoint {
		actor_id: x.actor_id,
		generation: x.generation,
		index: x.index,
	})
}

pub fn convert_actor_intent_v8_to_v9(x: v8::ActorIntent) -> Result<v9::ActorIntent> {
	Ok(match x {
		v8::ActorIntent::ActorIntentSleep => v9::ActorIntent::ActorIntentSleep,
		v8::ActorIntent::ActorIntentStop => v9::ActorIntent::ActorIntentStop,
	})
}

pub fn convert_actor_state_stopped_v8_to_v9(
	x: v8::ActorStateStopped,
) -> Result<v9::ActorStateStopped> {
	Ok(v9::ActorStateStopped {
		code: convert_stop_code_v8_to_v9(x.code)?,
		message: x.message,
	})
}

pub fn convert_actor_state_v8_to_v9(x: v8::ActorState) -> Result<v9::ActorState> {
	Ok(match x {
		v8::ActorState::ActorStateRunning => v9::ActorState::ActorStateRunning,
		v8::ActorState::ActorStateStopped(v) => {
			v9::ActorState::ActorStateStopped(convert_actor_state_stopped_v8_to_v9(v)?)
		}
	})
}

pub fn convert_event_actor_intent_v8_to_v9(
	x: v8::EventActorIntent,
) -> Result<v9::EventActorIntent> {
	Ok(v9::EventActorIntent {
		intent: convert_actor_intent_v8_to_v9(x.intent)?,
	})
}

pub fn convert_event_actor_state_update_v8_to_v9(
	x: v8::EventActorStateUpdate,
) -> Result<v9::EventActorStateUpdate> {
	Ok(v9::EventActorStateUpdate {
		state: convert_actor_state_v8_to_v9(x.state)?,
	})
}

pub fn convert_event_actor_set_alarm_v8_to_v9(
	x: v8::EventActorSetAlarm,
) -> Result<v9::EventActorSetAlarm> {
	Ok(v9::EventActorSetAlarm {
		alarm_ts: x.alarm_ts,
	})
}

pub fn convert_event_v8_to_v9(x: v8::Event) -> Result<v9::Event> {
	Ok(match x {
		v8::Event::EventActorIntent(v) => {
			v9::Event::EventActorIntent(convert_event_actor_intent_v8_to_v9(v)?)
		}
		v8::Event::EventActorStateUpdate(v) => {
			v9::Event::EventActorStateUpdate(convert_event_actor_state_update_v8_to_v9(v)?)
		}
		v8::Event::EventActorSetAlarm(v) => {
			v9::Event::EventActorSetAlarm(convert_event_actor_set_alarm_v8_to_v9(v)?)
		}
	})
}

pub fn convert_event_wrapper_v8_to_v9(x: v8::EventWrapper) -> Result<v9::EventWrapper> {
	Ok(v9::EventWrapper {
		checkpoint: convert_actor_checkpoint_v8_to_v9(x.checkpoint)?,
		inner: convert_event_v8_to_v9(x.inner)?,
	})
}

pub fn convert_preloaded_kv_entry_v8_to_v9(
	x: v8::PreloadedKvEntry,
) -> Result<v9::PreloadedKvEntry> {
	Ok(v9::PreloadedKvEntry {
		key: x.key,
		value: x.value,
		metadata: convert_kv_metadata_v8_to_v9(x.metadata)?,
	})
}

pub fn convert_preloaded_kv_v8_to_v9(x: v8::PreloadedKv) -> Result<v9::PreloadedKv> {
	Ok(v9::PreloadedKv {
		entries: x
			.entries
			.into_iter()
			.map(|v| convert_preloaded_kv_entry_v8_to_v9(v))
			.collect::<Result<Vec<_>>>()?,
		requested_get_keys: x.requested_get_keys,
		requested_prefixes: x.requested_prefixes,
	})
}

pub fn convert_hibernating_request_v8_to_v9(
	x: v8::HibernatingRequest,
) -> Result<v9::HibernatingRequest> {
	Ok(v9::HibernatingRequest {
		gateway_id: x.gateway_id,
		request_id: x.request_id,
	})
}

pub fn convert_command_start_actor_v8_to_v9(
	x: v8::CommandStartActor,
) -> Result<v9::CommandStartActor> {
	Ok(v9::CommandStartActor {
		config: convert_actor_config_v8_to_v9(x.config)?,
		hibernating_requests: x
			.hibernating_requests
			.into_iter()
			.map(|v| convert_hibernating_request_v8_to_v9(v))
			.collect::<Result<Vec<_>>>()?,
		preloaded_kv: x
			.preloaded_kv
			.map(|v| convert_preloaded_kv_v8_to_v9(v))
			.transpose()?,
		// A v8 envoy never received a fork.
		fork: None,
	})
}

pub fn convert_stop_actor_reason_v8_to_v9(x: v8::StopActorReason) -> Result<v9::StopActorReason> {
	Ok(match x {
		v8::StopActorReason::SleepIntent => v9::StopActorReason::SleepIntent,
		v8::StopActorReason::StopIntent => v9::StopActorReason::StopIntent,
		v8::StopActorReason::Destroy => v9::StopActorReason::Destroy,
		v8::StopActorReason::GoingAway => v9::StopActorReason::GoingAway,
		v8::StopActorReason::Lost => v9::StopActorReason::Lost,
	})
}

pub fn convert_command_stop_actor_v8_to_v9(
	x: v8::CommandStopActor,
) -> Result<v9::CommandStopActor> {
	Ok(v9::CommandStopActor {
		reason: convert_stop_actor_reason_v8_to_v9(x.reason)?,
	})
}

pub fn convert_command_v8_to_v9(x: v8::Command) -> Result<v9::Command> {
	Ok(match x {
		v8::Command::CommandStartActor(v) => {
			v9::Command::CommandStartActor(convert_command_start_actor_v8_to_v9(v)?)
		}
		v8::Command::CommandStopActor(v) => {
			v9::Command::CommandStopActor(convert_command_stop_actor_v8_to_v9(v)?)
		}
	})
}

pub fn convert_command_wrapper_v8_to_v9(x: v8::CommandWrapper) -> Result<v9::CommandWrapper> {
	Ok(v9::CommandWrapper {
		checkpoint: convert_actor_checkpoint_v8_to_v9(x.checkpoint)?,
		inner: convert_command_v8_to_v9(x.inner)?,
		// A v8 sender names no group, so the receiver checks none, as in v8.
		group: None,
	})
}

pub fn convert_actor_command_key_data_v8_to_v9(
	x: v8::ActorCommandKeyData,
) -> Result<v9::ActorCommandKeyData> {
	Ok(match x {
		v8::ActorCommandKeyData::CommandStartActor(v) => {
			v9::ActorCommandKeyData::CommandStartActor(convert_command_start_actor_v8_to_v9(v)?)
		}
		v8::ActorCommandKeyData::CommandStopActor(v) => {
			v9::ActorCommandKeyData::CommandStopActor(convert_command_stop_actor_v8_to_v9(v)?)
		}
	})
}

pub fn convert_message_id_v8_to_v9(x: v8::MessageId) -> Result<v9::MessageId> {
	Ok(v9::MessageId {
		gateway_id: x.gateway_id,
		request_id: x.request_id,
		message_index: x.message_index,
	})
}

pub fn convert_to_envoy_request_start_v8_to_v9(
	x: v8::ToEnvoyRequestStart,
) -> Result<v9::ToEnvoyRequestStart> {
	Ok(v9::ToEnvoyRequestStart {
		actor_id: x.actor_id,
		actor_generation: x.actor_generation,
		method: x.method,
		path: x.path,
		headers: x.headers,
		body: x.body,
		stream: x.stream,
		response_stream: x.response_stream,
		// A v8 sender names no group, so the receiver checks none, as in v8.
		group: None,
	})
}

pub fn convert_to_envoy_request_chunk_v8_to_v9(
	x: v8::ToEnvoyRequestChunk,
) -> Result<v9::ToEnvoyRequestChunk> {
	Ok(v9::ToEnvoyRequestChunk {
		body: x.body,
		finish: x.finish,
	})
}

pub fn convert_to_rivet_request_body_window_update_v8_to_v9(
	x: v8::ToRivetRequestBodyWindowUpdate,
) -> Result<v9::ToRivetRequestBodyWindowUpdate> {
	Ok(v9::ToRivetRequestBodyWindowUpdate {
		consumed_bytes: x.consumed_bytes,
	})
}

pub fn convert_http_stream_abort_reason_kind_v8_to_v9(
	x: v8::HttpStreamAbortReasonKind,
) -> Result<v9::HttpStreamAbortReasonKind> {
	Ok(match x {
		v8::HttpStreamAbortReasonKind::Unknown => v9::HttpStreamAbortReasonKind::Unknown,
		v8::HttpStreamAbortReasonKind::Cancelled => v9::HttpStreamAbortReasonKind::Cancelled,
		v8::HttpStreamAbortReasonKind::HandlerError => v9::HttpStreamAbortReasonKind::HandlerError,
		v8::HttpStreamAbortReasonKind::InternalError => {
			v9::HttpStreamAbortReasonKind::InternalError
		}
	})
}

pub fn convert_http_stream_abort_reason_v8_to_v9(
	x: v8::HttpStreamAbortReason,
) -> Result<v9::HttpStreamAbortReason> {
	Ok(v9::HttpStreamAbortReason {
		kind: convert_http_stream_abort_reason_kind_v8_to_v9(x.kind)?,
		detail: x.detail,
	})
}

pub fn convert_to_envoy_request_abort_v8_to_v9(
	x: v8::ToEnvoyRequestAbort,
) -> Result<v9::ToEnvoyRequestAbort> {
	Ok(v9::ToEnvoyRequestAbort {
		actor_id: x.actor_id,
		actor_generation: x.actor_generation,
		reason: convert_http_stream_abort_reason_v8_to_v9(x.reason)?,
	})
}

pub fn convert_to_rivet_response_start_v8_to_v9(
	x: v8::ToRivetResponseStart,
) -> Result<v9::ToRivetResponseStart> {
	Ok(v9::ToRivetResponseStart {
		status: x.status,
		headers: x.headers,
		body: x.body,
		stream: x.stream,
	})
}

pub fn convert_to_rivet_response_chunk_v8_to_v9(
	x: v8::ToRivetResponseChunk,
) -> Result<v9::ToRivetResponseChunk> {
	Ok(v9::ToRivetResponseChunk {
		body: x.body,
		finish: x.finish,
	})
}

pub fn convert_to_envoy_response_body_window_update_v8_to_v9(
	x: v8::ToEnvoyResponseBodyWindowUpdate,
) -> Result<v9::ToEnvoyResponseBodyWindowUpdate> {
	Ok(v9::ToEnvoyResponseBodyWindowUpdate {
		consumed_bytes: x.consumed_bytes,
	})
}

pub fn convert_to_rivet_response_abort_v8_to_v9(
	x: v8::ToRivetResponseAbort,
) -> Result<v9::ToRivetResponseAbort> {
	Ok(v9::ToRivetResponseAbort {
		reason: convert_http_stream_abort_reason_v8_to_v9(x.reason)?,
	})
}

pub fn convert_to_envoy_web_socket_open_v8_to_v9(
	x: v8::ToEnvoyWebSocketOpen,
) -> Result<v9::ToEnvoyWebSocketOpen> {
	Ok(v9::ToEnvoyWebSocketOpen {
		actor_id: x.actor_id,
		actor_generation: x.actor_generation,
		path: x.path,
		headers: x.headers,
		// A v8 sender names no group, so the receiver checks none, as in v8.
		group: None,
	})
}

pub fn convert_to_envoy_web_socket_message_v8_to_v9(
	x: v8::ToEnvoyWebSocketMessage,
) -> Result<v9::ToEnvoyWebSocketMessage> {
	Ok(v9::ToEnvoyWebSocketMessage {
		data: x.data,
		binary: x.binary,
	})
}

pub fn convert_to_envoy_web_socket_close_v8_to_v9(
	x: v8::ToEnvoyWebSocketClose,
) -> Result<v9::ToEnvoyWebSocketClose> {
	Ok(v9::ToEnvoyWebSocketClose {
		code: x.code,
		reason: x.reason,
	})
}

pub fn convert_to_rivet_web_socket_open_v8_to_v9(
	x: v8::ToRivetWebSocketOpen,
) -> Result<v9::ToRivetWebSocketOpen> {
	Ok(v9::ToRivetWebSocketOpen {
		can_hibernate: x.can_hibernate,
	})
}

pub fn convert_to_rivet_web_socket_message_v8_to_v9(
	x: v8::ToRivetWebSocketMessage,
) -> Result<v9::ToRivetWebSocketMessage> {
	Ok(v9::ToRivetWebSocketMessage {
		data: x.data,
		binary: x.binary,
	})
}

pub fn convert_to_rivet_web_socket_message_ack_v8_to_v9(
	x: v8::ToRivetWebSocketMessageAck,
) -> Result<v9::ToRivetWebSocketMessageAck> {
	Ok(v9::ToRivetWebSocketMessageAck { index: x.index })
}

pub fn convert_to_rivet_web_socket_close_v8_to_v9(
	x: v8::ToRivetWebSocketClose,
) -> Result<v9::ToRivetWebSocketClose> {
	Ok(v9::ToRivetWebSocketClose {
		code: x.code,
		reason: x.reason,
		hibernate: x.hibernate,
	})
}

pub fn convert_to_rivet_tunnel_message_kind_v8_to_v9(
	x: v8::ToRivetTunnelMessageKind,
) -> Result<v9::ToRivetTunnelMessageKind> {
	Ok(match x {
		v8::ToRivetTunnelMessageKind::ToRivetResponseStart(v) => {
			v9::ToRivetTunnelMessageKind::ToRivetResponseStart(
				convert_to_rivet_response_start_v8_to_v9(v)?,
			)
		}
		v8::ToRivetTunnelMessageKind::ToRivetResponseChunk(v) => {
			v9::ToRivetTunnelMessageKind::ToRivetResponseChunk(
				convert_to_rivet_response_chunk_v8_to_v9(v)?,
			)
		}
		v8::ToRivetTunnelMessageKind::ToRivetResponseAbort(v) => {
			v9::ToRivetTunnelMessageKind::ToRivetResponseAbort(
				convert_to_rivet_response_abort_v8_to_v9(v)?,
			)
		}
		v8::ToRivetTunnelMessageKind::ToRivetRequestBodyWindowUpdate(v) => {
			v9::ToRivetTunnelMessageKind::ToRivetRequestBodyWindowUpdate(
				convert_to_rivet_request_body_window_update_v8_to_v9(v)?,
			)
		}
		v8::ToRivetTunnelMessageKind::ToRivetRequestBodyCancel => {
			v9::ToRivetTunnelMessageKind::ToRivetRequestBodyCancel
		}
		v8::ToRivetTunnelMessageKind::ToRivetWebSocketOpen(v) => {
			v9::ToRivetTunnelMessageKind::ToRivetWebSocketOpen(
				convert_to_rivet_web_socket_open_v8_to_v9(v)?,
			)
		}
		v8::ToRivetTunnelMessageKind::ToRivetWebSocketMessage(v) => {
			v9::ToRivetTunnelMessageKind::ToRivetWebSocketMessage(
				convert_to_rivet_web_socket_message_v8_to_v9(v)?,
			)
		}
		v8::ToRivetTunnelMessageKind::ToRivetWebSocketMessageAck(v) => {
			v9::ToRivetTunnelMessageKind::ToRivetWebSocketMessageAck(
				convert_to_rivet_web_socket_message_ack_v8_to_v9(v)?,
			)
		}
		v8::ToRivetTunnelMessageKind::ToRivetWebSocketClose(v) => {
			v9::ToRivetTunnelMessageKind::ToRivetWebSocketClose(
				convert_to_rivet_web_socket_close_v8_to_v9(v)?,
			)
		}
	})
}

pub fn convert_to_rivet_tunnel_message_v8_to_v9(
	x: v8::ToRivetTunnelMessage,
) -> Result<v9::ToRivetTunnelMessage> {
	Ok(v9::ToRivetTunnelMessage {
		message_id: convert_message_id_v8_to_v9(x.message_id)?,
		message_kind: convert_to_rivet_tunnel_message_kind_v8_to_v9(x.message_kind)?,
	})
}

pub fn convert_to_envoy_tunnel_message_kind_v8_to_v9(
	x: v8::ToEnvoyTunnelMessageKind,
) -> Result<v9::ToEnvoyTunnelMessageKind> {
	Ok(match x {
		v8::ToEnvoyTunnelMessageKind::ToEnvoyRequestStart(v) => {
			v9::ToEnvoyTunnelMessageKind::ToEnvoyRequestStart(
				convert_to_envoy_request_start_v8_to_v9(v)?,
			)
		}
		v8::ToEnvoyTunnelMessageKind::ToEnvoyRequestChunk(v) => {
			v9::ToEnvoyTunnelMessageKind::ToEnvoyRequestChunk(
				convert_to_envoy_request_chunk_v8_to_v9(v)?,
			)
		}
		v8::ToEnvoyTunnelMessageKind::ToEnvoyRequestAbort(v) => {
			v9::ToEnvoyTunnelMessageKind::ToEnvoyRequestAbort(
				convert_to_envoy_request_abort_v8_to_v9(v)?,
			)
		}
		v8::ToEnvoyTunnelMessageKind::ToEnvoyRequestBodyCancel => {
			v9::ToEnvoyTunnelMessageKind::ToEnvoyRequestBodyCancel
		}
		v8::ToEnvoyTunnelMessageKind::ToEnvoyResponseBodyWindowUpdate(v) => {
			v9::ToEnvoyTunnelMessageKind::ToEnvoyResponseBodyWindowUpdate(
				convert_to_envoy_response_body_window_update_v8_to_v9(v)?,
			)
		}
		v8::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketOpen(v) => {
			v9::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketOpen(
				convert_to_envoy_web_socket_open_v8_to_v9(v)?,
			)
		}
		v8::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketMessage(v) => {
			v9::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketMessage(
				convert_to_envoy_web_socket_message_v8_to_v9(v)?,
			)
		}
		v8::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketClose(v) => {
			v9::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketClose(
				convert_to_envoy_web_socket_close_v8_to_v9(v)?,
			)
		}
	})
}

pub fn convert_to_envoy_tunnel_message_v8_to_v9(
	x: v8::ToEnvoyTunnelMessage,
) -> Result<v9::ToEnvoyTunnelMessage> {
	Ok(v9::ToEnvoyTunnelMessage {
		message_id: convert_message_id_v8_to_v9(x.message_id)?,
		message_kind: convert_to_envoy_tunnel_message_kind_v8_to_v9(x.message_kind)?,
	})
}

pub fn convert_to_envoy_ping_v8_to_v9(x: v8::ToEnvoyPing) -> Result<v9::ToEnvoyPing> {
	Ok(v9::ToEnvoyPing { ts: x.ts })
}

pub fn convert_to_rivet_metadata_v8_to_v9(x: v8::ToRivetMetadata) -> Result<v9::ToRivetMetadata> {
	Ok(v9::ToRivetMetadata {
		prepopulate_actor_names: x
			.prepopulate_actor_names
			.map(|v| {
				v.into_iter()
					.map(|(k, v)| -> Result<_> { Ok((k, convert_actor_name_v8_to_v9(v)?)) })
					.collect::<Result<_>>()
			})
			.transpose()?,
		metadata: x.metadata,
	})
}

pub fn convert_to_rivet_events_v8_to_v9(x: v8::ToRivetEvents) -> Result<v9::ToRivetEvents> {
	Ok(x.into_iter()
		.map(|v| convert_event_wrapper_v8_to_v9(v))
		.collect::<Result<Vec<_>>>()?)
}

pub fn convert_to_rivet_ack_commands_v8_to_v9(
	x: v8::ToRivetAckCommands,
) -> Result<v9::ToRivetAckCommands> {
	Ok(v9::ToRivetAckCommands {
		last_command_checkpoints: x
			.last_command_checkpoints
			.into_iter()
			.map(|v| convert_actor_checkpoint_v8_to_v9(v))
			.collect::<Result<Vec<_>>>()?,
	})
}

pub fn convert_to_rivet_pong_v8_to_v9(x: v8::ToRivetPong) -> Result<v9::ToRivetPong> {
	Ok(v9::ToRivetPong { ts: x.ts })
}

pub fn convert_to_rivet_kv_request_v8_to_v9(
	x: v8::ToRivetKvRequest,
) -> Result<v9::ToRivetKvRequest> {
	Ok(v9::ToRivetKvRequest {
		actor_id: x.actor_id,
		request_id: x.request_id,
		data: convert_kv_request_data_v8_to_v9(x.data)?,
	})
}

pub fn convert_to_rivet_sqlite_get_pages_request_v8_to_v9(
	x: v8::ToRivetSqliteGetPagesRequest,
) -> Result<v9::ToRivetSqliteGetPagesRequest> {
	Ok(v9::ToRivetSqliteGetPagesRequest {
		request_id: x.request_id,
		data: convert_sqlite_get_pages_request_v8_to_v9(x.data)?,
	})
}

pub fn convert_to_rivet_sqlite_commit_request_v8_to_v9(
	x: v8::ToRivetSqliteCommitRequest,
) -> Result<v9::ToRivetSqliteCommitRequest> {
	Ok(v9::ToRivetSqliteCommitRequest {
		request_id: x.request_id,
		data: convert_sqlite_commit_request_v8_to_v9(x.data)?,
	})
}

pub fn convert_to_rivet_sqlite_commit_stage_begin_request_v8_to_v9(
	x: v8::ToRivetSqliteCommitStageBeginRequest,
) -> Result<v9::ToRivetSqliteCommitStageBeginRequest> {
	Ok(v9::ToRivetSqliteCommitStageBeginRequest {
		request_id: x.request_id,
		data: convert_sqlite_commit_stage_begin_request_v8_to_v9(x.data)?,
	})
}

pub fn convert_to_rivet_sqlite_commit_stage_segment_request_v8_to_v9(
	x: v8::ToRivetSqliteCommitStageSegmentRequest,
) -> Result<v9::ToRivetSqliteCommitStageSegmentRequest> {
	Ok(v9::ToRivetSqliteCommitStageSegmentRequest {
		request_id: x.request_id,
		data: convert_sqlite_commit_stage_segment_request_v8_to_v9(x.data)?,
	})
}

pub fn convert_to_rivet_sqlite_commit_finalize_request_v8_to_v9(
	x: v8::ToRivetSqliteCommitFinalizeRequest,
) -> Result<v9::ToRivetSqliteCommitFinalizeRequest> {
	Ok(v9::ToRivetSqliteCommitFinalizeRequest {
		request_id: x.request_id,
		data: convert_sqlite_commit_finalize_request_v8_to_v9(x.data)?,
	})
}

pub fn convert_to_rivet_sqlite_exec_request_v8_to_v9(
	x: v8::ToRivetSqliteExecRequest,
) -> Result<v9::ToRivetSqliteExecRequest> {
	Ok(v9::ToRivetSqliteExecRequest {
		request_id: x.request_id,
		data: convert_sqlite_exec_request_v8_to_v9(x.data)?,
	})
}

pub fn convert_to_rivet_sqlite_execute_request_v8_to_v9(
	x: v8::ToRivetSqliteExecuteRequest,
) -> Result<v9::ToRivetSqliteExecuteRequest> {
	Ok(v9::ToRivetSqliteExecuteRequest {
		request_id: x.request_id,
		data: convert_sqlite_execute_request_v8_to_v9(x.data)?,
	})
}

pub fn convert_to_rivet_sqlite_execute_batch_request_v8_to_v9(
	x: v8::ToRivetSqliteExecuteBatchRequest,
) -> Result<v9::ToRivetSqliteExecuteBatchRequest> {
	Ok(v9::ToRivetSqliteExecuteBatchRequest {
		request_id: x.request_id,
		data: convert_sqlite_execute_batch_request_v8_to_v9(x.data)?,
	})
}

pub fn convert_to_rivet_v8_to_v9(x: v8::ToRivet) -> Result<v9::ToRivet> {
	Ok(match x {
		v8::ToRivet::ToRivetMetadata(v) => {
			v9::ToRivet::ToRivetMetadata(convert_to_rivet_metadata_v8_to_v9(v)?)
		}
		v8::ToRivet::ToRivetEvents(v) => {
			v9::ToRivet::ToRivetEvents(convert_to_rivet_events_v8_to_v9(v)?)
		}
		v8::ToRivet::ToRivetAckCommands(v) => {
			v9::ToRivet::ToRivetAckCommands(convert_to_rivet_ack_commands_v8_to_v9(v)?)
		}
		v8::ToRivet::ToRivetStopping => v9::ToRivet::ToRivetStopping,
		v8::ToRivet::ToRivetPong(v) => v9::ToRivet::ToRivetPong(convert_to_rivet_pong_v8_to_v9(v)?),
		v8::ToRivet::ToRivetKvRequest(v) => {
			v9::ToRivet::ToRivetKvRequest(convert_to_rivet_kv_request_v8_to_v9(v)?)
		}
		v8::ToRivet::ToRivetTunnelMessage(v) => {
			v9::ToRivet::ToRivetTunnelMessage(convert_to_rivet_tunnel_message_v8_to_v9(v)?)
		}
		v8::ToRivet::ToRivetSqliteGetPagesRequest(v) => v9::ToRivet::ToRivetSqliteGetPagesRequest(
			convert_to_rivet_sqlite_get_pages_request_v8_to_v9(v)?,
		),
		v8::ToRivet::ToRivetSqliteCommitRequest(v) => v9::ToRivet::ToRivetSqliteCommitRequest(
			convert_to_rivet_sqlite_commit_request_v8_to_v9(v)?,
		),
		v8::ToRivet::ToRivetSqliteCommitStageBeginRequest(v) => {
			v9::ToRivet::ToRivetSqliteCommitStageBeginRequest(
				convert_to_rivet_sqlite_commit_stage_begin_request_v8_to_v9(v)?,
			)
		}
		v8::ToRivet::ToRivetSqliteCommitStageSegmentRequest(v) => {
			v9::ToRivet::ToRivetSqliteCommitStageSegmentRequest(
				convert_to_rivet_sqlite_commit_stage_segment_request_v8_to_v9(v)?,
			)
		}
		v8::ToRivet::ToRivetSqliteCommitFinalizeRequest(v) => {
			v9::ToRivet::ToRivetSqliteCommitFinalizeRequest(
				convert_to_rivet_sqlite_commit_finalize_request_v8_to_v9(v)?,
			)
		}
		v8::ToRivet::ToRivetSqliteExecRequest(v) => {
			v9::ToRivet::ToRivetSqliteExecRequest(convert_to_rivet_sqlite_exec_request_v8_to_v9(v)?)
		}
		v8::ToRivet::ToRivetSqliteExecuteRequest(v) => v9::ToRivet::ToRivetSqliteExecuteRequest(
			convert_to_rivet_sqlite_execute_request_v8_to_v9(v)?,
		),
		v8::ToRivet::ToRivetSqliteExecuteBatchRequest(v) => {
			v9::ToRivet::ToRivetSqliteExecuteBatchRequest(
				convert_to_rivet_sqlite_execute_batch_request_v8_to_v9(v)?,
			)
		}
	})
}

pub fn convert_protocol_metadata_v8_to_v9(x: v8::ProtocolMetadata) -> Result<v9::ProtocolMetadata> {
	Ok(v9::ProtocolMetadata {
		envoy_lost_threshold: x.envoy_lost_threshold,
		actor_stop_threshold: x.actor_stop_threshold,
		max_response_payload_size: x.max_response_payload_size,
	})
}

pub fn convert_to_envoy_init_v8_to_v9(x: v8::ToEnvoyInit) -> Result<v9::ToEnvoyInit> {
	Ok(v9::ToEnvoyInit {
		metadata: convert_protocol_metadata_v8_to_v9(x.metadata)?,
	})
}

pub fn convert_to_envoy_commands_v8_to_v9(x: v8::ToEnvoyCommands) -> Result<v9::ToEnvoyCommands> {
	Ok(x.into_iter()
		.map(|v| convert_command_wrapper_v8_to_v9(v))
		.collect::<Result<Vec<_>>>()?)
}

pub fn convert_to_envoy_ack_events_v8_to_v9(
	x: v8::ToEnvoyAckEvents,
) -> Result<v9::ToEnvoyAckEvents> {
	Ok(v9::ToEnvoyAckEvents {
		last_event_checkpoints: x
			.last_event_checkpoints
			.into_iter()
			.map(|v| convert_actor_checkpoint_v8_to_v9(v))
			.collect::<Result<Vec<_>>>()?,
	})
}

pub fn convert_to_envoy_kv_response_v8_to_v9(
	x: v8::ToEnvoyKvResponse,
) -> Result<v9::ToEnvoyKvResponse> {
	Ok(v9::ToEnvoyKvResponse {
		request_id: x.request_id,
		data: convert_kv_response_data_v8_to_v9(x.data)?,
	})
}

pub fn convert_to_envoy_sqlite_get_pages_response_v8_to_v9(
	x: v8::ToEnvoySqliteGetPagesResponse,
) -> Result<v9::ToEnvoySqliteGetPagesResponse> {
	Ok(v9::ToEnvoySqliteGetPagesResponse {
		request_id: x.request_id,
		data: convert_sqlite_get_pages_response_v8_to_v9(x.data)?,
	})
}

pub fn convert_to_envoy_sqlite_commit_response_v8_to_v9(
	x: v8::ToEnvoySqliteCommitResponse,
) -> Result<v9::ToEnvoySqliteCommitResponse> {
	Ok(v9::ToEnvoySqliteCommitResponse {
		request_id: x.request_id,
		data: convert_sqlite_commit_response_v8_to_v9(x.data)?,
	})
}

pub fn convert_to_envoy_sqlite_commit_stage_begin_response_v8_to_v9(
	x: v8::ToEnvoySqliteCommitStageBeginResponse,
) -> Result<v9::ToEnvoySqliteCommitStageBeginResponse> {
	Ok(v9::ToEnvoySqliteCommitStageBeginResponse {
		request_id: x.request_id,
		data: convert_sqlite_commit_stage_begin_response_v8_to_v9(x.data)?,
	})
}

pub fn convert_to_envoy_sqlite_commit_stage_segment_response_v8_to_v9(
	x: v8::ToEnvoySqliteCommitStageSegmentResponse,
) -> Result<v9::ToEnvoySqliteCommitStageSegmentResponse> {
	Ok(v9::ToEnvoySqliteCommitStageSegmentResponse {
		request_id: x.request_id,
		data: convert_sqlite_commit_stage_segment_response_v8_to_v9(x.data)?,
	})
}

pub fn convert_to_envoy_sqlite_commit_finalize_response_v8_to_v9(
	x: v8::ToEnvoySqliteCommitFinalizeResponse,
) -> Result<v9::ToEnvoySqliteCommitFinalizeResponse> {
	Ok(v9::ToEnvoySqliteCommitFinalizeResponse {
		request_id: x.request_id,
		data: convert_sqlite_commit_finalize_response_v8_to_v9(x.data)?,
	})
}

pub fn convert_to_envoy_sqlite_exec_response_v8_to_v9(
	x: v8::ToEnvoySqliteExecResponse,
) -> Result<v9::ToEnvoySqliteExecResponse> {
	Ok(v9::ToEnvoySqliteExecResponse {
		request_id: x.request_id,
		data: convert_sqlite_exec_response_v8_to_v9(x.data)?,
	})
}

pub fn convert_to_envoy_sqlite_execute_response_v8_to_v9(
	x: v8::ToEnvoySqliteExecuteResponse,
) -> Result<v9::ToEnvoySqliteExecuteResponse> {
	Ok(v9::ToEnvoySqliteExecuteResponse {
		request_id: x.request_id,
		data: convert_sqlite_execute_response_v8_to_v9(x.data)?,
	})
}

pub fn convert_to_envoy_sqlite_execute_batch_response_v8_to_v9(
	x: v8::ToEnvoySqliteExecuteBatchResponse,
) -> Result<v9::ToEnvoySqliteExecuteBatchResponse> {
	Ok(v9::ToEnvoySqliteExecuteBatchResponse {
		request_id: x.request_id,
		data: convert_sqlite_execute_batch_response_v8_to_v9(x.data)?,
	})
}

pub fn convert_to_envoy_v8_to_v9(x: v8::ToEnvoy) -> Result<v9::ToEnvoy> {
	Ok(match x {
		v8::ToEnvoy::ToEnvoyInit(v) => v9::ToEnvoy::ToEnvoyInit(convert_to_envoy_init_v8_to_v9(v)?),
		v8::ToEnvoy::ToEnvoyCommands(v) => {
			v9::ToEnvoy::ToEnvoyCommands(convert_to_envoy_commands_v8_to_v9(v)?)
		}
		v8::ToEnvoy::ToEnvoyAckEvents(v) => {
			v9::ToEnvoy::ToEnvoyAckEvents(convert_to_envoy_ack_events_v8_to_v9(v)?)
		}
		v8::ToEnvoy::ToEnvoyKvResponse(v) => {
			v9::ToEnvoy::ToEnvoyKvResponse(convert_to_envoy_kv_response_v8_to_v9(v)?)
		}
		v8::ToEnvoy::ToEnvoyTunnelMessage(v) => {
			v9::ToEnvoy::ToEnvoyTunnelMessage(convert_to_envoy_tunnel_message_v8_to_v9(v)?)
		}
		v8::ToEnvoy::ToEnvoyPing(v) => v9::ToEnvoy::ToEnvoyPing(convert_to_envoy_ping_v8_to_v9(v)?),
		v8::ToEnvoy::ToEnvoySqliteGetPagesResponse(v) => {
			v9::ToEnvoy::ToEnvoySqliteGetPagesResponse(
				convert_to_envoy_sqlite_get_pages_response_v8_to_v9(v)?,
			)
		}
		v8::ToEnvoy::ToEnvoySqliteCommitResponse(v) => v9::ToEnvoy::ToEnvoySqliteCommitResponse(
			convert_to_envoy_sqlite_commit_response_v8_to_v9(v)?,
		),
		v8::ToEnvoy::ToEnvoySqliteCommitStageBeginResponse(v) => {
			v9::ToEnvoy::ToEnvoySqliteCommitStageBeginResponse(
				convert_to_envoy_sqlite_commit_stage_begin_response_v8_to_v9(v)?,
			)
		}
		v8::ToEnvoy::ToEnvoySqliteCommitStageSegmentResponse(v) => {
			v9::ToEnvoy::ToEnvoySqliteCommitStageSegmentResponse(
				convert_to_envoy_sqlite_commit_stage_segment_response_v8_to_v9(v)?,
			)
		}
		v8::ToEnvoy::ToEnvoySqliteCommitFinalizeResponse(v) => {
			v9::ToEnvoy::ToEnvoySqliteCommitFinalizeResponse(
				convert_to_envoy_sqlite_commit_finalize_response_v8_to_v9(v)?,
			)
		}
		v8::ToEnvoy::ToEnvoySqliteExecResponse(v) => v9::ToEnvoy::ToEnvoySqliteExecResponse(
			convert_to_envoy_sqlite_exec_response_v8_to_v9(v)?,
		),
		v8::ToEnvoy::ToEnvoySqliteExecuteResponse(v) => v9::ToEnvoy::ToEnvoySqliteExecuteResponse(
			convert_to_envoy_sqlite_execute_response_v8_to_v9(v)?,
		),
		v8::ToEnvoy::ToEnvoySqliteExecuteBatchResponse(v) => {
			v9::ToEnvoy::ToEnvoySqliteExecuteBatchResponse(
				convert_to_envoy_sqlite_execute_batch_response_v8_to_v9(v)?,
			)
		}
	})
}

pub fn convert_to_envoy_conn_ping_v8_to_v9(x: v8::ToEnvoyConnPing) -> Result<v9::ToEnvoyConnPing> {
	Ok(v9::ToEnvoyConnPing {
		gateway_id: x.gateway_id,
		request_id: x.request_id,
		ts: x.ts,
	})
}

pub fn convert_to_envoy_conn_v8_to_v9(x: v8::ToEnvoyConn) -> Result<v9::ToEnvoyConn> {
	Ok(match x {
		v8::ToEnvoyConn::ToEnvoyConnPing(v) => {
			v9::ToEnvoyConn::ToEnvoyConnPing(convert_to_envoy_conn_ping_v8_to_v9(v)?)
		}
		v8::ToEnvoyConn::ToEnvoyConnClose => v9::ToEnvoyConn::ToEnvoyConnClose,
		v8::ToEnvoyConn::ToEnvoyCommands(v) => {
			v9::ToEnvoyConn::ToEnvoyCommands(convert_to_envoy_commands_v8_to_v9(v)?)
		}
		v8::ToEnvoyConn::ToEnvoyAckEvents(v) => {
			v9::ToEnvoyConn::ToEnvoyAckEvents(convert_to_envoy_ack_events_v8_to_v9(v)?)
		}
		v8::ToEnvoyConn::ToEnvoyTunnelMessage(v) => {
			v9::ToEnvoyConn::ToEnvoyTunnelMessage(convert_to_envoy_tunnel_message_v8_to_v9(v)?)
		}
	})
}

pub fn convert_to_gateway_pong_v8_to_v9(x: v8::ToGatewayPong) -> Result<v9::ToGatewayPong> {
	Ok(v9::ToGatewayPong {
		request_id: x.request_id,
		ts: x.ts,
	})
}

pub fn convert_to_gateway_v8_to_v9(x: v8::ToGateway) -> Result<v9::ToGateway> {
	Ok(match x {
		v8::ToGateway::ToGatewayPong(v) => {
			v9::ToGateway::ToGatewayPong(convert_to_gateway_pong_v8_to_v9(v)?)
		}
		v8::ToGateway::ToRivetTunnelMessage(v) => {
			v9::ToGateway::ToRivetTunnelMessage(convert_to_rivet_tunnel_message_v8_to_v9(v)?)
		}
	})
}

pub fn convert_to_outbound_actor_start_v8_to_v9(
	x: v8::ToOutboundActorStart,
) -> Result<v9::ToOutboundActorStart> {
	Ok(v9::ToOutboundActorStart {
		namespace_id: x.namespace_id,
		pool_name: x.pool_name,
		checkpoint: convert_actor_checkpoint_v8_to_v9(x.checkpoint)?,
		actor_config: convert_actor_config_v8_to_v9(x.actor_config)?,
	})
}

pub fn convert_to_outbound_v8_to_v9(x: v8::ToOutbound) -> Result<v9::ToOutbound> {
	Ok(match x {
		v8::ToOutbound::ToOutboundActorStart(v) => {
			v9::ToOutbound::ToOutboundActorStart(convert_to_outbound_actor_start_v8_to_v9(v)?)
		}
	})
}

pub fn convert_kv_metadata_v9_to_v8(x: v9::KvMetadata) -> Result<v8::KvMetadata> {
	Ok(v8::KvMetadata {
		version: x.version,
		update_ts: x.update_ts,
	})
}

pub fn convert_kv_list_range_query_v9_to_v8(
	x: v9::KvListRangeQuery,
) -> Result<v8::KvListRangeQuery> {
	Ok(v8::KvListRangeQuery {
		start: x.start,
		end: x.end,
		exclusive: x.exclusive,
	})
}

pub fn convert_kv_list_prefix_query_v9_to_v8(
	x: v9::KvListPrefixQuery,
) -> Result<v8::KvListPrefixQuery> {
	Ok(v8::KvListPrefixQuery { key: x.key })
}

pub fn convert_kv_list_query_v9_to_v8(x: v9::KvListQuery) -> Result<v8::KvListQuery> {
	Ok(match x {
		v9::KvListQuery::KvListAllQuery => v8::KvListQuery::KvListAllQuery,
		v9::KvListQuery::KvListRangeQuery(v) => {
			v8::KvListQuery::KvListRangeQuery(convert_kv_list_range_query_v9_to_v8(v)?)
		}
		v9::KvListQuery::KvListPrefixQuery(v) => {
			v8::KvListQuery::KvListPrefixQuery(convert_kv_list_prefix_query_v9_to_v8(v)?)
		}
	})
}

pub fn convert_kv_get_request_v9_to_v8(x: v9::KvGetRequest) -> Result<v8::KvGetRequest> {
	Ok(v8::KvGetRequest { keys: x.keys })
}

pub fn convert_kv_list_request_v9_to_v8(x: v9::KvListRequest) -> Result<v8::KvListRequest> {
	Ok(v8::KvListRequest {
		query: convert_kv_list_query_v9_to_v8(x.query)?,
		reverse: x.reverse,
		limit: x.limit,
	})
}

pub fn convert_kv_put_request_v9_to_v8(x: v9::KvPutRequest) -> Result<v8::KvPutRequest> {
	Ok(v8::KvPutRequest {
		keys: x.keys,
		values: x.values,
	})
}

pub fn convert_kv_delete_request_v9_to_v8(x: v9::KvDeleteRequest) -> Result<v8::KvDeleteRequest> {
	Ok(v8::KvDeleteRequest { keys: x.keys })
}

pub fn convert_kv_delete_range_request_v9_to_v8(
	x: v9::KvDeleteRangeRequest,
) -> Result<v8::KvDeleteRangeRequest> {
	Ok(v8::KvDeleteRangeRequest {
		start: x.start,
		end: x.end,
	})
}

pub fn convert_kv_error_response_v9_to_v8(x: v9::KvErrorResponse) -> Result<v8::KvErrorResponse> {
	Ok(v8::KvErrorResponse { message: x.message })
}

pub fn convert_kv_get_response_v9_to_v8(x: v9::KvGetResponse) -> Result<v8::KvGetResponse> {
	Ok(v8::KvGetResponse {
		keys: x.keys,
		values: x.values,
		metadata: x
			.metadata
			.into_iter()
			.map(|v| convert_kv_metadata_v9_to_v8(v))
			.collect::<Result<Vec<_>>>()?,
	})
}

pub fn convert_kv_list_response_v9_to_v8(x: v9::KvListResponse) -> Result<v8::KvListResponse> {
	Ok(v8::KvListResponse {
		keys: x.keys,
		values: x.values,
		metadata: x
			.metadata
			.into_iter()
			.map(|v| convert_kv_metadata_v9_to_v8(v))
			.collect::<Result<Vec<_>>>()?,
	})
}

pub fn convert_kv_request_data_v9_to_v8(x: v9::KvRequestData) -> Result<v8::KvRequestData> {
	Ok(match x {
		v9::KvRequestData::KvGetRequest(v) => {
			v8::KvRequestData::KvGetRequest(convert_kv_get_request_v9_to_v8(v)?)
		}
		v9::KvRequestData::KvListRequest(v) => {
			v8::KvRequestData::KvListRequest(convert_kv_list_request_v9_to_v8(v)?)
		}
		v9::KvRequestData::KvPutRequest(v) => {
			v8::KvRequestData::KvPutRequest(convert_kv_put_request_v9_to_v8(v)?)
		}
		v9::KvRequestData::KvDeleteRequest(v) => {
			v8::KvRequestData::KvDeleteRequest(convert_kv_delete_request_v9_to_v8(v)?)
		}
		v9::KvRequestData::KvDeleteRangeRequest(v) => {
			v8::KvRequestData::KvDeleteRangeRequest(convert_kv_delete_range_request_v9_to_v8(v)?)
		}
		v9::KvRequestData::KvDropRequest => v8::KvRequestData::KvDropRequest,
	})
}

pub fn convert_kv_response_data_v9_to_v8(x: v9::KvResponseData) -> Result<v8::KvResponseData> {
	Ok(match x {
		v9::KvResponseData::KvErrorResponse(v) => {
			v8::KvResponseData::KvErrorResponse(convert_kv_error_response_v9_to_v8(v)?)
		}
		v9::KvResponseData::KvGetResponse(v) => {
			v8::KvResponseData::KvGetResponse(convert_kv_get_response_v9_to_v8(v)?)
		}
		v9::KvResponseData::KvListResponse(v) => {
			v8::KvResponseData::KvListResponse(convert_kv_list_response_v9_to_v8(v)?)
		}
		v9::KvResponseData::KvPutResponse => v8::KvResponseData::KvPutResponse,
		v9::KvResponseData::KvDeleteResponse => v8::KvResponseData::KvDeleteResponse,
		v9::KvResponseData::KvDropResponse => v8::KvResponseData::KvDropResponse,
	})
}

pub fn convert_sqlite_dirty_page_v9_to_v8(x: v9::SqliteDirtyPage) -> Result<v8::SqliteDirtyPage> {
	Ok(v8::SqliteDirtyPage {
		pgno: x.pgno,
		bytes: x.bytes,
	})
}

pub fn convert_sqlite_fetched_page_v9_to_v8(
	x: v9::SqliteFetchedPage,
) -> Result<v8::SqliteFetchedPage> {
	Ok(v8::SqliteFetchedPage {
		pgno: x.pgno,
		bytes: x.bytes,
	})
}

pub fn convert_sqlite_get_pages_request_v9_to_v8(
	x: v9::SqliteGetPagesRequest,
) -> Result<v8::SqliteGetPagesRequest> {
	Ok(v8::SqliteGetPagesRequest {
		actor_id: x.actor_id,
		pgnos: x.pgnos,
		expected_generation: x.expected_generation,
		expected_head_txid: x.expected_head_txid,
	})
}

pub fn convert_sqlite_get_pages_ok_v9_to_v8(
	x: v9::SqliteGetPagesOk,
) -> Result<v8::SqliteGetPagesOk> {
	Ok(v8::SqliteGetPagesOk {
		pages: x
			.pages
			.into_iter()
			.map(|v| convert_sqlite_fetched_page_v9_to_v8(v))
			.collect::<Result<Vec<_>>>()?,
		head_txid: x.head_txid,
	})
}

pub fn convert_sqlite_error_response_v9_to_v8(
	x: v9::SqliteErrorResponse,
) -> Result<v8::SqliteErrorResponse> {
	Ok(v8::SqliteErrorResponse {
		group: x.group,
		code: x.code,
		message: x.message,
	})
}

pub fn convert_sqlite_get_pages_response_v9_to_v8(
	x: v9::SqliteGetPagesResponse,
) -> Result<v8::SqliteGetPagesResponse> {
	Ok(match x {
		v9::SqliteGetPagesResponse::SqliteGetPagesOk(v) => {
			v8::SqliteGetPagesResponse::SqliteGetPagesOk(convert_sqlite_get_pages_ok_v9_to_v8(v)?)
		}
		v9::SqliteGetPagesResponse::SqliteErrorResponse(v) => {
			v8::SqliteGetPagesResponse::SqliteErrorResponse(convert_sqlite_error_response_v9_to_v8(
				v,
			)?)
		}
	})
}

pub fn convert_sqlite_commit_request_v9_to_v8(
	x: v9::SqliteCommitRequest,
) -> Result<v8::SqliteCommitRequest> {
	Ok(v8::SqliteCommitRequest {
		actor_id: x.actor_id,
		dirty_pages: x
			.dirty_pages
			.into_iter()
			.map(|v| convert_sqlite_dirty_page_v9_to_v8(v))
			.collect::<Result<Vec<_>>>()?,
		db_size_pages: x.db_size_pages,
		now_ms: x.now_ms,
		expected_generation: x.expected_generation,
		expected_head_txid: x.expected_head_txid,
	})
}

pub fn convert_sqlite_commit_ok_v9_to_v8(x: v9::SqliteCommitOk) -> Result<v8::SqliteCommitOk> {
	Ok(v8::SqliteCommitOk {
		head_txid: x.head_txid,
	})
}

pub fn convert_sqlite_commit_response_v9_to_v8(
	x: v9::SqliteCommitResponse,
) -> Result<v8::SqliteCommitResponse> {
	Ok(match x {
		v9::SqliteCommitResponse::SqliteCommitOk(v) => {
			v8::SqliteCommitResponse::SqliteCommitOk(convert_sqlite_commit_ok_v9_to_v8(v)?)
		}
		v9::SqliteCommitResponse::SqliteErrorResponse(v) => {
			v8::SqliteCommitResponse::SqliteErrorResponse(convert_sqlite_error_response_v9_to_v8(
				v,
			)?)
		}
	})
}

pub fn convert_sqlite_commit_stage_begin_request_v9_to_v8(
	x: v9::SqliteCommitStageBeginRequest,
) -> Result<v8::SqliteCommitStageBeginRequest> {
	Ok(v8::SqliteCommitStageBeginRequest {
		actor_id: x.actor_id,
		expected_generation: x.expected_generation,
		expected_head_txid: x.expected_head_txid,
	})
}

pub fn convert_sqlite_commit_stage_begin_ok_v9_to_v8(
	x: v9::SqliteCommitStageBeginOk,
) -> Result<v8::SqliteCommitStageBeginOk> {
	Ok(v8::SqliteCommitStageBeginOk { txid: x.txid })
}

pub fn convert_sqlite_commit_stage_begin_response_v9_to_v8(
	x: v9::SqliteCommitStageBeginResponse,
) -> Result<v8::SqliteCommitStageBeginResponse> {
	Ok(match x {
		v9::SqliteCommitStageBeginResponse::SqliteCommitStageBeginOk(v) => {
			v8::SqliteCommitStageBeginResponse::SqliteCommitStageBeginOk(
				convert_sqlite_commit_stage_begin_ok_v9_to_v8(v)?,
			)
		}
		v9::SqliteCommitStageBeginResponse::SqliteErrorResponse(v) => {
			v8::SqliteCommitStageBeginResponse::SqliteErrorResponse(
				convert_sqlite_error_response_v9_to_v8(v)?,
			)
		}
	})
}

pub fn convert_sqlite_commit_stage_segment_request_v9_to_v8(
	x: v9::SqliteCommitStageSegmentRequest,
) -> Result<v8::SqliteCommitStageSegmentRequest> {
	Ok(v8::SqliteCommitStageSegmentRequest {
		actor_id: x.actor_id,
		expected_generation: x.expected_generation,
		txid: x.txid,
		first_pgno: x.first_pgno,
		dirty_pages: x
			.dirty_pages
			.into_iter()
			.map(|v| convert_sqlite_dirty_page_v9_to_v8(v))
			.collect::<Result<Vec<_>>>()?,
	})
}

pub fn convert_sqlite_commit_stage_segment_ok_v9_to_v8(
	x: v9::SqliteCommitStageSegmentOk,
) -> Result<v8::SqliteCommitStageSegmentOk> {
	Ok(v8::SqliteCommitStageSegmentOk {
		staged_bytes: x.staged_bytes,
	})
}

pub fn convert_sqlite_commit_stage_segment_response_v9_to_v8(
	x: v9::SqliteCommitStageSegmentResponse,
) -> Result<v8::SqliteCommitStageSegmentResponse> {
	Ok(match x {
		v9::SqliteCommitStageSegmentResponse::SqliteCommitStageSegmentOk(v) => {
			v8::SqliteCommitStageSegmentResponse::SqliteCommitStageSegmentOk(
				convert_sqlite_commit_stage_segment_ok_v9_to_v8(v)?,
			)
		}
		v9::SqliteCommitStageSegmentResponse::SqliteErrorResponse(v) => {
			v8::SqliteCommitStageSegmentResponse::SqliteErrorResponse(
				convert_sqlite_error_response_v9_to_v8(v)?,
			)
		}
	})
}

pub fn convert_sqlite_commit_finalize_request_v9_to_v8(
	x: v9::SqliteCommitFinalizeRequest,
) -> Result<v8::SqliteCommitFinalizeRequest> {
	Ok(v8::SqliteCommitFinalizeRequest {
		actor_id: x.actor_id,
		expected_generation: x.expected_generation,
		txid: x.txid,
		new_db_size_pages: x.new_db_size_pages,
		now_ms: x.now_ms,
		segment_first_pgnos: x.segment_first_pgnos,
	})
}

pub fn convert_sqlite_commit_finalize_ok_v9_to_v8(
	x: v9::SqliteCommitFinalizeOk,
) -> Result<v8::SqliteCommitFinalizeOk> {
	Ok(v8::SqliteCommitFinalizeOk {
		head_txid: x.head_txid,
	})
}

pub fn convert_sqlite_commit_finalize_response_v9_to_v8(
	x: v9::SqliteCommitFinalizeResponse,
) -> Result<v8::SqliteCommitFinalizeResponse> {
	Ok(match x {
		v9::SqliteCommitFinalizeResponse::SqliteCommitFinalizeOk(v) => {
			v8::SqliteCommitFinalizeResponse::SqliteCommitFinalizeOk(
				convert_sqlite_commit_finalize_ok_v9_to_v8(v)?,
			)
		}
		v9::SqliteCommitFinalizeResponse::SqliteErrorResponse(v) => {
			v8::SqliteCommitFinalizeResponse::SqliteErrorResponse(
				convert_sqlite_error_response_v9_to_v8(v)?,
			)
		}
	})
}

pub fn convert_sqlite_value_integer_v9_to_v8(
	x: v9::SqliteValueInteger,
) -> Result<v8::SqliteValueInteger> {
	Ok(v8::SqliteValueInteger { value: x.value })
}

pub fn convert_sqlite_value_float_v9_to_v8(
	x: v9::SqliteValueFloat,
) -> Result<v8::SqliteValueFloat> {
	Ok(v8::SqliteValueFloat { value: x.value })
}

pub fn convert_sqlite_value_text_v9_to_v8(x: v9::SqliteValueText) -> Result<v8::SqliteValueText> {
	Ok(v8::SqliteValueText { value: x.value })
}

pub fn convert_sqlite_value_blob_v9_to_v8(x: v9::SqliteValueBlob) -> Result<v8::SqliteValueBlob> {
	Ok(v8::SqliteValueBlob { value: x.value })
}

pub fn convert_sqlite_bind_param_v9_to_v8(x: v9::SqliteBindParam) -> Result<v8::SqliteBindParam> {
	Ok(match x {
		v9::SqliteBindParam::SqliteValueNull => v8::SqliteBindParam::SqliteValueNull,
		v9::SqliteBindParam::SqliteValueInteger(v) => {
			v8::SqliteBindParam::SqliteValueInteger(convert_sqlite_value_integer_v9_to_v8(v)?)
		}
		v9::SqliteBindParam::SqliteValueFloat(v) => {
			v8::SqliteBindParam::SqliteValueFloat(convert_sqlite_value_float_v9_to_v8(v)?)
		}
		v9::SqliteBindParam::SqliteValueText(v) => {
			v8::SqliteBindParam::SqliteValueText(convert_sqlite_value_text_v9_to_v8(v)?)
		}
		v9::SqliteBindParam::SqliteValueBlob(v) => {
			v8::SqliteBindParam::SqliteValueBlob(convert_sqlite_value_blob_v9_to_v8(v)?)
		}
	})
}

pub fn convert_sqlite_column_value_v9_to_v8(
	x: v9::SqliteColumnValue,
) -> Result<v8::SqliteColumnValue> {
	Ok(match x {
		v9::SqliteColumnValue::SqliteValueNull => v8::SqliteColumnValue::SqliteValueNull,
		v9::SqliteColumnValue::SqliteValueInteger(v) => {
			v8::SqliteColumnValue::SqliteValueInteger(convert_sqlite_value_integer_v9_to_v8(v)?)
		}
		v9::SqliteColumnValue::SqliteValueFloat(v) => {
			v8::SqliteColumnValue::SqliteValueFloat(convert_sqlite_value_float_v9_to_v8(v)?)
		}
		v9::SqliteColumnValue::SqliteValueText(v) => {
			v8::SqliteColumnValue::SqliteValueText(convert_sqlite_value_text_v9_to_v8(v)?)
		}
		v9::SqliteColumnValue::SqliteValueBlob(v) => {
			v8::SqliteColumnValue::SqliteValueBlob(convert_sqlite_value_blob_v9_to_v8(v)?)
		}
	})
}

pub fn convert_sqlite_query_result_v9_to_v8(
	x: v9::SqliteQueryResult,
) -> Result<v8::SqliteQueryResult> {
	Ok(v8::SqliteQueryResult {
		columns: x.columns,
		rows: x
			.rows
			.into_iter()
			.map(|v| {
				v.into_iter()
					.map(|v| convert_sqlite_column_value_v9_to_v8(v))
					.collect::<Result<Vec<_>>>()
			})
			.collect::<Result<Vec<_>>>()?,
	})
}

pub fn convert_sqlite_execute_result_v9_to_v8(
	x: v9::SqliteExecuteResult,
) -> Result<v8::SqliteExecuteResult> {
	Ok(v8::SqliteExecuteResult {
		columns: x.columns,
		rows: x
			.rows
			.into_iter()
			.map(|v| {
				v.into_iter()
					.map(|v| convert_sqlite_column_value_v9_to_v8(v))
					.collect::<Result<Vec<_>>>()
			})
			.collect::<Result<Vec<_>>>()?,
		changes: x.changes,
		last_insert_row_id: x.last_insert_row_id,
	})
}

pub fn convert_sqlite_exec_request_v9_to_v8(
	x: v9::SqliteExecRequest,
) -> Result<v8::SqliteExecRequest> {
	Ok(v8::SqliteExecRequest {
		namespace_id: x.namespace_id,
		actor_id: x.actor_id,
		generation: x.generation,
		sql: x.sql,
	})
}

pub fn convert_sqlite_execute_request_v9_to_v8(
	x: v9::SqliteExecuteRequest,
) -> Result<v8::SqliteExecuteRequest> {
	Ok(v8::SqliteExecuteRequest {
		namespace_id: x.namespace_id,
		actor_id: x.actor_id,
		generation: x.generation,
		sql: x.sql,
		params: x
			.params
			.map(|v| {
				v.into_iter()
					.map(|v| convert_sqlite_bind_param_v9_to_v8(v))
					.collect::<Result<Vec<_>>>()
			})
			.transpose()?,
	})
}

pub fn convert_sqlite_batch_statement_v9_to_v8(
	x: v9::SqliteBatchStatement,
) -> Result<v8::SqliteBatchStatement> {
	Ok(v8::SqliteBatchStatement {
		sql: x.sql,
		params: x
			.params
			.map(|v| {
				v.into_iter()
					.map(|v| convert_sqlite_bind_param_v9_to_v8(v))
					.collect::<Result<Vec<_>>>()
			})
			.transpose()?,
	})
}

pub fn convert_sqlite_execute_batch_request_v9_to_v8(
	x: v9::SqliteExecuteBatchRequest,
) -> Result<v8::SqliteExecuteBatchRequest> {
	Ok(v8::SqliteExecuteBatchRequest {
		namespace_id: x.namespace_id,
		actor_id: x.actor_id,
		generation: x.generation,
		statements: x
			.statements
			.into_iter()
			.map(|v| convert_sqlite_batch_statement_v9_to_v8(v))
			.collect::<Result<Vec<_>>>()?,
	})
}

pub fn convert_sqlite_exec_ok_v9_to_v8(x: v9::SqliteExecOk) -> Result<v8::SqliteExecOk> {
	Ok(v8::SqliteExecOk {
		result: convert_sqlite_query_result_v9_to_v8(x.result)?,
	})
}

pub fn convert_sqlite_execute_ok_v9_to_v8(x: v9::SqliteExecuteOk) -> Result<v8::SqliteExecuteOk> {
	Ok(v8::SqliteExecuteOk {
		result: convert_sqlite_execute_result_v9_to_v8(x.result)?,
	})
}

pub fn convert_sqlite_execute_batch_ok_v9_to_v8(
	x: v9::SqliteExecuteBatchOk,
) -> Result<v8::SqliteExecuteBatchOk> {
	Ok(v8::SqliteExecuteBatchOk {
		results: x
			.results
			.into_iter()
			.map(|v| convert_sqlite_execute_result_v9_to_v8(v))
			.collect::<Result<Vec<_>>>()?,
	})
}

pub fn convert_sqlite_exec_response_v9_to_v8(
	x: v9::SqliteExecResponse,
) -> Result<v8::SqliteExecResponse> {
	Ok(match x {
		v9::SqliteExecResponse::SqliteExecOk(v) => {
			v8::SqliteExecResponse::SqliteExecOk(convert_sqlite_exec_ok_v9_to_v8(v)?)
		}
		v9::SqliteExecResponse::SqliteErrorResponse(v) => {
			v8::SqliteExecResponse::SqliteErrorResponse(convert_sqlite_error_response_v9_to_v8(v)?)
		}
	})
}

pub fn convert_sqlite_execute_response_v9_to_v8(
	x: v9::SqliteExecuteResponse,
) -> Result<v8::SqliteExecuteResponse> {
	Ok(match x {
		v9::SqliteExecuteResponse::SqliteExecuteOk(v) => {
			v8::SqliteExecuteResponse::SqliteExecuteOk(convert_sqlite_execute_ok_v9_to_v8(v)?)
		}
		v9::SqliteExecuteResponse::SqliteErrorResponse(v) => {
			v8::SqliteExecuteResponse::SqliteErrorResponse(convert_sqlite_error_response_v9_to_v8(
				v,
			)?)
		}
	})
}

pub fn convert_sqlite_execute_batch_response_v9_to_v8(
	x: v9::SqliteExecuteBatchResponse,
) -> Result<v8::SqliteExecuteBatchResponse> {
	Ok(match x {
		v9::SqliteExecuteBatchResponse::SqliteExecuteBatchOk(v) => {
			v8::SqliteExecuteBatchResponse::SqliteExecuteBatchOk(
				convert_sqlite_execute_batch_ok_v9_to_v8(v)?,
			)
		}
		v9::SqliteExecuteBatchResponse::SqliteErrorResponse(v) => {
			v8::SqliteExecuteBatchResponse::SqliteErrorResponse(
				convert_sqlite_error_response_v9_to_v8(v)?,
			)
		}
	})
}

pub fn convert_stop_code_v9_to_v8(x: v9::StopCode) -> Result<v8::StopCode> {
	Ok(match x {
		v9::StopCode::Ok => v8::StopCode::Ok,
		v9::StopCode::Error => v8::StopCode::Error,
	})
}

pub fn convert_actor_name_v9_to_v8(x: v9::ActorName) -> Result<v8::ActorName> {
	Ok(v8::ActorName {
		metadata: x.metadata,
	})
}

pub fn convert_actor_config_v9_to_v8(x: v9::ActorConfig) -> Result<v8::ActorConfig> {
	Ok(v8::ActorConfig {
		name: x.name,
		key: x.key,
		create_ts: x.create_ts,
		input: x.input,
	})
}

pub fn convert_actor_checkpoint_v9_to_v8(x: v9::ActorCheckpoint) -> Result<v8::ActorCheckpoint> {
	Ok(v8::ActorCheckpoint {
		actor_id: x.actor_id,
		generation: x.generation,
		index: x.index,
	})
}

pub fn convert_actor_intent_v9_to_v8(x: v9::ActorIntent) -> Result<v8::ActorIntent> {
	Ok(match x {
		v9::ActorIntent::ActorIntentSleep => v8::ActorIntent::ActorIntentSleep,
		v9::ActorIntent::ActorIntentStop => v8::ActorIntent::ActorIntentStop,
	})
}

pub fn convert_actor_state_stopped_v9_to_v8(
	x: v9::ActorStateStopped,
) -> Result<v8::ActorStateStopped> {
	Ok(v8::ActorStateStopped {
		code: convert_stop_code_v9_to_v8(x.code)?,
		message: x.message,
	})
}

pub fn convert_actor_state_v9_to_v8(x: v9::ActorState) -> Result<v8::ActorState> {
	Ok(match x {
		v9::ActorState::ActorStateRunning => v8::ActorState::ActorStateRunning,
		v9::ActorState::ActorStateStopped(v) => {
			v8::ActorState::ActorStateStopped(convert_actor_state_stopped_v9_to_v8(v)?)
		}
	})
}

pub fn convert_event_actor_intent_v9_to_v8(
	x: v9::EventActorIntent,
) -> Result<v8::EventActorIntent> {
	Ok(v8::EventActorIntent {
		intent: convert_actor_intent_v9_to_v8(x.intent)?,
	})
}

pub fn convert_event_actor_state_update_v9_to_v8(
	x: v9::EventActorStateUpdate,
) -> Result<v8::EventActorStateUpdate> {
	Ok(v8::EventActorStateUpdate {
		state: convert_actor_state_v9_to_v8(x.state)?,
	})
}

pub fn convert_event_actor_set_alarm_v9_to_v8(
	x: v9::EventActorSetAlarm,
) -> Result<v8::EventActorSetAlarm> {
	Ok(v8::EventActorSetAlarm {
		alarm_ts: x.alarm_ts,
	})
}

pub fn convert_event_v9_to_v8(x: v9::Event) -> Result<v8::Event> {
	Ok(match x {
		v9::Event::EventActorIntent(v) => {
			v8::Event::EventActorIntent(convert_event_actor_intent_v9_to_v8(v)?)
		}
		v9::Event::EventActorStateUpdate(v) => {
			v8::Event::EventActorStateUpdate(convert_event_actor_state_update_v9_to_v8(v)?)
		}
		v9::Event::EventActorSetAlarm(v) => {
			v8::Event::EventActorSetAlarm(convert_event_actor_set_alarm_v9_to_v8(v)?)
		}
	})
}

pub fn convert_event_wrapper_v9_to_v8(x: v9::EventWrapper) -> Result<v8::EventWrapper> {
	Ok(v8::EventWrapper {
		checkpoint: convert_actor_checkpoint_v9_to_v8(x.checkpoint)?,
		inner: convert_event_v9_to_v8(x.inner)?,
	})
}

pub fn convert_preloaded_kv_entry_v9_to_v8(
	x: v9::PreloadedKvEntry,
) -> Result<v8::PreloadedKvEntry> {
	Ok(v8::PreloadedKvEntry {
		key: x.key,
		value: x.value,
		metadata: convert_kv_metadata_v9_to_v8(x.metadata)?,
	})
}

pub fn convert_preloaded_kv_v9_to_v8(x: v9::PreloadedKv) -> Result<v8::PreloadedKv> {
	Ok(v8::PreloadedKv {
		entries: x
			.entries
			.into_iter()
			.map(|v| convert_preloaded_kv_entry_v9_to_v8(v))
			.collect::<Result<Vec<_>>>()?,
		requested_get_keys: x.requested_get_keys,
		requested_prefixes: x.requested_prefixes,
	})
}

pub fn convert_hibernating_request_v9_to_v8(
	x: v9::HibernatingRequest,
) -> Result<v8::HibernatingRequest> {
	Ok(v8::HibernatingRequest {
		gateway_id: x.gateway_id,
		request_id: x.request_id,
	})
}

/// v8 has no fork, so an older envoy's runtime never runs its fork hook. The forked actor still
/// starts on its copied database, as an actor without a fork hook would.
pub fn convert_command_start_actor_v9_to_v8(
	x: v9::CommandStartActor,
) -> Result<v8::CommandStartActor> {
	Ok(v8::CommandStartActor {
		config: convert_actor_config_v9_to_v8(x.config)?,
		hibernating_requests: x
			.hibernating_requests
			.into_iter()
			.map(|v| convert_hibernating_request_v9_to_v8(v))
			.collect::<Result<Vec<_>>>()?,
		preloaded_kv: x
			.preloaded_kv
			.map(|v| convert_preloaded_kv_v9_to_v8(v))
			.transpose()?,
	})
}

pub fn convert_stop_actor_reason_v9_to_v8(x: v9::StopActorReason) -> Result<v8::StopActorReason> {
	Ok(match x {
		v9::StopActorReason::SleepIntent => v8::StopActorReason::SleepIntent,
		v9::StopActorReason::StopIntent => v8::StopActorReason::StopIntent,
		v9::StopActorReason::Destroy => v8::StopActorReason::Destroy,
		v9::StopActorReason::GoingAway => v8::StopActorReason::GoingAway,
		v9::StopActorReason::Lost => v8::StopActorReason::Lost,
	})
}

pub fn convert_command_stop_actor_v9_to_v8(
	x: v9::CommandStopActor,
) -> Result<v8::CommandStopActor> {
	Ok(v8::CommandStopActor {
		reason: convert_stop_actor_reason_v9_to_v8(x.reason)?,
	})
}

pub fn convert_command_v9_to_v8(x: v9::Command) -> Result<v8::Command> {
	Ok(match x {
		v9::Command::CommandStartActor(v) => {
			v8::Command::CommandStartActor(convert_command_start_actor_v9_to_v8(v)?)
		}
		v9::Command::CommandStopActor(v) => {
			v8::Command::CommandStopActor(convert_command_stop_actor_v9_to_v8(v)?)
		}
	})
}

/// A v8 receiver checks no group, so the command's group is dropped and it gets v8 semantics.
pub fn convert_command_wrapper_v9_to_v8(x: v9::CommandWrapper) -> Result<v8::CommandWrapper> {
	Ok(v8::CommandWrapper {
		checkpoint: convert_actor_checkpoint_v9_to_v8(x.checkpoint)?,
		inner: convert_command_v9_to_v8(x.inner)?,
	})
}

pub fn convert_actor_command_key_data_v9_to_v8(
	x: v9::ActorCommandKeyData,
) -> Result<v8::ActorCommandKeyData> {
	Ok(match x {
		v9::ActorCommandKeyData::CommandStartActor(v) => {
			v8::ActorCommandKeyData::CommandStartActor(convert_command_start_actor_v9_to_v8(v)?)
		}
		v9::ActorCommandKeyData::CommandStopActor(v) => {
			v8::ActorCommandKeyData::CommandStopActor(convert_command_stop_actor_v9_to_v8(v)?)
		}
	})
}

pub fn convert_message_id_v9_to_v8(x: v9::MessageId) -> Result<v8::MessageId> {
	Ok(v8::MessageId {
		gateway_id: x.gateway_id,
		request_id: x.request_id,
		message_index: x.message_index,
	})
}

/// A v8 receiver checks no group, so the request's group is dropped and it gets v8 semantics.
pub fn convert_to_envoy_request_start_v9_to_v8(
	x: v9::ToEnvoyRequestStart,
) -> Result<v8::ToEnvoyRequestStart> {
	Ok(v8::ToEnvoyRequestStart {
		actor_id: x.actor_id,
		actor_generation: x.actor_generation,
		method: x.method,
		path: x.path,
		headers: x.headers,
		body: x.body,
		stream: x.stream,
		response_stream: x.response_stream,
	})
}

pub fn convert_to_envoy_request_chunk_v9_to_v8(
	x: v9::ToEnvoyRequestChunk,
) -> Result<v8::ToEnvoyRequestChunk> {
	Ok(v8::ToEnvoyRequestChunk {
		body: x.body,
		finish: x.finish,
	})
}

pub fn convert_to_rivet_request_body_window_update_v9_to_v8(
	x: v9::ToRivetRequestBodyWindowUpdate,
) -> Result<v8::ToRivetRequestBodyWindowUpdate> {
	Ok(v8::ToRivetRequestBodyWindowUpdate {
		consumed_bytes: x.consumed_bytes,
	})
}

pub fn convert_http_stream_abort_reason_kind_v9_to_v8(
	x: v9::HttpStreamAbortReasonKind,
) -> Result<v8::HttpStreamAbortReasonKind> {
	Ok(match x {
		v9::HttpStreamAbortReasonKind::Unknown => v8::HttpStreamAbortReasonKind::Unknown,
		v9::HttpStreamAbortReasonKind::Cancelled => v8::HttpStreamAbortReasonKind::Cancelled,
		v9::HttpStreamAbortReasonKind::HandlerError => v8::HttpStreamAbortReasonKind::HandlerError,
		v9::HttpStreamAbortReasonKind::InternalError => {
			v8::HttpStreamAbortReasonKind::InternalError
		}
	})
}

pub fn convert_http_stream_abort_reason_v9_to_v8(
	x: v9::HttpStreamAbortReason,
) -> Result<v8::HttpStreamAbortReason> {
	Ok(v8::HttpStreamAbortReason {
		kind: convert_http_stream_abort_reason_kind_v9_to_v8(x.kind)?,
		detail: x.detail,
	})
}

pub fn convert_to_envoy_request_abort_v9_to_v8(
	x: v9::ToEnvoyRequestAbort,
) -> Result<v8::ToEnvoyRequestAbort> {
	Ok(v8::ToEnvoyRequestAbort {
		actor_id: x.actor_id,
		actor_generation: x.actor_generation,
		reason: convert_http_stream_abort_reason_v9_to_v8(x.reason)?,
	})
}

pub fn convert_to_rivet_response_start_v9_to_v8(
	x: v9::ToRivetResponseStart,
) -> Result<v8::ToRivetResponseStart> {
	Ok(v8::ToRivetResponseStart {
		status: x.status,
		headers: x.headers,
		body: x.body,
		stream: x.stream,
	})
}

pub fn convert_to_rivet_response_chunk_v9_to_v8(
	x: v9::ToRivetResponseChunk,
) -> Result<v8::ToRivetResponseChunk> {
	Ok(v8::ToRivetResponseChunk {
		body: x.body,
		finish: x.finish,
	})
}

pub fn convert_to_envoy_response_body_window_update_v9_to_v8(
	x: v9::ToEnvoyResponseBodyWindowUpdate,
) -> Result<v8::ToEnvoyResponseBodyWindowUpdate> {
	Ok(v8::ToEnvoyResponseBodyWindowUpdate {
		consumed_bytes: x.consumed_bytes,
	})
}

pub fn convert_to_rivet_response_abort_v9_to_v8(
	x: v9::ToRivetResponseAbort,
) -> Result<v8::ToRivetResponseAbort> {
	Ok(v8::ToRivetResponseAbort {
		reason: convert_http_stream_abort_reason_v9_to_v8(x.reason)?,
	})
}

/// A v8 receiver checks no group, so the open's group is dropped and it gets v8 semantics.
pub fn convert_to_envoy_web_socket_open_v9_to_v8(
	x: v9::ToEnvoyWebSocketOpen,
) -> Result<v8::ToEnvoyWebSocketOpen> {
	Ok(v8::ToEnvoyWebSocketOpen {
		actor_id: x.actor_id,
		actor_generation: x.actor_generation,
		path: x.path,
		headers: x.headers,
	})
}

pub fn convert_to_envoy_web_socket_message_v9_to_v8(
	x: v9::ToEnvoyWebSocketMessage,
) -> Result<v8::ToEnvoyWebSocketMessage> {
	Ok(v8::ToEnvoyWebSocketMessage {
		data: x.data,
		binary: x.binary,
	})
}

pub fn convert_to_envoy_web_socket_close_v9_to_v8(
	x: v9::ToEnvoyWebSocketClose,
) -> Result<v8::ToEnvoyWebSocketClose> {
	Ok(v8::ToEnvoyWebSocketClose {
		code: x.code,
		reason: x.reason,
	})
}

pub fn convert_to_rivet_web_socket_open_v9_to_v8(
	x: v9::ToRivetWebSocketOpen,
) -> Result<v8::ToRivetWebSocketOpen> {
	Ok(v8::ToRivetWebSocketOpen {
		can_hibernate: x.can_hibernate,
	})
}

pub fn convert_to_rivet_web_socket_message_v9_to_v8(
	x: v9::ToRivetWebSocketMessage,
) -> Result<v8::ToRivetWebSocketMessage> {
	Ok(v8::ToRivetWebSocketMessage {
		data: x.data,
		binary: x.binary,
	})
}

pub fn convert_to_rivet_web_socket_message_ack_v9_to_v8(
	x: v9::ToRivetWebSocketMessageAck,
) -> Result<v8::ToRivetWebSocketMessageAck> {
	Ok(v8::ToRivetWebSocketMessageAck { index: x.index })
}

pub fn convert_to_rivet_web_socket_close_v9_to_v8(
	x: v9::ToRivetWebSocketClose,
) -> Result<v8::ToRivetWebSocketClose> {
	Ok(v8::ToRivetWebSocketClose {
		code: x.code,
		reason: x.reason,
		hibernate: x.hibernate,
	})
}

pub fn convert_to_rivet_tunnel_message_kind_v9_to_v8(
	x: v9::ToRivetTunnelMessageKind,
) -> Result<v8::ToRivetTunnelMessageKind> {
	Ok(match x {
		v9::ToRivetTunnelMessageKind::ToRivetResponseStart(v) => {
			v8::ToRivetTunnelMessageKind::ToRivetResponseStart(
				convert_to_rivet_response_start_v9_to_v8(v)?,
			)
		}
		v9::ToRivetTunnelMessageKind::ToRivetResponseChunk(v) => {
			v8::ToRivetTunnelMessageKind::ToRivetResponseChunk(
				convert_to_rivet_response_chunk_v9_to_v8(v)?,
			)
		}
		v9::ToRivetTunnelMessageKind::ToRivetResponseAbort(v) => {
			v8::ToRivetTunnelMessageKind::ToRivetResponseAbort(
				convert_to_rivet_response_abort_v9_to_v8(v)?,
			)
		}
		v9::ToRivetTunnelMessageKind::ToRivetRequestBodyWindowUpdate(v) => {
			v8::ToRivetTunnelMessageKind::ToRivetRequestBodyWindowUpdate(
				convert_to_rivet_request_body_window_update_v9_to_v8(v)?,
			)
		}
		v9::ToRivetTunnelMessageKind::ToRivetRequestBodyCancel => {
			v8::ToRivetTunnelMessageKind::ToRivetRequestBodyCancel
		}
		v9::ToRivetTunnelMessageKind::ToRivetWebSocketOpen(v) => {
			v8::ToRivetTunnelMessageKind::ToRivetWebSocketOpen(
				convert_to_rivet_web_socket_open_v9_to_v8(v)?,
			)
		}
		v9::ToRivetTunnelMessageKind::ToRivetWebSocketMessage(v) => {
			v8::ToRivetTunnelMessageKind::ToRivetWebSocketMessage(
				convert_to_rivet_web_socket_message_v9_to_v8(v)?,
			)
		}
		v9::ToRivetTunnelMessageKind::ToRivetWebSocketMessageAck(v) => {
			v8::ToRivetTunnelMessageKind::ToRivetWebSocketMessageAck(
				convert_to_rivet_web_socket_message_ack_v9_to_v8(v)?,
			)
		}
		v9::ToRivetTunnelMessageKind::ToRivetWebSocketClose(v) => {
			v8::ToRivetTunnelMessageKind::ToRivetWebSocketClose(
				convert_to_rivet_web_socket_close_v9_to_v8(v)?,
			)
		}
	})
}

pub fn convert_to_rivet_tunnel_message_v9_to_v8(
	x: v9::ToRivetTunnelMessage,
) -> Result<v8::ToRivetTunnelMessage> {
	Ok(v8::ToRivetTunnelMessage {
		message_id: convert_message_id_v9_to_v8(x.message_id)?,
		message_kind: convert_to_rivet_tunnel_message_kind_v9_to_v8(x.message_kind)?,
	})
}

pub fn convert_to_envoy_tunnel_message_kind_v9_to_v8(
	x: v9::ToEnvoyTunnelMessageKind,
) -> Result<v8::ToEnvoyTunnelMessageKind> {
	Ok(match x {
		v9::ToEnvoyTunnelMessageKind::ToEnvoyRequestStart(v) => {
			v8::ToEnvoyTunnelMessageKind::ToEnvoyRequestStart(
				convert_to_envoy_request_start_v9_to_v8(v)?,
			)
		}
		v9::ToEnvoyTunnelMessageKind::ToEnvoyRequestChunk(v) => {
			v8::ToEnvoyTunnelMessageKind::ToEnvoyRequestChunk(
				convert_to_envoy_request_chunk_v9_to_v8(v)?,
			)
		}
		v9::ToEnvoyTunnelMessageKind::ToEnvoyRequestAbort(v) => {
			v8::ToEnvoyTunnelMessageKind::ToEnvoyRequestAbort(
				convert_to_envoy_request_abort_v9_to_v8(v)?,
			)
		}
		v9::ToEnvoyTunnelMessageKind::ToEnvoyRequestBodyCancel => {
			v8::ToEnvoyTunnelMessageKind::ToEnvoyRequestBodyCancel
		}
		v9::ToEnvoyTunnelMessageKind::ToEnvoyResponseBodyWindowUpdate(v) => {
			v8::ToEnvoyTunnelMessageKind::ToEnvoyResponseBodyWindowUpdate(
				convert_to_envoy_response_body_window_update_v9_to_v8(v)?,
			)
		}
		v9::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketOpen(v) => {
			v8::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketOpen(
				convert_to_envoy_web_socket_open_v9_to_v8(v)?,
			)
		}
		v9::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketMessage(v) => {
			v8::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketMessage(
				convert_to_envoy_web_socket_message_v9_to_v8(v)?,
			)
		}
		v9::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketClose(v) => {
			v8::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketClose(
				convert_to_envoy_web_socket_close_v9_to_v8(v)?,
			)
		}
	})
}

pub fn convert_to_envoy_tunnel_message_v9_to_v8(
	x: v9::ToEnvoyTunnelMessage,
) -> Result<v8::ToEnvoyTunnelMessage> {
	Ok(v8::ToEnvoyTunnelMessage {
		message_id: convert_message_id_v9_to_v8(x.message_id)?,
		message_kind: convert_to_envoy_tunnel_message_kind_v9_to_v8(x.message_kind)?,
	})
}

pub fn convert_to_envoy_ping_v9_to_v8(x: v9::ToEnvoyPing) -> Result<v8::ToEnvoyPing> {
	Ok(v8::ToEnvoyPing { ts: x.ts })
}

pub fn convert_to_rivet_metadata_v9_to_v8(x: v9::ToRivetMetadata) -> Result<v8::ToRivetMetadata> {
	Ok(v8::ToRivetMetadata {
		prepopulate_actor_names: x
			.prepopulate_actor_names
			.map(|v| {
				v.into_iter()
					.map(|(k, v)| -> Result<_> { Ok((k, convert_actor_name_v9_to_v8(v)?)) })
					.collect::<Result<_>>()
			})
			.transpose()?,
		metadata: x.metadata,
	})
}

pub fn convert_to_rivet_events_v9_to_v8(x: v9::ToRivetEvents) -> Result<v8::ToRivetEvents> {
	Ok(x.into_iter()
		.map(|v| convert_event_wrapper_v9_to_v8(v))
		.collect::<Result<Vec<_>>>()?)
}

pub fn convert_to_rivet_ack_commands_v9_to_v8(
	x: v9::ToRivetAckCommands,
) -> Result<v8::ToRivetAckCommands> {
	Ok(v8::ToRivetAckCommands {
		last_command_checkpoints: x
			.last_command_checkpoints
			.into_iter()
			.map(|v| convert_actor_checkpoint_v9_to_v8(v))
			.collect::<Result<Vec<_>>>()?,
	})
}

pub fn convert_to_rivet_pong_v9_to_v8(x: v9::ToRivetPong) -> Result<v8::ToRivetPong> {
	Ok(v8::ToRivetPong { ts: x.ts })
}

pub fn convert_to_rivet_kv_request_v9_to_v8(
	x: v9::ToRivetKvRequest,
) -> Result<v8::ToRivetKvRequest> {
	Ok(v8::ToRivetKvRequest {
		actor_id: x.actor_id,
		request_id: x.request_id,
		data: convert_kv_request_data_v9_to_v8(x.data)?,
	})
}

pub fn convert_to_rivet_sqlite_get_pages_request_v9_to_v8(
	x: v9::ToRivetSqliteGetPagesRequest,
) -> Result<v8::ToRivetSqliteGetPagesRequest> {
	Ok(v8::ToRivetSqliteGetPagesRequest {
		request_id: x.request_id,
		data: convert_sqlite_get_pages_request_v9_to_v8(x.data)?,
	})
}

pub fn convert_to_rivet_sqlite_commit_request_v9_to_v8(
	x: v9::ToRivetSqliteCommitRequest,
) -> Result<v8::ToRivetSqliteCommitRequest> {
	Ok(v8::ToRivetSqliteCommitRequest {
		request_id: x.request_id,
		data: convert_sqlite_commit_request_v9_to_v8(x.data)?,
	})
}

pub fn convert_to_rivet_sqlite_commit_stage_begin_request_v9_to_v8(
	x: v9::ToRivetSqliteCommitStageBeginRequest,
) -> Result<v8::ToRivetSqliteCommitStageBeginRequest> {
	Ok(v8::ToRivetSqliteCommitStageBeginRequest {
		request_id: x.request_id,
		data: convert_sqlite_commit_stage_begin_request_v9_to_v8(x.data)?,
	})
}

pub fn convert_to_rivet_sqlite_commit_stage_segment_request_v9_to_v8(
	x: v9::ToRivetSqliteCommitStageSegmentRequest,
) -> Result<v8::ToRivetSqliteCommitStageSegmentRequest> {
	Ok(v8::ToRivetSqliteCommitStageSegmentRequest {
		request_id: x.request_id,
		data: convert_sqlite_commit_stage_segment_request_v9_to_v8(x.data)?,
	})
}

pub fn convert_to_rivet_sqlite_commit_finalize_request_v9_to_v8(
	x: v9::ToRivetSqliteCommitFinalizeRequest,
) -> Result<v8::ToRivetSqliteCommitFinalizeRequest> {
	Ok(v8::ToRivetSqliteCommitFinalizeRequest {
		request_id: x.request_id,
		data: convert_sqlite_commit_finalize_request_v9_to_v8(x.data)?,
	})
}

pub fn convert_to_rivet_sqlite_exec_request_v9_to_v8(
	x: v9::ToRivetSqliteExecRequest,
) -> Result<v8::ToRivetSqliteExecRequest> {
	Ok(v8::ToRivetSqliteExecRequest {
		request_id: x.request_id,
		data: convert_sqlite_exec_request_v9_to_v8(x.data)?,
	})
}

pub fn convert_to_rivet_sqlite_execute_request_v9_to_v8(
	x: v9::ToRivetSqliteExecuteRequest,
) -> Result<v8::ToRivetSqliteExecuteRequest> {
	Ok(v8::ToRivetSqliteExecuteRequest {
		request_id: x.request_id,
		data: convert_sqlite_execute_request_v9_to_v8(x.data)?,
	})
}

pub fn convert_to_rivet_sqlite_execute_batch_request_v9_to_v8(
	x: v9::ToRivetSqliteExecuteBatchRequest,
) -> Result<v8::ToRivetSqliteExecuteBatchRequest> {
	Ok(v8::ToRivetSqliteExecuteBatchRequest {
		request_id: x.request_id,
		data: convert_sqlite_execute_batch_request_v9_to_v8(x.data)?,
	})
}

pub fn convert_to_rivet_v9_to_v8(x: v9::ToRivet) -> Result<v8::ToRivet> {
	Ok(match x {
		v9::ToRivet::ToRivetMetadata(v) => {
			v8::ToRivet::ToRivetMetadata(convert_to_rivet_metadata_v9_to_v8(v)?)
		}
		v9::ToRivet::ToRivetEvents(v) => {
			v8::ToRivet::ToRivetEvents(convert_to_rivet_events_v9_to_v8(v)?)
		}
		v9::ToRivet::ToRivetAckCommands(v) => {
			v8::ToRivet::ToRivetAckCommands(convert_to_rivet_ack_commands_v9_to_v8(v)?)
		}
		v9::ToRivet::ToRivetStopping => v8::ToRivet::ToRivetStopping,
		v9::ToRivet::ToRivetPong(v) => v8::ToRivet::ToRivetPong(convert_to_rivet_pong_v9_to_v8(v)?),
		v9::ToRivet::ToRivetKvRequest(v) => {
			v8::ToRivet::ToRivetKvRequest(convert_to_rivet_kv_request_v9_to_v8(v)?)
		}
		v9::ToRivet::ToRivetTunnelMessage(v) => {
			v8::ToRivet::ToRivetTunnelMessage(convert_to_rivet_tunnel_message_v9_to_v8(v)?)
		}
		v9::ToRivet::ToRivetSqliteGetPagesRequest(v) => v8::ToRivet::ToRivetSqliteGetPagesRequest(
			convert_to_rivet_sqlite_get_pages_request_v9_to_v8(v)?,
		),
		v9::ToRivet::ToRivetSqliteCommitRequest(v) => v8::ToRivet::ToRivetSqliteCommitRequest(
			convert_to_rivet_sqlite_commit_request_v9_to_v8(v)?,
		),
		v9::ToRivet::ToRivetSqliteCommitStageBeginRequest(v) => {
			v8::ToRivet::ToRivetSqliteCommitStageBeginRequest(
				convert_to_rivet_sqlite_commit_stage_begin_request_v9_to_v8(v)?,
			)
		}
		v9::ToRivet::ToRivetSqliteCommitStageSegmentRequest(v) => {
			v8::ToRivet::ToRivetSqliteCommitStageSegmentRequest(
				convert_to_rivet_sqlite_commit_stage_segment_request_v9_to_v8(v)?,
			)
		}
		v9::ToRivet::ToRivetSqliteCommitFinalizeRequest(v) => {
			v8::ToRivet::ToRivetSqliteCommitFinalizeRequest(
				convert_to_rivet_sqlite_commit_finalize_request_v9_to_v8(v)?,
			)
		}
		v9::ToRivet::ToRivetSqliteExecRequest(v) => {
			v8::ToRivet::ToRivetSqliteExecRequest(convert_to_rivet_sqlite_exec_request_v9_to_v8(v)?)
		}
		v9::ToRivet::ToRivetSqliteExecuteRequest(v) => v8::ToRivet::ToRivetSqliteExecuteRequest(
			convert_to_rivet_sqlite_execute_request_v9_to_v8(v)?,
		),
		v9::ToRivet::ToRivetSqliteExecuteBatchRequest(v) => {
			v8::ToRivet::ToRivetSqliteExecuteBatchRequest(
				convert_to_rivet_sqlite_execute_batch_request_v9_to_v8(v)?,
			)
		}
	})
}

pub fn convert_protocol_metadata_v9_to_v8(x: v9::ProtocolMetadata) -> Result<v8::ProtocolMetadata> {
	Ok(v8::ProtocolMetadata {
		envoy_lost_threshold: x.envoy_lost_threshold,
		actor_stop_threshold: x.actor_stop_threshold,
		max_response_payload_size: x.max_response_payload_size,
	})
}

pub fn convert_to_envoy_init_v9_to_v8(x: v9::ToEnvoyInit) -> Result<v8::ToEnvoyInit> {
	Ok(v8::ToEnvoyInit {
		metadata: convert_protocol_metadata_v9_to_v8(x.metadata)?,
	})
}

pub fn convert_to_envoy_commands_v9_to_v8(x: v9::ToEnvoyCommands) -> Result<v8::ToEnvoyCommands> {
	Ok(x.into_iter()
		.map(|v| convert_command_wrapper_v9_to_v8(v))
		.collect::<Result<Vec<_>>>()?)
}

pub fn convert_to_envoy_ack_events_v9_to_v8(
	x: v9::ToEnvoyAckEvents,
) -> Result<v8::ToEnvoyAckEvents> {
	Ok(v8::ToEnvoyAckEvents {
		last_event_checkpoints: x
			.last_event_checkpoints
			.into_iter()
			.map(|v| convert_actor_checkpoint_v9_to_v8(v))
			.collect::<Result<Vec<_>>>()?,
	})
}

pub fn convert_to_envoy_kv_response_v9_to_v8(
	x: v9::ToEnvoyKvResponse,
) -> Result<v8::ToEnvoyKvResponse> {
	Ok(v8::ToEnvoyKvResponse {
		request_id: x.request_id,
		data: convert_kv_response_data_v9_to_v8(x.data)?,
	})
}

pub fn convert_to_envoy_sqlite_get_pages_response_v9_to_v8(
	x: v9::ToEnvoySqliteGetPagesResponse,
) -> Result<v8::ToEnvoySqliteGetPagesResponse> {
	Ok(v8::ToEnvoySqliteGetPagesResponse {
		request_id: x.request_id,
		data: convert_sqlite_get_pages_response_v9_to_v8(x.data)?,
	})
}

pub fn convert_to_envoy_sqlite_commit_response_v9_to_v8(
	x: v9::ToEnvoySqliteCommitResponse,
) -> Result<v8::ToEnvoySqliteCommitResponse> {
	Ok(v8::ToEnvoySqliteCommitResponse {
		request_id: x.request_id,
		data: convert_sqlite_commit_response_v9_to_v8(x.data)?,
	})
}

pub fn convert_to_envoy_sqlite_commit_stage_begin_response_v9_to_v8(
	x: v9::ToEnvoySqliteCommitStageBeginResponse,
) -> Result<v8::ToEnvoySqliteCommitStageBeginResponse> {
	Ok(v8::ToEnvoySqliteCommitStageBeginResponse {
		request_id: x.request_id,
		data: convert_sqlite_commit_stage_begin_response_v9_to_v8(x.data)?,
	})
}

pub fn convert_to_envoy_sqlite_commit_stage_segment_response_v9_to_v8(
	x: v9::ToEnvoySqliteCommitStageSegmentResponse,
) -> Result<v8::ToEnvoySqliteCommitStageSegmentResponse> {
	Ok(v8::ToEnvoySqliteCommitStageSegmentResponse {
		request_id: x.request_id,
		data: convert_sqlite_commit_stage_segment_response_v9_to_v8(x.data)?,
	})
}

pub fn convert_to_envoy_sqlite_commit_finalize_response_v9_to_v8(
	x: v9::ToEnvoySqliteCommitFinalizeResponse,
) -> Result<v8::ToEnvoySqliteCommitFinalizeResponse> {
	Ok(v8::ToEnvoySqliteCommitFinalizeResponse {
		request_id: x.request_id,
		data: convert_sqlite_commit_finalize_response_v9_to_v8(x.data)?,
	})
}

pub fn convert_to_envoy_sqlite_exec_response_v9_to_v8(
	x: v9::ToEnvoySqliteExecResponse,
) -> Result<v8::ToEnvoySqliteExecResponse> {
	Ok(v8::ToEnvoySqliteExecResponse {
		request_id: x.request_id,
		data: convert_sqlite_exec_response_v9_to_v8(x.data)?,
	})
}

pub fn convert_to_envoy_sqlite_execute_response_v9_to_v8(
	x: v9::ToEnvoySqliteExecuteResponse,
) -> Result<v8::ToEnvoySqliteExecuteResponse> {
	Ok(v8::ToEnvoySqliteExecuteResponse {
		request_id: x.request_id,
		data: convert_sqlite_execute_response_v9_to_v8(x.data)?,
	})
}

pub fn convert_to_envoy_sqlite_execute_batch_response_v9_to_v8(
	x: v9::ToEnvoySqliteExecuteBatchResponse,
) -> Result<v8::ToEnvoySqliteExecuteBatchResponse> {
	Ok(v8::ToEnvoySqliteExecuteBatchResponse {
		request_id: x.request_id,
		data: convert_sqlite_execute_batch_response_v9_to_v8(x.data)?,
	})
}

pub fn convert_to_envoy_v9_to_v8(x: v9::ToEnvoy) -> Result<v8::ToEnvoy> {
	Ok(match x {
		v9::ToEnvoy::ToEnvoyInit(v) => v8::ToEnvoy::ToEnvoyInit(convert_to_envoy_init_v9_to_v8(v)?),
		v9::ToEnvoy::ToEnvoyCommands(v) => {
			v8::ToEnvoy::ToEnvoyCommands(convert_to_envoy_commands_v9_to_v8(v)?)
		}
		v9::ToEnvoy::ToEnvoyAckEvents(v) => {
			v8::ToEnvoy::ToEnvoyAckEvents(convert_to_envoy_ack_events_v9_to_v8(v)?)
		}
		v9::ToEnvoy::ToEnvoyKvResponse(v) => {
			v8::ToEnvoy::ToEnvoyKvResponse(convert_to_envoy_kv_response_v9_to_v8(v)?)
		}
		v9::ToEnvoy::ToEnvoyTunnelMessage(v) => {
			v8::ToEnvoy::ToEnvoyTunnelMessage(convert_to_envoy_tunnel_message_v9_to_v8(v)?)
		}
		v9::ToEnvoy::ToEnvoyPing(v) => v8::ToEnvoy::ToEnvoyPing(convert_to_envoy_ping_v9_to_v8(v)?),
		v9::ToEnvoy::ToEnvoySqliteGetPagesResponse(v) => {
			v8::ToEnvoy::ToEnvoySqliteGetPagesResponse(
				convert_to_envoy_sqlite_get_pages_response_v9_to_v8(v)?,
			)
		}
		v9::ToEnvoy::ToEnvoySqliteCommitResponse(v) => v8::ToEnvoy::ToEnvoySqliteCommitResponse(
			convert_to_envoy_sqlite_commit_response_v9_to_v8(v)?,
		),
		v9::ToEnvoy::ToEnvoySqliteCommitStageBeginResponse(v) => {
			v8::ToEnvoy::ToEnvoySqliteCommitStageBeginResponse(
				convert_to_envoy_sqlite_commit_stage_begin_response_v9_to_v8(v)?,
			)
		}
		v9::ToEnvoy::ToEnvoySqliteCommitStageSegmentResponse(v) => {
			v8::ToEnvoy::ToEnvoySqliteCommitStageSegmentResponse(
				convert_to_envoy_sqlite_commit_stage_segment_response_v9_to_v8(v)?,
			)
		}
		v9::ToEnvoy::ToEnvoySqliteCommitFinalizeResponse(v) => {
			v8::ToEnvoy::ToEnvoySqliteCommitFinalizeResponse(
				convert_to_envoy_sqlite_commit_finalize_response_v9_to_v8(v)?,
			)
		}
		v9::ToEnvoy::ToEnvoySqliteExecResponse(v) => v8::ToEnvoy::ToEnvoySqliteExecResponse(
			convert_to_envoy_sqlite_exec_response_v9_to_v8(v)?,
		),
		v9::ToEnvoy::ToEnvoySqliteExecuteResponse(v) => v8::ToEnvoy::ToEnvoySqliteExecuteResponse(
			convert_to_envoy_sqlite_execute_response_v9_to_v8(v)?,
		),
		v9::ToEnvoy::ToEnvoySqliteExecuteBatchResponse(v) => {
			v8::ToEnvoy::ToEnvoySqliteExecuteBatchResponse(
				convert_to_envoy_sqlite_execute_batch_response_v9_to_v8(v)?,
			)
		}
		// v8 has no way to ask for a tail proof. Senders check the receiver's version first
		// and skip the request, so this refusal only guards a sender that did not.
		v9::ToEnvoy::ToEnvoySqliteProveTail(_) => {
			anyhow::bail!("envoy protocol v8 cannot carry a SQLite tail proof request")
		}
	})
}

pub fn convert_to_envoy_conn_ping_v9_to_v8(x: v9::ToEnvoyConnPing) -> Result<v8::ToEnvoyConnPing> {
	Ok(v8::ToEnvoyConnPing {
		gateway_id: x.gateway_id,
		request_id: x.request_id,
		ts: x.ts,
	})
}

pub fn convert_to_envoy_conn_v9_to_v8(x: v9::ToEnvoyConn) -> Result<v8::ToEnvoyConn> {
	Ok(match x {
		v9::ToEnvoyConn::ToEnvoyConnPing(v) => {
			v8::ToEnvoyConn::ToEnvoyConnPing(convert_to_envoy_conn_ping_v9_to_v8(v)?)
		}
		v9::ToEnvoyConn::ToEnvoyConnClose => v8::ToEnvoyConn::ToEnvoyConnClose,
		v9::ToEnvoyConn::ToEnvoyCommands(v) => {
			v8::ToEnvoyConn::ToEnvoyCommands(convert_to_envoy_commands_v9_to_v8(v)?)
		}
		v9::ToEnvoyConn::ToEnvoyAckEvents(v) => {
			v8::ToEnvoyConn::ToEnvoyAckEvents(convert_to_envoy_ack_events_v9_to_v8(v)?)
		}
		v9::ToEnvoyConn::ToEnvoyTunnelMessage(v) => {
			v8::ToEnvoyConn::ToEnvoyTunnelMessage(convert_to_envoy_tunnel_message_v9_to_v8(v)?)
		}
	})
}

pub fn convert_to_gateway_pong_v9_to_v8(x: v9::ToGatewayPong) -> Result<v8::ToGatewayPong> {
	Ok(v8::ToGatewayPong {
		request_id: x.request_id,
		ts: x.ts,
	})
}

pub fn convert_to_gateway_v9_to_v8(x: v9::ToGateway) -> Result<v8::ToGateway> {
	Ok(match x {
		v9::ToGateway::ToGatewayPong(v) => {
			v8::ToGateway::ToGatewayPong(convert_to_gateway_pong_v9_to_v8(v)?)
		}
		v9::ToGateway::ToRivetTunnelMessage(v) => {
			v8::ToGateway::ToRivetTunnelMessage(convert_to_rivet_tunnel_message_v9_to_v8(v)?)
		}
	})
}

pub fn convert_to_outbound_actor_start_v9_to_v8(
	x: v9::ToOutboundActorStart,
) -> Result<v8::ToOutboundActorStart> {
	Ok(v8::ToOutboundActorStart {
		namespace_id: x.namespace_id,
		pool_name: x.pool_name,
		checkpoint: convert_actor_checkpoint_v9_to_v8(x.checkpoint)?,
		actor_config: convert_actor_config_v9_to_v8(x.actor_config)?,
	})
}

pub fn convert_to_outbound_v9_to_v8(x: v9::ToOutbound) -> Result<v8::ToOutbound> {
	Ok(match x {
		v9::ToOutbound::ToOutboundActorStart(v) => {
			v8::ToOutbound::ToOutboundActorStart(convert_to_outbound_actor_start_v9_to_v8(v)?)
		}
	})
}
