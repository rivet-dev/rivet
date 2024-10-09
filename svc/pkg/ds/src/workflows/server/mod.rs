use std::collections::HashMap;

use chirp_workflow::prelude::*;

use crate::types::{GameClient, NetworkMode, Routing, ServerResources};

pub mod nomad;
pub mod pegboard;

// In ms, a small amount of time to separate the completion of the drain to the deletion of the
// cluster server. We want the drain to complete first.
const DRAIN_PADDING_MS: i64 = 10000;

#[derive(Debug, Serialize, Deserialize)]
pub struct Input {
	pub server_id: Uuid,
	pub env_id: Uuid,
	pub datacenter_id: Uuid,
	pub cluster_id: Uuid,
	pub client: GameClient,
	pub tags: HashMap<String, String>,
	pub resources: ServerResources,
	pub kill_timeout_ms: i64,
	pub image_id: Uuid,
	pub args: Vec<String>,
	pub network_mode: NetworkMode,
	pub environment: HashMap<String, String>,
	pub network_ports: HashMap<String, Port>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub struct Port {
	// Null when using host networking since one is automatically assigned
	pub internal_port: Option<i32>,
	pub routing: Routing,
}

#[workflow]
pub async fn ds_server(ctx: &mut WorkflowCtx, input: &Input) -> GlobalResult<()> {
	match input.client {
		GameClient::Nomad => {
			ctx.workflow(nomad::Input {
				server_id: input.server_id,
				env_id: input.env_id,
				datacenter_id: input.datacenter_id,
				cluster_id: input.cluster_id,
				tags: input.tags.clone(),
				resources: input.resources.clone(),
				kill_timeout_ms: input.kill_timeout_ms,
				image_id: input.image_id,
				args: input.args.clone(),
				network_mode: input.network_mode,
				environment: input.environment.clone(),
				network_ports: input.network_ports.clone(),
			})
			.output()
			.await
		}
		GameClient::Pegboard => {
			ctx.workflow(pegboard::Input {
				server_id: input.server_id,
				env_id: input.env_id,
				datacenter_id: input.datacenter_id,
				cluster_id: input.cluster_id,
				tags: input.tags.clone(),
				resources: input.resources.clone(),
				kill_timeout_ms: input.kill_timeout_ms,
				image_id: input.image_id,
				args: input.args.clone(),
				network_mode: input.network_mode,
				environment: input.environment.clone(),
				network_ports: input.network_ports.clone(),
			})
			.output()
			.await
		}
	}
}

#[message("ds_server_create_complete")]
pub struct CreateComplete {}

#[message("ds_server_create_failed")]
pub struct CreateFailed {}

#[signal("ds_server_destroy")]
pub struct Destroy {
	pub override_kill_timeout_ms: Option<i64>,
}

#[signal("ds_server_drain")]
pub struct Drain {
	pub drain_timeout: i64,
}

#[signal("ds_server_undrain")]
pub struct Undrain {}

#[rustfmt::skip]
join_signal!(DrainState {
	Undrain,
	Destroy,
});
