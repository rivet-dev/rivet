#[tokio::main]
async fn main() -> anyhow::Result<()> {
	rivet_test_envoy::run_from_env().await
}
