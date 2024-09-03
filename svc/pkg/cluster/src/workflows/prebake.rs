use chirp_workflow::prelude::*;
use serde_json::json;

use crate::{
	types::{PoolType, Provider},
	workflows::server::{GetDcInput, Linode},
};

#[derive(Debug, Serialize, Deserialize)]
pub struct Input {
	pub datacenter_id: Uuid,
	pub provider: Provider,
	pub pool_type: PoolType,
	pub install_script_hash: String,
	pub tags: Vec<String>,
}

#[workflow]
pub async fn cluster_prebake(ctx: &mut WorkflowCtx, input: &Input) -> GlobalResult<()> {
	let dc = ctx
		.activity(GetDcInput {
			datacenter_id: input.datacenter_id,
		})
		.await?;

	let prebake_server_id = ctx.activity(GenerateServerIdInput {}).await?;

	let mut tags = input.tags.clone();
	tags.push("prebake".to_string());

	match input.provider {
		Provider::Linode => {
			let workflow_id = ctx
				.workflow(linode::workflows::server::Input {
					server_id: prebake_server_id,
					provider_datacenter_id: dc.provider_datacenter_id.clone(),
					custom_image: None,
					api_token: dc.provider_api_token.clone(),
					hardware: linode::util::consts::PREBAKE_HARDWARE.to_string(),
					firewall_preset: match input.pool_type {
						PoolType::Job => linode::types::FirewallPreset::Job,
						PoolType::Gg => linode::types::FirewallPreset::Gg,
						PoolType::Ats => linode::types::FirewallPreset::Ats,
					},
					vlan_ip: None,
					tags,
				})
				.tag("server_id", prebake_server_id)
				.dispatch()
				.await?;

			match ctx.listen::<Linode>().await? {
				Linode::ProvisionComplete(sig) => {
					// Install server
					ctx.workflow(crate::workflows::server::install::Input {
						datacenter_id: input.datacenter_id,
						server_id: None,
						public_ip: sig.public_ip,
						pool_type: input.pool_type.clone(),
						initialize_immediately: false,
					})
					.run()
					.await?;

					// Create image
					let workflow_id = ctx
						.workflow(linode::workflows::image::Input {
							prebake_server_id,
							api_token: dc.provider_api_token.clone(),
							linode_id: sig.linode_id,
							boot_disk_id: sig.boot_disk_id,
						})
						.tag("linode_id", sig.linode_id)
						.dispatch()
						.await?;

					// Wait for image creation
					let image_create_res = ctx
						.listen::<linode::workflows::image::CreateComplete>()
						.await?;

					// Write image id to db
					ctx.activity(UpdateDbInput {
						provider: input.provider.clone(),
						datacenter_id: input.datacenter_id,
						pool_type: input.pool_type.clone(),
						install_script_hash: input.install_script_hash.clone(),
						image_id: image_create_res.image_id,
					})
					.await?;

					// Destroy linode server after the image is complete
					ctx.signal(linode::workflows::server::Destroy {})
						.tag("server_id", prebake_server_id)
						.send()
						.await?;

					// Wait for image workflow to get cleaned up by linode-gc after the image expires
					ctx.wait_for_workflow::<linode::workflows::server::Workflow>(workflow_id)
						.await?;
				}
				Linode::ProvisionFailed(_) => {
					tracing::error!(
						provision_workflow_id=%workflow_id,
						"failed to provision prebake server"
					);
				}
			}
		}
	}

	ctx.activity(SetDestroyedInput {
		provider: input.provider.clone(),
		datacenter_id: input.datacenter_id,
		pool_type: input.pool_type.clone(),
		install_script_hash: input.install_script_hash.clone(),
	})
	.await?;
	// UPDATE server_images2
	// SET destroy_ts = $1
	// WHERE =

	Ok(())
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct GenerateServerIdInput {}

#[activity(GenerateServerId)]
async fn generate_server_id(
	ctx: &ActivityCtx,
	input: &GenerateServerIdInput,
) -> GlobalResult<Uuid> {
	let prebake_server_id = Uuid::new_v4();

	ctx.update_workflow_tags(&json!({
		"server_id": prebake_server_id,
	}))
	.await?;

	Ok(prebake_server_id)
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct UpdateDbInput {
	datacenter_id: Uuid,
	provider: Provider,
	pool_type: PoolType,
	install_script_hash: String,
	image_id: String,
}

#[activity(UpdateDb)]
async fn update_db(ctx: &ActivityCtx, input: &UpdateDbInput) -> GlobalResult<()> {
	sql_execute!(
		[ctx]
		"
		UPDATE db_cluster.server_images2
		SET provider_image_id = $5
		WHERE
			provider = $1 AND
			install_hash = $2 AND
			datacenter_id = $3 AND
			pool_type = $4
		",
		serde_json::to_string(&input.provider)?,
		&input.install_script_hash,
		input.datacenter_id,
		serde_json::to_string(&input.pool_type)?,
		&input.image_id,
	)
	.await?;

	Ok(())
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct SetDestroyedInput {
	datacenter_id: Uuid,
	provider: Provider,
	pool_type: PoolType,
	install_script_hash: String,
}

#[activity(SetDestroyed)]
async fn set_destroyed(ctx: &ActivityCtx, input: &SetDestroyedInput) -> GlobalResult<()> {
	sql_execute!(
		[ctx]
		"
		UPDATE db_cluster.server_images2
		SET destroy_ts = $5
		WHERE
			provider = $1 AND
			install_hash = $2 AND
			datacenter_id = $3 AND
			pool_type = $4
		",
		serde_json::to_string(&input.provider)?,
		&input.install_script_hash,
		input.datacenter_id,
		serde_json::to_string(&input.pool_type)?,
		util::timestamp::now(),
	)
	.await?;

	Ok(())
}
