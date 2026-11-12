//! Creates on this process waiting for a namespace's group create or fork to finish.
//!
//! A create joins a wait, which subscribes to its own notice subject, and sends its create with
//! the wait's waiter. The namespace machine stores the waiter and sends it the namespace's state
//! when the namespace leaves `Creating` or `Forking`. A wait id is a per-process counter: the
//! worker id is fresh per process start, so `(worker_id, wait_id)` never repeats. A notice is
//! lossy, so the create reads the namespace once at its deadline when none arrives.
use std::sync::{
	Arc,
	atomic::{AtomicU64, AtomicUsize, Ordering},
};

use anyhow::{Context, Result, ensure};
use rivet_data::generated::pegboard_namespace_v1 as ns;
use vbare::Versioned;

use crate::keys::ups::NamespaceStartNoticeSubject;

/// The process's namespace start waits, owned by the HTTP router.
pub struct StartWaits {
	/// Where waits subscribe for their notices. `None` leaves every wait to its deadline read.
	ups: Option<universalpubsub::PubSub>,
	capacity: usize,
	len: AtomicUsize,
	next_wait_id: AtomicU64,
}

impl StartWaits {
	pub fn new(ups: Option<universalpubsub::PubSub>, capacity: usize) -> Arc<Self> {
		Arc::new(Self {
			ups,
			capacity: capacity.max(1),
			len: AtomicUsize::new(0),
			next_wait_id: AtomicU64::new(1),
		})
	}

	/// Starts a wait for the namespace `name` and subscribes it to its notice subject. Returns
	/// `None` when the process holds its capacity of waits or cannot subscribe, so the caller
	/// reads the namespace at its deadline instead.
	pub fn join(self: &Arc<Self>, name: &str) -> Option<Wait> {
		let ups = self.ups.as_ref()?;
		if self.len.fetch_add(1, Ordering::AcqRel) >= self.capacity {
			self.len.fetch_sub(1, Ordering::AcqRel);
			tracing::warn!(
				capacity = self.capacity,
				"namespace start waits are full; creates read the namespace at their deadline. Raise pegboard.gateway_start_waits_capacity if many namespaces are created at once"
			);
			return None;
		}
		let wait_id = self.next_wait_id.fetch_add(1, Ordering::Relaxed);
		let subscriber = match ups.subscribe_request(NamespaceStartNoticeSubject::new(wait_id)) {
			Ok(subscriber) => subscriber,
			Err(error) => {
				self.len.fetch_sub(1, Ordering::AcqRel);
				tracing::debug!(
					namespace = name,
					wait_id,
					?error,
					"namespace start notice subscription failed; the create reads at its deadline"
				);
				return None;
			}
		};
		Some(Wait {
			waits: self.clone(),
			worker_id: ups.worker_id().as_bytes().to_vec(),
			name: name.to_owned(),
			wait_id,
			subscriber: Some(subscriber),
		})
	}
}

/// One create's wait for one namespace. Dropping it ends the subscription.
pub struct Wait {
	waits: Arc<StartWaits>,
	worker_id: Vec<u8>,
	name: String,
	wait_id: u64,
	/// `None` once the subscription ended; the create then waits for its deadline.
	subscriber: Option<universalpubsub::RequestSubscriber>,
}

impl Wait {
	/// The waiter to send with a create, which registers this wait for the namespace's notice
	/// until `expires_unix_ms`, when the create gives up.
	pub fn waiter(&self, expires_unix_ms: i64) -> ns::NamespaceWaiter {
		ns::NamespaceWaiter {
			worker_id: self.worker_id.clone().into(),
			wait_id: self.wait_id,
			expires_unix_ms,
		}
	}

	/// Waits for the next valid notice. Pends forever once the subscription ended, so the
	/// caller's deadline decides.
	pub async fn notified(&mut self) -> ns::NamespaceState {
		loop {
			let Some(subscriber) = self.subscriber.as_mut() else {
				return std::future::pending().await;
			};
			let request = match subscriber.next().await {
				Ok(Some(request)) => request,
				Ok(None) => {
					self.subscriber = None;
					continue;
				}
				Err(error) => {
					tracing::debug!(namespace = %self.name, wait_id = self.wait_id, ?error, "namespace start notice subscription ended; the create reads at its deadline");
					self.subscriber = None;
					continue;
				}
			};
			match decode(&request, &self.name) {
				Ok(state) => {
					let _ = request.reply(&[]);
					return state;
				}
				Err(error) => {
					tracing::warn!(
						namespace = %self.name,
						wait_id = self.wait_id,
						?error,
						"rejecting an invalid namespace start notice; the sender is out of date or misrouted"
					);
					let _ = request.fail(&error).await;
				}
			}
		}
	}
}

impl Drop for Wait {
	fn drop(&mut self) {
		self.waits.len.fetch_sub(1, Ordering::AcqRel);
	}
}

/// Decodes a notice for the wait of the namespace `name`. A notice naming another namespace
/// was misrouted.
fn decode(request: &universalpubsub::RequestMessage, name: &str) -> Result<ns::NamespaceState> {
	ensure!(
		request.class() == universalpubsub::RequestClass::ActorStartNotice,
		"namespace start notice sent under the {} class",
		request.class().as_str()
	);
	let notice = rivet_data::versioned::NamespaceStartNotice::deserialize_with_embedded_version(
		&request.payload,
	)
	.context("invalid namespace start notice")?;
	ensure!(
		&*notice.name == name,
		"start notice for another namespace on this wait's subject"
	);
	Ok(notice.state)
}
