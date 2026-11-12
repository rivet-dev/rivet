use std::{
	any::Any,
	sync::{Arc, OnceLock},
};

use anyhow::*;
use depot_client::Client as DepotClient;
use rivet_config::Config;
use rivet_data::generated::pegboard_namespace_v1 as ns;
use rivet_topology::topology::Topology;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use crate::{Error, NodeId, ObjectStores, UpsPool, depot::DepotPool};

/// A namespace's state as a process last read it.
#[derive(Clone, Debug)]
pub struct CachedNamespace {
	state: Arc<ns::NamespaceState>,
	/// When the record was read, on the boot clock.
	read_at_boot_ms: u64,
	/// The view of `state` a caller derives once per fill and every lookup of the same fill
	/// shares, so request paths never rebuild it: pegboard's namespace standing. Empty until the
	/// first lookup derives it, and replaced with the entry when the state is read again. Erased
	/// because the crate that derives it depends on this one; [`Self::derived_or_init`] is the
	/// only way to fill or read it, so it holds one type per process.
	derived: Arc<OnceLock<Arc<dyn Any + Send + Sync>>>,
	/// Whether a background re-read of this fill runs now, so a stale fill has at most one; see
	/// [`EngineContext::begin_namespace_refresh`]. A new fill starts with its own clear flag.
	refreshing: Arc<AtomicBool>,
	/// The boot time before which no background re-read of this fill starts, set when one
	/// failed so a failing namespace volume is not read again by every stale request; see
	/// [`NamespaceRefresh::hold_off_until`].
	refresh_held_until_boot_ms: Arc<AtomicU64>,
}

impl CachedNamespace {
	/// A fill of `state` read at `read_at_boot_ms`, with nothing derived yet.
	pub(crate) fn new(state: ns::NamespaceState, read_at_boot_ms: u64) -> Self {
		Self {
			state: Arc::new(state),
			read_at_boot_ms,
			derived: Default::default(),
			refreshing: Default::default(),
			refresh_held_until_boot_ms: Default::default(),
		}
	}

	/// The namespace's state as read.
	pub fn state(&self) -> &Arc<ns::NamespaceState> {
		&self.state
	}

	/// When the record was read, on the boot clock.
	pub fn read_at_boot_ms(&self) -> u64 {
		self.read_at_boot_ms
	}

	/// The view of this fill's state that `init` derives, derived on first use and shared by
	/// every later lookup of the same fill. One derived type per process: the slot is filled
	/// only through this method, by pegboard with its namespace standing. A slot holding another
	/// type is a bug, so it fails debug builds and answers an error instead of a value.
	pub fn derived_or_init<T: Any + Send + Sync>(
		&self,
		init: impl FnOnce(&ns::NamespaceState) -> T,
	) -> Result<Arc<T>> {
		let derived = self
			.derived
			.get_or_init(|| {
				let value: Arc<dyn Any + Send + Sync> = Arc::new(init(&self.state));
				value
			})
			.clone();
		match derived.downcast::<T>() {
			std::result::Result::Ok(value) => Ok(value),
			std::result::Result::Err(_) => {
				debug_assert!(false, "a namespace cache entry holds two derived types");
				Err(anyhow!(
					"namespace cache entry holds a derived value of another type"
				))
			}
		}
	}
}

pub struct EngineAdapters {
	pub fs: rivet_env::fs::Fs,
	pub object_stores: ObjectStores,
	pub http: Arc<dyn rivet_env::capabilities::http::HttpClient>,
	/// HTTP and WebSocket clients for servers outside the cluster.
	pub external_http: Arc<dyn rivet_env::capabilities::http::ExternalHttpClient>,
	pub http_server: Arc<dyn rivet_env::capabilities::http::HttpServer>,
	pub depot_databases: Arc<dyn rivet_env::capabilities::slatedb::SlateDbFactory>,
}

// Context clones share capabilities; EngineBackground owns process task lifetime.
pub(crate) struct EngineContextInner {
	pub(crate) env: Arc<dyn rivet_env::Env>,
	pub(crate) fs: rivet_env::fs::Fs,

	pub(crate) depot_databases: Arc<dyn rivet_env::capabilities::slatedb::SlateDbFactory>,
	pub(crate) shutdown: rivet_env::capabilities::shutdown::Shutdown,
	pub(crate) http: Arc<dyn rivet_env::capabilities::http::HttpClient>,
	pub(crate) external_http: Arc<dyn rivet_env::capabilities::http::ExternalHttpClient>,
	pub(crate) http_server: Arc<dyn rivet_env::capabilities::http::HttpServer>,
	pub(crate) config: Config,
	pub(crate) node_id: NodeId,
	pub(crate) worker_identity: rivet_topology::worker::WorkerIdentity,
	background_started: AtomicBool,
	/// Namespace states by name. Names never change or disappear, so an entry is permanent;
	/// its phase is re-read by the caller once it is older than the namespace status TTL.
	namespaces: scc::HashMap<String, CachedNamespace>,
	/// The namespace name behind each volume group id this process has resolved. A group's
	/// key never changes, so an entry is permanent.
	group_names: scc::HashMap<[u8; 16], String>,
	/// Legacy: the namespace and actor name pairs this process recorded for the backfill after a
	/// legacy fallback read, so each costs one write per process. Bounded by the caller's
	/// configured capacity. Removed in 3.1 with the backfill.
	legacy_backfill_names: scc::HashSet<(String, String)>,
	/// Legacy: the backfill name record writes this process runs in the background, bounded by
	/// the caller's configured cap. Removed in 3.1 with the backfill.
	legacy_backfill_name_writes: std::sync::atomic::AtomicUsize,
	/// Legacy: whether the warning that the name record writes reached their cap was logged and
	/// the writes have not drained to half the cap since, so sustained pressure warns once.
	/// Removed in 3.1 with the backfill.
	legacy_backfill_name_writes_warned: CapWarning,
	/// Legacy: owns the backfill name record writes, which `EngineBackground::shutdown` cancels
	/// and joins. Removed in 3.1 with the backfill.
	legacy_backfill_name_tasks: rivet_env::TaskScope,
	/// The background namespace status re-reads this process runs, bounded by the caller's
	/// configured cap; see [`EngineContext::begin_namespace_refresh`].
	namespace_refreshes: std::sync::atomic::AtomicUsize,
	/// Whether the warning that namespace status re-reads reached their cap was logged and the
	/// re-reads have not drained to half the cap since.
	namespace_refreshes_warned: CapWarning,
	/// Owns the background namespace status re-reads, which `EngineBackground::shutdown` cancels
	/// and joins.
	namespace_refresh_tasks: rivet_env::TaskScope,
	/// The boot time until which no request settles the group create of the creating namespace
	/// of each name: the deadline of the settle that runs, or the hold-off after one failed. Keyed
	/// by name, not by cache fill, since a settle replaces the fill it read. An entry leaves when
	/// a settle succeeds, so the map holds only namespaces whose create has not settled; see
	/// [`EngineContext::begin_namespace_settle`].
	namespace_settles: scc::HashMap<String, u64>,
	ingress: crate::ingress::IngressDrain,
	storage_readiness: depot_worker::services::readiness::Readiness,
	pub(crate) ups: Option<UpsPool>,
	pub(crate) object_stores: ObjectStores,
	pub(crate) depot: DepotPool,
	/// One Epoxy client shared by unscoped callers, so its membership cache and background
	/// tasks are not rebuilt for every operation.
	pub(crate) epoxy: std::sync::OnceLock<epoxy::client::Client>,
	pub(crate) jwt: rivet_auth_jwt::runtime::Cache,
	/// One cache budget for every filesystem this process opens, shared by the volume client
	/// and the storage worker's command host.
	pub(crate) filesystem_cache: Arc<depot_client::filesystem::CacheBudget>,
}

/// Legacy: whether a namespace's backfill is done, so it records no more actor names. Removed in
/// 3.1 with the backfill.
fn legacy_backfill_done(state: &ns::NamespaceState) -> bool {
	match state.backfill {
		ns::NamespaceBackfill::NamespaceBackfillDone(_) => true,
		ns::NamespaceBackfill::NamespaceBackfillPending(_)
		| ns::NamespaceBackfill::NamespaceBackfillRunning(_) => false,
	}
}

/// Legacy: what [`EngineContext::legacy_mark_backfill_name`] did. Removed in 3.1 with the
/// backfill.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LegacyNameMark {
	/// Newly marked: the caller records the name.
	Marked,
	/// Already marked: the name was recorded by this process.
	Recorded,
	/// Not marked because the capacity is reached: the caller records nothing and counts the
	/// skip.
	Full,
	/// Not marked because this process caches the namespace's backfill as done, so no page will
	/// walk the name: the caller records nothing.
	NamespaceDone,
}

/// Legacy: one of the `capacity` slots of [`EngineContext::legacy_begin_name_write`], freed
/// when dropped. Removed in 3.1 with the backfill.
pub struct LegacyNameWrite {
	ctx: EngineContext,
	capacity: usize,
}

impl Drop for LegacyNameWrite {
	fn drop(&mut self) {
		let inner = &self.ctx.0;
		let before = inner
			.legacy_backfill_name_writes
			.fetch_sub(1, std::sync::atomic::Ordering::AcqRel);
		inner
			.legacy_backfill_name_writes_warned
			.released(before.saturating_sub(1), self.capacity);
	}
}

/// What [`EngineContext::begin_namespace_refresh`] did.
pub enum NamespaceRefreshBegin {
	/// The caller runs the re-read and holds the slot until it ends.
	Started(NamespaceRefresh),
	/// A re-read of the same fill already runs.
	InFlight,
	/// A re-read of the same fill failed recently and its hold-off has not ended: the caller
	/// re-reads nothing, and a request after the hold-off tries again.
	HeldOff,
	/// Every slot is taken: the caller re-reads nothing and counts the skip. A later request
	/// past the TTL tries again.
	Full,
}

/// One background re-read of a namespace's status: one of the `capacity` slots and the fill's
/// in-flight flag, both freed when dropped, so a re-read that fails or is cancelled lets the next
/// stale request start another.
pub struct NamespaceRefresh {
	ctx: EngineContext,
	refreshing: Arc<AtomicBool>,
	refresh_held_until_boot_ms: Arc<AtomicU64>,
	capacity: usize,
}

impl NamespaceRefresh {
	/// Whether this re-read took the last slot while the cap warning was armed, so the caller
	/// warns once for sustained pressure.
	pub fn reached_cap(&self) -> bool {
		let inner = &self.ctx.0;
		let in_flight = inner.namespace_refreshes.load(Ordering::Acquire);
		inner
			.namespace_refreshes_warned
			.reached(in_flight, self.capacity)
	}

	/// Holds off the fill's next background re-read until `boot_ms`, after this one failed.
	/// Call before dropping the refresh, so no stale request starts another read in between.
	pub fn hold_off_until(&self, boot_ms: u64) {
		self.refresh_held_until_boot_ms
			.store(boot_ms, Ordering::Release);
	}
}

impl Drop for NamespaceRefresh {
	fn drop(&mut self) {
		self.refreshing.store(false, Ordering::Release);
		let inner = &self.ctx.0;
		let before = inner.namespace_refreshes.fetch_sub(1, Ordering::AcqRel);
		inner
			.namespace_refreshes_warned
			.released(before.saturating_sub(1), self.capacity);
	}
}

/// The hysteresis of a warning that bounded background work reached its cap: it fires once when
/// a task takes the last slot, and re-arms only once a finishing task leaves the work drained to
/// half the cap or below, so sustained pressure warns once and a cap of 1 still warns again after
/// it drained.
#[derive(Default)]
pub(crate) struct CapWarning(AtomicBool);

impl CapWarning {
	/// Whether a write that took a slot with `in_flight` writes running, itself included, of
	/// `capacity`, warns now.
	pub(crate) fn reached(&self, in_flight: usize, capacity: usize) -> bool {
		in_flight >= capacity && !self.0.swap(true, Ordering::AcqRel)
	}

	/// Re-arms the warning when a finishing write left `in_flight` writes running, of
	/// `capacity`, at half the cap or below.
	pub(crate) fn released(&self, in_flight: usize, capacity: usize) {
		if in_flight <= capacity / 2 {
			self.0.store(false, Ordering::Release);
		}
	}
}

/// [`EngineContext::begin_namespace_settle`] over `settles`.
pub(crate) async fn begin_settle_in(
	settles: &scc::HashMap<String, u64>,
	name: &str,
	now_boot_ms: u64,
	deadline_boot_ms: u64,
) -> bool {
	match settles.entry_async(name.to_owned()).await {
		scc::hash_map::Entry::Occupied(mut occupied) => {
			if *occupied.get() > now_boot_ms {
				return false;
			}
			*occupied.get_mut() = deadline_boot_ms;
			true
		}
		scc::hash_map::Entry::Vacant(vacant) => {
			vacant.insert_entry(deadline_boot_ms);
			true
		}
	}
}

/// [`EngineContext::end_namespace_settle`] over `settles`.
pub(crate) async fn end_settle_in(
	settles: &scc::HashMap<String, u64>,
	name: &str,
	held_off_until_boot_ms: Option<u64>,
) {
	match held_off_until_boot_ms {
		Some(until) => match settles.entry_async(name.to_owned()).await {
			scc::hash_map::Entry::Occupied(mut occupied) => *occupied.get_mut() = until,
			scc::hash_map::Entry::Vacant(vacant) => {
				vacant.insert_entry(until);
			}
		},
		None => {
			settles.remove_async(name).await;
		}
	}
}

/// Legacy: marks `namespace` and `name` in `names`, the pairs recorded for the backfill; see
/// [`EngineContext::legacy_mark_backfill_name`]. A namespace `namespaces` already caches as done
/// is never marked, so a read holding a standing from before the namespace finished cannot mark
/// a name again after the finish pruned its marks. The done check and the mark run under the
/// namespace's cache entry, and [`EngineContext::remember_namespace`] prunes a finished
/// namespace's marks only after it replaced that entry, so a mark is either pruned or refused,
/// never left behind. Removed in 3.1 with the backfill.
pub(crate) async fn legacy_mark_name_in(
	namespaces: &scc::HashMap<String, CachedNamespace>,
	names: &scc::HashSet<(String, String)>,
	namespace: &str,
	name: &str,
	capacity: usize,
) -> LegacyNameMark {
	let pair = (namespace.to_owned(), name.to_owned());
	if names.contains_async(&pair).await {
		return LegacyNameMark::Recorded;
	}

	// Hold the namespace's entry from the done check through the insert.
	let entry = namespaces.entry_async(namespace.to_owned()).await;
	let done = match &entry {
		scc::hash_map::Entry::Occupied(occupied) => legacy_backfill_done(&occupied.get().state),
		scc::hash_map::Entry::Vacant(_) => false,
	};
	let mark = if done {
		LegacyNameMark::NamespaceDone
	} else if names.len() >= capacity {
		LegacyNameMark::Full
	} else {
		match names.insert_async(pair).await {
			std::result::Result::Ok(()) => LegacyNameMark::Marked,
			std::result::Result::Err(_) => LegacyNameMark::Recorded,
		}
	};
	drop(entry);
	mark
}

#[derive(Clone)]
pub struct EngineContext(
	Arc<EngineContextInner>,
	Option<Arc<crate::singleton_scope::SingletonScope>>,
	/// A request's absolute typed-command deadline, inherited by nested lookup helpers.
	Option<u64>,
);

impl EngineContext {
	pub fn jwt(&self) -> &rivet_auth_jwt::runtime::Cache {
		&self.0.jwt
	}
	#[tracing::instrument(skip_all)]
	pub async fn new(
		env: Arc<dyn rivet_env::Env>,
		shutdown: rivet_env::capabilities::shutdown::Shutdown,
		config: Config,
		adapters: EngineAdapters,
	) -> Result<EngineContext> {
		let worker_identity =
			rivet_topology::worker::WorkerIdentity::new(&mut env.rng("depot.identity"));
		Self::new_with_identity(env, shutdown, config, adapters, worker_identity).await
	}

	/// Uses the identity selected by bootstrap before constructing worker-addressed clients.
	/// Storage bootstrap restores the folder's worker ID and draws a fresh process incarnation.
	#[tracing::instrument(skip_all)]
	pub async fn new_with_identity(
		env: Arc<dyn rivet_env::Env>,
		shutdown: rivet_env::capabilities::shutdown::Shutdown,
		config: Config,
		adapters: EngineAdapters,
		worker_identity: rivet_topology::worker::WorkerIdentity,
	) -> Result<EngineContext> {
		let EngineAdapters {
			fs,
			object_stores,
			http,
			external_http,
			http_server,
			depot_databases,
		} = adapters;

		let node_id = NodeId::new(&mut env.rng("process.node_id"));

		let topology = Topology::new(
			env.clone(),
			object_stores.lease.clone(),
			rivet_topology::TopologyConfig {
				endpoint_capacity: (config.depot()).endpoint_cache_capacity(),
				lookup_request_capacity: config.depot().lookup_request_capacity(),
				..Default::default()
			},
		)?;

		let ups = crate::db::ups::setup(
			env.clone(),
			&config,
			&worker_identity,
			topology.clone(),
			http.clone(),
		)
		.await?;

		let mut depot_client = DepotClient::new(ups.clone(), topology.clone()).with_regions(
			Arc::new(depot_client::regional::Regions::new(
				config.topology().clone(),
				http.clone(),
				env.clock(),
			)),
		);
		if let Some(floor) = config.depot().command_timeout_floor() {
			depot_client = depot_client.with_command_timeout_floor(floor);
		}
		depot_client =
			depot_client.with_max_rollback_bytes(config.depot().filesystem.max_rollback_bytes);
		let depot = DepotPool {
			ready: Default::default(),
			volumes: Default::default(),
			service: Default::default(),
			client: depot_client,
			topology,
		};
		let filesystem_cache = depot_client::filesystem::CacheBudget::new(
			config.depot().filesystem.max_process_cache_bytes,
		);
		let legacy_backfill_name_tasks = rivet_env::TaskScope::new(env.clone());
		let namespace_refresh_tasks = rivet_env::TaskScope::new(env.clone());
		let context = EngineContext(
			Arc::new(EngineContextInner {
				jwt: rivet_auth_jwt::runtime::Cache::new(
					env.clone(),
					object_stores.lease.clone(),
					config.clone(),
				),
				fs,
				depot_databases,
				http,
				external_http,
				http_server,
				env,
				shutdown,
				config,
				node_id,
				worker_identity,
				background_started: AtomicBool::new(false),
				namespaces: Default::default(),
				group_names: Default::default(),
				legacy_backfill_names: Default::default(),
				legacy_backfill_name_writes: Default::default(),
				legacy_backfill_name_writes_warned: CapWarning::default(),
				legacy_backfill_name_tasks,
				namespace_refreshes: Default::default(),
				namespace_settles: Default::default(),
				namespace_refreshes_warned: CapWarning::default(),
				namespace_refresh_tasks,
				ingress: Default::default(),
				storage_readiness: Default::default(),
				ups: Some(ups),
				object_stores,
				depot,
				epoxy: std::sync::OnceLock::new(),
				filesystem_cache,
			}),
			None,
			None,
		);

		Ok(context)
	}

	/// Starts process services once and returns their cancellation/drain owner.
	pub async fn start_background(
		&self,
		registration: Option<rivet_topology::WorkerRegistration>,
	) -> Result<EngineBackground> {
		ensure!(
			self.1.is_none(),
			"a singleton scope cannot own process background startup"
		);
		ensure!(
			!self.0.background_started.swap(true, Ordering::AcqRel),
			"Engine background services already started"
		);
		let owner = EngineBackground(self.clone());
		let participates = registration.is_some();
		self.0.depot.topology.start(registration).await?;
		if participates {
			let service = crate::services::depot_client::start(
				self.0.config.depot(),
				self.ups()?,
				self.topology(),
				self.object_stores().primary.clone(),
				self.filesystem_cache(),
			)?;
			self.0
				.depot
				.volumes
				.set(service.client.clone())
				.map_err(|_| anyhow!("volume client already installed"))?;
			*self.0.depot.service.lock().await = Some(service);
			self.0.depot.ready.notify_waiters();
		}
		Ok(owner)
	}

	/// The cached state of the namespace `name`, with when it was read.
	pub async fn cached_namespace(&self, name: &str) -> Option<CachedNamespace> {
		self.0
			.namespaces
			.read_async(name, |_, entry| entry.clone())
			.await
	}

	/// The namespace name this process resolved for the volume group `group`, if any.
	pub async fn cached_group_name(&self, group: &[u8; 16]) -> Option<String> {
		self.0
			.group_names
			.read_async(group, |_, name| name.clone())
			.await
	}

	pub async fn remember_group_name(&self, group: [u8; 16], name: String) {
		let _ = self.0.group_names.insert_async(group, name).await;
	}

	/// Legacy: marks `namespace` and `name` recorded for the backfill. Answers `Recorded` when
	/// they already were, `Full` when `capacity` pairs are held, and `NamespaceDone` when this
	/// process caches the namespace's backfill as done; only `Marked` asks the caller to record
	/// the name. Removed in 3.1 with the backfill.
	pub async fn legacy_mark_backfill_name(
		&self,
		namespace: &str,
		name: &str,
		capacity: usize,
	) -> LegacyNameMark {
		legacy_mark_name_in(
			&self.0.namespaces,
			&self.0.legacy_backfill_names,
			namespace,
			name,
			capacity,
		)
		.await
	}

	/// Legacy: takes one of `capacity` slots for a backfill name record write, or `None` when
	/// every slot is taken. Removed in 3.1 with the backfill.
	pub fn legacy_begin_name_write(&self, capacity: usize) -> Option<LegacyNameWrite> {
		let taken = self.0.legacy_backfill_name_writes.try_update(
			std::sync::atomic::Ordering::AcqRel,
			std::sync::atomic::Ordering::Acquire,
			|writes| match writes < capacity {
				true => Some(writes + 1),
				false => None,
			},
		);
		match taken {
			std::result::Result::Ok(_) => Some(LegacyNameWrite {
				ctx: self.clone(),
				capacity,
			}),
			std::result::Result::Err(_) => None,
		}
	}

	/// Legacy: whether the write that took a slot with `in_flight` writes running, of
	/// `capacity`, should warn that the writes reached their cap: once when they reach it, and
	/// again only after a finishing write left them drained to half of it (see
	/// [`CapWarning`]). Removed in 3.1 with the backfill.
	pub fn legacy_name_writes_reached_cap(&self, in_flight: usize, capacity: usize) -> bool {
		self.0
			.legacy_backfill_name_writes_warned
			.reached(in_flight, capacity)
	}

	/// Legacy: how many backfill name record writes run now. Removed in 3.1 with the backfill.
	pub fn legacy_name_writes_in_flight(&self) -> usize {
		self.0.legacy_backfill_name_writes.load(Ordering::Acquire)
	}

	/// Legacy: runs a backfill name record write in the scope this context owns, which the
	/// process's background shutdown cancels and joins. Removed in 3.1 with the backfill.
	pub fn legacy_spawn_name_write(
		&self,
		write: impl std::future::Future<Output = ()> + Send + 'static,
	) {
		self.0.legacy_backfill_name_tasks.spawn(write);
	}

	/// Legacy: forgets a mark whose record write failed, so the next read records it again.
	/// Removed in 3.1 with the backfill.
	pub async fn legacy_unmark_backfill_name(&self, namespace: &str, name: &str) {
		let pair = (namespace.to_owned(), name.to_owned());
		self.0.legacy_backfill_names.remove_async(&pair).await;
	}

	/// Legacy: how many namespace and actor name pairs this process holds as recorded. Removed in
	/// 3.1 with the backfill.
	pub fn legacy_backfill_names_held(&self) -> usize {
		self.0.legacy_backfill_names.len()
	}

	/// Takes the in-flight flag of `cached`, a stale fill, and one of `capacity` slots for its
	/// background re-read at `now_boot_ms`. Answers `HeldOff` until a failed re-read's hold-off
	/// ends, `InFlight` while another re-read of the same fill runs and `Full` when every slot is
	/// taken, so each stale fill costs at most one read at a time and the process at most
	/// `capacity`.
	/// Takes the settle of the creating namespace `name` at `now_boot_ms`, for a settle that
	/// runs until `deadline_boot_ms` at the latest. Answers `false` while another settle of the
	/// name runs or a failed one's hold-off has not ended. A settle whose request is dropped
	/// frees the name at its deadline, so nothing has to release it.
	pub async fn begin_namespace_settle(
		&self,
		name: &str,
		now_boot_ms: u64,
		deadline_boot_ms: u64,
	) -> bool {
		begin_settle_in(
			&self.0.namespace_settles,
			name,
			now_boot_ms,
			deadline_boot_ms,
		)
		.await
	}

	/// Ends the settle of `name` that [`Self::begin_namespace_settle`] took: a namespace that
	/// settled frees its name, and a settle that failed holds the next one off until
	/// `held_off_until_boot_ms`.
	pub async fn end_namespace_settle(&self, name: &str, held_off_until_boot_ms: Option<u64>) {
		end_settle_in(&self.0.namespace_settles, name, held_off_until_boot_ms).await
	}

	pub fn begin_namespace_refresh(
		&self,
		cached: &CachedNamespace,
		capacity: usize,
		now_boot_ms: u64,
	) -> NamespaceRefreshBegin {
		if now_boot_ms < cached.refresh_held_until_boot_ms.load(Ordering::Acquire) {
			return NamespaceRefreshBegin::HeldOff;
		}
		if cached.refreshing.swap(true, Ordering::AcqRel) {
			return NamespaceRefreshBegin::InFlight;
		}
		// A re-read that failed between the check above and the swap set its hold-off before it
		// freed the flag, so the hold-off is checked again while this caller holds the flag.
		if now_boot_ms < cached.refresh_held_until_boot_ms.load(Ordering::Acquire) {
			cached.refreshing.store(false, Ordering::Release);
			return NamespaceRefreshBegin::HeldOff;
		}
		let taken = self.0.namespace_refreshes.try_update(
			Ordering::AcqRel,
			Ordering::Acquire,
			|refreshes| match refreshes < capacity {
				true => Some(refreshes + 1),
				false => None,
			},
		);
		match taken {
			std::result::Result::Ok(_) => NamespaceRefreshBegin::Started(NamespaceRefresh {
				ctx: self.clone(),
				refreshing: cached.refreshing.clone(),
				refresh_held_until_boot_ms: cached.refresh_held_until_boot_ms.clone(),
				capacity,
			}),
			std::result::Result::Err(_) => {
				cached.refreshing.store(false, Ordering::Release);
				NamespaceRefreshBegin::Full
			}
		}
	}

	/// Runs a background namespace status re-read in the scope this context owns, which the
	/// process's background shutdown cancels and joins.
	pub fn spawn_namespace_refresh(
		&self,
		refresh: impl std::future::Future<Output = ()> + Send + 'static,
	) {
		self.0.namespace_refresh_tasks.spawn(refresh);
	}

	/// Remembers `state` as the namespace's state read at `read_at_boot_ms`, replacing an entry
	/// read earlier, and answers the entry the cache holds afterwards. An entry read later is
	/// kept: a re-read that started before another path remembered a newer state, such as a
	/// fork's wait remembering `Live`, finishes with an older state and must not replace it.
	pub async fn remember_namespace(
		&self,
		name: &str,
		state: ns::NamespaceState,
		read_at_boot_ms: u64,
	) -> CachedNamespace {
		let entry = CachedNamespace::new(state, read_at_boot_ms);
		let done = legacy_backfill_done(&entry.state);
		let was_done = match self.0.namespaces.entry_async(name.to_owned()).await {
			scc::hash_map::Entry::Occupied(mut occupied) => {
				if occupied.get().read_at_boot_ms > read_at_boot_ms {
					return occupied.get().clone();
				}
				let was_done = legacy_backfill_done(&occupied.get().state);
				*occupied.get_mut() = entry.clone();
				was_done
			}
			scc::hash_map::Entry::Vacant(vacant) => {
				vacant.insert_entry(entry.clone());
				false
			}
		};

		// Legacy: a namespace seen done records no more names, so its marks leave room for
		// other namespaces. Pruned after the entry is replaced and released: a mark checks the
		// entry under its lock, so once the entry says done no mark lands, and every mark that
		// landed before is in the set the prune walks. The walk holds no namespace entry, so
		// lookups of other namespaces never wait on it. Removed in 3.1 with the backfill.
		if done && !was_done {
			self.0
				.legacy_backfill_names
				.retain_async(|(namespace, _)| namespace != name)
				.await;
		}
		entry
	}

	pub fn env(&self) -> &Arc<dyn rivet_env::Env> {
		&self.0.env
	}

	/// The process filesystem cache budget shared by every Depot filesystem owner.
	pub fn filesystem_cache(&self) -> Arc<depot_client::filesystem::CacheBudget> {
		self.0.filesystem_cache.clone()
	}

	/// Own accepted ingress until its HTTP bodies and runner connections finish.
	pub fn ingress(&self) -> &crate::ingress::IngressDrain {
		&self.0.ingress
	}

	/// Storage owns publication; HTTP readiness also checks the current topology lease.
	pub fn storage_readiness(&self) -> &depot_worker::services::readiness::Readiness {
		&self.0.storage_readiness
	}

	/// Readiness follows this process's selected role and is revoked from the first
	/// shutdown request, even while storage remains available for a graceful drain.
	pub fn is_ready(&self) -> bool {
		if self.shutdown().is_requested() {
			return false;
		}
		let topology = self.topology();
		let std::result::Result::Ok(snapshot) = topology.snapshot() else {
			return false;
		};
		let std::result::Result::Ok(now) = self.env().clock().observe() else {
			return false;
		};
		if !snapshot.may_admit(now.boot_ms) {
			return false;
		}
		match snapshot
			.worker_registration
			.as_ref()
			.map(|registration| registration.role)
		{
			Some(rivet_topology::worker::WorkerRole::Storage) => {
				self.storage_readiness().is_ready()
			}
			Some(rivet_topology::worker::WorkerRole::Stateless) => true,
			None => false,
		}
	}

	pub fn shutdown(&self) -> rivet_env::capabilities::shutdown::Shutdown {
		self.0.shutdown.clone()
	}

	pub fn depot_databases(&self) -> Arc<dyn rivet_env::capabilities::slatedb::SlateDbFactory> {
		self.0.depot_databases.clone()
	}

	pub fn http(&self) -> Arc<dyn rivet_env::capabilities::http::HttpClient> {
		self.1
			.as_ref()
			.map_or_else(|| self.0.http.clone(), |scope| scope.http.clone())
	}

	pub fn fs(&self) -> rivet_env::fs::Fs {
		self.0.fs.clone()
	}

	/// Clients for servers outside the cluster, such as the V2 deployment a migration from V2
	/// proxies to.
	pub fn external_http(&self) -> Arc<dyn rivet_env::capabilities::http::ExternalHttpClient> {
		self.0.external_http.clone()
	}

	pub fn http_server(&self) -> Arc<dyn rivet_env::capabilities::http::HttpServer> {
		self.0.http_server.clone()
	}

	// MARK: Getters
	pub fn config(&self) -> &Config {
		&self.0.config
	}

	pub fn node_id(&self) -> NodeId {
		self.0.node_id
	}

	/// Select scope-bound clients without changing process identity, Topology or ordinary clients.
	/// The coordinator runtime remains the sole owner of the shared election/lease.
	pub fn with_singleton_scope(
		&self,
		authority: rivet_singleton_coordinator::authority::Authority,
	) -> Result<Self> {
		ensure!(self.1.is_none(), "singleton scopes cannot be nested");
		ensure!(authority.permits(), "singleton authority already ended");
		let scope =
			crate::singleton_scope::SingletonScope::new(authority, self.depot(), self.http())?;
		Ok(Self(self.0.clone(), Some(Arc::new(scope)), self.2))
	}
	pub fn singleton_authority(
		&self,
	) -> Result<&rivet_singleton_coordinator::authority::Authority> {
		self.1
			.as_ref()
			.map(|scope| &scope.authority)
			.ok_or_else(|| anyhow!("service requires an elected singleton scope"))
	}
	/// Call after scoped service futures have been canceled and joined. Ordinary clients remain active.
	pub async fn close_singleton_scope(&self) -> Result<()> {
		self.1.as_ref().context("not a singleton scope")?.close();
		Ok(())
	}

	// MARK: Pool lookups
	pub fn ups(&self) -> Result<UpsPool> {
		self.0.ups.clone().ok_or(Error::MissingUpsPool.into())
	}

	pub fn worker_identity(&self) -> &rivet_topology::worker::WorkerIdentity {
		&self.0.worker_identity
	}

	/// The process's Epoxy client. A singleton scope dispatches through its own Depot guard and
	/// HTTP client, so it builds a scoped client instead of sharing the process one.
	pub fn epoxy(&self) -> Result<epoxy::client::Client> {
		// Before startup installs volume access, the Depot client is incomplete; do not cache
		// a client built from it.
		if self.1.is_some() || self.2.is_some() || self.0.depot.volumes.get().is_none() {
			return epoxy::client::Client::with_depot(self.config(), self.depot(), self.http());
		}
		if let Some(client) = self.0.epoxy.get() {
			return Ok(client.clone());
		}

		let client = epoxy::client::Client::with_depot(self.config(), self.depot(), self.http())?;
		Ok(self.0.epoxy.get_or_init(|| client).clone())
	}

	pub fn depot(&self) -> DepotClient {
		let client = self.1.as_ref().map_or_else(
			|| {
				if let Some(volumes) = self.0.depot.volumes.get() {
					// The service is constructed from this context's UPS identity.
					self.0
						.depot
						.client
						.with_volumes(volumes.clone())
						.expect("shared worker identity")
				} else {
					self.0.depot.client.clone()
				}
			},
			|scope| scope.depot.clone(),
		);
		match self.2 {
			Some(deadline) => client.with_command_deadline(deadline),
			None => client,
		}
	}

	/// Caps every typed Depot command in a request, including commands from nested helpers.
	/// Clones share the original absolute deadline; nested scopes cannot extend it.
	pub fn with_depot_command_deadline(&self, deadline_boot_ms: u64) -> Self {
		Self(
			self.0.clone(),
			self.1.clone(),
			Some(
				self.2
					.map_or(deadline_boot_ms, |previous| previous.min(deadline_boot_ms)),
			),
		)
	}

	/// Waits for process startup to install volume access before a peer service captures a client.
	/// This does not start services; the caller must cancel the wait if startup is aborted.
	pub async fn wait_depot_ready(&self) -> DepotClient {
		loop {
			let ready = self.0.depot.ready.notified();
			if self.0.depot.volumes.get().is_some() {
				return self.depot();
			}
			ready.await;
		}
	}

	/// Closes process-owned openers while UPS and the manifest watcher can still send requests.
	pub async fn close_depot_clients(&self) -> Result<()> {
		ensure!(
			self.1.is_none(),
			"a singleton scope cannot close process clients"
		);
		let mut service = self.0.depot.service.lock().await;
		if let Some(service) = service.as_mut() {
			service.close().await?;
		}
		service.take();
		Ok(())
	}

	pub fn topology(&self) -> Arc<Topology> {
		self.0.depot.topology.clone()
	}

	pub fn object_stores(&self) -> &ObjectStores {
		&self.0.object_stores
	}
}

/// Process lifetime guard. Dropping it cancels tasks even while context clones survive.
pub struct EngineBackground(EngineContext);

impl EngineBackground {
	pub async fn shutdown(self) -> Result<()> {
		// Legacy: backfill name record writes are best effort, so they are cancelled, not
		// awaited to completion. Removed in 3.1 with the backfill.
		self.0.0.legacy_backfill_name_tasks.shutdown().await;
		// Namespace status re-reads only refresh a cache, so they are cancelled too.
		self.0.0.namespace_refresh_tasks.shutdown().await;
		if let Some(epoxy) = self.0.0.epoxy.get() {
			epoxy.shutdown().await?;
		}
		self.0.close_depot_clients().await?;
		// The worker releases its heartbeat after closing storage. If it did not,
		// this background guard cannot certify cleanup and must leave the lease to expire.
		let topology = self.0.topology();
		let _ = topology
			.fence(rivet_topology::FenceReason::DependentFailed)
			.await;
		topology.shutdown().await
	}
}

impl Drop for EngineBackground {
	fn drop(&mut self) {
		let topology = self.0.topology();
		let ctx = self.0.clone();
		self.0.env().spawn(Box::pin(async move {
			// Drop cannot confirm dependent cleanup; leave the heartbeat to expire.
			let _ = topology
				.fence(rivet_topology::FenceReason::DependentFailed)
				.await;
			if let Err(error) = ctx.close_depot_clients().await {
				tracing::warn!(%error, "depot client shutdown failed");
			}
			if let Err(error) = topology.shutdown().await {
				tracing::warn!(%error, "topology shutdown failed");
			}
		}));
	}
}
