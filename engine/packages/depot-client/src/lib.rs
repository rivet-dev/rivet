//! Depot volume command transport and typed partition operations.
//!
//! Volume commands route through a cached tablet manifest. Each KV partition uses one cached
//! maintainer-owned filesystem, accessed only through typed command RPCs.

#![warn(unreachable_pub)]

pub mod checkpoints;
mod command;
pub mod fences;
mod volume_observer;
pub use command::{
	CommandOptions, Conflict, Corrupt, Fenced, Overloaded, ROUTE_UNRESOLVED_REASON,
	ReadInterrupted, Unavailable, overload_reason_label, retry_after_ms, route_unresolved,
	unavailable_reason_label,
};
pub mod filesystem;
pub mod forks;
mod keys;
pub mod metrics;
pub mod regional;
pub mod volume_requests;
pub mod volumes;

use std::sync::Arc;

use anyhow::Result;
use depot_types::VolumeLogKey;
use depot_volume::volume_client_machine::event::RpcError;
use rivet_data::generated::shared_depot_fs_v1::FsVolumeRecord;
use rivet_topology::topology::Topology;
use universalpubsub::{PubSub, WorkerId};

/// Returned by partition operations while their storage backend is unavailable.
#[derive(Debug, rivet_error::RivetError)]
#[error(
	"depot",
	"storage_unavailable",
	"Depot storage is not available in this build."
)]
pub struct StorageUnavailable;

/// Revocation prevents new dispatch; it does not prove that an already-submitted mutation did not land.
#[derive(Debug, rivet_error::RivetError)]
#[error(
	"depot",
	"scope_revoked",
	"The caller scope ended; submitted work may still complete."
)]
pub struct ScopeRevoked;

impl ScopeRevoked {
	/// Whether `error` is a revoked scope. Retrying through the same scope cannot succeed.
	pub fn is(error: &anyhow::Error) -> bool {
		error
			.downcast_ref::<rivet_error::RivetError>()
			.is_some_and(|error| error.group() == "depot" && error.code() == "scope_revoked")
	}
}

/// Application Depot handle. The service owner installs volume access without starting work here.
#[derive(Clone)]
pub struct Client {
	regions: Option<Arc<regional::Regions>>,
	volumes: Option<volumes::VolumeClient>,
	pubsub: PubSub,
	topology: Arc<Topology>,
	dispatch_guard: Option<Arc<dyn Fn() -> bool + Send + Sync>>,
	command_timeout_floor: Option<std::time::Duration>,
}

impl Client {
	pub fn new(pubsub: PubSub, topology: Arc<Topology>) -> Self {
		Self {
			regions: None,
			volumes: None,
			pubsub,
			topology,
			dispatch_guard: None,
			command_timeout_floor: None,
		}
	}

	/// Raises every typed command deadline to at least `floor`. Commands whose own
	/// deadline is longer keep it, so this can only widen a retry budget.
	pub fn with_command_timeout_floor(&self, floor: std::time::Duration) -> Self {
		Self {
			command_timeout_floor: Some(floor),
			..self.clone()
		}
	}

	pub(crate) fn command_timeout(&self, default: std::time::Duration) -> std::time::Duration {
		match self.command_timeout_floor {
			Some(floor) => default.max(floor),
			None => default,
		}
	}

	/// Attaches the explicitly started service; this handle never owns or starts its background tasks.
	pub fn with_volumes(&self, volumes: volumes::VolumeClient) -> Result<Self> {
		anyhow::ensure!(
			volumes.worker_id() == self.worker_id(),
			"volume client and Depot must share worker identity"
		);
		Ok(Self {
			volumes: Some(volumes),
			..self.clone()
		})
	}

	/// Opens an exclusive filesystem owner. Callers retain it for their actor or command lifetime.
	pub async fn open_filesystem(
		&self,
		key: depot_types::VolumeKey,
	) -> Result<filesystem::OpenedFilesystem> {
		self.volume_handle()?.open_filesystem(key).await
	}

	/// Opens an exclusive filesystem owner, creating the volume inside the same open when it is
	/// missing.
	pub async fn open_or_create_filesystem(
		&self,
		key: depot_types::VolumeKey,
	) -> Result<filesystem::OpenedFilesystem> {
		self.volume_handle()?.open_or_create_filesystem(key).await
	}

	/// See [`volumes::VolumeClient::open_or_create_filesystem_prefetching`].
	pub async fn open_or_create_filesystem_prefetching(
		&self,
		key: depot_types::VolumeKey,
		prefetch: filesystem::transport::OpenPrefetch,
	) -> Result<filesystem::OpenedFilesystem> {
		self.volume_handle()?
			.open_or_create_filesystem_prefetching(key, prefetch)
			.await
	}

	/// See [`volumes::VolumeClient::open_or_create_filesystem_with_first_write`].
	pub async fn open_or_create_filesystem_with_first_write<T, F>(
		&self,
		key: depot_types::VolumeKey,
		write: F,
	) -> Result<volumes::FirstWriteOpen<T>>
	where
		F: FnOnce(
			Arc<filesystem::Filesystem>,
		) -> futures_util::future::BoxFuture<'static, anyhow::Result<T>>,
	{
		self.volume_handle()?
			.open_or_create_filesystem_with_first_write(key, write)
			.await
	}

	/// Opens or reads a non-exclusive snapshot. All calls in one application request
	/// retain the caller's original monotonic deadline, including route refreshes.
	pub async fn read_filesystem(
		&self,
		request: rivet_data::generated::shared_depot_fs_read_v1::FsReadRequest,
		deadline_boot_ms: u64,
	) -> Result<rivet_data::generated::shared_depot_fs_read_v1::FsReadResponse> {
		use rivet_data::generated::{depot_client_protocol_v1::*, shared_depot_fs_read_v1::*};
		let response = self
			.volume_handle()?
			.read_requests()
			.request_at(ClientRequest::FsReadRequest(request), deadline_boot_ms)
			.await?;
		Ok(match response {
			ClientResponse::FsReadResponse(response) => response,
			ClientResponse::VolumeRejectResponse(reason) => {
				FsReadResponse::FsReadError(match reason {
					VolumeRejectResponse::VolumeNotFound | VolumeRejectResponse::VolumeDeleted => {
						FsReadError::NotFound
					}
					// A repair claim refuses only lifecycle requests; a read refused for one
					// retries like shed load.
					VolumeRejectResponse::Overloaded(_)
					| VolumeRejectResponse::VolumeUnderRepair(_) => FsReadError::Overloaded,
					// The volume's stored record is lost, which no retry mends; the read error
					// carries no incarnation, and `delete_current` reads through the volume
					// handle, whose error does.
					VolumeRejectResponse::VolumeRecordMissing(_) => FsReadError::Corrupt,
					VolumeRejectResponse::NotMaintainer(_)
					| VolumeRejectResponse::TabletSplitting
					| VolumeRejectResponse::VolumeExists => FsReadError::Unavailable,
				})
			}
			_ => FsReadResponse::FsReadError(FsReadError::Corrupt),
		})
	}

	/// Sends one operator repair request to the maintainer of the volume it names, once,
	/// within `deadline_boot_ms`. The caller decides any retry; a stale route refreshes the
	/// cached manifest so the next attempt finds the current maintainer.
	pub async fn repair_request(
		&self,
		request: rivet_data::generated::depot_client_protocol_v1::ClientRequest,
		deadline_boot_ms: u64,
	) -> Result<rivet_data::generated::depot_client_protocol_v1::ClientResponse> {
		self.volume_handle()?
			.read_requests()
			.request_at(request, deadline_boot_ms)
			.await
	}

	/// The tablet the cached manifest routes `key` to. A miss asks for a manifest refresh and
	/// fails; the caller tries again.
	pub fn placement(
		&self,
		key: &depot_types::VolumeKey,
	) -> Result<rivet_data::generated::depot_tablet_manifest_v1::Tablet> {
		self.volume_handle()?.read_requests().tablet(key)
	}

	/// Creates an idle volume through its maintainer. The first read or write opens its client.
	pub async fn create(
		&self,
		key: depot_types::VolumeKey,
	) -> Result<FsVolumeRecord, volumes::Error> {
		self.volume_handle()?.create(key).await
	}

	pub async fn create_with_retention(
		&self,
		key: depot_types::VolumeKey,
		retention: Option<depot_fs_core::FsRetentionPolicy>,
	) -> Result<FsVolumeRecord, volumes::Error> {
		self.volume_handle()?
			.create_with_retention(key, retention)
			.await
	}

	/// Names the incarnation explicitly so delayed deletes cannot remove a replacement volume.
	pub async fn delete(&self, key: VolumeLogKey) -> Result<(), volumes::Error> {
		self.volume_handle()?.delete(key).await
	}

	/// Deletes whatever incarnation of `key` is current, and reports whether one existed.
	///
	/// Valid only for keys that are never recreated, such as a destroyed actor's volumes: the
	/// current incarnation is resolved by one maintainer point read that registers no session,
	/// then deleted by name, so a replacement created between the two steps would survive. A volume that is missing or
	/// already deleted reports `false`. Only a serving maintainer's answer counts as missing: a
	/// maintainer that refuses the read, such as the frozen parent of a splitting tablet, has
	/// refreshed the route, and the caller retries the refusal like any stale route, since the
	/// volume may exist in a log only its new maintainer applies.
	pub async fn delete_current(
		&self,
		key: depot_types::VolumeKey,
	) -> Result<bool, volumes::Error> {
		use filesystem::remote::ReadTransport;
		use rivet_data::generated::shared_depot_fs_read_v1::{
			FsReadError, FsReadKeys, FsReadOperation, FsReadRequest, FsReadResponse, FsReadTarget,
		};

		// A point read of no keys answers with the incarnation it read and holds nothing after.
		let handle = self.volume_handle()?;
		let resolved = handle
			.lifecycle
			.read(FsReadRequest {
				target: FsReadTarget::VolumeKey(key.clone()),
				client_epoch: None,
				operation: FsReadOperation::FsReadKeys(FsReadKeys {
					position: None,
					file: Vec::new().into(),
					keys: Vec::new(),
				}),
			})
			.await;
		let volume_log_key = match resolved {
			Ok(FsReadResponse::FsReadValues(values)) => values.volume_log_key,
			Ok(FsReadResponse::FsReadError(FsReadError::NotFound))
			| Err(filesystem::Error::Filesystem(depot_fs_core::Error::NotFound)) => {
				return Ok(false);
			}
			Ok(FsReadResponse::FsReadError(FsReadError::NotServing))
			| Err(filesystem::Error::ColdReadRefused) => {
				return Err(volumes::Error::Lifecycle(RpcError::LifecycleRejected(
					depot_types::client::not_maintainer(),
				)));
			}
			Ok(FsReadResponse::FsReadError(FsReadError::Overloaded))
			| Err(filesystem::Error::Overloaded) => return Err(volumes::Error::Overloaded),
			// The current incarnation has no stored record, so no read can resolve it; the
			// refusal names it, and deleting it is exactly the repair the name needs.
			Err(filesystem::Error::RecordMissing { incarnation }) => {
				rivet_data::generated::shared_depot_types_v1::VolumeLogKey {
					volume_key: key,
					volume_incarnation: incarnation,
				}
			}
			Ok(_) | Err(_) => {
				return Err(volumes::Error::Lifecycle(RpcError::Unavailable));
			}
		};

		match handle.delete(VolumeLogKey(volume_log_key)).await {
			Ok(()) => Ok(true),
			Err(error) if error.is_volume_gone() => Ok(false),
			Err(error) => Err(error),
		}
	}

	pub(crate) fn volume_handle(&self) -> Result<volumes::VolumeClient, volumes::Error> {
		let handle = self.volumes.clone().ok_or(volumes::Error::Closed)?;
		Ok(if let Some(guard) = &self.dispatch_guard {
			handle.with_dispatch_guard(guard.clone())
		} else {
			handle
		})
	}

	pub fn env(&self) -> &Arc<dyn rivet_env::Env> {
		self.pubsub.env()
	}

	pub fn worker_id(&self) -> &WorkerId {
		self.pubsub.worker_id()
	}

	pub fn topology(&self) -> &Arc<Topology> {
		&self.topology
	}

	/// Restricts this clone to a caller-owned lifetime. The guard is checked before every call.
	pub fn with_dispatch_guard(&self, guard: Arc<dyn Fn() -> bool + Send + Sync>) -> Self {
		let previous = self.dispatch_guard.clone();
		Self {
			dispatch_guard: Some(Arc::new(move || {
				previous.as_ref().is_none_or(|old| old()) && guard()
			})),
			..self.clone()
		}
	}
}


