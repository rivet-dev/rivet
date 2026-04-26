# Envoy Surface Remaining Coverage

Follow-up work from the Envoy test expansion.

## P0

- Fix and test HTTP tunnel callback errors completing Guard requests instead of hanging.
- Re-enable the remaining ignored `engine/packages/engine/tests/envoy/` tests one by one, adapting Runner-era assumptions to Envoy semantics.
- Run the full engine test suite after the Envoy-specific sweep is stable.

## P1

- Add targeted SQLite Envoy startup/takeover coverage:
  - V1 migration lock
  - native V2 metadata
  - concurrent actor startup
  - failed takeover recovery
- Add multiple-Envoy coverage:
  - actor distribution
  - one Envoy loss reallocates only its actors
  - unrelated Envoys stay healthy
- Add explicit Envoy eviction coverage for intended actor/generation removal.
- Add WebSocket tunnel edge coverage:
  - client disconnect
  - engine disconnect
  - reconnect after actor sleep

## P2

- Deepen get-or-create idempotency/race coverage for Envoy-backed actors.
- Fill payload/validation gaps not already covered by copied API tests.
- Add backpressure/ordering assertions around command indexes and duplicate starts.
