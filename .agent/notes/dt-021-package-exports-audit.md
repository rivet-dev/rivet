# DT-021 package exports audit

- `./driver-helpers`: keep removed. The old entrypoint re-exported actor-instance, runtime-router, and gateway-resolution internals that are not a stable public surface anymore.
- `./driver-helpers/websocket`: keep removed. It was an internal lazy `WebSocket` loader wrapper, and the supported path is the public client/connection API rather than importing transport helpers directly.
- `./test`: restore. Examples and docs on this branch still import `rivetkit/test`, and the helper still makes sense as a thin native-envoy test bootstrap.
- `./inspector`: restore. The package still ships live inspector runtime code (`ActorInspector`) plus workflow-history transport helpers.
- `./topologies/*`: keep removed. The source modules are gone and `tests/package-surface.test.ts` already treats those subpaths as intentionally deleted.
- `./dynamic`: keep removed permanently. This branch no longer ships a supported dynamic actor package entrypoint.
- `./sandbox/*`: keep removed permanently. This branch no longer ships sandbox helpers from `rivetkit`.
