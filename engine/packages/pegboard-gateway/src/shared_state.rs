use anyhow::Result;
use gas::prelude::*;
use pegboard::tunnel::id::{self as tunnel_id, GatewayId, RequestId};
use rivet_guard_core::errors::WebSocketServiceTimeout;
use rivet_runner_protocol::{self as protocol, versioned, PROTOCOL_VERSION};
use scc::{hash_map::Entry, HashMap};
use std::{
	ops::Deref,
	sync::Arc,
	time::{Duration, Instant},
};
use tokio::sync::{mpsc, watch};
use universalpubsub::{NextOutput, PubSub, PublishOpts, Subscriber};
use vbare::OwnedVersionedData;

use crate::{metrics, WebsocketPendingLimitReached};

const GC_INTERVAL: Duration = Duration::from_secs(15);
const TUNNEL_PING_TIMEOUT: i64 = util::duration::seconds(30);
const HWS_MESSAGE_ACK_TIMEOUT: Duration = Duration::from_secs(30);
const HWS_MAX_PENDING_MSGS_SIZE_PER_REQ: u64 = util::size::mebibytes(1);

pub struct InFlightRequestHandle {
	pub msg_rx: mpsc::Receiver<protocol::ToServerTunnelMessageKind>,
	/// Used to check if the request handler has been dropped.
	///
	/// This is separate from `msg_rx` there may still be messages that need to be sent to the
	/// request after `msg_rx` has dropped.
	pub drop_rx: watch::Receiver<()>,
	pub new: bool,
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
	/// Message index counter for this request.
	message_index: tunnel_id::MessageIndex,
	hibernation_state: Option<HibernationState>,
	stopping: bool,
	last_pong: i64,
}

struct HibernationState {
	total_pending_ws_msgs_size: u64,
	pending_ws_msgs: Vec<PendingWebsocketMessage>,
	// Used to keep hibernating websockets from being GC'd
	last_ping: Instant,
}

pub struct PendingWebsocketMessage {
	payload: Vec<u8>,
	send_instant: Instant,
	message_index: tunnel_id::MessageIndex,
}

pub struct SharedStateInner {
	ups: PubSub,
	gateway_id: GatewayId,
	receiver_subject: String,
	in_flight_requests: HashMap<RequestId, InFlightRequest>,
	hibernation_timeout: i64,
}

#[derive(Clone)]
pub struct SharedState(Arc<SharedStateInner>);

impl SharedState {
	pub fn new(config: &rivet_config::Config, ups: PubSub) -> Self {
		let gateway_id = tunnel_id::generate_gateway_id();
		let receiver_subject =
			pegboard::pubsub_subjects::GatewayReceiverSubject::new(gateway_id).to_string();

		Self(Arc::new(SharedStateInner {
			ups,
			gateway_id,
			receiver_subject,
			in_flight_requests: HashMap::new(),
			hibernation_timeout: config.pegboard().hibernating_request_eligible_threshold(),
		}))
	}

	pub fn gateway_id(&self) -> GatewayId {
		self.gateway_id
	}

	#[tracing::instrument(skip_all)]
	pub async fn start(&self) -> Result<()> {
		let sub = self.ups.subscribe(&self.receiver_subject).await?;

		let self_clone = self.clone();
		tokio::spawn(async move { self_clone.receiver(sub).await });

		let self_clone = self.clone();
		tokio::spawn(async move { self_clone.gc().await });

		Ok(())
	}

	#[tracing::instrument(skip_all, fields(%receiver_subject, request_id=%tunnel_id::request_id_to_string(&request_id)))]
	pub async fn start_in_flight_request(
		&self,
		receiver_subject: String,
		request_id: RequestId,
	) -> InFlightRequestHandle {
		let (msg_tx, msg_rx) = mpsc::channel(128);
		let (drop_tx, drop_rx) = watch::channel(());

		let new = match self.in_flight_requests.entry_async(request_id).await {
			Entry::Vacant(entry) => {
				entry.insert_entry(InFlightRequest {
					receiver_subject,
					msg_tx,
					drop_tx,
					opened: false,
					message_index: 0,
					hibernation_state: None,
					stopping: false,
					last_pong: util::timestamp::now(),
				});

				true
			}
			Entry::Occupied(mut entry) => {
				entry.receiver_subject = receiver_subject;
				entry.msg_tx = msg_tx;
				entry.drop_tx = drop_tx;
				entry.opened = false;

				if entry.stopping {
					entry.hibernation_state = None;
					entry.stopping = false;
				}

				false
			}
		};

		InFlightRequestHandle {
			msg_rx,
			drop_rx,
			new,
		}
	}

	#[tracing::instrument(skip_all, fields(request_id=%tunnel_id::request_id_to_string(&request_id)))]
	pub async fn send_message(
		&self,
		request_id: RequestId,
		message_kind: protocol::ToClientTunnelMessageKind,
	) -> Result<()> {
		let mut req = self
			.in_flight_requests
			.get_async(&request_id)
			.await
			.context("request not in flight")?;

		// Generate message ID
		let message_id =
			tunnel_id::build_message_id(self.gateway_id, request_id, req.message_index)?;

		// Increment message index for next message
		let current_message_index = req.message_index;
		req.message_index = req.message_index.wrapping_add(1);

		let include_reply_to = !req.opened;
		if include_reply_to {
			// Mark as opened so subsequent messages skip reply_to
			req.opened = true;
		}

		// Check if this is a WebSocket message for hibernation tracking
		let is_ws_message = matches!(
			message_kind,
			protocol::ToClientTunnelMessageKind::ToClientWebSocketMessage(_)
		);

		let payload = protocol::ToClientTunnelMessage {
			message_id,
			message_kind,
		};

		tracing::debug!(?message_id, ?payload, "shared state send message");

		// Send message
		let message = protocol::ToRunner::ToClientTunnelMessage(payload);
		let message_serialized = versioned::ToRunner::wrap_latest(message)
			.serialize_with_embedded_version(PROTOCOL_VERSION)?;

		if let (Some(hs), true) = (&mut req.hibernation_state, is_ws_message) {
			hs.total_pending_ws_msgs_size += message_serialized.len() as u64;

			if hs.total_pending_ws_msgs_size > HWS_MAX_PENDING_MSGS_SIZE_PER_REQ
				|| hs.pending_ws_msgs.len() >= u16::MAX as usize
			{
				return Err(WebsocketPendingLimitReached {}.build());
			}

			let pending_ws_msg = PendingWebsocketMessage {
				payload: message_serialized.clone(),
				send_instant: Instant::now(),
				message_index: current_message_index,
			};

			hs.pending_ws_msgs.push(pending_ws_msg);
			tracing::debug!(
				index = current_message_index,
				new_count = hs.pending_ws_msgs.len(),
				"pushed pending websocket message"
			);
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

	#[tracing::instrument(skip_all, fields(request_id=%tunnel_id::request_id_to_string(&request_id)))]
	pub async fn send_and_check_ping(&self, request_id: RequestId) -> Result<()> {
		let req = self
			.in_flight_requests
			.get_async(&request_id)
			.await
			.context("request not in flight")?;

		let now = util::timestamp::now();

		// Verify ping timeout
		if now.saturating_sub(req.last_pong) > TUNNEL_PING_TIMEOUT {
			tracing::warn!("tunnel timeout");
			return Err(WebSocketServiceTimeout.build());
		}

		// Send message
		let message = protocol::ToRunner::ToRunnerPing(protocol::ToRunnerPing {
			gateway_id: self.gateway_id,
			request_id,
			ts: now,
		});
		let message_serialized = versioned::ToRunner::wrap_latest(message)
			.serialize_with_embedded_version(PROTOCOL_VERSION)?;

		self.ups
			.publish(
				&req.receiver_subject,
				&message_serialized,
				PublishOpts::one(),
			)
			.await?;

		Ok(())
	}

	#[tracing::instrument(skip_all, fields(request_id=%tunnel_id::request_id_to_string(&request_id)))]
	pub async fn keepalive_hws(&self, request_id: RequestId) -> Result<()> {
		let mut req = self
			.in_flight_requests
			.get_async(&request_id)
			.await
			.context("request not in flight")?;

		if let Some(hs) = &mut req.hibernation_state {
			hs.last_ping = Instant::now();
		} else {
			tracing::warn!("should not call keepalive_hws for non-hibernating ws");
		}

		Ok(())
	}

	#[tracing::instrument(skip_all)]
	async fn receiver(&self, mut sub: Subscriber) {
		while let Ok(NextOutput::Message(msg)) = sub.next().await {
			tracing::trace!(
				payload_len = msg.payload.len(),
				"received message from pubsub"
			);

			match versioned::ToGateway::deserialize_with_embedded_version(&msg.payload) {
				Ok(protocol::ToGateway::ToGatewayPong(pong)) => {
					let Some(mut in_flight) =
						self.in_flight_requests.get_async(&pong.request_id).await
					else {
						tracing::debug!(
							request_id=%tunnel_id::request_id_to_string(&pong.request_id),
							"in flight has already been disconnected, dropping ping"
						);
						continue;
					};

					let now = util::timestamp::now();
					in_flight.last_pong = now;

					let rtt = now.saturating_sub(pong.ts);
					metrics::TUNNEL_PING_DURATION.record(rtt as f64 * 0.001, &[]);
				}
				Ok(protocol::ToGateway::ToServerTunnelMessage(msg)) => {
					// Parse message ID to extract components
					let parts = match tunnel_id::parse_message_id(msg.message_id) {
						Ok(p) => p,
						Err(err) => {
							tracing::error!(?err, message_id=?msg.message_id, "failed to parse message id");
							continue;
						}
					};

					let Some(in_flight) =
						self.in_flight_requests.get_async(&parts.request_id).await
					else {
						tracing::warn!(
							gateway_id=%tunnel_id::gateway_id_to_string(&parts.gateway_id),
							request_id=%tunnel_id::request_id_to_string(&parts.request_id),
							message_index=parts.message_index,
							"in flight has already been disconnected, dropping message"
						);
						continue;
					};

					// Send message to the request handler to emulate the real network action
					tracing::debug!(
						gateway_id=%tunnel_id::gateway_id_to_string(&parts.gateway_id),
						request_id=%tunnel_id::request_id_to_string(&parts.request_id),
						message_index=parts.message_index,
						"forwarding message to request handler"
					);
					let _ = in_flight.msg_tx.send(msg.message_kind.clone()).await;
				}
				Err(err) => {
					tracing::error!(?err, "failed to parse message");
				}
			}
		}
	}

	#[tracing::instrument(skip_all, fields(request_id=%tunnel_id::request_id_to_string(&request_id), %enable))]
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
					pending_ws_msgs: Vec::new(),
					last_ping: Instant::now(),
				});
			}
			(false, false) => {}
		}

		Ok(())
	}

	#[tracing::instrument(skip_all, fields(request_id=%tunnel_id::request_id_to_string(&request_id)))]
	pub async fn resend_pending_websocket_messages(&self, request_id: RequestId) -> Result<()> {
		let Some(mut req) = self.in_flight_requests.get_async(&request_id).await else {
			bail!("request not in flight");
		};

		let receiver_subject = req.receiver_subject.clone();

		if let Some(hs) = &mut req.hibernation_state {
			if !hs.pending_ws_msgs.is_empty() {
				tracing::debug!(len=?hs.pending_ws_msgs.len(), "resending pending messages");

				for pending_msg in &hs.pending_ws_msgs {
					tracing::info!(?pending_msg.payload, ?pending_msg.message_index, "------2---------");

					self.ups
						.publish(&receiver_subject, &pending_msg.payload, PublishOpts::one())
						.await?;
				}
			}
		}

		Ok(())
	}

	#[tracing::instrument(skip_all, fields(request_id=%tunnel_id::request_id_to_string(&request_id)))]
	pub async fn has_pending_websocket_messages(&self, request_id: RequestId) -> Result<bool> {
		let Some(req) = self.in_flight_requests.get_async(&request_id).await else {
			bail!("request not in flight");
		};

		if let Some(hs) = &req.hibernation_state {
			Ok(!hs.pending_ws_msgs.is_empty())
		} else {
			Ok(false)
		}
	}

	#[tracing::instrument(skip_all, fields(request_id=%tunnel_id::request_id_to_string(&request_id), %ack_index))]
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

		// Retain messages with index > ack_index (messages that haven't been acknowledged yet)
		let len_before = hs.pending_ws_msgs.len();
		hs.pending_ws_msgs
			.retain(|msg| wrapping_gt(msg.message_index, ack_index));

		let len_after = hs.pending_ws_msgs.len();
		tracing::debug!(
			removed_count = len_before - len_after,
			remaining_count = len_after,
			"acked pending websocket messages"
		);

		Ok(())
	}

	#[tracing::instrument(skip_all)]
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
	#[tracing::instrument(skip_all)]
	async fn gc_in_flight_requests(&self) {
		#[derive(Debug)]
		enum MsgGcReason {
			/// Gateway channel is closed and is not hibernating
			GatewayClosed,
			/// WebSocket pending messages (ToServerWebSocketMessageAck)
			WebSocketMessageNotAcked {
				#[allow(dead_code)]
				first_msg_index: u16,
				#[allow(dead_code)]
				last_msg_index: u16,
			},
			/// The gateway has not kept alive the in flight request during hibernation for the given timeout
			/// duration.
			HibernationTimeout,
		}

		let now = Instant::now();
		let hibernation_timeout =
			Duration::from_millis(self.hibernation_timeout.try_into().unwrap_or(90_000));

		// First, check if an in flight req is beyond the timeout for tunnel message ack and websocket
		// message ack
		self.in_flight_requests
			.iter_mut_async(|mut entry| {
				let (request_id, req) = &mut *entry;

				if req.stopping {
					return true;
				}

				let reason = 'reason: {
					if let Some(hs) = &req.hibernation_state {
						if let Some(earliest_pending_ws_msg) = hs.pending_ws_msgs.first() {
							if now.duration_since(earliest_pending_ws_msg.send_instant)
								> HWS_MESSAGE_ACK_TIMEOUT
							{
								break 'reason Some(MsgGcReason::WebSocketMessageNotAcked {
									first_msg_index: earliest_pending_ws_msg.message_index,
									last_msg_index: req.message_index
								});
							}
						}

						let hs_elapsed = hs.last_ping.elapsed();
						tracing::debug!(
							hs_elapsed=%hs_elapsed.as_secs_f64(),
							timeout=%hibernation_timeout.as_secs_f64(),
							"checking hibernating state elapsed time"
						);
						if hs_elapsed> hibernation_timeout {
							break 'reason Some(MsgGcReason::HibernationTimeout);
						}
					} else if req.msg_tx.is_closed() {
						break 'reason Some(MsgGcReason::GatewayClosed);
					}

					None
				};

				if let Some(reason) = &reason {
					tracing::debug!(
						request_id=%tunnel_id::request_id_to_string(request_id),
						?reason,
						"gc stopping in flight request"
					);

					if req.drop_tx.send(()).is_err() {
						tracing::debug!(request_id=%tunnel_id::request_id_to_string(request_id), "failed to send timeout msg to tunnel");
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
						request_id=%tunnel_id::request_id_to_string(request_id),
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
