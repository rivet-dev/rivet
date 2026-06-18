use anyhow::Result;
use async_trait::async_trait;
use depot_client::vfs::SqliteTransport;
use rivet_envoy_client::{handle::EnvoyHandle, protocol};

pub(super) struct EnvoySqliteTransport {
	handle: EnvoyHandle,
}

impl EnvoySqliteTransport {
	pub(super) fn new(handle: EnvoyHandle) -> Self {
		Self { handle }
	}
}

#[async_trait]
impl SqliteTransport for EnvoySqliteTransport {
	async fn get_pages(
		&self,
		request: protocol::SqliteGetPagesRequest,
	) -> Result<protocol::SqliteGetPagesResponse> {
		self.handle.sqlite_get_pages(request).await
	}

	async fn commit(
		&self,
		request: protocol::SqliteCommitRequest,
	) -> Result<protocol::SqliteCommitResponse> {
		self.handle.sqlite_commit(request).await
	}

	async fn commit_stage_begin(
		&self,
		request: protocol::SqliteCommitStageBeginRequest,
	) -> Result<protocol::SqliteCommitStageBeginResponse> {
		self.handle.sqlite_commit_stage_begin(request).await
	}

	async fn commit_stage_pages(
		&self,
		request: protocol::SqliteCommitStagePagesRequest,
	) -> Result<protocol::SqliteCommitStagePagesResponse> {
		self.handle.sqlite_commit_stage_pages(request).await
	}

	async fn commit_stage_complete(
		&self,
		request: protocol::SqliteCommitStageCompleteRequest,
	) -> Result<protocol::SqliteCommitStageCompleteResponse> {
		self.handle.sqlite_commit_stage_complete(request).await
	}

	async fn commit_stage_finalize(
		&self,
		request: protocol::SqliteCommitStageFinalizeRequest,
	) -> Result<protocol::SqliteCommitResponse> {
		self.handle.sqlite_commit_stage_finalize(request).await
	}

	async fn commit_stage_abort(
		&self,
		request: protocol::SqliteCommitStageAbortRequest,
	) -> Result<protocol::SqliteCommitStageAbortResponse> {
		self.handle.sqlite_commit_stage_abort(request).await
	}
}
