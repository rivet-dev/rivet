use tokio::sync::broadcast::error::TryRecvError;

use crate::client::Client;

/// `dispose` sends explicitly and then the guard sends again on drop. The
/// channel has capacity 1, so the receiver may report `Lagged` instead of `Ok`.
/// Both mean shutdown was signalled.
fn signalled(rx: &mut tokio::sync::broadcast::Receiver<()>) -> bool {
	matches!(rx.try_recv(), Ok(()) | Err(TryRecvError::Lagged(_)))
}

#[test]
fn dropping_a_clone_does_not_signal_shutdown() {
	let client = Client::from_endpoint("http://127.0.0.1:1");
	let mut shutdown_rx = client.shutdown.shutdown_tx.subscribe();

	let clone = client.clone();
	drop(clone);

	assert!(
		matches!(shutdown_rx.try_recv(), Err(TryRecvError::Empty)),
		"dropping a clone must not signal shutdown while another Client is alive"
	);
	drop(client);
}

#[test]
fn dropping_the_last_client_signals_shutdown() {
	let client = Client::from_endpoint("http://127.0.0.1:1");
	let mut shutdown_rx = client.shutdown.shutdown_tx.subscribe();

	let clone = client.clone();
	drop(client);
	assert!(matches!(shutdown_rx.try_recv(), Err(TryRecvError::Empty)));

	drop(clone);
	assert!(matches!(shutdown_rx.try_recv(), Ok(())));
}

#[test]
fn dispose_on_the_last_client_signals_shutdown() {
	let client = Client::from_endpoint("http://127.0.0.1:1");
	let mut shutdown_rx = client.shutdown.shutdown_tx.subscribe();

	client.dispose();
	assert!(signalled(&mut shutdown_rx));
}

#[test]
fn dispose_on_a_clone_signals_shutdown_for_every_clone() {
	let client = Client::from_endpoint("http://127.0.0.1:1");
	let mut shutdown_rx = client.shutdown.shutdown_tx.subscribe();

	let clone = client.clone();
	clone.dispose();
	assert!(signalled(&mut shutdown_rx));
	drop(client);
}

#[test]
fn actor_handle_keeps_shutdown_from_firing_until_it_drops() {
	let client = Client::from_endpoint("http://127.0.0.1:1");
	let mut shutdown_rx = client.shutdown.shutdown_tx.subscribe();

	let handle = client
		.get("counter", Vec::new(), Default::default())
		.expect("handle");
	drop(client);
	assert!(
		matches!(shutdown_rx.try_recv(), Err(TryRecvError::Empty)),
		"a live ActorHandle must keep the client's connections alive"
	);

	drop(handle);
	assert!(matches!(shutdown_rx.try_recv(), Ok(())));
}
