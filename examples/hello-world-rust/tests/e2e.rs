use rivetkit::client::GetOrCreateOptions;
use rivetkit::test;
use serde_json::json;
use tokio::sync::oneshot;

#[tokio::test(flavor = "multi_thread")]
async fn counter_actor_works_through_rust_client() -> anyhow::Result<()> {
	let h = test::setup(example_hello_world_rust::registry()).await?;
	let counter =
		h.actor::<example_hello_world_rust::Counter>(example_hello_world_rust::ACTOR_NAME);

	assert_eq!(
		counter
			.send(example_hello_world_rust::Increment { amount: 1 })
			.await?,
		1
	);
	assert_eq!(
		counter
			.send(example_hello_world_rust::Increment { amount: 1 })
			.await?,
		2
	);
	assert_eq!(counter.send(example_hello_world_rust::GetCount).await?, 2);

	let (event_tx, event_rx) = oneshot::channel();
	let event_tx = std::sync::Arc::new(std::sync::Mutex::new(Some(event_tx)));
	let connection = counter.connect();
	connection
		.on::<example_hello_world_rust::NewCount>({
			let event_tx = std::sync::Arc::clone(&event_tx);
			move |event| {
				if let Some(event_tx) = event_tx.lock().expect("event lock poisoned").take() {
					let _ = event_tx.send(event.count);
				}
			}
		})
		.await;

	assert_eq!(
		connection
			.send(example_hello_world_rust::Increment { amount: 4 })
			.await?,
		6
	);
	assert_eq!(event_rx.await?, 6);
	connection.disconnect().await;

	let with_params = h.actor_with_options::<example_hello_world_rust::Counter>(
		example_hello_world_rust::ACTOR_NAME,
		["conn-state"],
		GetOrCreateOptions {
			params: Some(json!({ "label": "from-conn" })),
			..Default::default()
		},
	);
	let connection = with_params.connect();
	assert_eq!(
		connection
			.send(example_hello_world_rust::GetConnLabel)
			.await?,
		"from-conn"
	);
	connection.disconnect().await;

	h.shutdown().await;
	Ok(())
}
