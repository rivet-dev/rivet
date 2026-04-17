use anyhow::{Result, anyhow};
use rivet_envoy_client::handle::EnvoyHandle;
use rivet_envoy_client::protocol;

#[derive(Clone, Default)]
pub struct SqliteDb {
	handle: Option<EnvoyHandle>,
}

impl SqliteDb {
	/// `actor_id` is not stored here because the SQLite protocol request types already carry it.
	pub fn new(handle: EnvoyHandle) -> Self {
		Self { handle: Some(handle) }
	}

	pub async fn get_pages(
		&self,
		request: protocol::SqliteGetPagesRequest,
	) -> Result<protocol::SqliteGetPagesResponse> {
		self.handle()?.sqlite_get_pages(request).await
	}

	pub async fn commit(
		&self,
		request: protocol::SqliteCommitRequest,
	) -> Result<protocol::SqliteCommitResponse> {
		self.handle()?.sqlite_commit(request).await
	}

	pub async fn commit_stage_begin(
		&self,
		request: protocol::SqliteCommitStageBeginRequest,
	) -> Result<protocol::SqliteCommitStageBeginResponse> {
		self.handle()?.sqlite_commit_stage_begin(request).await
	}

	pub async fn commit_stage(
		&self,
		request: protocol::SqliteCommitStageRequest,
	) -> Result<protocol::SqliteCommitStageResponse> {
		self.handle()?.sqlite_commit_stage(request).await
	}

	pub fn commit_stage_fire_and_forget(
		&self,
		request: protocol::SqliteCommitStageRequest,
	) -> Result<()> {
		self.handle()?.sqlite_commit_stage_fire_and_forget(request)
	}

	pub async fn commit_finalize(
		&self,
		request: protocol::SqliteCommitFinalizeRequest,
	) -> Result<protocol::SqliteCommitFinalizeResponse> {
		self.handle()?.sqlite_commit_finalize(request).await
	}

	pub(crate) async fn cleanup(&self) -> Result<()> {
		Ok(())
	}

	fn handle(&self) -> Result<EnvoyHandle> {
		self.handle
			.clone()
			.ok_or_else(|| anyhow!("sqlite handle is not configured"))
	}
}

impl std::fmt::Debug for SqliteDb {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("SqliteDb")
			.field("configured", &self.handle.is_some())
			.finish()
	}
}
