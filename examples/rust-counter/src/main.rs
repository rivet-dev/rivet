use anyhow::Result;
use rivetkit_core::ServeConfig;
use tokio_util::sync::CancellationToken;

#[tokio::main]
async fn main() -> Result<()> {
	let _ = tracing_subscriber::fmt().with_env_filter("info").try_init();

	let config = ServeConfig::from_env();
	let shutdown = CancellationToken::new();
	let registry = rivetkit_rust_counter_example::registry();
	let serve = tokio::spawn({
		let shutdown = shutdown.clone();
		async move { registry.serve_with_config(config, shutdown).await }
	});

	tokio::signal::ctrl_c().await?;
	shutdown.cancel();
	serve.await??;

	Ok(())
}
