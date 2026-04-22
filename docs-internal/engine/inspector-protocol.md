# Inspector protocol

Wire-level and integration rules for the RivetKit actor inspector (WebSocket + HTTP).

## Two transports, one source of truth

- The HTTP inspector endpoints at `rivetkit-typescript/packages/rivetkit/src/actor/router.ts` mirror the WebSocket inspector at `rivetkit-typescript/packages/rivetkit/src/inspector/`. The HTTP API exists for agent-based debugging.
- When updating the WebSocket inspector, also update the HTTP endpoints.
- When adding or modifying inspector endpoints, also update:
  - Relevant tests in `rivetkit-typescript/packages/rivetkit/tests/` to cover all inspector HTTP endpoints.
  - Docs in `website/src/metadata/skill-base-rivetkit.md` and `website/src/content/docs/actors/debugging.mdx`.

## Version negotiation

- Wire-version negotiation belongs in `rivetkit-core` via `ActorContext.decodeInspectorRequest(...)` / `encodeInspectorResponse(...)`. Do not reintroduce TS-side `inspector-versioned.ts` converters.
- Downgrades for unsupported features become explicit `Error` messages with `inspector.*_dropped` codes. Do not silently strip payloads.

## WebSocket transport

- Outbound frames stay at wire format v4.
- Inbound request frames accept v1 through v4.
- Live updates fan out through `InspectorSignal` subscriptions.
- Snapshots read live queue state instead of trusting pre-attach counters.

## Queue-size reads

- Native inspector queue-size reads come from `ctx.inspectorSnapshot().queueSize` in `rivetkit-core`. Do not use TS-side caches or hardcoded fallback values.

## Workflow inspector support

- Workflow inspector support is inferred from mailbox replies — `actor.dropped_reply` means unsupported. Do not resurrect `Inspector` callback flags or unconditional workflow-enabled booleans.
