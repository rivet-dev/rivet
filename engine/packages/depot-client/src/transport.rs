//! SQLite transport adapters.
//!
//! `EmbeddedDepotSqliteTransport` is for deployments where the SQLite VFS runs in the
//! same process or server as the Depot backend. It calls `depot::conveyer::Db`
//! directly instead of routing page operations through an actor Envoy transport.

use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use rivet_envoy_protocol as protocol;

use crate::vfs::SqliteTransport;

pub struct EmbeddedDepotSqliteTransport {
	db: Arc<depot::conveyer::Db>,
}

impl EmbeddedDepotSqliteTransport {
	pub fn new(db: Arc<depot::conveyer::Db>) -> Self {
		Self { db }
	}
}

#[async_trait]
impl SqliteTransport for EmbeddedDepotSqliteTransport {
	async fn get_pages(
		&self,
		request: protocol::SqliteGetPagesRequest,
	) -> Result<protocol::SqliteGetPagesResponse> {
		match self.db.get_pages(request.pgnos).await {
			Ok(pages) => Ok(protocol::SqliteGetPagesResponse::SqliteGetPagesOk(
				protocol::SqliteGetPagesOk {
					pages: pages
						.into_iter()
						.map(|page| protocol::SqliteFetchedPage {
							pgno: page.pgno,
							bytes: page.bytes,
						})
						.collect(),
				},
			)),
			Err(err) => Ok(protocol::SqliteGetPagesResponse::SqliteErrorResponse(
				protocol::SqliteErrorResponse {
					message: sqlite_error_reason(&err),
				},
			)),
		}
	}

	async fn commit(
		&self,
		request: protocol::SqliteCommitRequest,
	) -> Result<protocol::SqliteCommitResponse> {
		match self
			.db
			.commit(
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
			)
			.await
		{
			Ok(()) => Ok(protocol::SqliteCommitResponse::SqliteCommitOk),
			Err(err) => Ok(protocol::SqliteCommitResponse::SqliteErrorResponse(
				protocol::SqliteErrorResponse {
					message: sqlite_error_reason(&err),
				},
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
