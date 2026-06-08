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
}

impl Key {
	pub(super) fn new(actor_id: String) -> Self {
		Self { actor_id }
	}
}

#[derive(Debug)]
pub(super) enum Message {
	Request(protocol::ToRivetKvRequest),
}

pub(super) async fn task(
	ctx: StandaloneCtx,
	conn: Arc<Conn>,
	key: Key,
	mut rx: mpsc::UnboundedReceiver<Message>,
) -> Result<TaskExit> {
	loop {
		match tokio::time::timeout(TASK_IDLE_TIMEOUT, rx.recv()).await {
			Ok(Some(Message::Request(req))) => {
				ws_to_tunnel_task::handle_kv_request(&ctx, &conn, req).await?;
			}
			Ok(None) | Err(_) => return Ok(TaskExit::Kv(key)),
		}
	}
}
