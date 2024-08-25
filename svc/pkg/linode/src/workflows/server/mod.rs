use std::net::Ipv4Addr;

use chirp_workflow::prelude::*;
use serde_json::json;

pub mod cleanup;

use crate::{
	types::FirewallPreset,
	util::{api, client},
};

const DEFAULT_IMAGE: &str = "linode/debian11";

#[derive(Debug, Serialize, Deserialize)]
pub struct Input {
	pub server_id: Uuid,
	pub provider_datacenter_id: String,
	pub custom_image: Option<String>,
	pub hardware: String,
	pub api_token: Option<String>,
	pub firewall_preset: FirewallPreset,
	pub vlan_ip: Option<Ipv4Addr>,
	pub tags: Vec<String>,
}

#[workflow]
pub async fn linode_server(ctx: &mut WorkflowCtx, input: &Input) -> GlobalResult<()> {
	let mut cleanup_ctx = CleanupCtx::default();

	let res = provision(ctx, input, &mut cleanup_ctx).await;
	let provision_res = match ctx.catch_unrecoverable(res)? {
		Ok(x) => x,
		// If we cannot recover a provisioning error, send a failed signal and clean up resources
		Err(err) => {
			tracing::warn!(?err, "unrecoverable provision, cleaning up");

			ctx.dispatch_workflow(cleanup::Input {
				api_token: input.api_token.clone(),
				ssh_key_id: cleanup_ctx.ssh_key_id,
				linode_id: cleanup_ctx.linode_id,
				firewall_id: cleanup_ctx.firewall_id,
			})
			.await?;

			ctx.tagged_signal(
				&json!({
					"server_id": input.server_id,
				}),
				ProvisionFailed {},
			)
			.await?;

			// Throw the original error from the provisioning activities
			return Err(err);
		}
	};

	ctx.tagged_signal(
		&json!({
			"server_id": input.server_id,
		}),
		ProvisionComplete {
			linode_id: provision_res.linode_id,
			public_ip: provision_res.public_ip,
			boot_disk_id: provision_res.boot_disk_id,
		},
	)
	.await?;

	// Wait for destroy signal
	ctx.listen::<Destroy>().await?;

	ctx.workflow(cleanup::Input {
		api_token: input.api_token.clone(),
		ssh_key_id: cleanup_ctx.ssh_key_id,
		linode_id: cleanup_ctx.linode_id,
		firewall_id: cleanup_ctx.firewall_id,
	})
	.await?;

	Ok(())
}

#[derive(Default)]
struct CleanupCtx {
	ssh_key_id: Option<u64>,
	linode_id: Option<u64>,
	firewall_id: Option<u64>,
}

struct ProvisionOutput {
	linode_id: u64,
	boot_disk_id: u64,
	public_ip: Ipv4Addr,
}

/// Group of activities for provisioning. Used to handle cleanups in the event of a retry failure.
/// The reason this is not a workflow is because we need to manually handle when an activity is no longer
/// retryable and clean it up afterwards.
async fn provision(
	ctx: &mut WorkflowCtx,
	input: &Input,
	cleanup: &mut CleanupCtx,
) -> GlobalResult<ProvisionOutput> {
	let is_test = input.tags.iter().any(|tag| tag == "test");
	let ns = util::env::namespace();
	// Linode label must be 3-64 characters, UUID's are 36
	let name = format!("{ns}-{}", input.server_id);

	let tags = input
		.tags
		.iter()
		.cloned()
		.chain([
			// HACK: Linode requires tags to be > 3 characters. We extend the namespace to make sure it
			// meets the minimum length requirement.
			format!("rivet-{ns}"),
			format!("{ns}-{}", input.provider_datacenter_id),
			format!("{ns}-{}", input.firewall_preset),
			format!(
				"{ns}-{}-{}",
				input.provider_datacenter_id, input.firewall_preset
			),
		])
		.collect::<Vec<_>>();

	let ssh_key_res = ctx
		.activity(CreateSshKeyInput {
			server_id: input.server_id,
			api_token: input.api_token.clone(),
			is_test,
		})
		.await?;

	cleanup.ssh_key_id = Some(ssh_key_res.ssh_key_id);

	let create_instance_res = ctx
		.activity(CreateInstanceInput {
			api_token: input.api_token.clone(),
			ssh_public_key: ssh_key_res.public_key.clone(),
			name,
			datacenter: input.provider_datacenter_id.clone(),
			hardware: input.hardware.clone(),
			tags: tags.clone(),
		})
		.await?;

	cleanup.linode_id = Some(create_instance_res.linode_id);

	ctx.activity(WaitInstanceReadyInput {
		api_token: input.api_token.clone(),
		linode_id: create_instance_res.linode_id,
	})
	.await?;

	let boot_disk_id = ctx
		.activity(CreateBootDiskInput {
			api_token: input.api_token.clone(),
			image: input
				.custom_image
				.clone()
				.unwrap_or_else(|| DEFAULT_IMAGE.to_string()),
			ssh_public_key: ssh_key_res.public_key.clone(),
			linode_id: create_instance_res.linode_id,
			disk_size: create_instance_res.server_disk_size,
		})
		.await?;

	ctx.activity(WaitDiskReadyInput {
		api_token: input.api_token.clone(),
		linode_id: create_instance_res.linode_id,
		disk_id: boot_disk_id,
	})
	.await?;

	let swap_disk_id = ctx
		.activity(CreateSwapDiskInput {
			api_token: input.api_token.clone(),
			linode_id: create_instance_res.linode_id,
		})
		.await?;

	ctx.activity(CreateInstanceConfigInput {
		api_token: input.api_token.clone(),
		vlan_ip: input.vlan_ip,
		linode_id: create_instance_res.linode_id,
		boot_disk_id,
		swap_disk_id,
	})
	.await?;

	let firewall_id = ctx
		.activity(CreateFirewallInput {
			server_id: input.server_id,
			api_token: input.api_token.clone(),
			firewall_preset: input.firewall_preset.clone(),
			tags,
			linode_id: create_instance_res.linode_id,
		})
		.await?;

	cleanup.firewall_id = Some(firewall_id);

	ctx.activity(BootInstanceInput {
		api_token: input.api_token.clone(),
		linode_id: create_instance_res.linode_id,
	})
	.await?;

	let public_ip = ctx
		.activity(GetPublicIpInput {
			api_token: input.api_token.clone(),
			linode_id: create_instance_res.linode_id,
		})
		.await?;

	Ok(ProvisionOutput {
		linode_id: create_instance_res.linode_id,
		boot_disk_id,
		public_ip,
	})
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct CreateSshKeyInput {
	server_id: Uuid,
	api_token: Option<String>,
	is_test: bool,
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct CreateSshKeyOutput {
	ssh_key_id: u64,
	public_key: String,
}

#[activity(CreateSshKey)]
async fn create_ssh_key(
	ctx: &ActivityCtx,
	input: &CreateSshKeyInput,
) -> GlobalResult<CreateSshKeyOutput> {
	// Build HTTP client
	let client = client::Client::new(input.api_token.clone()).await?;

	let ns = util::env::namespace();

	let ssh_key_label = format!("{ns}-{}", input.server_id);
	let ssh_key_res = api::create_ssh_key(&client, &ssh_key_label, input.is_test).await?;

	Ok(CreateSshKeyOutput {
		ssh_key_id: ssh_key_res.id,
		public_key: ssh_key_res.public_key,
	})
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct CreateInstanceInput {
	api_token: Option<String>,
	ssh_public_key: String,
	name: String,
	datacenter: String,
	hardware: String,
	tags: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct CreateInstanceOutput {
	linode_id: u64,
	server_disk_size: u64,
}

#[activity(CreateInstance)]
async fn create_instance(
	ctx: &ActivityCtx,
	input: &CreateInstanceInput,
) -> GlobalResult<CreateInstanceOutput> {
	// Build HTTP client
	let client = client::Client::new(input.api_token.clone()).await?;

	let create_instance_res = api::create_instance(
		&client,
		&input.name,
		&input.datacenter,
		&input.hardware,
		&input.tags,
		&input.ssh_public_key,
	)
	.await?;
	let linode_id = create_instance_res.id;

	Ok(CreateInstanceOutput {
		linode_id,
		server_disk_size: create_instance_res.specs.disk,
	})
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct WaitInstanceReadyInput {
	api_token: Option<String>,
	linode_id: u64,
}

#[activity(WaitInstanceReady)]
async fn wait_instance_ready(
	ctx: &ActivityCtx,
	input: &WaitInstanceReadyInput,
) -> GlobalResult<()> {
	// Build HTTP client
	let client = client::Client::new(input.api_token.clone()).await?;

	api::wait_instance_ready(&client, input.linode_id).await
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct CreateBootDiskInput {
	api_token: Option<String>,
	image: String,
	ssh_public_key: String,
	linode_id: u64,
	disk_size: u64,
}

#[activity(CreateBootDisk)]
async fn create_boot_disk(ctx: &ActivityCtx, input: &CreateBootDiskInput) -> GlobalResult<u64> {
	// Build HTTP client
	let client = client::Client::new(input.api_token.clone()).await?;

	api::create_boot_disk(
		&client,
		&input.ssh_public_key,
		input.linode_id,
		&input.image,
		input.disk_size,
	)
	.await
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct WaitDiskReadyInput {
	api_token: Option<String>,
	linode_id: u64,
	disk_id: u64,
}

#[activity(WaitDiskReady)]
async fn wait_disk_ready(ctx: &ActivityCtx, input: &WaitDiskReadyInput) -> GlobalResult<()> {
	// Build HTTP client
	let client = client::Client::new(input.api_token.clone()).await?;

	api::wait_disk_ready(&client, input.linode_id, input.disk_id).await
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct CreateSwapDiskInput {
	api_token: Option<String>,
	linode_id: u64,
}

#[activity(CreateSwapDisk)]
async fn create_swap_disk(ctx: &ActivityCtx, input: &CreateSwapDiskInput) -> GlobalResult<u64> {
	// Build HTTP client
	let client = client::Client::new(input.api_token.clone()).await?;

	api::create_swap_disk(&client, input.linode_id).await
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct CreateInstanceConfigInput {
	api_token: Option<String>,
	vlan_ip: Option<Ipv4Addr>,
	linode_id: u64,
	boot_disk_id: u64,
	swap_disk_id: u64,
}

#[activity(CreateInstanceConfig)]
async fn create_instance_config(
	ctx: &ActivityCtx,
	input: &CreateInstanceConfigInput,
) -> GlobalResult<()> {
	// Build HTTP client
	let client = client::Client::new(input.api_token.clone()).await?;

	api::create_instance_config(
		&client,
		input.vlan_ip.as_ref(),
		input.linode_id,
		input.boot_disk_id,
		input.swap_disk_id,
	)
	.await
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct CreateFirewallInput {
	server_id: Uuid,
	api_token: Option<String>,
	firewall_preset: FirewallPreset,
	tags: Vec<String>,
	linode_id: u64,
}

#[activity(CreateFirewall)]
async fn create_firewall(ctx: &ActivityCtx, input: &CreateFirewallInput) -> GlobalResult<u64> {
	// Build HTTP client
	let client = client::Client::new(input.api_token.clone()).await?;

	let firewall_res = api::create_firewall(
		&client,
		&input.firewall_preset,
		&input.tags,
		input.linode_id,
	)
	.await?;

	Ok(firewall_res.id)
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct BootInstanceInput {
	api_token: Option<String>,
	linode_id: u64,
}

#[activity(BootInstance)]
async fn boot_instance(ctx: &ActivityCtx, input: &BootInstanceInput) -> GlobalResult<()> {
	// Build HTTP client
	let client = client::Client::new(input.api_token.clone()).await?;

	api::boot_instance(&client, input.linode_id).await?;

	Ok(())
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct GetPublicIpInput {
	api_token: Option<String>,
	linode_id: u64,
}

#[activity(GetPublicIp)]
async fn get_public_ip(ctx: &ActivityCtx, input: &GetPublicIpInput) -> GlobalResult<Ipv4Addr> {
	// Build HTTP client
	let client = client::Client::new(input.api_token.clone()).await?;

	api::get_public_ip(&client, input.linode_id).await
}

#[signal("linode_server_provision_complete")]
pub struct ProvisionComplete {
	pub linode_id: u64,
	pub public_ip: Ipv4Addr,
	pub boot_disk_id: u64,
}

#[signal("linode_server_provision_failed")]
pub struct ProvisionFailed {}

#[signal("linode_server_destroy")]
pub struct Destroy {}
