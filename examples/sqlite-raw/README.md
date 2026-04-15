# SQLite Raw Example

This example demonstrates using the raw SQLite driver with RivetKit actors.

## Getting Started

```sh
git clone https://github.com/rivet-dev/rivet.git
cd rivet/examples/sqlite-raw
npm install
npm run dev
```


## Features

- **Raw SQLite API**: Direct SQL access using `@rivetkit/db/raw`
- **Migrations on Wake**: Uses `onMigrate` to create tables on actor wake
- **Todo List**: Simple CRUD operations with raw SQL queries

## Running the Example

```bash
pnpm install
pnpm dev
```

## Large Insert Benchmark

To benchmark a large payload insert against a local RivetKit actor and compare it
to native SQLite on disk:

```bash
pnpm bench:large-insert
```

To rebuild the engine and native addon, optionally start a fresh local engine,
run the benchmark, and append the structured result to the shared phase log:

```bash
pnpm --dir examples/sqlite-raw run bench:record -- --phase phase-0 --fresh-engine
```

To benchmark the pegboard-backed remote path instead of the inline local path,
spawn the example as a separate runner process and record the result in the same
append-only log:

```bash
pnpm --dir examples/sqlite-raw run bench:record -- --remote-runner --fresh-engine
```

The remote recorder defaults to `BENCH_MB=0.05` so it stays under the current
15 second gateway timeout while still exercising the pegboard-backed SQLite
path. Override `BENCH_MB` or `BENCH_REMOTE_MB` if you need a different envelope.

To re-evaluate the SQLite fast-path batch ceiling against larger page envelopes
and refresh the rendered ceiling table:

```bash
pnpm --dir examples/sqlite-raw run bench:record -- --evaluate-batch-ceiling --chosen-limit-pages 3328 --batch-pages 128,512,1024,2048,3328 --fresh-engine
```

Environment variables:

- `BENCH_MB`: Total payload size in MiB. Defaults to `10`.
- `BENCH_ROWS`: Number of rows to split the payload across. Defaults to `1`.
- `RIVET_ENDPOINT`: Engine endpoint. Defaults to `http://127.0.0.1:6420`.

The benchmark prints:

- Actor-side SQLite insert time
- End-to-end action latency
- Native SQLite baseline latency
- Relative slowdown versus native SQLite

Structured phase results live in:

- `examples/sqlite-raw/bench-results.json` for append-only run metadata
- `examples/sqlite-raw/BENCH_RESULTS.md` for the rendered side-by-side summary

Use the inline `--phase` workflow when iterating on actor-side VFS behavior and
comparing against the existing Phase 0 through Final history. Use
`--remote-runner` when you need pegboard-backed validation and non-zero server
telemetry from the fast-path storage path. The remote runner uses
`src/runner.ts`, which starts only the envoy connection instead of the full
serverful runtime entrypoint.

## Usage

The example creates a `todoList` actor with the following actions:

- `addTodo(title: string)` - Add a new todo
- `getTodos()` - Get all todos
- `toggleTodo(id: number)` - Toggle todo completion status
- `deleteTodo(id: number)` - Delete a todo

## Code Structure

- `src/registry.ts` - Actor definition, migrations, and shared registry
- `src/index.ts` - Example entrypoint that starts the registry
- `src/runner.ts` - Runner-only entrypoint for pegboard-backed remote benchmarks
- `scripts/client.ts` - Simple todo client
- `scripts/bench-large-insert.ts` - Large-payload benchmark runner
- `scripts/run-benchmark.ts` - Rebuilds dependencies, records per-phase runs, and renders `BENCH_RESULTS.md`
- `bench-results.json` - Append-only benchmark run log

## Database

The database uses the KV-backed SQLite VFS, which stores data in a key-value store. The schema is created using raw SQL in the `onMigrate` hook:

```typescript
db: db({
  onMigrate: async (db) => {
    await db.execute(`
      CREATE TABLE IF NOT EXISTS todos (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        title TEXT NOT NULL,
        completed INTEGER DEFAULT 0,
        created_at INTEGER NOT NULL
      )
    `);
  },
})
```

## License

MIT
