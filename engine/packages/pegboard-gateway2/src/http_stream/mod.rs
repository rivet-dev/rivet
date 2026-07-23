use rivet_envoy_protocol as protocol;
use tokio::sync::mpsc;

use crate::shared_state::{InFlightRequestHandle, RequestStopResult};

mod request;
mod response;

pub(super) use request::{
	send_to_envoy_or_actor_stopped, should_stream_http_request_body_hint,
	stream_http_request_and_wait_for_response, wait_for_http_response_start,
};
pub(super) use response::drain_http_response_stream;

pub(super) const HTTP_RESPONSE_BODY_CHANNEL_CAPACITY: usize = 16;
pub(super) type ResponseBodyError = Box<dyn std::error::Error + Send + Sync>;

pub(super) struct HttpClientDisconnectGuard {
	in_flight_req: Option<InFlightRequestHandle>,
}

impl HttpClientDisconnectGuard {
	pub(super) fn new(in_flight_req: InFlightRequestHandle) -> Self {
		Self {
			in_flight_req: Some(in_flight_req),
		}
	}

	pub(super) fn disarm(&mut self) {
		self.in_flight_req = None;
	}
}

impl Drop for HttpClientDisconnectGuard {
	fn drop(&mut self) {
		let Some(in_flight_req) = self.in_flight_req.take() else {
			return;
		};
		tokio::spawn(async move {
			send_http_request_abort(
				&in_flight_req,
				protocol::HttpStreamAbortReasonKind::ClientDisconnect,
				Some("client disconnected before the HTTP response started".to_owned()),
			)
			.await;
			in_flight_req
				.stop(RequestStopResult::ClientDisconnect)
				.await;
		});
	}
}

pub(super) async fn send_http_request_abort(
	in_flight_req: &InFlightRequestHandle,
	kind: protocol::HttpStreamAbortReasonKind,
	detail: impl Into<Option<String>>,
) {
	let message = protocol::ToEnvoyTunnelMessageKind::ToEnvoyRequestAbort(
		protocol::ToEnvoyRequestAbort {
			reason: protocol::HttpStreamAbortReason {
				kind,
				detail: detail.into(),
			},
		},
	);
	if let Err(err) = in_flight_req.send_message(message, true).await {
		tracing::debug!(?err, "failed sending http request abort to envoy");
	}
}

fn send_http_body_error(
	body_tx: &mpsc::Sender<Result<bytes::Bytes, ResponseBodyError>>,
	message: impl Into<String>,
) {
	let _ = body_tx.try_send(Err(Box::new(std::io::Error::other(message.into()))));
}
