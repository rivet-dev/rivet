use gas::prelude::*;
use rivet_runner_protocol::PROTOCOL_MK1_VERSION;
use universaldb::options::ConflictRangeType;
use universaldb::utils::IsolationLevel::*;

use crate::keys;

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
	let runner_eligible_threshold = ctx.config().pegboard().runner_eligible_threshold();

	let notifications = ctx
		.udb()?
		.run(|tx| {
			let runners = input.runners.clone();

			async move {
				let tx = tx.with_subspace(keys::subspace());
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
					let protocol_version_key =
						keys::runner::ProtocolVersionKey::new(runner.runner_id);
					let last_ping_ts_key = keys::runner::LastPingTsKey::new(runner.runner_id);
					let expired_ts_key = keys::runner::ExpiredTsKey::new(runner.runner_id);

					let (
						workflow_id_entry,
						namespace_id_entry,
						name_entry,
						version_entry,
						remaining_slots_entry,
						total_slots_entry,
						protocol_version_entry,
						last_ping_ts_entry,
						expired_ts_entry,
					) = tokio::try_join!(
						tx.read_opt(&workflow_id_key, Serializable),
						tx.read_opt(&namespace_id_key, Serializable),
						tx.read_opt(&name_key, Serializable),
						tx.read_opt(&version_key, Serializable),
						tx.read_opt(&remaining_slots_key, Serializable),
						tx.read_opt(&total_slots_key, Serializable),
						tx.read_opt(&protocol_version_key, Serializable),
						tx.read_opt(&last_ping_ts_key, Serializable),
						tx.read_opt(&expired_ts_key, Serializable),
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

					let protocol_version = protocol_version_entry.unwrap_or(PROTOCOL_MK1_VERSION);

					// Runner is expired, AddIdx is invalid and UpdatePing will do nothing
					if expired_ts_entry.is_some() {
						match runner.action {
							Action::ClearIdx => {}
							Action::AddIdx | Action::UpdatePing { .. } => {
								notifications.push(RunnerNotification {
									runner_id: runner.runner_id,
									workflow_id,
									eligibility: RunnerEligibility::Expired,
								});

								continue;
							}
						}
					}

					let remaining_millislots = (remaining_slots * 1000) / total_slots;

					let old_alloc_key = keys::ns::RunnerAllocIdxKey::new(
						namespace_id,
						name.clone(),
						version,
						remaining_millislots,
						old_last_ping_ts,
						runner.runner_id,
					);

					// Add read conflict
					tx.add_conflict_key(&old_alloc_key, ConflictRangeType::Read)?;

					match runner.action {
						Action::ClearIdx => {
							tx.delete(&old_alloc_key);
						}
						Action::AddIdx => {
							tx.write(
								&old_alloc_key,
								rivet_data::converted::RunnerAllocIdxKeyData {
									workflow_id,
									remaining_slots,
									total_slots,
									protocol_version,
								},
							)?;
						}
						Action::UpdatePing { rtt } => {
							let last_ping_ts = util::timestamp::now();

							// Write new ping
							tx.write(&last_ping_ts_key, last_ping_ts)?;

							let last_rtt_key = keys::runner::LastRttKey::new(runner.runner_id);
							tx.write(&last_rtt_key, rtt)?;

							// Only update allocation idx if it existed before
							if tx.exists(&old_alloc_key, Serializable).await? {
								// Clear old key
								tx.delete(&old_alloc_key);

								tx.write(
									&keys::ns::RunnerAllocIdxKey::new(
										namespace_id,
										name.clone(),
										version,
										remaining_millislots,
										last_ping_ts,
										runner.runner_id,
									),
									rivet_data::converted::RunnerAllocIdxKeyData {
										workflow_id,
										remaining_slots,
										total_slots,
										protocol_version,
									},
								)?;

								if last_ping_ts.saturating_sub(old_last_ping_ts)
									> runner_eligible_threshold
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
