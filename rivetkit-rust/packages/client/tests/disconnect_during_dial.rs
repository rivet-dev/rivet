use std::time::Duration;

use rivetkit_client::{Client, ClientConfig, GetOrCreateOptions};
use tokio::{io::AsyncReadExt, net::TcpListener, time::timeout};

#[tokio::test]
async fn disconnect_cancels_a_dial_in_flight() {
	// The server accepts the dial and never answers the WebSocket upgrade, like a gateway that
	// waits for an actor to be scheduled.
	let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
	let port = listener.local_addr().unwrap().port();
	let client = Client::new(
		ClientConfig::new(format!("http://127.0.0.1:{port}")).disable_metadata_lookup(true),
	);
	let conn = client
		.get_or_create("counter", vec!["a".into()], GetOrCreateOptions::default())
		.unwrap()
		.connect();

	let (mut socket, _) = timeout(Duration::from_secs(5), listener.accept())
		.await
		.expect("the client did not dial")
		.unwrap();

	timeout(Duration::from_secs(5), conn.disconnect())
		.await
		.expect("disconnect() waited for the dial to finish");

	let mut buf = [0u8; 4096];
	let closed = timeout(Duration::from_secs(5), async {
		loop {
			match socket.read(&mut buf).await {
				Ok(0) | Err(_) => break,
				Ok(_) => continue,
			}
		}
	})
	.await;
	assert!(closed.is_ok(), "the dial stayed open after disconnect()");
}
