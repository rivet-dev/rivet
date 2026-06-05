use rivetkit::test;
use serde_json::{Value as JsonValue, json};

#[tokio::test(flavor = "multi_thread")]
async fn chat_room_persists_history_through_rust_client() -> anyhow::Result<()> {
	let h = test::setup(example_chat_room_rust::registry()).await?;
	let room = h.actor(example_chat_room_rust::ACTOR_NAME);

	assert_eq!(room.action("getHistory", vec![]).await?, json!([]));

	let first = room
		.action(
			"sendMessage",
			vec![json!("Alice"), json!("Hello everyone!")],
		)
		.await?;
	assert_eq!(first.get("sender"), Some(&json!("Alice")));
	assert_eq!(first.get("text"), Some(&json!("Hello everyone!")));
	assert!(
		first
			.get("timestamp")
			.and_then(JsonValue::as_i64)
			.is_some_and(|ts| ts > 0)
	);

	room.action("sendMessage", vec![json!("Bob"), json!("Hi Alice!")])
		.await?;

	let history = room.action("getHistory", vec![]).await?;
	let messages = history.as_array().expect("history is an array");
	assert_eq!(messages.len(), 2);
	assert_eq!(messages[0].get("sender"), Some(&json!("Alice")));
	assert_eq!(messages[1].get("sender"), Some(&json!("Bob")));

	h.shutdown().await;
	Ok(())
}
