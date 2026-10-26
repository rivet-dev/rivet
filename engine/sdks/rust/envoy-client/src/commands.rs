use std::collections::{BTreeMap, HashMap};
use std::time::Duration;

use rivet_envoy_protocol as protocol;

use crate::actor::create_actor;
use crate::connection::ws_send;
use crate::envoy::EnvoyContext;
use crate::metrics::METRICS;
use crate::stringify::stringify_command_wrapper;
use crate::time::Instant;

/// The highest stopped generation of each recently stopped actor. Generations only grow
/// per actor, so a start at or below the floor is a late duplicate of a stopped
/// generation and is dropped.
///
/// A late start can only arrive within a bounded window: the engine drops a start whose
/// sender's deadline passed before forwarding it, and the connection's lost threshold
/// bounds how long a forwarded start can wait. So a floor expires after
/// `EnvoyConfig::stopped_generation_floor_ttl`, which must exceed that window. Expired
/// floors are pruned lazily when another floor is recorded.
/// `EnvoyConfig::stopped_generation_floor_capacity` is a memory backstop; at the cap the
/// least recently stopped actor is forgotten early, with a warning.
#[derive(Default)]
pub struct StoppedGenerationFloors {
	floors: HashMap<String, (u32, (Instant, u64))>,
	/// Recency order: (stopped at, stamp) to actor. The oldest entry expires first.
	order: BTreeMap<(Instant, u64), String>,
	next_stamp: u64,
	/// Whether the near-cap warning was logged since usage last fell below the threshold.
	warned: bool,
}

impl StoppedGenerationFloors {
	/// Records that `generation` of `actor_id` stopped at `now`, keeping the higher floor,
	/// and forgets floors older than `ttl`.
	pub fn record(
		&mut self,
		actor_id: &str,
		generation: u32,
		now: Instant,
		ttl: Duration,
		capacity: usize,
	) {
		self.expire(now, ttl);

		let key = (now, self.next_stamp);
		self.next_stamp += 1;
		let floor = match self.floors.remove(actor_id) {
			Some((floor, old_key)) => {
				self.order.remove(&old_key);
				floor.max(generation)
			}
			None => generation,
		};
		self.floors.insert(actor_id.to_owned(), (floor, key));
		self.order.insert(key, actor_id.to_owned());

		// The cap is a backstop for stop rates the TTL cannot bound.
		while self.floors.len() > capacity.max(1) {
			let Some((_, evicted)) = self.order.pop_first() else {
				break;
			};
			self.floors.remove(&evicted);
		}
		self.observe(capacity);
	}

	/// Forgets every floor recorded `ttl` or longer before `now`.
	fn expire(&mut self, now: Instant, ttl: Duration) {
		while let Some(entry) = self.order.first_entry() {
			let (stopped_at, _) = *entry.key();
			if now.saturating_duration_since(stopped_at) < ttl {
				break;
			}
			let actor_id = entry.remove();
			self.floors.remove(&actor_id);
		}
	}

	/// Whether a start for `generation` of `actor_id` repeats a stopped generation.
	pub fn blocks(&self, actor_id: &str, generation: u32) -> bool {
		self.floors
			.get(actor_id)
			.is_some_and(|(floor, _)| generation <= *floor)
	}

	pub fn len(&self) -> usize {
		self.floors.len()
	}

	pub fn is_empty(&self) -> bool {
		self.floors.is_empty()
	}

	fn observe(&mut self, capacity: usize) {
		let usage = self.floors.len();
		METRICS.stopped_floors.set(usage as i64);
		METRICS.stopped_floors_capacity.set(capacity as i64);
		let near_cap = usage.saturating_mul(10) >= capacity.saturating_mul(9);
		if near_cap && !self.warned {
			self.warned = true;
			tracing::warn!(
				usage,
				capacity,
				"stopped generation floors are near their cap; many actors stopped on this Envoy within the floor TTL, so raise stopped_generation_floor_capacity"
			);
		} else if !near_cap && usage.saturating_mul(10) < capacity.saturating_mul(8) {
			self.warned = false;
		}
	}
}

pub async fn handle_commands(ctx: &mut EnvoyContext, commands: Vec<protocol::CommandWrapper>) {
	tracing::info!(command_count = commands.len(), "received commands");
	for command_wrapper in &commands {
		tracing::info!(
			command = %stringify_command_wrapper(command_wrapper),
			"received command"
		);
	}

	// Collect every actor in the raw batch before dedup, so a replayed
	// (skipped) command is still re-acked instead of being replayed forever.
	let batch_actors: Vec<(String, u32)> = commands
		.iter()
		.map(|c| (c.checkpoint.actor_id.to_string(), c.checkpoint.generation))
		.collect();

	for command_wrapper in commands {
		let checkpoint = command_wrapper.checkpoint;
		let dedup_key = (checkpoint.actor_id.to_string(), checkpoint.generation);

		// A start for a generation that already stopped here is a late duplicate. Starting
		// it again would run a generation the engine no longer tracks.
		if matches!(
			command_wrapper.inner,
			protocol::Command::CommandStartActor(_)
		) && ctx
			.stopped_generation_floors
			.blocks(&checkpoint.actor_id, checkpoint.generation)
		{
			tracing::debug!(
				actor_id = %checkpoint.actor_id,
				generation = checkpoint.generation,
				"dropping start for a stopped generation"
			);
			continue;
		}

		// Drop repeated commands. The command index is monotonic per
		// `(actor_id, generation)`, so any index at or below the highest one
		// already processed for a hosted generation is a duplicate. A live actor
		// owns its checkpoint, including while its asynchronous startup is pending.
		let last_idx = ctx.processed_command_idx.get(&dedup_key).copied().max(
			ctx.actors
				.get(&*checkpoint.actor_id)
				.and_then(|generations| generations.get(&checkpoint.generation))
				.map(|entry| entry.last_command_idx),
		);
		if let Some(last_idx) = last_idx {
			if checkpoint.index <= last_idx {
				// Restore the cache entry so the batch ack also covers this replay.
				ctx.processed_command_idx.insert(dedup_key, last_idx);
				tracing::debug!(
					actor_id = %checkpoint.actor_id,
					generation = checkpoint.generation,
					index = checkpoint.index,
					last_idx,
					"skipping replayed command"
				);
				continue;
			}
		}
		ctx.processed_command_idx
			.insert(dedup_key, checkpoint.index);

		match command_wrapper.inner {
			protocol::Command::CommandStartActor(val) => {
				let actor_name = val.config.name.clone();
				let (handle, active_http_request_count) = create_actor(
					ctx.shared.clone(),
					checkpoint.actor_id.clone().to_string(),
					checkpoint.generation,
					val.config,
					val.hibernating_requests,
					val.preloaded_kv,
				);

				ctx.insert_actor(
					checkpoint.actor_id.to_string(),
					checkpoint.generation,
					handle,
					active_http_request_count,
					actor_name.to_string(),
					checkpoint.index,
				);
			}
			protocol::Command::CommandStopActor(val) => {
				let entry = ctx.get_actor_entry_mut(&checkpoint.actor_id, checkpoint.generation);

				if let Some(entry) = entry {
					entry.received_stop = true;
					entry.last_command_idx = checkpoint.index;
					let _ = entry.handle.send(crate::actor::ToActor::Stop {
						command_idx: checkpoint.index,
						reason: val.reason,
					});
				} else {
					tracing::debug!(
						actor_id = %checkpoint.actor_id,
						generation = checkpoint.generation,
						"stop for a generation this envoy does not host; reporting it stopped"
					);
					report_unhosted_generation_stopped(
						ctx,
						&checkpoint.actor_id,
						checkpoint.generation,
					)
					.await;
				}
			}
		}
	}

	// Ack the whole batch immediately, scoped to this batch's actors. Engines that
	// replay unacked commands on reconnect stop replaying them once acked; engines
	// that do not replay ignore the ack.
	if !batch_actors.is_empty() {
		send_batch_command_acks(ctx, &batch_actors).await;
	}

	// A command for a generation this Envoy does not host leaves nothing to
	// protect once acked. Hosted generations are forgotten by `remove_actor`.
	for key in &batch_actors {
		let hosted = ctx
			.actors
			.get(&key.0)
			.is_some_and(|generations| generations.contains_key(&key.1));
		if !hosted {
			ctx.processed_command_idx.remove(key);
		}
	}
}

/// Answers a stop for a generation this Envoy does not run with a stopped event, so the
/// engine ends the epoch now; it would otherwise keep resending the stop. The generation either never
/// started here or already stopped, and the engine sends a generation's commands only to the
/// Envoy it allocated, so no other Envoy runs it either.
///
/// The stopped floor is recorded first: a start for this generation that the stop overtook
/// is then dropped on arrival instead of running an epoch the engine has ended. A repeated
/// stop reports again, which the engine ignores once the epoch moved on.
async fn report_unhosted_generation_stopped(
	ctx: &mut EnvoyContext,
	actor_id: &str,
	generation: u32,
) {
	ctx.stopped_generation_floors.record(
		actor_id,
		generation,
		Instant::now(),
		ctx.shared.config.stopped_generation_floor_ttl,
		ctx.shared.config.stopped_generation_floor_capacity,
	);

	// Follow any unacknowledged events of the generation, so acking one cannot drop the other.
	let index = ctx
		.pending_events
		.get(&(actor_id.to_owned(), generation))
		.and_then(|pending| {
			pending
				.events
				.iter()
				.map(|event| event.checkpoint.index)
				.max()
		})
		.map_or(0, |index| index + 1);

	let event = protocol::EventWrapper {
		checkpoint: protocol::ActorCheckpoint {
			actor_id: actor_id.to_owned().into(),
			generation,
			index,
		},
		inner: protocol::Event::EventActorStateUpdate(protocol::EventActorStateUpdate {
			state: protocol::ActorState::ActorStateStopped(protocol::ActorStateStopped {
				code: protocol::StopCode::Ok,
				message: None,
			}),
		}),
	};
	crate::events::handle_send_events(ctx, vec![event]).await;
}

/// Ack only the given actors' latest processed command index. A failed send is
/// retried by the next delivery of the same batch.
async fn send_batch_command_acks(ctx: &EnvoyContext, actors: &[(String, u32)]) {
	let mut highest: HashMap<(String, u32), i64> = HashMap::new();
	for key in actors {
		if let Some(&index) = ctx.processed_command_idx.get(key) {
			highest.insert(key.clone(), index);
		}
	}

	if highest.is_empty() {
		return;
	}

	send_ack_checkpoints(ctx, checkpoints_from(highest)).await;
}

fn checkpoints_from(highest: HashMap<(String, u32), i64>) -> Vec<protocol::ActorCheckpoint> {
	highest
		.into_iter()
		.map(
			|((actor_id, generation), index)| protocol::ActorCheckpoint {
				actor_id: actor_id.into(),
				generation,
				index,
			},
		)
		.collect()
}

/// Send an ack for the given checkpoints. Returns whether the send failed.
async fn send_ack_checkpoints(
	ctx: &EnvoyContext,
	last_command_checkpoints: Vec<protocol::ActorCheckpoint>,
) -> bool {
	ws_send(
		&ctx.shared,
		protocol::ToRivet::ToRivetAckCommands(protocol::ToRivetAckCommands {
			last_command_checkpoints,
		}),
	)
	.await
}
