use anyhow::{Context, Result};
use gas::prelude::*;
use rivet_envoy_protocol as protocol;
use std::{fmt, sync::Arc};
use tokio::sync::mpsc;

use crate::{
	conn::Conn,
	ws_to_tunnel_task::{self, TASK_IDLE_TIMEOUT, TaskExit},
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) struct Key {
	gateway_id: protocol::GatewayId,
	request_id: protocol::RequestId,
}

impl Key {
	pub(super) fn new(gateway_id: protocol::GatewayId, request_id: protocol::RequestId) -> Self {
		Self {
			gateway_id,
			request_id,
		}
	}

	pub(super) fn gateway_id_display(&self) -> DisplayId<'_> {
		display_id(&self.gateway_id)
	}

	pub(super) fn request_id_display(&self) -> DisplayId<'_> {
		display_id(&self.request_id)
	}
}

pub(super) fn display_id(id: &[u8]) -> DisplayId<'_> {
	DisplayId(id)
}

#[derive(Clone, Copy)]
pub(super) struct DisplayId<'a>(&'a [u8]);

impl fmt::Display for DisplayId<'_> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		for byte in self.0 {
			write!(f, "{byte:02x}")?;
		}
		Ok(())
	}
}

#[derive(Debug)]
pub(super) enum Message {
	Message(protocol::ToRivetTunnelMessage),
}

pub(super) async fn task(
	ctx: StandaloneCtx,
	conn: Arc<Conn>,
	key: Key,
	mut rx: mpsc::UnboundedReceiver<Message>,
) -> Result<TaskExit> {
	loop {
		match tokio::time::timeout(TASK_IDLE_TIMEOUT, rx.recv()).await {
			Ok(Some(Message::Message(msg))) => {
				ws_to_tunnel_task::handle_tunnel_message(&ctx, &conn, msg)
					.await
					.context("failed to handle tunnel message")?;
			}
			Ok(None) | Err(_) => return Ok(TaskExit::Tunnel(key)),
		}
	}
}
