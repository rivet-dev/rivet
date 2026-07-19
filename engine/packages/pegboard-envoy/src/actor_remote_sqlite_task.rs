use anyhow::Result;
use gas::prelude::*;
use rivet_envoy_protocol as protocol;
use std::sync::Arc;
use tokio::sync::mpsc;

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
	Exec(protocol::ToRivetSqliteExecRequest),
	Execute(protocol::ToRivetSqliteExecuteRequest),
	ExecuteBatch(protocol::ToRivetSqliteExecuteBatchRequest),
}

pub(super) async fn task(
	ctx: StandaloneCtx,
	conn: Arc<Conn>,
	key: Key,
	mut rx: mpsc::UnboundedReceiver<Message>,
) -> Result<TaskExit> {
	loop {
		match tokio::time::timeout(TASK_IDLE_TIMEOUT, rx.recv()).await {
			Ok(Some(Message::Exec(req))) => {
				let response =
					ws_to_tunnel_task::handle_remote_sqlite_exec_response(&ctx, &conn, req.data)
						.await;
				ws_to_tunnel_task::send_sqlite_exec_response(&conn, req.request_id, response)
					.await?;
			}
			Ok(Some(Message::Execute(req))) => {
				let response =
					ws_to_tunnel_task::handle_remote_sqlite_execute_response(&ctx, &conn, req.data)
						.await;
				ws_to_tunnel_task::send_sqlite_execute_response(&conn, req.request_id, response)
					.await?;
			}
			Ok(Some(Message::ExecuteBatch(req))) => {
				let response = ws_to_tunnel_task::handle_remote_sqlite_execute_batch_response(
					&ctx, &conn, req.data,
				)
				.await;
				ws_to_tunnel_task::send_sqlite_execute_batch_response(
					&conn,
					req.request_id,
					response,
				)
				.await?;
			}
			Ok(None) | Err(_) => return Ok(TaskExit::RemoteSqlite(key)),
		}
	}
}
