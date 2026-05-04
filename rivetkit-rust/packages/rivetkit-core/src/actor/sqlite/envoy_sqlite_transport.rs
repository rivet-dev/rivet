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
}
