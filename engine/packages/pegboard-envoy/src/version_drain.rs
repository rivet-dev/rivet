//! Moves actors off an envoy that a newer version superseded, in a pool that drains on version
//! upgrade.
//!
//! Placement already sends new and waking actors to the pool's newest live version. What remains
//! on an older envoy are actors that never sleep, which would run old code until the node is
//! killed. When the pool drains on version upgrade, those actors are asked to move through the
//! actor machine's reallocation event, the same path an envoy check uses for a draining envoy:
//! the engine stops the actor on this envoy with `GoingAway`, the node sleeps it, and the next
//! start lands on the newest version.
//!
//! The decision rides the heartbeat. Every heartbeat's index reply names the pool's newest live
//! version, so a superseded envoy learns it on its next heartbeat and re-learns it on each one
//! after; a lost reply or a failed eviction is redone by the next heartbeat, and a newest
//! version whose envoys all died clears the drain. Eviction is paced by the pool's delay, period
//! and rate so the rest of the pool is not hit by every actor at once. The node itself is never
//! told to exit: it stays connected, serves no new actors while superseded, and its process
//! lifetime belongs to the operator's supervisor.
use anyhow::Result;
use pegboard::wire;
use rivet_engine_context::EngineContext;

/// How a pool paces eviction once an envoy is superseded.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Pacing {
	/// Time after the envoy was first seen superseded before any actor moves.
	pub delay_ms: u64,
	/// The smallest window between two eviction batches. A zero period batches on every
	/// heartbeat, each sized by the time since the last batch.
	pub period_ms: u64,
	/// Actors per second across the windows.
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

	/// How many actors a heartbeat at `now_boot_ms` may evict, for an envoy superseded since
	/// `since_boot_ms` whose last batch ran at `last_batch_boot_ms`. Nothing moves inside the
	/// delay, and a batch waits for the period to pass since the previous one, or since the
	/// delay ended for the first. A batch is sized by the rate over the time it covers, so the
	/// rate holds on average whatever the heartbeat interval.
	pub(crate) fn due_evictions(
		&self,
		since_boot_ms: u64,
		last_batch_boot_ms: Option<u64>,
		now_boot_ms: u64,
	) -> usize {
		let eligible_boot_ms = since_boot_ms.saturating_add(self.delay_ms);
		if now_boot_ms < eligible_boot_ms {
			return 0;
		}
		let window_start_boot_ms = last_batch_boot_ms.unwrap_or(eligible_boot_ms);
		let elapsed_ms = now_boot_ms.saturating_sub(window_start_boot_ms);
		if elapsed_ms < self.period_ms {
			return 0;
		}
		// A batch covers at least one second, so the first one after the delay, and every batch
		// of a zero period, moves at least the rate's worth of one second.
		let covered_ms = elapsed_ms.max(1_000);
		let count = (self.rate_per_second * covered_ms as f64 / 1_000.0).ceil();
		if !count.is_finite() || count <= 0.0 {
			return 0;
		}
		if count >= usize::MAX as f64 {
			return usize::MAX;
		}
		count as usize
	}
}

#[derive(Default)]
struct Drain {
	/// Boot time the envoy was first seen superseded in the current stretch, or `None`.
	since_boot_ms: Option<u64>,
	/// Boot time of the last eviction batch of the current stretch.
	last_batch_boot_ms: Option<u64>,
	/// The newest live version last reported, for logging the transition.
	newest_live_version: u32,
}

/// One connection's drain state, written by its heartbeats.
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
		self.drain.lock().await.since_boot_ms.is_some()
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
				drain.last_batch_boot_ms = None;
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
				drain.last_batch_boot_ms = None;
				self.asked.clear_async().await;
			}
			(None, false) | (Some(_), true) => {}
		}
		drain.newest_live_version = newest_live_version;
		superseded
	}

	/// Asks the actors a batch is due for to move, choosing among `bound`, the actors currently
	/// bound on the connection. Nothing happens outside a drain stretch or before a batch is
	/// due. A failed request is retried by a later heartbeat, which finds the actor still bound.
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
		let Some(since_boot_ms) = drain.since_boot_ms else {
			return Ok(());
		};
		let count = pacing.due_evictions(since_boot_ms, drain.last_batch_boot_ms, now_boot_ms);
		if count == 0 {
			return Ok(());
		}

		// Forget epochs that stopped so a later epoch of the same actor, should it
		// land here again, can be asked.
		self.asked
			.retain_async(|(actor_id, epoch)| {
				bound
					.iter()
					.any(|actor| actor.actor_id == *actor_id && actor.epoch == *epoch)
			})
			.await;
		let mut candidates = Vec::new();
		for actor in bound {
			if self
				.asked
				.contains_async(&(actor.actor_id.clone(), actor.epoch))
				.await
			{
				continue;
			}
			candidates.push(actor);
			if candidates.len() == count {
				break;
			}
		}
		if candidates.is_empty() {
			return Ok(());
		}
		drain.last_batch_boot_ms = Some(now_boot_ms);
		drop(drain);

		tracing::debug!(
			namespace = %scope.namespace,
			pool_name = %scope.pool_name,
			%envoy_key,
			batch = candidates.len(),
			"asking actors on a superseded envoy to move"
		);
		for actor in candidates {
			let actor_id = rivet_util_id::Id::parse(&actor.actor_id)?;
			// An actor the backfill has not moved, or skipped, is still bound to its legacy
			// volume on a grouped connection, so each actor is addressed where it lives.
			let group = scope.actor_group(ctx, actor_id).await?;
			pegboard::envoy_check::report_reallocate(ctx, group, actor_id, actor.epoch, false)
				.await?;
			let _ = self.asked.insert_async((actor.actor_id, actor.epoch)).await;
			pegboard::ENVOY_VERSION_EVICTIONS
				.with_label_values(&[&scope.namespace, &scope.pool_name])
				.inc();
		}
		Ok(())
	}
}
