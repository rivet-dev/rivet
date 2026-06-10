#[tokio::main]
async fn main() -> anyhow::Result<()> {
	example_hello_world_rust::registry().start().await
}
