//! In-process page transport for remote SQL: the actor VFS reads and commits pages through the
//! same page protocol the Envoy page path uses, on the lane's database and stage.
use super::Pages;
use anyhow::{Result, bail, ensure};
use async_trait::async_trait;
use rivet_envoy_protocol as p;
use std::sync::{
	Arc, Weak,
	atomic::{AtomicBool, AtomicU32, Ordering},
};
use tokio::sync::Mutex;

/// Serves one remote SQL session's page requests. The caller of remote SQL holds the actor lane
/// for the whole statement, so page requests never race the page path. They reach the lane's
/// pages through their own lock, never the lane lock the caller holds.
pub(super) struct LaneTransport {
	env: Arc<dyn rivet_env::Env>,
	pages: Arc<Mutex<Pages>>,
	lane: Weak<super::Lane>,
	/// The generation the session was opened for. Requests stamped with another generation are
	/// rejected, so a session that outlived its generation cannot touch the database.
	generation: u32,
	next_request_id: AtomicU32,
	/// An undecided page commit invalidates SQLite's rollback state, even while the
	/// filesystem retains its writer claim and can resolve the write by reading it back.
	unknown_commit: Arc<AtomicBool>,
}

impl LaneTransport {
	pub(super) fn new(
		env: Arc<dyn rivet_env::Env>,
		pages: Arc<Mutex<Pages>>,
		lane: Weak<super::Lane>,
		generation: u32,
		unknown_commit: Arc<AtomicBool>,
	) -> Self {
		Self {
			env,
			pages,
			lane,
			generation,
			next_request_id: AtomicU32::new(0),
			unknown_commit,
		}
	}

	/// Checks the request's generation and the lane, then runs one page protocol message.
	async fn execute(
		&self,
		expected_generation: Option<u64>,
		message: p::ToRivet,
	) -> Result<p::ToEnvoy> {
		ensure!(
			expected_generation == Some(u64::from(self.generation)),
			"SQLite request for another generation"
		);
		let Some(lane) = self.lane.upgrade() else {
			bail!("SQLite session ended");
		};
		lane.check()?;
		ensure!(
			!self.unknown_commit.load(Ordering::Acquire),
			super::IndeterminateResult
		);
		let mut pages = self.pages.lock().await;
		ensure!(pages.generation == self.generation, "SQLite session ended");
		let Pages {
			database, stage, ..
		} = &mut *pages;
		let Some(database) = database.as_mut() else {
			bail!("SQLite database is not open");
		};
		let guard = lane.guard();
		database.set_server_prefetch((lane.server_prefetch)());
		let reply = super::execute_pages(&self.env, guard.as_ref(), stage, database, message).await;
		if let Err(error) = &reply
			&& let Some(depot_client::filesystem::Error::Log(error)) =
				error.downcast_ref::<depot_client::filesystem::Error>()
			&& error.disposition()
				== depot_volume::volume_client_machine::types::ErrorDisposition::OutcomeUnknown
		{
			// SQLite must not issue rollback writes or reuse cached pages after a commit
			// whose outcome it cannot determine. Keep that uncertainty local to this session.
			self.unknown_commit.store(true, Ordering::Release);
		}
		lane.check()?;
		reply
	}

	fn request_id(&self) -> u32 {
		self.next_request_id.fetch_add(1, Ordering::Relaxed)
	}
}

#[async_trait]
impl sqlite_embedded::vfs::SqliteTransport for LaneTransport {
	async fn get_pages(
		&self,
		request: p::SqliteGetPagesRequest,
	) -> Result<p::SqliteGetPagesResponse> {
		let generation = request.expected_generation;
		let message = p::ToRivet::ToRivetSqliteGetPagesRequest(p::ToRivetSqliteGetPagesRequest {
			request_id: self.request_id(),
			data: request,
		});
		let p::ToEnvoy::ToEnvoySqliteGetPagesResponse(response) =
			self.execute(generation, message).await?
		else {
			bail!("unexpected SQLite page response");
		};
		Ok(response.data)
	}

	async fn commit(&self, request: p::SqliteCommitRequest) -> Result<p::SqliteCommitResponse> {
		let generation = request.expected_generation;
		let message = p::ToRivet::ToRivetSqliteCommitRequest(p::ToRivetSqliteCommitRequest {
			request_id: self.request_id(),
			data: request,
		});
		let p::ToEnvoy::ToEnvoySqliteCommitResponse(response) =
			self.execute(generation, message).await?
		else {
			bail!("unexpected SQLite commit response");
		};
		Ok(response.data)
	}

	async fn commit_stage_begin(
		&self,
		request: p::SqliteCommitStageBeginRequest,
	) -> Result<p::SqliteCommitStageBeginResponse> {
		let generation = request.expected_generation;
		let message = p::ToRivet::ToRivetSqliteCommitStageBeginRequest(
			p::ToRivetSqliteCommitStageBeginRequest {
				request_id: self.request_id(),
				data: request,
			},
		);
		let p::ToEnvoy::ToEnvoySqliteCommitStageBeginResponse(response) =
			self.execute(generation, message).await?
		else {
			bail!("unexpected SQLite stage response");
		};
		Ok(response.data)
	}

	async fn commit_stage_segment(
		&self,
		request: p::SqliteCommitStageSegmentRequest,
	) -> Result<p::SqliteCommitStageSegmentResponse> {
		let generation = request.expected_generation;
		let message = p::ToRivet::ToRivetSqliteCommitStageSegmentRequest(
			p::ToRivetSqliteCommitStageSegmentRequest {
				request_id: self.request_id(),
				data: request,
			},
		);
		let p::ToEnvoy::ToEnvoySqliteCommitStageSegmentResponse(response) =
			self.execute(generation, message).await?
		else {
			bail!("unexpected SQLite stage response");
		};
		Ok(response.data)
	}

	async fn commit_finalize(
		&self,
		request: p::SqliteCommitFinalizeRequest,
	) -> Result<p::SqliteCommitFinalizeResponse> {
		let generation = request.expected_generation;
		let message =
			p::ToRivet::ToRivetSqliteCommitFinalizeRequest(p::ToRivetSqliteCommitFinalizeRequest {
				request_id: self.request_id(),
				data: request,
			});
		let p::ToEnvoy::ToEnvoySqliteCommitFinalizeResponse(response) =
			self.execute(generation, message).await?
		else {
			bail!("unexpected SQLite stage response");
		};
		Ok(response.data)
	}
}
