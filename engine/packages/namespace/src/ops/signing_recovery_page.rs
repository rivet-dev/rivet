//! Bounded namespace enumeration for recovering idle signing workflows after leader changes.
use futures_util::TryStreamExt;
use gas::prelude::*;
use universaldb::{KeySelector, utils::IsolationLevel::Snapshot};
#[derive(Debug)]
pub struct Input {
	pub after: Option<Vec<u8>>,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct Output {
	pub namespace_ids: Vec<Id>,
	pub next: Option<Vec<u8>>,
}
#[operation]
pub async fn namespace_signing_recovery_page(ctx: &OperationCtx, input: &Input) -> Result<Output> {
	anyhow::ensure!(
		ctx.config().is_leader(),
		"namespace enumeration is leader-only"
	);
	ctx.udb()?
		.txn("namespace_signing_recovery_page", |tx| async move {
			let subspace = crate::keys::subspace();
			let mut range: universaldb::RangeOption = (&subspace).into();
			if let Some(after) = &input.after {
				range.begin = KeySelector::first_greater_than(after.clone());
			}
			range.limit = Some(512);
			let mut stream = tx.get_ranges_keyvalues(range, Snapshot);
			let mut namespace_ids = Vec::new();
			let mut last = None;
			let mut count = 0;
			while let Some(entry) = stream.try_next().await? {
				count += 1;
				last = Some(entry.key().to_vec());
				if let Ok(key) = subspace.unpack::<crate::keys::NameKey>(entry.key()) {
					// TupleUnpack alone accepts other three-field namespace keys.
					if subspace.pack(&key) == entry.key() {
						namespace_ids.push(key.namespace_id());
					}
				}
			}
			Ok(Output {
				namespace_ids,
				next: if count == 512 { last } else { None },
			})
		})
		.await
}
