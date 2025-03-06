use chirp_workflow::prelude::*;
use fdb_util::{FormalKey, SNAPSHOT};
use foundationdb::{
	self as fdb,
	options::{ConflictRangeType, StreamingMode},
};
use futures_util::TryStreamExt;

use crate::protocol;

use crate::{keys, workflows::client::CLIENT_ELIGIBLE_THRESHOLD_MS};

// TODO: Only uses memory for allocation atm, not cpu as well
/// Chooses a client that can allocate the given resources and reserves those resources. For container actors
/// this fills up clients until full (bin packing) and for isolate actors this distributes load evenly across
/// clients.
#[derive(Debug)]
pub struct Input {
	pub flavor: protocol::ClientFlavor,
	/// MiB.
	pub memory: u64,
}

#[derive(Debug)]
pub struct Output {
	pub client_id: Uuid,
	pub client_workflow_id: Uuid,
}

#[operation]
pub async fn pegboard_client_reserve(
	ctx: &OperationCtx,
	input: &Input,
) -> GlobalResult<Option<Output>> {
	ctx.fdb()
		.await?
		.run(|tx, _mc| async move {
			let ping_threshold_ts = util::timestamp::now() - CLIENT_ELIGIBLE_THRESHOLD_MS;

			// Select a range that only includes clients that have enough remaining mem to allocate this actor
			let start = keys::subspace().pack(
				&keys::datacenter::ClientsByRemainingMemKey::subspace_with_mem(
					input.flavor,
					input.memory,
				),
			);
			let client_allocation_subspace =
				keys::datacenter::ClientsByRemainingMemKey::subspace(input.flavor);
			let end = keys::subspace()
				.subspace(&client_allocation_subspace)
				.range()
				.1;

			let mut stream = tx.get_ranges_keyvalues(
				fdb::RangeOption {
					mode: StreamingMode::Iterator,
					// Containers bin pack so we reverse the order
					reverse: matches!(input.flavor, protocol::ClientFlavor::Container),
					..(start, end).into()
				},
				// NOTE: This is not SERIALIZABLE because we don't want to conflict with all of the keys, just
				// the one we choose
				SNAPSHOT,
			);

			loop {
				let Some(entry) = stream.try_next().await? else {
					return Ok(None);
				};

				let old_allocation_key = keys::subspace()
					.unpack::<keys::datacenter::ClientsByRemainingMemKey>(entry.key())
					.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?;

				// Scan by last ping
				if old_allocation_key.last_ping_ts < ping_threshold_ts {
					continue;
				}

				let client_workflow_id = old_allocation_key
					.deserialize(entry.value())
					.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?;

				// Add read conflict only for this key
				tx.add_conflict_range(entry.key(), entry.key(), ConflictRangeType::Read)?;

				// Clear old entry
				tx.clear(entry.key());

				// Update allocated amount
				let remaining_mem = old_allocation_key.remaining_mem - input.memory;
				let new_allocation_key = keys::datacenter::ClientsByRemainingMemKey::new(
					input.flavor,
					remaining_mem,
					old_allocation_key.last_ping_ts,
					old_allocation_key.client_id,
				);
				tx.set(&keys::subspace().pack(&new_allocation_key), entry.value());

				// Update client record
				let client_allocation_key =
					keys::client::RemainingMemKey::new(old_allocation_key.client_id);
				tx.set(
					&keys::subspace().pack(&client_allocation_key),
					&client_allocation_key
						.serialize(remaining_mem)
						.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?,
				);

				return Ok(Some(Output {
					client_id: old_allocation_key.client_id,
					client_workflow_id,
				}));
			}
		})
		.await
		.map_err(Into::into)
}
