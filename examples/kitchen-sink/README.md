# Kitchen Sink

Unified kitchen-sink showcasing Rivet Actor features with a single registry, grouped navigation, and interactive demos.

## Getting Started

```sh
git clone https://github.com/rivet-dev/rivet.git
cd rivet/examples/kitchen-sink
npm install
npm run dev
```


## Features

- Unified registry that aggregates actor fixtures and example actors
- Sidebar navigation grouped by core actor feature areas
- Action runner and event listener for quick experimentation
- Raw HTTP and WebSocket demos for handler-based actors
- Workflow and queue pattern coverage in a single kitchen-sink

## Prerequisites

- OpenAI API key (set `OPENAI_API_KEY`) for the AI actor demo

## Raw SQLite Fuzz Harness

Drive the raw SQLite fuzzer actor against a real endpoint:

```sh
export RIVET_ENDPOINT="http://127.0.0.1:6420"
pnpm --filter kitchen-sink exec tsx scripts/db-fuzz.ts --endpoint "$RIVET_ENDPOINT" --start-local-envoy --seed local-smoke --iterations 2 --actor-count 1 --concurrency 1
pnpm --filter kitchen-sink exec tsx scripts/db-fuzz.ts --endpoint "$RIVET_ENDPOINT" --start-local-envoy --seed stress-001 --iterations 10 --actor-count 4 --concurrency 4 --mode hot --sleep-every 2 --ops-per-phase 100
pnpm --filter kitchen-sink exec tsx scripts/db-fuzz.ts --endpoint "$RIVET_ENDPOINT" --start-local-envoy --seed vfs-001 --mode kitchen-sink --iterations 3 --actor-count 1 --concurrency 1 --ops-per-phase 80 --max-payload-bytes 131072 --growth-target-bytes 1048576
pnpm --filter kitchen-sink exec tsx scripts/db-fuzz.ts --endpoint "$RIVET_ENDPOINT" --start-local-envoy --seed growth-10m --mode growth --iterations 1 --actor-count 1 --concurrency 1 --ops-per-phase 20 --max-payload-bytes 98304 --growth-target-bytes 10485760
pnpm --filter kitchen-sink exec tsx scripts/db-fuzz.ts --endpoint "$RIVET_ENDPOINT" --start-local-envoy --seed nasty-script --mode nasty-script --iterations 1 --actor-count 1 --concurrency 1 --ops-per-phase 1 --max-payload-bytes 131072
```

The harness uses raw SQLite only. It checks live rows against an operation log, validates repeated updates, deletes, upserts, transfer transactions, payload checksums, SQLite integrity, indexed query parity, page-boundary payloads, fragmentation/VACUUM churn, schema churn, constraints, savepoints, idempotent replay, relational aggregates, PRAGMA probes, prepared statement churn, large DB growth, boundary keys, shadow checksums, read/write overlap, truncate/recreate stress, and persistence after optional sleep/wake cycles.

## Implementation

The kitchen-sink registry imports fixtures and example actors into one setup. The local runtime entry point starts it with `registry.start()`.

See the registry in [`src/index.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/kitchen-sink/src/index.ts), the runtime entry in [`src/start.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/kitchen-sink/src/start.ts), and the UI in [`frontend/App.tsx`](https://github.com/rivet-dev/rivet/tree/main/examples/kitchen-sink/frontend/App.tsx).

## Resources

Read more about [Rivet Actors](https://rivet.dev/docs/actors),
[actions](https://rivet.dev/docs/actors/actions), and
[connections](https://rivet.dev/docs/actors/connections).

## License

MIT
