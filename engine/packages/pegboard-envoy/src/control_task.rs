use anyhow::Result;
use gas::prelude::*;
use rivet_envoy_protocol as protocol;
use std::sync::Arc;
use tokio::sync::mpsc;

use crate::{
	conn::Conn,
	ws_to_tunnel_task::{self, TASK_IDLE_TIMEOUT, TaskExit},
};

#[derive(Debug)]
pub(super) enum Message {
	Metadata(protocol::ToRivetMetadata),
	AckCommands(protocol::ToRivetAckCommands),
}

pub(super) async fn task(
	ctx: StandaloneCtx,
	conn: Arc<Conn>,
	mut rx: mpsc::UnboundedReceiver<Message>,
) -> Result<TaskExit> {
	loop {
		match tokio::time::timeout(TASK_IDLE_TIMEOUT, rx.recv()).await {
			Ok(Some(Message::Metadata(metadata))) => {
				ws_to_tunnel_task::handle_metadata(
					&ctx,
					conn.namespace_id,
					&conn.envoy_key,
					metadata,
				)
				.await?;
			}
			Ok(Some(Message::AckCommands(ack))) => {
				ws_to_tunnel_task::ack_commands(&ctx, conn.namespace_id, &conn.envoy_key, ack)
					.await?;
			}
			Ok(None) | Err(_) => return Ok(TaskExit::Control),
		}
	}
}
