//! Moves actors off an envoy that is shutting down, or that a newer version superseded in a
//! pool that drains on version upgrade.
//!
//! A stopping envoy does not stop its own actors: it tells the engine it is stopping and waits
//! for stop commands, so a rolling deploy does not wake every actor of a node on other nodes at
//! once. Envoys older than protocol v10 still stop all their actors themselves, so their
//! connections are never drained when they stop.
//!
//! Placement already sends new and waking actors to the pool's newest live version. What remains
//! on an older envoy are actors that never sleep, which would run old code until the node is
//! killed. When the pool drains on version upgrade, those actors are asked to move too.
//!
//! Both drains use the actor machine's reallocation event, the same path an envoy check uses for
//! a draining envoy: the engine stops the actor on this envoy with `GoingAway`, the node sleeps
//! it, and the next start lands on another envoy. Each request is fenced by the actor's epoch.
//!
//! Both are paced by the pool's eviction delay, period and rate, and every actor has been asked
//! to move by the end of the delay plus the period. The node learns that time at connect, so its
//! own drain deadline covers it. The decisions ride the heartbeat, so nothing is armed per actor.
//! A superseded envoy learns the pool's newest live version from every heartbeat's index reply;
//! a lost reply or a failed eviction is redone by the next heartbeat, and a newest version whose
//! envoys all died clears that drain.
use anyhow::Result;
use pegboard::wire;
use rivet_engine_context::EngineContext;

/// The first envoy protocol version whose envoys wait for the engine to stop their actors when
/// they shut down.
pub(crate) const PACED_DRAIN_PROTOCOL_VERSION: u16 = 10;

/// How a pool paces the eviction of a draining envoy's actors.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Pacing {
	/// Time after the drain began before any actor moves.
	pub delay_ms: u64,
	/// The window after the delay over which every actor is asked to move. Zero asks all of
	/// them once the delay ends.
	pub period_ms: u64,
	/// The slowest the window may move actors, in actors per second. A drain moves faster when
	/// the window needs it, and never slower than this rate until every actor was asked.
	pub rate_per_second: f64,
}

impl Pacing {
	pub(crate) fn of_pool(config: &wire::NodePoolConfig) -> Self {
		Self {
			delay_ms: u64::from(config.actor_eviction_delay).saturating_mul(1_000),
			period_ms: u64::from(config.actor_eviction_period).saturating_mul(1_000),
			rate_per_second: config.actor_eviction_rate,
		}
	}

	/// The time after a drain began by which every actor has been asked to move.
	pub(crate) fn drain_ms(&self) -> u64 {
		self.delay_ms.saturating_add(self.period_ms)
	}

	/// How many of the `remaining` actors not yet asked a heartbeat at `now_boot_ms` may evict,
	/// for a drain that began at `since_boot_ms` and whose last batch ran at
	/// `last_batch_boot_ms`. Nothing moves inside the delay. After it, a batch is the larger of
	/// the rate's worth of the time since the last batch, or since the delay ended for the
	/// first, and the share of the remaining actors that this time is of what is left of the
	/// window, rounded up. So the rate holds whatever the heartbeat interval, the pace adapts to
	/// actors that stop on their own, and every actor has been asked once the window ends.
	pub(crate) fn due_evictions(
		&self,
		remaining: usize,
		since_boot_ms: u64,
		last_batch_boot_ms: Option<u64>,
		now_boot_ms: u64,
	) -> usize {
		let eligible_boot_ms = since_boot_ms.saturating_add(self.delay_ms);
		if now_boot_ms < eligible_boot_ms {
			return 0;
		}
		let end_boot_ms = eligible_boot_ms.saturating_add(self.period_ms);
		if now_boot_ms >= end_boot_ms {
			return remaining;
		}
		let window_start_boot_ms = last_batch_boot_ms
			.unwrap_or(eligible_boot_ms)
			.max(eligible_boot_ms);
		let covered_ms = now_boot_ms.saturating_sub(window_start_boot_ms);
		let left_ms = end_boot_ms.saturating_sub(window_start_boot_ms);

		let by_window = (remaining as u128 * covered_ms as u128).div_ceil(left_ms.max(1) as u128);
		let by_window = usize::try_from(by_window).unwrap_or(remaining);
		let by_rate = (self.rate_per_second * covered_ms as f64 / 1_000.0).ceil();
		let by_rate = if by_rate.is_finite() && by_rate > 0.0 {
			// The cast saturates, and the count is capped by `remaining` below.
			by_rate as usize
		} else {
			0
		};
		by_window.max(by_rate).min(remaining)
	}
}

#[derive(Default)]
struct Drain {
	/// Boot time the envoy was first seen superseded in the current stretch, or `None`.
	since_boot_ms: Option<u64>,
	/// Boot time the envoy began stopping. A stopping drain outranks a version stretch and
	/// never ends, since a stopping envoy does not come back into allocation.
	stopping_since_boot_ms: Option<u64>,
	/// Boot time of the last eviction batch of the current stretch.
	last_batch_boot_ms: Option<u64>,
	/// The newest live version last reported, for logging the transition.
	newest_live_version: u32,
}

/// One connection's drain state, written by its heartbeats and its stopping message.
#[derive(Default)]
pub(crate) struct State {
	drain: tokio::sync::Mutex<Drain>,
	/// Actor epochs already asked to move, so a slow stop does not spend the rate budget twice.
	/// Entries are dropped once the epoch is no longer bound on the connection.
	asked: scc::HashSet<(String, u32)>,
}

/// An actor bound on the connection that a batch may evict.
pub(crate) struct BoundActor {
	pub actor_id: String,
	/// The epoch of the actor running on the connection, which fences the move request so a
	/// later incarnation of the actor is never stopped by it.
	pub epoch: u32,
}

impl State {
	/// Whether the envoy is inside a drain stretch, so a heartbeat can skip gathering the bound
	/// actors otherwise.
	pub(crate) async fn is_draining(&self) -> bool {
		let drain = self.drain.lock().await;
		drain.since_boot_ms.is_some() || drain.stopping_since_boot_ms.is_some()
	}

	/// Starts the stopping drain at `now_boot_ms`. A repeated stopping message keeps the first
	/// start, so the node's deadline, counted from when it began stopping, still covers it.
	pub(crate) async fn begin_stopping(&self, now_boot_ms: u64) {
		let mut drain = self.drain.lock().await;
		if drain.stopping_since_boot_ms.is_some() {
			return;
		}
		drain.stopping_since_boot_ms = Some(now_boot_ms);
		drain.last_batch_boot_ms = None;
	}

	/// Records a heartbeat reply naming the pool's newest live version. Starts a drain stretch
	/// when `newest_live_version` passes the envoy's `version` while the pool drains on version
	/// upgrade, and ends it otherwise. Returns whether the envoy is superseded now.
	pub(crate) async fn observe(
		&self,
		scope: &crate::routes::EnvoyScope,
		envoy_key: &str,
		version: u32,
		newest_live_version: u32,
		drains: bool,
		now_boot_ms: u64,
	) -> bool {
		let superseded = drains && newest_live_version > version;
		let mut drain = self.drain.lock().await;
		match (drain.since_boot_ms, superseded) {
			(None, true) => {
				tracing::info!(
					namespace = %scope.namespace,
					pool_name = %scope.pool_name,
					%envoy_key,
					version,
					newest_live_version,
					"envoy superseded by a newer version in a pool that drains on version upgrade; its actors will move"
				);
				drain.since_boot_ms = Some(now_boot_ms);
				if drain.stopping_since_boot_ms.is_none() {
					drain.last_batch_boot_ms = None;
				}
			}
			(Some(_), false) => {
				tracing::info!(
					namespace = %scope.namespace,
					pool_name = %scope.pool_name,
					%envoy_key,
					version,
					newest_live_version,
					"envoy no longer superseded; its actors stay"
				);
				drain.since_boot_ms = None;
				// A stopping drain keeps its own progress.
				if drain.stopping_since_boot_ms.is_none() {
					drain.last_batch_boot_ms = None;
					self.asked.clear_async().await;
				}
			}
			(None, false) | (Some(_), true) => {}
		}
		drain.newest_live_version = newest_live_version;
		superseded
	}

	/// Asks the actors a batch is due for to move, choosing among `bound`, the actors currently
	/// bound on the connection, paced by the pool's `pacing` from the start of the stopping
	/// drain, or else of the version stretch. Nothing happens outside a drain or before a batch
	/// is due. A failed request is retried by a later heartbeat, which finds the actor still
	/// bound.
	pub(crate) async fn evict_due(
		&self,
		ctx: &EngineContext,
		scope: &crate::routes::EnvoyScope,
		envoy_key: &str,
		pacing: Pacing,
		bound: Vec<BoundActor>,
		now_boot_ms: u64,
	) -> Result<()> {
		let mut drain = self.drain.lock().await;
		let Some(since_boot_ms) = drain.stopping_since_boot_ms.or(drain.since_boot_ms) else {
			return Ok(());
		};

		// Forget epochs that stopped so a later epoch of the same actor, should it
		// land here again, can be asked.
		self.asked
			.retain_async(|(actor_id, epoch)| {
				bound
					.iter()
					.any(|actor| actor.actor_id == *actor_id && actor.epoch == *epoch)
			})
			.await;
		let mut unasked = Vec::new();
		for actor in bound {
			if !self
				.asked
				.contains_async(&(actor.actor_id.clone(), actor.epoch))
				.await
			{
				unasked.push(actor);
			}
		}

		let count = pacing.due_evictions(
			unasked.len(),
			since_boot_ms,
			drain.last_batch_boot_ms,
			now_boot_ms,
		);
		unasked.truncate(count);
		let candidates = unasked;
		if candidates.is_empty() {
			return Ok(());
		}
		let stopping = drain.stopping_since_boot_ms.is_some();
		drain.last_batch_boot_ms = Some(now_boot_ms);
		drop(drain);

		tracing::debug!(
			namespace = %scope.namespace,
			pool_name = %scope.pool_name,
			%envoy_key,
			batch = candidates.len(),
			stopping,
			"asking actors on a draining envoy to move"
		);
		let evictions = if stopping {
			&*pegboard::ENVOY_DRAIN_EVICTIONS
		} else {
			&*pegboard::ENVOY_VERSION_EVICTIONS
		};
		for actor in candidates {
			let actor_id = rivet_util_id::Id::parse(&actor.actor_id)?;
			// An actor the backfill has not moved, or skipped, is still bound to its legacy
			// volume on a grouped connection, so each actor is addressed where it lives.
			let group = scope.actor_group(ctx, actor_id).await?;
			pegboard::envoy_check::report_reallocate(ctx, group, actor_id, actor.epoch, false)
				.await?;
			let _ = self.asked.insert_async((actor.actor_id, actor.epoch)).await;
			evictions
				.with_label_values(&[&scope.namespace, &scope.pool_name])
				.inc();
		}
		Ok(())
	}
}
