//! Embedded Depot transport for depot-client.
//!
//! This crate is for deployments where the SQLite VFS runs in the same process
//! as the Depot backend. It keeps engine storage dependencies out of the base
//! `depot-client` crate used by NAPI.

use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use depot::error::SqliteStorageError;
use depot_client::{
	database::{NativeDatabaseHandle, open_database_from_transport},
	vfs::{SqliteTransport, SqliteVfsMetrics},
};
use rivet_envoy_protocol as protocol;
use tokio::runtime::Handle;
use uuid::Uuid;

pub struct EmbeddedDepotSqliteTransport {
	db: Arc<depot::conveyer::Db>,
}

impl EmbeddedDepotSqliteTransport {
	pub fn new(db: Arc<depot::conveyer::Db>) -> Self {
		Self { db }
	}
}

pub async fn open_database_from_embedded_depot(
	db: Arc<depot::conveyer::Db>,
	actor_id: String,
	generation: u64,
	rt_handle: Handle,
	metrics: Option<Arc<dyn SqliteVfsMetrics>>,
) -> Result<NativeDatabaseHandle> {
	open_database_from_transport(
		Arc::new(EmbeddedDepotSqliteTransport::new(db)),
		actor_id,
		generation,
		rt_handle,
		metrics,
	)
	.await
}

#[async_trait]
impl SqliteTransport for EmbeddedDepotSqliteTransport {
	async fn get_pages(
		&self,
		request: protocol::SqliteGetPagesRequest,
	) -> Result<protocol::SqliteGetPagesResponse> {
		match self
			.db
			.get_pages_with_options(
				request.pgnos,
				depot::types::GetPagesOptions {
					expected_head_txid: request.expected_head_txid,
				},
			)
			.await
		{
			Ok(result) => Ok(protocol::SqliteGetPagesResponse::SqliteGetPagesOk(
				protocol::SqliteGetPagesOk {
					pages: result
						.pages
						.into_iter()
						.map(|page| protocol::SqliteFetchedPage {
							pgno: page.pgno,
							bytes: page.bytes,
						})
						.collect(),
					head_txid: Some(result.head_txid),
				},
			)),
			Err(err) => Ok(protocol::SqliteGetPagesResponse::SqliteErrorResponse(
				sqlite_error_response(&err),
			)),
		}
	}

	async fn commit(
		&self,
		request: protocol::SqliteCommitRequest,
	) -> Result<protocol::SqliteCommitResponse> {
		match self
			.db
			.commit_with_options(
				request
					.dirty_pages
					.into_iter()
					.map(|page| depot::types::DirtyPage {
						pgno: page.pgno,
						bytes: page.bytes,
					})
					.collect(),
				request.db_size_pages,
				request.now_ms,
				depot::types::CommitOptions {
					expected_head_txid: request.expected_head_txid,
				},
			)
			.await
		{
			Ok(result) => Ok(protocol::SqliteCommitResponse::SqliteCommitOk(
				protocol::SqliteCommitOk {
					head_txid: Some(result.head_txid),
				},
			)),
			Err(err) => Ok(protocol::SqliteCommitResponse::SqliteErrorResponse(
				sqlite_error_response(&err),
			)),
		}
	}

	async fn commit_stage_begin(
		&self,
		request: protocol::SqliteCommitStageBeginRequest,
	) -> Result<protocol::SqliteCommitStageBeginResponse> {
		match self
			.db
			.commit_stage_begin(
				request.dirty_pgnos,
				request.db_size_pages,
				request.now_ms,
				depot::types::CommitOptions {
					expected_head_txid: request.expected_head_txid,
				},
			)
			.await
		{
			Ok(result) => Ok(
				protocol::SqliteCommitStageBeginResponse::SqliteCommitStageBeginOk(
					protocol::SqliteCommitStageBeginOk {
						stage_id: *result.stage_id.as_bytes(),
						max_pages_per_batch: result.max_pages_per_batch,
						max_batch_bytes: result.max_batch_bytes,
						observed_head_txid: result.observed_head_txid,
						staged_txid: result.staged_txid,
					},
				),
			),
			Err(err) => Ok(protocol::SqliteCommitStageBeginResponse::SqliteErrorResponse(
				sqlite_error_response(&err),
			)),
		}
	}

	async fn commit_stage_pages(
		&self,
		request: protocol::SqliteCommitStagePagesRequest,
	) -> Result<protocol::SqliteCommitStagePagesResponse> {
		match self
			.db
			.commit_stage_pages(
				stage_id_from_protocol(&request.stage_id)?,
				request.batch_idx,
				request
					.dirty_pages
					.into_iter()
					.map(|page| depot::types::DirtyPage {
						pgno: page.pgno,
						bytes: page.bytes,
					})
					.collect(),
			)
			.await
		{
			Ok(()) => Ok(protocol::SqliteCommitStagePagesResponse::SqliteCommitStagePagesOk),
			Err(err) => Ok(protocol::SqliteCommitStagePagesResponse::SqliteErrorResponse(
				sqlite_error_response(&err),
			)),
		}
	}

	async fn commit_stage_complete(
		&self,
		request: protocol::SqliteCommitStageCompleteRequest,
	) -> Result<protocol::SqliteCommitStageCompleteResponse> {
		match self
			.db
			.commit_stage_complete(
				stage_id_from_protocol(&request.stage_id)?,
				request.page_batch_count,
			)
			.await
		{
			Ok(()) => Ok(
				protocol::SqliteCommitStageCompleteResponse::SqliteCommitStageCompleteOk,
			),
			Err(err) => Ok(
				protocol::SqliteCommitStageCompleteResponse::SqliteErrorResponse(
					sqlite_error_response(&err),
				),
			),
		}
	}

	async fn commit_stage_finalize(
		&self,
		request: protocol::SqliteCommitStageFinalizeRequest,
	) -> Result<protocol::SqliteCommitResponse> {
		match self
			.db
			.commit_stage_finalize(stage_id_from_protocol(&request.stage_id)?)
			.await
		{
			Ok(result) => Ok(protocol::SqliteCommitResponse::SqliteCommitOk(
				protocol::SqliteCommitOk {
					head_txid: Some(result.head_txid),
				},
			)),
			Err(err) => Ok(protocol::SqliteCommitResponse::SqliteErrorResponse(
				sqlite_error_response(&err),
			)),
		}
	}

	async fn commit_stage_abort(
		&self,
		request: protocol::SqliteCommitStageAbortRequest,
	) -> Result<protocol::SqliteCommitStageAbortResponse> {
		match self
			.db
			.commit_stage_abort(stage_id_from_protocol(&request.stage_id)?)
			.await
		{
			Ok(()) => Ok(protocol::SqliteCommitStageAbortResponse::SqliteCommitStageAbortOk),
			Err(err) => Ok(protocol::SqliteCommitStageAbortResponse::SqliteErrorResponse(
				sqlite_error_response(&err),
			)),
		}
	}
}

fn sqlite_error_reason(err: &anyhow::Error) -> String {
	err.chain()
		.map(ToString::to_string)
		.collect::<Vec<_>>()
		.join(": ")
}

fn sqlite_error_response(err: &anyhow::Error) -> protocol::SqliteErrorResponse {
	let structured = depot_error(err)
		.map(|err| rivet_error::RivetError::extract(&err.clone().build()))
		.unwrap_or_else(|| rivet_error::RivetError::extract(err));
	protocol::SqliteErrorResponse {
		group: structured.group().to_string(),
		code: structured.code().to_string(),
		message: sqlite_error_reason(err),
		metadata: structured.metadata().map(|metadata| metadata.to_string()),
	}
}

fn stage_id_from_protocol(stage_id: &protocol::SqliteStageId) -> Result<Uuid> {
	Uuid::from_slice(stage_id).map_err(Into::into)
}

fn depot_error(err: &anyhow::Error) -> Option<&SqliteStorageError> {
	err.chain()
		.find_map(|source| source.downcast_ref::<SqliteStorageError>())
}
