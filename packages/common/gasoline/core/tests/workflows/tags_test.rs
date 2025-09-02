use gas::prelude::*;

#[derive(Debug, Serialize, Deserialize)]
pub struct TagsTestInput {
	pub tag_key: String,
	pub tag_value: String,
}

#[workflow(TagsTestWorkflow)]
pub async fn tags_test_workflow(ctx: &mut WorkflowCtx, input: &TagsTestInput) -> Result<()> {
	ctx.activity(UpdateTagsActivityInput {
		key: input.tag_key.clone(),
		value: input.tag_value.clone(),
	})
	.await?;

	Ok(())
}

#[derive(Debug, Serialize, Deserialize, Hash)]
pub struct UpdateTagsActivityInput {
	pub key: String,
	pub value: String,
}

#[activity(UpdateTagsActivity)]
pub async fn update_tags_activity(
	ctx: &ActivityCtx,
	input: &UpdateTagsActivityInput,
) -> Result<()> {
	let tags = serde_json::json!({
		&input.key: &input.value
	});

	ctx.update_workflow_tags(&tags).await?;
	Ok(())
}
