use super::*;

mod moved_tests {
	use std::sync::Arc;
	use std::sync::Mutex;

	use super::{WebSocket, WebSocketCloseCallback, WebSocketSendCallback};
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

	#[test]
	fn close_uses_configured_callback() {
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
		ws.close(Some(1000), Some("bye".to_owned()));

		assert_eq!(
			*closed.lock().expect("closed websocket lock poisoned"),
			Some((Some(1000), Some("bye".to_owned())))
		);
	}
}
