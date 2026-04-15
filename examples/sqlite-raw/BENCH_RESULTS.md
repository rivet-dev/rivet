# SQLite Large Insert Results

Captured on **2026-04-15** from `/home/nathan/rivet/examples/sqlite-raw`.

## Command

```bash
pnpm --dir examples/sqlite-raw bench:large-insert
```

Additional runs:

```bash
BENCH_MB=1 pnpm --dir examples/sqlite-raw bench:large-insert
BENCH_MB=5 pnpm --dir examples/sqlite-raw bench:large-insert
BENCH_MB=10 pnpm --dir examples/sqlite-raw bench:large-insert
RUST_LOG=rivetkit_sqlite_native::vfs=debug BENCH_MB=1 pnpm --dir examples/sqlite-raw bench:large-insert
```

## Environment

- Example: `examples/sqlite-raw`
- Endpoint: `http://127.0.0.1:6420`
- Payload shape: one row containing a large `TEXT` payload
- Comparison baseline: native SQLite on local disk via `node:sqlite`

## Results

| Payload | Actor DB Insert | Actor DB Verify | End-to-End Action | Native SQLite Insert | Actor DB vs Native | End-to-End vs Native |
| ------- | --------------- | --------------- | ----------------- | -------------------- | ------------------ | -------------------- |
| 1 MiB   | 832.2ms         | 0.4ms           | 1137.6ms          | 1.8ms                | 461.11x            | 630.34x              |
| 5 MiB   | 4199.6ms        | 3655.5ms        | 8186.3ms          | 25.3ms               | 166.19x            | 323.96x              |
| 10 MiB  | 9438.2ms        | 8973.5ms        | 19244.0ms         | 45.5ms               | 207.34x            | 422.75x              |

## Notes

- Local 10 MiB end-to-end latency was **19.2s**.
- The production number you shared for 10 MiB was **26.2s**.
- Native SQLite is fast enough that the bottleneck is clearly not SQLite itself.
- The actor-side DB path is already extremely slow before counting client/action overhead.

## Debug Trace Clue

From the debug run with `RUST_LOG=rivetkit_sqlite_native::vfs=debug` and `BENCH_MB=1`:

- `317` total KV round-trips
- `30` `get(...)` calls
- `287` `put(...)` calls
- `577` total keys written
- Aggregate traced KV time:
  - `get`: `63.1ms`
  - `put`: `856.0ms`

## Likely Bottleneck

The current SQLite-over-KV path is chunking the database into **4 KiB pages** and issuing a large number of KV writes and reads through the tunnel for a single large insert. The evidence points much more strongly at the SQLite VFS / KV channel / engine path than at raw SQLite execution.
