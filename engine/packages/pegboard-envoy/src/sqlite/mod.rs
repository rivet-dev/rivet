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
use rivet_state_machine::IndexedDeadlineQueue;
use rivet_util_id::Id;
pub(crate) use routing::{
	actor_id, expected_generation, not_running_failure, overloaded_failure, request,
	stopped_failure,
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

/// Reads whether page replies carry predicted pages.
pub(crate) type ServerPrefetch = Arc<dyn Fn() -> bool + Send + Sync>;

struct Lane {
	actor: Arc<Mutex<Actor>>,
	active: AtomicBool,
	filesystem: std::sync::Mutex<Option<Weak<depot_client::filesystem::Filesystem>>>,
	server_prefetch: ServerPrefetch,
	/// Whether the volume this lane opened, or is opening, may still dispatch. The volume
	/// client checks it before every request, and retiring the lane clears it.
	serving: std::sync::Mutex<Arc<AtomicBool>>,
}
impl Lane {
	fn new(server_prefetch: ServerPrefetch) -> Self {
		Self {
			actor: Arc::new(Mutex::new(Actor::default())),
			active: AtomicBool::new(true),
			filesystem: std::sync::Mutex::new(None),
			server_prefetch,
			serving: std::sync::Mutex::new(Arc::new(AtomicBool::new(true))),
		}
	}
	fn retire(&self) {
		self.active.store(false, Ordering::Release);
		self.fence();
	}

	/// Fences the filesystem this lane opened, if any, so a request still holding it cannot
	/// write through it.
	fn fence(&self) {
		let slot = self.filesystem.lock().unwrap();
		self.serving.lock().unwrap().store(false, Ordering::Release);
		if let Some(fs) = slot.as_ref().and_then(Weak::upgrade) {
			fs.fence_with(depot_client::fences::FenceReason::OwnerRetired);
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
	fn install(&self, fs: &Arc<depot_client::filesystem::Filesystem>) -> Result<()> {
		let mut slot = self.filesystem.lock().unwrap();
		if !self.active.load(Ordering::Acquire) {
			fs.fence_with(depot_client::fences::FenceReason::OwnerRetired);
			anyhow::bail!("SQLite session ended");
		}
		*slot = Some(Arc::downgrade(fs));
		Ok(())
	}
}
#[derive(Default)]
struct Actor {
	/// The lane's generation, meaningful once `bound`. Epochs wrap, so it is compared with
	/// the ring helpers, never the integer operators.
	generation: u32,
	/// Whether a generation has been bound to this lane. A fresh lane accepts any generation.
	bound: bool,
	/// The incarnation `generation` belongs to, the actor's `create_unix_ms`. It names the
	/// database volume.
	create_unix_ms: i64,
	/// The newest generation whose database volume was found fenced, so another owner took
	/// it. That generation and older ones never reopen it here; only a newer generation does.
	fenced_through: Option<u32>,
	/// A generation whose start showed its storage was never written. Its first SQLite
	/// commit opens the database volume with that commit as the volume's creating writes,
	/// instead of opening the volume and then committing.
	fresh_generation: Option<u32>,
	session: Option<remote::Session>,
	/// The page protocol state, shared with the remote session's in-process transport. Both
	/// paths reach it only while holding this lane.
	pages: Arc<Mutex<Pages>>,
}

/// One actor's open database volume, for the current generation: its SQLite file and staged
/// commit, and its KV inode. The SQLite file and the KV inode are created on first use, so a
/// KV-only actor creates no `/sqlite/`.
#[derive(Default)]
pub(super) struct Pages {
	generation: u32,
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
	pub fn new(
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
			actors: scc::HashMap::new(),
			lanes: AtomicUsize::new(0),
			capacity,
			remote,
			expiry: parking_lot::Mutex::new(IndexedDeadlineQueue::new()),
			expiry_changed: Notify::new(),
			server_prefetch,
		})
	}

	/// Retires `id`'s session for a stop of generation `through`, as [`Sessions::remove`]
	/// does, and closes its database volume now instead of when the lane's last holder drops
	/// it, so its maintainer can unload and tier it. The actor's next start, on any engine,
	/// opens the volume again.
	pub async fn stop(&self, id: Id, through: u32) {
		let lane = self.existing_lane(id).await;
		let Some(lane) = lane else {
			self.expiry.lock().cancel(&id);
			return;
		};
		let mut actor = lane.actor.clone().lock_owned().await;
		if actor.bound && wrapping_gt(actor.generation, through) {
			return;
		}
		if !self.remove_lane(id, &lane).await {
			return;
		}
		lane.active.store(false, Ordering::Release);
		actor.session = None;

		// Dropping the exclusive owner fences the volume and begins its close. The lane's own
		// fence follows; no request can run in between, because this holds the session lock.
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
		lane.fence();
		drop(actor);
		self.expiry.lock().cancel(&id);
	}

	/// Retires `id`'s session at once, without waiting for a request that holds it. Only for
	/// a stop whose drain timed out: no newer generation can hold the session yet, and the
	/// stuck request fails at its next session check.
	pub async fn retire_now(&self, id: Id) {
		if let Some((_, lane)) = self.actors.remove_async(&id).await {
			self.lanes.fetch_sub(1, Ordering::AcqRel);
			lane.retire();
		}
		self.expiry.lock().cancel(&id);
	}

	/// Retires `id`'s session when it belongs to `through` or an older generation. A session a
	/// newer generation already took is left alone, so retiring a stopped generation never ends
	/// its successor's session. The check and the retire run under the session lock, which
	/// every request takes before it binds its generation.
	pub async fn remove(&self, id: Id, through: u32) {
		let lane = self.existing_lane(id).await;
		let Some(lane) = lane else {
			self.expiry.lock().cancel(&id);
			return;
		};
		let actor = lane.actor.clone().lock_owned().await;
		if actor.bound && wrapping_gt(actor.generation, through) {
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
	/// Dropping a session closes its connection and VFS; the next SQL request
	/// reopens it.
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
	/// Runs one SQLite request as `generation` of the incarnation `create_unix_ms`, whose
	/// database volume it opens.
	pub async fn execute(
		&self,
		client: depot_client::Client,
		id: Id,
		generation: u32,
		create_unix_ms: i64,
		message: p::ToRivet,
	) -> Result<p::ToEnvoy> {
		let span = tracing::trace_span!(target: "sqlite_prefetch::trace", "execute",
            actor_id = %id, generation, create_unix_ms);
		trace_scope::scope(span, async {
			// The lane is resolved once and carried through both steps of the request.
			let lane = self.lane(id).await?;
			if let Some(reply) = self
				.first_commit(&lane, &client, id, generation, create_unix_ms, &message)
				.await?
			{
				return Ok(reply);
			}
			let env = client.env().clone();
			self.execute_in(
				&lane,
				env,
				id,
				generation,
				create_unix_ms,
				message,
				open_volume(client, id, create_unix_ms),
			)
			.await
		})
		.await
	}

	/// Records that `generation` starts on storage that was never written, so its first SQLite
	/// commit creates the database volume with that commit, in one depot round.
	pub async fn expect_fresh(&self, id: Id, generation: u32) -> Result<()> {
		let lane = self.lane(id).await?;
		let mut actor = lane.actor.lock().await;
		if !actor.bound || wrapping_ge(generation, actor.generation) {
			actor.fresh_generation = Some(generation);
		}
		Ok(())
	}

	/// Runs the first SQLite commit of a fresh generation as the creating writes of its
	/// database volume. Returns `None` when the request is not such a commit, or when the
	/// open did not create the volume with it; the volume is then open, and the request runs
	/// as usual. A reply is returned only once the commit is durable.
	async fn first_commit(
		&self,
		lane: &Arc<Lane>,
		client: &depot_client::Client,
		id: Id,
		generation: u32,
		create_unix_ms: i64,
		message: &p::ToRivet,
	) -> Result<Option<p::ToEnvoy>> {
		let span = tracing::trace_span!(target: "sqlite_prefetch::trace", "first_commit",
            actor_id = %id, generation, create_unix_ms);
		trace_scope::scope(span, async {
			let p::ToRivet::ToRivetSqliteCommitRequest(_) = message else {
				return Ok(None);
			};
			let mut actor = lane.actor.clone().lock_owned().await;
			lane.check()?;
			if actor.fresh_generation != Some(generation) || actor.fenced_through.is_some() {
				return Ok(None);
			}
			actor.fresh_generation = None;
			let mut pages = actor.pages.clone().lock_owned().await;
			if pages.volume.is_some() || (actor.bound && wrapping_lt(generation, actor.generation))
			{
				return Ok(None);
			}
			if !actor.bound || actor.generation != generation {
				actor.session = None;
				pages.stage = None;
				actor.generation = generation;
				actor.bound = true;
				pages.generation = generation;
			}
			actor.create_unix_ms = create_unix_ms;

			// The commit runs against an empty volume, and the open carries its writes.
			tracing::trace!(target: "sqlite_prefetch::trace", event = "first_commit_dispatch");
			let guard = lane.guard();
			let open = retry_refused_open(client.env(), guard.clone(), || {
				let request = message.clone();
				let client = client.clone().with_dispatch_guard(guard.clone());
				async move {
					client
						.open_or_create_filesystem_with_first_write(
							ActorDatabaseVolume::new(id, create_unix_ms).volume_key(),
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
			lane.install(&fs)?;
			pages.volume = Some(OpenVolume {
				fs,
				_owner: Some(open.opened),
			});
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

	/// Opens the actor's database volume for `generation` ahead of its first storage request, so
	/// the open overlaps the start command's delivery and the runtime's own startup. The first
	/// SQLite or KV request takes the same lane and finds the volume open, or waits for this open
	/// to finish. A stop retires the lane, which fences the volume and ends a pending open; a
	/// failed open leaves the lane without a volume, so the first request opens it again and
	/// reports its own error.
	pub async fn prefetch(
		&self,
		client: depot_client::Client,
		id: Id,
		generation: u32,
		create_unix_ms: i64,
	) {
		self.prefetch_on(
			id,
			generation,
			create_unix_ms,
			open_volume(client, id, create_unix_ms),
		)
		.await;
	}

	/// [`Sessions::prefetch`] with the volume open injected, so tests can count opens.
	async fn prefetch_on<F: std::future::Future<Output = Result<OpenVolume>>>(
		&self,
		id: Id,
		generation: u32,
		create_unix_ms: i64,
		open: impl FnOnce(Arc<dyn Fn() -> bool + Send + Sync>) -> F,
	) {
		#[allow(
			clippy::disallowed_methods,
			reason = "slow-open logging uses real time"
		)]
		let started = std::time::Instant::now();
		let result = async {
			let lane = self.lane(id).await?;
			self.prepare(&lane, generation, create_unix_ms, open)
				.await
				.map(|_| ())
		}
		.await;
		let elapsed = started.elapsed();
		// The start's first storage request waits behind this open, so a slow one stalls it.
		if elapsed >= std::time::Duration::from_secs(5) {
			tracing::warn!(
				%id,
				generation,
				elapsed_ms = elapsed.as_millis() as u64,
				ok = result.is_ok(),
				"slow actor database volume prefetch; the start's first storage request waits for it"
			);
		}
		if let Err(error) = result {
			tracing::debug!(%id, generation, ?error, "actor database volume prefetch failed; the first storage request opens it");
		}
	}

	/// Runs one actor KV request on the actor's database volume.
	pub async fn kv(
		&self,
		client: depot_client::Client,
		id: Id,
		generation: u32,
		create_unix_ms: i64,
		data: p::KvRequestData,
	) -> Result<p::KvResponseData> {
		let env = client.env().clone();
		self.kv_on(
			env,
			id,
			generation,
			create_unix_ms,
			data,
			open_volume(client, id, create_unix_ms),
		)
		.await
	}

	async fn kv_on<F: std::future::Future<Output = Result<OpenVolume>>>(
		&self,
		env: Arc<dyn rivet_env::Env>,
		id: Id,
		generation: u32,
		create_unix_ms: i64,
		data: p::KvRequestData,
		open: impl FnOnce(Arc<dyn Fn() -> bool + Send + Sync>) -> F,
	) -> Result<p::KvResponseData> {
		let lane = self.lane(id).await?;
		let (_actor, mut pages) = self
			.prepare(&lane, generation, create_unix_ms, open)
			.await?;
		let ino = pages.kv_ino().await?;
		let fs = pages.fs()?.clone();
		let result = crate::kv::execute(env.as_ref(), &fs, ino, data).await;
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
			anyhow::bail!("SQLite session capacity reached");
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

	/// Takes the lane for `generation` of the incarnation `create_unix_ms` and makes sure its
	/// database volume is open.
	async fn prepare<F: std::future::Future<Output = Result<OpenVolume>>>(
		&self,
		lane: &Arc<Lane>,
		generation: u32,
		create_unix_ms: i64,
		open: impl FnOnce(Arc<dyn Fn() -> bool + Send + Sync>) -> F,
	) -> Result<(
		tokio::sync::OwnedMutexGuard<Actor>,
		tokio::sync::OwnedMutexGuard<Pages>,
	)> {
		let mut actor = lane.actor.clone().lock_owned().await;
		lane.check()?;
		let mut pages = actor.pages.clone().lock_owned().await;
		// A lane's generation only moves forward. A late start or request of an older
		// generation would otherwise drop the live generation's staged commit and write with a
		// head the live generation has moved past, failing its next commit's head fence.
		ensure!(
			!actor.bound || wrapping_ge(generation, actor.generation),
			"storage request for an older actor generation"
		);
		// A fenced volume means another owner opened it, such as the actor's next generation
		// started elsewhere after this one was judged lost. The fence is terminal for the
		// generation that held it: reopening would fence the new owner in turn.
		if pages
			.volume
			.as_ref()
			.is_some_and(|volume| volume.fs.is_fenced())
		{
			let fenced = actor.fenced_through.map_or(actor.generation, |fenced| {
				wrapping_max(fenced, actor.generation)
			});
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
			if wrapping_le(generation, fenced) {
				return Err(anyhow::Error::new(page_protocol::HeadFenceMismatch)).with_context(
					|| {
						format!(
							"actor database volume was fenced for generation {fenced}; only a newer generation reopens it"
						)
					},
				);
			}
		}
		if !actor.bound || actor.generation != generation {
			// A new generation drops open SQL transactions and staged commits, retaining the
			// exclusive filesystem owner instead of claiming it on every request.
			actor.session = None;
			pages.stage = None;
			actor.generation = generation;
			actor.bound = true;
			pages.generation = generation;
		}
		actor.create_unix_ms = create_unix_ms;
		if pages.volume.is_none() {
			let volume = open(lane.guard()).await?;
			lane.install(&volume.fs)?;
			pages.volume = Some(volume);
		}
		Ok((actor, pages))
	}

	

	/// Keeps page recovery and execution inside one actor lane the caller already resolved,
	/// with the volume open injected so tests can supply their own I/O.
	#[allow(
		clippy::too_many_arguments,
		reason = "a request needs its lane, identity, generation, incarnation and open"
	)]
	async fn execute_in<F: std::future::Future<Output = Result<OpenVolume>>>(
		&self,
		lane: &Arc<Lane>,
		env: Arc<dyn rivet_env::Env>,
		id: Id,
		generation: u32,
		create_unix_ms: i64,
		message: p::ToRivet,
		open: impl FnOnce(Arc<dyn Fn() -> bool + Send + Sync>) -> F,
	) -> Result<p::ToEnvoy> {
		let span = tracing::trace_span!(target: "sqlite_prefetch::trace", "execute_in",
            actor_id = %id, generation, create_unix_ms);
		trace_scope::scope(span, async {
			let (mut actor, mut pages) =
				self.prepare(lane, generation, create_unix_ms, open).await?;
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
							Arc::downgrade(lane),
							format!("envoy-remote-sqlite-{}", env.rng("sqlite.vfs").uuid_v4()),
							id.to_string(),
							generation,
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

/// Opens the database volume of `id`'s incarnation `create_unix_ms` as its exclusive owner,
/// dispatching only while the lane lives.
fn open_volume(
	client: depot_client::Client,
	id: Id,
	create_unix_ms: i64,
) -> impl FnOnce(
	Arc<dyn Fn() -> bool + Send + Sync>,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<OpenVolume>> + Send>> {
	move |guard| {
		Box::pin(async move {
			let owner = retry_refused_open(client.env(), guard.clone(), || {
				database::open_volume(
					client.clone().with_dispatch_guard(guard.clone()),
					ActorDatabaseVolume::new(id, create_unix_ms).volume_key(),
				)
			})
			.await?;
			Ok(OpenVolume {
				fs: owner.filesystem().clone(),
				_owner: Some(owner),
			})
		})
	}
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
	fn new(env: &Arc<dyn rivet_env::Env>, label: &str) -> Result<Self> {
		let now_boot_ms = env.clock().observe()?.boot_ms;
		Self::starting_at(env, label, now_boot_ms)
	}

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

/// Sends an actor database open again while depot refuses it without running it; see
/// [`RefusalBackoff`]. An error that may have run, such as an unanswered open that carried
/// first writes, returns at once.
async fn retry_refused_open<T, F, Fut>(
	env: &Arc<dyn rivet_env::Env>,
	guard: Arc<dyn Fn() -> bool + Send + Sync>,
	mut open: F,
) -> Result<T>
where
	F: FnMut() -> Fut,
	Fut: std::future::Future<Output = Result<T>>,
{
	let mut backoff = RefusalBackoff::new(env, "pegboard_envoy.sqlite.open_refusal")?;
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
fn refused_write_hint_ms(error: &anyhow::Error) -> Option<u64> {
	let Some(depot_client::filesystem::Error::Log(error)) =
		error.downcast_ref::<depot_client::filesystem::Error>()
	else {
		return None;
	};
	if error.disposition() != ErrorDisposition::NotExecuted {
		return None;
	}
	error.retry_after_ms()
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


