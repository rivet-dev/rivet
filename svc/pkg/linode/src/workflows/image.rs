use chirp_workflow::prelude::*;
use serde_json::json;

use crate::util::{api, client};

#[derive(Debug, Serialize, Deserialize)]
pub struct Input {
	pub prebake_server_id: Uuid,
	pub api_token: Option<String>,
	pub linode_id: u64,
	pub boot_disk_id: u64,
}

#[workflow]
pub async fn linode_image(ctx: &mut WorkflowCtx, input: &Input) -> GlobalResult<()> {
	// Shut down server before creating custom image
	ctx.activity(ShutdownInput {
		linode_id: input.linode_id,
		api_token: input.api_token.clone(),
	})
	.await?;

	// NOTE: Linode imposes a restriction of 50 characters on custom image labels, so unfortunately we cannot
	// use the image variant as the name. All we need from the label is for it to be unique. Keep in mind that
	// the UUID and hyphen take 37 characters, leaving us with 13 for the namespace name
	let name = format!("{}-{}", util::env::namespace(), input.prebake_server_id);

	let image_id = ctx
		.activity(CreateCustomImageInput {
			linode_id: input.linode_id,
			api_token: input.api_token.clone(),
			boot_disk_id: input.boot_disk_id,
			name,
		})
		.await?;

	// Wait for image to complete creation
	let sig = ctx.listen::<CreateComplete>().await?;

	// Piggy back signal up to the cluster prebake workflow
	ctx.tagged_signal(
		&json!({
			"server_id": input.prebake_server_id,
		}),
		sig,
	)
	.await?;

	ctx.listen::<Destroy>().await?;

	ctx.activity(DestroyInput {
		api_token: input.api_token.clone(),
		image_id,
	})
	.await?;

	Ok(())
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct ShutdownInput {
	linode_id: u64,
	api_token: Option<String>,
}

#[activity(Shutdown)]
async fn shutdown(ctx: &ActivityCtx, input: &ShutdownInput) -> GlobalResult<()> {
	// Build HTTP client
	let client = client::Client::new(input.api_token.clone()).await?;

	api::shut_down(&client, input.linode_id).await?;

	Ok(())
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct CreateCustomImageInput {
	linode_id: u64,
	api_token: Option<String>,
	boot_disk_id: u64,
	name: String,
}

#[activity(CreateCustomImage)]
async fn create_custom_image(
	ctx: &ActivityCtx,
	input: &CreateCustomImageInput,
) -> GlobalResult<String> {
	// Build HTTP client
	let client = client::Client::new(input.api_token.clone()).await?;

	let create_image_res =
		api::create_custom_image(&client, &input.name, input.boot_disk_id).await?;

	sql_execute!(
		[ctx]
		"
		INSERT INTO db_linode.server_images (
			image_id
		)
		VALUES ($1)
		",
		&create_image_res.id,
	)
	.await?;

	// Add image id to workflow tags
	ctx.update_workflow_tags(&json!({
		"linode_id": input.linode_id,
		"image_id": create_image_res.id,
	}))
	.await?;

	Ok(create_image_res.id)
}

#[signal("linode-image-create-complete")]
pub struct CreateComplete {
	pub image_id: String,
}

#[signal("linode-image-destroy")]
pub struct Destroy {}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct DestroyInput {
	api_token: Option<String>,
	image_id: String,
}

#[activity(DestroyActivity)]
async fn destroy(ctx: &ActivityCtx, input: &DestroyInput) -> GlobalResult<()> {
	// Build HTTP client
	let client = client::Client::new(input.api_token.clone()).await?;

	api::delete_custom_image(&client, &input.image_id).await?;

	Ok(())
}
