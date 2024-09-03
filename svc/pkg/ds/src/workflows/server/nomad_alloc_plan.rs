use std::time::Duration;

use chirp_workflow::prelude::*;

use crate::util::{signal_allocation, NOMAD_CONFIG, NOMAD_REGION};

// TODO:
const TRAEFIK_GRACE_PERIOD: Duration = Duration::from_secs(2);

#[derive(Debug, Serialize, Deserialize)]
pub struct Input {
	pub server_id: Uuid,
	pub alloc: nomad_client::models::Allocation,
}

#[workflow]
pub async fn ds_server_nomad_alloc_plan(ctx: &mut WorkflowCtx, input: &Input) -> GlobalResult<()> {
	let alloc_id = unwrap_ref!(input.alloc.ID);
	let nomad_node_id = unwrap_ref!(input.alloc.node_id, "alloc has no node id");

	let node_res = ctx
		.activity(FetchNodeInfoInput {
			nomad_node_id: nomad_node_id.clone(),
		})
		.await?;

	// Wait for Traefik to be ready
	ctx.sleep(TRAEFIK_GRACE_PERIOD).await?;

	// Read ports
	let mut ports = Vec::new();
	let alloc_resources = unwrap_ref!(input.alloc.resources);
	if let Some(networks) = &alloc_resources.networks {
		for network in networks {
			let network_ip = unwrap_ref!(network.IP);

			if let Some(dynamic_ports) = &network.dynamic_ports {
				for port in dynamic_ports {
					// Don't share connect proxy ports
					let label = unwrap_ref!(port.label);
					ports.push(Port {
						label: label.clone(),
						source: *unwrap_ref!(port.value) as u32,
						target: *unwrap_ref!(port.to) as u32,
						ip: network_ip.clone(),
					});
				}
			}
		}
	} else {
		tracing::info!("no network on alloc");
	}

	let db_res = ctx
		.activity(UpdateDbInput {
			server_id: input.server_id,
			alloc_id: alloc_id.clone(),
			nomad_node_id: nomad_node_id.clone(),
			nomad_node_name: node_res.name,
			nomad_node_public_ipv4: node_res.public_ipv4,
			nomad_node_vlan_ipv4: node_res.vlan_ipv4,
			ports,
		})
		.await?;

	if db_res.kill_alloc {
		ctx.activity(KillAllocInput {
			alloc_id: alloc_id.clone(),
		})
		.await?;
	}

	Ok(())
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct FetchNodeInfoInput {
	nomad_node_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct FetchNodeInfoOutput {
	name: String,
	public_ipv4: String,
	vlan_ipv4: String,
}

#[activity(FetchNodeInfo)]
async fn fetch_node_info(
	ctx: &ActivityCtx,
	input: &FetchNodeInfoInput,
) -> GlobalResult<FetchNodeInfoOutput> {
	// Fetch node metadata
	let node = nomad_client::apis::nodes_api::get_node(
		&NOMAD_CONFIG,
		&input.nomad_node_id,
		None,
		None,
		None,
		None,
		None,
		None,
		None,
		None,
		None,
	)
	.await?;
	let mut meta = unwrap!(node.meta);

	Ok(FetchNodeInfoOutput {
		name: unwrap!(node.name),
		public_ipv4: unwrap!(meta.remove("network-public-ipv4")),
		vlan_ipv4: unwrap!(meta.remove("network-vlan-ipv4")),
	})
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct UpdateDbInput {
	server_id: Uuid,
	alloc_id: String,
	nomad_node_id: String,
	nomad_node_name: String,
	nomad_node_public_ipv4: String,
	nomad_node_vlan_ipv4: String,
	ports: Vec<Port>,
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct Port {
	label: String,
	source: u32,
	target: u32,
	ip: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct UpdateDbOutput {
	kill_alloc: bool,
}

#[activity(UpdateDb)]
async fn update_db(ctx: &ActivityCtx, input: &UpdateDbInput) -> GlobalResult<UpdateDbOutput> {
	let mut flat_port_labels = Vec::new();
	let mut flat_port_sources = Vec::new();
	let mut flat_port_ips = Vec::new();

	for port in &input.ports {
		flat_port_labels.push(port.label.as_str());
		flat_port_sources.push(port.source as i64);
		flat_port_ips.push(port.ip.as_str());
	}

	let (datacenter_id, nomad_alloc_id, updated) = sql_fetch_one!(
		[ctx, (Uuid, Option<String>, bool)]
		"
		WITH
			select_server AS (
				SELECT s.datacenter_id, sn.nomad_alloc_id
				FROM db_ds.server_nomad AS sn
				INNER JOIN db_ds.servers AS s
				ON s.server_id = sn.server_id
				WHERE sn.server_id = $1
			),
			update_server AS (
				UPDATE db_ds.servers
				SET connectable_ts = $2
				WHERE
					server_id = $1 AND
					connectable_ts IS NULL
				RETURNING 1
			),
			update_server_nomad AS (
				UPDATE db_ds.server_nomad
				SET
					nomad_alloc_id = $3,
					nomad_alloc_plan_ts = $2,
					nomad_node_id = $4,
					nomad_node_name = $5,
					nomad_node_public_ipv4 = $6,
					nomad_node_vlan_ipv4 = $7
				WHERE
					server_id = $1 AND
					nomad_alloc_plan_ts IS NULL
				RETURNING 1
			),
			insert_ports AS (
				INSERT INTO db_ds.internal_ports (
					server_id,
					nomad_label,
					nomad_source,
					nomad_ip
				)
				SELECT $1, label, source, ip
				FROM unnest($8, $9, $10) AS n(label, source, ip)
				WHERE EXISTS(
					SELECT 1 FROM update_server_nomad
				)
				RETURNING 1
			)
		SELECT datacenter_id, nomad_alloc_id, EXISTS(SELECT 1 FROM update_server_nomad) AS updated
		FROM select_server
		",
		input.server_id,
		util::timestamp::now(),
		&input.alloc_id,
		&input.nomad_node_id,
		&input.nomad_node_name, // 5
		&input.nomad_node_public_ipv4,
		&input.nomad_node_vlan_ipv4,
		flat_port_labels,
		flat_port_sources,
		flat_port_ips, // 10
	)
	.await?;

	if !updated {
		tracing::warn!("alloc was already planned");
	}
	// Invalidate cache when ports are updated
	else if !input.ports.is_empty() {
		ctx.cache().purge("servers_ports", [datacenter_id]).await?;
	}

	let kill_alloc = nomad_alloc_id
		.as_ref()
		.map(|id| id != &input.alloc_id)
		.unwrap_or_default();

	if kill_alloc {
		tracing::warn!(server_id=%input.server_id, existing_alloc_id=?nomad_alloc_id, new_alloc_id=%input.alloc_id, "different allocation id given, killing new allocation");
	}

	Ok(UpdateDbOutput { kill_alloc })
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct KillAllocInput {
	alloc_id: String,
}

#[activity(KillAlloc)]
async fn kill_alloc(ctx: &ActivityCtx, input: &KillAllocInput) -> GlobalResult<()> {
	if let Err(err) = signal_allocation(
		&NOMAD_CONFIG,
		&input.alloc_id,
		None,
		Some(NOMAD_REGION),
		None,
		None,
		Some(nomad_client_old::models::AllocSignalRequest {
			task: None,
			signal: Some("SIGKILL".to_string()),
		}),
	)
	.await
	{
		tracing::warn!(
			?err,
			?input.alloc_id,
			"error while trying to manually kill allocation"
		);
	}

	Ok(())
}
