use anyhow::Result;
use futures_util::TryStreamExt;
use gas::prelude::*;
use universaldb::options::StreamingMode;
use universaldb::utils::IsolationLevel::*;

use crate::{keys, metrics};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Input {
	pub namespace_id: Id,
	pub name: String,
	pub version: u32,
	/// Whether to publish drain signals via pubsub. Set to false if the caller
	/// will handle sending signals (e.g. from a workflow).
	pub send_runner_stop_signals: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Output {
	pub older_runner_workflow_ids: Vec<Id>,
}

#[operation]
pub async fn pegboard_runner_drain_older_versions(
	ctx: &OperationCtx,
	input: &Input,
) -> Result<Output> {
	let configs = ctx
		.op(crate::ops::runner_config::get::Input {
			runners: vec![(input.namespace_id, input.name.clone())],
			bypass_cache: false,
		})
		.await?;

	// Use config's drain_on_version_upgrade if config exists, otherwise default to false
	let drain_enabled = configs
		.into_iter()
		.next()
		.map(|c| c.config.drain_on_version_upgrade)
		.unwrap_or(false);

	if !drain_enabled {
		return Ok(Output {
			older_runner_workflow_ids: vec![],
		});
	}

	// Scan RunnerAllocIdxKey for older versions
	let older_runners = ctx
		.udb()?
		.run(|tx| async move {
			let tx = tx.with_subspace(keys::subspace());
			let mut older_runners = Vec::new();

			let runner_alloc_subspace = keys::subspace().subspace(
				&keys::ns::RunnerAllocIdxKey::subspace(input.namespace_id, input.name.clone()),
			);

			let mut stream = tx.get_ranges_keyvalues(
				universaldb::RangeOption {
					mode: StreamingMode::WantAll,
					..(&runner_alloc_subspace).into()
				},
				Snapshot,
			);

			while let Some(entry) = stream.try_next().await? {
				let (key, data) = tx.read_entry::<keys::ns::RunnerAllocIdxKey>(&entry)?;

				// Only collect runners with older versions
				if key.version < input.version {
					older_runners.push(data.workflow_id);
				}
			}

			Ok(older_runners)
		})
		.custom_instrument(tracing::info_span!("drain_older_versions_tx"))
		.await?;

	if !older_runners.is_empty() {
		tracing::info!(
			namespace_id = %input.namespace_id,
			runner_name = %input.name,
			new_version = input.version,
			older_runner_count = older_runners.len(),
			"draining older runner versions due to drain_on_version_upgrade"
		);

		metrics::RUNNER_VERSION_UPGRADE_DRAIN
			.with_label_values(&[&input.namespace_id.to_string(), &input.name])
			.inc_by(older_runners.len() as u64);

		if input.send_runner_stop_signals {
			for workflow_id in &older_runners {
				ctx.signal(crate::workflows::runner2::Stop {
					reset_actor_rescheduling: false,
				})
				.to_workflow_id(*workflow_id)
				.send()
				.await?;
			}
		}
	}

	Ok(Output {
		older_runner_workflow_ids: older_runners,
	})
}
