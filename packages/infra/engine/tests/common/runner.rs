use std::{path::Path, time::Duration};

use rivet_util::Id;
use tokio::process::{Child, Command};

pub struct TestRunner {
	pub runner_id: Id,
	internal_port: u16,
	pub port: u16,
	handle: Child,
}

impl TestRunner {
	pub async fn new(
		port: u16,
		namespace_name: &str,
		key: &str,
		version: u32,
		total_slots: u32,
	) -> Self {
		let internal_server_port = portpicker::pick_unused_port().expect("runner http server port");
		let http_server_port = portpicker::pick_unused_port().expect("runner http server port");

		tracing::info!(?internal_server_port, ?http_server_port, "starting runner");

		let manifest_dir = env!("CARGO_MANIFEST_DIR");
		let runner_script_path =
			Path::new(manifest_dir).join("../../../sdks/typescript/test-runner/dist/main.js");

		if !runner_script_path.exists() {
			panic!(
				"Runner script not found at '{}'. Build it first with `pnpm install && pnpm build -F @rivetkit/engine-test-runner`.",
				runner_script_path.display(),
			);
		}

		let handle = Command::new("node")
			.arg(runner_script_path)
			.env("INTERNAL_SERVER_PORT", internal_server_port.to_string())
			.env("RIVET_NAMESPACE", namespace_name)
			.env("RIVET_RUNNER_KEY", key.to_string())
			.env("RIVET_RUNNER_VERSION", version.to_string())
			.env("RIVET_RUNNER_TOTAL_SLOTS", total_slots.to_string())
			.env("RIVET_ENDPOINT", format!("http://127.0.0.1:{port}"))
			.kill_on_drop(true)
			.spawn()
			.expect("Failed to execute runner js file, node not installed");

		let runner_id = Self::wait_ready(internal_server_port).await;

		TestRunner {
			runner_id,
			internal_port: internal_server_port,
			port: http_server_port,
			handle,
		}
	}

	async fn wait_ready(port: u16) -> Id {
		let client = reqwest::Client::new();
		let mut attempts = 0;

		loop {
			let res = client
				.get(format!("http://127.0.0.1:{port}/wait-ready"))
				.send()
				.await;

			let response = match res {
				Ok(x) => x,
				Err(err) => {
					if attempts < 10 {
						attempts += 1;
						tokio::time::sleep(Duration::from_millis(150)).await;
						continue;
					} else {
						Err(err).expect("Failed to send wait ready request to runner")
					}
				}
			};

			if !response.status().is_success() {
				if attempts < 10 {
					attempts += 1;
					tokio::time::sleep(Duration::from_millis(150)).await;
					continue;
				}

				let text = response.text().await.expect("Failed to read response text");
				panic!("Failed to wait ready for runner: {text}");
			}

			return response
				.json()
				.await
				.expect("Failed to parse JSON response");
		}
	}

	pub async fn has_actor(&self, actor_id: &str) -> bool {
		let client = reqwest::Client::new();
		let response = client
			.get(format!("http://127.0.0.1:{}/has-actor", self.internal_port))
			.query(&[("actor", actor_id)])
			.send()
			.await
			.expect("Failed to send request has-actor to runner");

		if response.status() == reqwest::StatusCode::NOT_FOUND {
			return false;
		}

		if response.status().is_success() {
			return true;
		}

		let text = response.text().await.expect("Failed to fetch has-actor");
		panic!("Failed to fetch has-actor: {text}");
	}

	pub async fn shutdown(&self) {
		let client = reqwest::Client::new();
		let response = client
			.get(format!("http://127.0.0.1:{}/shutdown", self.internal_port))
			.send()
			.await
			.expect("Failed to send shutdown request to runner");

		if !response.status().is_success() {
			let text = response.text().await.expect("Failed to read response text");
			panic!("Failed to shutdown runner: {text}");
		}
	}
}
