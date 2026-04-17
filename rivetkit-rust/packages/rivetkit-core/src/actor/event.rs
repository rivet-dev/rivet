use http::StatusCode;

use crate::actor::callbacks::{ActorInstanceCallbacks, OnRequestRequest, OnWebSocketRequest, Request, Response};
use crate::actor::connection::ConnHandle;
use crate::actor::context::ActorContext;
use crate::websocket::WebSocket;

#[derive(Clone, Debug, Default)]
pub struct EventBroadcaster;

impl EventBroadcaster {
	pub fn broadcast(&self, connections: &[ConnHandle], name: &str, args: &[u8]) {
		for connection in connections {
			if connection.is_subscribed(name) {
				connection.send(name, args);
			}
		}
	}
}

#[allow(dead_code)]
pub(crate) async fn dispatch_request(
	callbacks: &ActorInstanceCallbacks,
	ctx: ActorContext,
	request: Request,
) -> Response {
	let Some(handler) = &callbacks.on_request else {
		return Response::from(
			http::Response::builder()
				.status(StatusCode::NOT_FOUND)
				.body(b"not found".to_vec())
				.expect("404 response should be valid"),
		);
	};

	match handler(OnRequestRequest { ctx, request }).await {
		Ok(response) => response,
		Err(error) => {
			tracing::error!(?error, "error in on_request callback");
			Response::from(
				http::Response::builder()
					.status(StatusCode::INTERNAL_SERVER_ERROR)
					.body(b"internal server error".to_vec())
					.expect("500 response should be valid"),
			)
		}
	}
}

#[allow(dead_code)]
pub(crate) async fn dispatch_websocket(
	callbacks: &ActorInstanceCallbacks,
	ctx: ActorContext,
	ws: WebSocket,
) {
	let Some(handler) = &callbacks.on_websocket else {
		ws.close(Some(1000), Some("websocket handler not configured".to_owned()));
		return;
	};

	let result = ctx
		.with_websocket_callback(|| async {
			handler(OnWebSocketRequest {
				ctx: ctx.clone(),
				ws: ws.clone(),
			})
			.await
		})
		.await;

	if let Err(error) = result {
		tracing::error!(?error, "error in on_websocket callback");
		ws.close(Some(1011), Some("Server Error".to_owned()));
	}
}

#[cfg(test)]
#[path = "../../tests/modules/event.rs"]
mod tests;
