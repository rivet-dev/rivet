use chirp_workflow::prelude::*;
use fdb_util::{end_of_key_range, FormalKey, SERIALIZABLE};
use foundationdb::{self as fdb, options::ConflictRangeType};

use crate::{keys, protocol};

#[derive(Debug)]
pub enum Action {
	ClearIdx,
	AddIdx,
	UpdatePing,
	ReleaseResources {
		/// MiB.
		memory: u64,
		/// Millicores.
		cpu: u64,
	},
}

#[derive(Debug)]
pub struct Input {
	pub client_id: Uuid,
	pub client_workflow_id: Uuid,
	pub flavor: protocol::ClientFlavor,
	pub action: Action,
}

#[operation]
pub async fn pegboard_client_update_allocation_idx(
	ctx: &OperationCtx,
	input: &Input,
) -> GlobalResult<()> {
	ctx.fdb()
		.await?
		.run(|tx, _mc| async move {
			let remaining_mem_key = keys::client::RemainingMemoryKey::new(input.client_id);
			let remaining_mem_key_buf = keys::subspace().pack(&remaining_mem_key);
			let remaining_cpu_key = keys::client::RemainingCpuKey::new(input.client_id);
			let remaining_cpu_key_buf = keys::subspace().pack(&remaining_cpu_key);
			let last_ping_ts_key = keys::client::LastPingTsKey::new(input.client_id);
			let last_ping_ts_key_buf = keys::subspace().pack(&last_ping_ts_key);

			// Read current allocated memory and last ping
			let (remaining_mem_entry, remaining_cpu_entry, last_ping_ts_entry) = tokio::try_join!(
				tx.get(&remaining_mem_key_buf, SERIALIZABLE),
				tx.get(&remaining_cpu_key_buf, SERIALIZABLE),
				tx.get(&last_ping_ts_key_buf, SERIALIZABLE),
			)?;
			let remaining_mem = remaining_mem_key
				.deserialize(
					&remaining_mem_entry.ok_or(fdb::FdbBindingError::CustomError(
						format!("key should exist: {remaining_mem_key:?}").into(),
					))?,
				)
				.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?;
			let remaining_cpu = remaining_cpu_key
				.deserialize(
					&remaining_cpu_entry.ok_or(fdb::FdbBindingError::CustomError(
						format!("key should exist: {remaining_cpu_key:?}").into(),
					))?,
				)
				.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?;
			let last_ping_ts = last_ping_ts_key
				.deserialize(&last_ping_ts_entry.ok_or(fdb::FdbBindingError::CustomError(
					format!("key should exist: {last_ping_ts_key:?}").into(),
				))?)
				.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?;

			let old_allocation_key = keys::datacenter::ClientsByRemainingMemKey::new(
				input.flavor,
				remaining_mem,
				last_ping_ts,
				input.client_id,
			);
			let old_allocation_key_buf = keys::subspace().pack(&old_allocation_key);

			// Add read conflict
			tx.add_conflict_range(
				&old_allocation_key_buf,
				&end_of_key_range(&old_allocation_key_buf),
				ConflictRangeType::Read,
			)?;

			match input.action {
				Action::ClearIdx => {
					tx.clear(&old_allocation_key_buf);
				}
				Action::AddIdx => {
					tx.set(
						&old_allocation_key_buf,
						&old_allocation_key
							.serialize(input.client_workflow_id)
							.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?,
					);
				}
				// TODO: Could be improved somehow to not require another `.get`
				Action::UpdatePing => {
					let last_ping_ts = util::timestamp::now();

					// Write new ping
					tx.set(
						&last_ping_ts_key_buf,
						&last_ping_ts_key
							.serialize(last_ping_ts)
							.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?,
					);

					// Only update allocation idx if it existed before
					if tx
						.get(&old_allocation_key_buf, SERIALIZABLE)
						.await?
						.is_some()
					{
						// Clear old key
						tx.clear(&old_allocation_key_buf);

						let new_allocation_key = keys::datacenter::ClientsByRemainingMemKey::new(
							input.flavor,
							remaining_mem,
							last_ping_ts,
							input.client_id,
						);
						let new_allocation_key_buf = keys::subspace().pack(&new_allocation_key);

						tx.set(
							&new_allocation_key_buf,
							&new_allocation_key
								.serialize(input.client_workflow_id)
								.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?,
						);
					}
				}
				Action::ReleaseResources { memory, cpu } => {
					let new_memory = remaining_mem + memory;
					let new_cpu = remaining_cpu + cpu;

					// Write new memory
					tx.set(
						&remaining_mem_key_buf,
						&remaining_mem_key
							.serialize(new_memory)
							.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?,
					);
					// Write new cpu
					tx.set(
						&remaining_cpu_key_buf,
						&remaining_cpu_key
							.serialize(new_cpu)
							.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?,
					);

					// Only update allocation idx if it existed before
					if tx
						.get(&old_allocation_key_buf, SERIALIZABLE)
						.await?
						.is_some()
					{
						// Clear old key
						tx.clear(&old_allocation_key_buf);

						let new_allocation_key = keys::datacenter::ClientsByRemainingMemKey::new(
							input.flavor,
							new_memory,
							last_ping_ts,
							input.client_id,
						);
						let new_allocation_key_buf = keys::subspace().pack(&new_allocation_key);

						tx.set(
							&new_allocation_key_buf,
							&new_allocation_key
								.serialize(input.client_workflow_id)
								.map_err(|x| fdb::FdbBindingError::CustomError(x.into()))?,
						);
					}
				}
			}

			Ok(())
		})
		.await
		.map_err(Into::into)
}
