//! Converters between envoy protocol v9 and v10. v10 adds the drain period to the protocol
//! metadata, which v9 cannot carry; every other type is unchanged and is copied field by field.

use anyhow::{Result, bail};

use crate::generated::{v9, v10};

pub fn convert_kv_metadata_v9_to_v10(x: v9::KvMetadata) -> Result<v10::KvMetadata> {
	Ok(v10::KvMetadata {
		version: x.version,
		update_ts: x.update_ts,
	})
}

pub fn convert_kv_list_range_query_v9_to_v10(
	x: v9::KvListRangeQuery,
) -> Result<v10::KvListRangeQuery> {
	Ok(v10::KvListRangeQuery {
		start: x.start,
		end: x.end,
		exclusive: x.exclusive,
	})
}

pub fn convert_kv_list_prefix_query_v9_to_v10(
	x: v9::KvListPrefixQuery,
) -> Result<v10::KvListPrefixQuery> {
	Ok(v10::KvListPrefixQuery { key: x.key })
}

pub fn convert_kv_list_query_v9_to_v10(x: v9::KvListQuery) -> Result<v10::KvListQuery> {
	Ok(match x {
		v9::KvListQuery::KvListAllQuery => v10::KvListQuery::KvListAllQuery,
		v9::KvListQuery::KvListRangeQuery(v) => {
			v10::KvListQuery::KvListRangeQuery(convert_kv_list_range_query_v9_to_v10(v)?)
		}
		v9::KvListQuery::KvListPrefixQuery(v) => {
			v10::KvListQuery::KvListPrefixQuery(convert_kv_list_prefix_query_v9_to_v10(v)?)
		}
	})
}

pub fn convert_kv_get_request_v9_to_v10(x: v9::KvGetRequest) -> Result<v10::KvGetRequest> {
	Ok(v10::KvGetRequest { keys: x.keys })
}

pub fn convert_kv_list_request_v9_to_v10(x: v9::KvListRequest) -> Result<v10::KvListRequest> {
	Ok(v10::KvListRequest {
		query: convert_kv_list_query_v9_to_v10(x.query)?,
		reverse: x.reverse,
		limit: x.limit,
	})
}

pub fn convert_kv_put_request_v9_to_v10(x: v9::KvPutRequest) -> Result<v10::KvPutRequest> {
	Ok(v10::KvPutRequest {
		keys: x.keys,
		values: x.values,
	})
}

pub fn convert_kv_delete_request_v9_to_v10(x: v9::KvDeleteRequest) -> Result<v10::KvDeleteRequest> {
	Ok(v10::KvDeleteRequest { keys: x.keys })
}

pub fn convert_kv_delete_range_request_v9_to_v10(
	x: v9::KvDeleteRangeRequest,
) -> Result<v10::KvDeleteRangeRequest> {
	Ok(v10::KvDeleteRangeRequest {
		start: x.start,
		end: x.end,
	})
}

pub fn convert_kv_error_response_v9_to_v10(x: v9::KvErrorResponse) -> Result<v10::KvErrorResponse> {
	Ok(v10::KvErrorResponse { message: x.message })
}

pub fn convert_kv_get_response_v9_to_v10(x: v9::KvGetResponse) -> Result<v10::KvGetResponse> {
	Ok(v10::KvGetResponse {
		keys: x.keys,
		values: x.values,
		metadata: x
			.metadata
			.into_iter()
			.map(|v| convert_kv_metadata_v9_to_v10(v))
			.collect::<Result<Vec<_>>>()?,
	})
}

pub fn convert_kv_list_response_v9_to_v10(x: v9::KvListResponse) -> Result<v10::KvListResponse> {
	Ok(v10::KvListResponse {
		keys: x.keys,
		values: x.values,
		metadata: x
			.metadata
			.into_iter()
			.map(|v| convert_kv_metadata_v9_to_v10(v))
			.collect::<Result<Vec<_>>>()?,
	})
}

pub fn convert_kv_request_data_v9_to_v10(x: v9::KvRequestData) -> Result<v10::KvRequestData> {
	Ok(match x {
		v9::KvRequestData::KvGetRequest(v) => {
			v10::KvRequestData::KvGetRequest(convert_kv_get_request_v9_to_v10(v)?)
		}
		v9::KvRequestData::KvListRequest(v) => {
			v10::KvRequestData::KvListRequest(convert_kv_list_request_v9_to_v10(v)?)
		}
		v9::KvRequestData::KvPutRequest(v) => {
			v10::KvRequestData::KvPutRequest(convert_kv_put_request_v9_to_v10(v)?)
		}
		v9::KvRequestData::KvDeleteRequest(v) => {
			v10::KvRequestData::KvDeleteRequest(convert_kv_delete_request_v9_to_v10(v)?)
		}
		v9::KvRequestData::KvDeleteRangeRequest(v) => {
			v10::KvRequestData::KvDeleteRangeRequest(convert_kv_delete_range_request_v9_to_v10(v)?)
		}
		v9::KvRequestData::KvDropRequest => v10::KvRequestData::KvDropRequest,
	})
}

pub fn convert_kv_response_data_v9_to_v10(x: v9::KvResponseData) -> Result<v10::KvResponseData> {
	Ok(match x {
		v9::KvResponseData::KvErrorResponse(v) => {
			v10::KvResponseData::KvErrorResponse(convert_kv_error_response_v9_to_v10(v)?)
		}
		v9::KvResponseData::KvGetResponse(v) => {
			v10::KvResponseData::KvGetResponse(convert_kv_get_response_v9_to_v10(v)?)
		}
		v9::KvResponseData::KvListResponse(v) => {
			v10::KvResponseData::KvListResponse(convert_kv_list_response_v9_to_v10(v)?)
		}
		v9::KvResponseData::KvPutResponse => v10::KvResponseData::KvPutResponse,
		v9::KvResponseData::KvDeleteResponse => v10::KvResponseData::KvDeleteResponse,
		v9::KvResponseData::KvDropResponse => v10::KvResponseData::KvDropResponse,
	})
}

pub fn convert_sqlite_dirty_page_v9_to_v10(x: v9::SqliteDirtyPage) -> Result<v10::SqliteDirtyPage> {
	Ok(v10::SqliteDirtyPage {
		pgno: x.pgno,
		bytes: x.bytes,
	})
}

pub fn convert_sqlite_fetched_page_v9_to_v10(
	x: v9::SqliteFetchedPage,
) -> Result<v10::SqliteFetchedPage> {
	Ok(v10::SqliteFetchedPage {
		pgno: x.pgno,
		bytes: x.bytes,
	})
}

pub fn convert_sqlite_get_pages_request_v9_to_v10(
	x: v9::SqliteGetPagesRequest,
) -> Result<v10::SqliteGetPagesRequest> {
	Ok(v10::SqliteGetPagesRequest {
		actor_id: x.actor_id,
		pgnos: x.pgnos,
		expected_generation: x.expected_generation,
		expected_head_txid: x.expected_head_txid,
	})
}

pub fn convert_sqlite_get_pages_ok_v9_to_v10(
	x: v9::SqliteGetPagesOk,
) -> Result<v10::SqliteGetPagesOk> {
	Ok(v10::SqliteGetPagesOk {
		pages: x
			.pages
			.into_iter()
			.map(|v| convert_sqlite_fetched_page_v9_to_v10(v))
			.collect::<Result<Vec<_>>>()?,
		head_txid: x.head_txid,
	})
}

pub fn convert_sqlite_error_response_v9_to_v10(
	x: v9::SqliteErrorResponse,
) -> Result<v10::SqliteErrorResponse> {
	Ok(v10::SqliteErrorResponse {
		group: x.group,
		code: x.code,
		message: x.message,
	})
}

pub fn convert_sqlite_get_pages_response_v9_to_v10(
	x: v9::SqliteGetPagesResponse,
) -> Result<v10::SqliteGetPagesResponse> {
	Ok(match x {
		v9::SqliteGetPagesResponse::SqliteGetPagesOk(v) => {
			v10::SqliteGetPagesResponse::SqliteGetPagesOk(convert_sqlite_get_pages_ok_v9_to_v10(v)?)
		}
		v9::SqliteGetPagesResponse::SqliteErrorResponse(v) => {
			v10::SqliteGetPagesResponse::SqliteErrorResponse(
				convert_sqlite_error_response_v9_to_v10(v)?,
			)
		}
	})
}

pub fn convert_sqlite_commit_request_v9_to_v10(
	x: v9::SqliteCommitRequest,
) -> Result<v10::SqliteCommitRequest> {
	Ok(v10::SqliteCommitRequest {
		actor_id: x.actor_id,
		dirty_pages: x
			.dirty_pages
			.into_iter()
			.map(|v| convert_sqlite_dirty_page_v9_to_v10(v))
			.collect::<Result<Vec<_>>>()?,
		db_size_pages: x.db_size_pages,
		now_ms: x.now_ms,
		expected_generation: x.expected_generation,
		expected_head_txid: x.expected_head_txid,
	})
}

pub fn convert_sqlite_commit_ok_v9_to_v10(x: v9::SqliteCommitOk) -> Result<v10::SqliteCommitOk> {
	Ok(v10::SqliteCommitOk {
		head_txid: x.head_txid,
	})
}

pub fn convert_sqlite_commit_response_v9_to_v10(
	x: v9::SqliteCommitResponse,
) -> Result<v10::SqliteCommitResponse> {
	Ok(match x {
		v9::SqliteCommitResponse::SqliteCommitOk(v) => {
			v10::SqliteCommitResponse::SqliteCommitOk(convert_sqlite_commit_ok_v9_to_v10(v)?)
		}
		v9::SqliteCommitResponse::SqliteErrorResponse(v) => {
			v10::SqliteCommitResponse::SqliteErrorResponse(convert_sqlite_error_response_v9_to_v10(
				v,
			)?)
		}
	})
}

pub fn convert_sqlite_commit_stage_begin_request_v9_to_v10(
	x: v9::SqliteCommitStageBeginRequest,
) -> Result<v10::SqliteCommitStageBeginRequest> {
	Ok(v10::SqliteCommitStageBeginRequest {
		actor_id: x.actor_id,
		expected_generation: x.expected_generation,
		expected_head_txid: x.expected_head_txid,
	})
}

pub fn convert_sqlite_commit_stage_begin_ok_v9_to_v10(
	x: v9::SqliteCommitStageBeginOk,
) -> Result<v10::SqliteCommitStageBeginOk> {
	Ok(v10::SqliteCommitStageBeginOk { txid: x.txid })
}

pub fn convert_sqlite_commit_stage_begin_response_v9_to_v10(
	x: v9::SqliteCommitStageBeginResponse,
) -> Result<v10::SqliteCommitStageBeginResponse> {
	Ok(match x {
		v9::SqliteCommitStageBeginResponse::SqliteCommitStageBeginOk(v) => {
			v10::SqliteCommitStageBeginResponse::SqliteCommitStageBeginOk(
				convert_sqlite_commit_stage_begin_ok_v9_to_v10(v)?,
			)
		}
		v9::SqliteCommitStageBeginResponse::SqliteErrorResponse(v) => {
			v10::SqliteCommitStageBeginResponse::SqliteErrorResponse(
				convert_sqlite_error_response_v9_to_v10(v)?,
			)
		}
	})
}

pub fn convert_sqlite_commit_stage_segment_request_v9_to_v10(
	x: v9::SqliteCommitStageSegmentRequest,
) -> Result<v10::SqliteCommitStageSegmentRequest> {
	Ok(v10::SqliteCommitStageSegmentRequest {
		actor_id: x.actor_id,
		expected_generation: x.expected_generation,
		txid: x.txid,
		first_pgno: x.first_pgno,
		dirty_pages: x
			.dirty_pages
			.into_iter()
			.map(|v| convert_sqlite_dirty_page_v9_to_v10(v))
			.collect::<Result<Vec<_>>>()?,
	})
}

pub fn convert_sqlite_commit_stage_segment_ok_v9_to_v10(
	x: v9::SqliteCommitStageSegmentOk,
) -> Result<v10::SqliteCommitStageSegmentOk> {
	Ok(v10::SqliteCommitStageSegmentOk {
		staged_bytes: x.staged_bytes,
	})
}

pub fn convert_sqlite_commit_stage_segment_response_v9_to_v10(
	x: v9::SqliteCommitStageSegmentResponse,
) -> Result<v10::SqliteCommitStageSegmentResponse> {
	Ok(match x {
		v9::SqliteCommitStageSegmentResponse::SqliteCommitStageSegmentOk(v) => {
			v10::SqliteCommitStageSegmentResponse::SqliteCommitStageSegmentOk(
				convert_sqlite_commit_stage_segment_ok_v9_to_v10(v)?,
			)
		}
		v9::SqliteCommitStageSegmentResponse::SqliteErrorResponse(v) => {
			v10::SqliteCommitStageSegmentResponse::SqliteErrorResponse(
				convert_sqlite_error_response_v9_to_v10(v)?,
			)
		}
	})
}

pub fn convert_sqlite_commit_finalize_request_v9_to_v10(
	x: v9::SqliteCommitFinalizeRequest,
) -> Result<v10::SqliteCommitFinalizeRequest> {
	Ok(v10::SqliteCommitFinalizeRequest {
		actor_id: x.actor_id,
		expected_generation: x.expected_generation,
		txid: x.txid,
		new_db_size_pages: x.new_db_size_pages,
		now_ms: x.now_ms,
		segment_first_pgnos: x.segment_first_pgnos,
	})
}

pub fn convert_sqlite_commit_finalize_ok_v9_to_v10(
	x: v9::SqliteCommitFinalizeOk,
) -> Result<v10::SqliteCommitFinalizeOk> {
	Ok(v10::SqliteCommitFinalizeOk {
		head_txid: x.head_txid,
	})
}

pub fn convert_sqlite_commit_finalize_response_v9_to_v10(
	x: v9::SqliteCommitFinalizeResponse,
) -> Result<v10::SqliteCommitFinalizeResponse> {
	Ok(match x {
		v9::SqliteCommitFinalizeResponse::SqliteCommitFinalizeOk(v) => {
			v10::SqliteCommitFinalizeResponse::SqliteCommitFinalizeOk(
				convert_sqlite_commit_finalize_ok_v9_to_v10(v)?,
			)
		}
		v9::SqliteCommitFinalizeResponse::SqliteErrorResponse(v) => {
			v10::SqliteCommitFinalizeResponse::SqliteErrorResponse(
				convert_sqlite_error_response_v9_to_v10(v)?,
			)
		}
	})
}

pub fn convert_sqlite_value_integer_v9_to_v10(
	x: v9::SqliteValueInteger,
) -> Result<v10::SqliteValueInteger> {
	Ok(v10::SqliteValueInteger { value: x.value })
}

pub fn convert_sqlite_value_float_v9_to_v10(
	x: v9::SqliteValueFloat,
) -> Result<v10::SqliteValueFloat> {
	Ok(v10::SqliteValueFloat { value: x.value })
}

pub fn convert_sqlite_value_text_v9_to_v10(x: v9::SqliteValueText) -> Result<v10::SqliteValueText> {
	Ok(v10::SqliteValueText { value: x.value })
}

pub fn convert_sqlite_value_blob_v9_to_v10(x: v9::SqliteValueBlob) -> Result<v10::SqliteValueBlob> {
	Ok(v10::SqliteValueBlob { value: x.value })
}

pub fn convert_sqlite_bind_param_v9_to_v10(x: v9::SqliteBindParam) -> Result<v10::SqliteBindParam> {
	Ok(match x {
		v9::SqliteBindParam::SqliteValueNull => v10::SqliteBindParam::SqliteValueNull,
		v9::SqliteBindParam::SqliteValueInteger(v) => {
			v10::SqliteBindParam::SqliteValueInteger(convert_sqlite_value_integer_v9_to_v10(v)?)
		}
		v9::SqliteBindParam::SqliteValueFloat(v) => {
			v10::SqliteBindParam::SqliteValueFloat(convert_sqlite_value_float_v9_to_v10(v)?)
		}
		v9::SqliteBindParam::SqliteValueText(v) => {
			v10::SqliteBindParam::SqliteValueText(convert_sqlite_value_text_v9_to_v10(v)?)
		}
		v9::SqliteBindParam::SqliteValueBlob(v) => {
			v10::SqliteBindParam::SqliteValueBlob(convert_sqlite_value_blob_v9_to_v10(v)?)
		}
	})
}

pub fn convert_sqlite_column_value_v9_to_v10(
	x: v9::SqliteColumnValue,
) -> Result<v10::SqliteColumnValue> {
	Ok(match x {
		v9::SqliteColumnValue::SqliteValueNull => v10::SqliteColumnValue::SqliteValueNull,
		v9::SqliteColumnValue::SqliteValueInteger(v) => {
			v10::SqliteColumnValue::SqliteValueInteger(convert_sqlite_value_integer_v9_to_v10(v)?)
		}
		v9::SqliteColumnValue::SqliteValueFloat(v) => {
			v10::SqliteColumnValue::SqliteValueFloat(convert_sqlite_value_float_v9_to_v10(v)?)
		}
		v9::SqliteColumnValue::SqliteValueText(v) => {
			v10::SqliteColumnValue::SqliteValueText(convert_sqlite_value_text_v9_to_v10(v)?)
		}
		v9::SqliteColumnValue::SqliteValueBlob(v) => {
			v10::SqliteColumnValue::SqliteValueBlob(convert_sqlite_value_blob_v9_to_v10(v)?)
		}
	})
}

pub fn convert_sqlite_query_result_v9_to_v10(
	x: v9::SqliteQueryResult,
) -> Result<v10::SqliteQueryResult> {
	Ok(v10::SqliteQueryResult {
		columns: x.columns,
		rows: x
			.rows
			.into_iter()
			.map(|v| {
				v.into_iter()
					.map(|v| convert_sqlite_column_value_v9_to_v10(v))
					.collect::<Result<Vec<_>>>()
			})
			.collect::<Result<Vec<_>>>()?,
	})
}

pub fn convert_sqlite_execute_result_v9_to_v10(
	x: v9::SqliteExecuteResult,
) -> Result<v10::SqliteExecuteResult> {
	Ok(v10::SqliteExecuteResult {
		columns: x.columns,
		rows: x
			.rows
			.into_iter()
			.map(|v| {
				v.into_iter()
					.map(|v| convert_sqlite_column_value_v9_to_v10(v))
					.collect::<Result<Vec<_>>>()
			})
			.collect::<Result<Vec<_>>>()?,
		changes: x.changes,
		last_insert_row_id: x.last_insert_row_id,
	})
}

pub fn convert_sqlite_exec_request_v9_to_v10(
	x: v9::SqliteExecRequest,
) -> Result<v10::SqliteExecRequest> {
	Ok(v10::SqliteExecRequest {
		namespace_id: x.namespace_id,
		actor_id: x.actor_id,
		generation: x.generation,
		sql: x.sql,
	})
}

pub fn convert_sqlite_execute_request_v9_to_v10(
	x: v9::SqliteExecuteRequest,
) -> Result<v10::SqliteExecuteRequest> {
	Ok(v10::SqliteExecuteRequest {
		namespace_id: x.namespace_id,
		actor_id: x.actor_id,
		generation: x.generation,
		sql: x.sql,
		params: x
			.params
			.map(|v| {
				v.into_iter()
					.map(|v| convert_sqlite_bind_param_v9_to_v10(v))
					.collect::<Result<Vec<_>>>()
			})
			.transpose()?,
	})
}

pub fn convert_sqlite_batch_statement_v9_to_v10(
	x: v9::SqliteBatchStatement,
) -> Result<v10::SqliteBatchStatement> {
	Ok(v10::SqliteBatchStatement {
		sql: x.sql,
		params: x
			.params
			.map(|v| {
				v.into_iter()
					.map(|v| convert_sqlite_bind_param_v9_to_v10(v))
					.collect::<Result<Vec<_>>>()
			})
			.transpose()?,
	})
}

pub fn convert_sqlite_execute_batch_request_v9_to_v10(
	x: v9::SqliteExecuteBatchRequest,
) -> Result<v10::SqliteExecuteBatchRequest> {
	Ok(v10::SqliteExecuteBatchRequest {
		namespace_id: x.namespace_id,
		actor_id: x.actor_id,
		generation: x.generation,
		statements: x
			.statements
			.into_iter()
			.map(|v| convert_sqlite_batch_statement_v9_to_v10(v))
			.collect::<Result<Vec<_>>>()?,
	})
}

pub fn convert_sqlite_exec_ok_v9_to_v10(x: v9::SqliteExecOk) -> Result<v10::SqliteExecOk> {
	Ok(v10::SqliteExecOk {
		result: convert_sqlite_query_result_v9_to_v10(x.result)?,
	})
}

pub fn convert_sqlite_execute_ok_v9_to_v10(x: v9::SqliteExecuteOk) -> Result<v10::SqliteExecuteOk> {
	Ok(v10::SqliteExecuteOk {
		result: convert_sqlite_execute_result_v9_to_v10(x.result)?,
	})
}

pub fn convert_sqlite_execute_batch_ok_v9_to_v10(
	x: v9::SqliteExecuteBatchOk,
) -> Result<v10::SqliteExecuteBatchOk> {
	Ok(v10::SqliteExecuteBatchOk {
		results: x
			.results
			.into_iter()
			.map(|v| convert_sqlite_execute_result_v9_to_v10(v))
			.collect::<Result<Vec<_>>>()?,
	})
}

pub fn convert_sqlite_exec_response_v9_to_v10(
	x: v9::SqliteExecResponse,
) -> Result<v10::SqliteExecResponse> {
	Ok(match x {
		v9::SqliteExecResponse::SqliteExecOk(v) => {
			v10::SqliteExecResponse::SqliteExecOk(convert_sqlite_exec_ok_v9_to_v10(v)?)
		}
		v9::SqliteExecResponse::SqliteErrorResponse(v) => {
			v10::SqliteExecResponse::SqliteErrorResponse(convert_sqlite_error_response_v9_to_v10(
				v,
			)?)
		}
	})
}

pub fn convert_sqlite_execute_response_v9_to_v10(
	x: v9::SqliteExecuteResponse,
) -> Result<v10::SqliteExecuteResponse> {
	Ok(match x {
		v9::SqliteExecuteResponse::SqliteExecuteOk(v) => {
			v10::SqliteExecuteResponse::SqliteExecuteOk(convert_sqlite_execute_ok_v9_to_v10(v)?)
		}
		v9::SqliteExecuteResponse::SqliteErrorResponse(v) => {
			v10::SqliteExecuteResponse::SqliteErrorResponse(
				convert_sqlite_error_response_v9_to_v10(v)?,
			)
		}
	})
}

pub fn convert_sqlite_execute_batch_response_v9_to_v10(
	x: v9::SqliteExecuteBatchResponse,
) -> Result<v10::SqliteExecuteBatchResponse> {
	Ok(match x {
		v9::SqliteExecuteBatchResponse::SqliteExecuteBatchOk(v) => {
			v10::SqliteExecuteBatchResponse::SqliteExecuteBatchOk(
				convert_sqlite_execute_batch_ok_v9_to_v10(v)?,
			)
		}
		v9::SqliteExecuteBatchResponse::SqliteErrorResponse(v) => {
			v10::SqliteExecuteBatchResponse::SqliteErrorResponse(
				convert_sqlite_error_response_v9_to_v10(v)?,
			)
		}
	})
}

pub fn convert_stop_code_v9_to_v10(x: v9::StopCode) -> Result<v10::StopCode> {
	Ok(match x {
		v9::StopCode::Ok => v10::StopCode::Ok,
		v9::StopCode::Error => v10::StopCode::Error,
	})
}

pub fn convert_actor_name_v9_to_v10(x: v9::ActorName) -> Result<v10::ActorName> {
	Ok(v10::ActorName {
		metadata: x.metadata,
	})
}

pub fn convert_actor_config_v9_to_v10(x: v9::ActorConfig) -> Result<v10::ActorConfig> {
	Ok(v10::ActorConfig {
		name: x.name,
		key: x.key,
		create_ts: x.create_ts,
		input: x.input,
	})
}

pub fn convert_actor_checkpoint_v9_to_v10(x: v9::ActorCheckpoint) -> Result<v10::ActorCheckpoint> {
	Ok(v10::ActorCheckpoint {
		actor_id: x.actor_id,
		generation: x.generation,
		index: x.index,
	})
}

pub fn convert_actor_intent_v9_to_v10(x: v9::ActorIntent) -> Result<v10::ActorIntent> {
	Ok(match x {
		v9::ActorIntent::ActorIntentSleep => v10::ActorIntent::ActorIntentSleep,
		v9::ActorIntent::ActorIntentStop => v10::ActorIntent::ActorIntentStop,
	})
}

pub fn convert_actor_state_stopped_v9_to_v10(
	x: v9::ActorStateStopped,
) -> Result<v10::ActorStateStopped> {
	Ok(v10::ActorStateStopped {
		code: convert_stop_code_v9_to_v10(x.code)?,
		message: x.message,
	})
}

pub fn convert_actor_state_v9_to_v10(x: v9::ActorState) -> Result<v10::ActorState> {
	Ok(match x {
		v9::ActorState::ActorStateRunning => v10::ActorState::ActorStateRunning,
		v9::ActorState::ActorStateStopped(v) => {
			v10::ActorState::ActorStateStopped(convert_actor_state_stopped_v9_to_v10(v)?)
		}
	})
}

pub fn convert_event_actor_intent_v9_to_v10(
	x: v9::EventActorIntent,
) -> Result<v10::EventActorIntent> {
	Ok(v10::EventActorIntent {
		intent: convert_actor_intent_v9_to_v10(x.intent)?,
	})
}

pub fn convert_event_actor_state_update_v9_to_v10(
	x: v9::EventActorStateUpdate,
) -> Result<v10::EventActorStateUpdate> {
	Ok(v10::EventActorStateUpdate {
		state: convert_actor_state_v9_to_v10(x.state)?,
	})
}

pub fn convert_event_actor_set_alarm_v9_to_v10(
	x: v9::EventActorSetAlarm,
) -> Result<v10::EventActorSetAlarm> {
	Ok(v10::EventActorSetAlarm {
		alarm_ts: x.alarm_ts,
	})
}

pub fn convert_event_v9_to_v10(x: v9::Event) -> Result<v10::Event> {
	Ok(match x {
		v9::Event::EventActorIntent(v) => {
			v10::Event::EventActorIntent(convert_event_actor_intent_v9_to_v10(v)?)
		}
		v9::Event::EventActorStateUpdate(v) => {
			v10::Event::EventActorStateUpdate(convert_event_actor_state_update_v9_to_v10(v)?)
		}
		v9::Event::EventActorSetAlarm(v) => {
			v10::Event::EventActorSetAlarm(convert_event_actor_set_alarm_v9_to_v10(v)?)
		}
	})
}

pub fn convert_event_wrapper_v9_to_v10(x: v9::EventWrapper) -> Result<v10::EventWrapper> {
	Ok(v10::EventWrapper {
		checkpoint: convert_actor_checkpoint_v9_to_v10(x.checkpoint)?,
		inner: convert_event_v9_to_v10(x.inner)?,
	})
}

pub fn convert_preloaded_kv_entry_v9_to_v10(
	x: v9::PreloadedKvEntry,
) -> Result<v10::PreloadedKvEntry> {
	Ok(v10::PreloadedKvEntry {
		key: x.key,
		value: x.value,
		metadata: convert_kv_metadata_v9_to_v10(x.metadata)?,
	})
}

pub fn convert_preloaded_kv_v9_to_v10(x: v9::PreloadedKv) -> Result<v10::PreloadedKv> {
	Ok(v10::PreloadedKv {
		entries: x
			.entries
			.into_iter()
			.map(|v| convert_preloaded_kv_entry_v9_to_v10(v))
			.collect::<Result<Vec<_>>>()?,
		requested_get_keys: x.requested_get_keys,
		requested_prefixes: x.requested_prefixes,
	})
}

pub fn convert_hibernating_request_v9_to_v10(
	x: v9::HibernatingRequest,
) -> Result<v10::HibernatingRequest> {
	Ok(v10::HibernatingRequest {
		gateway_id: x.gateway_id,
		request_id: x.request_id,
	})
}

pub fn convert_actor_fork_link_v9_to_v10(x: v9::ActorForkLink) -> Result<v10::ActorForkLink> {
	Ok(v10::ActorForkLink {
		actor_id: x.actor_id,
		key: x.key,
		forked_at: x.forked_at,
	})
}

pub fn convert_actor_fork_v9_to_v10(x: v9::ActorFork) -> Result<v10::ActorFork> {
	Ok(v10::ActorFork {
		fork_id: x.fork_id,
		actor_id: x.actor_id,
		key: x.key,
		forked_at: x.forked_at,
		chain: x
			.chain
			.into_iter()
			.map(|v| convert_actor_fork_link_v9_to_v10(v))
			.collect::<Result<Vec<_>>>()?,
	})
}

pub fn convert_command_start_actor_v9_to_v10(
	x: v9::CommandStartActor,
) -> Result<v10::CommandStartActor> {
	Ok(v10::CommandStartActor {
		config: convert_actor_config_v9_to_v10(x.config)?,
		hibernating_requests: x
			.hibernating_requests
			.into_iter()
			.map(|v| convert_hibernating_request_v9_to_v10(v))
			.collect::<Result<Vec<_>>>()?,
		preloaded_kv: x
			.preloaded_kv
			.map(|v| convert_preloaded_kv_v9_to_v10(v))
			.transpose()?,
		fork: x
			.fork
			.map(|v| convert_actor_fork_v9_to_v10(v))
			.transpose()?,
	})
}

pub fn convert_stop_actor_reason_v9_to_v10(x: v9::StopActorReason) -> Result<v10::StopActorReason> {
	Ok(match x {
		v9::StopActorReason::SleepIntent => v10::StopActorReason::SleepIntent,
		v9::StopActorReason::StopIntent => v10::StopActorReason::StopIntent,
		v9::StopActorReason::Destroy => v10::StopActorReason::Destroy,
		v9::StopActorReason::GoingAway => v10::StopActorReason::GoingAway,
		v9::StopActorReason::Lost => v10::StopActorReason::Lost,
	})
}

pub fn convert_command_stop_actor_v9_to_v10(
	x: v9::CommandStopActor,
) -> Result<v10::CommandStopActor> {
	Ok(v10::CommandStopActor {
		reason: convert_stop_actor_reason_v9_to_v10(x.reason)?,
	})
}

pub fn convert_command_v9_to_v10(x: v9::Command) -> Result<v10::Command> {
	Ok(match x {
		v9::Command::CommandStartActor(v) => {
			v10::Command::CommandStartActor(convert_command_start_actor_v9_to_v10(v)?)
		}
		v9::Command::CommandStopActor(v) => {
			v10::Command::CommandStopActor(convert_command_stop_actor_v9_to_v10(v)?)
		}
	})
}

pub fn convert_command_wrapper_v9_to_v10(x: v9::CommandWrapper) -> Result<v10::CommandWrapper> {
	Ok(v10::CommandWrapper {
		checkpoint: convert_actor_checkpoint_v9_to_v10(x.checkpoint)?,
		inner: convert_command_v9_to_v10(x.inner)?,
		group: x.group,
	})
}

pub fn convert_actor_command_key_data_v9_to_v10(
	x: v9::ActorCommandKeyData,
) -> Result<v10::ActorCommandKeyData> {
	Ok(match x {
		v9::ActorCommandKeyData::CommandStartActor(v) => {
			v10::ActorCommandKeyData::CommandStartActor(convert_command_start_actor_v9_to_v10(v)?)
		}
		v9::ActorCommandKeyData::CommandStopActor(v) => {
			v10::ActorCommandKeyData::CommandStopActor(convert_command_stop_actor_v9_to_v10(v)?)
		}
	})
}

pub fn convert_message_id_v9_to_v10(x: v9::MessageId) -> Result<v10::MessageId> {
	Ok(v10::MessageId {
		gateway_id: x.gateway_id,
		request_id: x.request_id,
		message_index: x.message_index,
	})
}

pub fn convert_to_envoy_request_start_v9_to_v10(
	x: v9::ToEnvoyRequestStart,
) -> Result<v10::ToEnvoyRequestStart> {
	Ok(v10::ToEnvoyRequestStart {
		actor_id: x.actor_id,
		actor_generation: x.actor_generation,
		method: x.method,
		path: x.path,
		headers: x.headers,
		body: x.body,
		stream: x.stream,
		response_stream: x.response_stream,
		group: x.group,
	})
}

pub fn convert_to_envoy_request_chunk_v9_to_v10(
	x: v9::ToEnvoyRequestChunk,
) -> Result<v10::ToEnvoyRequestChunk> {
	Ok(v10::ToEnvoyRequestChunk {
		body: x.body,
		finish: x.finish,
	})
}

pub fn convert_to_rivet_request_body_window_update_v9_to_v10(
	x: v9::ToRivetRequestBodyWindowUpdate,
) -> Result<v10::ToRivetRequestBodyWindowUpdate> {
	Ok(v10::ToRivetRequestBodyWindowUpdate {
		consumed_bytes: x.consumed_bytes,
	})
}

pub fn convert_http_stream_abort_reason_kind_v9_to_v10(
	x: v9::HttpStreamAbortReasonKind,
) -> Result<v10::HttpStreamAbortReasonKind> {
	Ok(match x {
		v9::HttpStreamAbortReasonKind::Unknown => v10::HttpStreamAbortReasonKind::Unknown,
		v9::HttpStreamAbortReasonKind::Cancelled => v10::HttpStreamAbortReasonKind::Cancelled,
		v9::HttpStreamAbortReasonKind::HandlerError => v10::HttpStreamAbortReasonKind::HandlerError,
		v9::HttpStreamAbortReasonKind::InternalError => {
			v10::HttpStreamAbortReasonKind::InternalError
		}
	})
}

pub fn convert_http_stream_abort_reason_v9_to_v10(
	x: v9::HttpStreamAbortReason,
) -> Result<v10::HttpStreamAbortReason> {
	Ok(v10::HttpStreamAbortReason {
		kind: convert_http_stream_abort_reason_kind_v9_to_v10(x.kind)?,
		detail: x.detail,
	})
}

pub fn convert_to_envoy_request_abort_v9_to_v10(
	x: v9::ToEnvoyRequestAbort,
) -> Result<v10::ToEnvoyRequestAbort> {
	Ok(v10::ToEnvoyRequestAbort {
		actor_id: x.actor_id,
		actor_generation: x.actor_generation,
		reason: convert_http_stream_abort_reason_v9_to_v10(x.reason)?,
	})
}

pub fn convert_to_rivet_response_start_v9_to_v10(
	x: v9::ToRivetResponseStart,
) -> Result<v10::ToRivetResponseStart> {
	Ok(v10::ToRivetResponseStart {
		status: x.status,
		headers: x.headers,
		body: x.body,
		stream: x.stream,
	})
}

pub fn convert_to_rivet_response_chunk_v9_to_v10(
	x: v9::ToRivetResponseChunk,
) -> Result<v10::ToRivetResponseChunk> {
	Ok(v10::ToRivetResponseChunk {
		body: x.body,
		finish: x.finish,
	})
}

pub fn convert_to_envoy_response_body_window_update_v9_to_v10(
	x: v9::ToEnvoyResponseBodyWindowUpdate,
) -> Result<v10::ToEnvoyResponseBodyWindowUpdate> {
	Ok(v10::ToEnvoyResponseBodyWindowUpdate {
		consumed_bytes: x.consumed_bytes,
	})
}

pub fn convert_to_rivet_response_abort_v9_to_v10(
	x: v9::ToRivetResponseAbort,
) -> Result<v10::ToRivetResponseAbort> {
	Ok(v10::ToRivetResponseAbort {
		reason: convert_http_stream_abort_reason_v9_to_v10(x.reason)?,
	})
}

pub fn convert_to_envoy_web_socket_open_v9_to_v10(
	x: v9::ToEnvoyWebSocketOpen,
) -> Result<v10::ToEnvoyWebSocketOpen> {
	Ok(v10::ToEnvoyWebSocketOpen {
		actor_id: x.actor_id,
		actor_generation: x.actor_generation,
		path: x.path,
		headers: x.headers,
		group: x.group,
	})
}

pub fn convert_to_envoy_web_socket_message_v9_to_v10(
	x: v9::ToEnvoyWebSocketMessage,
) -> Result<v10::ToEnvoyWebSocketMessage> {
	Ok(v10::ToEnvoyWebSocketMessage {
		data: x.data,
		binary: x.binary,
	})
}

pub fn convert_to_envoy_web_socket_close_v9_to_v10(
	x: v9::ToEnvoyWebSocketClose,
) -> Result<v10::ToEnvoyWebSocketClose> {
	Ok(v10::ToEnvoyWebSocketClose {
		code: x.code,
		reason: x.reason,
	})
}

pub fn convert_to_rivet_web_socket_open_v9_to_v10(
	x: v9::ToRivetWebSocketOpen,
) -> Result<v10::ToRivetWebSocketOpen> {
	Ok(v10::ToRivetWebSocketOpen {
		can_hibernate: x.can_hibernate,
	})
}

pub fn convert_to_rivet_web_socket_message_v9_to_v10(
	x: v9::ToRivetWebSocketMessage,
) -> Result<v10::ToRivetWebSocketMessage> {
	Ok(v10::ToRivetWebSocketMessage {
		data: x.data,
		binary: x.binary,
	})
}

pub fn convert_to_rivet_web_socket_message_ack_v9_to_v10(
	x: v9::ToRivetWebSocketMessageAck,
) -> Result<v10::ToRivetWebSocketMessageAck> {
	Ok(v10::ToRivetWebSocketMessageAck { index: x.index })
}

pub fn convert_to_rivet_web_socket_close_v9_to_v10(
	x: v9::ToRivetWebSocketClose,
) -> Result<v10::ToRivetWebSocketClose> {
	Ok(v10::ToRivetWebSocketClose {
		code: x.code,
		reason: x.reason,
		hibernate: x.hibernate,
	})
}

pub fn convert_to_rivet_tunnel_message_kind_v9_to_v10(
	x: v9::ToRivetTunnelMessageKind,
) -> Result<v10::ToRivetTunnelMessageKind> {
	Ok(match x {
		v9::ToRivetTunnelMessageKind::ToRivetResponseStart(v) => {
			v10::ToRivetTunnelMessageKind::ToRivetResponseStart(
				convert_to_rivet_response_start_v9_to_v10(v)?,
			)
		}
		v9::ToRivetTunnelMessageKind::ToRivetResponseChunk(v) => {
			v10::ToRivetTunnelMessageKind::ToRivetResponseChunk(
				convert_to_rivet_response_chunk_v9_to_v10(v)?,
			)
		}
		v9::ToRivetTunnelMessageKind::ToRivetResponseAbort(v) => {
			v10::ToRivetTunnelMessageKind::ToRivetResponseAbort(
				convert_to_rivet_response_abort_v9_to_v10(v)?,
			)
		}
		v9::ToRivetTunnelMessageKind::ToRivetRequestBodyWindowUpdate(v) => {
			v10::ToRivetTunnelMessageKind::ToRivetRequestBodyWindowUpdate(
				convert_to_rivet_request_body_window_update_v9_to_v10(v)?,
			)
		}
		v9::ToRivetTunnelMessageKind::ToRivetRequestBodyCancel => {
			v10::ToRivetTunnelMessageKind::ToRivetRequestBodyCancel
		}
		v9::ToRivetTunnelMessageKind::ToRivetWebSocketOpen(v) => {
			v10::ToRivetTunnelMessageKind::ToRivetWebSocketOpen(
				convert_to_rivet_web_socket_open_v9_to_v10(v)?,
			)
		}
		v9::ToRivetTunnelMessageKind::ToRivetWebSocketMessage(v) => {
			v10::ToRivetTunnelMessageKind::ToRivetWebSocketMessage(
				convert_to_rivet_web_socket_message_v9_to_v10(v)?,
			)
		}
		v9::ToRivetTunnelMessageKind::ToRivetWebSocketMessageAck(v) => {
			v10::ToRivetTunnelMessageKind::ToRivetWebSocketMessageAck(
				convert_to_rivet_web_socket_message_ack_v9_to_v10(v)?,
			)
		}
		v9::ToRivetTunnelMessageKind::ToRivetWebSocketClose(v) => {
			v10::ToRivetTunnelMessageKind::ToRivetWebSocketClose(
				convert_to_rivet_web_socket_close_v9_to_v10(v)?,
			)
		}
	})
}

pub fn convert_to_rivet_tunnel_message_v9_to_v10(
	x: v9::ToRivetTunnelMessage,
) -> Result<v10::ToRivetTunnelMessage> {
	Ok(v10::ToRivetTunnelMessage {
		message_id: convert_message_id_v9_to_v10(x.message_id)?,
		message_kind: convert_to_rivet_tunnel_message_kind_v9_to_v10(x.message_kind)?,
	})
}

pub fn convert_to_envoy_tunnel_message_kind_v9_to_v10(
	x: v9::ToEnvoyTunnelMessageKind,
) -> Result<v10::ToEnvoyTunnelMessageKind> {
	Ok(match x {
		v9::ToEnvoyTunnelMessageKind::ToEnvoyRequestStart(v) => {
			v10::ToEnvoyTunnelMessageKind::ToEnvoyRequestStart(
				convert_to_envoy_request_start_v9_to_v10(v)?,
			)
		}
		v9::ToEnvoyTunnelMessageKind::ToEnvoyRequestChunk(v) => {
			v10::ToEnvoyTunnelMessageKind::ToEnvoyRequestChunk(
				convert_to_envoy_request_chunk_v9_to_v10(v)?,
			)
		}
		v9::ToEnvoyTunnelMessageKind::ToEnvoyRequestAbort(v) => {
			v10::ToEnvoyTunnelMessageKind::ToEnvoyRequestAbort(
				convert_to_envoy_request_abort_v9_to_v10(v)?,
			)
		}
		v9::ToEnvoyTunnelMessageKind::ToEnvoyRequestBodyCancel => {
			v10::ToEnvoyTunnelMessageKind::ToEnvoyRequestBodyCancel
		}
		v9::ToEnvoyTunnelMessageKind::ToEnvoyResponseBodyWindowUpdate(v) => {
			v10::ToEnvoyTunnelMessageKind::ToEnvoyResponseBodyWindowUpdate(
				convert_to_envoy_response_body_window_update_v9_to_v10(v)?,
			)
		}
		v9::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketOpen(v) => {
			v10::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketOpen(
				convert_to_envoy_web_socket_open_v9_to_v10(v)?,
			)
		}
		v9::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketMessage(v) => {
			v10::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketMessage(
				convert_to_envoy_web_socket_message_v9_to_v10(v)?,
			)
		}
		v9::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketClose(v) => {
			v10::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketClose(
				convert_to_envoy_web_socket_close_v9_to_v10(v)?,
			)
		}
	})
}

pub fn convert_to_envoy_tunnel_message_v9_to_v10(
	x: v9::ToEnvoyTunnelMessage,
) -> Result<v10::ToEnvoyTunnelMessage> {
	Ok(v10::ToEnvoyTunnelMessage {
		message_id: convert_message_id_v9_to_v10(x.message_id)?,
		message_kind: convert_to_envoy_tunnel_message_kind_v9_to_v10(x.message_kind)?,
	})
}

pub fn convert_to_envoy_ping_v9_to_v10(x: v9::ToEnvoyPing) -> Result<v10::ToEnvoyPing> {
	Ok(v10::ToEnvoyPing { ts: x.ts })
}

pub fn convert_to_rivet_metadata_v9_to_v10(x: v9::ToRivetMetadata) -> Result<v10::ToRivetMetadata> {
	Ok(v10::ToRivetMetadata {
		prepopulate_actor_names: x
			.prepopulate_actor_names
			.map(|v| {
				v.into_iter()
					.map(|(k, v)| -> Result<_> { Ok((k, convert_actor_name_v9_to_v10(v)?)) })
					.collect::<Result<_>>()
			})
			.transpose()?,
		metadata: x.metadata,
	})
}

pub fn convert_to_rivet_events_v9_to_v10(x: v9::ToRivetEvents) -> Result<v10::ToRivetEvents> {
	Ok(x.into_iter()
		.map(|v| convert_event_wrapper_v9_to_v10(v))
		.collect::<Result<Vec<_>>>()?)
}

pub fn convert_to_rivet_ack_commands_v9_to_v10(
	x: v9::ToRivetAckCommands,
) -> Result<v10::ToRivetAckCommands> {
	Ok(v10::ToRivetAckCommands {
		last_command_checkpoints: x
			.last_command_checkpoints
			.into_iter()
			.map(|v| convert_actor_checkpoint_v9_to_v10(v))
			.collect::<Result<Vec<_>>>()?,
	})
}

pub fn convert_to_rivet_pong_v9_to_v10(x: v9::ToRivetPong) -> Result<v10::ToRivetPong> {
	Ok(v10::ToRivetPong { ts: x.ts })
}

pub fn convert_to_rivet_kv_request_v9_to_v10(
	x: v9::ToRivetKvRequest,
) -> Result<v10::ToRivetKvRequest> {
	Ok(v10::ToRivetKvRequest {
		actor_id: x.actor_id,
		request_id: x.request_id,
		data: convert_kv_request_data_v9_to_v10(x.data)?,
	})
}

pub fn convert_to_rivet_sqlite_get_pages_request_v9_to_v10(
	x: v9::ToRivetSqliteGetPagesRequest,
) -> Result<v10::ToRivetSqliteGetPagesRequest> {
	Ok(v10::ToRivetSqliteGetPagesRequest {
		request_id: x.request_id,
		data: convert_sqlite_get_pages_request_v9_to_v10(x.data)?,
	})
}

pub fn convert_to_rivet_sqlite_commit_request_v9_to_v10(
	x: v9::ToRivetSqliteCommitRequest,
) -> Result<v10::ToRivetSqliteCommitRequest> {
	Ok(v10::ToRivetSqliteCommitRequest {
		request_id: x.request_id,
		data: convert_sqlite_commit_request_v9_to_v10(x.data)?,
	})
}

pub fn convert_to_rivet_sqlite_commit_stage_begin_request_v9_to_v10(
	x: v9::ToRivetSqliteCommitStageBeginRequest,
) -> Result<v10::ToRivetSqliteCommitStageBeginRequest> {
	Ok(v10::ToRivetSqliteCommitStageBeginRequest {
		request_id: x.request_id,
		data: convert_sqlite_commit_stage_begin_request_v9_to_v10(x.data)?,
	})
}

pub fn convert_to_rivet_sqlite_commit_stage_segment_request_v9_to_v10(
	x: v9::ToRivetSqliteCommitStageSegmentRequest,
) -> Result<v10::ToRivetSqliteCommitStageSegmentRequest> {
	Ok(v10::ToRivetSqliteCommitStageSegmentRequest {
		request_id: x.request_id,
		data: convert_sqlite_commit_stage_segment_request_v9_to_v10(x.data)?,
	})
}

pub fn convert_to_rivet_sqlite_commit_finalize_request_v9_to_v10(
	x: v9::ToRivetSqliteCommitFinalizeRequest,
) -> Result<v10::ToRivetSqliteCommitFinalizeRequest> {
	Ok(v10::ToRivetSqliteCommitFinalizeRequest {
		request_id: x.request_id,
		data: convert_sqlite_commit_finalize_request_v9_to_v10(x.data)?,
	})
}

pub fn convert_to_rivet_sqlite_exec_request_v9_to_v10(
	x: v9::ToRivetSqliteExecRequest,
) -> Result<v10::ToRivetSqliteExecRequest> {
	Ok(v10::ToRivetSqliteExecRequest {
		request_id: x.request_id,
		data: convert_sqlite_exec_request_v9_to_v10(x.data)?,
	})
}

pub fn convert_to_rivet_sqlite_execute_request_v9_to_v10(
	x: v9::ToRivetSqliteExecuteRequest,
) -> Result<v10::ToRivetSqliteExecuteRequest> {
	Ok(v10::ToRivetSqliteExecuteRequest {
		request_id: x.request_id,
		data: convert_sqlite_execute_request_v9_to_v10(x.data)?,
	})
}

pub fn convert_to_rivet_sqlite_execute_batch_request_v9_to_v10(
	x: v9::ToRivetSqliteExecuteBatchRequest,
) -> Result<v10::ToRivetSqliteExecuteBatchRequest> {
	Ok(v10::ToRivetSqliteExecuteBatchRequest {
		request_id: x.request_id,
		data: convert_sqlite_execute_batch_request_v9_to_v10(x.data)?,
	})
}

pub fn convert_to_rivet_v9_to_v10(x: v9::ToRivet) -> Result<v10::ToRivet> {
	Ok(match x {
		v9::ToRivet::ToRivetMetadata(v) => {
			v10::ToRivet::ToRivetMetadata(convert_to_rivet_metadata_v9_to_v10(v)?)
		}
		v9::ToRivet::ToRivetEvents(v) => {
			v10::ToRivet::ToRivetEvents(convert_to_rivet_events_v9_to_v10(v)?)
		}
		v9::ToRivet::ToRivetAckCommands(v) => {
			v10::ToRivet::ToRivetAckCommands(convert_to_rivet_ack_commands_v9_to_v10(v)?)
		}
		v9::ToRivet::ToRivetStopping => v10::ToRivet::ToRivetStopping,
		v9::ToRivet::ToRivetPong(v) => {
			v10::ToRivet::ToRivetPong(convert_to_rivet_pong_v9_to_v10(v)?)
		}
		v9::ToRivet::ToRivetKvRequest(v) => {
			v10::ToRivet::ToRivetKvRequest(convert_to_rivet_kv_request_v9_to_v10(v)?)
		}
		v9::ToRivet::ToRivetTunnelMessage(v) => {
			v10::ToRivet::ToRivetTunnelMessage(convert_to_rivet_tunnel_message_v9_to_v10(v)?)
		}
		v9::ToRivet::ToRivetSqliteGetPagesRequest(v) => v10::ToRivet::ToRivetSqliteGetPagesRequest(
			convert_to_rivet_sqlite_get_pages_request_v9_to_v10(v)?,
		),
		v9::ToRivet::ToRivetSqliteCommitRequest(v) => v10::ToRivet::ToRivetSqliteCommitRequest(
			convert_to_rivet_sqlite_commit_request_v9_to_v10(v)?,
		),
		v9::ToRivet::ToRivetSqliteCommitStageBeginRequest(v) => {
			v10::ToRivet::ToRivetSqliteCommitStageBeginRequest(
				convert_to_rivet_sqlite_commit_stage_begin_request_v9_to_v10(v)?,
			)
		}
		v9::ToRivet::ToRivetSqliteCommitStageSegmentRequest(v) => {
			v10::ToRivet::ToRivetSqliteCommitStageSegmentRequest(
				convert_to_rivet_sqlite_commit_stage_segment_request_v9_to_v10(v)?,
			)
		}
		v9::ToRivet::ToRivetSqliteCommitFinalizeRequest(v) => {
			v10::ToRivet::ToRivetSqliteCommitFinalizeRequest(
				convert_to_rivet_sqlite_commit_finalize_request_v9_to_v10(v)?,
			)
		}
		v9::ToRivet::ToRivetSqliteExecRequest(v) => v10::ToRivet::ToRivetSqliteExecRequest(
			convert_to_rivet_sqlite_exec_request_v9_to_v10(v)?,
		),
		v9::ToRivet::ToRivetSqliteExecuteRequest(v) => v10::ToRivet::ToRivetSqliteExecuteRequest(
			convert_to_rivet_sqlite_execute_request_v9_to_v10(v)?,
		),
		v9::ToRivet::ToRivetSqliteExecuteBatchRequest(v) => {
			v10::ToRivet::ToRivetSqliteExecuteBatchRequest(
				convert_to_rivet_sqlite_execute_batch_request_v9_to_v10(v)?,
			)
		}
	})
}

pub fn convert_protocol_metadata_v9_to_v10(
	x: v9::ProtocolMetadata,
) -> Result<v10::ProtocolMetadata> {
	Ok(v10::ProtocolMetadata {
		envoy_lost_threshold: x.envoy_lost_threshold,
		actor_stop_threshold: x.actor_stop_threshold,
		max_response_payload_size: x.max_response_payload_size,
		// Nothing upgrades an init: an engine encodes it at the envoy's version, and an engine
		// that speaks only v9 refuses a v10 envoy at connect.
		envoy_drain_period: 0,
	})
}

pub fn convert_to_envoy_init_v9_to_v10(x: v9::ToEnvoyInit) -> Result<v10::ToEnvoyInit> {
	Ok(v10::ToEnvoyInit {
		metadata: convert_protocol_metadata_v9_to_v10(x.metadata)?,
	})
}

pub fn convert_to_envoy_commands_v9_to_v10(x: v9::ToEnvoyCommands) -> Result<v10::ToEnvoyCommands> {
	Ok(x.into_iter()
		.map(|v| convert_command_wrapper_v9_to_v10(v))
		.collect::<Result<Vec<_>>>()?)
}

pub fn convert_to_envoy_ack_events_v9_to_v10(
	x: v9::ToEnvoyAckEvents,
) -> Result<v10::ToEnvoyAckEvents> {
	Ok(v10::ToEnvoyAckEvents {
		last_event_checkpoints: x
			.last_event_checkpoints
			.into_iter()
			.map(|v| convert_actor_checkpoint_v9_to_v10(v))
			.collect::<Result<Vec<_>>>()?,
	})
}

pub fn convert_to_envoy_kv_response_v9_to_v10(
	x: v9::ToEnvoyKvResponse,
) -> Result<v10::ToEnvoyKvResponse> {
	Ok(v10::ToEnvoyKvResponse {
		request_id: x.request_id,
		data: convert_kv_response_data_v9_to_v10(x.data)?,
	})
}

pub fn convert_to_envoy_sqlite_get_pages_response_v9_to_v10(
	x: v9::ToEnvoySqliteGetPagesResponse,
) -> Result<v10::ToEnvoySqliteGetPagesResponse> {
	Ok(v10::ToEnvoySqliteGetPagesResponse {
		request_id: x.request_id,
		data: convert_sqlite_get_pages_response_v9_to_v10(x.data)?,
	})
}

pub fn convert_to_envoy_sqlite_commit_response_v9_to_v10(
	x: v9::ToEnvoySqliteCommitResponse,
) -> Result<v10::ToEnvoySqliteCommitResponse> {
	Ok(v10::ToEnvoySqliteCommitResponse {
		request_id: x.request_id,
		data: convert_sqlite_commit_response_v9_to_v10(x.data)?,
	})
}

pub fn convert_to_envoy_sqlite_commit_stage_begin_response_v9_to_v10(
	x: v9::ToEnvoySqliteCommitStageBeginResponse,
) -> Result<v10::ToEnvoySqliteCommitStageBeginResponse> {
	Ok(v10::ToEnvoySqliteCommitStageBeginResponse {
		request_id: x.request_id,
		data: convert_sqlite_commit_stage_begin_response_v9_to_v10(x.data)?,
	})
}

pub fn convert_to_envoy_sqlite_commit_stage_segment_response_v9_to_v10(
	x: v9::ToEnvoySqliteCommitStageSegmentResponse,
) -> Result<v10::ToEnvoySqliteCommitStageSegmentResponse> {
	Ok(v10::ToEnvoySqliteCommitStageSegmentResponse {
		request_id: x.request_id,
		data: convert_sqlite_commit_stage_segment_response_v9_to_v10(x.data)?,
	})
}

pub fn convert_to_envoy_sqlite_commit_finalize_response_v9_to_v10(
	x: v9::ToEnvoySqliteCommitFinalizeResponse,
) -> Result<v10::ToEnvoySqliteCommitFinalizeResponse> {
	Ok(v10::ToEnvoySqliteCommitFinalizeResponse {
		request_id: x.request_id,
		data: convert_sqlite_commit_finalize_response_v9_to_v10(x.data)?,
	})
}

pub fn convert_to_envoy_sqlite_exec_response_v9_to_v10(
	x: v9::ToEnvoySqliteExecResponse,
) -> Result<v10::ToEnvoySqliteExecResponse> {
	Ok(v10::ToEnvoySqliteExecResponse {
		request_id: x.request_id,
		data: convert_sqlite_exec_response_v9_to_v10(x.data)?,
	})
}

pub fn convert_to_envoy_sqlite_execute_response_v9_to_v10(
	x: v9::ToEnvoySqliteExecuteResponse,
) -> Result<v10::ToEnvoySqliteExecuteResponse> {
	Ok(v10::ToEnvoySqliteExecuteResponse {
		request_id: x.request_id,
		data: convert_sqlite_execute_response_v9_to_v10(x.data)?,
	})
}

pub fn convert_to_envoy_sqlite_execute_batch_response_v9_to_v10(
	x: v9::ToEnvoySqliteExecuteBatchResponse,
) -> Result<v10::ToEnvoySqliteExecuteBatchResponse> {
	Ok(v10::ToEnvoySqliteExecuteBatchResponse {
		request_id: x.request_id,
		data: convert_sqlite_execute_batch_response_v9_to_v10(x.data)?,
	})
}

pub fn convert_to_envoy_sqlite_prove_tail_v9_to_v10(
	x: v9::ToEnvoySqliteProveTail,
) -> Result<v10::ToEnvoySqliteProveTail> {
	Ok(v10::ToEnvoySqliteProveTail {
		actor_id: x.actor_id,
		generation: x.generation,
	})
}

pub fn convert_to_envoy_v9_to_v10(x: v9::ToEnvoy) -> Result<v10::ToEnvoy> {
	Ok(match x {
		v9::ToEnvoy::ToEnvoyInit(v) => {
			v10::ToEnvoy::ToEnvoyInit(convert_to_envoy_init_v9_to_v10(v)?)
		}
		v9::ToEnvoy::ToEnvoyCommands(v) => {
			v10::ToEnvoy::ToEnvoyCommands(convert_to_envoy_commands_v9_to_v10(v)?)
		}
		v9::ToEnvoy::ToEnvoyAckEvents(v) => {
			v10::ToEnvoy::ToEnvoyAckEvents(convert_to_envoy_ack_events_v9_to_v10(v)?)
		}
		v9::ToEnvoy::ToEnvoyKvResponse(v) => {
			v10::ToEnvoy::ToEnvoyKvResponse(convert_to_envoy_kv_response_v9_to_v10(v)?)
		}
		v9::ToEnvoy::ToEnvoyTunnelMessage(v) => {
			v10::ToEnvoy::ToEnvoyTunnelMessage(convert_to_envoy_tunnel_message_v9_to_v10(v)?)
		}
		v9::ToEnvoy::ToEnvoyPing(v) => {
			v10::ToEnvoy::ToEnvoyPing(convert_to_envoy_ping_v9_to_v10(v)?)
		}
		v9::ToEnvoy::ToEnvoySqliteGetPagesResponse(v) => {
			v10::ToEnvoy::ToEnvoySqliteGetPagesResponse(
				convert_to_envoy_sqlite_get_pages_response_v9_to_v10(v)?,
			)
		}
		v9::ToEnvoy::ToEnvoySqliteCommitResponse(v) => v10::ToEnvoy::ToEnvoySqliteCommitResponse(
			convert_to_envoy_sqlite_commit_response_v9_to_v10(v)?,
		),
		v9::ToEnvoy::ToEnvoySqliteCommitStageBeginResponse(v) => {
			v10::ToEnvoy::ToEnvoySqliteCommitStageBeginResponse(
				convert_to_envoy_sqlite_commit_stage_begin_response_v9_to_v10(v)?,
			)
		}
		v9::ToEnvoy::ToEnvoySqliteCommitStageSegmentResponse(v) => {
			v10::ToEnvoy::ToEnvoySqliteCommitStageSegmentResponse(
				convert_to_envoy_sqlite_commit_stage_segment_response_v9_to_v10(v)?,
			)
		}
		v9::ToEnvoy::ToEnvoySqliteCommitFinalizeResponse(v) => {
			v10::ToEnvoy::ToEnvoySqliteCommitFinalizeResponse(
				convert_to_envoy_sqlite_commit_finalize_response_v9_to_v10(v)?,
			)
		}
		v9::ToEnvoy::ToEnvoySqliteExecResponse(v) => v10::ToEnvoy::ToEnvoySqliteExecResponse(
			convert_to_envoy_sqlite_exec_response_v9_to_v10(v)?,
		),
		v9::ToEnvoy::ToEnvoySqliteExecuteResponse(v) => v10::ToEnvoy::ToEnvoySqliteExecuteResponse(
			convert_to_envoy_sqlite_execute_response_v9_to_v10(v)?,
		),
		v9::ToEnvoy::ToEnvoySqliteExecuteBatchResponse(v) => {
			v10::ToEnvoy::ToEnvoySqliteExecuteBatchResponse(
				convert_to_envoy_sqlite_execute_batch_response_v9_to_v10(v)?,
			)
		}
		v9::ToEnvoy::ToEnvoySqliteProveTail(v) => {
			v10::ToEnvoy::ToEnvoySqliteProveTail(convert_to_envoy_sqlite_prove_tail_v9_to_v10(v)?)
		}
	})
}

pub fn convert_to_envoy_conn_ping_v9_to_v10(
	x: v9::ToEnvoyConnPing,
) -> Result<v10::ToEnvoyConnPing> {
	Ok(v10::ToEnvoyConnPing {
		gateway_id: x.gateway_id,
		request_id: x.request_id,
		ts: x.ts,
	})
}

pub fn convert_to_envoy_conn_v9_to_v10(x: v9::ToEnvoyConn) -> Result<v10::ToEnvoyConn> {
	Ok(match x {
		v9::ToEnvoyConn::ToEnvoyConnPing(v) => {
			v10::ToEnvoyConn::ToEnvoyConnPing(convert_to_envoy_conn_ping_v9_to_v10(v)?)
		}
		v9::ToEnvoyConn::ToEnvoyConnClose => v10::ToEnvoyConn::ToEnvoyConnClose,
		v9::ToEnvoyConn::ToEnvoyCommands(v) => {
			v10::ToEnvoyConn::ToEnvoyCommands(convert_to_envoy_commands_v9_to_v10(v)?)
		}
		v9::ToEnvoyConn::ToEnvoyAckEvents(v) => {
			v10::ToEnvoyConn::ToEnvoyAckEvents(convert_to_envoy_ack_events_v9_to_v10(v)?)
		}
		v9::ToEnvoyConn::ToEnvoyTunnelMessage(v) => {
			v10::ToEnvoyConn::ToEnvoyTunnelMessage(convert_to_envoy_tunnel_message_v9_to_v10(v)?)
		}
	})
}

pub fn convert_to_gateway_pong_v9_to_v10(x: v9::ToGatewayPong) -> Result<v10::ToGatewayPong> {
	Ok(v10::ToGatewayPong {
		request_id: x.request_id,
		ts: x.ts,
	})
}

pub fn convert_to_gateway_v9_to_v10(x: v9::ToGateway) -> Result<v10::ToGateway> {
	Ok(match x {
		v9::ToGateway::ToGatewayPong(v) => {
			v10::ToGateway::ToGatewayPong(convert_to_gateway_pong_v9_to_v10(v)?)
		}
		v9::ToGateway::ToRivetTunnelMessage(v) => {
			v10::ToGateway::ToRivetTunnelMessage(convert_to_rivet_tunnel_message_v9_to_v10(v)?)
		}
	})
}

pub fn convert_to_outbound_actor_start_v9_to_v10(
	x: v9::ToOutboundActorStart,
) -> Result<v10::ToOutboundActorStart> {
	Ok(v10::ToOutboundActorStart {
		namespace_id: x.namespace_id,
		pool_name: x.pool_name,
		checkpoint: convert_actor_checkpoint_v9_to_v10(x.checkpoint)?,
		actor_config: convert_actor_config_v9_to_v10(x.actor_config)?,
	})
}

pub fn convert_to_outbound_v9_to_v10(x: v9::ToOutbound) -> Result<v10::ToOutbound> {
	Ok(match x {
		v9::ToOutbound::ToOutboundActorStart(v) => {
			v10::ToOutbound::ToOutboundActorStart(convert_to_outbound_actor_start_v9_to_v10(v)?)
		}
	})
}

pub fn convert_kv_metadata_v10_to_v9(x: v10::KvMetadata) -> Result<v9::KvMetadata> {
	Ok(v9::KvMetadata {
		version: x.version,
		update_ts: x.update_ts,
	})
}

pub fn convert_kv_list_range_query_v10_to_v9(
	x: v10::KvListRangeQuery,
) -> Result<v9::KvListRangeQuery> {
	Ok(v9::KvListRangeQuery {
		start: x.start,
		end: x.end,
		exclusive: x.exclusive,
	})
}

pub fn convert_kv_list_prefix_query_v10_to_v9(
	x: v10::KvListPrefixQuery,
) -> Result<v9::KvListPrefixQuery> {
	Ok(v9::KvListPrefixQuery { key: x.key })
}

pub fn convert_kv_list_query_v10_to_v9(x: v10::KvListQuery) -> Result<v9::KvListQuery> {
	Ok(match x {
		v10::KvListQuery::KvListAllQuery => v9::KvListQuery::KvListAllQuery,
		v10::KvListQuery::KvListRangeQuery(v) => {
			v9::KvListQuery::KvListRangeQuery(convert_kv_list_range_query_v10_to_v9(v)?)
		}
		v10::KvListQuery::KvListPrefixQuery(v) => {
			v9::KvListQuery::KvListPrefixQuery(convert_kv_list_prefix_query_v10_to_v9(v)?)
		}
	})
}

pub fn convert_kv_get_request_v10_to_v9(x: v10::KvGetRequest) -> Result<v9::KvGetRequest> {
	Ok(v9::KvGetRequest { keys: x.keys })
}

pub fn convert_kv_list_request_v10_to_v9(x: v10::KvListRequest) -> Result<v9::KvListRequest> {
	Ok(v9::KvListRequest {
		query: convert_kv_list_query_v10_to_v9(x.query)?,
		reverse: x.reverse,
		limit: x.limit,
	})
}

pub fn convert_kv_put_request_v10_to_v9(x: v10::KvPutRequest) -> Result<v9::KvPutRequest> {
	Ok(v9::KvPutRequest {
		keys: x.keys,
		values: x.values,
	})
}

pub fn convert_kv_delete_request_v10_to_v9(x: v10::KvDeleteRequest) -> Result<v9::KvDeleteRequest> {
	Ok(v9::KvDeleteRequest { keys: x.keys })
}

pub fn convert_kv_delete_range_request_v10_to_v9(
	x: v10::KvDeleteRangeRequest,
) -> Result<v9::KvDeleteRangeRequest> {
	Ok(v9::KvDeleteRangeRequest {
		start: x.start,
		end: x.end,
	})
}

pub fn convert_kv_error_response_v10_to_v9(x: v10::KvErrorResponse) -> Result<v9::KvErrorResponse> {
	Ok(v9::KvErrorResponse { message: x.message })
}

pub fn convert_kv_get_response_v10_to_v9(x: v10::KvGetResponse) -> Result<v9::KvGetResponse> {
	Ok(v9::KvGetResponse {
		keys: x.keys,
		values: x.values,
		metadata: x
			.metadata
			.into_iter()
			.map(|v| convert_kv_metadata_v10_to_v9(v))
			.collect::<Result<Vec<_>>>()?,
	})
}

pub fn convert_kv_list_response_v10_to_v9(x: v10::KvListResponse) -> Result<v9::KvListResponse> {
	Ok(v9::KvListResponse {
		keys: x.keys,
		values: x.values,
		metadata: x
			.metadata
			.into_iter()
			.map(|v| convert_kv_metadata_v10_to_v9(v))
			.collect::<Result<Vec<_>>>()?,
	})
}

pub fn convert_kv_request_data_v10_to_v9(x: v10::KvRequestData) -> Result<v9::KvRequestData> {
	Ok(match x {
		v10::KvRequestData::KvGetRequest(v) => {
			v9::KvRequestData::KvGetRequest(convert_kv_get_request_v10_to_v9(v)?)
		}
		v10::KvRequestData::KvListRequest(v) => {
			v9::KvRequestData::KvListRequest(convert_kv_list_request_v10_to_v9(v)?)
		}
		v10::KvRequestData::KvPutRequest(v) => {
			v9::KvRequestData::KvPutRequest(convert_kv_put_request_v10_to_v9(v)?)
		}
		v10::KvRequestData::KvDeleteRequest(v) => {
			v9::KvRequestData::KvDeleteRequest(convert_kv_delete_request_v10_to_v9(v)?)
		}
		v10::KvRequestData::KvDeleteRangeRequest(v) => {
			v9::KvRequestData::KvDeleteRangeRequest(convert_kv_delete_range_request_v10_to_v9(v)?)
		}
		v10::KvRequestData::KvDropRequest => v9::KvRequestData::KvDropRequest,
	})
}

pub fn convert_kv_response_data_v10_to_v9(x: v10::KvResponseData) -> Result<v9::KvResponseData> {
	Ok(match x {
		v10::KvResponseData::KvErrorResponse(v) => {
			v9::KvResponseData::KvErrorResponse(convert_kv_error_response_v10_to_v9(v)?)
		}
		v10::KvResponseData::KvGetResponse(v) => {
			v9::KvResponseData::KvGetResponse(convert_kv_get_response_v10_to_v9(v)?)
		}
		v10::KvResponseData::KvListResponse(v) => {
			v9::KvResponseData::KvListResponse(convert_kv_list_response_v10_to_v9(v)?)
		}
		v10::KvResponseData::KvPutResponse => v9::KvResponseData::KvPutResponse,
		v10::KvResponseData::KvDeleteResponse => v9::KvResponseData::KvDeleteResponse,
		v10::KvResponseData::KvDropResponse => v9::KvResponseData::KvDropResponse,
	})
}

pub fn convert_sqlite_dirty_page_v10_to_v9(x: v10::SqliteDirtyPage) -> Result<v9::SqliteDirtyPage> {
	Ok(v9::SqliteDirtyPage {
		pgno: x.pgno,
		bytes: x.bytes,
	})
}

pub fn convert_sqlite_fetched_page_v10_to_v9(
	x: v10::SqliteFetchedPage,
) -> Result<v9::SqliteFetchedPage> {
	Ok(v9::SqliteFetchedPage {
		pgno: x.pgno,
		bytes: x.bytes,
	})
}

pub fn convert_sqlite_get_pages_request_v10_to_v9(
	x: v10::SqliteGetPagesRequest,
) -> Result<v9::SqliteGetPagesRequest> {
	Ok(v9::SqliteGetPagesRequest {
		actor_id: x.actor_id,
		pgnos: x.pgnos,
		expected_generation: x.expected_generation,
		expected_head_txid: x.expected_head_txid,
	})
}

pub fn convert_sqlite_get_pages_ok_v10_to_v9(
	x: v10::SqliteGetPagesOk,
) -> Result<v9::SqliteGetPagesOk> {
	Ok(v9::SqliteGetPagesOk {
		pages: x
			.pages
			.into_iter()
			.map(|v| convert_sqlite_fetched_page_v10_to_v9(v))
			.collect::<Result<Vec<_>>>()?,
		head_txid: x.head_txid,
	})
}

pub fn convert_sqlite_error_response_v10_to_v9(
	x: v10::SqliteErrorResponse,
) -> Result<v9::SqliteErrorResponse> {
	Ok(v9::SqliteErrorResponse {
		group: x.group,
		code: x.code,
		message: x.message,
	})
}

pub fn convert_sqlite_get_pages_response_v10_to_v9(
	x: v10::SqliteGetPagesResponse,
) -> Result<v9::SqliteGetPagesResponse> {
	Ok(match x {
		v10::SqliteGetPagesResponse::SqliteGetPagesOk(v) => {
			v9::SqliteGetPagesResponse::SqliteGetPagesOk(convert_sqlite_get_pages_ok_v10_to_v9(v)?)
		}
		v10::SqliteGetPagesResponse::SqliteErrorResponse(v) => {
			v9::SqliteGetPagesResponse::SqliteErrorResponse(
				convert_sqlite_error_response_v10_to_v9(v)?,
			)
		}
	})
}

pub fn convert_sqlite_commit_request_v10_to_v9(
	x: v10::SqliteCommitRequest,
) -> Result<v9::SqliteCommitRequest> {
	Ok(v9::SqliteCommitRequest {
		actor_id: x.actor_id,
		dirty_pages: x
			.dirty_pages
			.into_iter()
			.map(|v| convert_sqlite_dirty_page_v10_to_v9(v))
			.collect::<Result<Vec<_>>>()?,
		db_size_pages: x.db_size_pages,
		now_ms: x.now_ms,
		expected_generation: x.expected_generation,
		expected_head_txid: x.expected_head_txid,
	})
}

pub fn convert_sqlite_commit_ok_v10_to_v9(x: v10::SqliteCommitOk) -> Result<v9::SqliteCommitOk> {
	Ok(v9::SqliteCommitOk {
		head_txid: x.head_txid,
	})
}

pub fn convert_sqlite_commit_response_v10_to_v9(
	x: v10::SqliteCommitResponse,
) -> Result<v9::SqliteCommitResponse> {
	Ok(match x {
		v10::SqliteCommitResponse::SqliteCommitOk(v) => {
			v9::SqliteCommitResponse::SqliteCommitOk(convert_sqlite_commit_ok_v10_to_v9(v)?)
		}
		v10::SqliteCommitResponse::SqliteErrorResponse(v) => {
			v9::SqliteCommitResponse::SqliteErrorResponse(convert_sqlite_error_response_v10_to_v9(
				v,
			)?)
		}
	})
}

pub fn convert_sqlite_commit_stage_begin_request_v10_to_v9(
	x: v10::SqliteCommitStageBeginRequest,
) -> Result<v9::SqliteCommitStageBeginRequest> {
	Ok(v9::SqliteCommitStageBeginRequest {
		actor_id: x.actor_id,
		expected_generation: x.expected_generation,
		expected_head_txid: x.expected_head_txid,
	})
}

pub fn convert_sqlite_commit_stage_begin_ok_v10_to_v9(
	x: v10::SqliteCommitStageBeginOk,
) -> Result<v9::SqliteCommitStageBeginOk> {
	Ok(v9::SqliteCommitStageBeginOk { txid: x.txid })
}

pub fn convert_sqlite_commit_stage_begin_response_v10_to_v9(
	x: v10::SqliteCommitStageBeginResponse,
) -> Result<v9::SqliteCommitStageBeginResponse> {
	Ok(match x {
		v10::SqliteCommitStageBeginResponse::SqliteCommitStageBeginOk(v) => {
			v9::SqliteCommitStageBeginResponse::SqliteCommitStageBeginOk(
				convert_sqlite_commit_stage_begin_ok_v10_to_v9(v)?,
			)
		}
		v10::SqliteCommitStageBeginResponse::SqliteErrorResponse(v) => {
			v9::SqliteCommitStageBeginResponse::SqliteErrorResponse(
				convert_sqlite_error_response_v10_to_v9(v)?,
			)
		}
	})
}

pub fn convert_sqlite_commit_stage_segment_request_v10_to_v9(
	x: v10::SqliteCommitStageSegmentRequest,
) -> Result<v9::SqliteCommitStageSegmentRequest> {
	Ok(v9::SqliteCommitStageSegmentRequest {
		actor_id: x.actor_id,
		expected_generation: x.expected_generation,
		txid: x.txid,
		first_pgno: x.first_pgno,
		dirty_pages: x
			.dirty_pages
			.into_iter()
			.map(|v| convert_sqlite_dirty_page_v10_to_v9(v))
			.collect::<Result<Vec<_>>>()?,
	})
}

pub fn convert_sqlite_commit_stage_segment_ok_v10_to_v9(
	x: v10::SqliteCommitStageSegmentOk,
) -> Result<v9::SqliteCommitStageSegmentOk> {
	Ok(v9::SqliteCommitStageSegmentOk {
		staged_bytes: x.staged_bytes,
	})
}

pub fn convert_sqlite_commit_stage_segment_response_v10_to_v9(
	x: v10::SqliteCommitStageSegmentResponse,
) -> Result<v9::SqliteCommitStageSegmentResponse> {
	Ok(match x {
		v10::SqliteCommitStageSegmentResponse::SqliteCommitStageSegmentOk(v) => {
			v9::SqliteCommitStageSegmentResponse::SqliteCommitStageSegmentOk(
				convert_sqlite_commit_stage_segment_ok_v10_to_v9(v)?,
			)
		}
		v10::SqliteCommitStageSegmentResponse::SqliteErrorResponse(v) => {
			v9::SqliteCommitStageSegmentResponse::SqliteErrorResponse(
				convert_sqlite_error_response_v10_to_v9(v)?,
			)
		}
	})
}

pub fn convert_sqlite_commit_finalize_request_v10_to_v9(
	x: v10::SqliteCommitFinalizeRequest,
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

pub fn convert_sqlite_commit_finalize_ok_v10_to_v9(
	x: v10::SqliteCommitFinalizeOk,
) -> Result<v9::SqliteCommitFinalizeOk> {
	Ok(v9::SqliteCommitFinalizeOk {
		head_txid: x.head_txid,
	})
}

pub fn convert_sqlite_commit_finalize_response_v10_to_v9(
	x: v10::SqliteCommitFinalizeResponse,
) -> Result<v9::SqliteCommitFinalizeResponse> {
	Ok(match x {
		v10::SqliteCommitFinalizeResponse::SqliteCommitFinalizeOk(v) => {
			v9::SqliteCommitFinalizeResponse::SqliteCommitFinalizeOk(
				convert_sqlite_commit_finalize_ok_v10_to_v9(v)?,
			)
		}
		v10::SqliteCommitFinalizeResponse::SqliteErrorResponse(v) => {
			v9::SqliteCommitFinalizeResponse::SqliteErrorResponse(
				convert_sqlite_error_response_v10_to_v9(v)?,
			)
		}
	})
}

pub fn convert_sqlite_value_integer_v10_to_v9(
	x: v10::SqliteValueInteger,
) -> Result<v9::SqliteValueInteger> {
	Ok(v9::SqliteValueInteger { value: x.value })
}

pub fn convert_sqlite_value_float_v10_to_v9(
	x: v10::SqliteValueFloat,
) -> Result<v9::SqliteValueFloat> {
	Ok(v9::SqliteValueFloat { value: x.value })
}

pub fn convert_sqlite_value_text_v10_to_v9(x: v10::SqliteValueText) -> Result<v9::SqliteValueText> {
	Ok(v9::SqliteValueText { value: x.value })
}

pub fn convert_sqlite_value_blob_v10_to_v9(x: v10::SqliteValueBlob) -> Result<v9::SqliteValueBlob> {
	Ok(v9::SqliteValueBlob { value: x.value })
}

pub fn convert_sqlite_bind_param_v10_to_v9(x: v10::SqliteBindParam) -> Result<v9::SqliteBindParam> {
	Ok(match x {
		v10::SqliteBindParam::SqliteValueNull => v9::SqliteBindParam::SqliteValueNull,
		v10::SqliteBindParam::SqliteValueInteger(v) => {
			v9::SqliteBindParam::SqliteValueInteger(convert_sqlite_value_integer_v10_to_v9(v)?)
		}
		v10::SqliteBindParam::SqliteValueFloat(v) => {
			v9::SqliteBindParam::SqliteValueFloat(convert_sqlite_value_float_v10_to_v9(v)?)
		}
		v10::SqliteBindParam::SqliteValueText(v) => {
			v9::SqliteBindParam::SqliteValueText(convert_sqlite_value_text_v10_to_v9(v)?)
		}
		v10::SqliteBindParam::SqliteValueBlob(v) => {
			v9::SqliteBindParam::SqliteValueBlob(convert_sqlite_value_blob_v10_to_v9(v)?)
		}
	})
}

pub fn convert_sqlite_column_value_v10_to_v9(
	x: v10::SqliteColumnValue,
) -> Result<v9::SqliteColumnValue> {
	Ok(match x {
		v10::SqliteColumnValue::SqliteValueNull => v9::SqliteColumnValue::SqliteValueNull,
		v10::SqliteColumnValue::SqliteValueInteger(v) => {
			v9::SqliteColumnValue::SqliteValueInteger(convert_sqlite_value_integer_v10_to_v9(v)?)
		}
		v10::SqliteColumnValue::SqliteValueFloat(v) => {
			v9::SqliteColumnValue::SqliteValueFloat(convert_sqlite_value_float_v10_to_v9(v)?)
		}
		v10::SqliteColumnValue::SqliteValueText(v) => {
			v9::SqliteColumnValue::SqliteValueText(convert_sqlite_value_text_v10_to_v9(v)?)
		}
		v10::SqliteColumnValue::SqliteValueBlob(v) => {
			v9::SqliteColumnValue::SqliteValueBlob(convert_sqlite_value_blob_v10_to_v9(v)?)
		}
	})
}

pub fn convert_sqlite_query_result_v10_to_v9(
	x: v10::SqliteQueryResult,
) -> Result<v9::SqliteQueryResult> {
	Ok(v9::SqliteQueryResult {
		columns: x.columns,
		rows: x
			.rows
			.into_iter()
			.map(|v| {
				v.into_iter()
					.map(|v| convert_sqlite_column_value_v10_to_v9(v))
					.collect::<Result<Vec<_>>>()
			})
			.collect::<Result<Vec<_>>>()?,
	})
}

pub fn convert_sqlite_execute_result_v10_to_v9(
	x: v10::SqliteExecuteResult,
) -> Result<v9::SqliteExecuteResult> {
	Ok(v9::SqliteExecuteResult {
		columns: x.columns,
		rows: x
			.rows
			.into_iter()
			.map(|v| {
				v.into_iter()
					.map(|v| convert_sqlite_column_value_v10_to_v9(v))
					.collect::<Result<Vec<_>>>()
			})
			.collect::<Result<Vec<_>>>()?,
		changes: x.changes,
		last_insert_row_id: x.last_insert_row_id,
	})
}

pub fn convert_sqlite_exec_request_v10_to_v9(
	x: v10::SqliteExecRequest,
) -> Result<v9::SqliteExecRequest> {
	Ok(v9::SqliteExecRequest {
		namespace_id: x.namespace_id,
		actor_id: x.actor_id,
		generation: x.generation,
		sql: x.sql,
	})
}

pub fn convert_sqlite_execute_request_v10_to_v9(
	x: v10::SqliteExecuteRequest,
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
					.map(|v| convert_sqlite_bind_param_v10_to_v9(v))
					.collect::<Result<Vec<_>>>()
			})
			.transpose()?,
	})
}

pub fn convert_sqlite_batch_statement_v10_to_v9(
	x: v10::SqliteBatchStatement,
) -> Result<v9::SqliteBatchStatement> {
	Ok(v9::SqliteBatchStatement {
		sql: x.sql,
		params: x
			.params
			.map(|v| {
				v.into_iter()
					.map(|v| convert_sqlite_bind_param_v10_to_v9(v))
					.collect::<Result<Vec<_>>>()
			})
			.transpose()?,
	})
}

pub fn convert_sqlite_execute_batch_request_v10_to_v9(
	x: v10::SqliteExecuteBatchRequest,
) -> Result<v9::SqliteExecuteBatchRequest> {
	Ok(v9::SqliteExecuteBatchRequest {
		namespace_id: x.namespace_id,
		actor_id: x.actor_id,
		generation: x.generation,
		statements: x
			.statements
			.into_iter()
			.map(|v| convert_sqlite_batch_statement_v10_to_v9(v))
			.collect::<Result<Vec<_>>>()?,
	})
}

pub fn convert_sqlite_exec_ok_v10_to_v9(x: v10::SqliteExecOk) -> Result<v9::SqliteExecOk> {
	Ok(v9::SqliteExecOk {
		result: convert_sqlite_query_result_v10_to_v9(x.result)?,
	})
}

pub fn convert_sqlite_execute_ok_v10_to_v9(x: v10::SqliteExecuteOk) -> Result<v9::SqliteExecuteOk> {
	Ok(v9::SqliteExecuteOk {
		result: convert_sqlite_execute_result_v10_to_v9(x.result)?,
	})
}

pub fn convert_sqlite_execute_batch_ok_v10_to_v9(
	x: v10::SqliteExecuteBatchOk,
) -> Result<v9::SqliteExecuteBatchOk> {
	Ok(v9::SqliteExecuteBatchOk {
		results: x
			.results
			.into_iter()
			.map(|v| convert_sqlite_execute_result_v10_to_v9(v))
			.collect::<Result<Vec<_>>>()?,
	})
}

pub fn convert_sqlite_exec_response_v10_to_v9(
	x: v10::SqliteExecResponse,
) -> Result<v9::SqliteExecResponse> {
	Ok(match x {
		v10::SqliteExecResponse::SqliteExecOk(v) => {
			v9::SqliteExecResponse::SqliteExecOk(convert_sqlite_exec_ok_v10_to_v9(v)?)
		}
		v10::SqliteExecResponse::SqliteErrorResponse(v) => {
			v9::SqliteExecResponse::SqliteErrorResponse(convert_sqlite_error_response_v10_to_v9(v)?)
		}
	})
}

pub fn convert_sqlite_execute_response_v10_to_v9(
	x: v10::SqliteExecuteResponse,
) -> Result<v9::SqliteExecuteResponse> {
	Ok(match x {
		v10::SqliteExecuteResponse::SqliteExecuteOk(v) => {
			v9::SqliteExecuteResponse::SqliteExecuteOk(convert_sqlite_execute_ok_v10_to_v9(v)?)
		}
		v10::SqliteExecuteResponse::SqliteErrorResponse(v) => {
			v9::SqliteExecuteResponse::SqliteErrorResponse(convert_sqlite_error_response_v10_to_v9(
				v,
			)?)
		}
	})
}

pub fn convert_sqlite_execute_batch_response_v10_to_v9(
	x: v10::SqliteExecuteBatchResponse,
) -> Result<v9::SqliteExecuteBatchResponse> {
	Ok(match x {
		v10::SqliteExecuteBatchResponse::SqliteExecuteBatchOk(v) => {
			v9::SqliteExecuteBatchResponse::SqliteExecuteBatchOk(
				convert_sqlite_execute_batch_ok_v10_to_v9(v)?,
			)
		}
		v10::SqliteExecuteBatchResponse::SqliteErrorResponse(v) => {
			v9::SqliteExecuteBatchResponse::SqliteErrorResponse(
				convert_sqlite_error_response_v10_to_v9(v)?,
			)
		}
	})
}

pub fn convert_stop_code_v10_to_v9(x: v10::StopCode) -> Result<v9::StopCode> {
	Ok(match x {
		v10::StopCode::Ok => v9::StopCode::Ok,
		v10::StopCode::Error => v9::StopCode::Error,
	})
}

pub fn convert_actor_name_v10_to_v9(x: v10::ActorName) -> Result<v9::ActorName> {
	Ok(v9::ActorName {
		metadata: x.metadata,
	})
}

pub fn convert_actor_config_v10_to_v9(x: v10::ActorConfig) -> Result<v9::ActorConfig> {
	Ok(v9::ActorConfig {
		name: x.name,
		key: x.key,
		create_ts: x.create_ts,
		input: x.input,
	})
}

pub fn convert_actor_checkpoint_v10_to_v9(x: v10::ActorCheckpoint) -> Result<v9::ActorCheckpoint> {
	Ok(v9::ActorCheckpoint {
		actor_id: x.actor_id,
		generation: x.generation,
		index: x.index,
	})
}

pub fn convert_actor_intent_v10_to_v9(x: v10::ActorIntent) -> Result<v9::ActorIntent> {
	Ok(match x {
		v10::ActorIntent::ActorIntentSleep => v9::ActorIntent::ActorIntentSleep,
		v10::ActorIntent::ActorIntentStop => v9::ActorIntent::ActorIntentStop,
	})
}

pub fn convert_actor_state_stopped_v10_to_v9(
	x: v10::ActorStateStopped,
) -> Result<v9::ActorStateStopped> {
	Ok(v9::ActorStateStopped {
		code: convert_stop_code_v10_to_v9(x.code)?,
		message: x.message,
	})
}

pub fn convert_actor_state_v10_to_v9(x: v10::ActorState) -> Result<v9::ActorState> {
	Ok(match x {
		v10::ActorState::ActorStateRunning => v9::ActorState::ActorStateRunning,
		v10::ActorState::ActorStateStopped(v) => {
			v9::ActorState::ActorStateStopped(convert_actor_state_stopped_v10_to_v9(v)?)
		}
	})
}

pub fn convert_event_actor_intent_v10_to_v9(
	x: v10::EventActorIntent,
) -> Result<v9::EventActorIntent> {
	Ok(v9::EventActorIntent {
		intent: convert_actor_intent_v10_to_v9(x.intent)?,
	})
}

pub fn convert_event_actor_state_update_v10_to_v9(
	x: v10::EventActorStateUpdate,
) -> Result<v9::EventActorStateUpdate> {
	Ok(v9::EventActorStateUpdate {
		state: convert_actor_state_v10_to_v9(x.state)?,
	})
}

pub fn convert_event_actor_set_alarm_v10_to_v9(
	x: v10::EventActorSetAlarm,
) -> Result<v9::EventActorSetAlarm> {
	Ok(v9::EventActorSetAlarm {
		alarm_ts: x.alarm_ts,
	})
}

pub fn convert_event_v10_to_v9(x: v10::Event) -> Result<v9::Event> {
	Ok(match x {
		v10::Event::EventActorIntent(v) => {
			v9::Event::EventActorIntent(convert_event_actor_intent_v10_to_v9(v)?)
		}
		v10::Event::EventActorStateUpdate(v) => {
			v9::Event::EventActorStateUpdate(convert_event_actor_state_update_v10_to_v9(v)?)
		}
		v10::Event::EventActorSetAlarm(v) => {
			v9::Event::EventActorSetAlarm(convert_event_actor_set_alarm_v10_to_v9(v)?)
		}
	})
}

pub fn convert_event_wrapper_v10_to_v9(x: v10::EventWrapper) -> Result<v9::EventWrapper> {
	Ok(v9::EventWrapper {
		checkpoint: convert_actor_checkpoint_v10_to_v9(x.checkpoint)?,
		inner: convert_event_v10_to_v9(x.inner)?,
	})
}

pub fn convert_preloaded_kv_entry_v10_to_v9(
	x: v10::PreloadedKvEntry,
) -> Result<v9::PreloadedKvEntry> {
	Ok(v9::PreloadedKvEntry {
		key: x.key,
		value: x.value,
		metadata: convert_kv_metadata_v10_to_v9(x.metadata)?,
	})
}

pub fn convert_preloaded_kv_v10_to_v9(x: v10::PreloadedKv) -> Result<v9::PreloadedKv> {
	Ok(v9::PreloadedKv {
		entries: x
			.entries
			.into_iter()
			.map(|v| convert_preloaded_kv_entry_v10_to_v9(v))
			.collect::<Result<Vec<_>>>()?,
		requested_get_keys: x.requested_get_keys,
		requested_prefixes: x.requested_prefixes,
	})
}

pub fn convert_hibernating_request_v10_to_v9(
	x: v10::HibernatingRequest,
) -> Result<v9::HibernatingRequest> {
	Ok(v9::HibernatingRequest {
		gateway_id: x.gateway_id,
		request_id: x.request_id,
	})
}

pub fn convert_actor_fork_link_v10_to_v9(x: v10::ActorForkLink) -> Result<v9::ActorForkLink> {
	Ok(v9::ActorForkLink {
		actor_id: x.actor_id,
		key: x.key,
		forked_at: x.forked_at,
	})
}

pub fn convert_actor_fork_v10_to_v9(x: v10::ActorFork) -> Result<v9::ActorFork> {
	Ok(v9::ActorFork {
		fork_id: x.fork_id,
		actor_id: x.actor_id,
		key: x.key,
		forked_at: x.forked_at,
		chain: x
			.chain
			.into_iter()
			.map(|v| convert_actor_fork_link_v10_to_v9(v))
			.collect::<Result<Vec<_>>>()?,
	})
}

pub fn convert_command_start_actor_v10_to_v9(
	x: v10::CommandStartActor,
) -> Result<v9::CommandStartActor> {
	Ok(v9::CommandStartActor {
		config: convert_actor_config_v10_to_v9(x.config)?,
		hibernating_requests: x
			.hibernating_requests
			.into_iter()
			.map(|v| convert_hibernating_request_v10_to_v9(v))
			.collect::<Result<Vec<_>>>()?,
		preloaded_kv: x
			.preloaded_kv
			.map(|v| convert_preloaded_kv_v10_to_v9(v))
			.transpose()?,
		fork: x
			.fork
			.map(|v| convert_actor_fork_v10_to_v9(v))
			.transpose()?,
	})
}

pub fn convert_stop_actor_reason_v10_to_v9(x: v10::StopActorReason) -> Result<v9::StopActorReason> {
	Ok(match x {
		v10::StopActorReason::SleepIntent => v9::StopActorReason::SleepIntent,
		v10::StopActorReason::StopIntent => v9::StopActorReason::StopIntent,
		v10::StopActorReason::Destroy => v9::StopActorReason::Destroy,
		v10::StopActorReason::GoingAway => v9::StopActorReason::GoingAway,
		v10::StopActorReason::Lost => v9::StopActorReason::Lost,
	})
}

pub fn convert_command_stop_actor_v10_to_v9(
	x: v10::CommandStopActor,
) -> Result<v9::CommandStopActor> {
	Ok(v9::CommandStopActor {
		reason: convert_stop_actor_reason_v10_to_v9(x.reason)?,
	})
}

pub fn convert_command_v10_to_v9(x: v10::Command) -> Result<v9::Command> {
	Ok(match x {
		v10::Command::CommandStartActor(v) => {
			v9::Command::CommandStartActor(convert_command_start_actor_v10_to_v9(v)?)
		}
		v10::Command::CommandStopActor(v) => {
			v9::Command::CommandStopActor(convert_command_stop_actor_v10_to_v9(v)?)
		}
	})
}

pub fn convert_command_wrapper_v10_to_v9(x: v10::CommandWrapper) -> Result<v9::CommandWrapper> {
	Ok(v9::CommandWrapper {
		checkpoint: convert_actor_checkpoint_v10_to_v9(x.checkpoint)?,
		inner: convert_command_v10_to_v9(x.inner)?,
		group: x.group,
	})
}

pub fn convert_actor_command_key_data_v10_to_v9(
	x: v10::ActorCommandKeyData,
) -> Result<v9::ActorCommandKeyData> {
	Ok(match x {
		v10::ActorCommandKeyData::CommandStartActor(v) => {
			v9::ActorCommandKeyData::CommandStartActor(convert_command_start_actor_v10_to_v9(v)?)
		}
		v10::ActorCommandKeyData::CommandStopActor(v) => {
			v9::ActorCommandKeyData::CommandStopActor(convert_command_stop_actor_v10_to_v9(v)?)
		}
	})
}

pub fn convert_message_id_v10_to_v9(x: v10::MessageId) -> Result<v9::MessageId> {
	Ok(v9::MessageId {
		gateway_id: x.gateway_id,
		request_id: x.request_id,
		message_index: x.message_index,
	})
}

pub fn convert_to_envoy_request_start_v10_to_v9(
	x: v10::ToEnvoyRequestStart,
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
		group: x.group,
	})
}

pub fn convert_to_envoy_request_chunk_v10_to_v9(
	x: v10::ToEnvoyRequestChunk,
) -> Result<v9::ToEnvoyRequestChunk> {
	Ok(v9::ToEnvoyRequestChunk {
		body: x.body,
		finish: x.finish,
	})
}

pub fn convert_to_rivet_request_body_window_update_v10_to_v9(
	x: v10::ToRivetRequestBodyWindowUpdate,
) -> Result<v9::ToRivetRequestBodyWindowUpdate> {
	Ok(v9::ToRivetRequestBodyWindowUpdate {
		consumed_bytes: x.consumed_bytes,
	})
}

pub fn convert_http_stream_abort_reason_kind_v10_to_v9(
	x: v10::HttpStreamAbortReasonKind,
) -> Result<v9::HttpStreamAbortReasonKind> {
	Ok(match x {
		v10::HttpStreamAbortReasonKind::Unknown => v9::HttpStreamAbortReasonKind::Unknown,
		v10::HttpStreamAbortReasonKind::Cancelled => v9::HttpStreamAbortReasonKind::Cancelled,
		v10::HttpStreamAbortReasonKind::HandlerError => v9::HttpStreamAbortReasonKind::HandlerError,
		v10::HttpStreamAbortReasonKind::InternalError => {
			v9::HttpStreamAbortReasonKind::InternalError
		}
	})
}

pub fn convert_http_stream_abort_reason_v10_to_v9(
	x: v10::HttpStreamAbortReason,
) -> Result<v9::HttpStreamAbortReason> {
	Ok(v9::HttpStreamAbortReason {
		kind: convert_http_stream_abort_reason_kind_v10_to_v9(x.kind)?,
		detail: x.detail,
	})
}

pub fn convert_to_envoy_request_abort_v10_to_v9(
	x: v10::ToEnvoyRequestAbort,
) -> Result<v9::ToEnvoyRequestAbort> {
	Ok(v9::ToEnvoyRequestAbort {
		actor_id: x.actor_id,
		actor_generation: x.actor_generation,
		reason: convert_http_stream_abort_reason_v10_to_v9(x.reason)?,
	})
}

pub fn convert_to_rivet_response_start_v10_to_v9(
	x: v10::ToRivetResponseStart,
) -> Result<v9::ToRivetResponseStart> {
	Ok(v9::ToRivetResponseStart {
		status: x.status,
		headers: x.headers,
		body: x.body,
		stream: x.stream,
	})
}

pub fn convert_to_rivet_response_chunk_v10_to_v9(
	x: v10::ToRivetResponseChunk,
) -> Result<v9::ToRivetResponseChunk> {
	Ok(v9::ToRivetResponseChunk {
		body: x.body,
		finish: x.finish,
	})
}

pub fn convert_to_envoy_response_body_window_update_v10_to_v9(
	x: v10::ToEnvoyResponseBodyWindowUpdate,
) -> Result<v9::ToEnvoyResponseBodyWindowUpdate> {
	Ok(v9::ToEnvoyResponseBodyWindowUpdate {
		consumed_bytes: x.consumed_bytes,
	})
}

pub fn convert_to_rivet_response_abort_v10_to_v9(
	x: v10::ToRivetResponseAbort,
) -> Result<v9::ToRivetResponseAbort> {
	Ok(v9::ToRivetResponseAbort {
		reason: convert_http_stream_abort_reason_v10_to_v9(x.reason)?,
	})
}

pub fn convert_to_envoy_web_socket_open_v10_to_v9(
	x: v10::ToEnvoyWebSocketOpen,
) -> Result<v9::ToEnvoyWebSocketOpen> {
	Ok(v9::ToEnvoyWebSocketOpen {
		actor_id: x.actor_id,
		actor_generation: x.actor_generation,
		path: x.path,
		headers: x.headers,
		group: x.group,
	})
}

pub fn convert_to_envoy_web_socket_message_v10_to_v9(
	x: v10::ToEnvoyWebSocketMessage,
) -> Result<v9::ToEnvoyWebSocketMessage> {
	Ok(v9::ToEnvoyWebSocketMessage {
		data: x.data,
		binary: x.binary,
	})
}

pub fn convert_to_envoy_web_socket_close_v10_to_v9(
	x: v10::ToEnvoyWebSocketClose,
) -> Result<v9::ToEnvoyWebSocketClose> {
	Ok(v9::ToEnvoyWebSocketClose {
		code: x.code,
		reason: x.reason,
	})
}

pub fn convert_to_rivet_web_socket_open_v10_to_v9(
	x: v10::ToRivetWebSocketOpen,
) -> Result<v9::ToRivetWebSocketOpen> {
	Ok(v9::ToRivetWebSocketOpen {
		can_hibernate: x.can_hibernate,
	})
}

pub fn convert_to_rivet_web_socket_message_v10_to_v9(
	x: v10::ToRivetWebSocketMessage,
) -> Result<v9::ToRivetWebSocketMessage> {
	Ok(v9::ToRivetWebSocketMessage {
		data: x.data,
		binary: x.binary,
	})
}

pub fn convert_to_rivet_web_socket_message_ack_v10_to_v9(
	x: v10::ToRivetWebSocketMessageAck,
) -> Result<v9::ToRivetWebSocketMessageAck> {
	Ok(v9::ToRivetWebSocketMessageAck { index: x.index })
}

pub fn convert_to_rivet_web_socket_close_v10_to_v9(
	x: v10::ToRivetWebSocketClose,
) -> Result<v9::ToRivetWebSocketClose> {
	Ok(v9::ToRivetWebSocketClose {
		code: x.code,
		reason: x.reason,
		hibernate: x.hibernate,
	})
}

pub fn convert_to_rivet_tunnel_message_kind_v10_to_v9(
	x: v10::ToRivetTunnelMessageKind,
) -> Result<v9::ToRivetTunnelMessageKind> {
	Ok(match x {
		v10::ToRivetTunnelMessageKind::ToRivetResponseStart(v) => {
			v9::ToRivetTunnelMessageKind::ToRivetResponseStart(
				convert_to_rivet_response_start_v10_to_v9(v)?,
			)
		}
		v10::ToRivetTunnelMessageKind::ToRivetResponseChunk(v) => {
			v9::ToRivetTunnelMessageKind::ToRivetResponseChunk(
				convert_to_rivet_response_chunk_v10_to_v9(v)?,
			)
		}
		v10::ToRivetTunnelMessageKind::ToRivetResponseAbort(v) => {
			v9::ToRivetTunnelMessageKind::ToRivetResponseAbort(
				convert_to_rivet_response_abort_v10_to_v9(v)?,
			)
		}
		v10::ToRivetTunnelMessageKind::ToRivetRequestBodyWindowUpdate(v) => {
			v9::ToRivetTunnelMessageKind::ToRivetRequestBodyWindowUpdate(
				convert_to_rivet_request_body_window_update_v10_to_v9(v)?,
			)
		}
		v10::ToRivetTunnelMessageKind::ToRivetRequestBodyCancel => {
			v9::ToRivetTunnelMessageKind::ToRivetRequestBodyCancel
		}
		v10::ToRivetTunnelMessageKind::ToRivetWebSocketOpen(v) => {
			v9::ToRivetTunnelMessageKind::ToRivetWebSocketOpen(
				convert_to_rivet_web_socket_open_v10_to_v9(v)?,
			)
		}
		v10::ToRivetTunnelMessageKind::ToRivetWebSocketMessage(v) => {
			v9::ToRivetTunnelMessageKind::ToRivetWebSocketMessage(
				convert_to_rivet_web_socket_message_v10_to_v9(v)?,
			)
		}
		v10::ToRivetTunnelMessageKind::ToRivetWebSocketMessageAck(v) => {
			v9::ToRivetTunnelMessageKind::ToRivetWebSocketMessageAck(
				convert_to_rivet_web_socket_message_ack_v10_to_v9(v)?,
			)
		}
		v10::ToRivetTunnelMessageKind::ToRivetWebSocketClose(v) => {
			v9::ToRivetTunnelMessageKind::ToRivetWebSocketClose(
				convert_to_rivet_web_socket_close_v10_to_v9(v)?,
			)
		}
	})
}

pub fn convert_to_rivet_tunnel_message_v10_to_v9(
	x: v10::ToRivetTunnelMessage,
) -> Result<v9::ToRivetTunnelMessage> {
	Ok(v9::ToRivetTunnelMessage {
		message_id: convert_message_id_v10_to_v9(x.message_id)?,
		message_kind: convert_to_rivet_tunnel_message_kind_v10_to_v9(x.message_kind)?,
	})
}

pub fn convert_to_envoy_tunnel_message_kind_v10_to_v9(
	x: v10::ToEnvoyTunnelMessageKind,
) -> Result<v9::ToEnvoyTunnelMessageKind> {
	Ok(match x {
		v10::ToEnvoyTunnelMessageKind::ToEnvoyRequestStart(v) => {
			v9::ToEnvoyTunnelMessageKind::ToEnvoyRequestStart(
				convert_to_envoy_request_start_v10_to_v9(v)?,
			)
		}
		v10::ToEnvoyTunnelMessageKind::ToEnvoyRequestChunk(v) => {
			v9::ToEnvoyTunnelMessageKind::ToEnvoyRequestChunk(
				convert_to_envoy_request_chunk_v10_to_v9(v)?,
			)
		}
		v10::ToEnvoyTunnelMessageKind::ToEnvoyRequestAbort(v) => {
			v9::ToEnvoyTunnelMessageKind::ToEnvoyRequestAbort(
				convert_to_envoy_request_abort_v10_to_v9(v)?,
			)
		}
		v10::ToEnvoyTunnelMessageKind::ToEnvoyRequestBodyCancel => {
			v9::ToEnvoyTunnelMessageKind::ToEnvoyRequestBodyCancel
		}
		v10::ToEnvoyTunnelMessageKind::ToEnvoyResponseBodyWindowUpdate(v) => {
			v9::ToEnvoyTunnelMessageKind::ToEnvoyResponseBodyWindowUpdate(
				convert_to_envoy_response_body_window_update_v10_to_v9(v)?,
			)
		}
		v10::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketOpen(v) => {
			v9::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketOpen(
				convert_to_envoy_web_socket_open_v10_to_v9(v)?,
			)
		}
		v10::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketMessage(v) => {
			v9::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketMessage(
				convert_to_envoy_web_socket_message_v10_to_v9(v)?,
			)
		}
		v10::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketClose(v) => {
			v9::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketClose(
				convert_to_envoy_web_socket_close_v10_to_v9(v)?,
			)
		}
	})
}

pub fn convert_to_envoy_tunnel_message_v10_to_v9(
	x: v10::ToEnvoyTunnelMessage,
) -> Result<v9::ToEnvoyTunnelMessage> {
	Ok(v9::ToEnvoyTunnelMessage {
		message_id: convert_message_id_v10_to_v9(x.message_id)?,
		message_kind: convert_to_envoy_tunnel_message_kind_v10_to_v9(x.message_kind)?,
	})
}

pub fn convert_to_envoy_ping_v10_to_v9(x: v10::ToEnvoyPing) -> Result<v9::ToEnvoyPing> {
	Ok(v9::ToEnvoyPing { ts: x.ts })
}

pub fn convert_to_rivet_metadata_v10_to_v9(x: v10::ToRivetMetadata) -> Result<v9::ToRivetMetadata> {
	Ok(v9::ToRivetMetadata {
		prepopulate_actor_names: x
			.prepopulate_actor_names
			.map(|v| {
				v.into_iter()
					.map(|(k, v)| -> Result<_> { Ok((k, convert_actor_name_v10_to_v9(v)?)) })
					.collect::<Result<_>>()
			})
			.transpose()?,
		metadata: x.metadata,
	})
}

pub fn convert_to_rivet_events_v10_to_v9(x: v10::ToRivetEvents) -> Result<v9::ToRivetEvents> {
	Ok(x.into_iter()
		.map(|v| convert_event_wrapper_v10_to_v9(v))
		.collect::<Result<Vec<_>>>()?)
}

pub fn convert_to_rivet_ack_commands_v10_to_v9(
	x: v10::ToRivetAckCommands,
) -> Result<v9::ToRivetAckCommands> {
	Ok(v9::ToRivetAckCommands {
		last_command_checkpoints: x
			.last_command_checkpoints
			.into_iter()
			.map(|v| convert_actor_checkpoint_v10_to_v9(v))
			.collect::<Result<Vec<_>>>()?,
	})
}

pub fn convert_to_rivet_pong_v10_to_v9(x: v10::ToRivetPong) -> Result<v9::ToRivetPong> {
	Ok(v9::ToRivetPong { ts: x.ts })
}

pub fn convert_to_rivet_kv_request_v10_to_v9(
	x: v10::ToRivetKvRequest,
) -> Result<v9::ToRivetKvRequest> {
	Ok(v9::ToRivetKvRequest {
		actor_id: x.actor_id,
		request_id: x.request_id,
		data: convert_kv_request_data_v10_to_v9(x.data)?,
	})
}

pub fn convert_to_rivet_sqlite_get_pages_request_v10_to_v9(
	x: v10::ToRivetSqliteGetPagesRequest,
) -> Result<v9::ToRivetSqliteGetPagesRequest> {
	Ok(v9::ToRivetSqliteGetPagesRequest {
		request_id: x.request_id,
		data: convert_sqlite_get_pages_request_v10_to_v9(x.data)?,
	})
}

pub fn convert_to_rivet_sqlite_commit_request_v10_to_v9(
	x: v10::ToRivetSqliteCommitRequest,
) -> Result<v9::ToRivetSqliteCommitRequest> {
	Ok(v9::ToRivetSqliteCommitRequest {
		request_id: x.request_id,
		data: convert_sqlite_commit_request_v10_to_v9(x.data)?,
	})
}

pub fn convert_to_rivet_sqlite_commit_stage_begin_request_v10_to_v9(
	x: v10::ToRivetSqliteCommitStageBeginRequest,
) -> Result<v9::ToRivetSqliteCommitStageBeginRequest> {
	Ok(v9::ToRivetSqliteCommitStageBeginRequest {
		request_id: x.request_id,
		data: convert_sqlite_commit_stage_begin_request_v10_to_v9(x.data)?,
	})
}

pub fn convert_to_rivet_sqlite_commit_stage_segment_request_v10_to_v9(
	x: v10::ToRivetSqliteCommitStageSegmentRequest,
) -> Result<v9::ToRivetSqliteCommitStageSegmentRequest> {
	Ok(v9::ToRivetSqliteCommitStageSegmentRequest {
		request_id: x.request_id,
		data: convert_sqlite_commit_stage_segment_request_v10_to_v9(x.data)?,
	})
}

pub fn convert_to_rivet_sqlite_commit_finalize_request_v10_to_v9(
	x: v10::ToRivetSqliteCommitFinalizeRequest,
) -> Result<v9::ToRivetSqliteCommitFinalizeRequest> {
	Ok(v9::ToRivetSqliteCommitFinalizeRequest {
		request_id: x.request_id,
		data: convert_sqlite_commit_finalize_request_v10_to_v9(x.data)?,
	})
}

pub fn convert_to_rivet_sqlite_exec_request_v10_to_v9(
	x: v10::ToRivetSqliteExecRequest,
) -> Result<v9::ToRivetSqliteExecRequest> {
	Ok(v9::ToRivetSqliteExecRequest {
		request_id: x.request_id,
		data: convert_sqlite_exec_request_v10_to_v9(x.data)?,
	})
}

pub fn convert_to_rivet_sqlite_execute_request_v10_to_v9(
	x: v10::ToRivetSqliteExecuteRequest,
) -> Result<v9::ToRivetSqliteExecuteRequest> {
	Ok(v9::ToRivetSqliteExecuteRequest {
		request_id: x.request_id,
		data: convert_sqlite_execute_request_v10_to_v9(x.data)?,
	})
}

pub fn convert_to_rivet_sqlite_execute_batch_request_v10_to_v9(
	x: v10::ToRivetSqliteExecuteBatchRequest,
) -> Result<v9::ToRivetSqliteExecuteBatchRequest> {
	Ok(v9::ToRivetSqliteExecuteBatchRequest {
		request_id: x.request_id,
		data: convert_sqlite_execute_batch_request_v10_to_v9(x.data)?,
	})
}

pub fn convert_to_rivet_v10_to_v9(x: v10::ToRivet) -> Result<v9::ToRivet> {
	Ok(match x {
		v10::ToRivet::ToRivetMetadata(v) => {
			v9::ToRivet::ToRivetMetadata(convert_to_rivet_metadata_v10_to_v9(v)?)
		}
		v10::ToRivet::ToRivetEvents(v) => {
			v9::ToRivet::ToRivetEvents(convert_to_rivet_events_v10_to_v9(v)?)
		}
		v10::ToRivet::ToRivetAckCommands(v) => {
			v9::ToRivet::ToRivetAckCommands(convert_to_rivet_ack_commands_v10_to_v9(v)?)
		}
		v10::ToRivet::ToRivetStopping => v9::ToRivet::ToRivetStopping,
		v10::ToRivet::ToRivetPong(v) => {
			v9::ToRivet::ToRivetPong(convert_to_rivet_pong_v10_to_v9(v)?)
		}
		v10::ToRivet::ToRivetKvRequest(v) => {
			v9::ToRivet::ToRivetKvRequest(convert_to_rivet_kv_request_v10_to_v9(v)?)
		}
		v10::ToRivet::ToRivetTunnelMessage(v) => {
			v9::ToRivet::ToRivetTunnelMessage(convert_to_rivet_tunnel_message_v10_to_v9(v)?)
		}
		v10::ToRivet::ToRivetSqliteGetPagesRequest(v) => v9::ToRivet::ToRivetSqliteGetPagesRequest(
			convert_to_rivet_sqlite_get_pages_request_v10_to_v9(v)?,
		),
		v10::ToRivet::ToRivetSqliteCommitRequest(v) => v9::ToRivet::ToRivetSqliteCommitRequest(
			convert_to_rivet_sqlite_commit_request_v10_to_v9(v)?,
		),
		v10::ToRivet::ToRivetSqliteCommitStageBeginRequest(v) => {
			v9::ToRivet::ToRivetSqliteCommitStageBeginRequest(
				convert_to_rivet_sqlite_commit_stage_begin_request_v10_to_v9(v)?,
			)
		}
		v10::ToRivet::ToRivetSqliteCommitStageSegmentRequest(v) => {
			v9::ToRivet::ToRivetSqliteCommitStageSegmentRequest(
				convert_to_rivet_sqlite_commit_stage_segment_request_v10_to_v9(v)?,
			)
		}
		v10::ToRivet::ToRivetSqliteCommitFinalizeRequest(v) => {
			v9::ToRivet::ToRivetSqliteCommitFinalizeRequest(
				convert_to_rivet_sqlite_commit_finalize_request_v10_to_v9(v)?,
			)
		}
		v10::ToRivet::ToRivetSqliteExecRequest(v) => v9::ToRivet::ToRivetSqliteExecRequest(
			convert_to_rivet_sqlite_exec_request_v10_to_v9(v)?,
		),
		v10::ToRivet::ToRivetSqliteExecuteRequest(v) => v9::ToRivet::ToRivetSqliteExecuteRequest(
			convert_to_rivet_sqlite_execute_request_v10_to_v9(v)?,
		),
		v10::ToRivet::ToRivetSqliteExecuteBatchRequest(v) => {
			v9::ToRivet::ToRivetSqliteExecuteBatchRequest(
				convert_to_rivet_sqlite_execute_batch_request_v10_to_v9(v)?,
			)
		}
	})
}

pub fn convert_protocol_metadata_v10_to_v9(
	x: v10::ProtocolMetadata,
) -> Result<v9::ProtocolMetadata> {
	Ok(v9::ProtocolMetadata {
		envoy_lost_threshold: x.envoy_lost_threshold,
		actor_stop_threshold: x.actor_stop_threshold,
		max_response_payload_size: x.max_response_payload_size,
	})
}

pub fn convert_to_envoy_init_v10_to_v9(x: v10::ToEnvoyInit) -> Result<v9::ToEnvoyInit> {
	Ok(v9::ToEnvoyInit {
		metadata: convert_protocol_metadata_v10_to_v9(x.metadata)?,
	})
}

pub fn convert_to_envoy_commands_v10_to_v9(x: v10::ToEnvoyCommands) -> Result<v9::ToEnvoyCommands> {
	Ok(x.into_iter()
		.map(|v| convert_command_wrapper_v10_to_v9(v))
		.collect::<Result<Vec<_>>>()?)
}

pub fn convert_to_envoy_ack_events_v10_to_v9(
	x: v10::ToEnvoyAckEvents,
) -> Result<v9::ToEnvoyAckEvents> {
	Ok(v9::ToEnvoyAckEvents {
		last_event_checkpoints: x
			.last_event_checkpoints
			.into_iter()
			.map(|v| convert_actor_checkpoint_v10_to_v9(v))
			.collect::<Result<Vec<_>>>()?,
	})
}

pub fn convert_to_envoy_kv_response_v10_to_v9(
	x: v10::ToEnvoyKvResponse,
) -> Result<v9::ToEnvoyKvResponse> {
	Ok(v9::ToEnvoyKvResponse {
		request_id: x.request_id,
		data: convert_kv_response_data_v10_to_v9(x.data)?,
	})
}

pub fn convert_to_envoy_sqlite_get_pages_response_v10_to_v9(
	x: v10::ToEnvoySqliteGetPagesResponse,
) -> Result<v9::ToEnvoySqliteGetPagesResponse> {
	Ok(v9::ToEnvoySqliteGetPagesResponse {
		request_id: x.request_id,
		data: convert_sqlite_get_pages_response_v10_to_v9(x.data)?,
	})
}

pub fn convert_to_envoy_sqlite_commit_response_v10_to_v9(
	x: v10::ToEnvoySqliteCommitResponse,
) -> Result<v9::ToEnvoySqliteCommitResponse> {
	Ok(v9::ToEnvoySqliteCommitResponse {
		request_id: x.request_id,
		data: convert_sqlite_commit_response_v10_to_v9(x.data)?,
	})
}

pub fn convert_to_envoy_sqlite_commit_stage_begin_response_v10_to_v9(
	x: v10::ToEnvoySqliteCommitStageBeginResponse,
) -> Result<v9::ToEnvoySqliteCommitStageBeginResponse> {
	Ok(v9::ToEnvoySqliteCommitStageBeginResponse {
		request_id: x.request_id,
		data: convert_sqlite_commit_stage_begin_response_v10_to_v9(x.data)?,
	})
}

pub fn convert_to_envoy_sqlite_commit_stage_segment_response_v10_to_v9(
	x: v10::ToEnvoySqliteCommitStageSegmentResponse,
) -> Result<v9::ToEnvoySqliteCommitStageSegmentResponse> {
	Ok(v9::ToEnvoySqliteCommitStageSegmentResponse {
		request_id: x.request_id,
		data: convert_sqlite_commit_stage_segment_response_v10_to_v9(x.data)?,
	})
}

pub fn convert_to_envoy_sqlite_commit_finalize_response_v10_to_v9(
	x: v10::ToEnvoySqliteCommitFinalizeResponse,
) -> Result<v9::ToEnvoySqliteCommitFinalizeResponse> {
	Ok(v9::ToEnvoySqliteCommitFinalizeResponse {
		request_id: x.request_id,
		data: convert_sqlite_commit_finalize_response_v10_to_v9(x.data)?,
	})
}

pub fn convert_to_envoy_sqlite_exec_response_v10_to_v9(
	x: v10::ToEnvoySqliteExecResponse,
) -> Result<v9::ToEnvoySqliteExecResponse> {
	Ok(v9::ToEnvoySqliteExecResponse {
		request_id: x.request_id,
		data: convert_sqlite_exec_response_v10_to_v9(x.data)?,
	})
}

pub fn convert_to_envoy_sqlite_execute_response_v10_to_v9(
	x: v10::ToEnvoySqliteExecuteResponse,
) -> Result<v9::ToEnvoySqliteExecuteResponse> {
	Ok(v9::ToEnvoySqliteExecuteResponse {
		request_id: x.request_id,
		data: convert_sqlite_execute_response_v10_to_v9(x.data)?,
	})
}

pub fn convert_to_envoy_sqlite_execute_batch_response_v10_to_v9(
	x: v10::ToEnvoySqliteExecuteBatchResponse,
) -> Result<v9::ToEnvoySqliteExecuteBatchResponse> {
	Ok(v9::ToEnvoySqliteExecuteBatchResponse {
		request_id: x.request_id,
		data: convert_sqlite_execute_batch_response_v10_to_v9(x.data)?,
	})
}

pub fn convert_to_envoy_sqlite_prove_tail_v10_to_v9(
	x: v10::ToEnvoySqliteProveTail,
) -> Result<v9::ToEnvoySqliteProveTail> {
	Ok(v9::ToEnvoySqliteProveTail {
		actor_id: x.actor_id,
		generation: x.generation,
	})
}

pub fn convert_to_envoy_v10_to_v9(x: v10::ToEnvoy) -> Result<v9::ToEnvoy> {
	Ok(match x {
		v10::ToEnvoy::ToEnvoyInit(v) => {
			v9::ToEnvoy::ToEnvoyInit(convert_to_envoy_init_v10_to_v9(v)?)
		}
		v10::ToEnvoy::ToEnvoyCommands(v) => {
			v9::ToEnvoy::ToEnvoyCommands(convert_to_envoy_commands_v10_to_v9(v)?)
		}
		v10::ToEnvoy::ToEnvoyAckEvents(v) => {
			v9::ToEnvoy::ToEnvoyAckEvents(convert_to_envoy_ack_events_v10_to_v9(v)?)
		}
		v10::ToEnvoy::ToEnvoyKvResponse(v) => {
			v9::ToEnvoy::ToEnvoyKvResponse(convert_to_envoy_kv_response_v10_to_v9(v)?)
		}
		v10::ToEnvoy::ToEnvoyTunnelMessage(v) => {
			v9::ToEnvoy::ToEnvoyTunnelMessage(convert_to_envoy_tunnel_message_v10_to_v9(v)?)
		}
		v10::ToEnvoy::ToEnvoyPing(v) => {
			v9::ToEnvoy::ToEnvoyPing(convert_to_envoy_ping_v10_to_v9(v)?)
		}
		v10::ToEnvoy::ToEnvoySqliteGetPagesResponse(v) => {
			v9::ToEnvoy::ToEnvoySqliteGetPagesResponse(
				convert_to_envoy_sqlite_get_pages_response_v10_to_v9(v)?,
			)
		}
		v10::ToEnvoy::ToEnvoySqliteCommitResponse(v) => v9::ToEnvoy::ToEnvoySqliteCommitResponse(
			convert_to_envoy_sqlite_commit_response_v10_to_v9(v)?,
		),
		v10::ToEnvoy::ToEnvoySqliteCommitStageBeginResponse(v) => {
			v9::ToEnvoy::ToEnvoySqliteCommitStageBeginResponse(
				convert_to_envoy_sqlite_commit_stage_begin_response_v10_to_v9(v)?,
			)
		}
		v10::ToEnvoy::ToEnvoySqliteCommitStageSegmentResponse(v) => {
			v9::ToEnvoy::ToEnvoySqliteCommitStageSegmentResponse(
				convert_to_envoy_sqlite_commit_stage_segment_response_v10_to_v9(v)?,
			)
		}
		v10::ToEnvoy::ToEnvoySqliteCommitFinalizeResponse(v) => {
			v9::ToEnvoy::ToEnvoySqliteCommitFinalizeResponse(
				convert_to_envoy_sqlite_commit_finalize_response_v10_to_v9(v)?,
			)
		}
		v10::ToEnvoy::ToEnvoySqliteExecResponse(v) => v9::ToEnvoy::ToEnvoySqliteExecResponse(
			convert_to_envoy_sqlite_exec_response_v10_to_v9(v)?,
		),
		v10::ToEnvoy::ToEnvoySqliteExecuteResponse(v) => v9::ToEnvoy::ToEnvoySqliteExecuteResponse(
			convert_to_envoy_sqlite_execute_response_v10_to_v9(v)?,
		),
		v10::ToEnvoy::ToEnvoySqliteExecuteBatchResponse(v) => {
			v9::ToEnvoy::ToEnvoySqliteExecuteBatchResponse(
				convert_to_envoy_sqlite_execute_batch_response_v10_to_v9(v)?,
			)
		}
		v10::ToEnvoy::ToEnvoySqliteProveTail(v) => {
			v9::ToEnvoy::ToEnvoySqliteProveTail(convert_to_envoy_sqlite_prove_tail_v10_to_v9(v)?)
		}
		v10::ToEnvoy::ToEnvoyPreConnect(_) => {
			bail!("ToEnvoyPreConnect requires envoy protocol v10")
		}
	})
}

pub fn convert_to_envoy_conn_ping_v10_to_v9(
	x: v10::ToEnvoyConnPing,
) -> Result<v9::ToEnvoyConnPing> {
	Ok(v9::ToEnvoyConnPing {
		gateway_id: x.gateway_id,
		request_id: x.request_id,
		ts: x.ts,
	})
}

pub fn convert_to_envoy_conn_v10_to_v9(x: v10::ToEnvoyConn) -> Result<v9::ToEnvoyConn> {
	Ok(match x {
		v10::ToEnvoyConn::ToEnvoyConnPing(v) => {
			v9::ToEnvoyConn::ToEnvoyConnPing(convert_to_envoy_conn_ping_v10_to_v9(v)?)
		}
		v10::ToEnvoyConn::ToEnvoyConnClose => v9::ToEnvoyConn::ToEnvoyConnClose,
		v10::ToEnvoyConn::ToEnvoyCommands(v) => {
			v9::ToEnvoyConn::ToEnvoyCommands(convert_to_envoy_commands_v10_to_v9(v)?)
		}
		v10::ToEnvoyConn::ToEnvoyAckEvents(v) => {
			v9::ToEnvoyConn::ToEnvoyAckEvents(convert_to_envoy_ack_events_v10_to_v9(v)?)
		}
		v10::ToEnvoyConn::ToEnvoyTunnelMessage(v) => {
			v9::ToEnvoyConn::ToEnvoyTunnelMessage(convert_to_envoy_tunnel_message_v10_to_v9(v)?)
		}
	})
}

pub fn convert_to_gateway_pong_v10_to_v9(x: v10::ToGatewayPong) -> Result<v9::ToGatewayPong> {
	Ok(v9::ToGatewayPong {
		request_id: x.request_id,
		ts: x.ts,
	})
}

pub fn convert_to_gateway_v10_to_v9(x: v10::ToGateway) -> Result<v9::ToGateway> {
	Ok(match x {
		v10::ToGateway::ToGatewayPong(v) => {
			v9::ToGateway::ToGatewayPong(convert_to_gateway_pong_v10_to_v9(v)?)
		}
		v10::ToGateway::ToRivetTunnelMessage(v) => {
			v9::ToGateway::ToRivetTunnelMessage(convert_to_rivet_tunnel_message_v10_to_v9(v)?)
		}
	})
}

pub fn convert_to_outbound_actor_start_v10_to_v9(
	x: v10::ToOutboundActorStart,
) -> Result<v9::ToOutboundActorStart> {
	Ok(v9::ToOutboundActorStart {
		namespace_id: x.namespace_id,
		pool_name: x.pool_name,
		checkpoint: convert_actor_checkpoint_v10_to_v9(x.checkpoint)?,
		actor_config: convert_actor_config_v10_to_v9(x.actor_config)?,
	})
}

pub fn convert_to_outbound_v10_to_v9(x: v10::ToOutbound) -> Result<v9::ToOutbound> {
	Ok(match x {
		v10::ToOutbound::ToOutboundActorStart(v) => {
			v9::ToOutbound::ToOutboundActorStart(convert_to_outbound_actor_start_v10_to_v9(v)?)
		}
	})
}
