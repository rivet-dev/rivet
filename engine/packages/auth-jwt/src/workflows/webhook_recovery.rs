//! Redispatch provisioned idle rings on the current authority, independent of the former DC.
use futures_util::FutureExt;
use gas::prelude::*;
#[derive(Debug, Serialize, Deserialize)]
pub struct Input;
#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
struct Page {
	after: Option<Vec<u8>>,
}
#[workflow]
pub async fn auth_webhook_recovery(ctx: &mut WorkflowCtx, _input: &Input) -> Result<()> {
	ctx.loope(None::<Vec<u8>>, |ctx, after| {
		async move {
			let next = ctx
				.activity(Page {
					after: after.clone(),
				})
				.await?;
			if next.is_none() {
				ctx.sleep(std::time::Duration::from_secs(60)).await?;
			}
			*after = next;
			Ok(Loop::<()>::Continue)
		}
		.boxed()
	})
	.await?;
	Ok(())
}
#[activity(WebhookRecoveryPage)]
#[max_retries = usize::MAX]
async fn page(ctx: &ActivityCtx, input: &Page) -> Result<Option<Vec<u8>>> {
	if !ctx.config().is_leader() {
		return Ok(None);
	}
	let page = ctx
		.op(namespace::ops::signing_recovery_page::Input {
			after: input.after.clone(),
		})
		.await?;
	Ok(recover_page(
		page,
		|namespace_id| async move {
			Ok(ctx
				.op(crate::ops::webhook::get::Input { namespace_id })
				.await?
				.is_some())
		},
		|namespace_id| async move {
			ctx.op(crate::ops::webhook::ensure_rotation::Input { namespace_id })
				.await
		},
	)
	.await)
}

/// A broken ring must not prevent unrelated namespaces from recovering. Advance past failures
/// and retry them on the next complete sweep; never replace or provision their key material.
async fn recover_page<Read, ReadFuture, Dispatch, DispatchFuture>(
	page: namespace::ops::signing_recovery_page::Output,
	mut provisioned: Read,
	mut dispatch: Dispatch,
) -> Option<Vec<u8>>
where
	Read: FnMut(Id) -> ReadFuture,
	ReadFuture: std::future::Future<Output = Result<bool>>,
	Dispatch: FnMut(Id) -> DispatchFuture,
	DispatchFuture: std::future::Future<Output = Result<()>>,
{
	for namespace_id in page.namespace_ids {
		match provisioned(namespace_id).await {
			Ok(false) => {}
			Ok(true) => {
				if dispatch(namespace_id).await.is_err() {
					tracing::warn!(namespace_id = %namespace_id, phase = "dispatch", "webhook recovery will retry namespace on the next sweep");
				}
			}
			Err(_) => {
				// Decode/storage errors may contain private persisted data: never log them.
				tracing::warn!(namespace_id = %namespace_id, phase = "read", "webhook recovery will retry namespace on the next sweep");
			}
		}
	}
	page.next
}

#[cfg(test)]
#[path = "../../tests/unit/webhook_recovery.rs"]
mod tests;
