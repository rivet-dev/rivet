//! The pre-connect message an engine sends first on every node connection, so a node that opened
//! two connections at once can keep the one whose engine holds fewer nodes.
//!
//! The Kubernetes Service picks an engine once per connection and node connections are
//! long-lived, so without a choice every engine restart pushes its nodes onto the engines still
//! up. A node opens two connections, reads each engine's pre-connect, sends `ToRivetMetadata` on
//! the lighter one and closes the other.
//!
//! A losing connection must cost only its TLS handshake, upgrade and auth. The engine therefore
//! sends the pre-connect and waits for the node's first message before it reads the envoy's
//! record, subscribes to anything or starts any task. A connection that closes in that window
//! ends with no storage I/O and nothing to clean up.
use std::sync::{
	Arc,
	atomic::{AtomicU32, Ordering},
};

use anyhow::{Context, Result};
use futures_util::StreamExt;
use rivet_engine_context::EngineContext;
use rivet_env::capabilities::websocket::{Message, WebSocket};
use rivet_envoy_protocol as p;
use vbare::Versioned;

/// The first envoy protocol version whose envoys expect a pre-connect.
pub(crate) const PRE_CONNECT_PROTOCOL_VERSION: u16 = 10;

/// The node connections this process holds that chose it: those whose node sent a first
/// message after the pre-connect. A connection is counted until it ends.
#[derive(Default)]
pub(crate) struct ConnectionCount(AtomicU32);

/// Counts one connection in a [`ConnectionCount`] while it is held.
pub(crate) struct CountedConnection(Arc<ConnectionCount>);

impl ConnectionCount {
	pub(crate) fn count(self: &Arc<Self>) -> CountedConnection {
		self.0.fetch_add(1, Ordering::Relaxed);
		CountedConnection(self.clone())
	}

	pub(crate) fn get(&self) -> u32 {
		self.0.load(Ordering::Relaxed)
	}
}

impl Drop for CountedConnection {
	fn drop(&mut self) {
		self.0.0.fetch_sub(1, Ordering::Relaxed);
	}
}

/// Sends the pre-connect to a node at `version`. Envoys older than
/// [`PRE_CONNECT_PROTOCOL_VERSION`] cannot decode it and get nothing. Returns false when the
/// node already closed the connection, as it does for one it dialed and then dropped.
pub(crate) async fn send(
	ctx: &EngineContext,
	socket: &mut WebSocket,
	version: u16,
	connections: &ConnectionCount,
) -> Result<bool> {
	if version < PRE_CONNECT_PROTOCOL_VERSION {
		return Ok(true);
	}

	let message = p::ToEnvoy::ToEnvoyPreConnect(p::ToEnvoyPreConnect {
		worker_id: *ctx.ups()?.worker_id().as_bytes(),
		envoy_connections: connections.get(),
	});
	let payload = p::versioned::ToEnvoy::serialize(&message, version)?;
	if let Err(error) = socket.send(Message::Binary(payload.into())).await {
		tracing::debug!(?error, "node closed its connection before the pre-connect");
		return Ok(false);
	}

	Ok(true)
}

/// Waits for the node's first message after the pre-connect, bounded by the ping timeout.
/// Returns `None` when the node closes the connection, sends nothing in time or the process
/// starts stopping first: a node that chose another engine closes this one without sending
/// anything.
pub(crate) async fn first_message(
	ctx: &EngineContext,
	socket: &mut WebSocket,
	stopping: tokio::sync::watch::Receiver<bool>,
) -> Result<Option<Message>> {
	let timeout_ms = u64::try_from(ctx.config().pegboard().envoy_ping_timeout())
		.context("pegboard.node_ping_timeout is negative")?;
	let clock = ctx.env().clock();

	await_first_message(socket, clock.as_ref(), timeout_ms, stopping).await
}

/// See [`first_message`].
async fn await_first_message<S, E>(
	socket: &mut S,
	clock: &dyn rivet_env::Clock,
	timeout_ms: u64,
	mut stopping: tokio::sync::watch::Receiver<bool>,
) -> Result<Option<Message>>
where
	S: futures_util::Stream<Item = std::result::Result<Message, E>> + Unpin,
	E: std::fmt::Debug,
{
	let wait = async {
		loop {
			match socket.next().await {
				Some(Ok(message @ Message::Binary(_))) => return Some(message),
				Some(Ok(Message::Close(_))) | None => return None,
				Some(Ok(Message::Text(_) | Message::Ping(_) | Message::Pong(_))) => {}
				Some(Err(error)) => {
					tracing::debug!(?error, "node connection failed before its first message");
					return None;
				}
			}
		}
	};
	let deadline = std::time::Duration::from_millis(timeout_ms);
	let first = rivet_env::select_biased!("stop waiting for a node that has not chosen this engine",
		_ = stopping.wait_for(|stopping| *stopping) => return Ok(None),
		first = rivet_env::timeout(clock, deadline, wait) => first,
	);

	match first {
		Ok(first) => Ok(first),
		Err(rivet_env::TimeoutError::Elapsed) => {
			tracing::debug!(
				timeout_ms,
				"node sent nothing after the pre-connect; closing its connection"
			);
			Ok(None)
		}
		Err(rivet_env::TimeoutError::Clock(error)) => {
			Err(error).context("clock failed while waiting for the node's first message")
		}
	}
}
