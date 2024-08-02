use chirp_workflow::prelude::*;

use crate::util::{api, client};

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Input {
	pub api_token: Option<String>,
	pub ssh_key_id: Option<u64>,
	pub linode_id: Option<u64>,
	pub firewall_id: Option<u64>,
}

#[workflow]
pub(crate) async fn linode_server_cleanup(
	ctx: &mut WorkflowCtx,
	input: &Input,
) -> GlobalResult<()> {
	if let Some(linode_id) = input.linode_id {
		ctx.activity(DeleteInstanceInput {
			api_token: input.api_token.clone(),
			linode_id,
		})
		.await?;
	}

	if let Some(firewall_id) = input.firewall_id {
		ctx.activity(DeleteFirewallInput {
			api_token: input.api_token.clone(),
			firewall_id,
		})
		.await?;
	}

	if let Some(ssh_key_id) = input.ssh_key_id {
		ctx.activity(DeleteSshKeyInput {
			api_token: input.api_token.clone(),
			ssh_key_id,
		})
		.await?;
	}

	Ok(())
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct DeleteInstanceInput {
	api_token: Option<String>,
	linode_id: u64,
}

#[activity(DeleteInstance)]
async fn delete_instance(ctx: &ActivityCtx, input: &DeleteInstanceInput) -> GlobalResult<()> {
	// Build HTTP client
	let client = client::Client::new(input.api_token.clone()).await?;

	api::delete_instance(&client, input.linode_id).await?;

	Ok(())
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct DeleteFirewallInput {
	api_token: Option<String>,
	firewall_id: u64,
}

#[activity(DeleteFirewall)]
async fn delete_firewall(ctx: &ActivityCtx, input: &DeleteFirewallInput) -> GlobalResult<()> {
	// Build HTTP client
	let client = client::Client::new(input.api_token.clone()).await?;

	api::delete_firewall(&client, input.firewall_id).await?;

	Ok(())
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct DeleteSshKeyInput {
	api_token: Option<String>,
	ssh_key_id: u64,
}

#[activity(DeleteSshKey)]
async fn delete_ssh_key(ctx: &ActivityCtx, input: &DeleteSshKeyInput) -> GlobalResult<()> {
	// Build HTTP client
	let client = client::Client::new(input.api_token.clone()).await?;

	api::delete_ssh_key(&client, input.ssh_key_id).await?;
	Ok(())
}
