use std::sync::atomic::{AtomicI64, Ordering};

use anyhow::*;
use pegboard::protocol;
use tokio::sync::broadcast;

use crate::Ctx;

/// Handles sending events in a sequentially consistent order.
pub struct EventSender {
	awaiting_event_idx: AtomicI64,
	tx: broadcast::Sender<i64>,
}

impl EventSender {
	pub fn new() -> Self {
		EventSender {
			awaiting_event_idx: AtomicI64::new(0),
			tx: broadcast::channel(4).0,
		}
	}

	pub fn set_idx(&self, idx: i64) {
		self.awaiting_event_idx.store(idx, Ordering::SeqCst);
	}

	pub async fn send(&self, ctx: &Ctx, event: protocol::Event, idx: i64) -> Result<()> {
		// Subscribe before checking the idx
		let mut rx = self.tx.subscribe();

		// Read source of truth
		if idx != self.awaiting_event_idx.load(Ordering::SeqCst) {
			// Wait for idx from channel
			loop {
				if rx.recv().await? == idx {
					break;
				}
			}
		}

		// Drop receiver so it does not become a "slow receiver"
		drop(rx);

		let wrapped_event = protocol::EventWrapper {
			index: idx,
			inner: protocol::Raw::new(&event)?,
		};

		ctx.send_packet(protocol::ToServer::Events(vec![wrapped_event]))
			.await?;

		// Increment idx only after sending. We don't use `fetch_add` because we need the next value to be
		// exactly `idx + 1`. This should always be the case anyway.
		self.awaiting_event_idx.store(idx + 1, Ordering::SeqCst);
		// An error means there are currently no receivers
		let _ = self.tx.send(idx + 1);

		Ok(())
	}
}
