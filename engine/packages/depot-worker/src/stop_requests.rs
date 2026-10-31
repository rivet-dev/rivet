//! Exact-worker readiness transport and expiring Active-storage discovery evidence.

use crate::tablet_coordinator_machine::{runner::StopTransport, types::StopTopology};
use depot_types::keys::pubsub::StopReadinessSubject;
use futures_util::future::BoxFuture;
use rivet_data::{
	DEPOT_STORAGE_WORKER_PROTOCOL_VERSION,
	generated::depot_storage_worker_protocol_v1::{StopReadiness, StopReadinessResponse},
	versioned::{DepotStopReadinessData, DepotStopReadinessResponseData},
};
use rivet_topology::{Topology, worker::WorkerId};
use std::{sync::Arc, time::Duration};
use universalpubsub::{PubSub, SubjectCapacities};
use vbare::Versioned;

pub struct StopRequests {
	pub replicas: Arc<dyn crate::tablet_maintainer_machine::runner::ReplicaTransport>,
	pub pubsub: PubSub,
	pub topology: Arc<Topology>,
	pub capacities: SubjectCapacities,
	/// A full round may visit many volumes; ordinary lifecycle RPC timeouts are too short.
	pub timeout: Duration,
}
impl StopTransport for StopRequests {
	fn disk_incarnation(&self, worker: [u8; 16]) -> BoxFuture<'_, anyhow::Result<[u8; 16]>> {
		Box::pin(async move {
			use rivet_data::generated::depot_storage_worker_protocol_v1::{
				StorageWorkerRequest, StorageWorkerResponse,
			};
			let reply = self
				.replicas
				.request(
					WorkerId::from_bytes(worker),
					StorageWorkerRequest::GetDiskIncarnationRequest,
				)
				.await
				.map_err(anyhow::Error::msg)?;
			let StorageWorkerResponse::GetDiskIncarnationResponse(reply) = reply else {
				anyhow::bail!("storage worker is not Serving")
			};
			Ok(reply.disk_incarnation)
		})
	}
	fn topology(&self) -> BoxFuture<'_, anyhow::Result<StopTopology>> {
		Box::pin(async move {
			self.topology.storage_workers().await?;
			let snapshot = self.topology.snapshot()?;
			anyhow::ensure!(
				self.pubsub.env().clock().observe()?.boot_ms < snapshot.valid_until_boot_ms,
				"storage discovery expired"
			);
			Ok(StopTopology {
				capacities: snapshot
					.storage_workers
					.iter()
					.map(|w| (*w.id.as_bytes(), w.storage_capacity_millicores))
					.collect(),
				workers: snapshot
					.storage_workers
					.iter()
					.map(|worker| (*worker.id.as_bytes(), *worker.instance_id.as_bytes()))
					.collect(),
				versions: snapshot
					.storage_workers
					.iter()
					.map(|worker| (*worker.id.as_bytes(), worker.versions.clone()))
					.collect(),
				valid_until_boot_ms: snapshot.valid_until_boot_ms,
			})
		})
	}
	fn next_storage_workers(
		&self,
		previous: Option<StopTopology>,
	) -> BoxFuture<'_, anyhow::Result<StopTopology>> {
		Box::pin(async move {
			let mut snapshots = self.topology.subscribe()?;
			let clock = self.pubsub.env().clock();
			let mut rng = self.pubsub.env().rng("depot.coordinator.discovery");
			loop {
				let (observed, valid_until_boot_ms) = {
					let snapshot = snapshots.borrow_and_update();
					(
						StopTopology {
							capacities: snapshot
								.storage_workers
								.iter()
								.map(|w| (*w.id.as_bytes(), w.storage_capacity_millicores))
								.collect(),
							workers: snapshot
								.storage_workers
								.iter()
								.map(|worker| {
									(*worker.id.as_bytes(), *worker.instance_id.as_bytes())
								})
								.collect(),
							versions: snapshot
								.storage_workers
								.iter()
								.map(|worker| (*worker.id.as_bytes(), worker.versions.clone()))
								.collect(),
							valid_until_boot_ms: snapshot.valid_until_boot_ms,
						},
						snapshot.valid_until_boot_ms,
					)
				};
				let now_boot_ms = clock.observe()?.boot_ms;
				if now_boot_ms < valid_until_boot_ms && previous.as_ref() != Some(&observed) {
					return Ok(observed);
				}
				if now_boot_ms >= valid_until_boot_ms {
					// Topology lists again when its view expires; a failed listing waits for the
					// next snapshot change instead of retrying at once.
					match self.topology.storage_workers().await {
						Ok(_) => continue,
						Err(error) => {
							tracing::debug!(?error, "storage discovery refresh failed");
							snapshots.changed().await?;
							continue;
						}
					}
				}
				rivet_env::select!(rng;
					changed = snapshots.changed() => changed?,
					expired = clock.sleep_until(valid_until_boot_ms) => expired?,
				);
			}
		})
	}
	fn handoff_readiness(
		&self,
		worker: [u8; 16],
		request: rivet_data::generated::depot_storage_worker_protocol_v1::StopReadiness,
	) -> BoxFuture<
		'_,
		anyhow::Result<
			rivet_data::generated::depot_storage_worker_protocol_v1::HandoffReadinessResponse,
		>,
	> {
		Box::pin(async move {
			anyhow::ensure!(self.topology.may_admit()?, "handoff caller is not serving");
			let worker = WorkerId::from_bytes(worker);
			let subject = StopReadinessSubject::new(worker.clone());
			let _reservation = self.pubsub.reserve_request_subject(
				&subject,
				universalpubsub::RequestClass::DepotStopReadiness,
				self.capacities,
			)?;
			let payload = DepotStopReadinessData::serialize_with_embedded_version(
				&request,
				rivet_data::DEPOT_STORAGE_WORKER_PROTOCOL_VERSION,
			)?;
			let reply = self
				.pubsub
				.request_with_timeout_guarded(
					universalpubsub::RequestClass::DepotStopReadiness,
					&worker,
					subject,
					&payload,
					self.timeout,
					&|| self.topology.may_admit().unwrap_or(false),
				)
				.await?;
			anyhow::ensure!(self.topology.may_admit()?, "handoff caller lost lease");
			decode_handoff_reply(&request, &reply)
		})
	}

	fn grant_written(
		&self,
		worker: [u8; 16],
		request: StopReadiness,
	) -> BoxFuture<'_, anyhow::Result<()>> {
		Box::pin(async move {
			anyhow::ensure!(
				self.topology.may_admit()?,
				"grant notice caller is not serving"
			);
			let worker = WorkerId::from_bytes(worker);
			let subject = StopReadinessSubject::new(worker.clone());
			let _reservation = self.pubsub.reserve_request_subject(
				&subject,
				universalpubsub::RequestClass::DepotStopReadiness,
				self.capacities,
			)?;
			let payload = DepotStopReadinessData::serialize_with_embedded_version(
				&request,
				DEPOT_STORAGE_WORKER_PROTOCOL_VERSION,
			)?;
			self.pubsub
				.request_with_timeout_guarded(
					universalpubsub::RequestClass::DepotStopReadiness,
					&worker,
					subject,
					&payload,
					self.timeout,
					&|| self.topology.may_admit().unwrap_or(false),
				)
				.await?;
			Ok(())
		})
	}

	fn readiness(
		&self,
		worker: [u8; 16],
		request: StopReadiness,
	) -> BoxFuture<'_, StopReadinessResponse> {
		Box::pin(async move {
			let result = async {
				anyhow::ensure!(
					self.topology.may_admit()?,
					"readiness caller is not serving"
				);
				let worker = WorkerId::from_bytes(worker);
				let subject = StopReadinessSubject::new(worker.clone());
				let _reservation = self.pubsub.reserve_request_subject(
					&subject,
					universalpubsub::RequestClass::DepotStopReadiness,
					self.capacities,
				)?;
				let payload = DepotStopReadinessData::serialize_with_embedded_version(
					&request,
					DEPOT_STORAGE_WORKER_PROTOCOL_VERSION,
				)?;
				let reply = self
					.pubsub
					.request_with_timeout_guarded(
						universalpubsub::RequestClass::DepotStopReadiness,
						&worker,
						subject,
						&payload,
						self.timeout,
						&|| self.topology.may_admit().unwrap_or(false),
					)
					.await?;
				anyhow::ensure!(self.topology.may_admit()?, "readiness caller lost lease");
				DepotStopReadinessResponseData::deserialize_with_embedded_version(&reply)
			}
			.await;
			result.unwrap_or_else(|error| {
				tracing::warn!(%error, maintainer = %WorkerId::from_bytes(worker), "stop readiness request failed");
				StopReadinessResponse::NotServing
			})
		})
	}
}

/// Decode capability from the actual wire reply, never from the retained request purpose.
fn decode_handoff_reply(
	request: &rivet_data::generated::depot_storage_worker_protocol_v1::StopReadiness,
	reply: &[u8],
) -> anyhow::Result<rivet_data::generated::depot_storage_worker_protocol_v1::HandoffReadinessResponse>
{
	let proof = rivet_data::versioned::DepotHandoffReadinessResponseData::deserialize_with_embedded_version(reply)?;
	anyhow::ensure!(
		rivet_data::versioned::handoff_proof_matches(request, &proof),
		"handoff proof names a different authority"
	);
	Ok(proof)
}


