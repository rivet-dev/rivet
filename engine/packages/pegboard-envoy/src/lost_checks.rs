//! Lost checks for actors hosted on a closed Envoy connection.
//!
//! An actor learns that its Envoy is gone only from an event, and a quiet actor sends none. When
//! a connection closes, the process schedules one check at `close + pegboard.envoy_lost_threshold`
//! for the epochs that connection hosted, the time the Envoy itself keeps them while disconnected. A reconnect to this process inside that window cancels
//! the check and hands its epochs to the new connection. When the check is due it reads the
//! Envoy's record once: a newer live connection means the Envoy came back, anywhere in the
//! cluster; otherwise each epoch is reported lost to its actor state machine with a
//! `Reallocate`, which applies it only to the epoch it names. The gateway runs the same check
//! when a request finds the envoy unreachable; this one covers actors nobody requests.
//!
//! The check is the deadline paired with the connection close. It lives only in memory: if this
//! process dies with the socket, nothing runs the check and the actor is found lost on its next
//! request or delivery.
use anyhow::{Context, Result};
use futures_util::{StreamExt, stream};
use pegboard::commands::EnvoyRead;
use pegboard::keys::partition::{ActorVolume, EnvoyVolume};
use pegboard::wire;
use rivet_engine_context::EngineContext;
use rivet_state_machine::IndexedDeadlineQueue;
use rivet_util_id::Id;
use std::collections::{BTreeMap, HashMap};
use tokio::sync::{Mutex, Notify};

/// The key of one Envoy's record: namespace, pool and Envoy key.
pub(crate) type EnvoyKey = (String, String, String);

/// One generation an Envoy connection hosted.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct HostedActor {
	pub actor_id: String,
	pub generation: u32,
}

/// The generations a closed connection hosted, and the connection that hosted them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Check {
	pub connection_id: Vec<u8>,
	pub actors: Vec<HostedActor>,
}

impl Check {
	/// Adds `other`'s generations. For an actor in both, the newer generation is kept.
	/// Indexes the existing generations by actor ID, so a merge costs O(actors) rather than one
	/// linear search per merged actor.
	fn merge(&mut self, other: Check) {
		let mut positions: HashMap<String, usize> = self
			.actors
			.iter()
			.enumerate()
			.map(|(index, actor)| (actor.actor_id.clone(), index))
			.collect();
		for actor in other.actors {
			match positions.get(&actor.actor_id) {
				Some(&index) => {
					let existing = &mut self.actors[index];
					existing.generation = existing.generation.max(actor.generation);
				}
				None => {
					positions.insert(actor.actor_id.clone(), self.actors.len());
					self.actors.push(actor);
				}
			}
		}
	}
}

/// Pending checks ordered by due time, at most one per Envoy.
pub(crate) struct Pending {
	capacity: usize,
	queue: IndexedDeadlineQueue<EnvoyKey>,
	checks: BTreeMap<EnvoyKey, Check>,
	/// Whether the near-capacity warning fired since the queue was last below 90 %.
	warned: bool,
}

impl Pending {
	pub(crate) fn new(capacity: usize) -> Self {
		Self {
			capacity: capacity.max(1),
			queue: IndexedDeadlineQueue::new(),
			checks: BTreeMap::new(),
			warned: false,
		}
	}

	/// Schedules `check` for `key` at `at_boot_ms`. A check already pending for the envoy keeps its
	/// earlier time and absorbs the new generations. When the queue is full, the check due soonest
	/// is dropped and returned.
	pub(crate) fn schedule(
		&mut self,
		key: EnvoyKey,
		check: Check,
		at_boot_ms: u64,
	) -> Option<(EnvoyKey, Check)> {
		if let Some(existing) = self.checks.get_mut(&key) {
			existing.connection_id = check.connection_id.clone();
			existing.merge(check);
			self.queue.schedule_earliest(key, at_boot_ms);
			return None;
		}

		let dropped = if self.len() >= self.capacity {
			self.queue
				.pop_due(u64::MAX, 1)
				.into_iter()
				.next()
				.and_then(|(_, oldest)| self.checks.remove(&oldest).map(|check| (oldest, check)))
		} else {
			None
		};

		self.queue.schedule(key.clone(), at_boot_ms);
		self.checks.insert(key, check);
		self.observe();
		dropped
	}

	/// Removes the check pending for `key`, returning its generations.
	pub(crate) fn cancel(&mut self, key: &EnvoyKey) -> Option<Check> {
		self.queue.cancel(key);
		let check = self.checks.remove(key);
		self.observe();
		check
	}

	/// Removes and returns every check due at `now_boot_ms`.
	pub(crate) fn pop_due(&mut self, now_boot_ms: u64) -> Vec<(EnvoyKey, Check)> {
		let due = self
			.queue
			.pop_due(now_boot_ms, usize::MAX)
			.into_iter()
			.filter_map(|(_, key)| self.checks.remove(&key).map(|check| (key, check)))
			.collect();
		self.observe();
		due
	}

	pub(crate) fn next_at(&self) -> Option<u64> {
		self.queue.next_at()
	}

	pub(crate) fn len(&self) -> usize {
		self.checks.len()
	}

	/// Warns once each time usage reaches 90 % of the capacity.
	fn observe(&mut self) {
		let len = self.checks.len();
		let near_full = len.saturating_mul(10) >= self.capacity.saturating_mul(9);
		if near_full && !self.warned {
			tracing::warn!(
				len,
				capacity = self.capacity,
				"envoy lost checks are near capacity; many Envoy connections closed at once, raise pegboard.envoy_lost_checks_capacity if this persists"
			);
		}
		self.warned = near_full;
	}
}

/// Whether the stored record shows the Envoy connected again on a connection other than the
/// closed one, served by a worker whose lease has not ended.
pub(crate) fn reconnected(
	stored: Option<&wire::EnvoyData>,
	closed_connection_id: &[u8],
	newer_worker_lease_ended: bool,
) -> bool {
	stored.is_some_and(|envoy| {
		envoy.connection_id.as_ref() != closed_connection_id
			&& envoy.connected
			&& !newer_worker_lease_ended
	})
}

/// The process-owned lost-check queue shared by every Envoy connection.
pub(crate) struct LostChecks {
	pending: Mutex<Pending>,
	/// Wakes the run loop when a check is scheduled, since it may be due before the head.
	changed: Notify,
}

/// Actors reported concurrently for one check.
const REPORT_CONCURRENCY: usize = 16;

impl LostChecks {
	pub(crate) fn new(capacity: usize) -> Self {
		Self {
			pending: Mutex::new(Pending::new(capacity)),
			changed: Notify::new(),
		}
	}

	/// Schedules the check for a connection that closed at `now_boot_ms`.
	pub(crate) async fn schedule(
		&self,
		key: EnvoyKey,
		check: Check,
		now_boot_ms: u64,
		delay_ms: u64,
	) {
		let dropped =
			self.pending
				.lock()
				.await
				.schedule(key, check, now_boot_ms.saturating_add(delay_ms));
		if let Some(((namespace, pool_name, envoy_key), check)) = dropped {
			tracing::warn!(
				%namespace,
				%pool_name,
				%envoy_key,
				actors = check.actors.len(),
				"dropped an envoy lost check at capacity; its actors are found lost on their next request"
			);
		}
		self.changed.notify_one();
	}

	/// Cancels the check of an Envoy that reconnected to this process, returning the generations
	/// the new connection now hosts.
	pub(crate) async fn cancel(&self, key: &EnvoyKey) -> Option<Check> {
		self.pending.lock().await.cancel(key)
	}

	/// Runs due checks until the caller drops this future. One sleep covers every check: it waits
	/// for the earliest due time.
	pub(crate) async fn run(&self, ctx: &EngineContext) -> Result<()> {
		let clock = ctx.env().clock();
		loop {
			let changed = self.changed.notified();
			let next_at = self.pending.lock().await.next_at().unwrap_or(u64::MAX);
			rivet_env::select_biased!("a newly scheduled check may be due before the current head",
				_ = changed => continue,
				slept = clock.sleep_until(next_at) => slept?,
			);
			let now = clock.observe()?.boot_ms;
			let due = self.pending.lock().await.pop_due(now);
			for (key, check) in due {
				if let Err(error) = run_check(ctx, &key, check).await {
					let (namespace, pool_name, envoy_key) = key;
					tracing::warn!(
						%namespace,
						%pool_name,
						%envoy_key,
						?error,
						"envoy lost check could not read the envoy record; its actors are found lost on their next request"
					);
				}
			}
		}
	}
}

/// Reads the Envoy's record and reports every hosted epoch lost unless the Envoy reconnected.
async fn run_check(ctx: &EngineContext, key: &EnvoyKey, check: Check) -> Result<()> {
	let (namespace, pool_name, envoy_key) = key;
	let volume =
		EnvoyVolume::new(namespace.clone(), pool_name.clone(), envoy_key.clone()).volume_key();
	let stored = match ctx.depot().command::<EnvoyRead>(&volume, ()).await {
		Ok(response) => response.data,
		Err(error) if pegboard::commands::is_volume_gone(&error) => None,
		Err(error) => return Err(error),
	};

	let newer_worker_lease_ended = match stored
		.as_ref()
		.filter(|stored| stored.connection_id.as_ref() != check.connection_id.as_slice())
	{
		Some(stored) => {
			let worker = universalpubsub::WorkerId::from_bytes(
				stored
					.worker_id
					.as_ref()
					.try_into()
					.context("invalid envoy worker id")?,
			);
			match ctx.topology().worker_lease(&worker).await {
				rivet_topology::WorkerLease::Expired | rivet_topology::WorkerLease::Released => {
					true
				}
				rivet_topology::WorkerLease::Live | rivet_topology::WorkerLease::Unknown => false,
			}
		}
		None => false,
	};
	if reconnected(
		stored.as_ref(),
		&check.connection_id,
		newer_worker_lease_ended,
	) {
		return Ok(());
	}

	tracing::info!(
		%namespace,
		%pool_name,
		%envoy_key,
		actors = check.actors.len(),
		"envoy did not reconnect within the ping timeout; reporting its actors lost"
	);
	stream::iter(check.actors)
		.for_each_concurrent(REPORT_CONCURRENCY, |actor| report_lost(ctx, actor))
		.await;
	Ok(())
}

/// Reports one hosted epoch lost with a `Reallocate`, once. A failure is left to the actor's
/// next request, which runs the same check from the gateway.
async fn report_lost(ctx: &EngineContext, actor: HostedActor) {
	let result = async {
		let id = Id::parse(&actor.actor_id).context("invalid hosted actor id")?;
		let volume = ActorVolume::new(id).volume_key();
		let event = wire::ActorEvent::Reallocate(wire::Reallocate {
			epoch: actor.generation,
			lost: true,
			waiter: None,
		});
		ctx.depot()
			.command::<pegboard::commands::ActorEvent>(&volume, event)
			.await
			.map(|_response| ())
	}
	.await;
	match result {
		Ok(()) => {}
		Err(error) if pegboard::commands::is_volume_gone(&error) => {}
		Err(error) => {
			tracing::warn!(
				actor_id = %actor.actor_id,
				generation = actor.generation,
				?error,
				"could not report an actor lost; its next request runs the lost check"
			);
		}
	}
}


