use anyhow::Result;
use async_trait::async_trait;
use rivet_envoy_client::utils::{RemoteSqliteIndeterminateResultError, SqliteRequestTimeoutError};
use rivet_envoy_client::{handle::EnvoyHandle, protocol};
use sqlite_embedded::vfs::{
	SqliteCommitIndeterminateError, SqliteStorageTimeoutError, SqliteTransport,
};

pub(super) struct EnvoySqliteTransport {
	handle: EnvoyHandle,
}

impl EnvoySqliteTransport {
	pub(super) fn new(handle: EnvoyHandle) -> Self {
		Self { handle }
	}
}

/// A page read that timed out changed nothing, so it reaches the VFS as a typed timeout.
fn map_read_error(error: anyhow::Error) -> anyhow::Error {
	match error.downcast::<SqliteRequestTimeoutError>() {
		Ok(timeout) => anyhow::Error::new(SqliteStorageTimeoutError {
			operation: timeout.operation.to_owned(),
		}),
		Err(error) => error,
	}
}

/// A write whose response was lost after it was sent, to a disconnect or to its deadline, may
/// have committed, so the VFS fails closed on it as indeterminate. One that timed out before it
/// was sent, or a stage request, committed nothing and reaches the VFS as a typed timeout.
fn map_commit_error(error: anyhow::Error) -> anyhow::Error {
	let error = match error.downcast::<RemoteSqliteIndeterminateResultError>() {
		Ok(error) => {
			return anyhow::Error::new(SqliteCommitIndeterminateError(error.operation.to_owned()));
		}
		Err(error) => error,
	};
	match error.downcast::<SqliteRequestTimeoutError>() {
		// A sent stage begin or segment commits nothing: the next attempt reopens the same
		// stage. Only a sent commit or finalize may have committed.
		Ok(timeout)
			if timeout.sent && matches!(timeout.operation, "commit" | "commit_finalize") =>
		{
			anyhow::Error::new(SqliteCommitIndeterminateError(timeout.operation.to_owned()))
		}
		Ok(timeout) => anyhow::Error::new(SqliteStorageTimeoutError {
			operation: timeout.operation.to_owned(),
		}),
		Err(error) => error,
	}
}

#[async_trait]
impl SqliteTransport for EnvoySqliteTransport {
	async fn get_pages(
		&self,
		request: protocol::SqliteGetPagesRequest,
	) -> Result<protocol::SqliteGetPagesResponse> {
		self.handle
			.sqlite_get_pages(request)
			.await
			.map_err(map_read_error)
	}

	async fn commit(
		&self,
		request: protocol::SqliteCommitRequest,
	) -> Result<protocol::SqliteCommitResponse> {
		self.handle
			.sqlite_commit(request)
			.await
			.map_err(map_commit_error)
	}

	async fn commit_stage_begin(
		&self,
		request: protocol::SqliteCommitStageBeginRequest,
	) -> Result<protocol::SqliteCommitStageBeginResponse> {
		self.handle
			.sqlite_commit_stage_begin(request)
			.await
			.map_err(map_commit_error)
	}

	async fn commit_stage_segment(
		&self,
		request: protocol::SqliteCommitStageSegmentRequest,
	) -> Result<protocol::SqliteCommitStageSegmentResponse> {
		self.handle
			.sqlite_commit_stage_segment(request)
			.await
			.map_err(map_commit_error)
	}

	async fn commit_finalize(
		&self,
		request: protocol::SqliteCommitFinalizeRequest,
	) -> Result<protocol::SqliteCommitFinalizeResponse> {
		self.handle
			.sqlite_commit_finalize(request)
			.await
			.map_err(map_commit_error)
	}
}
