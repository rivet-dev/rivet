//! One gateway request's wait for an actor to start.
//!
//! The request joins the process's wait for the actor and sends `TryStart` events carrying the
//! wait's start waiter, which wake the actor and register for its start notices: one when the
//! start command goes to an envoy, so the request can be sent right behind it, and one when the
//! actor runs. A notice can be lost, so the wait is paired with a fallback poll: a read at
//! `pegboard.gateway_start_poll_after_ms` after the wait began, then every
//! `pegboard.gateway_start_poll_interval_ms`. The request deadline bounds both.
use crate::start_waits::{Notice, StartWaits, WaitGuard};
use anyhow::{Context, Result};
use pegboard::{keys::partition::ActorVolume, wire};
use rivet_engine_context::EngineContext;
use rivet_util::Id;
use std::sync::Arc;

/// What a waiting request does next.
pub(crate) enum Next {
	/// Read the actor and run the per-phase logic once.
	Read,
	/// Wait for a notice or the fallback poll without reading first: a waiter is registered for
	/// an actor still on its start path.
	Wait,
	/// The actor runs `epoch` on `allocation`; route to it without a read.
	Running {
		epoch: u32,
		allocation: wire::ActorAllocation,
	},
	/// The start command of `epoch` went to `allocation`, and a waiter is stored. Send the
	/// request there at once; the envoy queues it until the actor has started.
	Starting {
		epoch: u32,
		allocation: wire::ActorAllocation,
	},
	/// Another datacenter holds the actor's key, so this gateway cannot serve the request.
	KeyOwnedElsewhere { reservation_id: Id },
}

/// Why the request last looked at the actor again.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Wake {
	Notice,
	Poll,
}

/// The fallback poll schedule of one wait, in boot ms.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PollSchedule {
	next_at_boot_ms: u64,
	interval_ms: u64,
}

impl PollSchedule {
	pub(crate) fn new(began_boot_ms: u64, after_ms: u64, interval_ms: u64) -> Self {
		Self {
			next_at_boot_ms: began_boot_ms.saturating_add(after_ms),
			interval_ms,
		}
	}

	pub(crate) fn next_at_boot_ms(&self) -> u64 {
		self.next_at_boot_ms
	}

	/// Schedules the next poll one interval after `now_boot_ms`, after a poll or a notice.
	pub(crate) fn rearm(&mut self, now_boot_ms: u64) {
		self.next_at_boot_ms = now_boot_ms.saturating_add(self.interval_ms);
	}
}

/// What a request does after `notice`.
pub(crate) fn next_after_notice(notice: Notice) -> Next {
	match notice {
		Notice::Starting { epoch, allocation } => Next::Starting { epoch, allocation },
		Notice::Running { epoch, allocation } => Next::Running { epoch, allocation },
		Notice::EpoxyAlreadyReserved { reservation_id } => {
			Next::KeyOwnedElsewhere { reservation_id }
		}
		// The read finds the actor destroyed and creates it again or reports it missing.
		Notice::Destroyed => Next::Read,
	}
}

/// One request's wait for one actor.
pub(crate) struct StartWait {
	waits: Arc<StartWaits>,
	actor_id: Id,
	/// Whether the request has joined (or tried to join) the process's wait.
	joined: bool,
	guard: Option<WaitGuard>,
	schedule: Option<PollSchedule>,
	/// Why the request last looked again, if it waited at all.
	last_wake: Option<Wake>,
}

impl StartWait {
	pub(crate) fn new(waits: Arc<StartWaits>, actor_id: Id) -> Self {
		Self {
			waits,
			actor_id,
			joined: false,
			guard: None,
			schedule: None,
			last_wake: None,
		}
	}

	/// Why the request last looked at the actor again, if it waited.
	pub(crate) fn last_wake(&self) -> Option<Wake> {
		self.last_wake
	}

	/// Joins the process's wait once and starts the poll schedule.
	async fn join(&mut self, ctx: &EngineContext) -> Result<()> {
		if self.joined {
			return Ok(());
		}
		self.joined = true;
		let now = ctx.env().clock().observe()?.boot_ms;
		let pegboard = ctx.config().pegboard();
		self.schedule = Some(PollSchedule::new(
			now,
			pegboard.gateway_start_poll_after_ms(),
			pegboard.gateway_start_poll_interval_ms(),
		));
		self.guard = self.waits.join(self.actor_id).await;
		if self.guard.is_none() {
			tracing::debug!(actor_id = %self.actor_id, "gateway start waits are full; waiting on the fallback poll alone");
		}
		Ok(())
	}

	/// The waiter to send with an event that may register this request for the actor's start
	/// notices. Joins the wait first. `None` when the process holds its capacity of waits, so no
	/// notice can reach this request. `deadline_unix_ms` is when the request gives up.
	pub(crate) async fn waiter(
		&mut self,
		ctx: &EngineContext,
		deadline_unix_ms: i64,
	) -> Result<Option<wire::StartWaiter>> {
		self.join(ctx).await?;
		let Some(guard) = &self.guard else {
			return Ok(None);
		};
		Ok(Some(wire::StartWaiter {
			worker_id: (ctx.ups()?.worker_id().as_bytes().to_vec()).into(),
			wait_id: guard.wait_id(),
			expires_unix_ms: deadline_unix_ms,
		}))
	}

	/// Handles a read that found the actor not running: sends a `TryStart` with the waiter,
	/// which wakes the actor and registers for its notices, and returns what the answer settled.
	/// A failed send is left to the next read; the fallback poll is the retry.
	pub(crate) async fn try_start(
		&mut self,
		ctx: &EngineContext,
		deadline_unix_ms: i64,
	) -> Result<Option<Next>> {
		let waiter = self.waiter(ctx, deadline_unix_ms).await?;
		let event = wire::ActorEvent::TryStart(wire::TryStart {
			waiter: waiter.clone(),
		});
		let response = match send_event(ctx, self.actor_id, event).await {
			Ok(response) => response,
			Err(error) if pegboard::commands::is_volume_gone(&error) => {
				return Ok(Some(Next::Read));
			}
			Err(error) => {
				tracing::debug!(actor_id = %self.actor_id, ?error, "actor TryStart failed; the fallback poll sends it again");
				return Ok(None);
			}
		};
		let outcome = response
			.outcome
			.context("the actor answered a TryStart without an outcome")?;
		self.on_outcome(waiter.as_ref(), outcome).await
	}

	/// Records the actor's answer to an event that carried `waiter`, or none, and returns the
	/// next step when the answer settles it, or `None` to wait for a notice or poll. Requests
	/// that joined this wait are completed with what the answer settled, since the notice they
	/// expect may never come.
	pub(crate) async fn on_outcome(
		&mut self,
		waiter: Option<&wire::StartWaiter>,
		outcome: wire::TryStartOutcome,
	) -> Result<Option<Next>> {
		let notice = match outcome {
			wire::TryStartOutcome::TryStartWaiting => return Ok(None),
			wire::TryStartOutcome::TryStartFull => {
				tracing::debug!(actor_id = %self.actor_id, "actor start waiter list is full; this request waits on its fallback poll. More gateway workers waited at once than pegboard.actor_start_waiters_capacity allows");
				return Ok(None);
			}
			// The actor is being destroyed, or is destroyed and this request may not create it.
			// A destroy in flight finishes within milliseconds, so the fallback poll's next
			// read, rather than an immediate one, finds it gone and creates it again.
			wire::TryStartOutcome::TryStartDestroyed => return Ok(Some(Next::Wait)),
			wire::TryStartOutcome::TryStartStarting(starting) => Notice::Starting {
				epoch: starting.epoch,
				allocation: starting.allocation,
			},
			wire::TryStartOutcome::TryStartRunning(running) => Notice::Running {
				epoch: running.epoch,
				allocation: running.allocation,
			},
			wire::TryStartOutcome::TryStartEpoxyAlreadyReserved(reserved) => {
				Notice::EpoxyAlreadyReserved {
					reservation_id: Id::from_slice(&reserved.reservation_id)?,
				}
			}
		};
		if let Some(waiter) = waiter {
			self.waits
				.complete(self.actor_id, waiter.wait_id, notice.clone())
				.await;
		}
		Ok(Some(next_after_notice(notice)))
	}

	/// Waits for a notice or the next fallback poll. The caller's deadline bounds it.
	pub(crate) async fn wait(&mut self, ctx: &EngineContext) -> Result<Next> {
		self.join(ctx).await?;
		let clock = ctx.env().clock();
		let poll_at = self
			.schedule
			.map(|schedule| schedule.next_at_boot_ms())
			.unwrap_or(0);

		let wake = match &mut self.guard {
			Some(guard) => {
				rivet_env::select_biased!("a notice that arrives with the poll is the cheaper wake",
					notice = guard.notified() => Ok::<_, anyhow::Error>((Wake::Notice, Some(notice))),
					slept = clock.sleep_until(poll_at) => {
						slept?;
						Ok((Wake::Poll, None))
					},
				)?
			}
			None => {
				clock.sleep_until(poll_at).await?;
				(Wake::Poll, None)
			}
		};

		let now = clock.observe()?.boot_ms;
		if let Some(schedule) = &mut self.schedule {
			schedule.rearm(now);
		}
		self.last_wake = Some(wake.0);
		match wake {
			(Wake::Notice, Some(notice)) => Ok(next_after_notice(notice)),
			(Wake::Notice, None) | (Wake::Poll, _) => Ok(Next::Read),
		}
	}

	/// Leaves the process's wait.
	pub(crate) async fn release(self) {
		if let Some(guard) = self.guard {
			guard.finish().await;
		}
	}
}

/// Sends one event to the actor's state machine through the `ActorEvent` command.
pub(crate) async fn send_event(
	ctx: &EngineContext,
	actor_id: Id,
	event: wire::ActorEvent,
) -> Result<wire::ActorEventResponse> {
	ctx.depot()
		.command::<pegboard::commands::ActorEvent>(&ActorVolume::new(actor_id).volume_key(), event)
		.await
}


