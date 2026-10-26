//! Bounded HTTP duplex forwarding; response ownership cancels unfinished actor work.
use anyhow::{Context, Result, anyhow, bail, ensure};
use axum::{body::Body, response::Response};
use bytes::Bytes;
use futures_util::StreamExt;
use rivet_engine_context::EngineContext;
use rivet_envoy_protocol as protocol;
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot, watch};
use universalpubsub::{DuplexStream, PacketSender};
use vbare::Versioned;

/// The request and response bodies advance independently under protocol byte credit.
/// Keeping the task scope in the response body ties cancellation to the HTTP client.
pub async fn proxy(
	ctx: &EngineContext,
	stream: DuplexStream,
	body: Body,
	message_id: protocol::MessageId,
	start: protocol::ToEnvoyRequestStart,
) -> Result<Response> {
	let scope = Arc::new(ctx.env().task_scope());

	let (headers_tx, headers_rx) = oneshot::channel();
	let (body_tx, body_rx) = mpsc::channel(8);
	let (consumed_tx, consumed_rx) = watch::channel(0u64);

	let clock = ctx.env().clock();
	scope.spawn(async move {
		let mut headers = Some(headers_tx);
		let result = forward(
			clock.as_ref(),
			stream,
			body,
			message_id,
			start,
			&mut headers,
			&body_tx,
			consumed_rx,
		)
		.await;

		if let Err(error) = result {
			if let Some(headers) = headers.take() {
				let _ = headers.send(Err(error));
			} else {
				// The client only sees a truncated body, so the cause is logged here.
				tracing::warn!(
					?error,
					"actor HTTP response failed after its headers; the client sees a truncated body"
				);
				let _ = body_tx.send(Err(error)).await;
			}
		}
	});

	let start = headers_rx
		.await
		.context("HTTP tunnel stopped before response headers")??;
	if let Some(response) = admission_rejection(&start)? {
		// The actor did not admit the request. Drop the upload/response owner and
		// let the caller re-resolve its actor and replay its own retained body.
		drop(scope);
		return Ok(response);
	}
	let mut response = http::Response::builder().status(start.status);
	let connection_headers: Vec<_> = start
		.headers
		.iter()
		.filter(|(name, _)| name.eq_ignore_ascii_case("connection"))
		.flat_map(|(_, value)| {
			value
				.split(',')
				.map(|name| name.trim().to_ascii_lowercase())
		})
		.collect();

	for (name, value) in start.headers {
		// Hyper owns HTTP framing on this hop. Runtime responses may echo the
		// incoming request headers, including its transfer encoding.
		if matches!(
			name.to_ascii_lowercase().as_str(),
			"connection"
				| "keep-alive"
				| "proxy-connection"
				| "transfer-encoding"
				| "te" | "trailer"
				| "upgrade"
		) || connection_headers
			.iter()
			.any(|header| name.eq_ignore_ascii_case(header))
		{
			continue;
		}
		response = response.header(name.as_str(), value.as_str());
	}

	let body = futures_util::stream::try_unfold(
		(body_rx, consumed_tx, 0u64, scope),
		|(mut receiver, credit, mut consumed, scope)| async move {
			let Some(bytes) = receiver.recv().await else {
				return Ok(None);
			};
			let bytes = bytes?;
			consumed = consumed
				.checked_add(bytes.len() as u64)
				.context("response byte count overflow")?;
			credit.send_replace(consumed);
			anyhow::Ok(Some((bytes, (receiver, credit, consumed, scope))))
		},
	);

	Ok(response.body(Body::from_stream(body))?)
}

async fn forward(
	clock: &dyn rivet_env::Clock,
	stream: DuplexStream,
	body: Body,
	mut identity: protocol::MessageId,
	start: protocol::ToEnvoyRequestStart,
	headers: &mut Option<oneshot::Sender<Result<protocol::ToRivetResponseStart>>>,
	output: &mpsc::Sender<Result<Bytes>>,
	mut response_consumed: watch::Receiver<u64>,
) -> Result<()> {
	let (mut sender, mut receiver) = stream.split();

	send(
		&mut sender,
		&mut identity,
		protocol::ToEnvoyTunnelMessageKind::ToEnvoyRequestStart(start),
	)
	.await?;

	let mut upload = body.into_data_stream();
	let mut pending = Bytes::new();
	let mut uploading = true;
	let mut sent = 0u64;
	let mut consumed = 0u64;
	// The envoy closes its input as soon as it has sent the final response chunk, which can
	// reach us after a credit update or upload chunk we try to send it. Once the envoy stops
	// taking packets, stop sending and keep draining the response until it completes.
	let mut peer_input_closed = false;
	loop {
		let available = protocol::HTTP_STREAM_INITIAL_WINDOW_BYTES - (sent - consumed);

		let credit_changed = async {
			if peer_input_closed {
				return std::future::pending().await;
			}
			response_consumed.changed().await
		};

		// Split frames into bounded protocol packets; do not poll upstream without credit.
		let next_upload = async {
			if peer_input_closed || !uploading || available == 0 {
				return std::future::pending().await;
			}

			if pending.is_empty() {
				match upload.next().await {
					Some(bytes) => pending = bytes?,
					None => return Ok::<_, axum::Error>(None),
				}
			}

			let length = pending.len().min(64 * 1024).min(available as usize);

			Ok(Some(pending.split_to(length)))
		};

		rivet_env::select_biased!("actor cancellation and response credit precede additional upload bytes",
			message = receiver.recv() => {
				let bytes = message?.context("actor HTTP tunnel ended before the response completed")?;

				// Decoding shares the packet, so response bodies reach the client without a copy.
				let message = match protocol::versioned::ToRivet::decode(bytes)? {
					protocol::ToRivet::ToRivetTunnelMessage(message) => message,
					_ => bail!("unexpected tunnel response"),
				};

				ensure!(
					message.message_id.gateway_id == identity.gateway_id &&
						message.message_id.request_id == identity.request_id,
					"HTTP response identity changed"
				);

				match message.message_kind {
					protocol::ToRivetTunnelMessageKind::ToRivetRequestBodyWindowUpdate(update) => {
						ensure!(
							update.consumed_bytes >= consumed &&
							update.consumed_bytes <= sent,
							"invalid request body credit"
						);
						consumed = update.consumed_bytes;
					}
					protocol::ToRivetTunnelMessageKind::ToRivetRequestBodyCancel => {
						uploading = false;
						pending = Bytes::new();
						// Dropping upstream is independent of the actor response lifetime.
						upload = Body::empty().into_data_stream();
					}
					protocol::ToRivetTunnelMessageKind::ToRivetResponseStart(mut start) => {
						let bytes = start.body.take();
						let finished = !start.stream;
						headers.take().context("duplicate HTTP response headers")?.send(Ok(start)).map_err(|_| anyhow!("HTTP caller disconnected"))?;

						if let Some(bytes) = bytes {
							output.send(Ok(Bytes::from(bytes))).await?;
						}
						if finished {
							finish_after_response(clock, sender).await;
							return Ok(());
						}
					}
					protocol::ToRivetTunnelMessageKind::ToRivetResponseChunk(chunk) => {
						ensure!(headers.is_none(), "HTTP body preceded response headers");

						if !chunk.body.is_empty() {
							output.send(Ok(Bytes::from(chunk.body))).await?;
						}

						if chunk.finish {
							finish_after_response(clock, sender).await;
							return Ok(());
						}
					}
					protocol::ToRivetTunnelMessageKind::ToRivetResponseAbort(_) => {
						bail!("actor response aborted")
					}
					_ => bail!("unexpected HTTP tunnel message"),
				}
			},
			changed = credit_changed => {
				changed.context("HTTP response client disconnected")?;

				let consumed_bytes = *response_consumed.borrow_and_update();

				let result = send(
					&mut sender,
					&mut identity,
					protocol::ToEnvoyTunnelMessageKind::ToEnvoyResponseBodyWindowUpdate(
						protocol::ToEnvoyResponseBodyWindowUpdate {
							consumed_bytes,
						}
					)
				).await;
				peer_input_closed = peer_closed_input(result)?;
			},
			bytes = next_upload => {
				let bytes = bytes?;
				let finish = bytes.is_none();
				let bytes = bytes.unwrap_or_default();
				sent = sent.checked_add(bytes.len() as u64).context("request byte count overflow")?;

				let result = send(
					&mut sender,
					&mut identity,
					protocol::ToEnvoyTunnelMessageKind::ToEnvoyRequestChunk(
						protocol::ToEnvoyRequestChunk {
							body: bytes, finish
						}
					)
				).await;
				peer_input_closed = peer_closed_input(result)?;

				if finish {
					uploading = false;
				}
			},
		);
	}
}

/// Reports whether a send found the envoy's input closed. The envoy closes it right after
/// its final response chunk, so that is not a failure while the response is still arriving;
/// the receive side decides whether the response completed. Other send errors are fatal.
fn peer_closed_input(result: Result<()>) -> Result<bool> {
	let Err(error) = result else {
		return Ok(false);
	};
	let extracted = rivet_error::RivetError::extract(&error);
	if extracted.group() != "ups" || extracted.code() != "consumer_closed" {
		return Err(error);
	}

	tracing::debug!(%error, "HTTP tunnel peer stopped taking packets; draining the response");
	Ok(true)
}

async fn send(
	sender: &mut PacketSender,
	identity: &mut protocol::MessageId,
	kind: protocol::ToEnvoyTunnelMessageKind,
) -> Result<()> {
	let msg = protocol::versioned::ToEnvoy::serialize_with_embedded_version(
		&protocol::ToEnvoy::ToEnvoyTunnelMessage(protocol::ToEnvoyTunnelMessage {
			message_id: identity.clone(),
			message_kind: kind,
		}),
		protocol::PROTOCOL_VERSION,
	)?;

	sender.send(msg).await?;

	identity.message_index = identity
		.message_index
		.checked_add(1)
		.context("HTTP message index overflow")?;
	Ok(())
}

/// These reserved runtime headers prove rejection before application admission.
/// Transport failure or an ordinary application error gives no such proof.
fn admission_rejection(start: &protocol::ToRivetResponseStart) -> Result<Option<Response>> {
	if start.status != 503
		|| !start.headers.iter().any(|(name, value)| {
			name.eq_ignore_ascii_case("x-rivet-error")
				&& matches!(
					value.as_str(),
					"envoy.actor_not_found"
						| "envoy.actor_generation_mismatch"
						| "runner.actor_not_found"
						| "runner.actor_generation_mismatch"
				)
		}) {
		return Ok(None);
	}
	Ok(Some(
		http::Response::builder()
			.status(http::StatusCode::SERVICE_UNAVAILABLE)
			.header(http::header::CONTENT_TYPE, "application/json")
			.header("x-rivet-error", "guard.actor_stopped_while_waiting")
			.body(Body::from(serde_json::to_vec(&serde_json::json!({
				"group": "guard",
				"code": "actor_stopped_while_waiting",
				"message": "Actor stopped before accepting the request."
			}))?))?,
	))
}



/// How long a complete response waits to finish its upload half before the final bytes go to
/// the HTTP caller anyway. A healthy envoy takes the terminal packet at once; a stalled envoy
/// must not hold a finished response for the whole stream deadline.
const RESPONSE_FINISH_GRACE: std::time::Duration = std::time::Duration::from_millis(250);

/// Ends the upload half once the actor's response is complete, within
/// [`RESPONSE_FINISH_GRACE`]. The envoy closes its side of the stream as soon as it forwards the
/// final response, so finishing may find the peer already gone; the client has its whole
/// response either way, so that is not an error.
async fn finish_after_response(clock: &dyn rivet_env::Clock, sender: PacketSender) {
	match rivet_env::timeout(clock, RESPONSE_FINISH_GRACE, sender.finish()).await {
		Ok(Ok(())) => {}
		Ok(Err(error)) => {
			tracing::debug!(%error, "HTTP tunnel peer closed before the upload finished");
		}
		Err(_) => {
			tracing::debug!("HTTP tunnel peer did not take the upload finish in time");
		}
	}
}
