#[tokio::main]
async fn main() -> anyhow::Result<()> {
	example_ai_agent_rust::registry().start().await
}
