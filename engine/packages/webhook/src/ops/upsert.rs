use gas::prelude::*;

use crate::{types::WebhookConfig, workflows};

#[derive(Debug)]
pub struct Input {
	pub namespace_id: Id,
	pub name: String,
	pub config: WebhookConfig,
}

#[operation]
pub async fn webhook_config_upsert(ctx: &OperationCtx, input: &Input) -> Result<()> {
	let topic = workflows::webhook::topic(input.namespace_id, &input.name);

	let mut complete_sub = ctx
		.subscribe::<workflows::webhook::UpsertComplete>(topic.clone())
		.await?;
	let mut failed_sub = ctx
		.subscribe::<workflows::webhook::Failed>(topic.clone())
		.await?;

	let signal_res = ctx
		.signal(workflows::webhook::Update {
			config: input.config.clone(),
		})
		.to_workflow::<workflows::webhook::Workflow>()
		.tag("namespace_id", input.namespace_id)
		.tag("name", input.name.clone())
		.graceful_not_found()
		.send()
		.await?;

	if signal_res.is_none() {
		ctx.workflow(workflows::webhook::Input {
			namespace_id: input.namespace_id,
			name: input.name.clone(),
			config: input.config.clone(),
		})
		.tag("namespace_id", input.namespace_id)
		.tag("name", input.name.clone())
		.unique()
		.dispatch()
		.await?;
	}

	tokio::select! {
		res = complete_sub.next() => { res?; }
		res = failed_sub.next() => {
			let msg = res?;
			return Err(msg.into_body().error.build());
		}
	}

	Ok(())
}
