use anyhow::Result;
use gas::prelude::*;
use rivet_envoy_protocol as protocol;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::Instrument;

use crate::{
	conn::Conn,
	ws_to_tunnel_task::{self, TASK_IDLE_TIMEOUT, TaskExit},
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) struct Key {
	actor_id: String,
	generation: u64,
}

impl Key {
	pub(super) fn new(actor_id: String, generation: u64) -> Self {
		Self {
			actor_id,
			generation,
		}
	}
}

#[derive(Debug)]
pub(super) enum Message {
	GetPages(protocol::ToRivetSqliteGetPagesRequest),
	Commit(protocol::ToRivetSqliteCommitRequest),
}

pub(super) async fn task(
	ctx: StandaloneCtx,
	conn: Arc<Conn>,
	key: Key,
	mut rx: mpsc::UnboundedReceiver<Message>,
) -> Result<TaskExit> {
	loop {
		match tokio::time::timeout(TASK_IDLE_TIMEOUT, rx.recv()).await {
			Ok(Some(Message::GetPages(req))) => {
				let response =
					ws_to_tunnel_task::handle_sqlite_get_pages_response(&ctx, &conn, req.data)
						.await;
				ws_to_tunnel_task::send_sqlite_get_pages_response(&conn, req.request_id, response)
					.await?;
			}
			Ok(Some(Message::Commit(req))) => {
				let actor_id = req.data.actor_id.clone();
				let request_id = req.request_id;
				let timed_response = async {
					ws_to_tunnel_task::handle_sqlite_commit_response(&ctx, &conn, req.data).await
				}
				.instrument(tracing::debug_span!(
					"handle_sqlite_commit",
					actor_id = %actor_id,
					request_id = ?request_id
				))
				.await;
				ws_to_tunnel_task::send_sqlite_commit_response(
					&conn,
					request_id,
					timed_response.response,
				)
				.await?;
				crate::metrics::SQLITE_COMMIT_ENVOY_RESPONSE_DURATION
					.with_label_values(&[
						conn.namespace_id.to_string().as_str(),
						conn.pool_name.as_str(),
					])
					.observe(timed_response.commit_completed_at.elapsed().as_secs_f64());
			}
			Ok(None) | Err(_) => return Ok(TaskExit::SqlitePage(key)),
		}
	}
}
