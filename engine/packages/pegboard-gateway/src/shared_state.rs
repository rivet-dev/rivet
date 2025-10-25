use anyhow::Result;
use gas::prelude::*;
use rivet_runner_protocol::{self as protocol, PROTOCOL_VERSION, RequestId, versioned};
use scc::HashMap;
use std::{
	ops::Deref,
	sync::Arc,
	time::{Duration, Instant},
};
use tokio::sync::mpsc;
use universalpubsub::{NextOutput, PubSub, PublishOpts, Subscriber};
use vbare::OwnedVersionedData;

use crate::WebsocketPendingLimitReached;

const GC_INTERVAL: Duration = Duration::from_secs(60);
const MESSAGE_ACK_TIMEOUT: Duration = Duration::from_secs(30);
const MAX_PENDING_MSGS_PER_REQ: usize = 1024;

struct InFlightRequest {
	/// UPS subject to send messages to for this request.
	receiver_subject: String,
	/// Sender for incoming messages to this request.
	msg_tx: mpsc::Sender<TunnelMessageData>,
	/// True once first message for this request has been sent (so runner learned reply_to).
	opened: bool,
}

pub struct PendingMessage {
	send_instant: Instant,
	payload: protocol::ToClientTunnelMessage,
}

impl Into<protocol::ToClientTunnelMessage> for PendingMessage {
	fn into(self) -> protocol::ToClientTunnelMessage {
		self.payload
	}
}

pub enum TunnelMessageData {
	Message(protocol::ToServerTunnelMessageKind),
	Timeout,
}

pub struct SharedStateInner {
	ups: PubSub,
	receiver_subject: String,
	requests_in_flight: HashMap<RequestId, InFlightRequest>,
	pending_messages: HashMap<RequestId, Vec<PendingMessage>>,
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
			requests_in_flight: HashMap::new(),
			pending_messages: HashMap::new(),
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

		self.requests_in_flight
			.upsert_async(
				request_id,
				InFlightRequest {
					receiver_subject,
					msg_tx,
					opened: false,
				},
			)
			.await;

		msg_rx
	}

	pub async fn send_message(
		&self,
		request_id: RequestId,
		message_kind: protocol::ToClientTunnelMessageKind,
	) -> Result<()> {
		let message_id = Uuid::new_v4().as_bytes().clone();

		// Get subject and whether this is the first message for this request
		let (tunnel_receiver_subject, include_reply_to) = {
			if let Some(mut req) = self.requests_in_flight.get_async(&request_id).await {
				let receiver_subject = req.receiver_subject.clone();

				let include_reply_to = !req.opened;
				if include_reply_to {
					// Mark as opened so subsequent messages skip reply_to
					req.opened = true;
				}

				(receiver_subject, include_reply_to)
			} else {
				bail!("request not in flight");
			}
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

		// Save pending message
		{
			let pending_msg = PendingMessage {
				send_instant: Instant::now(),
				payload: payload.clone(),
			};
			let mut pending_msgs_by_req_id = self
				.pending_messages
				.entry_async(request_id)
				.await
				.or_insert_with(Vec::new);
			let pending_msgs_by_req_id = pending_msgs_by_req_id.get_mut();

			tracing::info!(l=?pending_msgs_by_req_id.len(), message_id=?Uuid::from_bytes(payload.message_id), request_id=?Uuid::from_bytes(payload.request_id), "new msg -----------");

			if pending_msgs_by_req_id.len() >= MAX_PENDING_MSGS_PER_REQ {
				self.pending_messages.remove_async(&request_id).await;

				return Err(WebsocketPendingLimitReached {
					limit: MAX_PENDING_MSGS_PER_REQ,
				}
				.build());
			}

			pending_msgs_by_req_id.push(pending_msg);
		}

		// Send message
		let message = protocol::ToClient::ToClientTunnelMessage(payload);
		let message_serialized = versioned::ToClient::latest(message)
			.serialize_with_embedded_version(PROTOCOL_VERSION)?;
		self.ups
			.publish(
				&tunnel_receiver_subject,
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
					if let protocol::ToServerTunnelMessageKind::TunnelAck = &msg.message_kind {
						tracing::info!(message_id=?Uuid::from_bytes(msg.message_id), request_id=?Uuid::from_bytes(msg.request_id), "ack -----------");
						// Handle ack message
						if let Some(mut pending_msgs) =
							self.pending_messages.get_async(&msg.request_id).await
						{
							pending_msgs.retain(|m| m.payload.message_id != msg.message_id);
						} else {
							tracing::warn!(
								"pending message does not exist or ack received after message body"
							)
						};
					} else {
						// Send message to the request handler to emulate the real network action
						let Some(in_flight) =
							self.requests_in_flight.get_async(&msg.request_id).await
						else {
							tracing::debug!(
								?msg.request_id,
								"in flight has already been disconnected"
							);
							continue;
						};
						tracing::debug!(
							?msg.request_id,
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
								message_id: Uuid::new_v4().into_bytes(),
								gateway_reply_to: None,
								message_kind: protocol::ToClientTunnelMessageKind::TunnelAck,
							},
						);
						let ack_message_serialized = match versioned::ToClient::latest(ack_message)
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

	pub async fn send_reclaimed_messages(&self, request_id: RequestId) -> Result<()> {
		let receiver_subject =
			if let Some(req) = self.requests_in_flight.get_async(&request_id).await {
				req.receiver_subject.clone()
			} else {
				bail!("request not in flight");
			};

		// When a request is started again, read all of its pending messages and send them to the new receiver
		let Some(entry) = self.pending_messages.get_async(&request_id).await else {
			return Ok(());
		};
		let reclaimed_pending_msgs = entry.get();

		if !reclaimed_pending_msgs.is_empty() {
			tracing::debug!(request_id=?Uuid::from_bytes(request_id.clone()), "resending pending messages");

			for pending_msg in reclaimed_pending_msgs {
				// Send message
				let message =
					protocol::ToClient::ToClientTunnelMessage(pending_msg.payload.clone());
				let message_serialized = versioned::ToClient::latest(message)
					.serialize_with_embedded_version(PROTOCOL_VERSION)?;
				self.ups
					.publish(&receiver_subject, &message_serialized, PublishOpts::one())
					.await?;
			}
		}

		Ok(())
	}

	async fn gc(&self) {
		let mut interval = tokio::time::interval(GC_INTERVAL);
		loop {
			interval.tick().await;

			let now = Instant::now();

			// Purge unacked messages
			let mut expired_req_ids = Vec::new();
			self.pending_messages
				.retain_async(|request_id, pending_msgs| {
					if let Some(pending_msg) = pending_msgs.first() {
						if now.duration_since(pending_msg.send_instant) > MESSAGE_ACK_TIMEOUT {
							// Expired
							expired_req_ids.push(request_id.clone());
							false
						} else {
							true
						}
					} else {
						false
					}
				})
				.await;

			// Close in-flight requests for expired messages
			for request_id in expired_req_ids {
				if let Some(x) = self.requests_in_flight.get_async(&request_id).await {
					let _ = x.msg_tx.send(TunnelMessageData::Timeout);
				} else {
					tracing::debug!(
						request_id=?Uuid::from_bytes(request_id),
						"message expired for in flight that does not exist"
					);
				}
			}

			// Purge no longer in flight
			self.requests_in_flight
				.retain_async(|_k, v| !v.msg_tx.is_closed())
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
