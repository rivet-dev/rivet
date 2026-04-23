# rivetkit-core

## Module layout

- Actor subsystem implementations belong under `src/actor/`; keep root module aliases only for compatibility with existing public callers.

## Sleep state invariants

- Any mutation that changes a `can_sleep` input must call `ActorContext::reset_sleep_timer()` so the `ActorTask` sleep deadline is re-evaluated. Inputs are: `ready`/`started`, `prevent_sleep`, `no_sleep`, `active_http_request_count`, `sleep_keep_awake_count`, `sleep_internal_keep_awake_count`, `pending_disconnect_count`, `conns()`, and `websocket_callback_count`. Missing this call leaves the sleep timer armed against stale state and triggers the `"sleep idle deadline elapsed but actor stayed awake"` warning on the next tick.
- Counter `register_zero_notify(&idle_notify)` hooks only drive shutdown drain waits. They are not a substitute for the activity-dirty notification, so any new sleep-affecting counter must also notify on transitions that change `can_sleep`.
- When forwarding an existing `anyhow::Error` across lifecycle/action replies, preserve structured `RivetError` data with `RivetError::extract` instead of stringifying it.

## Queue invariants

- Register queue completion waiters before publishing messages to KV so fast consumers cannot complete a message before the waiter exists.

## Hibernatable WebSockets

- Raw `onWebSocket` hibernatable connections must create `HibernatableConnectionMetadata` and persist plus ack every inbound message through core before gateway replay state is correct.
- Actor-connect WebSocket setup errors must send a protocol `Error` frame before closing; JSON/CBOR connection-level errors include `actionId: null`.
- Flush the actor-connect `WebSocketSender` after queuing a setup `Error` frame and before closing so the envoy writer handles the error before the close terminates the connection.
