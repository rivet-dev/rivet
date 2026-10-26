//! Executor locks of one tablet filesystem store: a tablet lock and a fixed table of
//! volume stripes, both `tokio::sync::RwLock`.
//!
//! The maintainer core is the authority for the work it admits, and its footprints
//! already exclude every conflicting pair on one volume. In the default `core` mode,
//! work the core orders (applies, loads, session opens, lifecycle lookups and the flush
//! retention publish) takes only the tablet lock, and debug builds check it against the
//! footprint matrix instead. Checkpoint-session opens (held-image reads, base chunk reads
//! and fork metadata pages) are not ordered by the core, so they and the writers that
//! can collect their images (hold releases, checkpoint deletions, sweeps and a flush
//! stage's retention deletions) still take the volume stripe. The `stripes` mode keeps a
//! stripe on every operation.
//!
//! Rules every caller follows:
//!
//! 1. Take the tablet lock first, shared, or exclusive for open, close and direct
//!    administrative callers. Then take every stripe the operation needs in one call,
//!    in ascending stripe order and deduplicated, writing a stripe if any of its
//!    volumes is written. `lock` does both.
//! 2. Never hold any lock across an object-store read, and a SlateDB read can be one.
//!    Read first and lock for the in-memory check and the write, or drop the guard across
//!    the read and reacquire through `lock`, so a holder never waits for the tablet lock
//!    behind a queued exclusive waiter.
//! 3. Checkpoint-session opens take their stripe shared; every writer that takes a
//!    stripe takes it exclusively.
//!
//! A striped table has a fixed size, set by `depot.store_volume_lock_stripes`, and no
//! insert or remove races; two volumes on one stripe only add a short wait. Requests
//! carry a volume's stripe hash, and the table maps it to a stripe when it locks.

use super::store::LockOperation;
#[cfg(debug_assertions)]
use crate::tablet_maintainer_machine::state::footprint::compatible;
pub(crate) use crate::tablet_maintainer_machine::state::footprint::{StoreVolume, VolumeAccess};
use depot_types::{PartitionKeyHash, VolumeKey};
use rivet_metrics::{GaugeGuardExt, IntGaugeGuard};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

/// Stripes per tablet store when `depot.store_volume_lock_stripes` is unset.
pub(crate) const DEFAULT_VOLUME_LOCK_STRIPES: usize = 1024;

/// Which store operations take volume stripes, from `depot.store_volume_locks`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum VolumeLockMode {
	/// Work the maintainer core orders takes only the tablet lock.
	#[default]
	Core,
	/// Every operation takes its volumes' stripes.
	Stripes,
}

/// How an operation holds the tablet lock.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TabletMode {
	Shared,
	#[cfg_attr(
		not(test),
		allow(dead_code, reason = "taken by direct administrative callers and tests")
	)]
	Exclusive,
}

/// The locks one store operation needs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LockRequest {
	pub tablet: TabletMode,
	/// Stripe hashes taken shared unless also written.
	pub reads: Vec<u64>,
	/// Stripe hashes taken exclusively.
	pub writes: Vec<u64>,
}

impl LockRequest {
	/// The whole store, and no stripe.
	#[cfg_attr(
		not(test),
		allow(dead_code, reason = "taken by direct administrative callers and tests")
	)]
	pub(crate) fn exclusive() -> Self {
		Self {
			tablet: TabletMode::Exclusive,
			reads: Vec::new(),
			writes: Vec::new(),
		}
	}

	/// The tablet lock shared and no stripe yet.
	pub(crate) fn shared() -> Self {
		Self {
			tablet: TabletMode::Shared,
			reads: Vec::new(),
			writes: Vec::new(),
		}
	}

	pub(crate) fn read(mut self, stripe: u64) -> Self {
		self.reads.push(stripe);
		self
	}

	pub(crate) fn write(mut self, stripe: u64) -> Self {
		self.writes.push(stripe);
		self
	}

	/// Each stripe of a table of `count` once, ascending, written if any request writes
	/// it. Two hashes that map to one stripe take it once.
	fn stripes(&self, count: usize) -> Vec<(usize, bool)> {
		let index = |hash: &u64| (*hash % count as u64) as usize;
		let mut stripes = std::collections::BTreeMap::<usize, bool>::new();
		for stripe in &self.reads {
			stripes.entry(index(stripe)).or_insert(false);
		}
		for stripe in &self.writes {
			stripes.insert(index(stripe), true);
		}
		stripes.into_iter().collect()
	}
}

/// The stripe hash of a volume, from its partition key hash and name. A store maps it
/// to one of its stripes.
pub(crate) fn stripe_of(hash: &PartitionKeyHash, name: Option<&[u8]>) -> u64 {
	// FNV-1a over the already uniform hash and the name.
	let mut value: u64 = 0xcbf2_9ce4_8422_2325;
	for byte in hash.iter().chain(name.unwrap_or_default()) {
		value ^= u64::from(*byte);
		value = value.wrapping_mul(0x0000_0100_0000_01b3);
	}
	// A name of `None` must differ from an empty name.
	if name.is_none() {
		value ^= 0xff;
		value = value.wrapping_mul(0x0000_0100_0000_01b3);
	}
	value
}

/// The stripe hash of a volume key.
pub(crate) fn stripe(key: &VolumeKey) -> u64 {
	stripe_of(&depot_types::partition_key_hash(key), key.name.as_deref())
}

/// The stripe hash of a storage scope.
pub(crate) fn scope_stripe(scope: &depot_fs_core::keys::slatedb::Scope) -> u64 {
	stripe_of(&scope.hash, scope.name.as_deref())
}

impl LockRequest {
	/// The tablet lock shared and both of a fork's volume stripes written. Fork steps
	/// read and write rows of the destination and holds of the parent.
	pub(crate) fn fork(spec: &rivet_data::generated::shared_depot_fs_v1::FsForkSpec) -> Self {
		Self::shared()
			.write(stripe(&spec.parent_volume_key))
			.write(stripe(&spec.destination_volume_key))
	}

	/// The tablet lock shared and one volume's stripe written.
	pub(crate) fn volume(key: &VolumeKey) -> Self {
		Self::shared().write(stripe(key))
	}

	/// The tablet lock shared and one scope's stripe written.
	pub(crate) fn scope(scope: &depot_fs_core::keys::slatedb::Scope) -> Self {
		Self::shared().write(scope_stripe(scope))
	}
}

enum TabletGuard<'a> {
	Shared(#[allow(dead_code, reason = "held for its drop")] RwLockReadGuard<'a, ()>),
	Exclusive(#[allow(dead_code, reason = "held for its drop")] RwLockWriteGuard<'a, ()>),
}

enum StripeGuard<'a> {
	Read(#[allow(dead_code, reason = "held for its drop")] RwLockReadGuard<'a, ()>),
	Write(#[allow(dead_code, reason = "held for its drop")] RwLockWriteGuard<'a, ()>),
}

/// Held locks of one operation. Stripes are released before the tablet lock.
pub(crate) struct StoreGuard<'a> {
	_stripes: Vec<StripeGuard<'a>>,
	_tablet: TabletGuard<'a>,
	held: &'a StripeUsage,
	/// Counts these stripes on `FS_STORE_LOCK_STRIPES_HELD` while they are held.
	_held_metric: Option<IntGaugeGuard>,
}

impl Drop for StoreGuard<'_> {
	fn drop(&mut self) {
		self.held.release(self._stripes.len());
	}
}

/// Stripes one store has held, for the process-wide usage gauge and a warning when
/// most of its table is held at once.
struct StripeUsage {
	held: AtomicUsize,
	capacity: usize,
	/// Whether a warning was logged since usage last fell under half of the stripes.
	warned: AtomicBool,
}

impl StripeUsage {
	/// Records `stripes` newly held and returns the guard that counts them on the gauge.
	fn acquire(&self, stripes: usize) -> Option<IntGaugeGuard> {
		if stripes == 0 {
			return None;
		}
		let metric = crate::metrics::FS_STORE_LOCK_STRIPES_HELD.add_guard(stripes as i64);
		let held = self.held.fetch_add(stripes, Ordering::Relaxed) + stripes;
		if held * 4 >= self.capacity * 3 && !self.warned.swap(true, Ordering::Relaxed) {
			tracing::warn!(
				held,
				capacity = self.capacity,
				"most volume lock stripes of a tablet store are held at once; raise depot.store_volume_lock_stripes if store work waits on stripe locks"
			);
		}
		Some(metric)
	}

	fn release(&self, stripes: usize) {
		if stripes == 0 {
			return;
		}
		let held = self.held.fetch_sub(stripes, Ordering::Relaxed) - stripes;
		if held * 2 < self.capacity {
			self.warned.store(false, Ordering::Relaxed);
		}
	}
}

pub(crate) struct StoreLocks {
	tablet: RwLock<()>,
	stripes: Box<[RwLock<()>]>,
	usage: StripeUsage,
	mode: VolumeLockMode,
	_capacity_metric: IntGaugeGuard,
}

impl Default for StoreLocks {
	fn default() -> Self {
		Self::new(DEFAULT_VOLUME_LOCK_STRIPES)
	}
}

impl StoreLocks {
	/// A table of `stripes` volume stripes, at least one.
	pub(crate) fn new(stripes: usize) -> Self {
		let stripes = stripes.max(1);
		Self {
			tablet: RwLock::new(()),
			stripes: (0..stripes).map(|_| RwLock::new(())).collect(),
			usage: StripeUsage {
				held: AtomicUsize::new(0),
				capacity: stripes,
				warned: AtomicBool::new(false),
			},
			mode: VolumeLockMode::default(),
			_capacity_metric: crate::metrics::FS_STORE_LOCK_STRIPES.add_guard(stripes as i64),
		}
	}

	/// Sets which operations take stripes.
	pub(crate) fn with_mode(mut self, mode: VolumeLockMode) -> Self {
		self.mode = mode;
		self
	}

	pub(crate) fn mode(&self) -> VolumeLockMode {
		self.mode
	}

	/// The locks a core-ordered operation takes: `request` in `stripes` mode, and only
	/// its tablet lock in `core` mode.
	pub(crate) fn core_ordered(&self, request: LockRequest) -> LockRequest {
		match self.mode {
			VolumeLockMode::Stripes => request,
			VolumeLockMode::Core => LockRequest {
				tablet: request.tablet,
				reads: Vec::new(),
				writes: Vec::new(),
			},
		}
	}

	

	/// Takes the tablet lock, then every requested stripe in ascending order, and
	/// records how long `operation` waited at each scope.
	pub(crate) async fn lock(
		&self,
		request: LockRequest,
		operation: LockOperation,
	) -> StoreGuard<'_> {
		let mode = match request.tablet {
			TabletMode::Shared => "shared",
			TabletMode::Exclusive => "exclusive",
		};
		let tablet = {
			let _wait = crate::metrics::FS_STORE_LOCK_WAIT
				.with_label_values(&[mode, operation.label(), "tablet"])
				.start_timer();
			match request.tablet {
				TabletMode::Shared => TabletGuard::Shared(self.tablet.read().await),
				TabletMode::Exclusive => TabletGuard::Exclusive(self.tablet.write().await),
			}
		};
		let stripes = request.stripes(self.stripes.len());
		let mut held = Vec::with_capacity(stripes.len());
		if !stripes.is_empty() {
			let write = stripes.iter().any(|(_, write)| *write);
			let _wait = crate::metrics::FS_STORE_LOCK_WAIT
				.with_label_values(&[
					if write { "exclusive" } else { "shared" },
					operation.label(),
					"volume",
				])
				.start_timer();
			for (stripe, write) in stripes {
				let lock = &self.stripes[stripe];
				held.push(if write {
					StripeGuard::Write(lock.write().await)
				} else {
					StripeGuard::Read(lock.read().await)
				});
			}
		}
		let held_metric = self.usage.acquire(held.len());
		StoreGuard {
			_stripes: held,
			_tablet: tablet,
			held: &self.usage,
			_held_metric: held_metric,
		}
	}
}

/// The store operations running per volume, checked against the footprint matrix in
/// debug builds while core-ordered work takes no stripe. A core admission bug then fails
/// tests instead of racing. Release builds record nothing.
///
/// Every access is synchronous and short, and no async code waits on the map, so the
/// guard can release its entries from `Drop`.
#[derive(Default)]
pub(crate) struct ActiveAccesses {
	#[cfg(debug_assertions)]
	volumes: scc::HashMap<StoreVolume, Vec<(u64, VolumeAccess)>>,
	#[cfg(debug_assertions)]
	next: std::sync::atomic::AtomicU64,
}

/// Entries one operation recorded, removed when it ends.
pub(crate) struct AccessGuard<'a> {
	#[cfg(debug_assertions)]
	owner: &'a ActiveAccesses,
	#[cfg(debug_assertions)]
	id: u64,
	#[cfg(debug_assertions)]
	volumes: Vec<StoreVolume>,
	#[cfg(not(debug_assertions))]
	_owner: std::marker::PhantomData<&'a ()>,
}

impl ActiveAccesses {
	/// Records that an operation now uses each of `accesses`, panicking in debug builds
	/// if one is incompatible with an operation already running on that volume.
	pub(crate) fn enter(
		&self,
		accesses: impl IntoIterator<Item = (StoreVolume, VolumeAccess)>,
	) -> AccessGuard<'_> {
		#[cfg(debug_assertions)]
		{
			let id = self.next.fetch_add(1, Ordering::Relaxed);
			let mut volumes = Vec::new();
			for (volume, access) in accesses {
				let mut entry = self.volumes.entry_sync(volume.clone()).or_default();
				if let Some((_, running)) = entry
					.get()
					.iter()
					.find(|(_, running)| !compatible(*running, access))
				{
					panic!(
						"store footprint violation on {volume:?}: {access:?} started beside {running:?}; the maintainer core admitted conflicting work"
					);
				}
				entry.get_mut().push((id, access));
				volumes.push(volume);
			}
			AccessGuard {
				owner: self,
				id,
				volumes,
			}
		}
		#[cfg(not(debug_assertions))]
		{
			drop(accesses);
			AccessGuard {
				_owner: std::marker::PhantomData,
			}
		}
	}
}

#[cfg(debug_assertions)]
impl Drop for AccessGuard<'_> {
	fn drop(&mut self) {
		for volume in self.volumes.drain(..) {
			let _ = self.owner.volumes.remove_if_sync(&volume, |running| {
				running.retain(|(id, _)| *id != self.id);
				running.is_empty()
			});
		}
	}
}


