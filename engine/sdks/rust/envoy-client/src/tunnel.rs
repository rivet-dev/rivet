use rivet_envoy_protocol as protocol;
use std::collections::HashMap;
use std::time::Duration;

use crate::connection::ws_send;
use crate::envoy::{BufferedActorMessage, EnvoyContext, HttpRequestRoute, WebSocketRoute};

pub(crate) const HTTP_REQUEST_CANCELLATION_TTL: Duration = Duration::from_secs(60);

#[derive(Clone, Debug, Eq, Hash, PartialEq, PartialOrd, Ord)]
pub struct HttpRequestCancellationKey {
	gateway_id: protocol::GatewayId,
	request_id: protocol::RequestId,
	actor_id: String,
	actor_generation: u32,
}

fn request_cancellation_key(
	message_id: &protocol::MessageId,
	actor_id: &str,
	actor_generation: u32,
) -> HttpRequestCancellationKey {
	HttpRequestCancellationKey {
		gateway_id: message_id.gateway_id,
		request_id: message_id.request_id,
		actor_id: actor_id.to_owned(),
		actor_generation,
	}
}

fn request_abort_cancellation_key(
	message_id: &protocol::MessageId,
	abort: &protocol::ToEnvoyRequestAbort,
) -> Result<Option<HttpRequestCancellationKey>, &'static str> {
	match (&abort.actor_id, abort.actor_generation) {
		(Some(actor_id), Some(actor_generation)) => Ok(Some(request_cancellation_key(
			message_id,
			actor_id,
			actor_generation,
		))),
		(None, None) => Ok(None),
		_ => Err("HTTP request abort must include both actor id and generation"),
	}
}

/// Forgets one recorded cancellation once its TTL passed. A cancellation recorded again
/// since its deadline was set is scheduled again instead.
pub(crate) fn expire_http_request_cancellation(
	ctx: &mut EnvoyContext,
	key: HttpRequestCancellationKey,
	now: crate::time::Instant,
) {
	let Some(cancelled_at) = ctx.http_request_cancellations.get(&key).copied() else {
		return;
	};
	if now.saturating_duration_since(cancelled_at) < HTTP_REQUEST_CANCELLATION_TTL {
		ctx.deadlines.schedule(
			crate::deadlines::DeadlineKey::HttpCancellation(key),
			cancelled_at + HTTP_REQUEST_CANCELLATION_TTL,
		);
		return;
	}
	ctx.http_request_cancellations.remove(&key);
}

pub(crate) fn make_ws_key(
	gateway_id: &protocol::GatewayId,
	request_id: &protocol::RequestId,
) -> [u8; 8] {
	let mut key = [0u8; 8];
	key[..4].copy_from_slice(gateway_id);
	key[4..].copy_from_slice(request_id);
	key
}

fn advance_http_message_index(ctx: &mut EnvoyContext, message_id: &protocol::MessageId) -> bool {
	let key: [&[u8]; 2] = [&message_id.gateway_id, &message_id.request_id];
	let Some(expected) = ctx.http_message_indices.get_mut(&key) else {
		tracing::warn!(
			message_index = message_id.message_index,
			"received HTTP tunnel message without request start"
		);
		return false;
	};
	if message_id.message_index != *expected {
		tracing::warn!(
			expected_message_index = *expected,
			actual_message_index = message_id.message_index,
			"received reordered HTTP tunnel message"
		);
		return false;
	}
	*expected = expected.wrapping_add(1);
	true
}

pub struct HibernatingWebSocketMetadata {
	pub gateway_id: protocol::GatewayId,
	pub request_id: protocol::RequestId,
	pub envoy_message_index: u16,
	pub rivet_message_index: u16,
	pub path: String,
	pub headers: std::collections::HashMap<String, String>,
}

pub async fn handle_tunnel_message(
	ctx: &mut EnvoyContext,
	connection_session: u64,
	msg: protocol::ToEnvoyTunnelMessage,
) {
	let message_id = msg.message_id;
	let is_http_continuation = matches!(
		&msg.message_kind,
		protocol::ToEnvoyTunnelMessageKind::ToEnvoyRequestChunk(_)
			| protocol::ToEnvoyTunnelMessageKind::ToEnvoyRequestAbort(_)
			| protocol::ToEnvoyTunnelMessageKind::ToEnvoyRequestBodyCancel
			| protocol::ToEnvoyTunnelMessageKind::ToEnvoyResponseBodyWindowUpdate(_)
	);
	if is_http_continuation
		&& let Some(route) = ctx
			.http_request_routes
			.get(&[&message_id.gateway_id, &message_id.request_id])
		&& route.session != connection_session
	{
		handle_http_protocol_violation(
			ctx,
			connection_session,
			message_id,
			"HTTP request identifier belongs to another connection",
		)
		.await;
		return;
	}
	match msg.message_kind {
		protocol::ToEnvoyTunnelMessageKind::ToEnvoyRequestStart(req) => {
			handle_request_start(ctx, connection_session, message_id, req).await;
		}
		protocol::ToEnvoyTunnelMessageKind::ToEnvoyRequestChunk(chunk) => {
			if advance_http_message_index(ctx, &message_id) {
				handle_request_chunk(ctx, message_id, chunk).await;
			} else {
				handle_http_protocol_violation(
					ctx,
					connection_session,
					message_id,
					"invalid HTTP tunnel message sequence",
				)
				.await;
			}
		}
		protocol::ToEnvoyTunnelMessageKind::ToEnvoyRequestAbort(abort) => {
			let cancellation_key = match request_abort_cancellation_key(&message_id, &abort) {
				Ok(key) => key,
				Err(detail) => {
					handle_http_protocol_violation(ctx, connection_session, message_id, detail)
						.await;
					return;
				}
			};
			if let Some(cancellation_key) = cancellation_key {
				let route = ctx
					.http_request_routes
					.get(&[&message_id.gateway_id, &message_id.request_id])
					.map(|route| (route.actor_id.clone(), route.actor_generation));
				if let Some((route_actor_id, route_actor_generation)) = route
					&& (cancellation_key.actor_id != route_actor_id
						|| Some(cancellation_key.actor_generation) != route_actor_generation)
				{
					handle_http_protocol_violation(
						ctx,
						connection_session,
						message_id,
						"HTTP request abort actor identity does not match request start",
					)
					.await;
				} else {
					// Exact-generation cancellation is intentionally out of band from the
					// ordered body stream. It must work after an ambiguous message handoff,
					// when Gateway cannot know which message index Envoy accepted.
					handle_request_abort(ctx, message_id, abort, Some(cancellation_key));
				}
			} else if advance_http_message_index(ctx, &message_id) {
				// V6 cancellation has no actor identity, so preserve its ordered behavior.
				handle_request_abort(ctx, message_id, abort, None);
			} else {
				handle_http_protocol_violation(
					ctx,
					connection_session,
					message_id,
					"invalid HTTP request-abort sequence",
				)
				.await;
			}
		}
		protocol::ToEnvoyTunnelMessageKind::ToEnvoyRequestBodyCancel => {
			if advance_http_message_index(ctx, &message_id) {
				handle_request_body_cancel(ctx, message_id);
			} else {
				handle_http_protocol_violation(
					ctx,
					connection_session,
					message_id,
					"invalid HTTP request-body-cancel sequence",
				)
				.await;
			}
		}
		protocol::ToEnvoyTunnelMessageKind::ToEnvoyResponseBodyWindowUpdate(update) => {
			// The gateway releases response credit as its client reads the body, which can be
			// after this Envoy finished the request and forgot it. Credit for a finished
			// response has no effect, so it is dropped instead of answered as a violation.
			let key: [&[u8]; 2] = [&message_id.gateway_id, &message_id.request_id];
			if ctx.http_message_indices.get(&key).is_none() {
				tracing::debug!(
					message_index = message_id.message_index,
					"dropping response credit for an HTTP request this Envoy already finished"
				);
				return;
			}
			if advance_http_message_index(ctx, &message_id) {
				handle_response_body_window_update(ctx, message_id, update.consumed_bytes);
			} else {
				handle_http_protocol_violation(
					ctx,
					connection_session,
					message_id,
					"invalid HTTP response-window sequence",
				)
				.await;
			}
		}
		protocol::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketOpen(open) => {
			handle_ws_open(ctx, message_id, open).await;
		}
		protocol::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketMessage(msg) => {
			handle_ws_message(ctx, message_id, msg);
		}
		protocol::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketClose(close) => {
			handle_ws_close(ctx, message_id, close);
		}
	}
}

async fn handle_http_protocol_violation(
	ctx: &mut EnvoyContext,
	connection_session: u64,
	message_id: protocol::MessageId,
	detail: &'static str,
) {
	let key: [&[u8]; 2] = [&message_id.gateway_id, &message_id.request_id];
	if let Some(route) = ctx.http_request_routes.get(&key) {
		if route.session != connection_session {
			send_response_abort_for_session(ctx, connection_session, message_id, detail).await;
			return;
		}
		let actor_id = route.actor_id.clone();
		let actor_generation = route.actor_generation;
		if route.actor_admitted
			&& let Some(actor) = ctx.get_actor(&actor_id, actor_generation)
		{
			let _ = actor
				.handle
				.send(crate::actor::ToActor::ReqProtocolViolation {
					message_id: message_id.clone(),
					detail: detail.to_owned(),
				});
		}
		ctx.http_request_routes.remove(&key);
		ctx.http_message_indices.remove(&key);
		return;
	}

	send_response_abort_for_session(ctx, connection_session, message_id, detail).await;
}

async fn send_response_abort_for_session(
	ctx: &EnvoyContext,
	connection_session: u64,
	mut message_id: protocol::MessageId,
	detail: &'static str,
) {
	message_id.message_index = 0;
	crate::connection::ws_send_http_for_session_detached(
		&ctx.shared,
		protocol::ToRivet::ToRivetTunnelMessage(protocol::ToRivetTunnelMessage {
			message_id,
			message_kind: protocol::ToRivetTunnelMessageKind::ToRivetResponseAbort(
				protocol::ToRivetResponseAbort {
					reason: protocol::HttpStreamAbortReason {
						kind: protocol::HttpStreamAbortReasonKind::InternalError,
						detail: Some(detail.to_owned().into()),
					},
				},
			),
		}),
		connection_session,
	)
	.await;
}

fn handle_request_body_cancel(ctx: &mut EnvoyContext, message_id: protocol::MessageId) {
	let route = ctx
		.http_request_routes
		.get(&[&message_id.gateway_id, &message_id.request_id])
		.map(|route| {
			(
				route.actor_id.clone(),
				route.actor_generation,
				route.actor_admitted,
			)
		});
	if let Some((actor_id, actor_generation, true)) = &route
		&& let Some(actor) = ctx.get_actor(actor_id, *actor_generation)
	{
		let _ = actor.handle.send(crate::actor::ToActor::ReqBodyCancel {
			message_id: message_id.clone(),
		});
	}
	if matches!(route, Some((_, _, false))) {
		ctx.http_request_routes
			.remove(&[&message_id.gateway_id, &message_id.request_id]);
		ctx.http_message_indices
			.remove(&[&message_id.gateway_id, &message_id.request_id]);
	}
}

fn handle_response_body_window_update(
	ctx: &mut EnvoyContext,
	message_id: protocol::MessageId,
	consumed_bytes: u64,
) {
	let route = ctx
		.http_request_routes
		.get(&[&message_id.gateway_id, &message_id.request_id])
		.map(|route| {
			(
				route.actor_id.clone(),
				route.actor_generation,
				route.actor_admitted,
			)
		});
	if let Some((actor_id, actor_generation, true)) = &route
		&& let Some(actor) = ctx.get_actor(actor_id, *actor_generation)
	{
		let _ = actor
			.handle
			.send(crate::actor::ToActor::ResponseBodyWindowUpdate {
				message_id,
				consumed_bytes,
			});
	}
}

async fn handle_request_start(
	ctx: &mut EnvoyContext,
	connection_session: u64,
	message_id: protocol::MessageId,
	req: protocol::ToEnvoyRequestStart,
) {
	let key: [&[u8]; 2] = [&message_id.gateway_id, &message_id.request_id];
	let actor_id = req.actor_id.clone();
	let actor_generation = req.actor_generation;
	let response_stream = req.response_stream;
	let request_stream = req.stream;
	if message_id.message_index != 0 {
		tracing::warn!(
			message_index = message_id.message_index,
			"received invalid HTTP request start sequence"
		);
		send_response_abort_for_session(
			ctx,
			connection_session,
			message_id,
			"HTTP request start must use message index zero",
		)
		.await;
		return;
	}
	if let Some(actor_generation) = actor_generation {
		let cancellation_key = request_cancellation_key(&message_id, &actor_id, actor_generation);
		// An entry past its TTL whose deadline has not fired yet no longer counts.
		let now = crate::time::Instant::now();
		if ctx
			.http_request_cancellations
			.get(&cancellation_key)
			.is_some_and(|cancelled_at| {
				now.saturating_duration_since(*cancelled_at) < HTTP_REQUEST_CANCELLATION_TTL
			}) {
			tracing::debug!(
				%actor_id,
				actor_generation,
				gateway_id = ?message_id.gateway_id,
				request_id = ?message_id.request_id,
				"discarding HTTP request start cancelled before delivery"
			);
			return;
		}
	}
	if let Some(existing) = ctx.http_request_routes.get(&key) {
		let same_session = existing.session == connection_session;
		handle_http_protocol_violation(
			ctx,
			connection_session,
			message_id,
			if same_session {
				"duplicate HTTP request start"
			} else {
				"HTTP request identifier belongs to another connection"
			},
		)
		.await;
		return;
	}
	// The cap bounds requests waiting in the actor's inbox, mostly while it starts.
	let cap = ctx.shared.config.max_queued_requests_per_actor;
	let admitted = ctx
		.get_actor_for_admission(&actor_id, actor_generation)
		.map(|actor| (actor.handle.clone(), actor.queued_requests.admit(cap)));
	let (actor_handle, queued) = match admitted {
		Some((handle, Some(queued))) => (Some(handle), Some(queued)),
		Some((_, None)) => {
			crate::metrics::METRICS
				.actor_queued_requests_overloaded_total
				.inc();
			if request_stream {
				drain_rejected_upload(
					ctx,
					connection_session,
					&message_id,
					&actor_id,
					actor_generation,
				);
			}
			send_error_response(
				ctx,
				response_stream.then_some(connection_session),
				message_id.gateway_id,
				message_id.request_id,
				"envoy.actor_overloaded",
				"Actor has too many queued requests",
				actor_generation.map(|generation| (actor_id.as_str(), generation)),
			)
			.await;
			return;
		}
		None => (None, None),
	};

	let Some(actor_handle) = actor_handle else {
		let generation_mismatch =
			actor_generation.is_some() && ctx.get_actor(&actor_id, None).is_some();
		let (error_code, message) = if generation_mismatch {
			tracing::warn!(
				actor_id = %actor_id,
				?actor_generation,
				"received request for stale actor generation"
			);
			(
				"envoy.actor_generation_mismatch",
				"Actor generation does not match",
			)
		} else {
			tracing::warn!(actor_id = %actor_id, ?actor_generation, "received request for unknown actor");
			("envoy.actor_not_found", "Actor not found")
		};
		if request_stream {
			drain_rejected_upload(
				ctx,
				connection_session,
				&message_id,
				&actor_id,
				actor_generation,
			);
		}
		send_error_response(
			ctx,
			response_stream.then_some(connection_session),
			message_id.gateway_id,
			message_id.request_id,
			error_code,
			message,
			actor_generation.map(|generation| (actor_id.as_str(), generation)),
		)
		.await;
		return;
	};
	ctx.http_message_indices
		.insert(&[&message_id.gateway_id, &message_id.request_id], 1);

	ctx.http_request_routes.insert(
		&[&message_id.gateway_id, &message_id.request_id],
		HttpRequestRoute {
			actor_id: actor_id.clone().to_string(),
			actor_generation,
			actor_admitted: true,
			session: connection_session,
			gateway_id: message_id.gateway_id,
			request_id: message_id.request_id,
		},
	);

	if actor_handle
		.send(crate::actor::ToActor::ReqStart {
			message_id: message_id.clone(),
			req,
			connection_session,
			queued,
		})
		.is_err()
	{
		ctx.http_request_routes
			.remove(&[&message_id.gateway_id, &message_id.request_id]);
		ctx.http_message_indices
			.remove(&[&message_id.gateway_id, &message_id.request_id]);
		send_error_response(
			ctx,
			response_stream.then_some(connection_session),
			message_id.gateway_id,
			message_id.request_id,
			"envoy.actor_not_found",
			"Actor stopped before accepting request",
			actor_generation.map(|generation| (actor_id.as_str(), generation)),
		)
		.await;
	}
}

/// Keeps a short-lived drain route for a streamed upload whose start was refused.
/// RequestStart is deliberately not application-acknowledged, so body frames may already be in
/// flight when the rejection reaches Gateway; its finish, cancellation, abort, or connection
/// close removes the state.
fn drain_rejected_upload(
	ctx: &mut EnvoyContext,
	connection_session: u64,
	message_id: &protocol::MessageId,
	actor_id: &str,
	actor_generation: Option<u32>,
) {
	let key: [&[u8]; 2] = [&message_id.gateway_id, &message_id.request_id];
	ctx.http_message_indices.insert(&key, 1);
	ctx.http_request_routes.insert(
		&key,
		HttpRequestRoute {
			actor_id: actor_id.to_string(),
			actor_generation,
			actor_admitted: false,
			session: connection_session,
			gateway_id: message_id.gateway_id,
			request_id: message_id.request_id,
		},
	);
}

async fn handle_request_chunk(
	ctx: &mut EnvoyContext,
	message_id: protocol::MessageId,
	chunk: protocol::ToEnvoyRequestChunk,
) {
	let route = ctx
		.http_request_routes
		.get(&[&message_id.gateway_id, &message_id.request_id])
		.map(|route| {
			(
				route.actor_id.clone(),
				route.actor_generation,
				route.actor_admitted,
			)
		});

	if let Some((actor_id, actor_generation, actor_admitted)) = &route {
		if !actor_admitted {
			if chunk.finish {
				ctx.http_request_routes
					.remove(&[&message_id.gateway_id, &message_id.request_id]);
				ctx.http_message_indices
					.remove(&[&message_id.gateway_id, &message_id.request_id]);
			}
			return;
		}
		if let Some(actor) = ctx.get_actor(actor_id, *actor_generation) {
			let _ = actor.handle.send(crate::actor::ToActor::ReqChunk {
				message_id: message_id.clone(),
				chunk,
			});
		} else {
			tracing::warn!(actor_id = %actor_id, "received request chunk for unknown actor");
		}
	} else {
		tracing::warn!(
			gateway_id = ?message_id.gateway_id,
			request_id = ?message_id.request_id,
			message_index = message_id.message_index,
			"received request chunk without request start"
		);
		send_error_response(
			ctx,
			None,
			message_id.gateway_id,
			message_id.request_id,
			"envoy.request_not_found",
			"Request start was not delivered",
			None,
		)
		.await;
	}
}

fn handle_request_abort(
	ctx: &mut EnvoyContext,
	message_id: protocol::MessageId,
	abort: protocol::ToEnvoyRequestAbort,
	cancellation_key: Option<HttpRequestCancellationKey>,
) {
	let route = ctx
		.http_request_routes
		.get(&[&message_id.gateway_id, &message_id.request_id])
		.map(|route| {
			(
				route.actor_id.clone(),
				route.actor_generation,
				route.actor_admitted,
			)
		});
	if let Some((actor_id, actor_generation, true)) = &route {
		if let Some(actor) = ctx.get_actor(actor_id, *actor_generation) {
			let _ = actor.handle.send(crate::actor::ToActor::ReqAbort {
				message_id: message_id.clone(),
				reason: abort.reason,
			});
		}
	}
	if let Some(cancellation_key) = cancellation_key {
		let now = crate::time::Instant::now();
		ctx.deadlines.schedule(
			crate::deadlines::DeadlineKey::HttpCancellation(cancellation_key.clone()),
			now + HTTP_REQUEST_CANCELLATION_TTL,
		);
		ctx.http_request_cancellations.insert(cancellation_key, now);
		tracing::debug!(
			gateway_id = ?message_id.gateway_id,
			request_id = ?message_id.request_id,
			"recorded exact HTTP request cancellation"
		);
	}

	ctx.http_request_routes
		.remove(&[&message_id.gateway_id, &message_id.request_id]);
	ctx.http_message_indices
		.remove(&[&message_id.gateway_id, &message_id.request_id]);
}

async fn handle_ws_open(
	ctx: &mut EnvoyContext,
	message_id: protocol::MessageId,
	open: protocol::ToEnvoyWebSocketOpen,
) {
	let actor_id = open.actor_id.clone();
	let actor_generation = open.actor_generation;
	let actor_handle = ctx
		.get_actor_for_admission(&actor_id, actor_generation)
		.map(|actor| actor.handle.clone());

	let Some(actor_handle) = actor_handle else {
		let generation_mismatch =
			actor_generation.is_some() && ctx.get_actor(&actor_id, None).is_some();
		let reason = if generation_mismatch {
			tracing::warn!(
				actor_id = %actor_id,
				?actor_generation,
				"received ws open for stale actor generation"
			);
			"envoy.actor_generation_mismatch"
		} else {
			tracing::warn!(actor_id = %actor_id, ?actor_generation, "received ws open for unknown actor");
			"envoy.actor_not_found"
		};

		ws_send(
			&ctx.shared,
			protocol::ToRivet::ToRivetTunnelMessage(protocol::ToRivetTunnelMessage {
				message_id,
				message_kind: protocol::ToRivetTunnelMessageKind::ToRivetWebSocketClose(
					protocol::ToRivetWebSocketClose {
						code: Some(1011),
						reason: Some(reason.to_string().into()),
						hibernate: false,
					},
				),
			}),
		)
		.await;
		return;
	};

	ctx.request_to_actor.insert(
		&[&message_id.gateway_id, &message_id.request_id],
		WebSocketRoute {
			actor_id: actor_id.clone().to_string(),
			actor_generation,
		},
	);
	ctx.shared
		.live_tunnel_requests
		.lock()
		.expect("shared live tunnel request registry poisoned")
		.insert(
			make_ws_key(&message_id.gateway_id, &message_id.request_id),
			actor_id.clone().to_string(),
		);

	// Convert HashMap headers to BTreeMap for the actor message
	let headers = open
		.headers
		.iter()
		.map(|(k, v)| (k.to_string(), v.to_string()))
		.collect();

	if actor_handle
		.send(crate::actor::ToActor::WsOpen {
			message_id: message_id.clone(),
			path: open.path.to_string(),
			headers,
		})
		.is_err()
	{
		ctx.request_to_actor
			.remove(&[&message_id.gateway_id, &message_id.request_id]);
		ctx.shared
			.live_tunnel_requests
			.lock()
			.expect("shared live tunnel request registry poisoned")
			.remove(&make_ws_key(&message_id.gateway_id, &message_id.request_id));
		ws_send(
			&ctx.shared,
			protocol::ToRivet::ToRivetTunnelMessage(protocol::ToRivetTunnelMessage {
				message_id,
				message_kind: protocol::ToRivetTunnelMessageKind::ToRivetWebSocketClose(
					protocol::ToRivetWebSocketClose {
						code: Some(1011),
						reason: Some(("envoy.actor_not_found".to_string()).into()),
						hibernate: false,
					},
				),
			}),
		)
		.await;
	}
}

fn handle_ws_message(
	ctx: &mut EnvoyContext,
	message_id: protocol::MessageId,
	msg: protocol::ToEnvoyWebSocketMessage,
) {
	let route = ctx
		.request_to_actor
		.get(&[&message_id.gateway_id, &message_id.request_id])
		.cloned();
	if let Some(route) = &route {
		if let Some(actor) = ctx.get_actor(&route.actor_id, route.actor_generation) {
			let _ = actor
				.handle
				.send(crate::actor::ToActor::WsMsg { message_id, msg });
		} else if route.actor_generation.is_none() {
			ctx.buffered_actor_messages
				.entry(route.actor_id.clone())
				.or_default()
				.push(BufferedActorMessage::WsMsg { message_id, msg });
		}
	}
}

fn handle_ws_close(
	ctx: &mut EnvoyContext,
	message_id: protocol::MessageId,
	close: protocol::ToEnvoyWebSocketClose,
) {
	let route = ctx
		.request_to_actor
		.get(&[&message_id.gateway_id, &message_id.request_id])
		.cloned();
	if let Some(route) = &route {
		if let Some(actor) = ctx.get_actor(&route.actor_id, route.actor_generation) {
			let _ = actor.handle.send(crate::actor::ToActor::WsClose {
				message_id: message_id.clone(),
				close,
			});
		} else if route.actor_generation.is_none() {
			ctx.buffered_actor_messages
				.entry(route.actor_id.clone())
				.or_default()
				.push(BufferedActorMessage::WsClose {
					message_id: message_id.clone(),
					close,
				});
		}
	}

	ctx.request_to_actor
		.remove(&[&message_id.gateway_id, &message_id.request_id]);
	ctx.shared
		.live_tunnel_requests
		.lock()
		.expect("shared live tunnel request registry poisoned")
		.remove(&make_ws_key(&message_id.gateway_id, &message_id.request_id));
}

pub fn send_hibernatable_ws_message_ack(
	ctx: &mut EnvoyContext,
	gateway_id: protocol::GatewayId,
	request_id: protocol::RequestId,
	envoy_message_index: u16,
) {
	let route = ctx
		.request_to_actor
		.get(&[&gateway_id, &request_id])
		.cloned();
	if let Some(route) = &route {
		if let Some(actor) = ctx.get_actor(&route.actor_id, route.actor_generation) {
			let _ = actor.handle.send(crate::actor::ToActor::HwsAck {
				gateway_id,
				request_id,
				envoy_message_index,
			});
		}
	}
}

pub async fn resend_buffered_tunnel_messages(ctx: &mut EnvoyContext) {
	if ctx.buffered_messages.is_empty() {
		return;
	}

	tracing::info!(
		count = ctx.buffered_messages.len(),
		"resending buffered tunnel messages"
	);

	let messages = std::mem::take(&mut ctx.buffered_messages);
	let mut messages = messages.into_iter();
	while let Some(msg) = messages.next() {
		let failed = ws_send(
			&ctx.shared,
			protocol::ToRivet::ToRivetTunnelMessage(msg.clone()),
		)
		.await;
		if failed {
			ctx.buffered_messages.push(msg);
			ctx.buffered_messages.extend(messages);
			break;
		}
	}
}

pub async fn send_or_buffer_tunnel_message(
	ctx: &mut EnvoyContext,
	msg: protocol::ToRivetTunnelMessage,
) {
	let failed = ws_send(
		&ctx.shared,
		protocol::ToRivet::ToRivetTunnelMessage(msg.clone()),
	)
	.await;
	if failed {
		ctx.buffered_messages.push(msg);
	}
}

async fn send_error_response(
	ctx: &EnvoyContext,
	connection_session: Option<u64>,
	gateway_id: protocol::GatewayId,
	request_id: protocol::RequestId,
	error_code: &str,
	message: &str,
	actor: Option<(&str, u32)>,
) {
	let code = error_code.strip_prefix("envoy.").unwrap_or(error_code);
	let mut error = serde_json::json!({
		"group": "envoy",
		"code": code,
		"message": message,
	});
	if let Some((actor_id, generation)) = actor {
		error["actor"] = serde_json::json!({
			"actorId": actor_id,
			"generation": generation,
		});
	}
	let body = serde_json::to_vec(&error).expect("serialize canonical HTTP error response");
	let mut headers = HashMap::new();
	headers.insert("x-rivet-error".to_string(), error_code.to_owned());
	headers.insert("content-type".to_string(), "application/json".to_owned());
	headers.insert("content-length".to_string(), body.len().to_string());

	let response = protocol::ToRivet::ToRivetTunnelMessage(protocol::ToRivetTunnelMessage {
		message_id: protocol::MessageId {
			gateway_id,
			request_id,
			message_index: 0,
		},
		message_kind: protocol::ToRivetTunnelMessageKind::ToRivetResponseStart(
			protocol::ToRivetResponseStart {
				status: 503,
				headers: headers
					.into_iter()
					.map(|(k, v)| (k.into(), v.into()))
					.collect(),
				body: Some(body.into()),
				stream: false,
			},
		),
	});
	match connection_session {
		Some(session) => {
			crate::connection::ws_send_http_for_session_detached(&ctx.shared, response, session)
				.await;
		}
		None => {
			ws_send(&ctx.shared, response).await;
		}
	}
}
