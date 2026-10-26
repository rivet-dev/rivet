//! Checkpoint commands share the volume owner's caller fence and deadline.
//! A holder appends one log intent, then confirms the canonical name after apply.

use crate::{Client, volume_requests::VolumeRequests};
use depot_types::VolumeKey;
use rivet_data::generated::{depot_client_protocol_v1::*, shared_depot_fs_v1::*};

#[derive(Clone, Debug, thiserror::Error, PartialEq, Eq)]
pub enum Error {
	#[error("checkpoint rejected: {0:?}")]
	Rejected(CheckpointError),
	#[error("checkpoint routing rejected: {0:?}")]
	Lifecycle(VolumeRejectResponse),
	/// A read-only checkpoint command lost its reply. It changed nothing, so the edge may send
	/// it again after the hint.
	#[error("checkpoint read was interrupted; retry after {retry_after_ms} ms")]
	Interrupted { retry_after_ms: u64 },
	#[error("checkpoint result is unknown")]
	Unknown,
	#[error(transparent)]
	Filesystem(#[from] crate::filesystem::Error),
}

/// The hint of a checkpoint refused because its tablet is splitting, which carries none on
/// the wire. The split's children take over on the scale of a route refresh.
const TABLET_SPLITTING_RETRY_AFTER_MS: u64 = 100;

/// The hint of a confirmation refused because the maintainer has not applied the holder's
/// logged op yet, which carries none on the wire. The maintainer holds a confirmation until
/// its apply passes the op, so this answer comes only from a maintainer that has not
/// registered the volume for apply, which settles on the scale of a volume load.
const CHECKPOINT_LAGGING_RETRY_AFTER_MS: u64 = 100;

/// The hint of a read-only checkpoint command whose reply was lost, typically while the
/// tablet's maintainer moves, which settles on the scale of a route refresh.
const CHECKPOINT_INTERRUPTED_RETRY_AFTER_MS: u64 = 100;

impl Error {
	/// The hint of a refusal that ran nothing, which the edge waits before sending the
	/// checkpoint request again, or `None` for every other error.
	pub fn retry_after_ms(&self) -> Option<u64> {
		match self {
			Self::Lifecycle(VolumeRejectResponse::TabletSplitting) => {
				Some(TABLET_SPLITTING_RETRY_AFTER_MS)
			}
			Self::Lifecycle(reject) => depot_types::client::retry_after_ms(reject),
			Self::Rejected(CheckpointError::Lagging) => Some(CHECKPOINT_LAGGING_RETRY_AFTER_MS),
			Self::Interrupted { retry_after_ms } => Some(*retry_after_ms),
			Self::Rejected(_) | Self::Unknown | Self::Filesystem(_) => None,
		}
	}
}

/// Sends the checkpoint command once and returns its answer. The edge owns retries of a user
/// request, so a refusal goes back to the caller with its hint instead of being resent here:
/// a maintainer refusal, such as a moved grant, a splitting tablet or load, has already
/// triggered a route refresh, and a request that found no route was never sent. A confirmation
/// of a holder's logged op waits in the maintainer until the apply passes that op, so it needs
/// no resend either. A lost reply of a read-only command changed nothing and is refused as
/// interrupted; a lost reply of a change leaves its outcome unknown.
pub(crate) async fn request(
	transport: &VolumeRequests,
	clock: &dyn rivet_env::Clock,
	key: VolumeKey,
	operation: CheckpointOperation,
	deadline: u64,
) -> Result<VolumeCheckpointResponse, Error> {
	if clock.observe().map_err(|_| Error::Unknown)?.boot_ms >= deadline {
		return Err(Error::Unknown);
	}
	let read_only = matches!(
		operation,
		CheckpointOperation::CheckpointInspect(_) | CheckpointOperation::CheckpointList(_)
	);
	let response = transport
		.request_at(
			ClientRequest::VolumeCheckpointRequest(VolumeCheckpointRequest {
				volume_key: key,
				operation,
			}),
			deadline,
		)
		.await;
	match response {
		Ok(ClientResponse::VolumeCheckpointResponse(
			VolumeCheckpointResponse::CheckpointError(error),
		)) => Err(Error::Rejected(error)),
		Ok(ClientResponse::VolumeCheckpointResponse(response)) => Ok(response),
		Ok(ClientResponse::VolumeRejectResponse(reason)) => Err(Error::Lifecycle(reason)),
		Ok(_) => Err(Error::Unknown),
		Err(error) => {
			if let Some(reject) = crate::volume_requests::unsent_refusal(&error) {
				return Err(Error::Lifecycle(reject));
			}
			if read_only {
				return Err(Error::Interrupted {
					retry_after_ms: CHECKPOINT_INTERRUPTED_RETRY_AFTER_MS,
				});
			}
			Err(Error::Unknown)
		}
	}
}

/// A checkpoint change that the volume's holder logs as an administrative op.
#[derive(Clone, Debug)]
pub(crate) enum CheckpointUpdate {
	/// Clears the `named` flag of checkpoint `name` of `incarnation`.
	Release {
		incarnation: [u8; 16],
		name: Vec<u8>,
	},
	/// Sets the volume's retention policy; `None` restores the default.
	Retention(Option<FsRetentionPolicy>),
}

impl CheckpointUpdate {
	/// Whether the maintainer could act on this change at all.
	pub(crate) fn valid(&self) -> bool {
		match self {
			Self::Release { name, .. } => {
				!name.is_empty() && name.len() <= depot_fs_core::checkpoints::MAX_NAME_BYTES
			}
			Self::Retention(policy) => depot_fs_core::checkpoints::valid_policy(policy.as_ref()),
		}
	}

	/// The maintainer operation for this change, confirming the logged op at `minimum` when
	/// the holder logged it.
	pub(crate) fn operation(&self, minimum: Option<CheckpointPosition>) -> CheckpointOperation {
		match self {
			Self::Release { incarnation, name } => {
				CheckpointOperation::CheckpointDelete(CheckpointDelete {
					volume_incarnation: *incarnation,
					name: name.clone().into(),
					minimum,
				})
			}
			Self::Retention(retention) => {
				CheckpointOperation::CheckpointSetRetention(CheckpointSetRetention {
					retention: retention.clone(),
					minimum,
				})
			}
		}
	}
}

impl Client {
	pub async fn checkpoint_create(
		&self,
		key: VolumeKey,
		name: Vec<u8>,
	) -> anyhow::Result<FsCheckpointInfo> {
		Ok(self.volume_handle()?.checkpoint_create(key, name).await?)
	}

	/// Paginated across all retained incarnations. An empty page with a cursor
	/// means more metadata remains; it is not the end of the listing.
	pub async fn checkpoint_list(
		&self,
		key: VolumeKey,
		after: Option<Vec<u8>>,
		limit: u32,
	) -> anyhow::Result<CheckpointPage> {
		match self
			.checkpoint_command(
				key,
				CheckpointOperation::CheckpointList(CheckpointList {
					after: after.map(Into::into),
					limit,
				}),
			)
			.await?
		{
			VolumeCheckpointResponse::CheckpointPage(page) => Ok(page),
			_ => Err(Error::Unknown.into()),
		}
	}
	/// Removes only Named; retention and each fork hold remain independent. The holder of the
	/// volume logs the release; the call returns once it is applied.
	pub async fn checkpoint_delete(
		&self,
		key: VolumeKey,
		incarnation: [u8; 16],
		name: Vec<u8>,
	) -> anyhow::Result<()> {
		Ok(self
			.volume_handle()?
			.checkpoint_update(key, CheckpointUpdate::Release { incarnation, name })
			.await?)
	}

	/// The holder of the volume logs the policy; the call returns once it is applied.
	pub async fn set_retention(
		&self,
		key: VolumeKey,
		retention: Option<FsRetentionPolicy>,
	) -> anyhow::Result<()> {
		Ok(self
			.volume_handle()?
			.checkpoint_update(key, CheckpointUpdate::Retention(retention))
			.await?)
	}
	async fn checkpoint_command(
		&self,
		key: VolumeKey,
		op: CheckpointOperation,
	) -> anyhow::Result<VolumeCheckpointResponse> {
		let transport = self.volume_handle()?.read_requests();
		let clock = self.pubsub.env().clock();
		let deadline = transport.deadline()?;
		Ok(request(&transport, clock.as_ref(), key, op, deadline).await?)
	}
}
