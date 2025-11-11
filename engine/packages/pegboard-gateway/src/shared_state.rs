use anyhow::Result;
use gas::prelude::*;
use rivet_runner_protocol::{self as protocol, MessageId, PROTOCOL_VERSION, RequestId, versioned};
use scc::{HashMap, hash_map::Entry};
use std::{
	ops::Deref,
	sync::Arc,
	time::{Duration, Instant},
};
use tokio::sync::{mpsc, watch};
use universalpubsub::{NextOutput, PubSub, PublishOpts, Subscriber};
use vbare::OwnedVersionedData;

use crate::WebsocketPendingLimitReached;

const GC_INTERVAL: Duration = Duration::from_secs(15);
const MESSAGE_ACK_TIMEOUT: Duration = Duration::from_secs(30);
const MAX_PENDING_MSGS_SIZE_PER_REQ: u64 = util::size::mebibytes(1);

pub struct InFlightRequestHandle {
	pub msg_rx: mpsc::Receiver<protocol::ToServerTunnelMessageKind>,
	/// Used to check if the request handler has been dropped.
	///
	/// This is separate from `msg_rx` there may still be messages that need to be sent to the
	/// request after `msg_rx` has dropped.
	pub drop_rx: watch::Receiver<()>,
}

struct InFlightRequest {
	/// UPS subject to send messages to for this request.
	receiver_subject: String,
	/// Sender for incoming messages to this request.
	msg_tx: mpsc::Sender<protocol::ToServerTunnelMessageKind>,
	/// Used to check if the request handler has been dropped.
	drop_tx: watch::Sender<()>,
	/// True once first message for this request has been sent (so runner learned reply_to).
	opened: bool,
	pending_msgs: Vec<PendingMessage>,
	hibernation_state: Option<HibernationState>,
	stopping: bool,
}

pub struct PendingMessage {
	message_id: MessageId,
	send_instant: Instant,
}

struct HibernationState {
	total_pending_ws_msgs_size: u64,
	last_ws_msg_index: u16,
	pending_ws_msgs: Vec<PendingWebsocketMessage>,
}

pub struct PendingWebsocketMessage {
	payload: Vec<u8>,
	send_instant: Instant,
}

pub struct SharedStateInner {
	ups: PubSub,
	receiver_subject: String,
	in_flight_requests: HashMap<RequestId, InFlightRequest>,
}

#[derive(Clone)]
pub struct SharedState(Arc<SharedStateInner>);

impl SharedState {
	pub fn new(ups: PubSub) -> Self {
		let gateway_id = Uuid::new_v4();
		let receiver_subject =
			pegboard::pubsub_subjects::GatewayReceiverSubject::new(gateway_id).to_string();

		Self(Arc::new(SharedStateInner {
			ups,
			receiver_subject,
			in_flight_requests: HashMap::new(),
		}))
	}

	pub async fn start(&self) -> Result<()> {
		let sub = self.ups.subscribe(&self.receiver_subject).await?;

		let self_clone = self.clone();
		tokio::spawn(async move { self_clone.receiver(sub).await });

		let self_clone = self.clone();
		tokio::spawn(async move { self_clone.gc().await });

		Ok(())
	}

	pub async fn start_in_flight_request(
		&self,
		receiver_subject: String,
		request_id: RequestId,
	) -> InFlightRequestHandle {
		let (msg_tx, msg_rx) = mpsc::channel(128);
		let (drop_tx, drop_rx) = watch::channel(());

		match self.in_flight_requests.entry_async(request_id).await {
			Entry::Vacant(entry) => {
				entry.insert_entry(InFlightRequest {
					receiver_subject,
					msg_tx,
					drop_tx,
					opened: false,
					pending_msgs: Vec::new(),
					hibernation_state: None,
					stopping: false,
				});
			}
			Entry::Occupied(mut entry) => {
				entry.receiver_subject = receiver_subject;
				entry.msg_tx = msg_tx;
				entry.drop_tx = drop_tx;
				entry.opened = false;
				entry.pending_msgs.clear();

				if entry.stopping {
					entry.hibernation_state = None;
					entry.stopping = false;
				}
			}
		}

		InFlightRequestHandle { msg_rx, drop_rx }
	}

	pub async fn send_message(
		&self,
		request_id: RequestId,
		mut message_kind: protocol::ToClientTunnelMessageKind,
	) -> Result<()> {
		let message_id = Uuid::new_v4().as_bytes().clone();

		let mut req = self
			.in_flight_requests
			.get_async(&request_id)
			.await
			.context("request not in flight")?;

		let include_reply_to = !req.opened;
		if include_reply_to {
			// Mark as opened so subsequent messages skip reply_to
			req.opened = true;
		}

		let ws_msg_index =
			if let (Some(hs), protocol::ToClientTunnelMessageKind::ToClientWebSocketMessage(msg)) =
				(&req.hibernation_state, &mut message_kind)
			{
				// TODO: This ends up skipping 0 as an index when initiated but whatever
				msg.index = hs.last_ws_msg_index.wrapping_add(1);

				Some(msg.index)
			} else {
				None
			};

		let payload = protocol::ToClientTunnelMessage {
			request_id: request_id.clone(),
			message_id,
			// Only send reply to subject on the first message for this request. This reduces
			// overhead of subsequent messages.
			gateway_reply_to: if include_reply_to {
				Some(self.receiver_subject.clone())
			} else {
				None
			},
			message_kind,
		};

		let now = Instant::now();
		req.pending_msgs.push(PendingMessage {
			message_id,
			send_instant: now,
		});

		// Send message
		let message = protocol::ToClient::ToClientTunnelMessage(payload);
		let message_serialized = versioned::ToClient::wrap_latest(message)
			.serialize_with_embedded_version(PROTOCOL_VERSION)?;

		if let (Some(hs), Some(ws_msg_index)) = (&mut req.hibernation_state, ws_msg_index) {
			hs.total_pending_ws_msgs_size += message_serialized.len() as u64;

			if hs.total_pending_ws_msgs_size > MAX_PENDING_MSGS_SIZE_PER_REQ
				|| hs.pending_ws_msgs.len() >= u16::MAX as usize
			{
				return Err(WebsocketPendingLimitReached {}.build());
			}

			hs.last_ws_msg_index = ws_msg_index;

			let pending_ws_msg = PendingWebsocketMessage {
				payload: message_serialized.clone(),
				send_instant: now,
			};

			hs.pending_ws_msgs.push(pending_ws_msg);
		}

		self.ups
			.publish(
				&req.receiver_subject,
				&message_serialized,
				PublishOpts::one(),
			)
			.await?;

		Ok(())
	}

	async fn receiver(&self, mut sub: Subscriber) {
		while let Ok(NextOutput::Message(msg)) = sub.next().await {
			tracing::trace!(
				payload_len = msg.payload.len(),
				"received message from pubsub"
			);

			match versioned::ToGateway::deserialize_with_embedded_version(&msg.payload) {
				Ok(protocol::ToGateway { message: msg }) => {
					tracing::debug!(
						request_id=?Uuid::from_bytes(msg.request_id),
						message_id=?Uuid::from_bytes(msg.message_id),
						"successfully deserialized message"
					);

					let Some(mut in_flight) =
						self.in_flight_requests.get_async(&msg.request_id).await
					else {
						tracing::debug!(
							request_id=?Uuid::from_bytes(msg.request_id),
							"in flight has already been disconnected"
						);
						continue;
					};

					if let protocol::ToServerTunnelMessageKind::TunnelAck = &msg.message_kind {
						let prev_len = in_flight.pending_msgs.len();

						in_flight
							.pending_msgs
							.retain(|m| m.message_id != msg.message_id);

						if prev_len == in_flight.pending_msgs.len() {
							tracing::warn!(
								"pending message does not exist or ack received after message body"
							)
						}
					} else {
						// Send message to the request handler to emulate the real network action
						tracing::debug!(
							request_id=?Uuid::from_bytes(msg.request_id),
							"forwarding message to request handler"
						);
						let _ = in_flight.msg_tx.send(msg.message_kind).await;

						// Send ack back to runner
						let ups_clone = self.ups.clone();
						let receiver_subject = in_flight.receiver_subject.clone();
						let ack_message = protocol::ToClient::ToClientTunnelMessage(
							protocol::ToClientTunnelMessage {
								request_id: msg.request_id,
								message_id: msg.message_id,
								gateway_reply_to: None,
								message_kind: protocol::ToClientTunnelMessageKind::TunnelAck,
							},
						);
						let ack_message_serialized =
							match versioned::ToClient::wrap_latest(ack_message)
								.serialize_with_embedded_version(PROTOCOL_VERSION)
							{
								Ok(x) => x,
								Err(err) => {
									tracing::error!(?err, "failed to serialize ack");
									continue;
								}
							};
						tokio::spawn(async move {
							if let Err(err) = ups_clone
								.publish(
									&receiver_subject,
									&ack_message_serialized,
									PublishOpts::one(),
								)
								.await
							{
								tracing::warn!(?err, "failed to ack message")
							}
						});
					}
				}
				Err(err) => {
					tracing::error!(?err, "failed to parse message");
				}
			}
		}
	}

	pub async fn toggle_hibernation(&self, request_id: RequestId, enable: bool) -> Result<()> {
		let mut req = self
			.in_flight_requests
			.get_async(&request_id)
			.await
			.context("request not in flight")?;

		match (req.hibernation_state.is_some(), enable) {
			(true, true) => {}
			(true, false) => req.hibernation_state = None,
			(false, true) => {
				req.hibernation_state = Some(HibernationState {
					total_pending_ws_msgs_size: 0,
					last_ws_msg_index: 0,
					pending_ws_msgs: Vec::new(),
				});
			}
			(false, false) => {}
		}

		Ok(())
	}

	pub async fn resend_pending_websocket_messages(&self, request_id: RequestId) -> Result<()> {
		let Some(mut req) = self.in_flight_requests.get_async(&request_id).await else {
			bail!("request not in flight");
		};

		let receiver_subject = req.receiver_subject.clone();

		if let Some(hs) = &mut req.hibernation_state {
			if !hs.pending_ws_msgs.is_empty() {
				tracing::debug!(request_id=?Uuid::from_bytes(request_id.clone()), len=?hs.pending_ws_msgs.len(), "resending pending messages");

				for pending_msg in &hs.pending_ws_msgs {
					self.ups
						.publish(&receiver_subject, &pending_msg.payload, PublishOpts::one())
						.await?;
				}
			}
		}

		Ok(())
	}

	pub async fn ack_pending_websocket_messages(
		&self,
		request_id: RequestId,
		ack_index: u16,
	) -> Result<()> {
		let Some(mut req) = self.in_flight_requests.get_async(&request_id).await else {
			bail!("request not in flight");
		};

		let Some(hs) = &mut req.hibernation_state else {
			tracing::warn!("cannot ack ws messages, hibernation is not enabled");
			return Ok(());
		};

		let len = hs.pending_ws_msgs.len().try_into()?;
		let mut iter_index = 0u16;
		hs.pending_ws_msgs.retain(|_| {
			let msg_index = hs
				.last_ws_msg_index
				.wrapping_sub(len)
				.wrapping_add(1)
				.wrapping_add(iter_index);
			let keep = wrapping_gt(msg_index, ack_index);

			iter_index += 1;

			keep
		});

		Ok(())
	}

	async fn gc(&self) {
		let mut interval = tokio::time::interval(GC_INTERVAL);
		interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

		loop {
			interval.tick().await;

			self.gc_in_flight_requests().await;
		}
	}

	/// This will remove all in flight requests that are cancelled or had an ack timeout.
	///
	/// Purging requests is done in a 2 phase commit in order to ensure that the InFlightRequest is
	/// kept until the ToClientWebSocketClose message has been successfully sent.
	///
	/// If we did not use a 2 phase commit (i.e. a single `retain` for any GC purge), the
	/// InFlightRequest would be removed immediately and the runner would never receive the
	/// ToClientWebSocketClose.
	///
	/// **Phase 1**
	///
	/// 1a. Find requests that need to be purged (either closed by gateway or message acknowledgement took too long)
	/// 1b. Flag the request as `stopping` to prevent re-purging this request in the next GC tick
	/// 1c. Send a `Timeout` message to `msg_tx` which will terminate the task in `handle_websocket`
	/// 1d. Once both tasks terminate, `handle_websocket` sends the `ToClientWebSocketClose` to the in flight request
	/// 1e. `handle_websocket` exits and drops `drop_rx`
	///
	/// **Phase 2**
	///
	/// 2a. Remove all requests where it was flagged as stopping and `drop_rx` has been dropped
	async fn gc_in_flight_requests(&self) {
		#[derive(Debug)]
		enum MsgGcReason {
			/// Gateway channel is closed and there are no pending messages
			GatewayClosed,
			/// Any tunnel message not acked (TunnelAck)
			MessageNotAcked,
			/// WebSocket pending messages (ToServerWebSocketMessageAck)
			WebSocketMessageNotAcked,
		}

		let now = Instant::now();

		// First, check if an in flight req is beyond the timeout for tunnel message ack and websocket
		// message ack
		self.in_flight_requests
			.iter_mut_async(|mut entry| {
				let (request_id, req) = &mut *entry;

				if req.stopping {
					return true;
				}

				let reason = 'reason: {
					// If we have no pending messages of any kind and the channel is closed, remove the
					// in flight req
					if req.msg_tx.is_closed()
						&& req.pending_msgs.is_empty()
						&& req
							.hibernation_state
							.as_ref()
							.map(|hs| hs.pending_ws_msgs.is_empty())
							.unwrap_or(true)
					{
						break 'reason Some(MsgGcReason::GatewayClosed);
					}

					if let Some(earliest_pending_msg) = req.pending_msgs.first() {
						if now.duration_since(earliest_pending_msg.send_instant)
							<= MESSAGE_ACK_TIMEOUT
						{
							break 'reason Some(MsgGcReason::MessageNotAcked);
						}
					}

					if let Some(hs) = &req.hibernation_state
						&& let Some(earliest_pending_ws_msg) = hs.pending_ws_msgs.first()
					{
						if now.duration_since(earliest_pending_ws_msg.send_instant)
							<= MESSAGE_ACK_TIMEOUT
						{
							break 'reason Some(MsgGcReason::WebSocketMessageNotAcked);
						}
					}

					None
				};

				if let Some(reason) = &reason {
					tracing::debug!(
						request_id=?Uuid::from_bytes(*request_id),
						?reason,
						"gc stopping in flight request"
					);

					if req.drop_tx.send(()).is_err() {
						tracing::debug!(request_id=?Uuid::from_bytes(*request_id), "failed to send timeout msg to tunnel");
					}

					// Mark req as stopping to skip this loop next time the gc is run
					req.stopping = true;
				}

				true
			})
			.await;

		self.in_flight_requests
			.retain_async(|request_id, req| {
				// The reason we check for stopping here is because drop_tx could be dropped if we are
				// between websocket retries (we don't want to remove the in flight req in this case).
				// When the websocket reconnects a new channel will be created
				if req.stopping && req.drop_tx.is_closed() {
					tracing::debug!(
						request_id=?Uuid::from_bytes(*request_id),
						"gc removing in flight request"
					);

					return false;
				}

				true
			})
			.await;
	}
}

impl Deref for SharedState {
	type Target = SharedStateInner;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

fn wrapping_gt(a: u16, b: u16) -> bool {
	a != b && a.wrapping_sub(b) < u16::MAX / 2
}

// fn wrapping_lt(a: u16, b: u16) -> bool {
//     b.wrapping_sub(a) < u16::MAX / 2
// }
