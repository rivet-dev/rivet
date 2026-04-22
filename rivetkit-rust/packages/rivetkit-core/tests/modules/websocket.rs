use super::*;

mod moved_tests {
	use std::sync::Arc;
	use std::sync::Mutex;

	use super::{WebSocket, WebSocketCloseCallback, WebSocketSendCallback};
	use crate::ActorContext;
	use crate::actor::sleep::CanSleep;
	use crate::types::WsMessage;

	#[test]
	fn send_uses_configured_callback() {
		let sent = Arc::new(Mutex::new(Vec::<WsMessage>::new()));
		let sent_clone = sent.clone();
		let ws = WebSocket::new();
		let send_callback: WebSocketSendCallback = Arc::new(move |message| {
			sent_clone
				.lock()
				.expect("sent websocket messages lock poisoned")
				.push(message);
			Ok(())
		});

		ws.configure_send_callback(Some(send_callback));
		ws.send(WsMessage::Text("hello".to_owned()));

		assert_eq!(
			*sent.lock().expect("sent websocket messages lock poisoned"),
			vec![WsMessage::Text("hello".to_owned())]
		);
	}

	#[tokio::test]
	async fn close_uses_configured_callback() {
		let closed = Arc::new(Mutex::new(None::<(Option<u16>, Option<String>)>));
		let closed_clone = closed.clone();
		let ws = WebSocket::new();
		let close_callback: WebSocketCloseCallback = Arc::new(move |code, reason| {
			let closed_clone = closed_clone.clone();
			Box::pin(async move {
				*closed_clone.lock().expect("closed websocket lock poisoned") =
					Some((code, reason));
				Ok(())
			})
		});

		ws.configure_close_callback(Some(close_callback));
		ws.close(Some(1000), Some("bye".to_owned())).await;

		assert_eq!(
			*closed.lock().expect("closed websocket lock poisoned"),
			Some((Some(1000), Some("bye".to_owned())))
		);
	}

	#[tokio::test]
	async fn close_event_callback_region_blocks_sleep_until_callback_finishes() {
		let ctx = ActorContext::new(
			"actor-websocket-close",
			"websocket-close",
			Vec::new(),
			"local",
		);
		ctx.set_ready(true);
		ctx.set_started(true);

		let ws = WebSocket::new();
		let region_ctx = ctx.clone();
		ws.configure_close_event_callback_region(Some(Arc::new(move || {
			region_ctx.websocket_callback_region()
		})));

		let (started_tx, started_rx) = tokio::sync::oneshot::channel();
		let started_tx = Arc::new(Mutex::new(Some(started_tx)));
		let (release_tx, release_rx) = tokio::sync::oneshot::channel();
		let release_rx = Arc::new(Mutex::new(Some(release_rx)));
		ws.configure_close_event_callback(Some(Arc::new(move |_, _, _| {
			let started_tx = started_tx.clone();
			let release_rx = release_rx.clone();
			Box::pin(async move {
				if let Some(started_tx) = started_tx
					.lock()
					.expect("started sender lock poisoned")
					.take()
				{
					let _ = started_tx.send(());
				}
				let release_rx = release_rx
					.lock()
					.expect("release receiver lock poisoned")
					.take()
					.expect("release receiver should be present");
				let _ = release_rx.await;
				Ok(())
			})
		})));

		let task = tokio::spawn({
			let ws = ws.clone();
			async move {
				ws.dispatch_close_event(1000, "normal".to_owned(), true)
					.await;
			}
		});

		started_rx.await.expect("close event callback should start");
		assert_eq!(ctx.can_sleep().await, CanSleep::ActiveWebSocketCallbacks);

		release_tx
			.send(())
			.expect("close event callback should still be waiting");
		task.await.expect("close event callback should join");

		assert_eq!(ctx.can_sleep().await, CanSleep::Yes);
	}
}
