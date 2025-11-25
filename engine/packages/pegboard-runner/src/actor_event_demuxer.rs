use std::collections::HashMap;
use std::time::{Duration, Instant};

use anyhow::Result;
use gas::prelude::*;
use rivet_runner_protocol as protocol;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

const GC_INTERVAL: Duration = Duration::from_secs(30);
const MAX_LAST_SEEN: Duration = Duration::from_secs(30);

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
}

impl ActorEventDemuxer {
	pub fn new(ctx: StandaloneCtx, runner_id: Id) -> Self {
		Self {
			ctx,
			runner_id,
			channels: HashMap::new(),
			last_gc: Instant::now(),
		}
	}

	/// Process an event by routing it to the appropriate actor's queue
	pub fn ingest(&mut self, actor_id: Id, event: protocol::mk2::EventWrapper) {
		tracing::debug!(runner_id=?self.runner_id, ?actor_id, index=?event.checkpoint.index, "actor demuxer ingest");

		if let Some(channel) = self.channels.get(&actor_id) {
			let _ = channel.tx.send(event);
		} else {
			let (tx, mut rx) = mpsc::unbounded_channel();

			let ctx = self.ctx.clone();
			let runner_id = self.runner_id;
			let handle = tokio::spawn(async move {
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
			});

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

		// Run gc periodically
		if self.last_gc.elapsed() > GC_INTERVAL {
			self.last_gc = Instant::now();

			self.channels.retain(|_, channel| {
				let keep = channel.last_seen.elapsed() < MAX_LAST_SEEN;

				if !keep {
					// TODO: Verify aborting is safe here
					channel.handle.abort();
				}

				keep
			});
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
