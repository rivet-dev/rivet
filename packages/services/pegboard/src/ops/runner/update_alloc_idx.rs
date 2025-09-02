use gas::prelude::*;
use udb_util::{SERIALIZABLE, TxnExt};
use universaldb::options::ConflictRangeType;

use crate::{keys, workflows::runner::RUNNER_ELIGIBLE_THRESHOLD_MS};

#[derive(Debug)]
pub struct Input {
	pub runners: Vec<Runner>,
}

#[derive(Debug, Clone)]
pub struct Runner {
	pub runner_id: Id,
	pub action: Action,
}

#[derive(Debug, Copy, Clone)]
pub enum Action {
	ClearIdx,
	AddIdx,
	UpdatePing { rtt: u32 },
}

#[derive(Debug)]
pub struct Output {
	// Inform the caller of certain runner eligibility changes they should know about.
	pub notifications: Vec<RunnerNotification>,
}

#[derive(Debug)]
pub struct RunnerNotification {
	pub runner_id: Id,
	pub workflow_id: Id,
	pub eligibility: RunnerEligibility,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunnerEligibility {
	// The runner that was just updated is now eligible again for allocation.
	ReEligible,
	// The runner that was just updated is expired.
	Expired,
}

#[operation]
pub async fn pegboard_runner_update_alloc_idx(ctx: &OperationCtx, input: &Input) -> Result<Output> {
	let notifications = ctx
		.udb()?
		.run(|tx, _mc| {
			let runners = input.runners.clone();

			async move {
				let txs = tx.subspace(keys::subspace());
				let mut notifications = Vec::new();

				// TODO: Parallelize
				for runner in &runners {
					let workflow_id_key = keys::runner::WorkflowIdKey::new(runner.runner_id);
					let namespace_id_key = keys::runner::NamespaceIdKey::new(runner.runner_id);
					let name_key = keys::runner::NameKey::new(runner.runner_id);
					let version_key = keys::runner::VersionKey::new(runner.runner_id);
					let remaining_slots_key =
						keys::runner::RemainingSlotsKey::new(runner.runner_id);
					let total_slots_key = keys::runner::TotalSlotsKey::new(runner.runner_id);
					let last_ping_ts_key = keys::runner::LastPingTsKey::new(runner.runner_id);
					let expired_ts_key = keys::runner::ExpiredTsKey::new(runner.runner_id);

					let (
						workflow_id_entry,
						namespace_id_entry,
						name_entry,
						version_entry,
						remaining_slots_entry,
						total_slots_entry,
						last_ping_ts_entry,
						expired_ts_entry,
					) = tokio::try_join!(
						txs.read_opt(&workflow_id_key, SERIALIZABLE),
						txs.read_opt(&namespace_id_key, SERIALIZABLE),
						txs.read_opt(&name_key, SERIALIZABLE),
						txs.read_opt(&version_key, SERIALIZABLE),
						txs.read_opt(&remaining_slots_key, SERIALIZABLE),
						txs.read_opt(&total_slots_key, SERIALIZABLE),
						txs.read_opt(&last_ping_ts_key, SERIALIZABLE),
						txs.read_opt(&expired_ts_key, SERIALIZABLE),
					)?;

					let (
						Some(workflow_id),
						Some(namespace_id),
						Some(name),
						Some(version),
						Some(remaining_slots),
						Some(total_slots),
						Some(old_last_ping_ts),
					) = (
						workflow_id_entry,
						namespace_id_entry,
						name_entry,
						version_entry,
						remaining_slots_entry,
						total_slots_entry,
						last_ping_ts_entry,
					)
					else {
						tracing::debug!(runner_id=?runner.runner_id, "runner has not initiated yet");
						continue;
					};

					// Runner is expired, updating will do nothing
					if expired_ts_entry.is_some() {
						notifications.push(RunnerNotification {
							runner_id: runner.runner_id,
							workflow_id,
							eligibility: RunnerEligibility::Expired,
						});

						continue;
					}

					let remaining_millislots = (remaining_slots * 1000) / total_slots;

					let old_alloc_key = keys::datacenter::RunnerAllocIdxKey::new(
						namespace_id,
						name.clone(),
						version,
						remaining_millislots,
						old_last_ping_ts,
						runner.runner_id,
					);

					// Add read conflict
					txs.add_conflict_key(&old_alloc_key, ConflictRangeType::Read)?;

					match runner.action {
						Action::ClearIdx => {
							txs.delete(&old_alloc_key);
						}
						Action::AddIdx => {
							txs.write(
								&old_alloc_key,
								rivet_key_data::converted::RunnerAllocIdxKeyData {
									workflow_id,
									remaining_slots,
									total_slots,
								},
							)?;
						}
						Action::UpdatePing { rtt } => {
							let last_ping_ts = util::timestamp::now();

							// Write new ping
							txs.write(&last_ping_ts_key, last_ping_ts)?;

							let last_rtt_key = keys::runner::LastRttKey::new(runner.runner_id);
							txs.write(&last_rtt_key, rtt)?;

							// Only update allocation idx if it existed before
							if txs.exists(&old_alloc_key, SERIALIZABLE).await? {
								// Clear old key
								txs.delete(&old_alloc_key);

								txs.write(
									&keys::datacenter::RunnerAllocIdxKey::new(
										namespace_id,
										name.clone(),
										version,
										remaining_millislots,
										last_ping_ts,
										runner.runner_id,
									),
									rivet_key_data::converted::RunnerAllocIdxKeyData {
										workflow_id,
										remaining_slots,
										total_slots,
									},
								)?;

								if last_ping_ts.saturating_sub(old_last_ping_ts)
									> RUNNER_ELIGIBLE_THRESHOLD_MS
								{
									notifications.push(RunnerNotification {
										runner_id: runner.runner_id,
										workflow_id,
										eligibility: RunnerEligibility::ReEligible,
									});
								}
							}
						}
					}
				}

				Ok(notifications)
			}
		})
		.custom_instrument(tracing::info_span!("runner_update_alloc_idx_tx"))
		.await?;

	Ok(Output { notifications })
}
