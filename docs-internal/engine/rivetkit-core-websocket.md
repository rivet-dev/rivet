# RivetKit Core WebSocket

## Async close handlers

- Rivet actor WebSockets intentionally support async close handlers even though browser WebSocket close listeners are fire-and-forget.
- TypeScript actor code may return a `Promise` from `ws.addEventListener("close", async handler)` or `ws.onclose = async handler`.
- While a close handler promise is in flight, sleep readiness must report active WebSocket callback work and the actor must not finish sleeping.
- Core wraps close-event delivery in `WebSocketCallbackRegion`; the TypeScript native adapter opens one additional region per promise-returning user handler and closes that exact region when the promise settles.
- This is separate from `onDisconnect` gating. Close handlers are WebSocket event work; `onDisconnect` is connection lifecycle work.

## Testing

- Core coverage lives in `rivetkit-core` websocket and sleep tests.
- Driver coverage lives in `rivetkit-typescript/packages/rivetkit/tests/driver/actor-sleep.test.ts` and `actor-sleep-db.test.ts`.
