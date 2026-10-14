use gas::prelude::*;
use universaldb::utils::IsolationLevel::Serializable;

#[derive(Debug)]
pub struct Input {
	pub namespace_id: Id,
}

/// Bypasses metadata caches for signing admission. Only the namespace authority reads it.
#[operation]
pub async fn namespace_exists_authoritative(ctx: &OperationCtx, input: &Input) -> Result<bool> {
	anyhow::ensure!(
		ctx.config().is_leader(),
		"namespace existence reads are leader-only"
	);
	let key = crate::keys::NameKey::new(input.namespace_id);
	ctx.udb()?
		.txn("namespace_exists_authoritative", |tx| {
			let key = &key;
			async move {
				Ok(tx
					.with_subspace(crate::keys::subspace())
					.read_opt(key, Serializable)
					.await?
					.is_some())
			}
		})
		.await
}
