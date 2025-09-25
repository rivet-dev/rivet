use anyhow::Result;
use gas::prelude::*;
use rivet_runner_protocol::{self as protocol, MessageId, PROTOCOL_VERSION, RequestId, versioned};
use std::{
	collections::HashMap,
	ops::Deref,
	sync::Arc,
	time::{Duration, Instant},
};
use tokio::sync::{Mutex, mpsc};
use universalpubsub::{NextOutput, PubSub, PublishOpts, Subscriber};
use versioned_data_util::OwnedVersionedData as _;

const GC_INTERVAL: Duration = Duration::from_secs(60);
const MESSAGE_ACK_TIMEOUT: Duration = Duration::from_secs(5);

struct InFlightRequest {
	/// UPS subject to send messages to for this request.
	receiver_subject: String,
	/// Sender for incoming messages to this request.
	msg_tx: mpsc::Sender<TunnelMessageData>,
	/// True once first message for this request has been sent (so runner learned reply_to).
	opened: bool,
}

struct PendingMessage {
	request_id: RequestId,
	send_instant: Instant,
}

pub enum TunnelMessageData {
	Message(protocol::ToServerTunnelMessageKind),
	Timeout,
}

pub struct SharedStateInner {
	ups: PubSub,
	receiver_subject: String,
	requests_in_flight: Mutex<HashMap<RequestId, InFlightRequest>>,
	pending_messages: Mutex<HashMap<MessageId, PendingMessage>>,
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
			requests_in_flight: Mutex::new(HashMap::new()),
			pending_messages: Mutex::new(HashMap::new()),
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

	pub async fn send_message(
		&self,
		request_id: RequestId,
		message_kind: protocol::ToClientTunnelMessageKind,
	) -> Result<()> {
		let message_id = Uuid::new_v4().as_bytes().clone();

		// Get subject and whether this is the first message for this request
		let (tunnel_receiver_subject, include_reply_to) = {
			let mut requests_in_flight = self.requests_in_flight.lock().await;
			if let Some(req) = requests_in_flight.get_mut(&request_id) {
				let receiver_subject = req.receiver_subject.clone();
				let include_reply_to = !req.opened;
				if include_reply_to {
					// Mark as opened so subsequent messages skip reply_to
					req.opened = true;
				}
				(receiver_subject, include_reply_to)
			} else {
				bail!("request not in flight")
			}
		};

		// Save pending message
		{
			let mut pending_messages = self.pending_messages.lock().await;
			pending_messages.insert(
				message_id,
				PendingMessage {
					request_id,
					send_instant: Instant::now(),
				},
			);
		}

		// Send message
		let message = protocol::ToClient::ToClientTunnelMessage(protocol::ToClientTunnelMessage {
			request_id,
			message_id,
			// Only send reply to subject on the first message for this request. This reduces
			// overhead of subsequent messages.
			gateway_reply_to: if include_reply_to {
				Some(self.receiver_subject.clone())
			} else {
				None
			},
			message_kind,
		});
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

	pub async fn start_in_flight_request(
		&self,
		receiver_subject: String,
	) -> (RequestId, mpsc::Receiver<TunnelMessageData>) {
		let id = Uuid::new_v4().into_bytes();
		let (msg_tx, msg_rx) = mpsc::channel(128);
		self.requests_in_flight.lock().await.insert(
			id,
			InFlightRequest {
				receiver_subject,
				msg_tx,
				opened: false,
			},
		);
		(id, msg_rx)
	}

	async fn receiver(&self, mut sub: Subscriber) {
		while let Ok(NextOutput::Message(msg)) = sub.next().await {
			tracing::info!(
				payload_len = msg.payload.len(),
				"received message from pubsub"
			);

			match versioned::ToGateway::deserialize_with_embedded_version(&msg.payload) {
				Ok(protocol::ToGateway { message: msg }) => {
					tracing::debug!(
						?msg.request_id,
						?msg.message_id,
						"successfully deserialized message"
					);
					if let protocol::ToServerTunnelMessageKind::TunnelAck = &msg.message_kind {
						// Handle ack message

						let mut pending_messages = self.pending_messages.lock().await;
						if pending_messages.remove(&msg.message_id).is_none() {
							tracing::warn!(
								"pending message does not exist or ack received after message body"
							);
						}
					} else {
						// Send message to the request handler to emulate the real network action
						let requests_in_flight = self.requests_in_flight.lock().await;
						let Some(in_flight) = requests_in_flight.get(&msg.request_id) else {
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

	async fn gc(&self) {
		let mut interval = tokio::time::interval(GC_INTERVAL);
		loop {
			interval.tick().await;

			let now = Instant::now();

			// Purge unacked messages
			{
				let mut pending_messages = self.pending_messages.lock().await;
				let mut removed_req_ids = Vec::new();
				pending_messages.retain(|_k, v| {
					if now.duration_since(v.send_instant) > MESSAGE_ACK_TIMEOUT {
						// Expired
						removed_req_ids.push(v.request_id.clone());
						false
					} else {
						true
					}
				});

				// Close in-flight messages
				let requests_in_flight = self.requests_in_flight.lock().await;
				for req_id in removed_req_ids {
					if let Some(x) = requests_in_flight.get(&req_id) {
						let _ = x.msg_tx.send(TunnelMessageData::Timeout);
					} else {
						tracing::warn!(
							?req_id,
							"message expired for in flight that does not exist"
						);
					}
				}
			}

			// Purge no longer in flight
			{
				let mut requests_in_flight = self.requests_in_flight.lock().await;
				requests_in_flight.retain(|_k, v| !v.msg_tx.is_closed());
			}
		}
	}
}

impl Deref for SharedState {
	type Target = SharedStateInner;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}
