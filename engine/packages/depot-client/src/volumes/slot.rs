//! One cache slot serializes lifecycle changes while allowing concurrent reads and writes.

use super::{Error, Response, owner::Dependencies};
use crate::{
	checkpoints::Error as CheckpointFailure,
	filesystem::{
		Filesystem, NoteExpiry, OpenedFilesystem,
		owner::{Completion, Lease},
		remote::RemoteColdStore,
		transport::VolumeJournal,
	},
	metrics,
};
use depot_types::VolumeLogKey;
use depot_volume::{
	ups::{
		UpsLogClient,
		client::{UpsVolumeTransport, VolumeLifecycleTransport},
	},
	volume_client_machine::{
		event::RpcError,
		open_refusal,
		runner::Volume,
		state::{VolumeClientPhase, VolumeClientState, VolumeIdentity},
		types::WriteError,
	},
};
use futures_util::{FutureExt, future::BoxFuture};
use rivet_data::generated::{
	depot_client_protocol_v1::{
		CheckpointError, ClientRequest, ClientResponse, VolumeCreateRequest, VolumeDeleteRequest,
		VolumeOpenResponse, VolumeRejectResponse,
	},
	shared_depot_types_v1::VolumeLogKey as WireVolumeLogKey,
};
use std::{
	future::Future,
	sync::{
		Arc,
		atomic::{AtomicU64, Ordering},
	},
	time::Duration,
};
use tokio::sync::RwLock;

/// One cache slot owns its created replicas, live runner or exact pending close.
enum State {
	/// No runner remains; the cache can remove this slot when no request references it.
	Empty,

	/// Creation loaded replicas, but no read or write has started the ordinary client yet.
	Created {
		key: VolumeLogKey,
		client_incarnation: [u8; 16],
		opened: VolumeOpenResponse,
	},

	/// Exactly one runner owns this logical key. Concurrent calls share it under a read guard.
	Client {
		key: depot_types::VolumeKey,
		volume: Box<Volume>,
		filesystem: Option<FilesystemSlot>,
	},

	/// The runner is joined; a failed lifecycle close retains the exact incarnation for retry.
	Closing {
		key: VolumeLogKey,
		client: Option<rivet_data::generated::shared_depot_types_v1::ClientIdentity>,
	},
}

struct FilesystemSlot {
	fs: Arc<Filesystem>,
	lease: std::sync::Weak<Lease>,
	completion: Arc<Completion>,
}
impl FilesystemSlot {
	fn active(&self) -> bool {
		self.lease.upgrade().is_some_and(|lease| lease.active())
	}
}
impl Drop for FilesystemSlot {
	fn drop(&mut self) {
		self.fs
			.fence_with(crate::fences::FenceReason::OwnerReleased);
		self.completion.finish();
	}
}

/// Runs a held filesystem's note timer. A held filesystem is never closed; it only drops the
/// notes of its writes once their timer passes, and a filesystem with no note waits for the
/// signal its next write sends.
fn idle_notes(now: u64, fs: &Arc<Filesystem>) -> IdleCheck {
	match fs.expire_notes(now) {
		NoteExpiry::Idle => IdleCheck::Held,
		NoteExpiry::Due(at) => IdleCheck::NotYetDue { at },
		NoteExpiry::Busy => IdleCheck::Busy,
	}
}

/// The outcome of one idle check, which decides the key's next place in the idle queue.
pub(super) enum IdleCheck {
	/// A caller holds the filesystem lease. Its release signals the key again.
	Held,

	/// The slot was used since it was scheduled; check again at `at`.
	NotYetDue { at: u64 },

	/// A caller holds the slot lock; retry after the sweep interval.
	Busy,

	/// No runner remains. The owner removes the slot when no request references it; a
	/// referencing request signals the key again when it finishes.
	Empty,

	/// The slot is idle or terminal; this future closes it under the exclusive lock.
	Release(BoxFuture<'static, ()>),
}

pub(super) struct Slot {
	state: Arc<RwLock<State>>,
	last_used_boot_ms: AtomicU64,
	_metric: rivet_metrics::IntGaugeGuard,
}

impl Slot {
	pub(crate) async fn fork_minimum(
		&self,
		deps: &Dependencies,
		may_dispatch: Arc<dyn Fn() -> bool + Send + Sync>,
	) -> Result<Response, Error> {
		if !may_dispatch() {
			return Err(Error::Closed);
		}
		self.touch(deps)?;
		let fs = {
			let state = self.state.read().await;
			let State::Client {
				filesystem: Some(fs),
				..
			} = &*state
			else {
				return Ok(Response::ForkMinimum(None));
			};
			if !fs.active() {
				return Err(Error::Closed);
			}
			fs.fs.clone()
		};
		let minimum = fs.confirmed_minimum().await.map_err(|_| Error::Closed)?;
		if !may_dispatch() {
			return Err(Error::Closed);
		}
		Ok(Response::ForkMinimum(Some(minimum)))
	}

	pub(crate) async fn checkpoint(
		&self,
		deps: &Dependencies,
		key: depot_types::VolumeKey,
		name: Vec<u8>,
		may_dispatch: Arc<dyn Fn() -> bool + Send + Sync>,
	) -> Result<Response, Error> {
		use crate::checkpoints::request;
		use rivet_data::generated::depot_client_protocol_v1::*;
		if !may_dispatch() {
			return Err(Error::Closed);
		}
		self.touch(deps)?;
		let transport = deps.lifecycle.clone().with_dispatch_guard(may_dispatch);
		let clock = deps.env.clock();
		let deadline = rivet_env::deadline_after(clock.as_ref(), deps.timeout)
			.map_err(|_| Error::Checkpoint(CheckpointFailure::Unknown))?;
		let fs = {
			let state = self.state.read().await;
			match &*state {
				State::Client {
					filesystem: Some(fs),
					..
				} => {
					if !fs.active() {
						return Err(Error::Closed);
					}
					Some(fs.fs.clone())
				}
				_ => None,
			}
		};
		let minimum = if let Some(fs) = fs {
			let inspection = request(
				&transport,
				clock.as_ref(),
				key.clone(),
				CheckpointOperation::CheckpointInspect(CheckpointInspect {
					name: name.clone().into(),
				}),
				deadline,
			)
			.await
			.map_err(Error::Checkpoint)?;
			let VolumeCheckpointResponse::CheckpointInspection(inspection) = inspection else {
				return Err(Error::Checkpoint(CheckpointFailure::Unknown));
			};
			if fs.volume_key().0.volume_incarnation != inspection.volume_incarnation {
				return Err(Error::Checkpoint(CheckpointFailure::Rejected(
					CheckpointError::IncarnationChanged,
				)));
			}
			if let Some(info) = inspection.checkpoint {
				return if info.checkpoint.named {
					Ok(Response::Checkpoint(info))
				} else {
					Err(Error::Checkpoint(CheckpointFailure::Rejected(
						CheckpointError::NameInUse,
					)))
				};
			}
			if inspection.named_count >= inspection.max_named {
				return Err(Error::Checkpoint(CheckpointFailure::Rejected(
					CheckpointError::TooManyNamed,
				)));
			}
			Some(
				fs.checkpoint_entry(name.clone())
					.await
					.map_err(|e| Error::Checkpoint(e.into()))?,
			)
		} else {
			None
		};
		let logged = minimum.is_some();
		let response = request(
			&transport,
			clock.as_ref(),
			key,
			CheckpointOperation::CheckpointCreate(CheckpointCreate {
				name: name.into(),
				minimum,
			}),
			deadline,
		)
		.await
		.map_err(|error| confirmation_error(logged, error))?;
		match response {
			VolumeCheckpointResponse::FsCheckpointInfo(info) => Ok(Response::Checkpoint(info)),
			_ => Err(Error::Checkpoint(CheckpointFailure::Unknown)),
		}
	}

	/// Logs a checkpoint release or retention change when this slot holds the volume, then
	/// asks the maintainer to confirm it after apply. Without a holder here the maintainer
	/// makes the change itself.
	pub(crate) async fn checkpoint_update(
		&self,
		deps: &Dependencies,
		key: depot_types::VolumeKey,
		update: crate::checkpoints::CheckpointUpdate,
		may_dispatch: Arc<dyn Fn() -> bool + Send + Sync>,
	) -> Result<Response, Error> {
		use crate::checkpoints::{CheckpointUpdate, request};
		use rivet_data::generated::depot_client_protocol_v1::*;
		if !may_dispatch() {
			return Err(Error::Closed);
		}
		self.touch(deps)?;
		let transport = deps.lifecycle.clone().with_dispatch_guard(may_dispatch);
		let clock = deps.env.clock();
		let deadline = rivet_env::deadline_after(clock.as_ref(), deps.timeout)
			.map_err(|_| Error::Checkpoint(CheckpointFailure::Unknown))?;
		let fs = {
			let state = self.state.read().await;
			match &*state {
				State::Client {
					filesystem: Some(fs),
					..
				} => {
					if !fs.active() {
						return Err(Error::Closed);
					}
					Some(fs.fs.clone())
				}
				_ => None,
			}
		};
		let minimum = match (&fs, &update) {
			(Some(fs), CheckpointUpdate::Release { incarnation, name }) => Some(
				fs.checkpoint_release_entry(*incarnation, name.clone())
					.await
					.map_err(|e| Error::Checkpoint(e.into()))?,
			),
			(Some(fs), CheckpointUpdate::Retention(policy)) => Some(
				fs.retention_entry(policy.clone())
					.await
					.map_err(|e| Error::Checkpoint(e.into()))?,
			),
			(None, _) => None,
		};
		let logged = minimum.is_some();
		let response = request(
			&transport,
			clock.as_ref(),
			key,
			update.operation(minimum),
			deadline,
		)
		.await
		.map_err(|error| confirmation_error(logged, error))?;
		match response {
			VolumeCheckpointResponse::CheckpointUpdated => Ok(Response::CheckpointUpdated),
			_ => Err(Error::Checkpoint(CheckpointFailure::Unknown)),
		}
	}

	pub(crate) fn new() -> Self {
		Self {
			state: Arc::new(RwLock::new(State::Empty)),
			last_used_boot_ms: AtomicU64::new(0),
			// No count matches this, so the first check starts the quiet window instead of
			// releasing caches a filesystem just filled while opening.
			_metric: rivet_metrics::GaugeGuardExt::inc_guard(&*metrics::VOLUME_SLOTS),
		}
	}

	pub(super) fn held(
		&self,
	) -> Option<(
		VolumeLogKey,
		Option<depot_volume::volume_client_machine::runner::VolumeHandle>,
	)> {
		let state = self.state.try_read().ok()?;
		match &*state {
			// A create that claimed nothing holds nothing, so it reports nothing held: counting
			// it as a holder would only hide another holder's absence from the maintainer.
			State::Created { key, opened, .. } if opened.open_claim.is_some() => {
				Some((key.clone(), None))
			}
			State::Client { volume, .. } => {
				let handle = volume.handle();
				let key = handle
					.snapshot
					.borrow()
					.volume_log_key
					.as_deref()
					.cloned()?;
				Some((key, Some(handle)))
			}
			_ => None,
		}
	}

	pub(crate) fn is_empty(&self) -> bool {
		self.state
			.try_read()
			.is_ok_and(|state| matches!(*state, State::Empty))
	}

	fn touch(&self, deps: &Dependencies) -> Result<(), Error> {
		let now = deps
			.env
			.clock()
			.observe()
			.map_err(|_| Error::Lifecycle(RpcError::Unavailable))?;
		self.last_used_boot_ms.store(now.boot_ms, Ordering::Release);
		Ok(())
	}

	pub(crate) async fn create(
		&self,
		deps: &Dependencies,
		key: depot_types::VolumeKey,
		retention: Option<depot_fs_core::FsRetentionPolicy>,
		may_dispatch: Arc<dyn Fn() -> bool + Send + Sync>,
	) -> Result<Response, Error> {
		let mut state = self.state.write().await;
		self.create_locked(deps, &mut state, key, retention, may_dispatch)
			.await
	}

	/// Serializes create with local lifecycle work; an unknown outcome expires on the maintainer.
	async fn create_locked(
		&self,
		deps: &Dependencies,
		state: &mut State,
		key: depot_types::VolumeKey,
		retention: Option<depot_fs_core::FsRetentionPolicy>,
		may_dispatch: Arc<dyn Fn() -> bool + Send + Sync>,
	) -> Result<Response, Error> {
		if !may_dispatch() {
			return Err(Error::Closed);
		}
		self.touch(deps)?;
		// A stopped cached client still owns its final CloseVolume. Retire that
		// lifetime before create can register this worker as the fresh opener.
		let retiring = matches!(&*state, State::Client { volume, .. }
			if volume.handle().snapshot.borrow().phase == VolumeClientPhase::Stopped);
		// The create did not run, so its caller may send it again once the close settles; the
		// refusal names what kept the close from settling.
		if retiring || matches!(*state, State::Closing { .. }) {
			cleanup_client(deps, state).await?;
		}
		// Repeating create while this slot holds the volume is the same logical open.
		// A new identity would discard its claim and make idle close unable to release it.
		let client_incarnation = match &*state {
			State::Created {
				key: held,
				client_incarnation,
				..
			} if held.0.volume_key == key => *client_incarnation,
			State::Client {
				key: held, volume, ..
			} if held == &key => volume.handle().snapshot.borrow().client.client_incarnation,
			_ => *deps.env.rng("depot.volume.create").uuid_v4().as_bytes(),
		};
		let result = deps
			.lifecycle
			.clone()
			.with_dispatch_guard(may_dispatch)
			.lifecycle_request(ClientRequest::VolumeCreateRequest(VolumeCreateRequest {
				client_incarnation,
				retention,
				volume_key: key.clone(),
				holder: deps.lifecycle.holder(),
			}))
			.await;
		match result {
			Ok(ClientResponse::VolumeCreateResponse(response)) => {
				if response.record.volume_key() != key {
					return Err(Error::Lifecycle(RpcError::Unavailable));
				}
				let created_key: VolumeLogKey = WireVolumeLogKey {
					volume_key: key,
					volume_incarnation: response.record.volume_incarnation,
				}
				.into();
				let same_client = matches!(&*state, State::Client { volume, .. } if volume.handle().snapshot.borrow().volume_log_key.as_deref() == Some(&created_key));
				if !same_client {
					// A recreated incarnation cannot inherit the previous client or lose its newly created replicas.
					let old = std::mem::replace(
						&mut *state,
						State::Created {
							key: created_key,
							client_incarnation,
							opened: VolumeOpenResponse {
								record: response.record.clone(),
								open_claim: response.open_claim,
								states: response.states,
								created: None,
								read: None,
								// A created volume has nothing applied above its cold.
								applied: None,
							},
						},
					);
					if let State::Client {
						volume, filesystem, ..
					} = old
					{
						if let Some(fs) = &filesystem {
							fs.fs.fence_with(crate::fences::FenceReason::SlotReplaced);
						}
						volume
							.shutdown()
							.await
							.map_err(|_| Error::Lifecycle(RpcError::Unavailable))?;
						drop(filesystem);
					}
				}
				self.touch(deps)?;
				Ok(Response::Created(response.record))
			}
			Err(error) => Err(Error::Lifecycle(error)),
			Ok(
				ClientResponse::FsForkResponse(_)
				| ClientResponse::FsForkReferenceResponse(_)
				| ClientResponse::VolumeCheckpointResponse(_)
				| ClientResponse::FsReadResponse(_)
				| ClientResponse::VolumeDeleteResponse
				| ClientResponse::VolumeOpenResponse(_)
				| ClientResponse::ReplicaStatusNoticeResponse(_)
				| ClientResponse::OperatorTabletReport(_)
				| ClientResponse::OperatorResponse(_)
				| ClientResponse::RepairInspection(_)
				| ClientResponse::RepairResponse(_)
				| ClientResponse::RepairDiagnosticsPage(_)
				| ClientResponse::RepairAuditPage(_)
				| ClientResponse::VolumeCloseResponse
				| ClientResponse::VolumeRejectResponse(_),
			) => Err(Error::Lifecycle(RpcError::Unavailable)),
		}
	}

	/// Opens the volume as its exclusive filesystem owner. With `create_if_absent`, a first open
	/// lets the maintainer create a missing volume inside the open.
	pub(crate) async fn open_filesystem(
		&self,
		deps: &Arc<Dependencies>,
		key: depot_types::VolumeKey,
		create_if_absent: bool,
		initial: Vec<rivet_data::generated::depot_log_shard_v1::VolumeLogEntryBody>,
		prefetch: crate::filesystem::transport::OpenPrefetch,
		guard: Arc<dyn Fn() -> bool + Send + Sync>,
	) -> Result<Response, Error> {
		let mut timings = OpenTimings::start();
		let result = self
			.open_filesystem_timed(
				deps,
				key,
				create_if_absent,
				initial,
				prefetch,
				guard,
				&mut timings,
			)
			.await;
		timings.warn_if_slow(&result);
		result
	}

	#[allow(
		clippy::too_many_arguments,
		reason = "one open's parameters and its timings"
	)]
	async fn open_filesystem_timed(
		&self,
		deps: &Arc<Dependencies>,
		key: depot_types::VolumeKey,
		create_if_absent: bool,
		initial: Vec<rivet_data::generated::depot_log_shard_v1::VolumeLogEntryBody>,
		prefetch: crate::filesystem::transport::OpenPrefetch,
		guard: Arc<dyn Fn() -> bool + Send + Sync>,
		timings: &mut OpenTimings,
	) -> Result<Response, Error> {
		let mut state = self.state.write().await;
		timings.locked();
		let mut initial = initial;
		// A slot that created the volume opens from the claim its create returned. Another
		// worker may have opened the volume since and fenced that claim; this open is a new
		// logical open, so it opens afresh once instead of failing with the stale claim.
		let mut created_claim_fenced = false;
		let mut next_key = Some(key);
		loop {
			let guard = guard.clone();
			let Some(key) = next_key.take() else {
				return Err(Error::InvalidRequest);
			};
			if !guard() {
				return Err(Error::Closed);
			}
			if matches!(&*state,State::Client{filesystem:Some(fs),..} if fs.active()) {
				return Err(Error::InUse);
			}
			if matches!(*state, State::Client { .. } | State::Closing { .. }) {
				let cleanup_started = OpenTimings::now();
				let settled = cleanup_client(deps, &mut state).await;
				timings.cleanup += cleanup_started.elapsed();
				// The open did not run, so its caller may send it again once the close
				// settles; the refusal names what kept the close from settling.
				settled?;
			}
			self.touch(deps)?;
			let (identity, opened) = match &*state {
				State::Created {
					key,
					client_incarnation,
					opened,
				} => (
					key.clone().into(),
					Some((*client_incarnation, opened.clone())),
				),
				State::Empty => (
					VolumeIdentity::Unresolved {
						key: key.clone(),
						create_if_absent,
					},
					None,
				),
				_ => unreachable!("lifecycle was resolved under the exclusive lock"),
			};
			// Only an open that may create a volume it does not know yet can carry its first
			// writes; a volume this slot already created opens as usual.
			let initial = match (&identity, create_if_absent) {
				(VolumeIdentity::Unresolved { .. }, true) => std::mem::take(&mut initial),
				(VolumeIdentity::Unresolved { .. }, false) | (VolumeIdentity::Known(_), _) => {
					Vec::new()
				}
			};
			let from_created_claim = opened.is_some();
			let retry_key = (from_created_claim && !created_claim_fenced).then(|| key.clone());
			let initial_count = initial.len() as u64;
			let volume = start_client(deps, &key, identity, opened, guard.clone(), initial)?;
			let handle = volume.handle();
			let signal_key = key.clone();
			*state = State::Client {
				key,
				volume: Box::new(volume),
				filesystem: None,
			};
			let result = async {
				let ready_started = OpenTimings::now();
				let ready = handle.ready().await;
				timings.ready += ready_started.elapsed();
				ready.map_err(Error::Write)?;
				if !guard() {
					return Err(Error::Closed);
				}
				// The creation covers exactly the initial writes it carried. Writes over the
				// creating open's budget are not sent, and the volume is created empty.
				let created_with_initial = initial_count > 0
					&& handle
						.created()
						.is_some_and(|created| created.volume_seq == initial_count);
				let journal = Arc::new(VolumeJournal::new(
					handle.clone(),
					guard.clone(),
					deps.versions.clone(),
				));
				let cold = Arc::new(RemoteColdStore::new(
					Arc::new(deps.lifecycle.clone().with_dispatch_guard(guard.clone())),
					journal.clone(),
					deps.env.clock(),
				));
				// A volume never written opens without reading cold rows or the log, and its
				// owner stages its files into the first commit.
				let fs = Filesystem::open_staging_empty(
					journal.clone(),
					cold,
					deps.env.clock(),
					deps.config.filesystem.clone(),
					deps.config.process_cache.clone(),
					prefetch.clone(),
				)
				.await
				.map_err(|error| {
					tracing::warn!(?error, "filesystem opening failed");
					// The volume's creation already carried this open's first writes, so a
					// refused read after it cannot report the open as not run.
					if created_with_initial && error == crate::filesystem::Error::ColdReadRefused {
						return Error::Filesystem(crate::filesystem::Error::Unavailable);
					}
					Error::Filesystem(error)
				})?;
				let completion = Completion::new();
				let release_signal = {
					let idle = deps.idle.clone();
					let key = signal_key.clone();
					Arc::new(move || idle.signal(&key)) as Arc<dyn Fn() + Send + Sync>
				};
				fs.on_notes_armed(release_signal.clone());
				// Schedule the note timer now for the notes the open's replay left, as a write's
				// first note does later.
				release_signal();
				let opened = OpenedFilesystem::new(fs.clone(), completion.clone())
					.on_release(release_signal);
				if let State::Client { filesystem, .. } = &mut *state {
					*filesystem = Some(FilesystemSlot {
						fs,
						lease: opened.lease(),
						completion,
					});
				}
				Ok(Response::Opened(opened, created_with_initial))
			}
			.await;
			if result.is_err() {
				let cleanup_started = OpenTimings::now();
				// The caller gets the open's own error; an unsettled close leaves the slot
				// `Closing` for the next caller to settle.
				if let Err(error) = cleanup_client(deps, &mut state).await {
					tracing::debug!(?error, "the failed open's client close did not settle");
				}
				timings.cleanup += cleanup_started.elapsed();
			}
			// The kept create claim may be stale in any way a fresh open is not: a newer claim
			// fences it, and an unload since the create leaves it no loaded quorum. Either way
			// this logical open asks the maintainer for a current claim once.
			let stale_claim = from_created_claim
				&& !created_claim_fenced
				&& matches!(
					result,
					Err(Error::Write(
						WriteError::Fenced { .. } | WriteError::QuorumUnavailable { .. }
					))
				);
			if !stale_claim {
				return result;
			}
			tracing::debug!("a created volume's kept claim is no longer current; opening afresh");
			created_claim_fenced = true;
			next_key = retry_key;
		}
	}

	pub(crate) async fn delete(
		&self,
		deps: &Dependencies,
		key: VolumeLogKey,
		may_dispatch: Arc<dyn Fn() -> bool + Send + Sync>,
	) -> Result<Response, Error> {
		let mut state = self.state.write().await;
		let response = deps
			.lifecycle
			.clone()
			.with_dispatch_guard(may_dispatch)
			.lifecycle_request(ClientRequest::VolumeDeleteRequest(VolumeDeleteRequest {
				volume_log_key: key.clone().into(),
			}))
			.await
			.map_err(Error::Lifecycle)?;
		match response {
			ClientResponse::VolumeDeleteResponse => {
				// Only stop this incarnation; an old delete must not evict a newer local client.
				let matches = match &*state {
					State::Created { key: current, .. } | State::Closing { key: current, .. } => {
						current == &key
					}
					State::Client { volume, .. } => {
						volume.handle().snapshot.borrow().volume_log_key.as_deref() == Some(&key)
					}
					State::Empty => false,
				};
				if matches
					&& let State::Client {
						volume, filesystem, ..
					} = std::mem::replace(&mut *state, State::Empty)
				{
					if let Some(fs) = &filesystem {
						fs.fs.fence_with(crate::fences::FenceReason::VolumeDeleted);
					}
					volume
						.shutdown()
						.await
						.map_err(|_| Error::Lifecycle(RpcError::Unavailable))?;
					drop(filesystem);
				}
				Ok(Response::Deleted)
			}
			ClientResponse::FsForkResponse(_)
			| ClientResponse::FsForkReferenceResponse(_)
			| ClientResponse::VolumeCheckpointResponse(_)
			| ClientResponse::FsReadResponse(_)
			| ClientResponse::VolumeCreateResponse(_)
			| ClientResponse::VolumeOpenResponse(_)
			| ClientResponse::ReplicaStatusNoticeResponse(_)
			| ClientResponse::OperatorTabletReport(_)
			| ClientResponse::OperatorResponse(_)
			| ClientResponse::RepairInspection(_)
			| ClientResponse::RepairResponse(_)
			| ClientResponse::RepairDiagnosticsPage(_)
			| ClientResponse::RepairAuditPage(_)
			| ClientResponse::VolumeCloseResponse
			| ClientResponse::VolumeRejectResponse(_) => Err(Error::Lifecycle(RpcError::Unavailable)),
		}
	}

	/// Revalidates one due idle check. The exclusive lock prevents cleanup from racing a caller
	/// or creating duplicate cleanup work.
	pub(crate) fn idle_check(self: &Arc<Self>, deps: &Arc<Dependencies>, now: u64) -> IdleCheck {
		let Ok(mut state) = self.state.clone().try_write_owned() else {
			return IdleCheck::Busy;
		};
		if let State::Client {
			filesystem: Some(fs),
			..
		} = &*state
			&& fs.active()
		{
			return idle_notes(now, &fs.fs);
		}
		if matches!(*state, State::Empty) {
			return IdleCheck::Empty;
		}
		let terminal = matches!(&*state,State::Client{filesystem:Some(fs),..} if !fs.active())
			|| matches!(&*state, State::Client { volume, .. } if matches!(volume.handle().snapshot.borrow().phase, VolumeClientPhase::Stopped | VolumeClientPhase::Fenced { .. }));
		let idle_at = self
			.last_used_boot_ms
			.load(Ordering::Acquire)
			.saturating_add(deps.config.close_after_idle_ms);
		if !terminal && now < idle_at {
			return IdleCheck::NotYetDue { at: idle_at };
		}
		let deps = deps.clone();
		IdleCheck::Release(
			async move {
				// An unsettled close leaves the slot `Closing`; the next idle pass or caller
				// settles it.
				if let Err(error) = cleanup_client(&deps, &mut state).await {
					tracing::debug!(?error, "the idle volume's client close did not settle");
				}
			}
			.boxed(),
		)
	}

	pub(crate) async fn shutdown(&self, deps: &Dependencies) {
		let mut state = self.state.write().await;
		loop {
			if cleanup_client(deps, &mut state).await.is_ok() {
				return;
			}
			if !(deps.may_dispatch)() {
				// The maintainer will unload idle replicas; no local runner remains.
				tracing::warn!("volume release unresolved after worker authority expired");
				return;
			}
			if rivet_env::sleep(
				deps.env.clock().as_ref(),
				Duration::from_millis(deps.config.sweep_interval_ms),
			)
			.await
			.is_err()
			{
				return;
			}
		}
	}
}

/// The error of a checkpoint command whose op the holder may already have logged. A logged op
/// is durable and applies whatever its confirmation answers, so once `logged`, a refusal, a
/// lagging apply or a lost reply leaves the change applied or about to apply. It is reported
/// as unknown, which the edge never sends again as a new change; only a final rejection of
/// the op is reported as is.
fn confirmation_error(logged: bool, error: CheckpointFailure) -> Error {
	if !logged {
		return Error::Checkpoint(error);
	}
	match error {
		CheckpointFailure::Rejected(CheckpointError::Lagging)
		| CheckpointFailure::Lifecycle(_)
		| CheckpointFailure::Interrupted { .. }
		| CheckpointFailure::Unknown => Error::Checkpoint(CheckpointFailure::Unknown),
		error @ (CheckpointFailure::Rejected(_) | CheckpointFailure::Filesystem(_)) => {
			Error::Checkpoint(error)
		}
	}
}

/// Wall time of one filesystem open's waits, for the slow-open warning. A wake's first storage
/// request waits for this open, and the client gives up after 30 s.
struct OpenTimings {
	started: std::time::Instant,
	/// Waiting for the slot's exclusive lock, held by a concurrent open or an idle release.
	lock_wait: std::time::Duration,
	/// Closing the previous client and settling its claim before this open.
	cleanup: std::time::Duration,
	/// Waiting for the new client's quorum open.
	ready: std::time::Duration,
}

impl OpenTimings {
	/// An open at least this slow logs its waits.
	const SLOW: std::time::Duration = std::time::Duration::from_secs(5);

	fn now() -> std::time::Instant {
		#[allow(
			clippy::disallowed_methods,
			reason = "slow-open logging uses real time"
		)]
		std::time::Instant::now()
	}

	fn start() -> Self {
		Self {
			started: Self::now(),
			lock_wait: std::time::Duration::ZERO,
			cleanup: std::time::Duration::ZERO,
			ready: std::time::Duration::ZERO,
		}
	}

	fn locked(&mut self) {
		self.lock_wait = self.started.elapsed();
	}

	fn warn_if_slow(&self, result: &Result<Response, Error>) {
		let total = self.started.elapsed();
		if total < Self::SLOW {
			return;
		}
		tracing::warn!(
			total_ms = total.as_millis() as u64,
			lock_wait_ms = self.lock_wait.as_millis() as u64,
			cleanup_ms = self.cleanup.as_millis() as u64,
			ready_ms = self.ready.as_millis() as u64,
			ok = result.is_ok(),
			error = result.as_ref().err().map(|error| format!("{error:?}")),
			"slow volume filesystem open; the largest wait names the stalled step"
		);
	}
}

/// Closes the exact claimed identity after joining the client. A close that does not settle
/// stays owned by the slot, which remains `Closing` for the next caller to settle, and the
/// error is the refusal an open or create waiting on this cleanup returns to its caller; see
/// `unsettled_close_refusal`. A release that nobody waits on only logs it.
async fn cleanup_client(deps: &Dependencies, state: &mut State) -> Result<(), Error> {
	if matches!(state, State::Client { .. }) {
		let State::Client {
			key: _,
			volume,
			filesystem,
		} = std::mem::replace(state, State::Empty)
		else {
			unreachable!("client matched")
		};
		if let Some(fs) = &filesystem {
			fs.fs.fence_with(crate::fences::FenceReason::SlotClosed);
		}
		let handle = volume.handle();
		let close = rivet_env::timeout(
			deps.env.clock().as_ref(),
			deps.timeout + Duration::from_millis(deps.config.client.stall_timeout_ms),
			handle.close(),
		)
		.await;
		if close.is_err() {
			tracing::warn!(
				"volume client close timed out during release; a pending write or its quorum open did not finish"
			);
		}
		// Copy both fields under one read guard and drop it before building the state. The runner
		// is still live and may queue a write; a second read behind that writer would deadlock.
		let (volume_log_key, client, acknowledged) = {
			let snapshot = handle.snapshot.borrow();
			(
				snapshot.volume_log_key.as_deref().cloned(),
				snapshot.client.clone(),
				snapshot.close_acknowledged,
			)
		};
		*state = after_client_close(volume_log_key, client, acknowledged);
		if let Err(error) = volume.shutdown().await {
			tracing::error!(?error, "volume runner failed during release");
		}
		drop(filesystem);
	}
	let (key, client) = match state {
		State::Empty => return Ok(()),
		State::Created {
			key,
			opened,
			client_incarnation,
		} => (
			key.clone(),
			opened
				.states
				.iter()
				.filter_map(|m| m.state.claimed_client.as_ref())
				.filter(|c| c.client_incarnation == *client_incarnation)
				.max_by_key(|c| c.client_epoch)
				.cloned(),
		),
		State::Closing { key, client } => (key.clone(), client.clone()),
		State::Client { .. } => {
			unreachable!("client cleanup completed")
		}
	};
	*state = State::Closing {
		key: key.clone(),
		client: client.clone(),
	};
	let clock = deps.env.clock();
	// A failed clock sent nothing, so the waiting open is refused as not executed.
	let Ok(deadline_boot_ms) = rivet_env::deadline_after(clock.as_ref(), deps.timeout) else {
		return Err(Error::Overloaded);
	};
	let settled = close_until_settled(clock.as_ref(), deadline_boot_ms, || {
		deps.lifecycle.close(key.clone(), client.clone())
	})
	.await;
	match settled {
		Ok(()) => {
			*state = State::Empty;
			Ok(())
		}
		Err(Unsettled::Answer(refusal)) => Err(unsettled_close_refusal(refusal)),
		Err(Unsettled::ClockFailed) => Err(Error::Overloaded),
	}
}

/// The error an open or create gets when the slot's own close of its previous client did not
/// settle. The open ran nothing, so its caller may send it again after the hint, and the
/// reason is the close's: what kept the close from settling keeps the open from running. It
/// is classified exactly as a fresh open's refusal, so the edge waits out a maintainer that
/// moved, is splitting, has no route or did not answer, and sheds only a refusal under load.
/// A definite rejection the slot cannot wait out fails the open with that answer.
fn unsettled_close_refusal(error: RpcError) -> Error {
	match open_refusal(&error) {
		Some((reason, retry_after_ms)) => Error::Write(WriteError::Refused {
			reason,
			retry_after_ms: retry_after_ms.max(1),
		}),
		None => Error::Lifecycle(error),
	}
}

/// The slot state once a client's own close finished. A close the maintainer acknowledged is
/// complete, so the slot sends no second one. Otherwise, as after a fence or an unreachable
/// maintainer, the slot keeps the exact claimed identity and closes it itself.
fn after_client_close(
	volume_log_key: Option<VolumeLogKey>,
	client: rivet_data::generated::shared_depot_types_v1::ClientIdentity,
	acknowledged: bool,
) -> State {
	match volume_log_key {
		Some(_) if acknowledged => State::Empty,
		Some(key) => State::Closing {
			key,
			client: Some(client),
		},
		None => State::Empty,
	}
}

/// The wait before sending a close again after a refusal that carries no hint, such as a
/// splitting tablet or a missing route. Both settle on the scale of a route refresh.
const CLOSE_ROUTE_RETRY_MS: u64 = 100;

/// Sends `close` until the maintainer settles it or `deadline_boot_ms` passes. `Ok` means it
/// settled: acknowledged, or the volume is already gone. `Err` names what kept it from
/// settling, so the caller waiting on the cleanup can be refused for that reason.
///
/// A refusal ran nothing, so the close waits out its hint within the deadline and is sent
/// again instead of failing the open or create that is replacing the client; see
/// `close_retry_wait_ms` for each refusal's wait. The close is this slot's own cleanup intent,
/// which no edge would send again, so re-driving it here on a deadline is not a retry of a
/// caller's request. An answer waiting cannot change ends the attempt; the slot stays
/// `Closing` and a later caller retries.
async fn close_until_settled<F, Fut>(
	clock: &dyn rivet_env::Clock,
	deadline_boot_ms: u64,
	mut close: F,
) -> Result<(), Unsettled>
where
	F: FnMut() -> Fut,
	Fut: Future<Output = Result<(), RpcError>>,
{
	loop {
		let refusal = match close().await {
			Ok(())
			| Err(RpcError::LifecycleRejected(
				VolumeRejectResponse::VolumeNotFound | VolumeRejectResponse::VolumeDeleted,
			)) => return Ok(()),
			Err(error) => error,
		};
		// The maintainer did not answer, as while it is down, or rejected the close outright.
		// Waiting here would not change that, so the slot stays `Closing` and the caller it
		// refuses, or the next one, sends the close again.
		let Some(wait_ms) = close_retry_wait_ms(&refusal) else {
			tracing::warn!(
				error = ?refusal,
				"volume close will retry while the worker remains live"
			);
			return Err(Unsettled::Answer(refusal));
		};

		// Wait out the operation holding the key, but never past the original deadline.
		let Ok(now_boot_ms) = clock.boot_ms() else {
			return Err(Unsettled::ClockFailed);
		};
		let next_boot_ms = now_boot_ms.saturating_add(wait_ms);
		if next_boot_ms >= deadline_boot_ms {
			tracing::warn!(
				error = ?refusal,
				"volume close stayed refused until its deadline; a later caller retries it"
			);
			return Err(Unsettled::Answer(refusal));
		}
		if clock.sleep_until(next_boot_ms).await.is_err() {
			return Err(Unsettled::ClockFailed);
		}
	}
}

/// How long a cleanup close waits before it is sent again after `refusal`, or `None` for an
/// answer waiting would not change. A refusal ran nothing: `Overloaded` usually means this
/// client's own reopen of the key is still loading after a fence ended its writes;
/// `NotMaintainer`, `TabletSplitting` and a missing route mean the tablet's maintainer is
/// moving. A maintainer that did not answer, or rejected the close outright, ends the attempt.
/// A close the maintainer settled never reaches this.
fn close_retry_wait_ms(refusal: &RpcError) -> Option<u64> {
	match refusal {
		RpcError::LifecycleRejected(VolumeRejectResponse::Overloaded(refusal)) => {
			Some(refusal.retry_after_ms.max(1))
		}
		RpcError::LifecycleRejected(VolumeRejectResponse::NotMaintainer(refusal)) => {
			Some(refusal.retry_after_ms.max(1))
		}
		RpcError::LifecycleRejected(VolumeRejectResponse::VolumeUnderRepair(refusal)) => {
			Some(refusal.retry_after_ms.max(1))
		}
		RpcError::LifecycleRejected(VolumeRejectResponse::TabletSplitting)
		| RpcError::RoutingUnavailable => Some(CLOSE_ROUTE_RETRY_MS),
		// This process's own transport refused the close before sending it.
		RpcError::Overloaded { retry_after_ms } => Some((*retry_after_ms).max(1)),
		RpcError::Unavailable
		| RpcError::Rejected(_)
		| RpcError::LifecycleRejected(
			VolumeRejectResponse::VolumeExists
			| VolumeRejectResponse::VolumeNotFound
			| VolumeRejectResponse::VolumeDeleted
			| VolumeRejectResponse::VolumeRecordMissing(_),
		) => None,
	}
}

/// Why a slot's cleanup close did not settle.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Unsettled {
	/// The close's last answer: a refusal it waited out until its deadline, or an answer it
	/// cannot wait out.
	Answer(RpcError),
	/// This process's clock failed, so the close could neither wait nor measure its deadline.
	ClockFailed,
}



/// Uses the shared manifest, lease guard and transport for raw volume clients.
fn start_client(
	deps: &Dependencies,
	key: &depot_types::VolumeKey,
	identity: VolumeIdentity,
	opened: Option<([u8; 16], VolumeOpenResponse)>,
	may_dispatch: Arc<dyn Fn() -> bool + Send + Sync>,
	initial: Vec<rivet_data::generated::depot_log_shard_v1::VolumeLogEntryBody>,
) -> Result<Volume, Error> {
	let tablet = deps
		.lifecycle
		.tablet(key)
		.map_err(|_| Error::Lifecycle(RpcError::RoutingUnavailable))?;
	let config = deps.config.client.clone();
	let client_state = match opened {
		Some((client_incarnation, response)) => VolumeClientState::from_kept_open_response(
			config,
			client_incarnation,
			response,
			tablet.replica_set,
		),
		None => VolumeClientState::with_replica_set(config, identity, tablet.replica_set),
	}
	.map_err(|_| Error::InvalidRequest)?;
	let lifecycle = deps
		.lifecycle
		.clone()
		.with_dispatch_guard(may_dispatch.clone());
	let terminal_signal = {
		let idle = deps.idle.clone();
		let key = key.clone();
		Box::new(move || idle.signal(&key)) as Box<dyn Fn() + Send + Sync>
	};
	let volume = deps
		.lifecycle
		.start_volume_with_initial(
			deps.env.clone(),
			client_state,
			Some(terminal_signal),
			initial,
			|guard| {
				let lease = deps.may_dispatch.clone();
				Ok(Arc::new(UpsVolumeTransport::new(
					UpsLogClient::new(
						deps.pubsub.clone(),
						deps.capacities,
						deps.timeout,
						Arc::new(move || guard() && lease() && may_dispatch()),
					)?
					.with_lanes(deps.log_lanes.clone()),
					Arc::new(lifecycle),
				)))
			},
		)
		.map_err(|_| Error::Lifecycle(RpcError::Unavailable))?;
	Ok(volume)
}
