use std::collections::HashMap;
use std::time::{Duration, Instant};

use anyhow::Result;
use gas::prelude::*;
use rivet_envoy_protocol as protocol;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio_util::sync::{CancellationToken, DropGuard};

use crate::metrics;

struct Channel {
	tx: mpsc::UnboundedSender<protocol::EventWrapper>,
	handle: JoinHandle<()>,
	last_seen: Instant,
}

pub struct ActorEventDemuxer {
	ctx: StandaloneCtx,
	envoy_key: String,
	channels: HashMap<Id, Channel>,
	last_gc: Instant,
	gc_interval: Duration,
	max_last_seen: Duration,
	cancellation_token: CancellationToken,
	_guard: DropGuard,
}

impl ActorEventDemuxer {
	pub fn new(ctx: StandaloneCtx, envoy_key: String) -> Self {
		let pegboard_config = ctx.config().pegboard();
		let gc_interval = Duration::from_millis(pegboard_config.envoy_event_demuxer_gc_interval());
		let max_last_seen =
			Duration::from_millis(pegboard_config.envoy_event_demuxer_max_last_seen_threshold());
		let token = CancellationToken::new();
		let drop_guard = token.clone().drop_guard();

		Self {
			ctx,
			envoy_key,
			channels: HashMap::new(),
			last_gc: Instant::now(),
			gc_interval,
			max_last_seen,
			cancellation_token: token,
			_guard: drop_guard,
		}
	}

	/// Process an event by routing it to the appropriate actor's queue
	pub fn ingest(&mut self, actor_id: Id, event: protocol::EventWrapper) {
		let index = event.checkpoint.index;

		tracing::debug!(envoy_key=%self.envoy_key, ?actor_id, %index, "actor demuxer ingest");

		if let Some(channel) = self.channels.get_mut(&actor_id) {
			if channel.tx.send(event).is_err() {
				tracing::warn!(envoy_key=%self.envoy_key, ?actor_id, %index, "failed to ingest event, channel closed");
			}

			channel.last_seen = Instant::now();
		} else {
			let (tx, rx) = mpsc::unbounded_channel();

			let handle = tokio::spawn(channel_handler(
				self.ctx.clone(),
				self.envoy_key.clone(),
				self.cancellation_token.clone(),
				actor_id,
				rx,
			));

			// Send initial event
			if tx.send(event).is_err() {
				tracing::warn!(envoy_key=%self.envoy_key, ?actor_id, %index, "failed to ingest event, channel closed");
			}

			self.channels.insert(
				actor_id,
				Channel {
					tx,
					handle,
					last_seen: Instant::now(),
				},
			);
		}

		metrics::INGESTED_EVENTS_TOTAL.inc();

		// Run gc periodically
		if self.last_gc.elapsed() > self.gc_interval {
			self.last_gc = Instant::now();

			// This will drop the tx side of the channel and it's task will exit automatically. No abort
			// required.
			self.channels.retain(|_, channel| {
				!channel.handle.is_finished() && channel.last_seen.elapsed() < self.max_last_seen
			});

			metrics::EVENT_DEMUXER_COUNT.set(self.channels.len() as i64);
		}
	}

	/// Shutdown all tasks and wait for them to complete
	#[tracing::instrument(level = "debug", skip_all)]
	pub async fn shutdown(self) {
		tracing::debug!(channels=?self.channels.len(), "shutting down actor demuxer");

		// Drop all senders
		let handles = self
			.channels
			.into_iter()
			.map(|(_, channel)| channel.handle)
			.collect::<Vec<_>>();

		// Handlers still drain buffered events, but stop retrying failed dispatches.
		// The envoy resends unacknowledged events when it reconnects.
		self.cancellation_token.cancel();

		// Await remaining tasks
		for handle in handles {
			let _ = handle.await;
		}

		tracing::debug!("actor demuxer shut down");
	}
}

#[tracing::instrument(name="demuxer_channel", skip_all, fields(ray_id=?ctx.ray_id(), req_id=?ctx.req_id(), %envoy_key, ?actor_id))]
async fn channel_handler(
	ctx: StandaloneCtx,
	envoy_key: String,
	cancellation_token: CancellationToken,
	actor_id: Id,
	mut rx: mpsc::UnboundedReceiver<protocol::EventWrapper>,
) {
	loop {
		let mut buffer = Vec::new();

		// Batch process events. This returns 0 only once every sender is dropped,
		// so events buffered before shutdown still dispatch.
		if rx.recv_many(&mut buffer, 1024).await == 0 {
			break;
		}

		// Retry the same batch until it lands so later events never skip past it.
		// The actor workflow ignores events it already received.
		let mut backoff = rivet_util::throttle::Backoff::new(5, None, 250, 250);
		loop {
			match dispatch_events(&ctx, &envoy_key, actor_id, &buffer).await {
				Ok(()) => break,
				Err(err) => {
					tracing::warn!(
						%envoy_key,
						?actor_id,
						?err,
						attempt = backoff.tick_index(),
						count = buffer.len(),
						"actor event dispatch failed, retrying, storage may be unavailable"
					);
					tokio::select! {
						_ = backoff.tick() => {}
						_ = cancellation_token.cancelled() => return,
					}
				}
			}
		}
	}
}

#[tracing::instrument(level = "debug", skip_all, fields(%envoy_key, ?actor_id))]
async fn dispatch_events(
	ctx: &StandaloneCtx,
	envoy_key: &str,
	actor_id: Id,
	events: &[protocol::EventWrapper],
) -> Result<()> {
	tracing::debug!(%envoy_key, ?actor_id, count=?events.len(), "actor demuxer dispatch");

	let res = ctx
		.signal(pegboard::workflows::actor2::Events {
			envoy_key: envoy_key.to_string(),
			events: events.to_vec(),
		})
		.to_workflow::<pegboard::workflows::actor2::Workflow>()
		.tag("actor_id", actor_id)
		.graceful_not_found()
		.send()
		.await
		.with_context(|| format!("failed to forward signal to actor workflow: {}", actor_id))?;
	if res.is_none() {
		tracing::warn!(
			?actor_id,
			"failed to send signal to actor workflow, likely already stopped"
		);
	}

	tracing::debug!(%envoy_key, ?actor_id, "actor demuxer dispatch success");

	Ok(())
}
