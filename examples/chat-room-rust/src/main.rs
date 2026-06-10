#[tokio::main]
async fn main() -> anyhow::Result<()> {
	example_chat_room_rust::registry().start().await
}
