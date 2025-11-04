use anyhow::Result;
use gas::prelude::*;
use rivet_runner_protocol::{self as protocol, MessageId, PROTOCOL_VERSION, RequestId, versioned};
use scc::{HashMap, hash_map::Entry};
use std::{
	ops::Deref,
	sync::Arc,
	time::{Duration, Instant},
};
use tokio::sync::mpsc;
use universalpubsub::{NextOutput, PubSub, PublishOpts, Subscriber};
use vbare::OwnedVersionedData;

use crate::WebsocketPendingLimitReached;

const GC_INTERVAL: Duration = Duration::from_secs(15);
const MESSAGE_ACK_TIMEOUT: Duration = Duration::from_secs(30);
const MAX_PENDING_MSGS_SIZE_PER_REQ: u64 = util::size::mebibytes(1);

pub enum TunnelMessageData {
	Message(protocol::ToServerTunnelMessageKind),
	Timeout,
}

struct InFlightRequest {
	/// UPS subject to send messages to for this request.
	receiver_subject: String,
	/// Sender for incoming messages to this request.
	msg_tx: mpsc::Sender<TunnelMessageData>,
	/// True once first message for this request has been sent (so runner learned reply_to).
	opened: bool,
	pending_msgs: Vec<PendingMessage>,
	hibernation_state: Option<HibernationState>,
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
	) -> mpsc::Receiver<TunnelMessageData> {
		let (msg_tx, msg_rx) = mpsc::channel(128);

		match self.in_flight_requests.entry_async(request_id).await {
			Entry::Vacant(entry) => {
				entry.insert_entry(InFlightRequest {
					receiver_subject,
					msg_tx,
					opened: false,
					pending_msgs: Vec::new(),
					hibernation_state: None,
				});
			}
			Entry::Occupied(mut entry) => {
				entry.receiver_subject = receiver_subject;
				entry.msg_tx = msg_tx;
				entry.opened = false;
				entry.pending_msgs.clear();
			}
		}

		msg_rx
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
						let _ = in_flight
							.msg_tx
							.send(TunnelMessageData::Message(msg.message_kind))
							.await;

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

	pub async fn resend_pending_websocket_messages(
		&self,
		request_id: RequestId,
		last_msg_index: i64,
	) -> Result<()> {
		let Some(mut req) = self.in_flight_requests.get_async(&request_id).await else {
			bail!("request not in flight");
		};

		let receiver_subject = req.receiver_subject.clone();

		if let Some(hs) = &mut req.hibernation_state {
			if !hs.pending_ws_msgs.is_empty() {
				tracing::debug!(request_id=?Uuid::from_bytes(request_id.clone()), len=?hs.pending_ws_msgs.len(), ?last_msg_index, "resending pending messages");

				let len = hs.pending_ws_msgs.len().try_into()?;

				for (iter_index, pending_msg) in hs.pending_ws_msgs.iter().enumerate() {
					let msg_index = hs
						.last_ws_msg_index
						.wrapping_sub(len)
						.wrapping_add(1)
						.wrapping_add(iter_index.try_into()?);

					if last_msg_index < 0 || wrapping_gt(msg_index, last_msg_index.try_into()?) {
						self.ups
							.publish(&receiver_subject, &pending_msg.payload, PublishOpts::one())
							.await?;
					}
				}

				// Perform ack
				if last_msg_index >= 0 {
					let last_msg_index = last_msg_index.try_into()?;
					let mut iter_index = 0;

					hs.pending_ws_msgs.retain(|_| {
						let msg_index = hs
							.last_ws_msg_index
							.wrapping_sub(len)
							.wrapping_add(1)
							.wrapping_add(iter_index);
						let keep = wrapping_gt(msg_index, last_msg_index);

						iter_index += 1;

						keep
					});

					if hs.pending_ws_msgs.is_empty() {
						hs.last_ws_msg_index = last_msg_index;
					}
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
		#[derive(Debug)]
		enum MsgGcReason {
			/// Any tunnel message not acked (TunnelAck)
			MessageNotAcked,
			/// WebSocket pending messages (ToServerWebSocketMessageAck)
			WebSocketMessageNotAcked,
		}

		let mut interval = tokio::time::interval(GC_INTERVAL);
		interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

		loop {
			interval.tick().await;

			let now = Instant::now();

			self.in_flight_requests
				.retain_async(|request_id, req| {
					if req.msg_tx.is_closed() {
						return false;
					}

					let reason = 'reason: {
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
							"gc collecting in flight request"
						);
						let _ = req.msg_tx.send(TunnelMessageData::Timeout);
					}

					// Return true if the request was not gc'd
					reason.is_none()
				})
				.await;
		}
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
