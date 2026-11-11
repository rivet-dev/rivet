//! Runs several cluster singleton services under one election.
use std::sync::Arc;

use anyhow::Result;
use rivet_singleton_coordinator::{
	authority::Authority, runtime::SingletonServices,
	singleton_coordinator_machine::types::CoordinatorTenure,
};

/// Every service runs for the whole tenure. The tenure's run ends once all of them return, which
/// each does when the authority is revoked.
pub(crate) struct CompositeSingletons {
	/// Each service with a bounded name for stop diagnostics.
	services: Vec<(&'static str, Arc<dyn SingletonServices>)>,
}

impl CompositeSingletons {
	pub(crate) fn new(services: Vec<(&'static str, Arc<dyn SingletonServices>)>) -> Self {
		Self { services }
	}
}

#[async_trait::async_trait]
impl SingletonServices for CompositeSingletons {
	async fn run(&self, authority: Authority) -> Result<()> {
		let runs = self
			.services
			.iter()
			.map(|(_, service)| service.run(authority.clone()));
		let results = futures_util::future::join_all(runs).await;
		results.into_iter().collect::<Result<Vec<_>>>()?;
		Ok(())
	}

	/// Joins every service, then reports the first error, so one failure never leaves another
	/// service's tasks running past the lease.
	async fn reconcile_stopped(&self, tenure: &CoordinatorTenure) -> Result<()> {
		let mut first_error = None;
		for (name, service) in &self.services {
			let result = service.reconcile_stopped(tenure).await;
			// A service that never finishes stopping is the one named last before the
			// coordinator's stop watchdog exits the process.
			tracing::info!(
				service = name,
				ok = result.is_ok(),
				"singleton service reconciled"
			);
			if let Err(error) = result {
				tracing::warn!(?error, "a singleton service failed to stop cleanly");
				first_error.get_or_insert(error);
			}
		}
		match first_error {
			Some(error) => Err(error),
			None => Ok(()),
		}
	}
}
