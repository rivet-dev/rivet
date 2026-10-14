use futures_util::FutureExt;
use gas::prelude::*;

#[derive(Debug, Serialize, Deserialize)]
pub struct Input {
	pub namespace_id: Id,
}

#[workflow]
pub async fn auth_webhook_rotation(ctx: &mut WorkflowCtx, input: &Input) -> Result<()> {
	let namespace_id = input.namespace_id;
	ctx.loope((), |ctx, _| {
		async move {
			let next = ctx.activity(ReconcileInput { namespace_id }).await?;
			match next {
				Some(next) => {
					ctx.sleep_until(next).await?;
					Ok(Loop::<()>::Continue)
				}
				None => Ok(Loop::Break(())),
			}
		}
		.boxed()
	})
	.await?;
	Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
struct ReconcileInput {
	namespace_id: Id,
}

#[activity(WebhookReconcile)]
#[max_retries = usize::MAX]
async fn reconcile(ctx: &ActivityCtx, input: &ReconcileInput) -> Result<Option<i64>> {
	if !ctx.config().is_leader() {
		return Ok(Some(rivet_util::timestamp::now() + 60_000));
	}
	if !ctx
		.op(namespace::ops::exists_authoritative::Input {
			namespace_id: input.namespace_id,
		})
		.await?
	{
		return Ok(None);
	}
	match ctx
		.op(crate::ops::webhook::mutate::Input {
			namespace_id: input.namespace_id,
			change: crate::ops::webhook::Change::Reconcile,
		})
		.await
	{
		Ok(ring) => Ok(Some(
			crate::next_wake_ts(&ring.keys, crate::webhook::policy())?
				.max(rivet_util::timestamp::now() + 1_000)
				.min(rivet_util::timestamp::now() + 60_000),
		)),
		Err(_) => {
			tracing::warn!(namespace_id = %input.namespace_id, "webhook key lifecycle reconciliation will retry");
			Ok(Some(rivet_util::timestamp::now() + 5_000))
		}
	}
}
