//! Typed operator commands and split reports use the same exact-worker UPS route.

use crate::tablet_maintainer_machine::runner::{SplitReportFuture, SplitReporter};
use anyhow::{Result, bail, ensure};
use depot_types::keys::pubsub::TabletCoordinatorSubject;
use rivet_data::{
	generated::depot_tablet_coordinator_protocol_v1::{
		AlarmCoordinatorRequest, AlarmIndexReady, AlarmSplitPrepared, SplitPrepared,
		TabletCoordinatorRequest, TabletCoordinatorResponse,
	},
	versioned::{DepotAlarmCoordinatorRequestData, DepotTabletCoordinatorResponseData},
};
use rivet_singleton_coordinator::singleton_coordinator_machine::{
	executor::LeaseStore, types::LeaseRead,
};
use rivet_topology::Topology;
use std::{sync::Arc, time::Duration};
use tokio::sync::watch;
use universalpubsub::{PubSub, SubjectCapacities};
use vbare::Versioned;

/// Discovery is a routing hint. The receiving coordinator checks live tenure authority.
/// This adapter never retries a command: a lost reply can hide a completed manifest mutation.
#[derive(Clone)]
pub struct CoordinatorRequests {
	pubsub: PubSub,
	topology: Arc<Topology>,
	lease: Arc<dyn LeaseStore>,
	capacities: SubjectCapacities,
	timeout: Duration,
	manifest_refresh: Option<watch::Sender<()>>,
}

impl CoordinatorRequests {
	pub fn new(
		pubsub: PubSub,
		topology: Arc<Topology>,
		lease: Arc<dyn LeaseStore>,
		capacities: SubjectCapacities,
		timeout: Duration,
	) -> Result<Self> {
		ensure!(
			!timeout.is_zero(),
			"coordinator request timeout must be positive"
		);
		ensure!(
			capacities.incoming > 0 && capacities.outgoing > 0,
			"coordinator capacities must be positive"
		);
		Ok(Self {
			pubsub,
			topology,
			lease,
			capacities,
			timeout,
			manifest_refresh: None,
		})
	}

	/// Published or superseded phase reports wake this worker's assignment watcher immediately.
	pub fn with_manifest_refresh(mut self, refresh: watch::Sender<()>) -> Self {
		self.manifest_refresh = Some(refresh);
		self
	}

	/// Resolve once and send once within one deadline, including lease discovery and endpoint lookup.
	#[tracing::instrument(skip_all)]
	pub async fn request(
		&self,
		request: TabletCoordinatorRequest,
	) -> Result<TabletCoordinatorResponse> {
		self.request_envelope(AlarmCoordinatorRequest::TabletCoordinatorRequest(request))
			.await
	}
	pub(crate) async fn request_envelope(
		&self,
		request: AlarmCoordinatorRequest,
	) -> Result<TabletCoordinatorResponse> {
		let clock = self.pubsub.env().clock();
		let deadline = rivet_env::deadline_after(clock.as_ref(), self.timeout)?;
		ensure!(
			self.topology.may_admit()?,
			"coordinator caller is not serving"
		);
		let observed = rivet_env::timeout_at(clock.as_ref(), deadline, self.lease.read()).await?;
		let worker = match observed {
			LeaseRead::Found(lease) if !lease.released => lease.tenure.owner.worker_id,
			LeaseRead::Found(_) | LeaseRead::Missing | LeaseRead::Unknown => {
				bail!("coordinator route unavailable")
			}
		};

		let subject = TabletCoordinatorSubject::new(worker.clone());
		let _reservation = self.pubsub.reserve_request_subject(
			&subject,
			universalpubsub::RequestClass::DepotCoordinator,
			self.capacities,
		)?;
		let payload = DepotAlarmCoordinatorRequestData::serialize_with_embedded_version(
			&request,
			DepotAlarmCoordinatorRequestData::LATEST,
		)?;
		let remaining = deadline.saturating_sub(clock.observe()?.boot_ms);
		ensure!(remaining > 0, "coordinator discovery deadline expired");

		let reply = self
			.pubsub
			.request_with_timeout_guarded(
				universalpubsub::RequestClass::DepotCoordinator,
				&worker,
				subject,
				&payload,
				Duration::from_millis(remaining),
				&|| self.topology.may_admit().unwrap_or(false),
			)
			.await?;
		ensure!(
			self.topology.may_admit()?,
			"coordinator caller lost its lease before reply"
		);
		DepotTabletCoordinatorResponseData::deserialize_with_embedded_version(&reply)
	}
}

impl SplitReporter for CoordinatorRequests {
	fn report_with_alarms(
		&self,
		report: SplitPrepared,
		manifest_id: u64,
		maintainer: [u8; 16],
	) -> SplitReportFuture<'_> {
		Box::pin(async move {
			let result = self
				.request_envelope(AlarmCoordinatorRequest::AlarmSplitPrepared(
					AlarmSplitPrepared {
						report,
						manifest_id,
						maintainer,
					},
				))
				.await;
			split_completed(self.manifest_refresh.as_ref(), result)
		})
	}
	fn alarm_index_ready(&self, request: AlarmIndexReady) -> SplitReportFuture<'_> {
		Box::pin(async move {
			self.request_envelope(AlarmCoordinatorRequest::AlarmIndexReady(request))
				.await
				.map_err(|e| e.to_string())
		})
	}
	fn recover(
		&self,
		request: rivet_data::generated::depot_tablet_coordinator_protocol_v1::RecoverTablets,
	) -> SplitReportFuture<'_> {
		Box::pin(async move {
			self.request(TabletCoordinatorRequest::RecoverTablets(request))
				.await
				.map_err(|error| error.to_string())
		})
	}

	fn report_catch_up(
		&self,
		request: rivet_data::generated::depot_tablet_coordinator_protocol_v1::CatchUpReport,
	) -> SplitReportFuture<'_> {
		Box::pin(async move {
			let result = self
				.request(TabletCoordinatorRequest::CatchUpReport(request.clone()))
				.await;
			catch_up_completed(self.manifest_refresh.as_ref(), &request, result)
		})
	}

	fn report_deleted(
		&self,
		request: rivet_data::generated::depot_tablet_coordinator_protocol_v1::RetiredTabletDeleted,
	) -> SplitReportFuture<'_> {
		Box::pin(async move {
			self.request(TabletCoordinatorRequest::RetiredTabletDeleted(request))
				.await
				.map_err(|error| error.to_string())
		})
	}

	fn report_parent_log_imported(
		&self,
		request: rivet_data::generated::depot_tablet_coordinator_protocol_v1::ParentLogImported,
	) -> SplitReportFuture<'_> {
		Box::pin(async move {
			self.request(TabletCoordinatorRequest::ParentLogImported(request))
				.await
				.map_err(|error| error.to_string())
		})
	}

	fn report_version_marks(
		&self,
		request: rivet_data::generated::depot_tablet_coordinator_protocol_v1::VersionMarksReport,
	) -> SplitReportFuture<'_> {
		Box::pin(async move {
			self.request(TabletCoordinatorRequest::VersionMarksReport(request))
				.await
				.map_err(|error| error.to_string())
		})
	}

	fn report(&self, request: SplitPrepared) -> SplitReportFuture<'_> {
		Box::pin(async move {
			let result = self
				.request(TabletCoordinatorRequest::SplitPrepared(request))
				.await;
			split_completed(self.manifest_refresh.as_ref(), result)
		})
	}
}

/// An accepted split report has published the frozen parent's children in the stored
/// manifest. Reading it at once lets this worker's supervisor start the children instead of
/// keeping the parent frozen, refusing every request, until the fallback manifest poll.
fn split_completed(
	refresh: Option<&watch::Sender<()>>,
	result: Result<TabletCoordinatorResponse>,
) -> std::result::Result<TabletCoordinatorResponse, String> {
	if let (Ok(TabletCoordinatorResponse::Accepted(_)), Some(refresh)) = (&result, refresh) {
		refresh.send_replace(());
	}
	result.map_err(|error| error.to_string())
}

// A published phase must reach the local watcher even for final Stable, when
// the departing disk no longer needs stop checks. A definitive newer version
// also prompts a refresh after a committed report's reply was lost; overload is no such hint.
fn catch_up_completed(
	refresh: Option<&watch::Sender<()>>,
	report: &rivet_data::generated::depot_tablet_coordinator_protocol_v1::CatchUpReport,
	result: Result<TabletCoordinatorResponse>,
) -> std::result::Result<TabletCoordinatorResponse, String> {
	use rivet_data::generated::depot_tablet_coordinator_protocol_v1::TabletCoordinatorRejectResponse as Reject;
	let changed = match &result {
		Ok(TabletCoordinatorResponse::Accepted(_)) => true,
		Ok(TabletCoordinatorResponse::TabletCoordinatorRejectResponse(
			Reject::StaleTabletReplicaSet(stale),
		)) => stale.tablet_replica_set_version > report.tablet_replica_set_version,
		Ok(TabletCoordinatorResponse::TabletCoordinatorRejectResponse(
			Reject::StaleTabletGrantEpoch(stale),
		)) => stale.tablet_grant_epoch > report.tablet_grant_epoch,
		_ => false,
	};
	if changed {
		if let Some(refresh) = refresh {
			refresh.send_replace(());
		}
	}
	result.map_err(|error| error.to_string())
}


