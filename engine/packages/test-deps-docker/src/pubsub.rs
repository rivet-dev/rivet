use anyhow::*;
use uuid::Uuid;

use crate::DockerRunConfig;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestPubSub {
	Nats,
	Memory,
}

impl TestPubSub {
	pub fn from_env() -> Self {
		use std::result::Result::{Err, Ok};
		match std::env::var("RIVET_TEST_PUBSUB") {
			Ok(val) => match val.as_str() {
				"nats" => TestPubSub::Nats,
				"memory" => TestPubSub::Memory,
				_ => TestPubSub::Memory, // Default
			},
			Err(_) => TestPubSub::Memory, // Default
		}
	}

	pub async fn config(
		&self,
		test_id: Uuid,
		_dc_label: u16,
	) -> Result<(rivet_config::config::PubSub, Option<DockerRunConfig>)> {
		match self {
			TestPubSub::Nats => {
				let container_name = format!("test-nats-{}", test_id);

				// Check if container already exists and get its port
				let (port, existing_container_id) = if let Some(container_id) =
					crate::check_container_exists(&container_name).await?
				{
					// Get the existing port from the container
					let existing_port = crate::get_container_port(&container_name)
						.await?
						.context("failed to get port from existing NATS container")?;
					(existing_port, Some(container_id))
				} else {
					// Pick a new port for a new container
					let new_port = portpicker::pick_unused_port().context("nats port")?;
					(new_port, None)
				};

				let config =
					rivet_config::config::PubSub::Nats(rivet_config::config::pubsub::Nats {
						addresses: vec![format!("127.0.0.1:{}", port)],
						..Default::default()
					});

				let docker_config = DockerRunConfig {
					image: "nats:2.10.22-scratch".to_string(),
					container_name,
					port_mapping: (port, 4222),
					env_vars: vec![],
					container_id: existing_container_id,
				};

				Ok((config, Some(docker_config)))
			}
			TestPubSub::Memory => {
				// Use a unique channel for each test
				let config =
					rivet_config::config::PubSub::Memory(rivet_config::config::pubsub::Memory {
						channel: format!("test-{}", test_id),
					});

				Ok((config, None))
			}
		}
	}
}
