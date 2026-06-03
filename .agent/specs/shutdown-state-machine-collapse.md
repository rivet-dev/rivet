# Shutdown State Machine Collapse

## Problem

`ActorTask` in `rivetkit-rust/packages/rivetkit-core/src/actor/task.rs` carries two pieces of
scaffolding that exist to support behaviors that do not actually occur under the engine actor2
workflow:

1. `shutdown_replies: Vec<PendingLifecycleReply>` (task.rs:523), with a fan-out loop in
   `send_shutdown_replies` (task.rs:1929) and two "Stop arrives during shutdown" re-entry arms at
   `begin_stop` (task.rs:816) and `handle_sleep_grace_lifecycle` (task.rs:743). This exists so N
   `Stop` / `Destroy` commands for the same instance can all receive the same final reply.
2. A boxed-future shutdown state machine: `shutdown_step: Option<ShutdownStep>` (task.rs:526),
   `shutdown_finalize_reply: Option<oneshot::Receiver<…>>` (task.rs:532), plus
   `shutdown_phase` / `shutdown_reason` / `shutdown_deadline` / `shutdown_started_at` (task.rs:510,
   513, 516, 519), driven by the `ShutdownPhase` enum (task.rs:410) and the
   `install_shutdown_step` / `on_shutdown_step_complete` / `boxed_shutdown_step` /
   `poll_shutdown_step` / `drive_shutdown_to_completion` helpers
   (task.rs:1562, 1538, 1752, 1529, 1521). The state machine exists so each shutdown phase runs
   as a `select!` arm alongside the actor's inbox/event/timer arms. It lets the main loop keep
   servicing `lifecycle_inbox`, `lifecycle_events`, `dispatch_inbox`, activity, sleep grace, and
   timers *while* finalize/drain/await‑run are in flight.

Neither capability is load‑bearing:

- **Multi-reply fan-out** requires the engine to send more than one `Stop` per actor instance.
  It doesn't (see "Engine actor2 invariant" below).
- **Concurrent finalize** requires the main loop to do useful work during finalize. Once the
  actor enters `LifecycleState::SleepFinalize` / `Destroying`:
  - `accepting_dispatch()` (task.rs:1960) returns `false` (only `Started | SleepGrace`), so the
    dispatch arm is dead.
  - `fire_due_alarms()` (task.rs:1315) early-returns for the same reason — alarms are suspended
    via `ctx.suspend_alarm_dispatch()` at the top of `enter_shutdown_state_machine` (task.rs:1460).
  - `schedule_state_save` early-returns (task.rs:1979), so `state_save_tick` does not arm.
  - `begin_stop` during `SleepFinalize | Destroying` is unreachable under the engine invariant.
  - `Destroy` no longer preempts `Sleep` at this stage — `begin_stop` in the
    `SleepFinalize | Destroying` branch just registers another reply (task.rs:816) and takes no
    further action. Preemption only happens during `SleepGrace`, and `SleepGrace` is a separate
    concurrent state (its own `select!` arm on `sleep_grace: Option<SleepGraceState>`,
    task.rs:529, and is not part of this spec).

So finalize is terminal. A straight `async fn run_shutdown(&mut self, reason)` with plain `.await`
between phases gives identical behavior with none of the scaffolding.

## Engine actor2 invariant

The engine actor2 workflow at `engine/packages/pegboard/src/workflows/actor2/mod.rs` enforces "at
most one `CommandStopActor` per actor instance" via its `Transition` state machine:

- `Main::Events` (mod.rs:631, :655): only sends `CommandStopActor` when `Transition::Running`.
  `SleepIntent` / `StopIntent` / `Sleeping` / `Destroying` arms are no-op.
- `Main::Reschedule` (mod.rs:883): when in `SleepIntent` / `StopIntent`, transitions to
  `GoingAway` but explicitly skips sending. Comment: `// Stop command was already sent`
  (mod.rs:914).
- `Main::Destroy` (mod.rs:990): when already in `SleepIntent` / `StopIntent` / `GoingAway`,
  transitions to `Destroying` but explicitly skips sending. Comment: `// Stop command was already
  sent` (mod.rs:1023).

That single `CommandStopActor` flows: `pegboard-envoy` → `envoy-client` →
`EnvoyCallbacks::on_actor_stop_with_completion` → `RegistryDispatcher::stop_actor`
(`rivetkit-rust/packages/rivetkit-core/src/registry/mod.rs:737-768`) → exactly one
`LifecycleCommand::Stop` on `lifecycle_inbox`.

The only other sender of `LifecycleCommand::Stop` in core is the test-only `ActorTask::handle_stop`
(task.rs:759, `#[cfg_attr(not(test), allow(dead_code))]`). It is a single-shot helper: each call
constructs its own oneshot, sends one Stop, awaits its own reply. No test issues concurrent Stops.

## Runtime Contract

- Core treats "exactly one `LifecycleCommand::Stop` per actor instance" as an invariant supplied by
  the engine actor2 workflow. A second `Stop` reaching `ActorTask` is a bug.
- Duplicate-Stop handling: `debug_assert!` in dev/test; release-mode warn‑and‑drop the new sender
  (keep the original reply, log at `tracing::warn!` level).
- Reply delivery semantics for the surviving single reply are unchanged: caller hands in a
  `oneshot::Sender<Result<()>>`, receives one `Ok/Err` when the shutdown state machine completes.
- Finalize is terminal. The main `select!` loop exits with a `ShutdownTrigger { reason }`, and the
  rest of shutdown runs inline as a single `async fn`. No concurrent inbox servicing.
- Sleep grace stays a `select!` arm in the main loop (out of scope here; US-104 owns that path).
  The boundary between grace and finalize is the one place the main loop still breaks out into
  the inline shutdown function.
- Panic isolation: one `AssertUnwindSafe + catch_unwind` wrapper at the `run_shutdown` call site
  replaces per-phase wrapping inside `boxed_shutdown_step`.

## Design

### Main-loop control flow

```rust
pub async fn run(mut self) -> Result<()> {
    self.startup().await?;
    let trigger = self.run_live().await;       // existing select! loop, minus the shutdown_step arm
    let reason  = match trigger {
        LiveExit::Shutdown { reason } => reason,
        LiveExit::Terminated          => return Ok(()),  // nothing to finalize
    };
    let result  = match AssertUnwindSafe(self.run_shutdown(reason)).catch_unwind().await {
        Ok(r)  => r,
        Err(_) => Err(anyhow!("shutdown panicked during {reason:?}")),
    };
    if matches!(reason, StopReason::Destroy) && result.is_ok() {
        self.ctx.mark_destroy_completed();
    }
    if let Some(pending) = self.shutdown_reply.take() {
        let delivered = pending.reply.send(clone_shutdown_result(&result)).is_ok();
        tracing::debug!(
            actor_id = %self.ctx.actor_id(),
            command = pending.command,
            reason = pending.reason,
            outcome = result_outcome(&result),
            delivered,
            "actor lifecycle command replied",
        );
    }
    self.transition_to(LifecycleState::Terminated);
    result
}
```

Trigger sources — the live loop exits with a `LiveExit::Shutdown { reason }` in these cases:

- `begin_stop(Destroy, Started)` (task.rs:799-801): capture the reply into `self.shutdown_reply`,
  exit with `{ reason: Destroy }`.
- Sleep grace completion (`on_sleep_grace_complete`, task.rs:1403): exit with `{ reason: Sleep }`.
  The originating `begin_stop(Sleep, Started)` already captured the reply into
  `self.shutdown_reply` before starting grace.

Paths that do NOT produce a shutdown trigger (preserve existing behavior):

- `handle_run_handle_outcome` (task.rs:1326): when the user's `run` handler exits on its own
  with `sleep_requested()` or `destroy_requested()` set, today the code only transitions
  `self.lifecycle` to `SleepFinalize` / `Destroying` and returns. The main loop keeps spinning
  until an inbound `LifecycleCommand::Stop` arrives and drives shutdown via `begin_stop`. The
  new design MUST preserve this behavior — do not short‑circuit the live loop into
  `run_shutdown` from the run‑handle arm. (An earlier draft of this spec proposed exiting the
  live loop directly from this arm; that was a silent behavior change and is rejected.)
- Inbound `Stop` arriving in `LifecycleState::SleepFinalize | Destroying` (task.rs:816-818):
  under the engine actor2 one‑Stop invariant this is unreachable. The arm becomes
  `debug_assert!(false, "engine actor2 sends one Stop per actor instance")` + release‑mode
  `tracing::warn!` + immediate `Ok(())` ack via `reply_lifecycle_command` (no
  `register_shutdown_reply`).
- Inbound `Stop(Sleep)` during `SleepGrace` (`handle_sleep_grace_lifecycle`, task.rs:737-742):
  existing idempotent `Ok(())` ack stays as‑is (defensive no‑op; cheap under the invariant).
- Inbound `Stop(Destroy)` during `SleepGrace` (`handle_sleep_grace_lifecycle`, task.rs:743-750):
  under the engine actor2 one‑Stop invariant this would be a *second* Stop and is unreachable.
  Collapse to the same `debug_assert!` + release‑mode `tracing::warn!` + immediate `Ok(())` ack
  as the `SleepFinalize | Destroying` arm. Do NOT escalate into a Destroy‑shutdown from this
  path; escalation could only be triggered by a legitimate second command the invariant forbids.
  (An earlier draft kept this arm wired to capture the reply and clear `sleep_grace`; that is
  inconsistent with the one‑Stop invariant and is rejected.)

### `run_shutdown`

```rust
async fn run_shutdown(&mut self, reason: StopReason) -> Result<()> {
    // Prologue (formerly enter_shutdown_state_machine, task.rs:1420-1464)
    let started_at = Instant::now();
    let deadline = started_at + match reason {
        StopReason::Sleep   => self.factory.config().effective_sleep_grace_period(),
        StopReason::Destroy => self.factory.config().effective_on_destroy_timeout(),
    };
    self.transition_to(match reason {
        StopReason::Sleep   => LifecycleState::SleepFinalize,
        StopReason::Destroy => LifecycleState::Destroying,
    });
    if matches!(reason, StopReason::Destroy) {
        for conn in self.ctx.conns() {
            if conn.is_hibernatable() {
                self.ctx.request_hibernation_transport_removal(conn.id().to_owned());
            }
        }
    }
    self.state_save_deadline = None;
    self.inspector_serialize_state_deadline = None;
    self.sleep_deadline = None;
    self.ctx.cancel_sleep_timer();
    self.ctx.suspend_alarm_dispatch();
    self.ctx.cancel_local_alarm_timeouts();
    self.ctx.set_local_alarm_callback(None);

    // Phase 1: SendingFinalize + AwaitingFinalizeReply fused
    let (reply_tx, reply_rx) = oneshot::channel();
    let on_state_change_timeout = self.factory.config().action_timeout;
    if !self.ctx.wait_for_on_state_change_idle(on_state_change_timeout).await {
        tracing::warn!(
            actor_id = %self.ctx.actor_id(),
            reason = shutdown_reason_label(reason),
            timeout_ms = on_state_change_timeout.as_millis() as u64,
            "actor shutdown timed out waiting for on_state_change callback",
        );
    }
    if let Some(sender) = self.actor_event_tx.clone() {
        let event = match reason {
            StopReason::Sleep   => ActorEvent::FinalizeSleep { reply: Reply::from(reply_tx) },
            StopReason::Destroy => ActorEvent::Destroy        { reply: Reply::from(reply_tx) },
        };
        if let Ok(permit) = sender.try_reserve_owned() {
            permit.send(event);
        } else {
            tracing::warn!(reason = shutdown_reason_label(reason), "failed to enqueue shutdown event");
        }
    }
    match timeout(remaining_shutdown_budget(deadline), reply_rx).await {
        Ok(Ok(Ok(())))  => {}
        Ok(Ok(Err(e)))  => tracing::error!(?e, reason = shutdown_reason_label(reason), "actor shutdown event failed"),
        Ok(Err(e))      => tracing::error!(?e, reason = shutdown_reason_label(reason), "actor shutdown reply dropped"),
        Err(_)          => tracing::warn!(reason = shutdown_reason_label(reason), "actor shutdown event timed out"),
    }

    // Phase 2: DrainingBefore
    if !Self::drain_tracked_work_with_ctx(self.ctx.clone(), reason, "before_disconnect", deadline).await {
        self.ctx.record_shutdown_timeout(reason);
        tracing::warn!(reason = shutdown_reason_label(reason), "shutdown timed out waiting for shutdown tasks");
    }

    // Phase 3: DisconnectingConns
    Self::disconnect_for_shutdown_with_ctx(
        self.ctx.clone(),
        match reason { StopReason::Sleep => "actor sleeping", StopReason::Destroy => "actor destroyed" },
        matches!(reason, StopReason::Sleep),
    ).await?;

    // Phase 4: DrainingAfter
    if !Self::drain_tracked_work_with_ctx(self.ctx.clone(), reason, "after_disconnect", deadline).await {
        self.ctx.record_shutdown_timeout(reason);
        tracing::warn!(reason = shutdown_reason_label(reason), "shutdown timed out after disconnect callbacks");
    }

    // Phase 5: AwaitingRunHandle
    self.close_actor_event_channel();
    if let Some(mut run_handle) = self.run_handle.take() {
        tokio::select! {
            outcome = &mut run_handle => match outcome {
                Ok(Ok(())) => {}
                Ok(Err(e)) => tracing::error!(?e, "actor run handler failed during shutdown"),
                Err(e)     => tracing::error!(?e, "actor run handler join failed during shutdown"),
            },
            _ = sleep(remaining_shutdown_budget(deadline)) => {
                run_handle.abort();
                tracing::warn!(reason = shutdown_reason_label(reason), "actor run handler timed out during shutdown");
            }
        }
    }

    // Phase 6: Finalizing (existing finish_shutdown_cleanup_with_ctx body, task.rs:1811-1905)
    Self::finish_shutdown_cleanup_with_ctx(self.ctx.clone(), reason).await?;

    if let Some(duration) = started_at.elapsed().into() {
        self.ctx.record_shutdown_wait(reason, duration);
    }
    Ok(())
}
```

Each former phase is one `.await` point. Deadlines still enforced via
`timeout(remaining_shutdown_budget(deadline), …)` and explicit `sleep(…)` arms. The body uses the
existing helpers (`drain_tracked_work_with_ctx`, `disconnect_for_shutdown_with_ctx`,
`finish_shutdown_cleanup_with_ctx`) unchanged.

## Field And Function Changes

### Fields removed from `ActorTask`

- `shutdown_phase: Option<ShutdownPhase>` (task.rs:510)
- `shutdown_reason: Option<StopReason>` (task.rs:513) — becomes a local in `run_shutdown`
- `shutdown_deadline: Option<Instant>` (task.rs:516) — local
- `shutdown_started_at: Option<Instant>` (task.rs:519) — local
- `shutdown_step: Option<ShutdownStep>` (task.rs:526)
- `shutdown_finalize_reply: Option<oneshot::Receiver<Result<()>>>` (task.rs:532)

### Field replaced

- `shutdown_replies: Vec<PendingLifecycleReply>` (task.rs:523) → `shutdown_reply: Option<PendingLifecycleReply>`.
  Doc comment explains the engine-supplied one-Stop invariant.

### Types removed

- `enum ShutdownPhase` (task.rs:410) — delete.
- `type ShutdownStep = Pin<Box<dyn Future<…>>>` (task.rs:421) — delete.
- `fn shutdown_phase_label(ShutdownPhase) -> &'static str` (task.rs:2312 area) — delete.

### Functions removed

- `install_shutdown_step` (task.rs:1562)
- `on_shutdown_step_complete` (task.rs:1538)
- `boxed_shutdown_step` (task.rs:1752)
- `poll_shutdown_step` (task.rs:1529)
- `drive_shutdown_to_completion` (task.rs:1521) — `handle_stop` (test-only) is rewritten to call
  `run_shutdown` directly.
- `enter_shutdown_state_machine` (task.rs:1420) — body inlined as the prologue of `run_shutdown`.
- `complete_shutdown` (task.rs:1907) — body inlined at the end of `run`.
- `send_shutdown_replies` (task.rs:1929) — body inlined at the end of `run` as a single
  `if let Some(pending) = …` block.

### Functions kept

- `drain_tracked_work_with_ctx` (task.rs:1764)
- `disconnect_for_shutdown_with_ctx` (task.rs:1788)
- `finish_shutdown_cleanup_with_ctx` (task.rs:1811)
- `close_actor_event_channel` (task.rs:1370)
- `register_shutdown_reply` (task.rs:1507) — body becomes
  `debug_assert!(self.shutdown_reply.is_none(), …)` plus `self.shutdown_reply = Some(…)`; release
  path keeps the existing reply and logs a `tracing::warn!` on the dropped duplicate.
- `handle_stop` (task.rs:759, test-only) — rewritten as:

  ```rust
  #[cfg_attr(not(test), allow(dead_code))]
  async fn handle_stop(&mut self, reason: StopReason) -> Result<()> {
      let (reply_tx, reply_rx) = oneshot::channel();
      self.shutdown_reply = Some(PendingLifecycleReply {
          command: "stop",
          reason: Some(shutdown_reason_label(reason)),
          reply: reply_tx,
      });
      // For Sleep, simulate the grace drain that the live loop would otherwise do.
      if matches!(reason, StopReason::Sleep) {
          self.transition_to(LifecycleState::SleepGrace);
          self.start_sleep_grace();
          while self.sleep_grace.is_some() {
              let idle_ready = Self::poll_sleep_grace(self.sleep_grace.as_mut()).await;
              self.on_sleep_grace_complete(idle_ready).await;
          }
      }
      // Run the inline finalize directly. Panic handling matches the `run` call site.
      let result = match AssertUnwindSafe(self.run_shutdown(reason)).catch_unwind().await {
          Ok(r)  => r,
          Err(_) => Err(anyhow!("shutdown panicked during {reason:?}")),
      };
      if matches!(reason, StopReason::Destroy) && result.is_ok() {
          self.ctx.mark_destroy_completed();
      }
      if let Some(pending) = self.shutdown_reply.take() {
          let _ = pending.reply.send(clone_shutdown_result(&result));
      }
      self.transition_to(LifecycleState::Terminated);
      reply_rx
          .await
          .expect("direct stop reply channel should remain open")
  }
  ```

  Notes:
  - Bypasses `begin_stop` so it does not contend with `register_shutdown_reply`'s
    `debug_assert!`.
  - Pumps grace in-line (the live loop is not running here; tests call this directly).
  - Uses the same panic wrapper and reply-delivery path as `run`. If the reply-delivery block
    is extracted into a `deliver_shutdown_reply(&mut self, &Result<()>)` helper,
    `handle_stop` and `run` both call it.

### Main loop changes

- Remove the `shutdown_step` arm in `ActorTask::run` (task.rs:652‑653). The
  `wait_for_run_handle` arm's `self.shutdown_step.is_none()` guard drops the shutdown check
  (task.rs:667) — the main loop no longer runs concurrently with finalize.
- The live loop body returns `ShutdownTrigger { reason }` instead of calling
  `install_shutdown_step`. `begin_stop(Stop, SleepFinalize | Destroying)` becomes a
  `debug_assert!` + release warn path.
- `on_sleep_grace_complete` (task.rs:1403) no longer calls `enter_shutdown_state_machine`; it
  returns `ShutdownTrigger { reason: Sleep }` up through the live-loop return.
- `handle_run_handle_outcome` (task.rs:1326): when `sleep_requested` / `destroy_requested` is set,
  return `ShutdownTrigger` with the appropriate reason and `shutdown_reply = None`. The
  `LifecycleState::Terminated` branch returns `None` (terminates the live loop cleanly without
  running shutdown).

### Panic handling

- Delete per-phase `AssertUnwindSafe + catch_unwind` inside `boxed_shutdown_step` (task.rs:1757).
- Wrap the single `run_shutdown` call site with `AssertUnwindSafe(self.run_shutdown(reason)).catch_unwind().await`.
  A panic becomes `Err(anyhow!("shutdown panicked during {reason:?}"))`, the reply is still sent,
  and the task terminates cleanly.
- Regression test `shutdown_step_panic_returns_error_instead_of_crashing_task_loop` (tests/modules/task.rs:2823)
  is adapted to assert on the single wrapper instead of per-phase behavior; the observable
  outcome is the same (Err reply, no task crash).

### Test fixtures

- `sleep_finalize_keeps_lifecycle_events_live_between_shutdown_steps` (tests/modules/task.rs:2743)
  documents the *old* concurrent-finalize behavior. Update or delete it. Under the new design,
  finalize does not service `lifecycle_events` — that's by design (the inbox cannot meaningfully
  produce work once all lifecycle-state gates have flipped). Confirm that no production code path
  relies on events being serviced during finalize before deleting.
- Global test hooks `install_shutdown_cleanup_hook` / lifecycle-event/reply hooks already have
  the actor-scoped, serialized-in-tests contract (per CLAUDE.md). No change needed.

## Out Of Scope

- US-103 and US-104 have already landed on this branch (commits `1cecba8a7` and `094fde428`).
  `sleep_grace: Option<SleepGraceState>` is in place at task.rs:529; `shutdown_for_sleep_grace`
  is already gone; grace runs in the main `select!` loop. This spec does not modify sleep grace
  and only reads `sleep_grace` as a live-loop field.
- Changing the `LifecycleCommand` schema or the registry-side `try_send_lifecycle_command` helper.
- Changing engine actor2 invariants. This spec consumes them; it does not modify them.
- Changes to `ActorContext` sleep / activity / drain APIs. The existing `wait_for_shutdown_tasks`,
  `wait_for_on_state_change_idle`, `record_shutdown_wait`, `mark_destroy_completed`, etc. are used
  verbatim.

## Verification

- `cargo build -p rivetkit-core`.
- `cargo test -p rivetkit-core` — `actor::task` module tests must pass. Expect two test updates:
  - `sleep_finalize_keeps_lifecycle_events_live_between_shutdown_steps` (delete or repurpose).
  - `shutdown_step_panic_returns_error_instead_of_crashing_task_loop` (adapt to assert the
    single-wrapper equivalent).
  Every other shutdown lifecycle test must pass unmodified. If any test relies on
  `shutdown_replies.len() > 1` or on `ShutdownPhase` transitions being observable from outside
  the shutdown function, treat that as a real regression and stop.
- `pnpm --filter @rivetkit/rivetkit-napi build:force`.
- `pnpm build -F rivetkit`.
- Driver suite from `rivetkit-typescript/packages/rivetkit`:
  - `pnpm test tests/driver/actor-sleep.test.ts -t "static registry.*encoding \\(bare\\).*Actor Sleep Tests"`
  - `pnpm test tests/driver/actor-lifecycle.test.ts -t "static registry.*encoding \\(bare\\).*Actor Lifecycle Tests"`
  - `pnpm test tests/driver/actor-conn-hibernation.test.ts -t "static registry.*encoding \\(bare\\).*Actor Connection Hibernation Tests"`
  - `pnpm test tests/driver/actor-error-handling.test.ts -t "static registry.*encoding \\(bare\\)"`
- No regressions expected. The `debug_assert!` on `shutdown_reply.is_none()` must never trip under
  the existing engine actor2 paths; if it does, the engine invariant assumed here is wrong and
  the story should be aborted (not patched around by re‑introducing the `Vec`).
- Cross-check the resulting `ActorTask` struct doc (task.rs:441 area) so the
  field-comment block reflects the inline-shutdown design and the engine-supplied one-Stop
  invariant.
