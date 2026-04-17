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
		return http::Response::builder()
			.status(StatusCode::NOT_FOUND)
			.body(b"not found".to_vec())
			.expect("404 response should be valid");
	};

	match handler(OnRequestRequest { ctx, request }).await {
		Ok(response) => response,
		Err(error) => {
			tracing::error!(?error, "error in on_request callback");
			http::Response::builder()
				.status(StatusCode::INTERNAL_SERVER_ERROR)
				.body(b"internal server error".to_vec())
				.expect("500 response should be valid")
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
mod tests {
	use std::sync::Arc;
	use std::sync::Mutex;

	use anyhow::Result;
	use futures::future::BoxFuture;
	use rivet_error::INTERNAL_ERROR;

	use super::{EventBroadcaster, dispatch_request, dispatch_websocket};
	use crate::actor::callbacks::{ActorInstanceCallbacks, RequestCallback};
	use crate::actor::connection::{ConnHandle, EventSendCallback, OutgoingEvent};
	use crate::actor::context::ActorContext;
	use crate::websocket::{WebSocket, WebSocketCloseCallback};

	fn request_callback<F>(callback: F) -> RequestCallback
	where
		F: Fn(
				crate::actor::callbacks::OnRequestRequest,
			) -> BoxFuture<'static, Result<crate::actor::callbacks::Response>>
			+ Send
			+ Sync
			+ 'static,
	{
		Box::new(callback)
	}

	#[test]
	fn broadcaster_only_fans_out_to_subscribed_connections() {
		let sent = Arc::new(Mutex::new(Vec::<(String, OutgoingEvent)>::new()));
		let sent_clone = sent.clone();
		let subscribed = ConnHandle::new("subscribed", Vec::new(), Vec::new(), false);
		let idle = ConnHandle::new("idle", Vec::new(), Vec::new(), false);

		let sender: EventSendCallback = Arc::new(move |event| {
			sent_clone
				.lock()
				.expect("sent events lock poisoned")
				.push(("subscribed".to_owned(), event));
			Ok(())
		});

		subscribed.configure_event_sender(Some(sender));
		subscribed.subscribe("updated");

		EventBroadcaster::default().broadcast(&[subscribed, idle], "updated", b"payload");

		assert_eq!(
			*sent.lock().expect("sent events lock poisoned"),
			vec![(
				"subscribed".to_owned(),
				OutgoingEvent {
					name: "updated".to_owned(),
					args: b"payload".to_vec(),
				},
			)]
		);
	}

	#[tokio::test]
	async fn request_dispatch_returns_callback_response() {
		let mut callbacks = ActorInstanceCallbacks::default();
		callbacks.on_request = Some(request_callback(|request| {
			Box::pin(async move {
				assert_eq!(request.request.uri().path(), "/ok");
				Ok(
					http::Response::builder()
						.status(http::StatusCode::ACCEPTED)
						.body(b"ok".to_vec())
						.expect("accepted response should build"),
				)
			})
		}));

		let response = dispatch_request(
			&callbacks,
			ActorContext::default(),
			http::Request::builder()
				.uri("/ok")
				.body(Vec::new())
				.expect("request should build"),
		)
		.await;

		assert_eq!(response.status(), http::StatusCode::ACCEPTED);
		assert_eq!(response.body(), b"ok");
	}

	#[tokio::test]
	async fn request_dispatch_returns_500_on_error() {
		let mut callbacks = ActorInstanceCallbacks::default();
		callbacks.on_request = Some(request_callback(|_| {
			Box::pin(async move { Err(INTERNAL_ERROR.build()) })
		}));

		let response = dispatch_request(
			&callbacks,
			ActorContext::default(),
			http::Request::builder()
				.uri("/boom")
				.body(Vec::new())
				.expect("request should build"),
		)
		.await;

		assert_eq!(response.status(), http::StatusCode::INTERNAL_SERVER_ERROR);
		assert_eq!(response.body(), b"internal server error");
	}

	#[tokio::test]
	async fn websocket_dispatch_closes_on_callback_error() {
		let closed = Arc::new(Mutex::new(None::<(Option<u16>, Option<String>)>));
		let closed_clone = closed.clone();
		let ws = WebSocket::new();
		let close_callback: WebSocketCloseCallback = Arc::new(move |code, reason| {
			*closed_clone
				.lock()
				.expect("closed websocket lock poisoned") = Some((code, reason));
			Ok(())
		});
		ws.configure_close_callback(Some(close_callback));

		let mut callbacks = ActorInstanceCallbacks::default();
		callbacks.on_websocket = Some(Box::new(|_| {
			Box::pin(async move { Err(INTERNAL_ERROR.build()) })
		}));

		dispatch_websocket(&callbacks, ActorContext::default(), ws).await;

		assert_eq!(
			*closed.lock().expect("closed websocket lock poisoned"),
			Some((Some(1011), Some("Server Error".to_owned())))
		);
	}
}
