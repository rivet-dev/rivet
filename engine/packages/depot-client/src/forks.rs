//! Public fork and restore operations retain their exact identity across uncertainty.
use crate::Client;
use depot_fs_core::{FsForkError, FsForkNamed, FsForkSelector, FsForkSpec, FsVolumeRecord};
use depot_types::VolumeKey;
use rivet_data::generated::depot_client_protocol_v2::*;
use vbare::Versioned;

#[derive(Debug, thiserror::Error)]
pub enum Error {
	#[error("fork rejected: {0:?}")]
	Rejected(FsForkError),
	/// A maintainer refused the fork for good with a lifecycle answer: the parent or the
	/// destination is missing or deleted, the destination already exists, its group is archived,
	/// or a record is missing. Sending this body again cannot change the answer.
	#[error("fork rejected by the volume lifecycle: {0:?}")]
	LifecycleRejected(VolumeRejectResponse),
	/// This exact body may already be durable. Pass it to resume_fork; do not
	/// generate another operation id or recapture its Latest minimum.
	#[error("fork outcome is unknown; resume the retained operation")]
	Unknown { spec: Box<FsForkSpec> },
	/// The destination refused this attempt. The refusal does not prove that nothing ran: it
	/// can answer a caller of a fork job that is already durable and keeps running. Only this
	/// exact body may be sent again, with resume_fork after `retry_after_ms`, which joins that
	/// job; a newly prepared body would start a second fork.
	#[error("fork refused ({reason:?}); resume the retained operation after {retry_after_ms} ms")]
	Refused {
		spec: Box<FsForkSpec>,
		reason: Refusal,
		retry_after_ms: u64,
	},
	/// No fork command was sent. Source confirmation or local validation failed.
	#[error("fork preparation failed: {0}")]
	Preparation(#[source] anyhow::Error),
}

/// Why the destination refused a fork attempt.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Refusal {
	/// The fork job could not run yet: another job holds the destination, the parent has not
	/// applied the requested position, or a base or region is briefly unavailable. As
	/// `TailUnproven`, the parent's members hold a tail nothing proved committed, which waits
	/// for a recovery of the parent: a caller that knows the parent's writer is quiet can open
	/// the parent, whose claim proves the tail, before it resends.
	Fork(FsForkError),
	/// The destination's maintainer did not admit the request, or the request found no route
	/// to one and was not sent.
	Lifecycle(VolumeRejectResponse),
}

/// The hint of a fork refusal that carries none on the wire. A busy destination or a lagging
/// parent settles on the scale of a route refresh or an apply.
const FORK_REFUSED_RETRY_AFTER_MS: u64 = 100;

impl Error {
	/// Whether the fork was refused because the parent's tail is unproven.
	pub fn tail_unproven(&self) -> bool {
		matches!(
			self,
			Self::Refused {
				reason: Refusal::Fork(FsForkError::TailUnproven),
				..
			}
		)
	}

	/// The hint after which the edge sends the retained body again, or `None` for an error
	/// the edge must not retry.
	pub fn retry_after_ms(&self) -> Option<u64> {
		match self {
			Self::Refused { retry_after_ms, .. } => Some(*retry_after_ms),
			Self::Preparation(error) => crate::retry_after_ms(error),
			Self::Rejected(_) | Self::LifecycleRejected(_) | Self::Unknown { .. } => None,
		}
	}
}

impl Client {
	pub async fn fork(
		&self,
		parent: VolumeKey,
		at: FsForkSelector,
		destination: VolumeKey,
	) -> Result<FsVolumeRecord, Error> {
		let region = self.local_region().map_err(Error::Preparation)?;
		self.fork_to_region(parent, at, destination, region).await
	}

	/// The parent is in this client's region; the destination may be remote.
	pub async fn fork_to_region(
		&self,
		parent: VolumeKey,
		at: FsForkSelector,
		destination: VolumeKey,
		region: u16,
	) -> Result<FsVolumeRecord, Error> {
		let spec = self.prepare_fork(parent, at, destination, region).await?;
		self.resume_fork(spec).await
	}

	/// Preparing a request does not create a destination. Persist this body before
	/// dispatch if the application must resume after its own process restarts.
	pub async fn prepare_fork(
		&self,
		parent: VolumeKey,
		at: FsForkSelector,
		destination: VolumeKey,
		destination_region: u16,
	) -> Result<FsForkSpec, Error> {
		let parent_region = self.local_region().map_err(Error::Preparation)?;
		if !self
			.regions
			.as_ref()
			.is_some_and(|routes| routes.contains(destination_region))
		{
			return Err(Error::Rejected(FsForkError::RegionUnavailable));
		}
		let minimum = if matches!(at, FsForkSelector::FsForkLatest) {
			self.volume_handle()
				.map_err(|e| Error::Preparation(e.into()))?
				.fork_minimum(parent.clone())
				.await
				.map_err(|e| Error::Preparation(e.into()))?
		} else {
			None
		};
		let spec = FsForkSpec {
			operation_id: *self.env().rng("depot.filesystem.fork").uuid_v4().as_bytes(),
			parent_region,
			parent_volume_key: parent,
			selector: at,
			destination_region,
			destination_volume_key: destination,
			minimum,
			restore: None,
		};
		if !depot_fs_core::forks::valid_spec(&spec, destination_region) {
			return Err(Error::Rejected(FsForkError::Invalid));
		}
		Ok(spec)
	}

	pub async fn checkpoint_restore(
		&self,
		key: VolumeKey,
		name: Vec<u8>,
	) -> Result<FsVolumeRecord, Error> {
		let spec = self.prepare_checkpoint_restore(key, name).await?;
		self.resume_fork(spec).await
	}

	/// Resolve the current incarnation without claiming the volume from an exclusive writer.
	/// The target name can belong to an older retained incarnation of this key.
	pub async fn prepare_checkpoint_restore(
		&self,
		key: VolumeKey,
		name: Vec<u8>,
	) -> Result<FsForkSpec, Error> {
		if !depot_fs_core::checkpoints::valid_name(&name) {
			return Err(Error::Rejected(FsForkError::Invalid));
		}
		let region = self.local_region().map_err(Error::Preparation)?;
		let transport = self
			.volume_handle()
			.map_err(|e| Error::Preparation(e.into()))?
			.read_requests();
		let deadline = transport.deadline().map_err(Error::Preparation)?;
		let reply = crate::checkpoints::request(
			&transport,
			self.env().clock().as_ref(),
			key.clone(),
			CheckpointOperation::CheckpointInspect(CheckpointInspect {
				name: name.clone().into(),
			}),
			deadline,
		)
		.await
		.map_err(|e| Error::Preparation(e.into()))?;
		let VolumeCheckpointResponse::CheckpointInspection(inspection) = reply else {
			return Err(Error::Preparation(anyhow::anyhow!(
				"unexpected restore inspection"
			)));
		};
		Ok(FsForkSpec {
			operation_id: *self.env().rng("depot.filesystem.fork").uuid_v4().as_bytes(),
			parent_region: region,
			parent_volume_key: key.clone(),
			selector: FsForkSelector::FsForkNamed(FsForkNamed {
				incarnation: None,
				name: name.into(),
			}),
			destination_region: region,
			destination_volume_key: key,
			minimum: None,
			restore: Some(inspection.volume_incarnation),
		})
	}

	/// A new transport deadline does not change the logical request or its source
	/// image. Every ambiguous return carries the body needed to resume it again.
	pub async fn resume_fork(&self, spec: FsForkSpec) -> Result<FsVolumeRecord, Error> {
		if !depot_fs_core::forks::valid_spec(&spec, spec.destination_region) {
			return Err(Error::Rejected(FsForkError::Invalid));
		}
		let unknown = || Error::Unknown {
			spec: Box::new(spec.clone()),
		};
		let transport = self.volume_handle().map_err(|_| unknown())?.read_requests();
		let deadline = transport.deadline().map_err(|_| unknown())?;
		execute(
			spec,
			self.env().clock().as_ref(),
			deadline,
			|body, deadline| async move {
				self.request_in_region(
					body.destination_region,
					ClientRequest::FsForkRequest(FsForkRequest { spec: body }),
					deadline,
				)
				.await
			},
		)
		.await
	}
}

/// Sends the fork once, within the original call deadline. The edge owns retries of a user
/// request, so a refusal goes back with the retained body and its hint, and a lost reply goes
/// back as unknown with the same body; neither is resent here.
async fn execute<F, Fut>(
	spec: FsForkSpec,
	clock: &dyn rivet_env::Clock,
	deadline: u64,
	attempt: F,
) -> Result<FsVolumeRecord, Error>
where
	F: FnOnce(FsForkSpec, u64) -> Fut,
	Fut: std::future::Future<Output = anyhow::Result<ClientResponse>>,
{
	let unknown = || Error::Unknown {
		spec: Box::new(spec.clone()),
	};
	let refused = |reason: Refusal, retry_after_ms: u64| Error::Refused {
		spec: Box::new(spec.clone()),
		reason,
		retry_after_ms,
	};
	if clock.observe().map_err(|_| unknown())?.boot_ms >= deadline {
		return Err(unknown());
	}
	let response = attempt(spec.clone(), deadline).await;
	if clock.observe().map_err(|_| unknown())?.boot_ms >= deadline {
		return Err(unknown());
	}
	match response {
		Ok(ClientResponse::FsForkResponse(FsForkResponse::FsVolumeRecord(record)))
			if record.volume_key() == spec.destination_volume_key
				&& record.volume_incarnation == spec.operation_id =>
		{
			Ok(record)
		}
		Ok(ClientResponse::FsForkResponse(FsForkResponse::FsForkError(
			error @ (FsForkError::Busy
			| FsForkError::Lagging
			| FsForkError::BaseUnavailable
			| FsForkError::RegionUnavailable
			| FsForkError::TailUnproven),
		))) => Err(refused(Refusal::Fork(error), FORK_REFUSED_RETRY_AFTER_MS)),
		Ok(ClientResponse::FsForkResponse(FsForkResponse::FsForkError(error))) => {
			Err(Error::Rejected(error))
		}
		Ok(ClientResponse::VolumeRejectResponse(reject)) if permanent_reject(&reject) => {
			Err(Error::LifecycleRejected(reject))
		}
		// A refusal can answer an expired caller of a job that keeps running, so it never
		// establishes that durable work was rolled back; the retained body joins that job.
		Ok(ClientResponse::VolumeRejectResponse(reject)) => {
			let retry_after_ms =
				depot_types::client::retry_after_ms(&reject).unwrap_or(FORK_REFUSED_RETRY_AFTER_MS);
			Err(refused(Refusal::Lifecycle(reject), retry_after_ms))
		}
		Err(error) => match crate::volume_requests::unsent_refusal(&error) {
			Some(reject) => {
				let retry_after_ms = depot_types::client::retry_after_ms(&reject)
					.unwrap_or(FORK_REFUSED_RETRY_AFTER_MS);
				Err(refused(Refusal::Lifecycle(reject), retry_after_ms))
			}
			None => Err(unknown()),
		},
		Ok(_) => Err(unknown()),
	}
}

/// Whether a lifecycle answer to a fork holds for good. A missing, deleted or existing volume,
/// an archived group and a record no maintainer can rebuild do not change by waiting; a
/// maintainer move, a split, load and an operator repair claim pass.
fn permanent_reject(reject: &VolumeRejectResponse) -> bool {
	match reject {
		VolumeRejectResponse::VolumeNotFound
		| VolumeRejectResponse::VolumeExists
		| VolumeRejectResponse::VolumeDeleted
		| VolumeRejectResponse::VolumeRecordMissing(_)
		| VolumeRejectResponse::VolumeGroupArchived => true,
		VolumeRejectResponse::NotMaintainer(_)
		| VolumeRejectResponse::TabletSplitting
		| VolumeRejectResponse::Overloaded(_)
		| VolumeRejectResponse::VolumeUnderRepair(_)
		| VolumeRejectResponse::VolumeTailUnproven(_) => false,
	}
}

/// Encodes a prepared fork body for an application to persist before it sends the body, so it
/// can resume the same fork after a restart with [`decode_spec`] and [`Client::resume_fork`].
pub fn encode_spec(spec: &FsForkSpec) -> anyhow::Result<Vec<u8>> {
	rivet_data::versioned::DepotFsForkSpecData::serialize_with_embedded_version(
		spec,
		rivet_data::versioned::DepotFsForkSpecData::LATEST,
	)
}

/// Why a persisted fork body does not decode.
#[derive(Debug, thiserror::Error)]
pub enum DecodeSpecError {
	/// The body names the schema version one past this binary's latest, as a newer binary
	/// writes during a rolling upgrade. Assuming no rollback, it decodes once a binary that knows
	/// the version handles it, and an application may keep the fork waiting until then; after a
	/// rollback it waits until such a binary runs again.
	#[error("fork body version {version} is newer than this binary's latest, {latest}")]
	NewerVersion { version: u16, latest: u16 },
	/// No binary decodes the body: it is truncated, corrupt or names no valid version, including
	/// a version more than one past this binary's latest, which no rolling upgrade produces. An
	/// application fails the fork rather than wait on it.
	#[error("fork body does not decode")]
	Corrupt(#[source] anyhow::Error),
}

/// Decodes a fork body [`encode_spec`] wrote.
pub fn decode_spec(bytes: &[u8]) -> Result<FsForkSpec, DecodeSpecError> {
	let latest = rivet_data::versioned::DepotFsForkSpecData::LATEST;
	match rivet_data::versioned::DepotFsForkSpecData::deserialize_with_embedded_version(bytes) {
		Ok(spec) => Ok(spec),
		Err(error) => match embedded_version(bytes) {
			Some(version) if version == latest + 1 => {
				Err(DecodeSpecError::NewerVersion { version, latest })
			}
			Some(_) | None => Err(DecodeSpecError::Corrupt(error)),
		},
	}
}

/// The schema version in the two-byte little-endian header vbare puts before every versioned
/// body, or `None` when the body is too short to hold one.
fn embedded_version(bytes: &[u8]) -> Option<u16> {
	match bytes {
		[low, high, ..] => Some(u16::from_le_bytes([*low, *high])),
		[] | [_] => None,
	}
}
