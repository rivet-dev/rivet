//! Process-owned volume slots with retained lifecycle and cancellation cleanup.

mod owner;
mod slot;

pub use owner::Volumes;

use crate::metrics;
use depot_types::VolumeLogKey;
use depot_volume::volume_client_machine::{
	event::RpcError,
	types::{ErrorDisposition, VolumeClientConfig, WriteError},
};
use rivet_data::generated::{
	depot_client_protocol_v1::{CheckpointError, VolumeRejectResponse},
	shared_depot_fs_v1::FsVolumeRecord,
};
use std::sync::Arc;
use tokio::sync::{OwnedSemaphorePermit, Semaphore, mpsc, oneshot};

/// Process bounds include queued requests and operations whose callers stopped waiting.
#[derive(Clone, Debug)]
pub struct Config {
	/// Volumes this process holds open at once. Each costs about 14 KiB of volume client state
	/// and, measured in load heap profiles, under 64 KiB with its slot and filesystem owner;
	/// page caches are bounded separately by the process cache budget. The default of 32768
	/// therefore holds under 2 GiB, which a process serving thousands of actors can afford.
	pub max_loaded_volumes: usize,
	pub max_requests: usize,
	pub max_volume_key_bytes: usize,
	pub sweep_interval_ms: u64,
	/// Quiet time after its last use at which the process closes a held volume and releases
	/// its slot. Defaults to 60 seconds.
	pub close_after_idle_ms: u64,
	/// Idle volume releases in flight at once. Releases share lifecycle admission with active
	/// volume requests, so a large idle cohort must not take all of it.
	pub max_idle_releases: usize,
	pub client: VolumeClientConfig,
	pub filesystem: crate::filesystem::Config,
	/// The process cache budget shared with every other filesystem owner in this process.
	pub process_cache: Option<Arc<crate::filesystem::CacheBudget>>,
	/// Log shard requests every volume shares in flight to one storage worker before later
	/// ones queue for a batch.
	pub log_inflight_per_worker: usize,
}

impl Default for Config {
	fn default() -> Self {
		Self {
			max_loaded_volumes: 32_768,
			max_requests: 1024,
			max_volume_key_bytes: 4096,
			sweep_interval_ms: 100,
			close_after_idle_ms: 60_000,
			max_idle_releases: 32,
			client: VolumeClientConfig::default(),
			filesystem: crate::filesystem::Config::default(),
			process_cache: None,
			log_inflight_per_worker: depot_volume::ups::DEFAULT_INFLIGHT_PER_WORKER,
		}
	}
}

/// Preserves retry evidence instead of flattening a possibly applied write into a routing failure.
#[derive(Debug)]
pub enum Error {
	Checkpoint(crate::checkpoints::Error),
	Filesystem(crate::filesystem::Error),
	Closed,
	Overloaded,
	InvalidRequest,
	InUse,
	Lifecycle(RpcError),
	Write(WriteError),
}

impl Error {
	/// Whether the lifecycle rejected the request because the volume does not exist or was deleted.
	pub fn is_volume_gone(&self) -> bool {
		matches!(
			self,
			Self::Lifecycle(RpcError::LifecycleRejected(
				VolumeRejectResponse::VolumeNotFound | VolumeRejectResponse::VolumeDeleted
			))
		)
	}

	pub fn disposition(&self) -> ErrorDisposition {
		match self {
			Self::Checkpoint(
				crate::checkpoints::Error::Rejected(CheckpointError::Lagging)
				| crate::checkpoints::Error::Interrupted { .. },
			) => ErrorDisposition::NotExecuted,
			Self::Checkpoint(crate::checkpoints::Error::Rejected(_)) => ErrorDisposition::Permanent,
			Self::Checkpoint(crate::checkpoints::Error::Lifecycle(
				VolumeRejectResponse::VolumeRecordMissing(_),
			)) => ErrorDisposition::Permanent,
			Self::Checkpoint(error @ crate::checkpoints::Error::Lifecycle(_))
				if error.retry_after_ms().is_some() =>
			{
				ErrorDisposition::NotExecuted
			}
			Self::Checkpoint(_) => ErrorDisposition::OutcomeUnknown,
			Self::Filesystem(crate::filesystem::Error::RecordMissing { .. }) => {
				ErrorDisposition::Permanent
			}
			Self::Filesystem(crate::filesystem::Error::ColdReadRefused) => {
				ErrorDisposition::NotExecuted
			}
			Self::Filesystem(_) => ErrorDisposition::OutcomeUnknown,
			Self::Closed | Self::Overloaded => ErrorDisposition::NotExecuted,
			Self::InvalidRequest => ErrorDisposition::Permanent,
			Self::InUse => ErrorDisposition::NotExecuted,
			Self::Write(error) => error.disposition(),
			Self::Lifecycle(RpcError::RoutingUnavailable | RpcError::Overloaded { .. }) => {
				ErrorDisposition::NotExecuted
			}
			Self::Lifecycle(RpcError::Unavailable | RpcError::Rejected(_)) => {
				ErrorDisposition::OutcomeUnknown
			}
			Self::Lifecycle(RpcError::LifecycleRejected(reason)) => match reason {
				VolumeRejectResponse::VolumeNotFound
				| VolumeRejectResponse::VolumeExists
				| VolumeRejectResponse::VolumeRecordMissing(_) => ErrorDisposition::Permanent,
				VolumeRejectResponse::NotMaintainer(_)
				| VolumeRejectResponse::TabletSplitting
				| VolumeRejectResponse::VolumeDeleted
				| VolumeRejectResponse::Overloaded(_)
				| VolumeRejectResponse::VolumeUnderRepair(_) => ErrorDisposition::NotExecuted,
			},
		}
	}
}

impl std::fmt::Display for Error {
	fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(formatter, "volume request failed: {self:?}")
	}
}

impl std::error::Error for Error {}

enum Command {
	ForkMinimum {
		key: depot_types::VolumeKey,
	},
	Checkpoint {
		key: depot_types::VolumeKey,
		name: Vec<u8>,
	},
	CheckpointUpdate {
		key: depot_types::VolumeKey,
		update: crate::checkpoints::CheckpointUpdate,
	},
	Open {
		key: depot_types::VolumeKey,
		/// Whether the maintainer creates a missing volume inside this open.
		create_if_absent: bool,
		/// Entries a creating open writes as the new volume's first writes. Empty for an
		/// ordinary open.
		initial: Vec<rivet_data::generated::depot_log_shard_v1::VolumeLogEntryBody>,
		/// What the opening reads first, answered with the cold snapshot's open.
		prefetch: crate::filesystem::transport::OpenPrefetch,
	},
	Create {
		key: depot_types::VolumeKey,
		retention: Option<depot_fs_core::FsRetentionPolicy>,
	},
	Delete {
		key: VolumeLogKey,
	},
}

impl Command {
	fn key(&self) -> &depot_types::VolumeKey {
		match self {
			Self::ForkMinimum { key }
			| Self::Checkpoint { key, .. }
			| Self::CheckpointUpdate { key, .. }
			| Self::Open { key, .. }
			| Self::Create { key, .. } => key,
			Self::Delete { key } => &key.0.volume_key,
		}
	}
}

enum Response {
	ForkMinimum(Option<depot_fs_core::FsForkMinimum>),
	Checkpoint(rivet_data::generated::shared_depot_fs_v1::FsCheckpointInfo),
	CheckpointUpdated,
	/// The opened filesystem, and whether this open created the volume with its `initial`
	/// entries as the first writes.
	Opened(crate::filesystem::OpenedFilesystem, bool),
	Created(FsVolumeRecord),
	Deleted,
}

struct Request {
	may_dispatch: Arc<dyn Fn() -> bool + Send + Sync>,
	command: Command,
	reply: oneshot::Sender<Result<Response, Error>>,
	_permit: RequestPermit,
}

/// Tracks accepted work independently of how long its caller keeps the response future alive.
struct RequestPermit {
	_permit: OwnedSemaphorePermit,
	_metric: rivet_metrics::IntGaugeGuard,
}

/// Clones are weak handles: dropping the owner closes admission even if clones remain alive.
/// A filesystem opened with a known first write; see
/// [`VolumeClient::open_or_create_filesystem_with_first_write`].
pub struct FirstWriteOpen<T> {
	pub opened: crate::filesystem::OpenedFilesystem,
	/// The first write's result when the open created the volume with its entries, which are
	/// then durable. `None` when the caller must run the write on `opened`.
	pub first: Option<anyhow::Result<T>>,
}

#[derive(Clone)]
pub struct VolumeClient {
	pub(crate) lifecycle: crate::volume_requests::VolumeRequests,
	stopping: tokio::sync::watch::Receiver<bool>,
	worker_id: rivet_topology::worker::WorkerId,
	inbox: mpsc::WeakSender<Request>,
	requests: Arc<Semaphore>,
	config: Config,
	may_dispatch: Arc<dyn Fn() -> bool + Send + Sync>,
	versions: Arc<dyn rivet_topology::versions::WriteVersions>,
}

impl VolumeClient {
	/// A nonholder returns no hint and never opens or takes over the source.
	pub(crate) async fn fork_minimum(
		&self,
		key: depot_types::VolumeKey,
	) -> Result<Option<depot_fs_core::FsForkMinimum>, Error> {
		match self.request(Command::ForkMinimum { key }).await? {
			Response::ForkMinimum(minimum) => Ok(minimum),
			_ => unreachable!("fork minimum has one response"),
		}
	}
	pub async fn checkpoint_create(
		&self,
		key: depot_types::VolumeKey,
		name: Vec<u8>,
	) -> Result<rivet_data::generated::shared_depot_fs_v1::FsCheckpointInfo, Error> {
		if !depot_fs_core::checkpoints::valid_name(&name) {
			return Err(Error::InvalidRequest);
		}
		match self.request(Command::Checkpoint { key, name }).await? {
			Response::Checkpoint(info) => Ok(info),
			_ => unreachable!("checkpoint has one response"),
		}
	}

	/// Releases a named checkpoint or sets the retention policy. A holder of the volume logs
	/// the change and returns once the maintainer has applied it.
	pub(crate) async fn checkpoint_update(
		&self,
		key: depot_types::VolumeKey,
		update: crate::checkpoints::CheckpointUpdate,
	) -> Result<(), Error> {
		if !update.valid() {
			return Err(Error::InvalidRequest);
		}
		match self
			.request(Command::CheckpointUpdate { key, update })
			.await?
		{
			Response::CheckpointUpdated => Ok(()),
			_ => unreachable!("checkpoint update has one response"),
		}
	}

	pub async fn open_filesystem(
		&self,
		key: depot_types::VolumeKey,
	) -> anyhow::Result<crate::filesystem::OpenedFilesystem> {
		self.open(
			key,
			false,
			crate::filesystem::transport::OpenPrefetch::default(),
		)
		.await
	}

	/// Opens an exclusive filesystem owner, creating the volume inside the same open when it is
	/// missing. One maintainer admission serves both, so a new volume needs no separate create.
	pub async fn open_or_create_filesystem(
		&self,
		key: depot_types::VolumeKey,
	) -> anyhow::Result<crate::filesystem::OpenedFilesystem> {
		self.open(
			key,
			true,
			crate::filesystem::transport::OpenPrefetch::default(),
		)
		.await
	}

	/// [`VolumeClient::open_or_create_filesystem`] for an opening that knows which file it
	/// reads first: the cold snapshot's open also returns `prefetch`'s rows, so the opening's
	/// first lookups, attribute reads and page reads need no round trip of their own.
	pub async fn open_or_create_filesystem_prefetching(
		&self,
		key: depot_types::VolumeKey,
		prefetch: crate::filesystem::transport::OpenPrefetch,
	) -> anyhow::Result<crate::filesystem::OpenedFilesystem> {
		self.open(key, true, prefetch).await
	}

	/// [`VolumeClient::open_or_create_filesystem`] for a volume whose first write is known:
	/// `write` runs against an empty filesystem until its first commit, and that commit's
	/// entries become the creating open's initial writes, so the volume's creation and the
	/// commit share one disk write.
	///
	/// The write's commit waits for the open. When the open created the volume with those
	/// entries, the commit succeeds with them durable and the write's result is returned in
	/// [`FirstWriteOpen::first`]. When the volume already existed, the entries were too large
	/// for the creating open, or the write finished without committing, its commit fails
	/// before the write acts on it and `first` is `None`: the caller runs the write again on
	/// the opened filesystem, which then reads whatever the volume holds.
	pub async fn open_or_create_filesystem_with_first_write<T, F>(
		&self,
		key: depot_types::VolumeKey,
		write: F,
	) -> anyhow::Result<FirstWriteOpen<T>>
	where
		F: FnOnce(
			Arc<crate::filesystem::Filesystem>,
		) -> futures_util::future::BoxFuture<'static, anyhow::Result<T>>,
	{
		use crate::filesystem::capture::{CaptureJournal, NoColdStore};

		// Run the write on an empty in-memory volume up to its first commit.
		let journal = Arc::new(CaptureJournal::new(
			VolumeLogKey(rivet_data::generated::shared_depot_types_v1::VolumeLogKey {
				volume_key: key.clone(),
				volume_incarnation: [0; 16],
			}),
			self.versions.clone(),
		));
		let empty = crate::filesystem::Filesystem::open_staging_empty(
			journal.clone(),
			Arc::new(NoColdStore),
			self.lifecycle.clock(),
			self.config.filesystem.clone(),
			None,
			crate::filesystem::transport::OpenPrefetch::default(),
		)
		.await?;
		let mut run = write(empty);
		let committing = rivet_env::select_biased!("a write that finished needs no creating writes",
			_ = &mut run => false,
			() = journal.commit_requested() => true,
		);
		let initial = journal.initial_writes();
		if !committing || initial.is_empty() {
			journal.decide(false);
			let opened = self
				.open(
					key,
					true,
					crate::filesystem::transport::OpenPrefetch::default(),
				)
				.await?;
			return Ok(FirstWriteOpen {
				opened,
				first: None,
			});
		}

		// Open with those entries; the write's commit stands only if they created the volume.
		let opened = self
			.request(Command::Open {
				key,
				create_if_absent: true,
				initial,
				prefetch: crate::filesystem::transport::OpenPrefetch::default(),
			})
			.await;
		let (opened, created) = match opened {
			Ok(Response::Opened(opened, created)) => (opened, created),
			Ok(_) => unreachable!("open has one response type"),
			Err(error) => {
				journal.decide(false);
				return Err(error.into());
			}
		};
		journal.decide(created);
		let first = match created {
			true => Some(run.await),
			false => None,
		};
		Ok(FirstWriteOpen { opened, first })
	}

	async fn open(
		&self,
		key: depot_types::VolumeKey,
		create_if_absent: bool,
		prefetch: crate::filesystem::transport::OpenPrefetch,
	) -> anyhow::Result<crate::filesystem::OpenedFilesystem> {
		let command = Command::Open {
			key,
			create_if_absent,
			initial: Vec::new(),
			prefetch,
		};
		match self.request(command).await? {
			Response::Opened(opened, _) => Ok(opened),
			_ => unreachable!("open has one response type"),
		}
	}

	pub fn worker_id(&self) -> &rivet_topology::worker::WorkerId {
		&self.worker_id
	}

	/// Restricts only this caller's admission and replies, without revoking other users of the client.
	pub(crate) fn with_dispatch_guard(
		mut self,
		guard: Arc<dyn Fn() -> bool + Send + Sync>,
	) -> Self {
		let previous = self.may_dispatch;
		self.may_dispatch = Arc::new(move || previous() && guard());
		self
	}

	/// Maintainer reads use the same caller fence and process lifetime as owned volumes.
	pub(crate) fn read_requests(&self) -> crate::volume_requests::VolumeRequests {
		let guard = self.may_dispatch.clone();
		let stopping = self.stopping.clone();
		self.lifecycle
			.clone()
			.with_dispatch_guard(Arc::new(move || !*stopping.borrow() && guard()))
	}

	/// Creates through the maintainer and retains idle ownership even if this future is cancelled.
	pub async fn create(&self, key: depot_types::VolumeKey) -> Result<FsVolumeRecord, Error> {
		self.create_with_retention(key, None).await
	}
	pub async fn create_with_retention(
		&self,
		key: depot_types::VolumeKey,
		retention: Option<depot_fs_core::FsRetentionPolicy>,
	) -> Result<FsVolumeRecord, Error> {
		if !depot_fs_core::checkpoints::valid_policy(retention.as_ref()) {
			return Err(Error::InvalidRequest);
		}
		match self.request(Command::Create { key, retention }).await? {
			Response::Created(row) => Ok(row),
			_ => unreachable!("create has one response type"),
		}
	}

	/// Deletes this incarnation only, so a delayed caller cannot delete a recreated volume.
	pub async fn delete(&self, key: VolumeLogKey) -> Result<(), Error> {
		match self.request(Command::Delete { key }).await? {
			Response::Deleted => Ok(()),
			_ => unreachable!("delete has one response type"),
		}
	}

	/// Admission transfers both ownership and the request permit before the first cancellable wait.
	async fn request(&self, command: Command) -> Result<Response, Error> {
		if !(self.may_dispatch)() {
			return Err(Error::Closed);
		}
		if command.key().byte_len() > self.config.max_volume_key_bytes {
			return Err(Error::InvalidRequest);
		}
		let permit = self.requests.clone().try_acquire_owned().map_err(|_| {
			metrics::REJECTED.with_label_values(&["requests"]).inc();
			Error::Overloaded
		})?;
		let permit = RequestPermit {
			_permit: permit,
			_metric: rivet_metrics::GaugeGuardExt::inc_guard(&*metrics::VOLUME_REQUESTS),
		};
		if self.requests.available_permits() == 0 {
			tracing::warn!(
				capacity = self.config.max_requests,
				"volume request capacity reached"
			);
		}
		let may_dispatch: Arc<dyn Fn() -> bool + Send + Sync> =
			if matches!(command, Command::Open { .. }) {
				let guard = self.may_dispatch.clone();
				let stop = self.stopping.clone();
				Arc::new(move || !(*stop.borrow()) && guard())
			} else {
				self.may_dispatch.clone()
			};
		let inbox = self.inbox.upgrade().ok_or(Error::Closed)?;
		let (reply, receive) = oneshot::channel();
		inbox
			.try_send(Request {
				may_dispatch: may_dispatch.clone(),
				command,
				reply,
				_permit: permit,
			})
			.map_err(|error| match error {
				mpsc::error::TrySendError::Full(_) => Error::Overloaded,
				mpsc::error::TrySendError::Closed(_) => Error::Closed,
			})?;
		drop(inbox);
		let response = receive
			.await
			.map_err(|_| Error::Lifecycle(RpcError::Unavailable))?;
		if !may_dispatch() {
			// Revocation after admission cannot assert that the operation was never executed.
			return Err(Error::Lifecycle(RpcError::Unavailable));
		}
		response
	}
}
