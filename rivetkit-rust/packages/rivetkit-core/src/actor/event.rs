use std::time::Duration;

use http::StatusCode;
use tokio::time::sleep;

use crate::actor::callbacks::{ActorInstanceCallbacks, OnRequestRequest, OnWebSocketRequest, Request, Response};
use crate::actor::connection::ConnHandle;
use crate::actor::context::ActorContext;
use crate::actor::sleep::CanSleep;
use crate::websocket::WebSocket;

fn rearm_sleep_after_http_request(ctx: &ActorContext) {
	let sleep_ctx = ctx.clone();
	ctx.wait_until(async move {
		while sleep_ctx.can_sleep().await == CanSleep::ActiveHttpRequests {
			sleep(Duration::from_millis(10)).await;
		}
		sleep_ctx.reset_sleep_timer();
	});
}

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

	ctx.cancel_sleep_timer();

	match handler(OnRequestRequest {
		ctx: ctx.clone(),
		request,
	})
	.await
	{
		Ok(response) => {
			rearm_sleep_after_http_request(&ctx);
			ctx.request_sleep_if_pending();
			response
		}
		Err(error) => {
			tracing::error!(?error, "error in on_request callback");
			rearm_sleep_after_http_request(&ctx);
			ctx.request_sleep_if_pending();
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
					conn: None,
					ws: ws.clone(),
					request: None,
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
