# Shutdown-as-State-Machine — rivetkit-core ActorTask main loop

Status: LANDED in US-105. US-102 already split sleep into `SleepGrace` and `SleepFinalize`; this spec's state-machine work now covers the post-grace `SleepFinalize` and `Destroying` phases while keeping the main loop live between shutdown steps.

## Problem

Today, `ActorTask::run`'s `select!` parks inside the `lifecycle_inbox` arm's handler for the entire `shutdown_for_sleep` / `shutdown_for_destroy` sequence (`task.rs:323-340`). For the full grace period, the main loop cannot:

- Service other select arms (even if future features need them to).
- Observe shutdown progress externally.
- React to a tick-driven completion signal — termination is implicit ("handler returns, then `should_terminate()` is true").
- Surface panic recovery above the `run()` future boundary.

The goal: make the shutdown sequence a **state machine driven by the same select loop** that already dispatches the actor's other work. Shutdown becomes N discrete steps; each step's pending future lives on `self`; the select loop polls it alongside the other arms and advances state when the step completes.

**No separate task.** Everything stays inside `ActorTask::run`'s one tokio future.

## Design

### New fields on `ActorTask`

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ShutdownPhase {
    None,
    DrainingIdle,           // wait_for_sleep_idle_window
    SendingSleepEvent,      // push ActorEvent::Sleep to adapter
    AwaitingSleepReply,     // await the Reply oneshot
    DrainingTrackedBefore,  // drain_tracked_work (before disconnect)
    DisconnectingConns,     // disconnect_for_shutdown
    DrainingTrackedAfter,   // drain_tracked_work (after disconnect)
    AwaitingActorEntry,     // wait_for_actor_entry_shutdown
    Finalizing,             // finish_shutdown_cleanup (save, alarms, sqlite, teardown)
    Done,                   // ready to transition to LifecycleState::Terminated
}

struct ActorTask {
    // ... existing fields ...

    shutdown_phase: ShutdownPhase,
    shutdown_reason: Option<StopReason>,
    shutdown_deadline: Option<Instant>,
    shutdown_reply: Option<oneshot::Sender<Result<()>>>,

    /// The in-flight future for the current shutdown phase. Pinned on the heap
    /// because self-referential future state is hard to hold on the stack.
    /// Returns the phase to transition to next on success, or an error.
    shutdown_step: Option<Pin<Box<dyn Future<Output = Result<ShutdownPhase>> + Send>>>,
}
```

### Main loop select

```rust
pub async fn run(mut self) -> Result<()> {
    loop {
        self.record_inbox_depths();
        tokio::select! {
            biased;
            Some(cmd) = self.lifecycle_inbox.recv(), if self.shutdown_phase == ShutdownPhase::None => {
                self.handle_lifecycle(cmd);   // synchronous; may install shutdown_step
            }
            outcome = Self::poll_shutdown_step(self.shutdown_step.as_mut()),
                      if self.shutdown_step.is_some() => {
                self.on_shutdown_step_complete(outcome);
            }
            Some(event) = self.lifecycle_events.recv() => {
                self.handle_event(event).await;
            }
            Some(cmd) = self.dispatch_inbox.recv(), if self.accepting_dispatch() => {
                self.handle_dispatch(cmd).await;
            }
            outcome = Self::wait_for_actor_entry(self.actor_entry.as_mut()),
                      if self.actor_entry.is_some() && self.shutdown_phase != ShutdownPhase::AwaitingActorEntry => {
                self.handle_actor_entry_outcome(outcome);
            }
            _ = Self::state_save_tick(self.state_save_deadline), if self.state_save_timer_active() && self.shutdown_phase == ShutdownPhase::None => {
                self.on_state_save_tick().await;
            }
            _ = Self::inspector_serialize_state_tick(self.inspector_serialize_state_deadline),
                if self.inspector_serialize_timer_active() && self.shutdown_phase == ShutdownPhase::None => {
                self.on_inspector_serialize_state_tick().await;
            }
            _ = Self::sleep_tick(self.sleep_deadline),
                if self.sleep_timer_active() && self.shutdown_phase == ShutdownPhase::None => {
                self.on_sleep_tick().await;
            }
            else => break,
        }

        if self.should_terminate() { break; }
    }
    self.record_inbox_depths();
    Ok(())
}
```

### Step helper

```rust
impl ActorTask {
    /// Await the pending shutdown step. Returns std::future::pending() if none.
    async fn poll_shutdown_step(
        step: Option<&mut Pin<Box<dyn Future<Output = Result<ShutdownPhase>> + Send>>>,
    ) -> Result<ShutdownPhase> {
        match step {
            Some(f) => f.await,
            None => std::future::pending().await,
        }
    }

    fn on_shutdown_step_complete(&mut self, outcome: Result<ShutdownPhase>) {
        self.shutdown_step = None;
        match outcome {
            Ok(next) => self.install_shutdown_step(next),
            Err(e) => {
                self.shutdown_phase = ShutdownPhase::Done;
                let _ = self.shutdown_reply.take().map(|r| r.send(Err(e)));
            }
        }
    }

    /// Transitions `shutdown_phase` to `next` and boxes a fresh future for that
    /// phase, storing it in `shutdown_step`. `ShutdownPhase::Done` clears the
    /// step and fires the reply.
    fn install_shutdown_step(&mut self, next: ShutdownPhase) {
        self.shutdown_phase = next;
        let deadline = self.shutdown_deadline.expect("shutdown deadline set on entry");
        let ctx = self.ctx.clone();

        self.shutdown_step = match next {
            ShutdownPhase::DrainingIdle => {
                Some(Box::pin(async move {
                    ctx.wait_for_sleep_idle_window(deadline).await;
                    Ok(ShutdownPhase::SendingSleepEvent)
                }))
            }
            ShutdownPhase::SendingSleepEvent => {
                let (tx, rx) = oneshot::channel();
                self.pending_shutdown_reply = Some(rx);    // new field
                let actor_event_tx = self.actor_event_tx.clone();
                let event_kind = self.shutdown_reason_event();  // Sleep or Destroy
                Some(Box::pin(async move {
                    actor_event_tx.try_reserve()?.send(event_kind(tx.into()));
                    Ok(ShutdownPhase::AwaitingSleepReply)
                }))
            }
            ShutdownPhase::AwaitingSleepReply => {
                let reply_rx = self.pending_shutdown_reply.take().unwrap();
                Some(Box::pin(async move {
                    match tokio::time::timeout_at(deadline, reply_rx).await {
                        Ok(Ok(Ok(()))) => Ok(ShutdownPhase::DrainingTrackedBefore),
                        Ok(Ok(Err(e))) => Err(e),
                        Ok(Err(_))     => Err(anyhow!("adapter reply channel closed")),
                        Err(_)         => Err(anyhow!("adapter shutdown reply timed out")),
                    }
                }))
            }
            ShutdownPhase::DrainingTrackedBefore
            | ShutdownPhase::DrainingTrackedAfter => {
                let phase = next;
                let reason = self.shutdown_reason.unwrap();
                let next_phase = match next {
                    ShutdownPhase::DrainingTrackedBefore => ShutdownPhase::DisconnectingConns,
                    ShutdownPhase::DrainingTrackedAfter => ShutdownPhase::AwaitingActorEntry,
                    _ => unreachable!(),
                };
                Some(Box::pin(async move {
                    ctx.drain_tracked_work(reason, deadline).await;
                    Ok(next_phase)
                }))
            }
            ShutdownPhase::DisconnectingConns => {
                let preserve_hibernatable = matches!(
                    self.shutdown_reason, Some(StopReason::Sleep)
                );
                Some(Box::pin(async move {
                    ctx.disconnect_for_shutdown(preserve_hibernatable).await?;
                    Ok(ShutdownPhase::DrainingTrackedAfter)
                }))
            }
            ShutdownPhase::AwaitingActorEntry => {
                let entry = self.actor_entry.take();
                Some(Box::pin(async move {
                    if let Some(handle) = entry {
                        let _ = tokio::time::timeout_at(deadline, handle).await;
                    }
                    Ok(ShutdownPhase::Finalizing)
                }))
            }
            ShutdownPhase::Finalizing => {
                let reason = self.shutdown_reason.unwrap();
                Some(Box::pin(async move {
                    ctx.finish_shutdown_cleanup(reason).await?;
                    Ok(ShutdownPhase::Done)
                }))
            }
            ShutdownPhase::Done => {
                self.transition_to(LifecycleState::Terminated);
                if matches!(self.shutdown_reason, Some(StopReason::Destroy)) {
                    self.ctx.mark_destroy_completed();
                }
                let _ = self.shutdown_reply.take().map(|r| r.send(Ok(())));
                None
            }
            ShutdownPhase::None => None,
        };
    }
}
```

### `handle_lifecycle` becomes synchronous for Stop

```rust
fn handle_lifecycle(&mut self, command: LifecycleCommand) {
    match command {
        LifecycleCommand::Stop { reason, reply } => {
            self.drain_accepted_dispatch_sync();
            self.transition_to(match reason {
                StopReason::Sleep => LifecycleState::Sleeping,
                StopReason::Destroy => LifecycleState::Destroying,
            });
            let grace = match reason {
                StopReason::Sleep => self.factory.config().effective_sleep_grace_period(),
                StopReason::Destroy => self.factory.config().effective_on_destroy_timeout(),
            };
            self.shutdown_reason = Some(reason);
            self.shutdown_deadline = Some(Instant::now() + grace);
            self.shutdown_reply = Some(reply);

            // Cancel deadlines that are gated off during shutdown.
            self.state_save_deadline = None;
            self.inspector_serialize_state_deadline = None;
            self.sleep_deadline = None;
            self.ctx.schedule().suspend_alarm_dispatch();
            self.ctx.cancel_local_alarm_timeouts();

            self.install_shutdown_step(ShutdownPhase::DrainingIdle);
            // Main loop's next iteration: poll_shutdown_step fires the draining-idle future.
        }
        LifecycleCommand::FireAlarm { reply } => {
            // ... existing; enqueue or synchronous handler
        }
    }
}
```

## Why this works (one task, no spawn)

- `shutdown_step: Option<Pin<Box<dyn Future>>>` holds the **current step's future** directly on `self`.
- The select arm `poll_shutdown_step` awaits that boxed future alongside every other arm.
- Between steps, control returns to the select loop. Other arms (lifecycle_events, dispatch, etc.) get a chance to fire. Even if nothing useful fires today, the loop is LIVE — it isn't parked inside a single long handler.
- Each step is a small async block that captures owned clones (`ctx.clone()`, deadline, reason) so it doesn't borrow `self`. The `&mut self` is free to service other arms between steps.
- `on_shutdown_step_complete` advances `shutdown_phase` and calls `install_shutdown_step` to box the next step's future. Mutations to `self.actor_entry`, `self.shutdown_reason`, `self.shutdown_reply` happen here — between polls, never during one.
- Termination is truly tick-driven: `ShutdownPhase::Done` clears the step, sends the reply, and leaves `shutdown_phase == Done` + `LifecycleState::Terminated`. Next iteration, `should_terminate()` breaks the loop.

## Key differences from today

| Aspect | Today | Proposed |
|--------|-------|----------|
| Shutdown body | One async fn with ~10 awaits inline | ~10 small boxed async blocks, one per phase |
| Main loop during shutdown | Parked in the lifecycle_inbox arm's handler | Live; polling `poll_shutdown_step` arm alongside others |
| Inter-step state | Stack locals of `shutdown_for_sleep` | `ShutdownPhase` enum + a few fields on `ActorTask` |
| Error handling | `?` propagates up the nested `async fn` | Each step returns `Result<ShutdownPhase>`; errors route through `on_shutdown_step_complete` |
| Completion signal | Handler returns, loop checks `should_terminate()` | `Done` phase fires `shutdown_reply` + sets Terminated; loop checks `should_terminate()` |

## Same-task, same state machine

- No `tokio::spawn`.
- Shutdown phases live on the same `Self` as every other lifecycle state.
- Every step mutates the same `ShutdownPhase` enum.
- Panic safety unchanged (no additional task boundary).

## Acceptance criteria

1. `rivetkit-core/src/actor/task.rs` introduces `ShutdownPhase` enum + fields `shutdown_phase`, `shutdown_reason`, `shutdown_deadline`, `shutdown_reply`, `shutdown_step: Option<Pin<Box<dyn Future<Output = Result<ShutdownPhase>> + Send>>>`.
2. `handle_lifecycle::Stop` is synchronous: drains accepted dispatch, transitions to Sleeping/Destroying, cancels deadline arms, stores reason/deadline/reply, installs first shutdown step, returns.
3. `install_shutdown_step(phase)` sets `shutdown_phase` and boxes a fresh future for that phase. `Done` phase clears the step, transitions to Terminated, fires `shutdown_reply`, and calls `mark_destroy_completed` for Destroy.
4. Main loop `select!` gains `poll_shutdown_step` arm gated by `shutdown_step.is_some()`. Other deadline arms (`state_save`, `inspector`, `sleep`) are additionally gated by `shutdown_phase == ShutdownPhase::None` so they don't fire during shutdown.
5. All step bodies are small async blocks that capture owned values (ctx.clone, deadline, reason). No step body holds `&mut self`.
6. Shutdown ordering preserved exactly: DrainingIdle → SendingSleepEvent → AwaitingSleepReply → DrainingTrackedBefore → DisconnectingConns → DrainingTrackedAfter → AwaitingActorEntry → Finalizing → Done.
7. Regression test: full Sleep shutdown cycle completes and the main loop services at least one `lifecycle_events.recv()` between steps. Push a `LifecycleEvent::StateMutated` after installing the first shutdown step; assert it was processed before `Done`.
8. Regression test: shutdown step future panics — panic propagates out of `poll_shutdown_step.await`. Wrap step polling in `AssertUnwindSafe(...).catch_unwind()` so the main loop converts the panic to `Err(anyhow!("shutdown phase X panicked"))` and exits cleanly via the reply.
9. Regression test: Destroy shutdown still calls `mark_destroy_completed()` before the reply sends.
10. Regression test: second `LifecycleCommand::Stop` during in-flight shutdown is gated off by `if self.shutdown_phase == ShutdownPhase::None` on the `lifecycle_inbox` arm. Sender sees the command queue in the mpsc until shutdown finalizes (or receives a rejection — match existing contract).
11. Grep: `rivetkit-core/src/actor/task.rs` contains no standalone `shutdown_for_sleep` / `shutdown_for_destroy` async fns on `&mut self`. The step bodies are inline boxed futures inside `install_shutdown_step`.
12. CLAUDE.md: add a bullet under rivetkit-core sleep shutdown: "Shutdown is a state machine polled by the main `ActorTask::run` select loop via `shutdown_step: Pin<Box<dyn Future>>`. Each phase produces the next phase on success. Do not reintroduce a single inline async fn that blocks the select arm for the full grace period."
13. `cargo check -p rivetkit-core`, `cargo test -p rivetkit-core`, TS driver-test-suite baseline all pass.

## Risks / tradeoffs

- **Boxed dyn futures** add one allocation per phase (~10 per shutdown). Negligible.
- **No `&mut self` inside step bodies** means every step captures owned clones. `ctx: ActorContext` is already `Arc`-backed so this is cheap. Other fields used by shutdown (`actor_entry`, `pending_shutdown_reply`) are owned on take.
- **More code** than today: ~150 lines of state-machine boilerplate vs ~80 lines of inline async fn. This is the cost of "live main loop." Worth it only if you actually want future evolutions (escalation, progress, cancellation) or strongly dislike the "main loop parked" property.
- **Panic containment** weaker than a separate task: a panic inside a step future propagates out of the select arm. Mitigated by wrapping `poll_shutdown_step` in `catch_unwind` — preserves today's effective behavior of "panic kills the actor task" but lets us observe and reply cleanly.

## What this spec does NOT introduce

- No `tokio::spawn` for shutdown. Same-task, same future.
- No change to `shutdown_deadline` propagation or grace-period semantics.
- No change to which select arms are gated during shutdown. The goal is to make the main loop LIVE between shutdown steps; not to change WHAT it services.
- No change to `ShutdownController` / `SleepController` internals (`wait_for_sleep_idle_window`, `drain_tracked_work`, etc. stay the same — they are now called FROM the inline boxed step futures, via `ctx.` helpers, instead of from a monolithic `async fn shutdown_for_sleep`).
- No change to existing `request_shutdown_completion`, `disconnect_for_shutdown`, `finish_shutdown_cleanup` internals. These are refactored into methods on `ActorContext` (or a shutdown helper struct) so they can be called from step bodies without `&mut self` on `ActorTask`.
