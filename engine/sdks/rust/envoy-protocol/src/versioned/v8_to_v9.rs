// Field-by-field protocol v8 -> v9 conversion. No serialization round trips.
#![allow(dead_code, unused_variables)]
use crate::generated::{v8, v9};
use anyhow::Result;
pub fn convert_id_v8_to_v9(x: v8::Id) -> Result<v9::Id> {
	Ok(x)
}
pub fn convert_json_v8_to_v9(x: v8::Json) -> Result<v9::Json> {
	Ok(x)
}
pub fn convert_gateway_id_v8_to_v9(x: v8::GatewayId) -> Result<v9::GatewayId> {
	Ok(x)
}
pub fn convert_request_id_v8_to_v9(x: v8::RequestId) -> Result<v9::RequestId> {
	Ok(x)
}
pub fn convert_message_index_v8_to_v9(x: v8::MessageIndex) -> Result<v9::MessageIndex> {
	Ok(x)
}
pub fn convert_kv_key_v8_to_v9(x: v8::KvKey) -> Result<v9::KvKey> {
	Ok(x)
}
pub fn convert_kv_value_v8_to_v9(x: v8::KvValue) -> Result<v9::KvValue> {
	Ok(x)
}
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
		start: convert_kv_key_v8_to_v9(x.start)?,
		end: convert_kv_key_v8_to_v9(x.end)?,
		exclusive: x.exclusive,
	})
}
pub fn convert_kv_list_prefix_query_v8_to_v9(
	x: v8::KvListPrefixQuery,
) -> Result<v9::KvListPrefixQuery> {
	Ok(v9::KvListPrefixQuery {
		key: convert_kv_key_v8_to_v9(x.key)?,
	})
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
	Ok(v9::KvGetRequest {
		keys: x
			.keys
			.into_iter()
			.map(|v| Ok::<_, anyhow::Error>(convert_kv_key_v8_to_v9(v)?))
			.collect::<Result<Vec<_>>>()?,
	})
}
pub fn convert_kv_list_request_v8_to_v9(x: v8::KvListRequest) -> Result<v9::KvListRequest> {
	Ok(v9::KvListRequest {
		query: convert_kv_list_query_v8_to_v9(x.query)?,
		reverse: x.reverse.map(|v| Ok::<_, anyhow::Error>(v)).transpose()?,
		limit: x.limit.map(|v| Ok::<_, anyhow::Error>(v)).transpose()?,
	})
}
pub fn convert_kv_put_request_v8_to_v9(x: v8::KvPutRequest) -> Result<v9::KvPutRequest> {
	Ok(v9::KvPutRequest {
		keys: x
			.keys
			.into_iter()
			.map(|v| Ok::<_, anyhow::Error>(convert_kv_key_v8_to_v9(v)?))
			.collect::<Result<Vec<_>>>()?,
		values: x
			.values
			.into_iter()
			.map(|v| Ok::<_, anyhow::Error>(convert_kv_value_v8_to_v9(v)?))
			.collect::<Result<Vec<_>>>()?,
	})
}
pub fn convert_kv_delete_request_v8_to_v9(x: v8::KvDeleteRequest) -> Result<v9::KvDeleteRequest> {
	Ok(v9::KvDeleteRequest {
		keys: x
			.keys
			.into_iter()
			.map(|v| Ok::<_, anyhow::Error>(convert_kv_key_v8_to_v9(v)?))
			.collect::<Result<Vec<_>>>()?,
	})
}
pub fn convert_kv_delete_range_request_v8_to_v9(
	x: v8::KvDeleteRangeRequest,
) -> Result<v9::KvDeleteRangeRequest> {
	Ok(v9::KvDeleteRangeRequest {
		start: convert_kv_key_v8_to_v9(x.start)?,
		end: convert_kv_key_v8_to_v9(x.end)?,
	})
}
pub fn convert_kv_error_response_v8_to_v9(x: v8::KvErrorResponse) -> Result<v9::KvErrorResponse> {
	Ok(v9::KvErrorResponse { message: x.message })
}
pub fn convert_kv_get_response_v8_to_v9(x: v8::KvGetResponse) -> Result<v9::KvGetResponse> {
	Ok(v9::KvGetResponse {
		keys: x
			.keys
			.into_iter()
			.map(|v| Ok::<_, anyhow::Error>(convert_kv_key_v8_to_v9(v)?))
			.collect::<Result<Vec<_>>>()?,
		values: x
			.values
			.into_iter()
			.map(|v| Ok::<_, anyhow::Error>(convert_kv_value_v8_to_v9(v)?))
			.collect::<Result<Vec<_>>>()?,
		metadata: x
			.metadata
			.into_iter()
			.map(|v| Ok::<_, anyhow::Error>(convert_kv_metadata_v8_to_v9(v)?))
			.collect::<Result<Vec<_>>>()?,
	})
}
pub fn convert_kv_list_response_v8_to_v9(x: v8::KvListResponse) -> Result<v9::KvListResponse> {
	Ok(v9::KvListResponse {
		keys: x
			.keys
			.into_iter()
			.map(|v| Ok::<_, anyhow::Error>(convert_kv_key_v8_to_v9(v)?))
			.collect::<Result<Vec<_>>>()?,
		values: x
			.values
			.into_iter()
			.map(|v| Ok::<_, anyhow::Error>(convert_kv_value_v8_to_v9(v)?))
			.collect::<Result<Vec<_>>>()?,
		metadata: x
			.metadata
			.into_iter()
			.map(|v| Ok::<_, anyhow::Error>(convert_kv_metadata_v8_to_v9(v)?))
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
pub fn convert_sqlite_pgno_v8_to_v9(x: v8::SqlitePgno) -> Result<v9::SqlitePgno> {
	Ok(x)
}
pub fn convert_sqlite_generation_v8_to_v9(x: v8::SqliteGeneration) -> Result<v9::SqliteGeneration> {
	Ok(x)
}
pub fn convert_sqlite_page_bytes_v8_to_v9(x: v8::SqlitePageBytes) -> Result<v9::SqlitePageBytes> {
	Ok(x)
}
pub fn convert_sqlite_dirty_page_v8_to_v9(x: v8::SqliteDirtyPage) -> Result<v9::SqliteDirtyPage> {
	Ok(v9::SqliteDirtyPage {
		pgno: convert_sqlite_pgno_v8_to_v9(x.pgno)?,
		bytes: convert_sqlite_page_bytes_v8_to_v9(x.bytes)?,
	})
}
pub fn convert_sqlite_fetched_page_v8_to_v9(
	x: v8::SqliteFetchedPage,
) -> Result<v9::SqliteFetchedPage> {
	Ok(v9::SqliteFetchedPage {
		pgno: convert_sqlite_pgno_v8_to_v9(x.pgno)?,
		bytes: x
			.bytes
			.map(|v| Ok::<_, anyhow::Error>(convert_sqlite_page_bytes_v8_to_v9(v)?))
			.transpose()?,
	})
}
pub fn convert_sqlite_get_pages_request_v8_to_v9(
	x: v8::SqliteGetPagesRequest,
) -> Result<v9::SqliteGetPagesRequest> {
	Ok(v9::SqliteGetPagesRequest {
		actor_id: convert_id_v8_to_v9(x.actor_id)?,
		pgnos: x
			.pgnos
			.into_iter()
			.map(|v| Ok::<_, anyhow::Error>(convert_sqlite_pgno_v8_to_v9(v)?))
			.collect::<Result<Vec<_>>>()?,
		expected_generation: x
			.expected_generation
			.map(|v| Ok::<_, anyhow::Error>(v))
			.transpose()?,
		expected_head_txid: x
			.expected_head_txid
			.map(|v| Ok::<_, anyhow::Error>(v))
			.transpose()?,
	})
}
pub fn convert_sqlite_get_pages_ok_v8_to_v9(
	x: v8::SqliteGetPagesOk,
) -> Result<v9::SqliteGetPagesOk> {
	Ok(v9::SqliteGetPagesOk {
		pages: x
			.pages
			.into_iter()
			.map(|v| Ok::<_, anyhow::Error>(convert_sqlite_fetched_page_v8_to_v9(v)?))
			.collect::<Result<Vec<_>>>()?,
		head_txid: x.head_txid.map(|v| Ok::<_, anyhow::Error>(v)).transpose()?,
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
		actor_id: convert_id_v8_to_v9(x.actor_id)?,
		dirty_pages: x
			.dirty_pages
			.into_iter()
			.map(|v| Ok::<_, anyhow::Error>(convert_sqlite_dirty_page_v8_to_v9(v)?))
			.collect::<Result<Vec<_>>>()?,
		db_size_pages: x.db_size_pages,
		now_ms: x.now_ms,
		expected_generation: x
			.expected_generation
			.map(|v| Ok::<_, anyhow::Error>(v))
			.transpose()?,
		expected_head_txid: x
			.expected_head_txid
			.map(|v| Ok::<_, anyhow::Error>(v))
			.transpose()?,
	})
}
pub fn convert_sqlite_commit_ok_v8_to_v9(x: v8::SqliteCommitOk) -> Result<v9::SqliteCommitOk> {
	Ok(v9::SqliteCommitOk {
		head_txid: x.head_txid.map(|v| Ok::<_, anyhow::Error>(v)).transpose()?,
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
		actor_id: convert_id_v8_to_v9(x.actor_id)?,
		expected_generation: x
			.expected_generation
			.map(|v| Ok::<_, anyhow::Error>(v))
			.transpose()?,
		expected_head_txid: x
			.expected_head_txid
			.map(|v| Ok::<_, anyhow::Error>(v))
			.transpose()?,
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
		actor_id: convert_id_v8_to_v9(x.actor_id)?,
		expected_generation: x
			.expected_generation
			.map(|v| Ok::<_, anyhow::Error>(v))
			.transpose()?,
		txid: x.txid,
		first_pgno: x.first_pgno,
		dirty_pages: x
			.dirty_pages
			.into_iter()
			.map(|v| Ok::<_, anyhow::Error>(convert_sqlite_dirty_page_v8_to_v9(v)?))
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
		actor_id: convert_id_v8_to_v9(x.actor_id)?,
		expected_generation: x
			.expected_generation
			.map(|v| Ok::<_, anyhow::Error>(v))
			.transpose()?,
		txid: x.txid,
		new_db_size_pages: x.new_db_size_pages,
		now_ms: x.now_ms,
		segment_first_pgnos: x
			.segment_first_pgnos
			.into_iter()
			.map(|v| Ok::<_, anyhow::Error>(v))
			.collect::<Result<Vec<_>>>()?,
	})
}
pub fn convert_sqlite_commit_finalize_ok_v8_to_v9(
	x: v8::SqliteCommitFinalizeOk,
) -> Result<v9::SqliteCommitFinalizeOk> {
	Ok(v9::SqliteCommitFinalizeOk {
		head_txid: x.head_txid.map(|v| Ok::<_, anyhow::Error>(v)).transpose()?,
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
		columns: x
			.columns
			.into_iter()
			.map(|v| Ok::<_, anyhow::Error>(v))
			.collect::<Result<Vec<_>>>()?,
		rows: x
			.rows
			.into_iter()
			.map(|v| {
				Ok::<_, anyhow::Error>(
					v.into_iter()
						.map(|v| Ok::<_, anyhow::Error>(convert_sqlite_column_value_v8_to_v9(v)?))
						.collect::<Result<Vec<_>>>()?,
				)
			})
			.collect::<Result<Vec<_>>>()?,
	})
}
pub fn convert_sqlite_execute_result_v8_to_v9(
	x: v8::SqliteExecuteResult,
) -> Result<v9::SqliteExecuteResult> {
	Ok(v9::SqliteExecuteResult {
		columns: x
			.columns
			.into_iter()
			.map(|v| Ok::<_, anyhow::Error>(v))
			.collect::<Result<Vec<_>>>()?,
		rows: x
			.rows
			.into_iter()
			.map(|v| {
				Ok::<_, anyhow::Error>(
					v.into_iter()
						.map(|v| Ok::<_, anyhow::Error>(convert_sqlite_column_value_v8_to_v9(v)?))
						.collect::<Result<Vec<_>>>()?,
				)
			})
			.collect::<Result<Vec<_>>>()?,
		changes: x.changes,
		last_insert_row_id: x
			.last_insert_row_id
			.map(|v| Ok::<_, anyhow::Error>(v))
			.transpose()?,
	})
}
pub fn convert_sqlite_exec_request_v8_to_v9(
	x: v8::SqliteExecRequest,
) -> Result<v9::SqliteExecRequest> {
	Ok(v9::SqliteExecRequest {
		namespace_id: convert_id_v8_to_v9(x.namespace_id)?,
		actor_id: convert_id_v8_to_v9(x.actor_id)?,
		generation: convert_sqlite_generation_v8_to_v9(x.generation)?,
		sql: x.sql,
	})
}
pub fn convert_sqlite_execute_request_v8_to_v9(
	x: v8::SqliteExecuteRequest,
) -> Result<v9::SqliteExecuteRequest> {
	Ok(v9::SqliteExecuteRequest {
		namespace_id: convert_id_v8_to_v9(x.namespace_id)?,
		actor_id: convert_id_v8_to_v9(x.actor_id)?,
		generation: convert_sqlite_generation_v8_to_v9(x.generation)?,
		sql: x.sql,
		params: x
			.params
			.map(|v| {
				Ok::<_, anyhow::Error>(
					v.into_iter()
						.map(|v| Ok::<_, anyhow::Error>(convert_sqlite_bind_param_v8_to_v9(v)?))
						.collect::<Result<Vec<_>>>()?,
				)
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
				Ok::<_, anyhow::Error>(
					v.into_iter()
						.map(|v| Ok::<_, anyhow::Error>(convert_sqlite_bind_param_v8_to_v9(v)?))
						.collect::<Result<Vec<_>>>()?,
				)
			})
			.transpose()?,
	})
}
pub fn convert_sqlite_execute_batch_request_v8_to_v9(
	x: v8::SqliteExecuteBatchRequest,
) -> Result<v9::SqliteExecuteBatchRequest> {
	Ok(v9::SqliteExecuteBatchRequest {
		namespace_id: convert_id_v8_to_v9(x.namespace_id)?,
		actor_id: convert_id_v8_to_v9(x.actor_id)?,
		generation: convert_sqlite_generation_v8_to_v9(x.generation)?,
		statements: x
			.statements
			.into_iter()
			.map(|v| Ok::<_, anyhow::Error>(convert_sqlite_batch_statement_v8_to_v9(v)?))
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
			.map(|v| Ok::<_, anyhow::Error>(convert_sqlite_execute_result_v8_to_v9(v)?))
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
		metadata: convert_json_v8_to_v9(x.metadata)?,
	})
}
pub fn convert_actor_config_v8_to_v9(x: v8::ActorConfig) -> Result<v9::ActorConfig> {
	Ok(v9::ActorConfig {
		name: x.name,
		key: x.key.map(|v| Ok::<_, anyhow::Error>(v)).transpose()?,
		create_ts: x.create_ts,
		input: x.input.map(|v| Ok::<_, anyhow::Error>(v)).transpose()?,
	})
}
pub fn convert_actor_checkpoint_v8_to_v9(x: v8::ActorCheckpoint) -> Result<v9::ActorCheckpoint> {
	Ok(v9::ActorCheckpoint {
		actor_id: convert_id_v8_to_v9(x.actor_id)?,
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
		message: x.message.map(|v| Ok::<_, anyhow::Error>(v)).transpose()?,
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
		alarm_ts: x.alarm_ts.map(|v| Ok::<_, anyhow::Error>(v)).transpose()?,
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
		key: convert_kv_key_v8_to_v9(x.key)?,
		value: convert_kv_value_v8_to_v9(x.value)?,
		metadata: convert_kv_metadata_v8_to_v9(x.metadata)?,
	})
}
pub fn convert_preloaded_kv_v8_to_v9(x: v8::PreloadedKv) -> Result<v9::PreloadedKv> {
	Ok(v9::PreloadedKv {
		entries: x
			.entries
			.into_iter()
			.map(|v| Ok::<_, anyhow::Error>(convert_preloaded_kv_entry_v8_to_v9(v)?))
			.collect::<Result<Vec<_>>>()?,
		requested_get_keys: x
			.requested_get_keys
			.into_iter()
			.map(|v| Ok::<_, anyhow::Error>(convert_kv_key_v8_to_v9(v)?))
			.collect::<Result<Vec<_>>>()?,
		requested_prefixes: x
			.requested_prefixes
			.into_iter()
			.map(|v| Ok::<_, anyhow::Error>(convert_kv_key_v8_to_v9(v)?))
			.collect::<Result<Vec<_>>>()?,
	})
}
pub fn convert_hibernating_request_v8_to_v9(
	x: v8::HibernatingRequest,
) -> Result<v9::HibernatingRequest> {
	Ok(v9::HibernatingRequest {
		gateway_id: convert_gateway_id_v8_to_v9(x.gateway_id)?,
		request_id: convert_request_id_v8_to_v9(x.request_id)?,
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
			.map(|v| Ok::<_, anyhow::Error>(convert_hibernating_request_v8_to_v9(v)?))
			.collect::<Result<Vec<_>>>()?,
		preloaded_kv: x
			.preloaded_kv
			.map(|v| Ok::<_, anyhow::Error>(convert_preloaded_kv_v8_to_v9(v)?))
			.transpose()?,
		sqlite_fence: None,
		sqlite_startup: None,
		waiting_requests: Vec::new(),
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
		gateway_id: convert_gateway_id_v8_to_v9(x.gateway_id)?,
		request_id: convert_request_id_v8_to_v9(x.request_id)?,
		message_index: convert_message_index_v8_to_v9(x.message_index)?,
	})
}
pub fn convert_to_envoy_request_start_v8_to_v9(
	x: v8::ToEnvoyRequestStart,
) -> Result<v9::ToEnvoyRequestStart> {
	Ok(v9::ToEnvoyRequestStart {
		actor_id: convert_id_v8_to_v9(x.actor_id)?,
		actor_generation: x
			.actor_generation
			.map(|v| Ok::<_, anyhow::Error>(v))
			.transpose()?,
		method: x.method,
		path: x.path,
		headers: x
			.headers
			.into_iter()
			.map(|(k, v)| Ok((k, v)))
			.collect::<Result<std::collections::HashMap<_, _>>>()?,
		body: x.body.map(|v| Ok::<_, anyhow::Error>(v)).transpose()?,
		stream: x.stream,
		response_stream: x.response_stream,
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
		detail: x.detail.map(|v| Ok::<_, anyhow::Error>(v)).transpose()?,
	})
}
pub fn convert_to_envoy_request_abort_v8_to_v9(
	x: v8::ToEnvoyRequestAbort,
) -> Result<v9::ToEnvoyRequestAbort> {
	Ok(v9::ToEnvoyRequestAbort {
		actor_id: x
			.actor_id
			.map(|v| Ok::<_, anyhow::Error>(convert_id_v8_to_v9(v)?))
			.transpose()?,
		actor_generation: x
			.actor_generation
			.map(|v| Ok::<_, anyhow::Error>(v))
			.transpose()?,
		reason: convert_http_stream_abort_reason_v8_to_v9(x.reason)?,
	})
}
pub fn convert_to_rivet_response_start_v8_to_v9(
	x: v8::ToRivetResponseStart,
) -> Result<v9::ToRivetResponseStart> {
	Ok(v9::ToRivetResponseStart {
		status: x.status,
		headers: x
			.headers
			.into_iter()
			.map(|(k, v)| Ok((k, v)))
			.collect::<Result<std::collections::HashMap<_, _>>>()?,
		body: x.body.map(|v| Ok::<_, anyhow::Error>(v)).transpose()?,
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
		actor_id: convert_id_v8_to_v9(x.actor_id)?,
		actor_generation: x
			.actor_generation
			.map(|v| Ok::<_, anyhow::Error>(v))
			.transpose()?,
		path: x.path,
		headers: x
			.headers
			.into_iter()
			.map(|(k, v)| Ok((k, v)))
			.collect::<Result<std::collections::HashMap<_, _>>>()?,
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
		code: x.code.map(|v| Ok::<_, anyhow::Error>(v)).transpose()?,
		reason: x.reason.map(|v| Ok::<_, anyhow::Error>(v)).transpose()?,
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
	Ok(v9::ToRivetWebSocketMessageAck {
		index: convert_message_index_v8_to_v9(x.index)?,
	})
}
pub fn convert_to_rivet_web_socket_close_v8_to_v9(
	x: v8::ToRivetWebSocketClose,
) -> Result<v9::ToRivetWebSocketClose> {
	Ok(v9::ToRivetWebSocketClose {
		code: x.code.map(|v| Ok::<_, anyhow::Error>(v)).transpose()?,
		reason: x.reason.map(|v| Ok::<_, anyhow::Error>(v)).transpose()?,
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
				Ok::<_, anyhow::Error>(
					v.into_iter()
						.map(|(k, v)| Ok((k, convert_actor_name_v8_to_v9(v)?)))
						.collect::<Result<std::collections::HashMap<_, _>>>()?,
				)
			})
			.transpose()?,
		metadata: x
			.metadata
			.map(|v| Ok::<_, anyhow::Error>(convert_json_v8_to_v9(v)?))
			.transpose()?,
	})
}
pub fn convert_to_rivet_events_v8_to_v9(x: v8::ToRivetEvents) -> Result<v9::ToRivetEvents> {
	Ok(x.into_iter()
		.map(|v| Ok::<_, anyhow::Error>(convert_event_wrapper_v8_to_v9(v)?))
		.collect::<Result<Vec<_>>>()?)
}
pub fn convert_to_rivet_ack_commands_v8_to_v9(
	x: v8::ToRivetAckCommands,
) -> Result<v9::ToRivetAckCommands> {
	Ok(v9::ToRivetAckCommands {
		last_command_checkpoints: x
			.last_command_checkpoints
			.into_iter()
			.map(|v| Ok::<_, anyhow::Error>(convert_actor_checkpoint_v8_to_v9(v)?))
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
		actor_id: convert_id_v8_to_v9(x.actor_id)?,
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
		.map(|v| Ok::<_, anyhow::Error>(convert_command_wrapper_v8_to_v9(v)?))
		.collect::<Result<Vec<_>>>()?)
}
pub fn convert_to_envoy_ack_events_v8_to_v9(
	x: v8::ToEnvoyAckEvents,
) -> Result<v9::ToEnvoyAckEvents> {
	Ok(v9::ToEnvoyAckEvents {
		last_event_checkpoints: x
			.last_event_checkpoints
			.into_iter()
			.map(|v| Ok::<_, anyhow::Error>(convert_actor_checkpoint_v8_to_v9(v)?))
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
		gateway_id: convert_gateway_id_v8_to_v9(x.gateway_id)?,
		request_id: convert_request_id_v8_to_v9(x.request_id)?,
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
		request_id: convert_request_id_v8_to_v9(x.request_id)?,
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
		namespace_id: convert_id_v8_to_v9(x.namespace_id)?,
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
