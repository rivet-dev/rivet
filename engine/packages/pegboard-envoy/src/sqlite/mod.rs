//! Connection-owned SQLite sessions and staging; databases are files in independently addressed filesystems.
use sqlite_embedded_filesystem::page_protocol::{self, Stage, database};
mod lane_transport;
mod remote;
mod routing;
mod trace_scope;
use anyhow::{Context, Result, ensure};
use depot_volume::volume_client_machine::types::ErrorDisposition;
use pegboard::epoch::{wrapping_ge, wrapping_gt, wrapping_le, wrapping_lt, wrapping_max};
use pegboard::keys::partition::ActorDatabaseVolume;
use rivet_envoy_protocol as p;
use rivet_keys::keys::codec::GroupId;
use rivet_state_machine::IndexedDeadlineQueue;
use rivet_topology::versions::WriteVersions;
use rivet_util_id::Id;
pub(crate) use routing::{
	actor_id, expected_epoch, inbound_overloaded_failure, not_running_failure, overloaded_failure,
	request, stopped_failure,
};
use std::sync::{
	Arc, Weak,
	atomic::{AtomicBool, AtomicUsize, Ordering},
};
use tokio::sync::{Mutex, Notify};
pub(crate) use trace_scope::scope as trace_request;

/// Limits for the optional remote SQL session each actor lane may hold.
#[derive(Clone)]
pub(crate) struct RemoteLimits {
	pub idle_ms: u64,
	/// Largest response, in logical bytes, one remote SQL request may return.
	pub max_response_bytes: usize,
	/// The worker pool shared by every remote SQL session in the process.
	pub pool: sqlite_embedded::worker::SqlitePool,
}

impl Default for RemoteLimits {
	fn default() -> Self {
		Self {
			idle_ms: 30_000,
			max_response_bytes: 24 * 1024 * 1024,
			pool: sqlite_embedded::worker::SqlitePool::new(4, None),
		}
	}
}

pub(crate) struct Sessions {
	/// The group of the connection's namespace, which every database volume opened here is in.
	group: Option<rivet_keys::keys::codec::GroupId>,
	/// Legacy: which actors' database volumes may still be unprefixed, as the connection's
	/// scope reads it, so each legacy-bound open resolves its shape; see
	/// `pegboard::legacy_volumes`. Removed in 3.1 with the backfill.
	legacy_fallback: crate::routes::LegacyFallbackView,
	/// Legacy: how a legacy-bound open reads its namespace's cached standing and, on a connection
	/// without a group, its namespace's current group; see [`legacy_open_resolve`]. Removed in
	/// 3.1 with the backfill.
	legacy_namespace: Arc<LegacyNamespaceCheck>,
	/// Each actor's lane. A request finds an existing lane with a shared bucket read that
	/// allocates nothing, so requests of different actors never queue on one map lock.
	actors: scc::HashMap<Id, Arc<Lane>>,
	/// The number of lanes in `actors`, kept here because `scc::HashMap::len` scans the table.
	lanes: AtomicUsize,
	capacity: usize,
	remote: RemoteLimits,
	/// Idle check times for lanes whose last SQL request left a remote session open. A due
	/// lane is revalidated: a session in a transaction or a busy lane is checked again later.
	expiry: parking_lot::Mutex<IndexedDeadlineQueue<Id>>,
	/// Wakes the expiry loop when a lane is scheduled.
	expiry_changed: Notify,
	/// Whether page replies carry predicted pages, read from the runtime config per request.
	server_prefetch: ServerPrefetch,
}

/// The epoch that owns an actor's storage on a connection, as it was bound: by its start
/// command, or by the actor's record on a connection that did not deliver the start.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct OwnedEpoch {
	pub epoch: u32,
	/// The incarnation the epoch belongs to, the actor's `create_unix_ms`. It names the
	/// database volume.
	pub create_unix_ms: i64,
	/// The group the actor was in when the epoch was bound, or `None` for a legacy actor.
	/// The epoch's storage opens only there and never follows the actor's current
	/// records, which a namespace move changes under a running epoch.
	pub group: Option<GroupId>,
}

/// Reads whether page replies carry predicted pages.
pub(crate) type ServerPrefetch = Arc<dyn Fn() -> bool + Send + Sync>;

/// Legacy: reads the group a namespace has now, uncached
/// (`pegboard::catalog::legacy_read_namespace_group`). Removed in 3.1 with the backfill.
pub(crate) type LegacyNamespaceGroupRead = Arc<
	dyn Fn() -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Option<GroupId>>> + Send>>
		+ Send
		+ Sync,
>;

/// Legacy: reads a namespace's standing from the process cache, with no command
/// (`pegboard::catalog::cached_namespace_standing`). Removed in 3.1 with the backfill.
pub(crate) type LegacyCachedStandingRead = Arc<
	dyn Fn() -> std::pin::Pin<
			Box<
				dyn std::future::Future<Output = Result<Option<Arc<pegboard::NamespaceStanding>>>>
					+ Send,
			>,
		> + Send
		+ Sync,
>;

/// Legacy: the connection's namespace, the read of its current group, which a legacy-bound open
/// on a connection without a group makes before it opens, and the read of its cached standing,
/// which a legacy-bound open on a connection with a group makes for its fallback. Removed in 3.1
/// with the backfill.
#[derive(Clone)]
pub(crate) struct LegacyNamespaceCheck {
	namespace: String,
	read: LegacyNamespaceGroupRead,
	cached_standing: LegacyCachedStandingRead,
}

impl LegacyNamespaceCheck {
	pub(crate) fn new(
		namespace: String,
		read: LegacyNamespaceGroupRead,
		cached_standing: LegacyCachedStandingRead,
	) -> Self {
		Self {
			namespace,
			read,
			cached_standing,
		}
	}

	/// The namespace's group now and its name, or `None` while it has no group.
	async fn current_group(&self) -> Result<Option<(GroupId, String)>> {
		let group = (self.read)().await?;
		Ok(group.map(|group| (group, self.namespace.clone())))
	}

	/// The namespace an open of `id`'s database falls back under now, on a connection in
	/// `group` whose fallback is `fallback`.
	async fn fallback_namespace(
		&self,
		fallback: &crate::routes::LegacyFallbackView,
		group: Option<GroupId>,
		id: Id,
	) -> Option<String> {
		fallback
			.current(group, || (self.cached_standing)())
			.await
			.namespace_for(id)
			.map(str::to_owned)
	}
}

struct Lane {
	actor: Arc<Mutex<Actor>>,
	active: AtomicBool,
	volume: std::sync::Mutex<Option<VolumeRetirement>>,
	server_prefetch: ServerPrefetch,
	/// Whether the volume this lane opened, or is opening, may still dispatch. The volume
	/// client checks it before every request, and retiring the lane clears it.
	serving: std::sync::Mutex<Arc<AtomicBool>>,
}

/// Revokes the same lease record as its owner without keeping that owner alive for cleanup.
struct VolumeRetirement {
	filesystem: Weak<depot_client::filesystem::Filesystem>,
	owner: Option<depot_client::filesystem::owner::FilesystemRetirement>,
}

impl VolumeRetirement {
	fn retire(&self) {
		if let Some(owner) = &self.owner {
			owner.retire();
		} else if let Some(filesystem) = self.filesystem.upgrade() {
			filesystem.fence_with(depot_client::fences::FenceReason::OwnerRetired);
		}
	}
}

impl Lane {
	fn new(server_prefetch: ServerPrefetch) -> Self {
		Self {
			actor: Arc::new(Mutex::new(Actor::default())),
			active: AtomicBool::new(true),
			volume: std::sync::Mutex::new(None),
			server_prefetch,
			serving: std::sync::Mutex::new(Arc::new(AtomicBool::new(true))),
		}
	}
	fn retire(&self) {
		self.active.store(false, Ordering::Release);
		self.fence();
	}

	/// Revoke dispatch and release the exact owner bound to this lane, even while a request
	/// holds its actor or page lock. Cleanup may keep the fenced filesystem context alive.
	fn fence(&self) {
		let slot = self.volume.lock().unwrap();
		self.serving.lock().unwrap().store(false, Ordering::Release);
		if let Some(volume) = slot.as_ref() {
			volume.retire();
		}
	}

	/// The dispatch permission for a volume this lane opens.
	fn guard(&self) -> Arc<dyn Fn() -> bool + Send + Sync> {
		let serving = self.serving.lock().unwrap().clone();
		Arc::new(move || serving.load(Ordering::Acquire))
	}

	fn check(&self) -> Result<()> {
		ensure!(self.active.load(Ordering::Acquire), "SQLite session ended");
		Ok(())
	}
	fn install(&self, volume: &OpenVolume) -> Result<()> {
		let mut slot = self.volume.lock().unwrap();
		if !self.active.load(Ordering::Acquire) {
			volume.retire();
			anyhow::bail!("SQLite session ended");
		}
		*slot = Some(VolumeRetirement {
			filesystem: Arc::downgrade(&volume.fs),
			owner: volume
				._owner
				.as_ref()
				.map(|owner| owner.retirement_handle()),
		});
		Ok(())
	}
}

/// Awaits `wait`, or answers `None` once `deadline_boot_ms` passes first. With no deadline it
/// awaits `wait` alone.
async fn until_deadline<F: std::future::Future>(
	clock: &dyn rivet_env::Clock,
	deadline_boot_ms: Option<u64>,
	wait: F,
) -> Result<Option<F::Output>> {
	let Some(deadline_boot_ms) = deadline_boot_ms else {
		return Ok(Some(wait.await));
	};
	match rivet_env::timeout_at(clock, deadline_boot_ms, wait).await {
		Ok(output) => Ok(Some(output)),
		Err(rivet_env::TimeoutError::Elapsed) => Ok(None),
		Err(rivet_env::TimeoutError::Clock(error)) => {
			Err(error).context("reading the clock for a tail proof's deadline")
		}
	}
}

/// The answer to [`Sessions::prove_tail`].
#[derive(Debug, PartialEq, Eq)]
pub enum TailProof {
	/// A quorum committed an entry of this connection's claim at `committed`, covering the
	/// previous writer's tail.
	Proven { committed: u64 },
	/// This connection does not hold the actor's volume for the asked epoch, so nothing
	/// was written. Expected when the actor restarted, slept or moved after pegboard read it;
	/// `reason` is a stable label for logs.
	NotHeld { reason: &'static str },
	/// The proof did not decide before its deadline, as under a slow quorum: nothing is fenced,
	/// an entry it admitted settles with the volume's next request, and pegboard's next resend
	/// asks again. `error` says which deadline or answer left it undecided.
	Undecided {
		error: depot_client::filesystem::Error,
	},
}

#[derive(Default)]
struct Actor {
	/// The lane's epoch, meaningful once `bound`. Epochs wrap, so it is compared with
	/// the ring helpers, never the integer operators.
	epoch: u32,
	/// Whether an epoch has been bound to this lane. A fresh lane accepts any epoch.
	bound: bool,
	/// The incarnation `epoch` belongs to, the actor's `create_unix_ms`. It names the
	/// database volume.
	create_unix_ms: i64,
	/// The newest epoch whose database volume was found fenced, so another owner took
	/// it. That epoch and older ones never reopen it here; only a newer epoch does.
	fenced_through: Option<u32>,
	/// An epoch whose start showed its storage was never written. Its first SQLite
	/// commit opens the database volume with that commit as the volume's creating writes,
	/// instead of opening the volume and then committing.
	fresh_epoch: Option<u32>,
	session: Option<remote::Session>,
	/// The page protocol state, shared with the remote session's in-process transport. Both
	/// paths reach it only while holding this lane.
	pages: Arc<Mutex<Pages>>,
}

/// One actor's open database volume, for the current epoch: its SQLite file and staged
/// commit, and its KV inode. The SQLite file and the KV inode are created on first use, so a
/// KV-only actor creates no `/sqlite/`.
#[derive(Default)]
pub(super) struct Pages {
	epoch: u32,
	volume: Option<OpenVolume>,
	database: Option<database::Database>,
	stage: Option<Stage>,
	kv_ino: Option<u64>,
}

/// An actor's database volume, open as its exclusive filesystem owner.
pub(crate) struct OpenVolume {
	pub fs: Arc<depot_client::filesystem::Filesystem>,
	/// Keeps the exclusive owner alive; tests open a bare filesystem with no owner.
	pub _owner: Option<depot_client::filesystem::OpenedFilesystem>,
}

impl OpenVolume {
	/// End serving permission independently of references held by page work or native cleanup.
	fn retire(&self) {
		if let Some(owner) = &self._owner {
			owner.retire();
		} else {
			self.fs
				.fence_with(depot_client::fences::FenceReason::OwnerRetired);
		}
	}
}

impl Pages {
	fn fs(&self) -> Result<&Arc<depot_client::filesystem::Filesystem>> {
		Ok(&self
			.volume
			.as_ref()
			.context("actor database volume is not open")?
			.fs)
	}

	/// Opens the SQLite file on first SQLite use.
	async fn database(&mut self) -> Result<&mut database::Database> {
		if self.database.is_none() {
			let fs = self.fs()?.clone();
			let opened = database::Database::from_filesystem(fs).await?;
			self.database = Some(opened);
		}
		self.database
			.as_mut()
			.context("SQLite database is not open")
	}

	/// Finds or creates the KV inode on first KV use.
	async fn kv_ino(&mut self) -> Result<u64> {
		if let Some(ino) = self.kv_ino {
			return Ok(ino);
		}
		let fs = self.fs()?.clone();
		let actor = database::actor_directory(&fs).await?;
		let ino = match fs.lookup(actor, KV_NAME).await {
			Ok(entry) => {
				ensure!(
					entry.kind == depot_fs_core::FsKind::Kv,
					"actor KV inode has the wrong kind"
				);
				entry.child
			}
			Err(depot_client::filesystem::Error::Filesystem(depot_fs_core::Error::NotFound)) => {
				fs.create(
					actor,
					KV_NAME,
					depot_fs_core::rules::new_inode(depot_fs_core::FsKind::Kv, 0o600, 0, 0, 0),
				)
				.await?
			}
			Err(error) => return Err(error.into()),
		};
		self.kv_ino = Some(ino);
		Ok(ino)
	}
}

/// The KV inode's name under the database volume's root.
const KV_NAME: &[u8] = b"kv";
impl Sessions {
	/// A connection's sessions. `legacy_namespace` is how a legacy-bound open on a connection
	/// without a group reads its namespace's current group (legacy, removed in 3.1 with the
	/// backfill).
	pub fn new(
		group: Option<rivet_keys::keys::codec::GroupId>,
		legacy_namespace: LegacyNamespaceCheck,
		capacity: usize,
		remote: RemoteLimits,
		server_prefetch: ServerPrefetch,
	) -> Result<Self> {
		ensure!(capacity > 0, "SQLite session capacity must be positive");
		ensure!(
			remote.idle_ms > 0,
			"remote SQLite session limits must be positive"
		);
		Ok(Self {
			group,
			legacy_fallback: crate::routes::LegacyFallbackView::new(
				pegboard::catalog::LegacyFallback::Off,
			),
			legacy_namespace: Arc::new(legacy_namespace),
			actors: scc::HashMap::new(),
			lanes: AtomicUsize::new(0),
			capacity,
			remote,
			expiry: parking_lot::Mutex::new(IndexedDeadlineQueue::new()),
			expiry_changed: Notify::new(),
			server_prefetch,
		})
	}

	/// Legacy: makes the database opens of the actors `fallback` covers resolve the actor's
	/// volume shape. Removed in 3.1 with the backfill.
	pub fn with_legacy_fallback(mut self, fallback: crate::routes::LegacyFallbackView) -> Self {
		self.legacy_fallback = fallback;
		self
	}

	/// The group the database volume of an epoch bound to `bound_group` opens in; see
	/// [`legacy_bound_volume_group`].
	/// A refused namespace read is sent again under `guard` within the refusal budget counted
	/// from `started_boot_ms`, which the open that follows shares.
	async fn bound_volume_group(
		&self,
		client: &depot_client::Client,
		id: Id,
		bound_group: Option<GroupId>,
		guard: Arc<dyn Fn() -> bool + Send + Sync>,
		started_boot_ms: u64,
	) -> Result<Option<GroupId>> {
		legacy_bound_volume_group(bound_group, || async move {
			let namespace = self.legacy_namespace.clone();
			let fallback_namespace = namespace
				.fallback_namespace(&self.legacy_fallback, self.group, id)
				.await;
			legacy_open_resolve(
				self.group,
				fallback_namespace,
				|| legacy_namespace_group(client.env(), guard, started_boot_ms, namespace),
				|group, fallback_namespace| async move {
					pegboard::legacy_volumes::legacy_resolve_actor_volume_in(
						client,
						group,
						fallback_namespace.as_deref(),
						id,
					)
					.await
				},
			)
			.await
		})
		.await
	}

	/// The open of the database volume of `id`'s incarnation `create_unix_ms` for an epoch
	/// bound to `bound_group`. It never resolves the actor's current group for a grouped
	/// epoch; see [`legacy_bound_volume_group`].
	fn open_volume(
		&self,
		client: depot_client::Client,
		id: Id,
		create_unix_ms: i64,
		bound_group: Option<GroupId>,
	) -> impl FnOnce(
		Arc<dyn Fn() -> bool + Send + Sync>,
	)
		-> std::pin::Pin<Box<dyn std::future::Future<Output = Result<OpenVolume>> + Send>>
	+ use<> {
		let group = self.group;
		// Legacy: only a legacy-bound epoch resolves on its open, so only its requests take
		// the fallback and the namespace check; a grouped epoch's requests touch neither.
		// Removed in 3.1 with the backfill.
		let legacy = match bound_group {
			Some(_) => None,
			None => Some((self.legacy_fallback.clone(), self.legacy_namespace.clone())),
		};
		move |guard| {
			Box::pin(async move {
				// The namespace read, the resolve and the open share one refusal budget.
				let started_boot_ms = client.env().clock().observe()?.boot_ms;
				// Legacy: a legacy-bound epoch resolves on the open itself, so only opens pay
				// for it, never a request on an open volume. Removed in 3.1 with the backfill.
				let resolving_client = &client;
				let reading_guard = guard.clone();
				let group = legacy_bound_volume_group(bound_group, || async move {
					let (fallback, namespace) =
						legacy.context("a legacy-bound open has no namespace check")?;
					let fallback_namespace =
						namespace.fallback_namespace(&fallback, group, id).await;
					legacy_open_resolve(
						group,
						fallback_namespace,
						|| {
							legacy_namespace_group(
								resolving_client.env(),
								reading_guard,
								started_boot_ms,
								namespace,
							)
						},
						|group, fallback_namespace| async move {
							pegboard::legacy_volumes::legacy_resolve_actor_volume_in(
								resolving_client,
								group,
								fallback_namespace.as_deref(),
								id,
							)
							.await
						},
					)
					.await
				})
				.await?;
				open_volume_in(client, group, id, create_unix_ms, guard, started_boot_ms).await
			})
		}
	}

	/// Retires `id`'s session for a stop of epoch `through`, as [`Sessions::remove`]
	/// does. The exclusive lease ends before stop completes; queued native cleanup retains
	/// only fenced filesystem context, allowing the next local epoch to open immediately.
	pub async fn stop(&self, id: Id, through: u32) {
		let lane = self.existing_lane(id).await;
		let Some(lane) = lane else {
			self.expiry.lock().cancel(&id);
			return;
		};
		let mut actor = lane.actor.clone().lock_owned().await;
		if actor.bound && wrapping_gt(actor.epoch, through) {
			return;
		}
		if !self.remove_lane(id, &lane).await {
			return;
		}
		lane.retire();
		actor.session = None;

		// Retired native sessions keep their filesystem context, never this exclusive lease.
		{
			let mut pages = actor.pages.lock().await;
			pages.stage = None;
			if let Some(database) = pages.database.as_mut() {
				database.shutdown_prefetch().await;
			}
			pages.database = None;
			pages.kv_ino = None;
			pages.volume = None;
		}
		drop(actor);
		self.expiry.lock().cancel(&id);
	}

	/// Retires `id`'s session at once, without waiting for a request that holds it. Only for
	/// a stop whose drain timed out: no newer epoch can hold the session yet, and the
	/// stuck request fails at its next session check.
	pub async fn retire_now(&self, id: Id) {
		if let Some((_, lane)) = self.actors.remove_async(&id).await {
			self.lanes.fetch_sub(1, Ordering::AcqRel);
			lane.retire();
		}
		self.expiry.lock().cancel(&id);
	}

	/// Retires `id`'s session when it belongs to `through` or an older epoch. A session a
	/// newer epoch already took is left alone, so retiring a stopped epoch never ends
	/// its successor's session. The check and the retire run under the session lock, which
	/// every request takes before it binds its epoch.
	pub async fn remove(&self, id: Id, through: u32) {
		let lane = self.existing_lane(id).await;
		let Some(lane) = lane else {
			self.expiry.lock().cancel(&id);
			return;
		};
		let actor = lane.actor.clone().lock_owned().await;
		if actor.bound && wrapping_gt(actor.epoch, through) {
			return;
		}
		if !self.remove_lane(id, &lane).await {
			return;
		}
		lane.retire();
		drop(actor);
		self.expiry.lock().cancel(&id);
	}

	/// Closes remote SQL sessions that stayed unused for the idle time, until the caller drops
	/// this future. One sleep covers every lane: it waits for the earliest scheduled check.
	/// A close request stops SQL admission at once; the pool owns native cleanup. The next SQL
	/// request reopens the session.
	pub async fn run_expiry(&self, env: Arc<dyn rivet_env::Env>) -> Result<()> {
		let clock = env.clock();
		loop {
			let changed = self.expiry_changed.notified();
			let next_at = self.expiry.lock().next_at().unwrap_or(u64::MAX);
			rivet_env::select_biased!("a newly scheduled lane may be due earlier than the current head",
				_ = changed => continue,
				slept = clock.sleep_until(next_at) => slept?,
			);
			let now = clock.observe()?.boot_ms;
			let due = self.expiry.lock().pop_due(now, usize::MAX);
			for (_, id) in due {
				self.expire(id, now).await?;
			}
		}
	}

	/// Drops one lane's idle remote session, or reschedules it while it cannot be dropped.
	async fn expire(&self, id: Id, now: u64) -> Result<()> {
		let lane = self.existing_lane(id).await;
		let Some(lane) = lane else {
			return Ok(());
		};
		let retry_at = now.saturating_add(self.remote.idle_ms);
		// A request holding the lane schedules the lane again when it finishes.
		let Ok(mut actor) = lane.actor.clone().try_lock_owned() else {
			return Ok(());
		};
		if actor.session.is_none() {
			return Ok(());
		}
		// Closing a session inside a manual transaction would roll it back.
		if remote::in_transaction(&actor.session) {
			self.expiry.lock().schedule(id, retry_at);
			return Ok(());
		}
		let session = actor.session.take();
		drop(actor);
		drop(session);
		Ok(())
	}

	fn schedule_expiry(&self, env: &Arc<dyn rivet_env::Env>, id: Id) -> Result<()> {
		let at = env
			.clock()
			.observe()?
			.boot_ms
			.saturating_add(self.remote.idle_ms);
		let earlier = schedule_waking(&mut self.expiry.lock(), id, at);
		if earlier {
			self.expiry_changed.notify_one();
		}
		Ok(())
	}
	/// Runs one SQLite request as `epoch` of the incarnation `create_unix_ms`, whose
	/// database volume, in `bound_group` (the group the epoch was bound to), it opens.
	pub async fn execute(
		&self,
		client: depot_client::Client,
		id: Id,
		epoch: u32,
		create_unix_ms: i64,
		bound_group: Option<GroupId>,
		message: p::ToRivet,
	) -> Result<p::ToEnvoy> {
		let span = tracing::trace_span!(target: "sqlite_prefetch::trace", "execute",
            actor_id = %id, epoch, create_unix_ms);
		trace_scope::scope(span, async {
			// The lane is resolved once and carried through both steps of the request.
			let lane = self.lane(id).await?;
			if let Some(reply) = self
				.first_commit(
					&lane,
					&client,
					id,
					epoch,
					create_unix_ms,
					bound_group,
					&message,
				)
				.await?
			{
				return Ok(reply);
			}
			let env = client.env().clone();
			self.execute_in(
				&lane,
				env,
				id,
				epoch,
				create_unix_ms,
				message,
				self.open_volume(client, id, create_unix_ms, bound_group),
			)
			.await
		})
		.await
	}

	/// Makes a quorum of `id`'s database volume prove its tail committed, when this connection
	/// holds that volume open for `epoch`, and returns the committed position. See
	/// [`depot_client::filesystem::Filesystem::prove_tail`], which never runs past
	/// `caller_deadline_boot_ms` and is never cancelled from here. Answers
	/// [`TailProof::NotHeld`] without writing anything when the actor has no session here, its
	/// session belongs to another epoch or ended, or its volume is not open, since then
	/// this process does not hold the claim a fork is waiting on. Answers
	/// [`TailProof::Undecided`] when the proof reached its deadline before its entry decided and
	/// the volume still serves. A proof that fenced the volume, such as one refused for good or
	/// answered by a newer claim, is an error, so its caller reports it. The waits for the
	/// actor's session locks come before the proof is admitted, so they end at
	/// `caller_deadline_boot_ms` too, answering [`TailProof::Undecided`] with
	/// [`depot_client::filesystem::Error::DeadlineExceeded`]: a request holding the session
	/// may wait on storage far longer than the caller.
	pub async fn prove_tail(
		&self,
		clock: &dyn rivet_env::Clock,
		id: Id,
		epoch: u32,
		caller_deadline_boot_ms: Option<u64>,
	) -> Result<TailProof> {
		let Some(lane) = self.existing_lane(id).await else {
			return Ok(TailProof::NotHeld {
				reason: "no_session",
			});
		};
		let undecided = || TailProof::Undecided {
			error: depot_client::filesystem::Error::DeadlineExceeded,
		};
		let fs = {
			let Some(actor) =
				until_deadline(clock, caller_deadline_boot_ms, lane.actor.lock()).await?
			else {
				return Ok(undecided());
			};
			if lane.check().is_err() {
				return Ok(TailProof::NotHeld {
					reason: "session_ended",
				});
			}
			if !actor.bound || actor.epoch != epoch {
				return Ok(TailProof::NotHeld {
					reason: "other_epoch",
				});
			}
			let Some(pages) =
				until_deadline(clock, caller_deadline_boot_ms, actor.pages.lock()).await?
			else {
				return Ok(undecided());
			};
			match pages.fs() {
				Ok(fs) => fs.clone(),
				Err(_) => {
					return Ok(TailProof::NotHeld {
						reason: "volume_not_open",
					});
				}
			}
		};
		match fs.prove_tail(caller_deadline_boot_ms).await {
			Ok(committed) => Ok(TailProof::Proven { committed }),
			Err(error) if fs.proof_undecided(&error) => Ok(TailProof::Undecided { error }),
			Err(error) => Err(error.into()),
		}
	}

	/// Records that `epoch` starts on storage that was never written, so its first SQLite
	/// commit creates the database volume with that commit, in one depot round.
	pub async fn expect_fresh(&self, id: Id, epoch: u32) -> Result<()> {
		let lane = self.lane(id).await?;
		let mut actor = lane.actor.lock().await;
		if !actor.bound || wrapping_ge(epoch, actor.epoch) {
			actor.fresh_epoch = Some(epoch);
		}
		Ok(())
	}

	/// Runs the first SQLite commit of a fresh epoch as the creating writes of its
	/// database volume. Returns `None` when the request is not such a commit, or when the
	/// open did not create the volume with it; the volume is then open, and the request runs
	/// as usual. A reply is returned only once the commit is durable.
	#[allow(
		clippy::too_many_arguments,
		reason = "a first commit needs its lane, client, identity, epoch, incarnation, bound group and request"
	)]
	async fn first_commit(
		&self,
		lane: &Arc<Lane>,
		client: &depot_client::Client,
		id: Id,
		epoch: u32,
		create_unix_ms: i64,
		bound_group: Option<GroupId>,
		message: &p::ToRivet,
	) -> Result<Option<p::ToEnvoy>> {
		let span = tracing::trace_span!(target: "sqlite_prefetch::trace", "first_commit",
            actor_id = %id, epoch, create_unix_ms);
		trace_scope::scope(span, async {
			let p::ToRivet::ToRivetSqliteCommitRequest(_) = message else {
				return Ok(None);
			};
			let mut actor = lane.actor.clone().lock_owned().await;
			lane.check()?;
			if actor.fresh_epoch != Some(epoch) || actor.fenced_through.is_some() {
				return Ok(None);
			}
			actor.fresh_epoch = None;
			let mut pages = actor.pages.clone().lock_owned().await;
			if pages.volume.is_some() || (actor.bound && wrapping_lt(epoch, actor.epoch)) {
				return Ok(None);
			}
			if !actor.bound || actor.epoch != epoch {
				actor.session = None;
				pages.stage = None;
				actor.epoch = epoch;
				actor.bound = true;
				pages.epoch = epoch;
			}
			actor.create_unix_ms = create_unix_ms;

			// The commit runs against an empty volume, and the open carries its writes.
			tracing::trace!(target: "sqlite_prefetch::trace", event = "first_commit_dispatch");
			let guard = lane.guard();
			// The namespace read, the resolve and the open share one refusal budget.
			let started_boot_ms = client.env().clock().observe()?.boot_ms;
			let group = match self
				.bound_volume_group(client, id, bound_group, guard.clone(), started_boot_ms)
				.await
			{
				Ok(group) => group,
				Err(error) => return Err(fence_moved_epoch(&mut actor, epoch, error)),
			};
			tracing::debug!(actor_id = %id, ?group, create_unix_ms, "creating actor database volume with its first commit");
			let open = retry_refused_open(client.env(), guard.clone(), started_boot_ms, || {
				let request = message.clone();
				let client = client.clone().with_dispatch_guard(guard.clone());
				async move {
					client
						.open_or_create_filesystem_with_first_write(
							ActorDatabaseVolume::in_namespace(group, id, create_unix_ms),
							move |fs| {
								Box::pin(async move {
									let mut database =
										database::Database::from_filesystem(fs).await?;
									let mut stage = None;
									page_protocol::execute(&mut stage, &mut database, request).await
								})
							},
						)
						.await
				}
			})
			.await?;

			// The volume is open either way. Its SQLite file opens from it on the next request.
			let fs = open.opened.filesystem().clone();
			let volume = OpenVolume {
				fs,
				_owner: Some(open.opened),
			};
			lane.install(&volume)?;
			pages.volume = Some(volume);
			pages.database = None;
			match open.first {
				Some(reply) => {
					lane.check()?;
					reply.map(Some)
				}
				None => Ok(None),
			}
		})
		.await
	}

	/// Reserves the actor's lane now and returns the volume open to spawn before forwarding its
	/// start. Resolving the lane before spawning lets stop retire even an unpolled prefetch;
	/// the returned future never resolves a replacement lane. The first
	/// SQLite or KV request takes the same lane and finds the volume open, or waits for this open
	/// to finish. A stop retires the lane, which fences the volume and ends a pending open; a
	/// failed open leaves the lane without a volume, so the first request opens it again and
	/// reports its own error.
	pub async fn prefetch(
		&self,
		client: depot_client::Client,
		id: Id,
		epoch: u32,
		create_unix_ms: i64,
		bound_group: Option<GroupId>,
	) -> Result<impl std::future::Future<Output = ()> + Send + use<>> {
		self.prefetch_on(
			id,
			epoch,
			create_unix_ms,
			self.open_volume(client, id, create_unix_ms, bound_group),
		)
		.await
	}

	/// [`Sessions::prefetch`] with the volume open injected, so tests can count opens.
	async fn prefetch_on<F, O>(
		&self,
		id: Id,
		epoch: u32,
		create_unix_ms: i64,
		open: O,
	) -> Result<impl std::future::Future<Output = ()> + use<F, O>>
	where
		F: std::future::Future<Output = Result<OpenVolume>>,
		O: FnOnce(Arc<dyn Fn() -> bool + Send + Sync>) -> F,
	{
		let lane = self.lane(id).await?;
		Ok(async move {
			#[allow(
				clippy::disallowed_methods,
				reason = "slow-open logging uses real time"
			)]
			let started = std::time::Instant::now();
			let result = Self::prepare(&lane, epoch, create_unix_ms, open)
				.await
				.map(|_| ());
			let elapsed = started.elapsed();
			// The start's first storage request waits behind this open, so a slow one stalls it.
			if elapsed >= std::time::Duration::from_secs(5) {
				tracing::warn!(
					%id,
					epoch,
					elapsed_ms = elapsed.as_millis() as u64,
					ok = result.is_ok(),
					"slow actor database volume prefetch; the start's first storage request waits for it"
				);
			}
			if let Err(error) = result {
				tracing::debug!(%id, epoch, ?error, "actor database volume prefetch failed; the first storage request opens it");
			}
		})
	}

	/// Runs one actor KV request on the actor's database volume, in `bound_group`, the group
	/// `epoch` was bound to.
	pub async fn kv(
		&self,
		client: depot_client::Client,
		id: Id,
		epoch: u32,
		create_unix_ms: i64,
		bound_group: Option<GroupId>,
		data: p::KvRequestData,
	) -> Result<p::KvResponseData> {
		let env = client.env().clone();
		let versions: Arc<dyn WriteVersions> = client.topology().clone();
		let retry_after_ms = client.topology().config().lease.refresh_hint_interval_ms;
		self.kv_on(
			env,
			versions,
			retry_after_ms,
			id,
			epoch,
			create_unix_ms,
			data,
			self.open_volume(client, id, create_unix_ms, bound_group),
		)
		.await
	}

	async fn kv_on<F: std::future::Future<Output = Result<OpenVolume>>>(
		&self,
		env: Arc<dyn rivet_env::Env>,
		versions: Arc<dyn WriteVersions>,
		retry_after_ms: u64,
		id: Id,
		epoch: u32,
		create_unix_ms: i64,
		data: p::KvRequestData,
		open: impl FnOnce(Arc<dyn Fn() -> bool + Send + Sync>) -> F,
	) -> Result<p::KvResponseData> {
		let lane = self.lane(id).await?;
		let (_actor, mut pages) = Self::prepare(&lane, epoch, create_unix_ms, open).await?;
		let ino = pages.kv_ino().await?;
		let fs = pages.fs()?.clone();
		let result = crate::kv::execute(
			env.as_ref(),
			versions.as_ref(),
			retry_after_ms,
			&fs,
			ino,
			data,
		)
		.await;
		lane.check()?;
		result
	}

	/// Returns `id`'s lane, adding one within the session capacity. An existing lane costs one
	/// shared bucket read and no allocation; only a missing lane takes the bucket's write lock.
	async fn lane(&self, id: Id) -> Result<Arc<Lane>> {
		if let Some(lane) = self.existing_lane(id).await {
			return Ok(lane);
		}
		match self.actors.entry_async(id).await {
			scc::hash_map::Entry::Occupied(entry) => Ok(entry.get().clone()),
			scc::hash_map::Entry::Vacant(entry) => {
				self.reserve_lane()?;
				let lane = Arc::new(Lane::new(self.server_prefetch.clone()));
				entry.insert_entry(lane.clone());
				Ok(lane)
			}
		}
	}

	async fn existing_lane(&self, id: Id) -> Option<Arc<Lane>> {
		self.actors.read_async(&id, |_, lane| lane.clone()).await
	}

	/// Counts a new lane against the session capacity, or refuses it at capacity.
	fn reserve_lane(&self) -> Result<()> {
		let reserved = self
			.lanes
			.try_update(Ordering::AcqRel, Ordering::Acquire, |lanes| {
				if lanes < self.capacity {
					Some(lanes + 1)
				} else {
					None
				}
			});
		if reserved.is_err() {
			tracing::warn!(capacity = self.capacity, "sqlite session capacity reached");
			// No lane was reserved and no storage operation ran. Keep this refusal typed so
			// the SQLite reply carries a retry hint instead of a generic storage error.
			return Err(depot_client::Overloaded {
				reason: "sqlite_session_capacity".into(),
				retry_after_ms: 100,
			}
			.build());
		}
		Ok(())
	}

	/// Removes `id`'s lane only while it is still `lane`, so retiring an old lane never removes
	/// a successor that replaced it. Returns whether it removed the lane.
	async fn remove_lane(&self, id: Id, lane: &Arc<Lane>) -> bool {
		let removed = self
			.actors
			.remove_if_async(&id, |current| Arc::ptr_eq(current, lane))
			.await
			.is_some();
		if removed {
			self.lanes.fetch_sub(1, Ordering::AcqRel);
		}
		removed
	}

	/// Takes the lane for `epoch` of the incarnation `create_unix_ms` and makes sure its
	/// database volume is open.
	async fn prepare<F: std::future::Future<Output = Result<OpenVolume>>>(
		lane: &Arc<Lane>,
		epoch: u32,
		create_unix_ms: i64,
		open: impl FnOnce(Arc<dyn Fn() -> bool + Send + Sync>) -> F,
	) -> Result<(
		tokio::sync::OwnedMutexGuard<Actor>,
		tokio::sync::OwnedMutexGuard<Pages>,
	)> {
		let mut actor = lane.actor.clone().lock_owned().await;
		lane.check()?;
		let mut pages = actor.pages.clone().lock_owned().await;
		// A lane's epoch only moves forward. A late start or request of an older
		// epoch would otherwise drop the live epoch's staged commit and write with a
		// head the live epoch has moved past, failing its next commit's head fence.
		ensure!(
			!actor.bound || wrapping_ge(epoch, actor.epoch),
			"storage request for an older actor epoch"
		);
		// A fenced volume means another owner opened it, such as the actor's next epoch
		// started elsewhere after this one was judged lost. The fence is terminal for the
		// epoch that held it: reopening would fence the new owner in turn.
		if pages
			.volume
			.as_ref()
			.is_some_and(|volume| volume.fs.is_fenced())
		{
			let fenced = actor
				.fenced_through
				.map_or(actor.epoch, |fenced| wrapping_max(fenced, actor.epoch));
			actor.fenced_through = Some(fenced);
			actor.session = None;
			pages.stage = None;
			if let Some(database) = pages.database.as_mut() {
				database.shutdown_prefetch().await;
			}
			pages.database = None;
			pages.kv_ino = None;
			pages.volume = None;
		}
		if let Some(fenced) = actor.fenced_through {
			if wrapping_le(epoch, fenced) {
				return Err(anyhow::Error::new(page_protocol::HeadFenceMismatch)).with_context(
					|| {
						format!(
							"actor database volume was fenced for epoch {fenced}; only a newer epoch reopens it"
						)
					},
				);
			}
		}
		if !actor.bound || actor.epoch != epoch {
			// A new epoch drops open SQL transactions and staged commits, retaining the
			// exclusive filesystem owner instead of claiming it on every request.
			actor.session = None;
			pages.stage = None;
			actor.epoch = epoch;
			actor.bound = true;
			pages.epoch = epoch;
		}
		actor.create_unix_ms = create_unix_ms;
		if pages.volume.is_none() {
			let volume = match open(lane.guard()).await {
				Ok(volume) => volume,
				Err(error) => return Err(fence_moved_epoch(&mut actor, epoch, error)),
			};
			lane.install(&volume)?;
			pages.volume = Some(volume);
		}
		Ok((actor, pages))
	}

	/// Keeps page recovery and execution inside one actor lane the caller already resolved,
	/// with the volume open injected so tests can supply their own I/O.
	#[allow(
		clippy::too_many_arguments,
		reason = "a request needs its lane, identity, epoch, incarnation and open"
	)]
	async fn execute_in<F: std::future::Future<Output = Result<OpenVolume>>>(
		&self,
		lane: &Arc<Lane>,
		env: Arc<dyn rivet_env::Env>,
		id: Id,
		epoch: u32,
		create_unix_ms: i64,
		message: p::ToRivet,
		open: impl FnOnce(Arc<dyn Fn() -> bool + Send + Sync>) -> F,
	) -> Result<p::ToEnvoy> {
		let span = tracing::trace_span!(target: "sqlite_prefetch::trace", "execute_in",
            actor_id = %id, epoch, create_unix_ms);
		trace_scope::scope(span, async {
			let (mut actor, mut pages) = Self::prepare(lane, epoch, create_unix_ms, open).await?;
			if pages.database.is_none() {
				pages.database().await?;
			}

			let message = match remote::RemoteRequest::from_message(message) {
				Ok(request) => {
					ensure!(pages.stage.is_none(), "staged SQLite commit in progress");
					let fs = pages.fs()?.clone();
					// The remote session's transport takes the page lock for each page request.
					drop(pages);
					if actor.session.is_none() {
						let session = remote::open_session(
							env.clone(),
							actor.pages.clone(),
							fs.clone(),
							Arc::downgrade(lane),
							format!("envoy-remote-sqlite-{}", env.rng("sqlite.vfs").uuid_v4()),
							id.to_string(),
							epoch,
							&self.remote.pool,
							sqlite_embedded::guard::ConnectionGuard {
								max_result_bytes: self.remote.max_response_bytes as u64,
								..Default::default()
							},
						)
						.await?;
						actor.session = Some(session);
					}
					let fenced = || fs.is_fenced();
					let session = actor.session.as_ref().context("remote SQL session")?;
					let reply =
						remote::execute(session, self.remote.max_response_bytes, fenced, request)
							.await;
					if reply.is_err() {
						actor.session = None;
					}
					lane.check()?;
					drop(actor);
					self.schedule_expiry(&env, id)?;
					return reply;
				}
				Err(message) => message,
			};
			// Page requests cannot race or replace a manual SQL transaction. The remote session's
			// VFS caches pages, so a page request also drops it; the next remote SQL reopens it.
			ensure!(
				!remote::in_transaction(&actor.session),
				"SQLite transaction in progress"
			);
			actor.session = None;
			let Pages {
				database, stage, ..
			} = &mut *pages;
			let database = database.as_mut().context("SQLite database is not open")?;
			let guard = lane.guard();
			database.set_server_prefetch((lane.server_prefetch)());
			let result = execute_pages(&env, guard.as_ref(), stage, database, message).await;
			lane.check()?;
			result
		})
		.await
	}
}

/// Opens the database volume of `id`'s incarnation `create_unix_ms` in `group` as its exclusive
/// owner, dispatching only while the lane lives.
fn open_volume_in(
	client: depot_client::Client,
	group: Option<GroupId>,
	id: Id,
	create_unix_ms: i64,
	guard: Arc<dyn Fn() -> bool + Send + Sync>,
	started_boot_ms: u64,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<OpenVolume>> + Send>> {
	Box::pin(async move {
		tracing::debug!(actor_id = %id, ?group, create_unix_ms, "opening actor database volume");
		let owner = retry_refused_open(client.env(), guard.clone(), started_boot_ms, || {
			database::open_volume(
				client.clone().with_dispatch_guard(guard.clone()),
				ActorDatabaseVolume::in_namespace(group, id, create_unix_ms),
			)
		})
		.await?;
		Ok(OpenVolume {
			fs: owner.filesystem().clone(),
			_owner: Some(owner),
		})
	})
}

/// Legacy: the group of the database volume an epoch bound to `bound_group` opens. A
/// grouped epoch opens in the group it was bound to, with no read. A legacy-bound
/// epoch opens its unprefixed volume unless `resolve`, the actor's volume resolve as the
/// connection makes it (no read on a connection without a fallback), finds the actor in a
/// group: the backfill moved it there, and its copy keeps its id and incarnation, so that
/// group's volume is the copy's database. The epoch then fails for good with
/// [`LegacyEpochMoved`] and never opens, claims or creates the copy's database. The legacy
/// behavior opened wherever the resolve answered; it is kept only as this check, taken on the
/// opens of legacy-bound epochs. Removed in 3.1 with the backfill.
async fn legacy_bound_volume_group<F, Fut>(
	bound_group: Option<GroupId>,
	resolve: F,
) -> Result<Option<GroupId>>
where
	F: FnOnce() -> Fut,
	Fut: std::future::Future<Output = Result<Option<GroupId>>>,
{
	if bound_group.is_some() {
		return Ok(bound_group);
	}
	match resolve().await? {
		None => Ok(None),
		Some(_) => Err(LegacyEpochMoved.into()),
	}
}

/// Legacy: the actor's volume resolve for a legacy-bound open on a connection whose group is
/// `connection_group`, which [`legacy_bound_volume_group`] checks. A connection with a group
/// resolves as it always did: `resolve` with that group and `fallback_namespace`, the connection's
/// fallback for the actor. A connection without a group was opened before its namespace moved
/// into one and stays open after the move, and its view of the namespace never changes,
/// so its resolve alone would answer the legacy volume with no read even after the backfill cut
/// the actor's copy; the open would then claim or create the legacy database the move's destroy
/// deletes, and acknowledge writes into it. So it first reads the namespace's current group with
/// `read_namespace_group`, uncached: while the namespace has none, no backfill has started and no
/// copy exists, and the open goes ahead with no further read; once it has one, `resolve` runs
/// with that group and the namespace as the fallback, which answers the group when the copy
/// exists and the legacy volume while the actor has not moved. A connection without a group has
/// no fallback (`pegboard::catalog::LegacyFallback::Off`), so `fallback_namespace` is unused
/// there. The legacy behavior resolved with the connection's view alone. Removed in 3.1 with the
/// backfill.
async fn legacy_open_resolve<N, NF, R, RF>(
	connection_group: Option<GroupId>,
	fallback_namespace: Option<String>,
	read_namespace_group: N,
	resolve: R,
) -> Result<Option<GroupId>>
where
	N: FnOnce() -> NF,
	NF: std::future::Future<Output = Result<Option<(GroupId, String)>>>,
	R: FnOnce(Option<GroupId>, Option<String>) -> RF,
	RF: std::future::Future<Output = Result<Option<GroupId>>>,
{
	if connection_group.is_some() {
		return resolve(connection_group, fallback_namespace).await;
	}
	match read_namespace_group().await? {
		None => Ok(None),
		Some((group, namespace)) => resolve(Some(group), Some(namespace)).await,
	}
}

/// Legacy: the current group of the namespace `check` names, with its name, for
/// [`legacy_open_resolve`]. The read is part of the open, so a read depot refused without
/// running it, as while the namespace volume's maintainer moves or under overload, is sent again
/// under `guard` within the refusal budget counted from `started_boot_ms`, like the open's own
/// refusals; any other error fails the open at once. Removed in 3.1 with the backfill.
async fn legacy_namespace_group(
	env: &Arc<dyn rivet_env::Env>,
	guard: Arc<dyn Fn() -> bool + Send + Sync>,
	started_boot_ms: u64,
	check: Arc<LegacyNamespaceCheck>,
) -> Result<Option<(GroupId, String)>> {
	retry_refused_open(env, guard, started_boot_ms, || check.current_group()).await
}

/// Legacy: makes an epoch whose open found its legacy actor moved into its namespace's
/// group fenced on its lane, so its later requests fail without opening again, and turns the
/// error into the head fence mismatch the runtime treats as lost ownership. Any other error is
/// returned unchanged. Removed in 3.1 with the backfill.
fn fence_moved_epoch(actor: &mut Actor, epoch: u32, error: anyhow::Error) -> anyhow::Error {
	if !error.is::<LegacyEpochMoved>() {
		return error;
	}
	let fenced = actor
		.fenced_through
		.map_or(epoch, |fenced| wrapping_max(fenced, epoch));
	actor.fenced_through = Some(fenced);
	anyhow::Error::new(page_protocol::HeadFenceMismatch).context(LegacyEpochMoved)
}

/// The longest this envoy keeps sending one actor storage request that depot refused without
/// running it. It covers a maintainer move: the old grant's lease expiry plus the successor's
/// open.
const REFUSAL_BUDGET: std::time::Duration = std::time::Duration::from_secs(10);

/// The first backoff ceiling between refused sends, doubled per refusal up to one second.
const REFUSAL_BASE_BACKOFF_MS: u64 = 25;

const REFUSAL_MAX_BACKOFF_MS: u64 = 1_000;

/// The waits between sends of one actor storage request that depot refused without running it,
/// such as a moved maintainer, an overload or a missing route. The actor runtime treats a
/// failed storage request as fatal and never retries it, so this envoy is the edge for actor
/// storage: it waits a jittered exponential backoff that is never shorter than the refusal's
/// hint, within [`REFUSAL_BUDGET`] of the first refusal and only while the lane still owns the
/// actor.
struct RefusalBackoff {
	clock: Arc<dyn rivet_env::Clock>,
	deadline: u64,
	rng: rivet_env::RngStream,
	ceiling_ms: u64,
}

impl RefusalBackoff {
	/// A backoff whose budget is counted from `started_boot_ms`, when the request arrived.
	fn starting_at(
		env: &Arc<dyn rivet_env::Env>,
		label: &str,
		started_boot_ms: u64,
	) -> Result<Self> {
		let clock = env.clock();
		let budget_ms = u64::try_from(REFUSAL_BUDGET.as_millis()).unwrap_or(u64::MAX);
		let deadline = started_boot_ms.saturating_add(budget_ms);
		Ok(Self {
			clock,
			deadline,
			rng: env.rng(label),
			ceiling_ms: REFUSAL_BASE_BACKOFF_MS,
		})
	}

	/// Waits before the next send of a request refused with `hint_ms`, and returns whether it
	/// may be sent: `false` without waiting once the lane no longer owns the actor or the wait
	/// would end past the budget, and `false` after the wait when the lane lost the actor
	/// meanwhile.
	async fn wait(
		&mut self,
		hint_ms: u64,
		guard: &(dyn Fn() -> bool + Send + Sync),
	) -> Result<bool> {
		let ceiling_ms = self.ceiling_ms;
		let backoff_ms = ceiling_ms / 2 + self.rng.gen_range(0..ceiling_ms / 2 + 1);
		let wake = self
			.clock
			.observe()?
			.boot_ms
			.saturating_add(hint_ms.max(backoff_ms));
		if !guard() || wake >= self.deadline {
			return Ok(false);
		}
		self.clock.sleep_until(wake).await?;
		self.ceiling_ms = (ceiling_ms * 2).min(REFUSAL_MAX_BACKOFF_MS);
		Ok(guard())
	}
}

/// Sends an actor database open, or a read the open makes first, again while depot refuses it
/// without running it; see [`RefusalBackoff`]. The budget is counted from `started_boot_ms`, so
/// an open and the reads before it share one. An error that may have run, such as an unanswered
/// open that carried first writes, returns at once.
async fn retry_refused_open<T, F, Fut>(
	env: &Arc<dyn rivet_env::Env>,
	guard: Arc<dyn Fn() -> bool + Send + Sync>,
	started_boot_ms: u64,
	mut open: F,
) -> Result<T>
where
	F: FnMut() -> Fut,
	Fut: std::future::Future<Output = Result<T>>,
{
	let mut backoff =
		RefusalBackoff::starting_at(env, "pegboard_envoy.sqlite.open_refusal", started_boot_ms)?;
	loop {
		let error = match open().await {
			Ok(opened) => return Ok(opened),
			Err(error) => error,
		};
		let Some(hint_ms) = open_refusal_hint_ms(&error) else {
			return Err(error);
		};
		if !backoff.wait(hint_ms, guard.as_ref()).await? {
			return Err(error);
		}
		tracing::debug!(hint_ms, "actor database open refused; sending it again");
	}
}

/// Runs one page protocol request, sending it again while it is refused without running, as
/// when its cold read is refused while the tablet's maintainer moves, or when the volume client
/// refuses a commit's write before dispatching it under overload; see [`RefusalBackoff`].
/// Such a refusal changed neither the database nor its staged commit, which the page protocol
/// keeps for the identical request, so a staged commit's segment or finalize resumes the same
/// stage. No resend starts later than [`REFUSAL_BUDGET`] after the request arrived, a third
/// of the actor's request deadline; each attempt still carries the filesystem's own operation
/// deadline, as the first one does. Every other error, including one that may have run,
/// returns at once.
pub(super) async fn execute_pages(
	env: &Arc<dyn rivet_env::Env>,
	guard: &(dyn Fn() -> bool + Send + Sync),
	stage: &mut Option<Stage>,
	database: &mut database::Database,
	message: p::ToRivet,
) -> Result<p::ToEnvoy> {
	let mut backoff: Option<RefusalBackoff> = None;
	let arrived_boot_ms = env.clock().observe()?.boot_ms;
	loop {
		// Each attempt takes a copy of the request; its page bytes are shared, not copied.
		let error = match page_protocol::execute(stage, database, message.clone()).await {
			Ok(reply) => {
				database.start_prefetch(env);
				return Ok(reply);
			}
			Err(error) => error,
		};
		let Some(hint_ms) = refused_page_request_hint_ms(&message, &error) else {
			return Err(error);
		};
		let backoff = match &mut backoff {
			Some(backoff) => backoff,
			None => backoff.insert(RefusalBackoff::starting_at(
				env,
				"pegboard_envoy.sqlite.page_refusal",
				arrived_boot_ms,
			)?),
		};
		if !backoff.wait(hint_ms, guard).await? {
			// The runtime gets the refusal and restarts its transaction, so the stage the
			// refused request kept is aborted rather than met by that restart.
			page_protocol::abandon_stage(stage, database).await;
			return Err(error);
		}
		tracing::debug!(
			hint_ms,
			"actor SQLite page request refused; sending it again"
		);
	}
}

/// The hint of a page request that depot refused without running it, or `None` for every other
/// error. A cold read refusal qualifies for any page request. A write refusal qualifies only for
/// a single-request commit: a staged commit's refused write does not keep its stage, so it
/// returns to the runtime, which restarts the transaction.
fn refused_page_request_hint_ms(message: &p::ToRivet, error: &anyhow::Error) -> Option<u64> {
	if let Some(hint_ms) = refused_cold_read_hint_ms(error) {
		return Some(hint_ms);
	}
	match message {
		p::ToRivet::ToRivetSqliteCommitRequest(_) => refused_write_hint_ms(error),
		_ => None,
	}
}

/// The hint of a write the volume client refused with a hint before dispatching it, such as the
/// shards' overload. Nothing took a position in the log, so the identical commit may be sent
/// again; a fence that lands meanwhile fails the resend on the filesystem's own status check.
/// A refusal of an admitted write never reaches here as `NotExecuted`, because the filesystem
/// reports it as outcome unknown. A refusal without a hint, such as an entry that is too large,
/// would be refused the same way and returns at once.
///
/// A write the selected entry version cannot carry, and a write refused because the volume's
/// admitted writes are too far ahead of its tablet's apply, were refused at admission too, so
/// they qualify the same way.
fn refused_write_hint_ms(error: &anyhow::Error) -> Option<u64> {
	match error.downcast_ref::<depot_client::filesystem::Error>() {
		Some(
			depot_client::filesystem::Error::VersionUnavailable { retry_after_ms }
			| depot_client::filesystem::Error::ApplyBacklog { retry_after_ms },
		) => Some(*retry_after_ms),
		Some(depot_client::filesystem::Error::Log(error))
			if error.disposition() == ErrorDisposition::NotExecuted =>
		{
			error.retry_after_ms()
		}
		Some(_) | None => None,
	}
}

/// The hint of a page request whose cold read the tablet's maintainer refused, or `None` for
/// every other error.
fn refused_cold_read_hint_ms(error: &anyhow::Error) -> Option<u64> {
	if !page_protocol::refused_without_running(error) {
		return None;
	}
	depot_client::retry_after_ms(error)
}

/// The hint of an open that depot refused without running it, or `None` when it may have run
/// or will be refused the same way however often it is sent. A missing route and a full
/// volume cache or request queue refuse without a hint.
fn open_refusal_hint_ms(error: &anyhow::Error) -> Option<u64> {
	use depot_client::volumes::Error;
	use depot_volume::volume_client_machine::{event::RpcError, types::WriteError};

	if let Some(hint_ms) = depot_client::retry_after_ms(error) {
		return Some(hint_ms);
	}
	match error.downcast_ref::<Error>() {
		Some(
			Error::Overloaded
			| Error::Lifecycle(RpcError::RoutingUnavailable)
			| Error::Write(WriteError::RoutingUnavailable),
		) => Some(REFUSAL_BASE_BACKOFF_MS),
		Some(_) | None => None,
	}
}

/// Legacy: a legacy actor's epoch found, at its storage open, that the backfill moved its
/// actor into its namespace's group. Its storage ended for good: the group's database volume
/// with its id and incarnation is the copy's. Removed in 3.1 with the backfill.
#[derive(Debug)]
pub(crate) struct LegacyEpochMoved;

impl std::fmt::Display for LegacyEpochMoved {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.write_str(
			"actor moved into its namespace's group; this legacy epoch's storage ended and only the moved actor opens its database",
		)
	}
}

impl std::error::Error for LegacyEpochMoved {}

#[derive(Debug)]
pub(crate) struct IndeterminateResult;
impl std::fmt::Display for IndeterminateResult {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.write_str("SQLite result is indeterminate; recover the filesystem before retrying")
	}
}
impl std::error::Error for IndeterminateResult {}

impl Drop for Sessions {
	fn drop(&mut self) {
		// Drop is a forced-sync context, and no request can hold the map while it drops.
		self.actors.iter_sync(|_, lane| {
			lane.retire();
			true
		});
	}
}

/// Schedules `id` at `at` and reports whether the expiry task must wake: only when the queue
/// had no earlier head. A renewal moves a live session later, so steady traffic never wakes it.
fn schedule_waking(queue: &mut IndexedDeadlineQueue<Id>, id: Id, at: u64) -> bool {
	let previous_head = queue.next_at();
	queue.schedule(id, at);
	previous_head.is_none_or(|head| at < head)
}
