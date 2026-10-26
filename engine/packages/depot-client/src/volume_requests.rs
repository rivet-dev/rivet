//! Cached manifest routing and single-attempt lifecycle requests over the shared UPS transport.

use anyhow::{Result, ensure};
use depot_types::hash_range_index::{HashRangeIndex, HashRangeLookup};
use depot_types::{
	VolumeLogKey, client::volume_key, keys::pubsub::VolumeCommandSubject, partition_key_hash,
};
use depot_volume::{
	ups::client::VolumeLifecycleTransport,
	volume_client_machine::{event::RpcError, executor::TransportFuture},
	volume_client_machine::{
		executor::VolumeClientTransport, runner::Volume, state::VolumeClientState,
	},
};
use parking_lot::Mutex;
use rivet_data::{
	DEPOT_CLIENT_PROTOCOL_VERSION,
	generated::{
		depot_client_protocol_v1::{
			ClientRequest, ClientResponse, VolumeCloseRequest, VolumeCreateOptions,
			VolumeOpenRequest, VolumeRejectResponse,
		},
		depot_tablet_manifest_v1::{Tablet, TabletManifest},
		shared_depot_types_v1::VolumeHolder,
	},
	versioned::{DepotClientRequestData, DepotClientResponseData},
};
use rivet_env::Env;
use rivet_topology::worker::WorkerId;
use std::{sync::Arc, time::Duration};
use tokio::sync::watch;
use universalpubsub::{PubSub, SubjectCapacities};
use vbare::Versioned;

/// No manifest route was available before dispatch, so this attempt executed no command.
#[derive(Debug, rivet_error::RivetError)]
#[error(
	"depot",
	"routing_unavailable",
	"No tablet maintainer route is available."
)]
pub struct RoutingUnavailable;

/// The process owns the manifest watcher; this cloneable handle starts no tasks and never lists workers.
#[derive(Clone)]
pub struct VolumeRequests {
	pubsub: PubSub,
	manifests: watch::Receiver<Option<TabletManifest>>,
	/// Range index over the tablets of the manifest version it was built from.
	tablet_ranges: Arc<Mutex<Option<(u64, Arc<HashRangeIndex<usize>>)>>>,
	refresh: watch::Sender<()>,
	capacities: SubjectCapacities,
	/// The window of each worker's KV command subject. Every process that reserves one worker's
	/// subject must agree on it, so it follows `depot.kv_commands.request_capacity`.
	command_capacities: SubjectCapacities,
	timeout: Duration,
	pub(crate) max_command_bytes: Option<usize>,
	may_dispatch: Arc<dyn Fn() -> bool + Send + Sync>,
	/// The process-wide deadline queue every volume client started here keeps its alarms in,
	/// when the owner runs one.
	timers: Option<rivet_state_machine_ephemeral::SharedTimers>,
}

impl VolumeRequests {
	/// The clock of the process these requests run in.
	pub(crate) fn clock(&self) -> Arc<dyn rivet_env::Clock> {
		self.pubsub.env().clock()
	}

	/// The cached tablet manifest as the process's watcher refreshes it, for an owner that
	/// reacts to a manifest change.
	pub(crate) fn manifests(&self) -> watch::Receiver<Option<TabletManifest>> {
		self.manifests.clone()
	}

	pub fn new(
		pubsub: PubSub,
		manifests: watch::Receiver<Option<TabletManifest>>,
		refresh: watch::Sender<()>,
		capacities: SubjectCapacities,
		timeout: Duration,
		may_dispatch: Arc<dyn Fn() -> bool + Send + Sync>,
	) -> Result<Self> {
		ensure!(
			!timeout.is_zero(),
			"volume request timeout must be positive"
		);
		ensure!(
			capacities.incoming > 0 && capacities.outgoing > 0,
			"volume subject capacities must be positive"
		);
		Ok(Self {
			pubsub,
			manifests,
			tablet_ranges: Arc::new(Mutex::new(None)),
			refresh,
			capacities,
			command_capacities: command_capacities(
				rivet_config::config::depot::KvCommandConfig::default().request_capacity(),
			),
			timeout,
			max_command_bytes: None,
			may_dispatch,
			timers: None,
		})
	}

	/// Sets the KV command subject window, which must match the window the target worker's
	/// tablet service reserves.
	pub fn with_command_capacity(mut self, capacity: usize) -> Result<Self> {
		ensure!(capacity > 0, "KV command request capacity must be positive");
		self.command_capacities = command_capacities(capacity);
		Ok(self)
	}

	/// Keeps the alarms of every volume client started from this transport in `timers`, one
	/// deadline queue for the process, instead of one sleep per client. Configure once before
	/// cloning the transport. The caller owns the queue's driver and must run it until every
	/// volume client started here stopped.
	pub fn with_shared_timers(
		mut self,
		timers: rivet_state_machine_ephemeral::SharedTimers,
	) -> Self {
		self.timers = Some(timers);
		self
	}

	/// Starts a volume runner observed for this process's dirty-bytes metric. The runner owns
	/// its observer, so stopping it removes the volume's share.
	/// `on_terminal` runs once, on the runner task, when the runner stops or is fenced.
	pub fn start_volume(
		&self,
		env: Arc<dyn Env>,
		state: VolumeClientState,
		on_terminal: Option<Box<dyn Fn() + Send + Sync>>,
		make_transport: impl FnOnce(
			Arc<dyn Fn() -> bool + Send + Sync>,
		) -> Result<Arc<dyn VolumeClientTransport>>,
	) -> Result<Volume> {
		let observer = crate::volume_observer::VolumeObserver::new(on_terminal);
		let volume = Volume::start_with_timers(
			env,
			state,
			Some(Arc::new(observer)),
			Vec::new(),
			self.timers.clone(),
			make_transport,
		)?;
		Ok(volume)
	}

	/// Opens the volume, creating it when absent with `initial` as its first writes, and waits
	/// until it is ready. The inner result is the open's outcome; the volume is returned either
	/// way so the caller owns its shutdown.
	///
	/// [`Opened::Created`] means the writes are committed and the volume holds nothing else.
	/// [`Opened::Existed`] means this open did not create the volume with them: it existed, the
	/// writes were too large or withheld, or the create did not finish. Its history may then
	/// hold none, all or a prefix of the writes, or another opener's, so the caller reads the
	/// volume and decides from its contents before writing again.
	///
	/// `read` names keys of one KV file to read back when the volume exists. The maintainer
	/// answers them in the open itself only when the volume has no log tail; `Existed` then
	/// carries their committed values, and otherwise `None`, and the caller reads them after the
	/// open as usual.
	pub async fn open_or_create_with(
		&self,
		env: Arc<dyn Env>,
		mut state: VolumeClientState,
		on_terminal: Option<Box<dyn Fn() + Send + Sync>>,
		initial: Vec<rivet_data::generated::depot_log_shard_v1::VolumeLogEntryBody>,
		read: Option<rivet_data::generated::depot_client_protocol_v1::VolumeReadKeys>,
		make_transport: impl FnOnce(
			Arc<dyn Fn() -> bool + Send + Sync>,
		) -> Result<Arc<dyn VolumeClientTransport>>,
	) -> Result<(
		Volume,
		std::result::Result<Opened, depot_volume::volume_client_machine::types::WriteError>,
	)> {
		state.create_if_absent();
		state.set_read_keys(read);
		let volume =
			self.start_volume_with_initial(env, state, on_terminal, initial, make_transport)?;
		let opened = volume
			.handle()
			.ready()
			.await
			.map(|()| match volume.handle().created() {
				Some(through) => Opened::Created(through),
				None => Opened::Existed {
					read: volume.handle().read_back(),
				},
			});
		Ok((volume, opened))
	}

	/// Like [`VolumeRequests::start_volume`] for an opener that creates the volume with
	/// `initial` as its first writes; see `Volume::start_with_initial`.
	pub(crate) fn start_volume_with_initial(
		&self,
		env: Arc<dyn Env>,
		state: VolumeClientState,
		on_terminal: Option<Box<dyn Fn() + Send + Sync>>,
		initial: Vec<rivet_data::generated::depot_log_shard_v1::VolumeLogEntryBody>,
		make_transport: impl FnOnce(
			Arc<dyn Fn() -> bool + Send + Sync>,
		) -> Result<Arc<dyn VolumeClientTransport>>,
	) -> Result<Volume> {
		let observer = crate::volume_observer::VolumeObserver::new(on_terminal);
		let volume = Volume::start_with_timers(
			env,
			state,
			Some(Arc::new(observer)),
			initial,
			self.timers.clone(),
			make_transport,
		)?;
		Ok(volume)
	}

	/// Installs a known server limit so callers can reject oversized commands before dispatch.
	pub fn with_command_limit(mut self, max_bytes: usize) -> Self {
		self.max_command_bytes = Some(max_bytes);
		self
	}

	/// Adds a caller fence without changing the process-owned lifecycle transport used for cleanup.
	pub fn with_dispatch_guard(mut self, guard: Arc<dyn Fn() -> bool + Send + Sync>) -> Self {
		let process = self.may_dispatch;
		self.may_dispatch = Arc::new(move || process() && guard());
		self
	}

	pub(crate) fn deadline(&self) -> Result<u64> {
		rivet_env::deadline_after(self.pubsub.env().clock().as_ref(), self.timeout)
			.map_err(Into::into)
	}

	pub fn worker_id(&self) -> &WorkerId {
		self.pubsub.worker_id()
	}

	/// Returns one coherent cached placement for client construction. A miss requests refresh;
	/// it never falls back to a worker listing or an independently remembered replica set.
	pub fn tablet(&self, key: &depot_types::VolumeKey) -> Result<Tablet> {
		let hash = partition_key_hash(key);
		let manifest = self.manifests.borrow();
		if let Some(tablet) = manifest
			.as_ref()
			.and_then(|manifest| self.covering_tablet(manifest, hash))
		{
			return Ok(tablet.clone());
		}
		drop(manifest);
		self.refresh.send_replace(());
		Err(RoutingUnavailable.build())
	}

	/// Finds the tablet covering `hash` through a range index rebuilt once per manifest
	/// version. A candidate is confirmed against the manifest itself, so an index built from
	/// different contents under the same version only costs a linear search.
	///
	/// With disjoint tablets a confirmed candidate is the only tablet that covers the hash, so
	/// the answer matches a linear search.
	fn covering_tablet<'a>(
		&self,
		manifest: &'a TabletManifest,
		hash: depot_types::PartitionKeyHash,
	) -> Option<&'a Tablet> {
		let index = {
			let mut cached = self.tablet_ranges.lock();
			match &*cached {
				Some((version, index)) if *version == manifest.tablet_manifest_version => {
					index.clone()
				}
				Some(_) | None => {
					let index = Arc::new(HashRangeIndex::new(
						manifest
							.tablets
							.iter()
							.enumerate()
							.map(|(position, tablet)| (position, tablet.hash_range)),
					));
					*cached = Some((manifest.tablet_manifest_version, index.clone()));
					index
				}
			}
		};

		let candidate = match index.lookup(hash) {
			HashRangeLookup::Covered(position) => manifest.tablets.get(*position),
			HashRangeLookup::Uncovered | HashRangeLookup::Unindexed => None,
		};
		// A miss is rare and asks for a refresh, so it confirms against the manifest itself.
		match candidate {
			Some(tablet) if tablet.hash_range.contains(hash) => Some(tablet),
			Some(_) | None => manifest
				.tablets
				.iter()
				.find(|tablet| tablet.hash_range.contains(hash)),
		}
	}

	/// Sends the exact command once. The owner decides retries; stale routes and transport failure
	/// wake the shared manifest watcher without extending this call's original deadline.
	#[tracing::instrument(skip_all)]
	pub async fn request(&self, request: ClientRequest) -> Result<ClientResponse> {
		let clock = self.pubsub.env().clock();
		let deadline = rivet_env::deadline_after(clock.as_ref(), self.timeout)?;
		self.request_at(request, deadline).await
	}

	/// Every retry uses the original deadline, including the deadline sent across UPS.
	pub(crate) async fn request_at(
		&self,
		request: ClientRequest,
		deadline: u64,
	) -> Result<ClientResponse> {
		let Some(operation) = fs_read_label(&request) else {
			return self.request_at_untimed(request, deadline).await;
		};
		#[allow(clippy::disallowed_methods, reason = "metrics use real time")]
		let started = std::time::Instant::now();
		let sent_boot_ms = self.pubsub.env().clock().observe()?.boot_ms;
		let response = self.request_at_untimed(request, deadline).await;
		crate::metrics::FS_READ_SECONDS
			.with_label_values(&[operation])
			.observe(started.elapsed().as_secs_f64());
		// The environment clock, not the metric timer, so a simulation's trace shows each
		// read's place in the open on the simulated clock.
		let answered_boot_ms = self.pubsub.env().clock().observe()?.boot_ms;
		tracing::debug!(
			operation,
			sent_boot_ms,
			elapsed_ms = answered_boot_ms.saturating_sub(sent_boot_ms),
			ok = response.is_ok(),
			"depot filesystem read answered"
		);
		response
	}

	async fn request_at_untimed(
		&self,
		request: ClientRequest,
		deadline: u64,
	) -> Result<ClientResponse> {
		let clock = self.pubsub.env().clock();
		if !(self.may_dispatch)() {
			return Err(crate::ScopeRevoked.build());
		}
		let tablet = self
			.tablet(volume_key(&request).ok_or_else(|| anyhow::anyhow!("empty volume request"))?)?;
		let worker = match tablet.maintainer {
			Some(id) => WorkerId::from_bytes(id),
			None => {
				self.refresh.send_replace(());
				return Err(RoutingUnavailable.build());
			}
		};
		let subject = VolumeCommandSubject::new(worker.clone());
		let _reservation = self.pubsub.reserve_request_subject(
			&subject,
			universalpubsub::RequestClass::DepotVolume,
			self.capacities,
		)?;
		let payload = DepotClientRequestData::serialize_with_embedded_version(
			&request,
			DEPOT_CLIENT_PROTOCOL_VERSION,
		)?;
		let remaining = deadline.saturating_sub(clock.observe()?.boot_ms);
		if remaining == 0 {
			return Err(universalpubsub::errors::Timeout.build());
		}

		let result = self
			.pubsub
			.request_with_timeout_guarded(
				universalpubsub::RequestClass::DepotVolume,
				&worker,
				subject,
				&payload,
				Duration::from_millis(remaining),
				self.may_dispatch.as_ref(),
			)
			.await;
		// Reply authority is fresh too; a sent operation may still have executed.
		if !(self.may_dispatch)() {
			return Err(crate::ScopeRevoked.build());
		}
		if clock.observe()?.boot_ms >= deadline {
			return Err(universalpubsub::errors::Timeout.build());
		}
		let result = result
			.and_then(|bytes| DepotClientResponseData::decode(bytes).map_err(anyhow::Error::from));
		// A stale route refreshes the manifest, so the caller's next attempt finds the
		// current maintainer or the split's children. A read refused `NotServing` came from a
		// maintainer that no longer serves the volume, such as a split's frozen parent, so it
		// refreshes too; without it the reader would retry the same maintainer until the
		// scheduled poll.
		if result.is_err()
			|| matches!(
				&result,
				Ok(ClientResponse::VolumeRejectResponse(
					VolumeRejectResponse::NotMaintainer(_) | VolumeRejectResponse::TabletSplitting
				)) | Ok(ClientResponse::FsReadResponse(
					rivet_data::generated::shared_depot_fs_read_v1::FsReadResponse::FsReadError(
						rivet_data::generated::shared_depot_fs_read_v1::FsReadError::NotServing
					)
				))
			) {
			self.refresh.send_replace(());
		}
		result
	}
}

/// The bounded `operation` label of a filesystem read, or `None` for every other request.
fn fs_read_label(request: &ClientRequest) -> Option<&'static str> {
	use rivet_data::generated::shared_depot_fs_read_v1::FsReadOperation;
	let ClientRequest::FsReadRequest(read) = request else {
		return None;
	};
	Some(match &read.operation {
		FsReadOperation::FsReadOpen(_) => "open",
		FsReadOperation::FsReadGet(_) => "get",
		FsReadOperation::FsReadScan(_) => "scan",
		FsReadOperation::FsReadChunks(_) => "chunk",
		FsReadOperation::FsReadClose(_) => "close",
		FsReadOperation::FsBaseChunksRequest(_) => "base_chunks",
		FsReadOperation::FsForkMetadataRequest(_) => "fork_metadata",
		FsReadOperation::FsBaseKeysRequest(_) => "base_keys",
		FsReadOperation::FsReadKeys(_) => "point",
		FsReadOperation::FsReadRaw(_) => "raw",
	})
}

impl VolumeRequests {
	/// This process as the holder of the volumes it opens: its topology worker and the
	/// instance that identifies this process's lifetime.
	pub fn holder(&self) -> VolumeHolder {
		VolumeHolder {
			worker_id: *self.pubsub.worker_id().as_bytes(),
			worker_incarnation: *self.pubsub.instance_id().as_bytes(),
		}
	}

	/// Sends one lifecycle command within one deadline and returns its answer, refusals
	/// included. The edge owns retries of a user request, so a refusal goes back to the caller
	/// with its hint instead of being resent here; a stale route only triggers a manifest
	/// refresh, so the caller's next attempt finds the current maintainer. The client core
	/// owns replica retries and the decision to reopen a volume.
	pub(crate) async fn lifecycle_request(
		&self,
		request: ClientRequest,
	) -> Result<ClientResponse, RpcError> {
		let clock = self.pubsub.env().clock();
		let deadline = rivet_env::deadline_after(clock.as_ref(), self.timeout)
			.map_err(|_| RpcError::Unavailable)?;
		if !(self.may_dispatch)() {
			return Err(RpcError::Unavailable);
		}
		match self.request_at(request, deadline).await {
			Ok(ClientResponse::VolumeRejectResponse(reason)) => {
				Err(RpcError::LifecycleRejected(reason))
			}
			Ok(response) => Ok(response),
			Err(error) => Err(lifecycle_transport_error(&error)),
		}
	}
}

/// Classifies a lifecycle request that got no answer. A missing route and a UPS admission
/// refusal sent nothing that ran, so they are refusals; any other failure leaves the outcome
/// unknown.
fn lifecycle_transport_error(error: &anyhow::Error) -> RpcError {
	if routing_unavailable(error) {
		return RpcError::RoutingUnavailable;
	}
	if depot_volume::ups::client::is_ups_overloaded(error) {
		return RpcError::Overloaded {
			retry_after_ms: depot_volume::ups::client::TRANSPORT_OVERLOAD_RETRY_AFTER_MS,
		};
	}
	RpcError::Unavailable
}

/// Inspect the structured local error, never a transport error's display text.
fn routing_unavailable(error: &anyhow::Error) -> bool {
	error
		.downcast_ref::<rivet_error::RivetError>()
		.is_some_and(|error| error.group() == "depot" && error.code() == "routing_unavailable")
}

/// The refusal that a request which failed before it was sent stands for, or `None` when it
/// may have been sent and run. A missing route or an unresolved maintainer endpoint follows
/// the tablet to its next maintainer, as a stale route does, and the failed attempt already
/// refreshed the manifest; a full UPS lane is shed load.
pub(crate) fn unsent_refusal(error: &anyhow::Error) -> Option<VolumeRejectResponse> {
	let endpoint_unresolved = error
		.downcast_ref::<rivet_error::RivetError>()
		.is_some_and(|error| error.group() == "ups" && error.code() == "endpoint_unresolved");
	if routing_unavailable(error) || endpoint_unresolved {
		return Some(depot_types::client::not_maintainer());
	}
	if depot_volume::ups::client::is_ups_overloaded(error) {
		return Some(depot_types::client::overloaded());
	}
	None
}

impl VolumeLifecycleTransport for VolumeRequests {
	fn open(
		&self,
		volume_key: depot_types::VolumeKey,
		client_incarnation: [u8; 16],
		create_if_absent: bool,
		initial: Vec<rivet_data::generated::depot_log_shard_v1::PayloadWrite>,
		read: Option<rivet_data::generated::depot_client_protocol_v1::VolumeReadKeys>,
		deadline_boot_ms: u64,
	) -> TransportFuture<'_, rivet_data::generated::depot_client_protocol_v1::VolumeOpenResponse> {
		Box::pin(async move {
			// Command volumes take the default retention, matching a separate create.
			let create_if_absent = if create_if_absent {
				Some(VolumeCreateOptions {
					retention: None,
					initial,
					read,
				})
			} else {
				None
			};
			match self
				.request_at(
					ClientRequest::VolumeOpenRequest(VolumeOpenRequest {
						create_if_absent,
						volume_key,
						client_incarnation,
						holder: self.holder(),
					}),
					deadline_boot_ms,
				)
				.await
				.map_err(|error| lifecycle_transport_error(&error))?
			{
				ClientResponse::VolumeOpenResponse(response) => Ok(response),
				ClientResponse::VolumeRejectResponse(reason) => {
					Err(RpcError::LifecycleRejected(reason))
				}
				_ => Err(RpcError::Unavailable),
			}
		})
	}

	fn close(
		&self,
		key: VolumeLogKey,
		client: Option<rivet_data::generated::shared_depot_types_v1::ClientIdentity>,
	) -> TransportFuture<'_, ()> {
		Box::pin(async move {
			match self
				.lifecycle_request(ClientRequest::VolumeCloseRequest(VolumeCloseRequest {
					client,
					volume_log_key: key.into(),
				}))
				.await?
			{
				ClientResponse::VolumeCloseResponse => Ok(()),
				_ => Err(RpcError::Unavailable),
			}
		})
	}
}

impl VolumeRequests {
	/// Performs one command attempt; retry policy belongs to the typed caller. `bytes` is the
	/// request for `volume_key`, sealed once by the caller and resent unchanged on every attempt.
	pub(crate) async fn command_at(
		&self,
		volume_key: &depot_types::VolumeKey,
		bytes: &[u8],
		deadline: u64,
	) -> Result<rivet_data::generated::depot_kv_command_v1::KvCommandResponse> {
		use depot_types::keys::pubsub::KvCommandSubject;
		use rivet_data::versioned::DepotKvCommandResponseData;
		ensure!((self.may_dispatch)(), "command scope revoked");
		let tablet = self.tablet(volume_key)?;
		let Some(worker) = tablet.maintainer else {
			self.refresh.send_replace(());
			return Err(RoutingUnavailable.build());
		};
		let worker = WorkerId::from_bytes(worker);
		let subject = KvCommandSubject::new(worker.clone());
		let _reservation = self.pubsub.reserve_request_subject(
			&subject,
			universalpubsub::RequestClass::DepotKvCommand,
			self.command_capacities,
		)?;
		if bytes.len() > self.pubsub.max_message_bytes() {
			return Ok(
				rivet_data::generated::depot_kv_command_v1::KvCommandResponse::CommandTooLarge,
			);
		}
		let remaining = deadline.saturating_sub(self.pubsub.env().clock().observe()?.boot_ms);
		if remaining == 0 {
			return Err(crate::command::RequestDeadlineExceeded.build());
		}
		let response = self
			.pubsub
			.request_with_timeout_guarded(
				universalpubsub::RequestClass::DepotKvCommand,
				&worker,
				subject,
				bytes,
				Duration::from_millis(remaining),
				self.may_dispatch.as_ref(),
			)
			.await?;
		// The response body shares the reply buffer.
		Ok(DepotKvCommandResponseData::decode(response)?)
	}

	/// Retry immediately when a new route arrives; an unchanged view cannot hot-spin requests.
	pub(crate) async fn refresh_command_manifest(&self) {
		let mut view = self.manifests.clone();
		view.borrow_and_update();
		self.refresh_manifest();
		let clock = self.pubsub.env().clock();
		let _ = rivet_env::timeout(clock.as_ref(), Duration::from_millis(20), async {
			if view.changed().await.is_err() {
				let _ = rivet_env::sleep(clock.as_ref(), Duration::from_millis(20)).await;
			}
		})
		.await;
	}

	pub(crate) fn refresh_manifest(&self) {
		self.refresh.send_replace(());
	}
}

/// One worker's KV command subject takes the same window in both directions.
fn command_capacities(capacity: usize) -> SubjectCapacities {
	SubjectCapacities {
		incoming: capacity,
		outgoing: capacity,
	}
}

/// How [`VolumeRequests::open_or_create_with`] found the volume.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Opened {
	/// This open created the volume with its initial writes, committed through this position.
	Created(rivet_data::generated::shared_depot_types_v1::VolumeLogPosition),
	/// The volume existed or was created without the initial writes. `read` holds the
	/// requested keys' committed values when the open could answer them.
	Existed {
		read: Option<Vec<Option<vbare::Bytes>>>,
	},
}
