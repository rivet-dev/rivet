//! Tells an actor's hibernated WebSockets on this gateway that the actor runs again.
//!
//! A hibernatable WebSocket whose actor went to sleep stays open at the gateway and re-opens its
//! tunnel when its own client sends a frame. The woken actor restores every hibernated
//! connection and may send on any of them, but a connection without a tunnel cannot deliver
//! what the actor sends. So every socket of the actor on this gateway shares one record of the
//! latest epoch the gateway reached the actor at, and a hibernated socket re-opens its tunnel
//! as soon as that epoch is newer than the one it went to sleep on.
//!
//! The record holds the latest epoch, so a socket that learns of its actor's stop after a
//! sibling already woke the actor still sees the newer epoch and re-opens at once.
//!
//! A newer epoch only tells a socket to look for its actor. The re-open it triggers attaches to
//! an actor that runs and never starts one (`Reach::AttachOnly` in `acquire.rs`), so an actor
//! that went back to sleep before a socket re-attached stays asleep, and the socket then waits
//! for an epoch newer than the one it looked for. Only a request a client made starts an
//! actor, so every epoch in the record goes back to one.
//!
//! The record is local to one gateway process. An actor woken without a request through this
//! gateway (an alarm, or a request through another gateway node) does not re-attach the sockets
//! hibernated here until a request for it passes through this gateway.

use rivet_util::Id;
use std::sync::{Arc, Weak};
use tokio::sync::watch;

/// The latest epoch this gateway reached each actor at, kept only for actors with an open
/// WebSocket.
#[derive(Default)]
pub struct SiblingWakes {
	actors: scc::HashMap<Id, Weak<watch::Sender<u32>>>,
}

impl SiblingWakes {
	pub fn new() -> Arc<Self> {
		Arc::new(Self::default())
	}

	/// Joins the actor's record for the life of one WebSocket.
	pub async fn subscribe(self: &Arc<Self>, actor_id: Id) -> SiblingWake {
		let new_record = || Arc::new(watch::channel(0).0);
		let epochs = match self.actors.entry_async(actor_id).await {
			scc::hash_map::Entry::Occupied(mut occupied) => match occupied.get().upgrade() {
				Some(epochs) => epochs,
				// The last socket is being dropped and has not removed the entry yet.
				None => {
					let epochs = new_record();
					*occupied.get_mut() = Arc::downgrade(&epochs);
					epochs
				}
			},
			scc::hash_map::Entry::Vacant(vacant) => {
				let epochs = new_record();
				vacant.insert_entry(Arc::downgrade(&epochs));
				epochs
			}
		};
		SiblingWake {
			updates: epochs.subscribe(),
			epochs: Some(epochs),
			wakes: self.clone(),
			actor_id,
		}
	}

	/// Records that the gateway reached the actor at `epoch`. Costs one map read when the actor
	/// has no WebSocket on this gateway.
	pub async fn publish(&self, actor_id: Id, epoch: u32) {
		let epochs = self
			.actors
			.read_async(&actor_id, |_, epochs| epochs.upgrade())
			.await
			.flatten();
		if let Some(epochs) = epochs {
			advance(&epochs, epoch);
		}
	}

	#[cfg(test)]
	fn len(&self) -> usize {
		self.actors.len()
	}
}

fn advance(epochs: &watch::Sender<u32>, epoch: u32) {
	epochs.send_if_modified(|latest| {
		if epoch > *latest {
			*latest = epoch;
			true
		} else {
			false
		}
	});
}

/// One WebSocket's membership in its actor's record. Dropping the last one removes the record.
pub struct SiblingWake {
	epochs: Option<Arc<watch::Sender<u32>>>,
	updates: watch::Receiver<u32>,
	wakes: Arc<SiblingWakes>,
	actor_id: Id,
}

impl SiblingWake {
	/// Records that this socket reached the actor at `epoch`.
	pub fn publish(&self, epoch: u32) {
		if let Some(epochs) = &self.epochs {
			advance(epochs, epoch);
		}
	}

	/// The latest epoch the gateway reached the actor at.
	pub fn latest(&self) -> u32 {
		*self.updates.borrow()
	}

	/// Resolves once the gateway reached the actor at an epoch after `epoch`, at once if it
	/// already did.
	pub async fn newer_than(&self, epoch: u32) {
		let mut updates = self.updates.clone();
		if updates.wait_for(|latest| *latest > epoch).await.is_err() {
			// The record cannot close while this socket holds it.
			std::future::pending::<()>().await;
		}
	}
}

impl Drop for SiblingWake {
	fn drop(&mut self) {
		// Release the record first: the entry is removed only once no socket holds it, and a
		// socket that joins meanwhile replaces the dead entry under the same entry lock.
		self.epochs = None;
		self.wakes
			.actors
			.remove_if_sync(&self.actor_id, |epochs| epochs.strong_count() == 0);
	}
}

#[cfg(test)]
#[allow(
	clippy::disallowed_methods,
	reason = "the tests wait on the record with a tokio runtime"
)]
mod tests {
	use super::*;
	use std::time::Duration;

	fn actor(n: u128) -> Id {
		Id::v1(uuid::Uuid::from_u128(n), 1)
	}

	async fn resolves(wake: &SiblingWake, epoch: u32) -> bool {
		tokio::time::timeout(Duration::from_millis(100), wake.newer_than(epoch))
			.await
			.is_ok()
	}

	#[tokio::test]
	async fn a_newer_epoch_wakes_every_sibling_and_an_older_one_does_not() {
		let wakes = SiblingWakes::new();
		let first = wakes.subscribe(actor(1)).await;
		let second = wakes.subscribe(actor(1)).await;
		let other_actor = wakes.subscribe(actor(2)).await;

		first.publish(3);
		assert!(!resolves(&second, 3).await, "the same epoch is not a wake");

		first.publish(4);
		assert_eq!(second.latest(), 4);
		assert!(resolves(&second, 3).await);
		assert!(resolves(&first, 3).await);
		assert!(!resolves(&other_actor, 3).await, "wakes are per actor");

		// An epoch never moves back.
		first.publish(2);
		assert!(!resolves(&second, 4).await);
	}

	#[tokio::test]
	async fn a_wake_published_before_the_sibling_waits_is_still_seen() {
		let wakes = SiblingWakes::new();
		let sibling = wakes.subscribe(actor(1)).await;
		wakes.publish(actor(1), 7).await;
		assert!(resolves(&sibling, 6).await);
	}

	#[tokio::test]
	async fn the_record_is_removed_with_its_last_socket() {
		let wakes = SiblingWakes::new();
		let first = wakes.subscribe(actor(1)).await;
		let second = wakes.subscribe(actor(1)).await;
		assert_eq!(wakes.len(), 1);
		drop(first);
		assert_eq!(wakes.len(), 1);
		second.publish(5);
		drop(second);
		assert_eq!(wakes.len(), 0);

		// Publishing for an actor without a socket keeps nothing.
		wakes.publish(actor(1), 9).await;
		assert_eq!(wakes.len(), 0);
		// A later socket starts a new record and does not see the old epoch.
		let later = wakes.subscribe(actor(1)).await;
		assert!(!resolves(&later, 0).await);
	}
}
