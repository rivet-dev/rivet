# ActorTask dispatch

Routing for actor lifecycle and user-facing work inside `rivetkit-core`. Captures the current `ActorTask` + `DispatchCommand` wiring — expect this to evolve as the ActorTask migration completes.

## Migration status

- `RegistryDispatcher` stores per-actor `ActorTaskHandle`s, but startup still runs through `ActorLifecycle::startup` before `LifecycleCommand::Start`. Later migration stories own moving startup fully inside `ActorTask`.
- `ActorContext::restart_run_handler()` enqueues `LifecycleEvent::RestartRunHandler` once `ActorTask` is configured. Only pre-task startup uses the legacy fallback.

## Dispatch commands

- `DispatchCommand::Action` — spawns a `UserTaskKind::Action` child in `ActorTask.children`. Reply flows from that child task. Action children must remain concurrent; do not reintroduce a per-actor action lock because unblock/finish actions need to run while long-running actions await.
- `DispatchCommand::Http` — spawns a `UserTaskKind::Http` child in `ActorTask.children`. Reply flows from that child task.
- `DispatchCommand::OpenWebSocket` — spawns a `UserTaskKind::WebSocketLifetime` child. Message/close callbacks stay inline under the WebSocket callback guard.

## Run task

- `run` is spawned in a detached panic-catching task during startup.
- Tracked via the `ActorTask` run handle; sleep shutdown waits for it before finalize.

## Side tasks

- Actor-scoped side tasks from `ActorContext` run through `WorkRegistry.shutdown_tasks` so sleep/destroy teardown can drain or abort them. Store explicit `JoinHandle`s only for timers/tasks with their own cancellation slot.

## Engine process supervision

- Engine subprocess supervision lives in `rivetkit-core/src/engine_process.rs`. `registry.rs` calls `EngineProcessManager` only from serve / startup / shutdown plumbing.

## Metrics

- Actor runtime Prometheus metrics flow through the shared `ActorContext` `ActorMetrics`. Use `UserTaskKind` / `StateMutationReason` metric labels instead of string literals at call sites.

## Test hooks

- Process-global `ActorTask` test hooks (`install_shutdown_cleanup_hook`, lifecycle-event / reply hooks) must be actor-scoped and serialized in tests or parallel `cargo test` runs will cross-wire unrelated actors.
