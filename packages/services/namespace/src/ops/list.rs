use anyhow::Result;
use futures_util::TryStreamExt;
use gas::prelude::*;
use udb_util::SNAPSHOT;
use universaldb::{self as udb, options::StreamingMode};

use crate::{errors, keys, types::Namespace};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Input {
	pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Output {
	pub namespaces: Vec<Namespace>,
}

#[operation]
pub async fn namespace_list(ctx: &OperationCtx, input: &Input) -> Result<Output> {
	if !ctx.config().is_leader() {
		return Err(errors::Namespace::NotLeader.build());
	}

	let namespaces = ctx
		.udb()?
		.run(|tx, _mc| async move {
			let mut namespaces = Vec::new();
			let limit = input.limit.unwrap_or(1000); // Default limit to 1000

			let mut stream = tx.get_ranges_keyvalues(
				udb::RangeOption {
					mode: StreamingMode::Iterator,
					..(&keys::subspace()).into()
				},
				SNAPSHOT,
			);

			let mut seen_namespaces = std::collections::HashSet::new();

			while let Some(entry) = stream.try_next().await? {
				// Try to unpack as a NameKey
				if let Ok(name_key) = keys::subspace().unpack::<keys::NameKey>(entry.key()) {
					let namespace_id = name_key.namespace_id();

					// Skip if we've already seen this namespace
					if !seen_namespaces.insert(namespace_id) {
						continue;
					}

					// Get the full namespace data
					if let Some(namespace) = super::get_local::get_inner(namespace_id, &tx).await? {
						namespaces.push(namespace);

						if namespaces.len() >= limit {
							break;
						}
					}
				}
			}

			Result::<_, udb::FdbBindingError>::Ok(namespaces)
		})
		.custom_instrument(tracing::info_span!("namespace_list_tx"))
		.await?;

	Ok(Output { namespaces })
}
