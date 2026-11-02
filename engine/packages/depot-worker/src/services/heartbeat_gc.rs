//! Hosts topology's heartbeat garbage collector under the storage owner's singleton election,
//! so one collector runs per cluster. Topology owns the collector; this module only adapts its
//! entry point to the election's `SingletonServices` contract and resolves its configuration.

use anyhow::{Context, Result};
use object_store::ObjectStore;
use rivet_config::config::depot::HeartbeatGcConfig;
use rivet_env::Env;
use rivet_singleton_coordinator::{
	authority::Authority, runtime::SingletonServices,
	singleton_coordinator_machine::types::CoordinatorTenure,
};
use rivet_topology::{Topology, heartbeat_gc::Config};
use std::sync::Arc;

/// Topology's defaults with the operator's overrides applied.
pub fn config(overrides: &HeartbeatGcConfig) -> Config {
	let defaults = Config::default();
	Config {
		retention_ms: overrides.retention_ms.unwrap_or(defaults.retention_ms),
		batch_size: overrides.batch_size.unwrap_or(defaults.batch_size),
		delete_concurrency: overrides
			.delete_concurrency
			.unwrap_or(defaults.delete_concurrency),
		batch_timeout_ms: overrides
			.batch_timeout_ms
			.unwrap_or(defaults.batch_timeout_ms),
		batch_interval_ms: overrides
			.batch_interval_ms
			.unwrap_or(defaults.batch_interval_ms),
		pass_interval_ms: overrides
			.pass_interval_ms
			.unwrap_or(defaults.pass_interval_ms),
	}
}

pub struct HeartbeatGcHost {
	env: Arc<dyn Env>,
	store: Arc<dyn ObjectStore>,
	topology: Arc<Topology>,
	config: Config,
}

impl HeartbeatGcHost {
	/// Validates `config` against the topology lease timing now, so a misconfiguration fails
	/// the storage service at start instead of ending every tenure the election grants.
	pub fn new(
		env: Arc<dyn Env>,
		store: Arc<dyn ObjectStore>,
		topology: Arc<Topology>,
		config: Config,
	) -> Result<Self> {
		config.validate(&topology.config().lease)?;
		Ok(Self {
			env,
			store,
			topology,
			config,
		})
	}
}

#[async_trait::async_trait]
impl SingletonServices for HeartbeatGcHost {
	/// Runs until the tenure is revoked. A collector failure is logged and the collector
	/// restarted after one pass interval instead of returned, because returning would end the
	/// shared tenure and bounce the tablet coordinator for a job that can simply wait.
	async fn run(&self, authority: Authority) -> Result<()> {
		let clock = self.env.clock();
		loop {
			let collector = rivet_topology::services::heartbeat_gc::start(
				self.env.clone(),
				self.store.clone(),
				self.topology.clone(),
				self.config.clone(),
			);
			let error = rivet_env::select_biased!("revocation ends the collector before its next batch",
				_ = authority.revoked() => return Ok(()),
				result = collector => match result {
					Ok(()) => anyhow::anyhow!("heartbeat collector returned without an error"),
					Err(error) => error,
				},
			);
			tracing::error!(
				?error,
				retry_after_ms = self.config.pass_interval_ms,
				"heartbeat GC stopped; restarting after the pass interval"
			);
			let retry_at = clock
				.observe()?
				.boot_ms
				.checked_add(self.config.pass_interval_ms)
				.context("heartbeat GC restart deadline overflow")?;
			rivet_env::select_biased!("revocation wins over the restart delay",
				_ = authority.revoked() => return Ok(()),
				result = clock.sleep_until(retry_at) => result?,
			);
		}
	}

	/// Nothing to reconcile: every delete the cancelled run sent was conditioned on the ETag
	/// of a record older than the retention, which is safe whoever completes it, and the next
	/// tenure's collector re-lists every record instead of resuming this one's listing.
	async fn reconcile_stopped(&self, _: &CoordinatorTenure) -> Result<()> {
		Ok(())
	}
}
