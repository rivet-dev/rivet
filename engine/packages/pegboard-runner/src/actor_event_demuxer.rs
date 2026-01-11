use std::collections::HashMap;
use std::time::{Duration, Instant};

use anyhow::Result;
use gas::prelude::*;
use rivet_runner_protocol as protocol;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use crate::metrics;

struct Channel {
	tx: mpsc::UnboundedSender<protocol::mk2::EventWrapper>,
	handle: JoinHandle<()>,
	last_seen: Instant,
}

pub struct ActorEventDemuxer {
	ctx: StandaloneCtx,
	runner_id: Id,
	channels: HashMap<Id, Channel>,
	last_gc: Instant,
	gc_interval: Duration,
	max_last_seen: Duration,
}

impl ActorEventDemuxer {
	pub fn new(ctx: StandaloneCtx, runner_id: Id) -> Self {
		let pegboard_config = ctx.config().pegboard();
		let gc_interval =
			Duration::from_millis(pegboard_config.runner_event_demuxer_gc_interval_ms());
		let max_last_seen =
			Duration::from_millis(pegboard_config.runner_event_demuxer_max_last_seen_ms());
		Self {
			ctx,
			runner_id,
			channels: HashMap::new(),
			last_gc: Instant::now(),
			gc_interval,
			max_last_seen,
		}
	}

	/// Process an event by routing it to the appropriate actor's queue
	pub fn ingest(&mut self, actor_id: Id, event: protocol::mk2::EventWrapper) {
		tracing::debug!(runner_id=?self.runner_id, ?actor_id, index=?event.checkpoint.index, "actor demuxer ingest");

		if let Some(channel) = self.channels.get_mut(&actor_id) {
			let _ = channel.tx.send(event);
			channel.last_seen = Instant::now();
		} else {
			let (tx, rx) = mpsc::unbounded_channel();

			let handle = tokio::spawn(channel_handler(
				self.ctx.clone(),
				self.runner_id,
				actor_id,
				rx,
			));

			// Send initial event
			let _ = tx.send(event);

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

			self.channels.retain(|_, channel| {
				let keep = channel.last_seen.elapsed() < self.max_last_seen;

				if !keep {
					// TODO: Verify aborting is safe here
					channel.handle.abort();
				}

				keep
			});

			metrics::EVENT_MULTIPLEXER_COUNT.set(self.channels.len() as i64);
		}
	}

	/// Shutdown all tasks and wait for them to complete
	#[tracing::instrument(skip_all)]
	pub async fn shutdown(self) {
		tracing::debug!(channels=?self.channels.len(), "shutting down actor demuxer");

		// Drop all senders
		let handles = self
			.channels
			.into_iter()
			.map(|(_, channel)| channel.handle)
			.collect::<Vec<_>>();

		// Await remaining tasks
		for handle in handles {
			let _ = handle.await;
		}

		tracing::debug!("actor demuxer shut down");
	}
}

#[tracing::instrument(name="demuxer_channel", skip_all, fields(ray_id=?ctx.ray_id(), req_id=?ctx.req_id(), ?runner_id, ?actor_id))]
async fn channel_handler(
	ctx: StandaloneCtx,
	runner_id: Id,
	actor_id: Id,
	mut rx: mpsc::UnboundedReceiver<protocol::mk2::EventWrapper>,
) {
	loop {
		let mut buffer = Vec::new();

		// Batch process events
		if rx.recv_many(&mut buffer, 1024).await == 0 {
			break;
		}

		if let Err(err) = dispatch_events(&ctx, runner_id, actor_id, buffer).await {
			tracing::error!(?err, "actor event processor failed");
			break;
		}
	}
}

#[tracing::instrument(skip_all, fields(?runner_id, ?actor_id))]
async fn dispatch_events(
	ctx: &StandaloneCtx,
	runner_id: Id,
	actor_id: Id,
	events: Vec<protocol::mk2::EventWrapper>,
) -> Result<()> {
	tracing::debug!(count=?events.len(), "actor demuxer dispatch");

	let res = ctx
		.signal(pegboard::workflows::actor::Events { runner_id, events })
		.to_workflow::<pegboard::workflows::actor::Workflow>()
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

	Ok(())
}
