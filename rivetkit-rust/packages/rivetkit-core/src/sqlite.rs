use anyhow::{Result, anyhow};
use rivet_envoy_client::handle::EnvoyHandle;
use rivet_envoy_client::protocol;

#[derive(Clone)]
pub struct SqliteRuntimeConfig {
	pub handle: EnvoyHandle,
	pub actor_id: String,
	pub schema_version: u32,
	pub startup_data: Option<protocol::SqliteStartupData>,
}

#[derive(Clone, Default)]
pub struct SqliteDb {
	handle: Option<EnvoyHandle>,
	actor_id: Option<String>,
	schema_version: Option<u32>,
	startup_data: Option<protocol::SqliteStartupData>,
}

impl SqliteDb {
	pub fn new(
		handle: EnvoyHandle,
		actor_id: impl Into<String>,
		schema_version: u32,
		startup_data: Option<protocol::SqliteStartupData>,
	) -> Self {
		Self {
			handle: Some(handle),
			actor_id: Some(actor_id.into()),
			schema_version: Some(schema_version),
			startup_data,
		}
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

	pub fn runtime_config(&self) -> Result<SqliteRuntimeConfig> {
		Ok(SqliteRuntimeConfig {
			handle: self.handle()?,
			actor_id: self
				.actor_id
				.clone()
				.ok_or_else(|| anyhow!("sqlite actor id is not configured"))?,
			schema_version: self
				.schema_version
				.ok_or_else(|| anyhow!("sqlite schema version is not configured"))?,
			startup_data: self.startup_data.clone(),
		})
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
			.field("actor_id", &self.actor_id)
			.field("schema_version", &self.schema_version)
			.finish()
	}
}
