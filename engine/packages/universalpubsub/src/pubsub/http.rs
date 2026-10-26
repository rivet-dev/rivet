use anyhow::Result;
use axum::{
	Router,
	body::Body,
	extract::{Request, State},
	http::{StatusCode, Version},
	response::{IntoResponse, Response},
	routing::post,
};
use bytes::BytesMut;
use futures_util::{StreamExt, stream};
use rivet_data::{generated::ups_request_v1, versioned::UpsRequestData};
use rivet_error::RivetError;
use rivet_topology::worker::Worker;
use vbare::Versioned;

use super::{
	PubSub, WorkerId,
	class::RequestClass,
	stream::{HttpBody, Reader, seal_frame},
};
use crate::errors;

pub(crate) const PATH: &str = "/ups/v1/request";

use rivet_env::capabilities::http::PeerLane;

impl PubSub {
	pub(super) async fn send_stream(
		&self,
		class: RequestClass,
		worker: &WorkerId,
		endpoint: &Worker,
		subject: Vec<u8>,
		body: HttpBody,
		open_remaining_ms: u64,
		lifetime_remaining_ms: Option<u64>,
		may_dispatch: &(dyn Fn() -> bool + Send + Sync),
	) -> Result<HttpBody> {
		let mut header = BytesMut::new();
		seal_frame::<UpsRequestData>(
			&mut header,
			&ups_request_v1::Data {
				source_worker_id: *self.worker_id().as_bytes(),
				worker_id: *worker.as_bytes(),
				instance_id: *endpoint.instance_id.as_bytes(),
				subject: subject.into(),
				open_remaining_ms,
				lifetime_remaining_ms,
				class: class.into(),
			},
			rivet_data::UPS_REQUEST_VERSION,
		)?;
		let header = header.freeze();
		let body = stream::once(async move { Ok(header) }).chain(body).boxed();
		let request = rivet_env::capabilities::http::Request::builder()
			.method("POST")
			.version(Version::HTTP_2)
			.uri(format!(
				"{}{PATH}",
				endpoint.api_peer_address.trim_end_matches('/')
			))
			.header("content-type", "application/octet-stream")
			// Unary requests ride peer channels, so every stream sent here is long-lived.
			.extension(rivet_env::capabilities::http::PeerRoute {
				lane: PeerLane::Stream,
				key: self
					.inner
					.next_route_key
					.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
			})
			.body(body)?;
		// Framing and endpoint lookup may outlive caller authority. This is the last UPS boundary
		// before the injected HTTP adapter owns the request; target-side fences protect sent effects.
		if !may_dispatch() {
			return Err(errors::DispatchRejected.build());
		}
		let open = crate::metrics::STREAM_OPEN_DURATION
			.with_label_values(&[class.as_str()])
			.start_timer();
		let response = match self.inner.client.execute_stream(request).await {
			Ok(response) => {
				open.observe_duration();
				response
			}
			Err(error) => {
				open.stop_and_discard();
				crate::metrics::PEER_CONNECTION_ERRORS
					.with_label_values(&[
						PeerLane::Stream.as_str(),
						connection_error_reason(&error),
					])
					.inc();
				return Err(transport_error(error));
			}
		};
		if response.status() != StatusCode::OK {
			return Err(status_error(response.status()));
		}
		Ok(response
			.into_body()
			.map(|result| result.map_err(transport_error))
			.boxed())
	}

	pub fn router(&self) -> Router {
		Router::new()
			.route(PATH, post(receive))
			.route(super::channel::PATH, post(super::channel::server::receive))
			.with_state(self.clone())
	}
}

async fn receive(State(ups): State<PubSub>, request: Request) -> Response {
	let result = async {
		let mut active = ups.accept_request(None)?;
		if request.version() != Version::HTTP_2 {
			return Err(errors::InvalidStream.build());
		}
		let clock = ups.env().clock();
		let read_deadline =
			rivet_env::deadline_after(clock.as_ref(), ups.inner.options.request_timeout)?;
		let body = request
			.into_body()
			.into_data_stream()
			.map(|result| result.map_err(anyhow::Error::from))
			.boxed();
		let mut reader = Reader::new(body);
		let bytes = rivet_env::timeout_at(clock.as_ref(), read_deadline, reader.frame(2048))
			.await
			.map_err(super::request::timeout_error)??;
		// The header is read in place and dropped once the stream is dispatched.
		let header = UpsRequestData::open(bytes).map_err(|_| errors::InvalidMessage.build())?;
		let header = header.get();
		if WorkerId::from_bytes(*header.worker_id()) != *ups.worker_id()
			|| header.instance_id() != ups.instance_id().as_bytes()
		{
			return Err(errors::WrongWorker.build());
		}
		let subject = header.subject();
		ups.validate(subject, &[])?;
		let class = RequestClass::from(header.class());
		// Header readers share one pool; a decoded request swaps to exactly one class or subject
		// permit, so held streams never keep a header permit.
		active.replace_permit(ups.incoming_permit(class, subject)?);

		let now = clock.observe()?;
		let _open_deadline =
			super::peer_deadline::local_deadline_ms(header.open_remaining_ms(), now)?;
		let deadline = header
			.lifetime_remaining_ms()
			.map(|remaining| super::peer_deadline::local_deadline_ms(remaining, now))
			.transpose()?;
		ups.dispatch_stream(
			class,
			WorkerId::from_bytes(*header.source_worker_id()),
			subject,
			reader.into_body(),
			deadline,
			std::sync::Arc::new(active),
		)
	}
	.await;

	match result {
		Ok(body) => Response::builder()
			.status(StatusCode::OK)
			.header("content-type", "application/octet-stream")
			.body(Body::from_stream(body))
			.unwrap(),
		Err(error) => error_status(&error).into_response(),
	}
}

fn error_status(error: &anyhow::Error) -> StatusCode {
	match RivetError::extract(error).code() {
		"no_responders" => StatusCode::NOT_FOUND,
		"wrong_worker" => StatusCode::CONFLICT,
		"overloaded" => StatusCode::TOO_MANY_REQUESTS,
		"timeout" => StatusCode::GATEWAY_TIMEOUT,
		"invalid_message" | "invalid_stream" => StatusCode::BAD_REQUEST,
		_ => StatusCode::BAD_GATEWAY,
	}
}

pub(super) fn status_error(status: StatusCode) -> anyhow::Error {
	match status {
		StatusCode::NOT_FOUND => errors::NoResponders.build(),
		StatusCode::CONFLICT => errors::WrongWorker.build(),
		StatusCode::TOO_MANY_REQUESTS => errors::Overloaded.build(),
		StatusCode::GATEWAY_TIMEOUT => errors::Timeout.build(),
		StatusCode::BAD_REQUEST | StatusCode::PAYLOAD_TOO_LARGE => errors::InvalidMessage.build(),
		StatusCode::BAD_GATEWAY => errors::ConsumerClosed.build(),
		_ => errors::Transport.build(),
	}
}

/// A bounded cause for a failed peer request, read from the transport's error chain.
pub(super) fn connection_error_reason(error: &anyhow::Error) -> &'static str {
	let text = format!("{error:#}").to_ascii_lowercase();
	if text.contains("goaway") || text.contains("go away") || text.contains("go_away") {
		"go_away"
	} else if text.contains("reset") {
		"reset"
	} else if text.contains("connect") {
		"connect"
	} else if text.contains("closed") || text.contains("broken pipe") || text.contains("eof") {
		"closed"
	} else {
		"other"
	}
}

fn transport_error(error: anyhow::Error) -> anyhow::Error {
	if error
		.downcast_ref::<rivet_env::capabilities::http::ResponseTooLarge>()
		.is_some()
	{
		errors::InvalidMessage.build().context(error)
	} else {
		errors::Transport.build().context(error)
	}
}
