# HTTP Routing Unification

## Framework Routes

- `/metrics`: owned by `rivetkit-core::handle_fetch`; never delegated to user `onRequest`.
- `/inspector/*`: owned by `rivetkit-core::handle_fetch` unless the registry is configured to handle inspector HTTP in the runtime.
- `/action/:name`: owned by `rivetkit-core::handle_fetch`; only `POST` is valid.
- `/queue/:name`: owned by `rivetkit-core::handle_fetch`; only `POST` is valid.
- Everything else: delegated to the user `onRequest` callback when configured, otherwise returns `404`.

## Action Contract

- Core parses the path, request encoding, request body, connection params header, and message-size limits.
- Core creates the request-scoped connection and dispatches through `DispatchCommand::Action`.
- TypeScript keeps action schema validation inside the NAPI action callback before invoking the user handler.
- Core serializes the framework HTTP response for JSON, CBOR, or BARE.

## Queue Contract

- Core parses the path, request encoding, request body, connection params header, and incoming message-size limit.
- Core creates the request-scoped connection and dispatches a queue-send framework event through the actor task.
- TypeScript keeps queue schema validation and `canPublish` checks in the NAPI queue-send callback before writing to the native queue.
- Core serializes the framework HTTP response for JSON, CBOR, or BARE.

## Delegation Rule

- User `onRequest` is no longer a fallback router for framework paths.
- Any path matching `/action/*` or `/queue/*` is consumed by core even when the method is invalid or the route body is malformed.
