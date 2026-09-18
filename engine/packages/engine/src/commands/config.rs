use std::collections::HashMap;

use anyhow::*;
use clap::Parser;
use rivet_config::DynamicConfigUpdate;
use rivet_dynamic_config::{SetDynamicConfigMessage, pubsub_subjects::DynamicConfigSubject};
use universalpubsub::PublishOpts;

#[derive(Parser)]
pub enum SubCommand {
	Show,
	/// Update config properties at runtime, on every engine process in the datacenter
	Update {
		#[clap(subcommand)]
		command: UpdateSubCommand,
	},
}

#[derive(Parser)]
pub enum UpdateSubCommand {
	/// Maximum bytes of FoundationDB storage a single SQLite database may occupy
	MaxStorageBytes {
		/// Omit to revert to the value in the config file
		bytes: Option<u64>,
	},
	/// Percentage (0-100) of database branches admitted to run compaction
	CompactionAdmissionPercent {
		/// Omit to revert to the value in the config file
		percent: Option<f64>,
	},
	/// Cluster-wide budget, in bytes per second, for FoundationDB writes issued by depot compaction
	CompactionWriteBytesPerSecond {
		/// Omit to revert to the value in the config file
		bytes_per_second: Option<u64>,
	},
	/// Cluster-wide budget, in bytes per second, for FoundationDB reads issued by depot compaction
	CompactionReadBytesPerSecond {
		/// Omit to revert to the value in the config file
		bytes_per_second: Option<u64>,
	},
	/// Cluster-wide budget, in bytes per second, for FoundationDB writes issued by actor commits,
	/// including each segment of a staged commit. Omit the value to leave it unthrottled.
	ActorWriteBytesPerSecond {
		/// Omit to revert to the value in the config file
		bytes_per_second: Option<u64>,
	},
	/// Cluster-wide budget, in bytes per second, for FoundationDB reads issued by actor get_pages.
	/// Omit the value to leave it unthrottled.
	ActorReadBytesPerSecond {
		/// Omit to revert to the value in the config file
		bytes_per_second: Option<u64>,
	},
	/// Whether hot compaction folds shard images straight into the live shard tier instead of staging
	/// them for install to copy across. Halves hot compaction's FDB write volume.
	CompactionHotFoldDirectToShard {
		/// Omit to revert to the value in the config file
		enabled: Option<bool>,
	},
	/// Largest span of txids one hot compaction drain folds before its install advances the hot
	/// watermark. The watermark is what makes a delta reclaimable, so this is also the granularity
	/// of reclaim eligibility: lower it to fold in smaller chunks so reclaim interleaves and the
	/// peak footprint comes down, at the cost of more jobs. Minimum 128.
	CompactionMaxHotDrainSpanTxids {
		/// Omit to revert to the value in the config file
		txids: Option<u64>,
	},
	/// Multiplier hot staging measures the shared compaction budget against. Below 1 holds the lane
	/// that writes a second copy back so install and reclaim keep admitting. Inert while
	/// compaction-hot-fold-direct-to-shard is on.
	CompactionStageThrottleBudgetMultiplier {
		/// Omit to revert to the value in the config file
		multiplier: Option<f64>,
	},
	/// Utilization of the staging budget below which every staging charge is admitted
	CompactionStageThrottleAdmitSoftUtil {
		/// Omit to revert to the value in the config file
		util: Option<f64>,
	},
	/// How long, in milliseconds, a throttled staging slice backs off. Inert while
	/// compaction-hot-fold-direct-to-shard is on.
	CompactionStageThrottleBackoffMs {
		/// Omit to revert to the value in the config file
		backoff_ms: Option<i64>,
	},
	/// Multiplier the lanes that advance without duplicating measure the budget against: hot install,
	/// cold publish, cold staging, and direct-to-shard folds
	CompactionInstallThrottleBudgetMultiplier {
		/// Omit to revert to the value in the config file
		multiplier: Option<f64>,
	},
	/// Utilization of the advancing lanes' budget below which every such charge is admitted
	CompactionInstallThrottleAdmitSoftUtil {
		/// Omit to revert to the value in the config file
		util: Option<f64>,
	},
	/// Multiplier reclaim measures the budget against. Above 1 so deletion keeps admitting where the
	/// producing lanes have backed off, and the only class that may exceed the configured rate.
	CompactionReclaimThrottleBudgetMultiplier {
		/// Omit to revert to the value in the config file
		multiplier: Option<f64>,
	},
	/// Utilization of the reclaim budget below which every reclaim charge is admitted
	CompactionReclaimThrottleAdmitSoftUtil {
		/// Omit to revert to the value in the config file
		util: Option<f64>,
	},
	/// Worker pull interval in milliseconds
	WorkerPollIntervalMs {
		/// Omit to revert to the value in the config file
		milliseconds: Option<u64>,
	},
	/// Maximum wake keys read per workflow name in one pull. Pass "default" as the workflow name
	/// to apply to all workflow names.
	WorkerMaxWakeKeysPerWorkflowNamePerPull {
		workflow_name: String,
		/// Omit to revert to the value in the config file
		max: Option<usize>,
	},
	/// Maximum unique workflows retained after wake-key deduplication in one pull
	WorkerMaxDedupedWorkflowsPerPull {
		/// Omit to revert to the value in the config file
		max: Option<usize>,
	},
	/// Maximum assigned workflow candidates attempted in one pull
	WorkerMaxWorkflowsPerPull {
		/// Omit to revert to the value in the config file
		max: Option<usize>,
	},
	/// Maximum wake-condition keys eligible for clearing in one pull
	WorkerMaxWakeConditionClearsPerPull {
		/// Omit to revert to the value in the config file
		max: Option<usize>,
	},
	/// Maximum concurrently running workflows of one workflow name for the entire cluster
	WorkerMaxConcurrentWorkflows {
		workflow_name: String,
		/// Omit to revert to the value in the config file
		max: Option<usize>,
	},
}

impl SubCommand {
	pub async fn execute(self, config: rivet_config::Config) -> Result<()> {
		match self {
			Self::Show => {
				println!("{:#?}", *config);
				Ok(())
			}
			Self::Update { command } => command.execute(config).await,
		}
	}
}

impl UpdateSubCommand {
	pub async fn execute(self, config: rivet_config::Config) -> Result<()> {
		let (update, property, value) = match self {
			Self::WorkerPollIntervalMs { milliseconds } => (
				DynamicConfigUpdate {
					worker_poll_interval_ms: Some(milliseconds),
					..DynamicConfigUpdate::default()
				},
				"runtime.worker_poll_interval_ms".to_string(),
				milliseconds.map(|value| value.to_string()),
			),
			Self::WorkerMaxWakeKeysPerWorkflowNamePerPull { workflow_name, max } => (
				DynamicConfigUpdate {
					worker_max_wake_keys_per_workflow_name_per_pull: Some(HashMap::from([(
						workflow_name.clone(),
						max,
					)])),
					..DynamicConfigUpdate::default()
				},
				format!("runtime.worker_max_wake_keys_per_workflow_name_per_pull.{workflow_name}"),
				max.map(|value| value.to_string()),
			),
			Self::WorkerMaxDedupedWorkflowsPerPull { max } => (
				DynamicConfigUpdate {
					worker_max_deduped_workflows_per_pull: Some(max),
					..DynamicConfigUpdate::default()
				},
				"runtime.worker_max_deduped_workflows_per_pull".to_string(),
				max.map(|value| value.to_string()),
			),
			Self::WorkerMaxWorkflowsPerPull { max } => (
				DynamicConfigUpdate {
					worker_max_workflows_per_pull: Some(max),
					..DynamicConfigUpdate::default()
				},
				"runtime.worker_max_workflows_per_pull".to_string(),
				max.map(|value| value.to_string()),
			),
			Self::WorkerMaxWakeConditionClearsPerPull { max } => (
				DynamicConfigUpdate {
					worker_max_wake_condition_clears_per_pull: Some(max),
					..DynamicConfigUpdate::default()
				},
				"runtime.worker_max_wake_condition_clears_per_pull".to_string(),
				max.map(|value| value.to_string()),
			),
			Self::MaxStorageBytes { bytes } => (
				DynamicConfigUpdate {
					max_storage_bytes: Some(bytes),
					..DynamicConfigUpdate::default()
				},
				"sqlite.max_storage_bytes".to_string(),
				bytes.map(|bytes| bytes.to_string()),
			),
			Self::CompactionAdmissionPercent { percent } => (
				DynamicConfigUpdate {
					compaction_admission_percent: Some(percent),
					..DynamicConfigUpdate::default()
				},
				"sqlite.compaction_admission_percent".to_string(),
				percent.map(|percent| percent.to_string()),
			),
			Self::CompactionWriteBytesPerSecond { bytes_per_second } => (
				DynamicConfigUpdate {
					compaction_write_bytes_per_second: Some(bytes_per_second),
					..DynamicConfigUpdate::default()
				},
				"sqlite.compaction_write_bytes_per_second".to_string(),
				bytes_per_second.map(|bytes| bytes.to_string()),
			),
			Self::ActorWriteBytesPerSecond { bytes_per_second } => (
				DynamicConfigUpdate {
					actor_write_bytes_per_second: Some(bytes_per_second),
					..DynamicConfigUpdate::default()
				},
				"sqlite.actor_write_bytes_per_second".to_string(),
				bytes_per_second.map(|bytes| bytes.to_string()),
			),
			Self::ActorReadBytesPerSecond { bytes_per_second } => (
				DynamicConfigUpdate {
					actor_read_bytes_per_second: Some(bytes_per_second),
					..DynamicConfigUpdate::default()
				},
				"sqlite.actor_read_bytes_per_second".to_string(),
				bytes_per_second.map(|bytes| bytes.to_string()),
			),
			Self::CompactionReadBytesPerSecond { bytes_per_second } => (
				DynamicConfigUpdate {
					compaction_read_bytes_per_second: Some(bytes_per_second),
					..DynamicConfigUpdate::default()
				},
				"sqlite.compaction_read_bytes_per_second".to_string(),
				bytes_per_second.map(|bytes| bytes.to_string()),
			),
			Self::CompactionHotFoldDirectToShard { enabled } => (
				DynamicConfigUpdate {
					compaction_hot_fold_direct_to_shard: Some(enabled),
					..DynamicConfigUpdate::default()
				},
				"sqlite.compaction_hot_fold_direct_to_shard".to_string(),
				enabled.map(|enabled| enabled.to_string()),
			),
			Self::CompactionMaxHotDrainSpanTxids { txids } => (
				DynamicConfigUpdate {
					compaction_max_hot_drain_span_txids: Some(txids),
					..DynamicConfigUpdate::default()
				},
				"sqlite.compaction_max_hot_drain_span_txids".to_string(),
				txids.map(|txids| txids.to_string()),
			),
			Self::CompactionStageThrottleBudgetMultiplier { multiplier } => (
				DynamicConfigUpdate {
					compaction_stage_throttle_budget_multiplier: Some(multiplier),
					..DynamicConfigUpdate::default()
				},
				"sqlite.compaction_stage_throttle_budget_multiplier".to_string(),
				multiplier.map(|multiplier| multiplier.to_string()),
			),
			Self::CompactionStageThrottleAdmitSoftUtil { util } => (
				DynamicConfigUpdate {
					compaction_stage_throttle_admit_soft_util: Some(util),
					..DynamicConfigUpdate::default()
				},
				"sqlite.compaction_stage_throttle_admit_soft_util".to_string(),
				util.map(|util| util.to_string()),
			),
			Self::CompactionStageThrottleBackoffMs { backoff_ms } => (
				DynamicConfigUpdate {
					compaction_stage_throttle_backoff_ms: Some(backoff_ms),
					..DynamicConfigUpdate::default()
				},
				"sqlite.compaction_stage_throttle_backoff_ms".to_string(),
				backoff_ms.map(|backoff_ms| backoff_ms.to_string()),
			),
			Self::CompactionInstallThrottleBudgetMultiplier { multiplier } => (
				DynamicConfigUpdate {
					compaction_install_throttle_budget_multiplier: Some(multiplier),
					..DynamicConfigUpdate::default()
				},
				"sqlite.compaction_install_throttle_budget_multiplier".to_string(),
				multiplier.map(|multiplier| multiplier.to_string()),
			),
			Self::CompactionInstallThrottleAdmitSoftUtil { util } => (
				DynamicConfigUpdate {
					compaction_install_throttle_admit_soft_util: Some(util),
					..DynamicConfigUpdate::default()
				},
				"sqlite.compaction_install_throttle_admit_soft_util".to_string(),
				util.map(|util| util.to_string()),
			),
			Self::CompactionReclaimThrottleBudgetMultiplier { multiplier } => (
				DynamicConfigUpdate {
					compaction_reclaim_throttle_budget_multiplier: Some(multiplier),
					..DynamicConfigUpdate::default()
				},
				"sqlite.compaction_reclaim_throttle_budget_multiplier".to_string(),
				multiplier.map(|multiplier| multiplier.to_string()),
			),
			Self::CompactionReclaimThrottleAdmitSoftUtil { util } => (
				DynamicConfigUpdate {
					compaction_reclaim_throttle_admit_soft_util: Some(util),
					..DynamicConfigUpdate::default()
				},
				"sqlite.compaction_reclaim_throttle_admit_soft_util".to_string(),
				util.map(|util| util.to_string()),
			),
			Self::WorkerMaxConcurrentWorkflows { workflow_name, max } => (
				DynamicConfigUpdate {
					worker_max_concurrent_workflows: Some(HashMap::from([(
						workflow_name.clone(),
						max,
					)])),
					..DynamicConfigUpdate::default()
				},
				format!("runtime.worker_max_concurrent_workflows.{workflow_name}"),
				max.map(|max| max.to_string()),
			),
		};

		// Apply against this process's own config first. The operator sees a rejected value as an
		// error here instead of finding it in every process's logs after the broadcast.
		config
			.apply_dynamic(&update)
			.with_context(|| format!("invalid value for {property}"))?;

		let message = serde_json::to_vec(&SetDynamicConfigMessage { update })?;

		let pools = rivet_pools::Pools::new(config).await?;
		pools
			.ups()?
			.publish(DynamicConfigSubject, &message, PublishOpts::broadcast())
			.await?;

		if let Some(value) = value {
			rivet_term::status::success("Updated", format!("{property} = {value}"));
		} else {
			rivet_term::status::success(
				"Reverted",
				format!("{property} back to the value in the config file"),
			);
		}

		Ok(())
	}
}
