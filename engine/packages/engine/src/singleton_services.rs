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
	services: Vec<Arc<dyn SingletonServices>>,
}

impl CompositeSingletons {
	pub(crate) fn new(services: Vec<Arc<dyn SingletonServices>>) -> Self {
		Self { services }
	}
}

#[async_trait::async_trait]
impl SingletonServices for CompositeSingletons {
	async fn run(&self, authority: Authority) -> Result<()> {
		let runs = self
			.services
			.iter()
			.map(|service| service.run(authority.clone()));
		let results = futures_util::future::join_all(runs).await;
		results.into_iter().collect::<Result<Vec<_>>>()?;
		Ok(())
	}

	/// Joins every service, then reports the first error, so one failure never leaves another
	/// service's tasks running past the lease.
	async fn reconcile_stopped(&self, tenure: &CoordinatorTenure) -> Result<()> {
		let mut first_error = None;
		for service in &self.services {
			if let Err(error) = service.reconcile_stopped(tenure).await {
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
