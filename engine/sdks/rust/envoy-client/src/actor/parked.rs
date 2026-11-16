//! What an actor sends on a hibernated WebSocket that has no tunnel yet.
//!
//! A connection restored at actor start is parked until the gateway re-opens its tunnel. What
//! the actor sends meanwhile is held in order and goes out behind the open. The gateway may
//! never re-open: the actor was woken by an alarm or through another gateway node, the gateway
//! restarted, or the client is gone. So a parked connection holds a bounded amount, by message
//! count and by bytes. An actor that sends more loses the connection: what was held is dropped,
//! the actor is told the connection closed, and a later re-open is answered with the same
//! close, so the client reconnects and reads fresh state instead of a stream with a hole in it.

use std::collections::VecDeque;

use rivet_envoy_protocol as protocol;
use tokio::sync::{mpsc, oneshot};

use super::ToActor;
use crate::websocket::WsOutgoing;

/// Close code of a parked connection that held more than its limits allow.
pub(super) const OVERFLOW_CLOSE_CODE: u16 = 1011;

/// Close reason of a parked connection that held more than its limits allow.
pub(super) const OVERFLOW_CLOSE_REASON: &str = "ws.parked_overflow";

/// The most one parked connection holds.
#[derive(Clone, Copy, Debug)]
pub(super) struct ParkedLimits {
	pub(super) messages: usize,
	pub(super) bytes: usize,
}

/// A parked connection's side of its outgoing task.
pub(super) struct ParkedTunnel {
	/// Fires when the gateway re-opened the tunnel. Dropped when the connection ended first.
	pub(super) gate: oneshot::Receiver<()>,
	pub(super) limits: ParkedLimits,
	/// The connection's actor, told when the connection overflows.
	pub(super) actor_tx: mpsc::UnboundedSender<ToActor>,
}

/// How a connection left the parked state.
enum Parked {
	/// The gateway re-opened the tunnel. These go out first, in order.
	Opened(VecDeque<WsOutgoing>),
	/// The actor sent more than the connection may hold. What was held is dropped.
	Overflowed,
	/// The connection ended while parked.
	Ended,
}

/// Holds what the actor sends until the gate fires, and answers flushes at once so no caller
/// waits on a tunnel that may never open.
async fn hold(
	outgoing_rx: &mut mpsc::UnboundedReceiver<WsOutgoing>,
	mut gate: oneshot::Receiver<()>,
	limits: ParkedLimits,
) -> Parked {
	let mut held = VecDeque::new();
	let mut held_bytes = 0usize;
	loop {
		tokio::select! {
			biased;
			opened = &mut gate => {
				return match opened {
					Ok(()) => Parked::Opened(held),
					Err(_) => Parked::Ended,
				};
			}
			msg = outgoing_rx.recv() => match msg {
				Some(WsOutgoing::Flush { tx }) => {
					let _ = tx.send(());
				}
				Some(msg) => {
					held_bytes = held_bytes.saturating_add(payload_bytes(&msg));
					held.push_back(msg);
					if held.len() > limits.messages || held_bytes > limits.bytes {
						return Parked::Overflowed;
					}
				}
				None => return Parked::Ended,
			},
		}
	}
}

fn payload_bytes(msg: &WsOutgoing) -> usize {
	match msg {
		WsOutgoing::Message { data, .. } => data.len(),
		WsOutgoing::Close { reason, .. } => reason.as_ref().map_or(0, String::len),
		WsOutgoing::Flush { .. } => 0,
	}
}

/// Runs the parked part of a connection's outgoing task. Returns what to send first once the
/// gateway re-opened the tunnel, or `None` when the connection ended and the task has nothing
/// left to do.
///
/// A connection that overflows tells its actor, which closes it. Until the actor has let go
/// of the connection, what it still sends is dropped and its flushes are answered.
pub(super) async fn park(
	outgoing_rx: &mut mpsc::UnboundedReceiver<WsOutgoing>,
	parked: ParkedTunnel,
	gateway_id: protocol::GatewayId,
	request_id: protocol::RequestId,
) -> Option<VecDeque<WsOutgoing>> {
	match hold(outgoing_rx, parked.gate, parked.limits).await {
		Parked::Opened(held) => Some(held),
		Parked::Ended => None,
		Parked::Overflowed => {
			crate::metrics::METRICS
				.parked_websocket_overflow_total
				.inc();
			tracing::warn!(
				request_id = crate::utils::id_to_str(&request_id),
				max_messages = parked.limits.messages,
				max_bytes = parked.limits.bytes,
				"actor sent more to a hibernated websocket than it may hold before the gateway re-opens it; closing the connection"
			);
			let _ = parked.actor_tx.send(ToActor::WsParkedOverflow {
				gateway_id,
				request_id,
			});
			while let Some(msg) = outgoing_rx.recv().await {
				if let WsOutgoing::Flush { tx } = msg {
					let _ = tx.send(());
				}
			}
			None
		}
	}
}

#[cfg(test)]
#[allow(
	clippy::disallowed_methods,
	reason = "the tests run the parked task on a tokio runtime"
)]
mod tests {
	use super::*;
	use crate::websocket::WebSocketSender;
	use std::time::Duration;

	const GATEWAY_ID: protocol::GatewayId = [1; 4];
	const REQUEST_ID: protocol::RequestId = [2; 4];

	/// One parked connection: the actor's sender, the gate the gateway's re-open fires, the
	/// actor's inbox and the parked half of the outgoing task.
	struct Connection {
		sender: WebSocketSender,
		gate: oneshot::Sender<()>,
		actor_rx: mpsc::UnboundedReceiver<ToActor>,
		task: tokio::task::JoinHandle<Option<VecDeque<WsOutgoing>>>,
	}

	fn parked(messages: usize, bytes: usize) -> Connection {
		let (tx, mut outgoing_rx) = mpsc::unbounded_channel();
		let (gate, gate_rx) = oneshot::channel();
		let (actor_tx, actor_rx) = mpsc::unbounded_channel();
		let task = tokio::spawn(async move {
			park(
				&mut outgoing_rx,
				ParkedTunnel {
					gate: gate_rx,
					limits: ParkedLimits { messages, bytes },
					actor_tx,
				},
				GATEWAY_ID,
				REQUEST_ID,
			)
			.await
		});
		Connection {
			sender: WebSocketSender { tx },
			gate,
			actor_rx,
			task,
		}
	}

	async fn finished<T>(task: tokio::task::JoinHandle<T>) -> T {
		tokio::time::timeout(Duration::from_secs(5), task)
			.await
			.expect("the parked task did not finish")
			.unwrap()
	}

	async fn overflow_notice(actor_rx: &mut mpsc::UnboundedReceiver<ToActor>) {
		let notice = tokio::time::timeout(Duration::from_secs(5), actor_rx.recv())
			.await
			.expect("the actor was not told of the overflow");
		assert!(matches!(
			notice,
			Some(ToActor::WsParkedOverflow {
				gateway_id: GATEWAY_ID,
				request_id: REQUEST_ID,
			})
		));
	}

	fn texts(held: VecDeque<WsOutgoing>) -> Vec<String> {
		held.into_iter()
			.map(|msg| match msg {
				WsOutgoing::Message { data, .. } => String::from_utf8(data).unwrap(),
				_ => panic!("only messages were sent"),
			})
			.collect()
	}

	#[tokio::test]
	async fn what_the_actor_sends_is_held_in_order_until_the_tunnel_opens() {
		let mut conn = parked(4, 1024);
		for text in ["one", "two", "three", "four"] {
			conn.sender.send_text(text);
		}
		// A flush is answered while parked and holds nothing.
		tokio::time::timeout(Duration::from_secs(5), conn.sender.flush())
			.await
			.expect("a flush on a parked connection must not wait for the tunnel");

		conn.gate.send(()).unwrap();
		let held = finished(conn.task).await.expect("the tunnel opened");
		assert_eq!(texts(held), ["one", "two", "three", "four"]);
		assert!(
			conn.actor_rx.try_recv().is_err(),
			"a connection at its limit is not closed"
		);
	}

	/// A ticker fills a parked connection past its message limit. What was held is dropped, the
	/// actor is told once, and everything sent afterwards is dropped too, so the connection
	/// holds nothing for as long as the actor keeps sending on it.
	#[tokio::test]
	async fn a_connection_filled_past_its_message_limit_is_dropped_and_closed() {
		let mut conn = parked(4, 1024);
		for tick in 0..5 {
			conn.sender.send_text(&format!("tick-{tick}"));
		}
		overflow_notice(&mut conn.actor_rx).await;

		// The actor keeps sending until it handles the close. Nothing is held, flushes still
		// return, and a late re-open by the gateway gets nothing from this task.
		for tick in 5..5_000 {
			conn.sender.send_text(&format!("tick-{tick}"));
		}
		tokio::time::timeout(Duration::from_secs(5), conn.sender.flush())
			.await
			.expect("a flush on an overflowed connection must return");
		let _ = conn.gate.send(());
		assert!(conn.actor_rx.try_recv().is_err(), "the actor is told once");

		// The task ends when the actor lets go of the connection.
		drop(conn.sender);
		assert!(finished(conn.task).await.is_none());
	}

	#[tokio::test]
	async fn a_connection_filled_past_its_byte_limit_is_dropped_and_closed() {
		// Exactly the limit is held.
		let at_limit = parked(1024, 16);
		at_limit.sender.send(vec![0; 8], true);
		at_limit.sender.send(vec![0; 8], true);
		at_limit.sender.flush().await;
		at_limit.gate.send(()).unwrap();
		assert_eq!(
			finished(at_limit.task).await.map(|held| held.len()),
			Some(2)
		);

		// One byte more is not, whatever the message count.
		let mut over = parked(1024, 16);
		over.sender.send(vec![0; 8], true);
		over.sender.send(vec![0; 9], true);
		overflow_notice(&mut over.actor_rx).await;
		drop(over.sender);
		assert!(finished(over.task).await.is_none());

		// A single message larger than the limit overflows by itself.
		let mut large = parked(1024, 16);
		large.sender.send(vec![0; 17], true);
		overflow_notice(&mut large.actor_rx).await;
	}

	#[tokio::test]
	async fn a_connection_that_ends_while_parked_tells_no_one() {
		// The actor closed its side.
		let conn = parked(4, 1024);
		conn.sender.send_text("held");
		drop(conn.sender);
		assert!(finished(conn.task).await.is_none());

		// The connection was removed, which drops the gate.
		let mut conn = parked(4, 1024);
		conn.sender.send_text("held");
		drop(conn.gate);
		assert!(finished(conn.task).await.is_none());
		assert!(conn.actor_rx.try_recv().is_err());
	}
}
