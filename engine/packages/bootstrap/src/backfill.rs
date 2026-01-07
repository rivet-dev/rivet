use gas::prelude::*;
use universaldb::utils::IsolationLevel::*;

pub async fn run(ctx: &StandaloneCtx) -> Result<()> {
	// Actor runner name selector backfill
	if !is_complete(
		ctx,
		pegboard::workflows::actor_runner_name_selector_backfill::BACKFILL_NAME,
	)
	.await?
	{
		ctx.workflow(pegboard::workflows::actor_runner_name_selector_backfill::Input {})
			.unique()
			.dispatch()
			.await?;
	}

	// Serverless backfill
	if !is_complete(
		ctx,
		pegboard::workflows::serverless::backfill::BACKFILL_NAME,
	)
	.await?
	{
		ctx.workflow(pegboard::workflows::serverless::backfill::Input {})
			.unique()
			.dispatch()
			.await?;
	}

	Ok(())
}

async fn is_complete(ctx: &StandaloneCtx, name: &str) -> Result<bool> {
	let complete = ctx
		.udb()?
		.run(|tx| {
			let name = name.to_string();
			async move {
				let tx = tx.with_subspace(rivet_types::keys::backfill::subspace());
				tx.exists(&pegboard::keys::backfill::CompleteKey::new(&name), Snapshot)
					.await
			}
		})
		.custom_instrument(tracing::info_span!("check_backfill_complete_tx"))
		.await?;

	if complete {
		tracing::debug!(%name, "backfill already complete, skipping");
	}

	Ok(complete)
}
