use rivetkit::test;

#[tokio::test(flavor = "multi_thread")]
async fn chat_room_persists_history_through_rust_client() -> anyhow::Result<()> {
	let h = test::setup(example_chat_room_rust::registry()).await?;
	let room = h.actor::<example_chat_room_rust::ChatRoom>(example_chat_room_rust::ACTOR_NAME);

	assert_eq!(room.send(example_chat_room_rust::GetHistory).await?, vec![]);

	let first = room
		.send(example_chat_room_rust::SendMessage {
			sender: "Alice".to_owned(),
			text: "Hello everyone!".to_owned(),
		})
		.await?;
	assert_eq!(first.sender, "Alice");
	assert_eq!(first.text, "Hello everyone!");
	assert!(first.timestamp > 0);

	room.send(example_chat_room_rust::SendMessage {
		sender: "Bob".to_owned(),
		text: "Hi Alice!".to_owned(),
	})
	.await?;

	let messages = room.send(example_chat_room_rust::GetHistory).await?;
	assert_eq!(messages.len(), 2);
	assert_eq!(messages[0].sender, "Alice");
	assert_eq!(messages[1].sender, "Bob");

	let stats = room.send(example_chat_room_rust::GetStats).await?;
	assert_eq!(stats.sent_count, 2);
	assert!(stats.started_at_ms > 0);

	h.shutdown().await;
	Ok(())
}
