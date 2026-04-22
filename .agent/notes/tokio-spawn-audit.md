# Tokio Spawn Audit

Date: 2026-04-22
Story: US-079

## Scope

Audited `tokio::spawn`, `Handle::spawn`, and `JoinSet::spawn` usage in:

- `rivetkit-rust/packages/rivetkit-core/src/`
- `rivetkit-rust/packages/rivetkit-sqlite/src/`

Inline `#[cfg(test)]` modules were classified separately from production code. `rivetkit-sqlite/src/` has no production `tokio::spawn` sites; its thread spawns are test-only SQLite VFS worker coverage.

## Actor-Scoped Sites

- `actor/context.rs::sleep` - was a loose runtime spawn for envoy sleep intent. Migrated to the actor sleep `WorkRegistry.shutdown_tasks` `JoinSet`; fallback calls envoy directly when no runtime or teardown already started.
- `actor/context.rs::destroy` - was a loose runtime spawn for envoy destroy intent. Migrated to the same actor sleep `JoinSet`; fallback calls envoy directly when no runtime or teardown already started.
- `actor/context.rs::dispatch_scheduled_action` - was a loose `tokio::spawn` that held an internal keep-awake guard while dispatching an overdue scheduled action. Migrated to the actor sleep `JoinSet` so sleep/destroy teardown can drain or abort it.
- `actor/sleep.rs::track_shutdown_task` - existing actor-owned `JoinSet`; now returns whether the task was accepted so callers can choose an immediate fallback when needed.

## Already Tracked Or Abortable

- `actor/sleep.rs::reset_sleep_timer_state` - compatibility timer stored in `sleep_timer` and aborted by `cancel_sleep_timer`.
- `actor/state.rs::schedule_save` - delayed save stored in `pending_save`, replaced/aborted by later saves, and drained by state shutdown waits.
- `actor/state.rs::persist_now_tracked` - immediate save stored in `tracked_persist` and awaited by shutdown.
- `actor/schedule.rs::set_alarm_tracked` - ack/persist completion is tracked by `schedule_pending_alarm_writes`.
- `actor/schedule.rs::arm_local_alarm` - local alarm timer stored in `schedule_local_alarm_task` and aborted on resync/shutdown.
- `actor/task.rs::spawn_run_handle` - user run handler stored in `ActorTask.run_handle`, awaited or aborted during shutdown.
- `engine_process.rs::spawn_engine_log_task` - process-manager log tasks stored on `EngineProcessManager` and joined during manager shutdown.
- `registry.rs::start_actor` actor task spawn - stored in `ActorTaskHandle.join`; registry shutdown paths lock and join/abort it.
- `registry.rs` inspector overlay task - stored in the inspector websocket close slot and aborted when the websocket closes.

## Process/Callback Scoped Sites Left As Spawns

- `registry.rs` pending stop queued during actor startup - registry-scoped handoff that completes an envoy stop handle after startup; not actor-owned until the actor instance exists.
- `registry.rs` actor websocket action response task - connection callback fanout that lets the websocket receive loop keep reading. It dispatches through the actor task and intentionally handles hibernatable replay/ack ordering. Migrating this needs a smaller follow-up because dropping it after teardown could change client ack semantics.
- `registry.rs` inspector subscription signal task - inspector websocket fanout task that builds one pushed message per signal. It is scoped to the inspector websocket subscription rather than actor teardown.
- `registry.rs::on_actor_stop_with_completion` handoff - envoy callback must return immediately after handing stop completion to the dispatcher; registry owns the follow-up stop flow.

## Test-Only Sites

- `actor/queue.rs`, `actor/connection.rs`, `actor/sleep.rs`, `tests/modules/*`, and `rivetkit-sqlite/src/vfs.rs` spawn helpers are test-only concurrency harnesses. They are intentionally not migrated to actor-owned production `JoinSet`s.
