use std::collections::HashMap;
use testcontainers::{Container, Docker, Image, WaitForMessage};

const CONTAINER_IDENTIFIER: &str = "nats";
const DEFAULT_TAG: &str = "2.2.0";

#[derive(Debug, Default)]
pub struct Nats;

impl Image for Nats {
	type Args = Vec<String>;
	type EnvVars = HashMap<String, String>;
	type Volumes = HashMap<String, String>;
	type EntryPoint = std::convert::Infallible;

	fn descriptor(&self) -> String {
		format!("{}:{}", CONTAINER_IDENTIFIER, DEFAULT_TAG)
	}

	fn wait_until_ready<D: Docker>(&self, container: &Container<'_, D, Self>) {
		container
			.logs()
			.stderr
			.wait_for_message("Listening for route connections on")
			.unwrap();
	}

	fn args(&self) -> <Self as Image>::Args {
		Vec::new()
	}

	fn volumes(&self) -> Self::Volumes {
		HashMap::new()
	}

	fn env_vars(&self) -> Self::EnvVars {
		HashMap::new()
	}

	fn with_args(self, _arguments: <Self as Image>::Args) -> Self {
		Nats
	}
}
