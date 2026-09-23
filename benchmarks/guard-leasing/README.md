# Actor wake benchmark

A Rust actor and HTTP benchmark for the opt-in Guard leasing prototype. `cold-start`
measures **waking an existing, initialized actor** after sleep, not creating a fresh
actor. Creation, sleep, and the one-second settling period are outside the timer.
Each sample checks that the actor generation changed. `warm` measures requests to
an already running actor.

Use an isolated local Engine and namespace. `init` creates the namespace when
needed and updates its `default` runner configuration; each benchmark run creates
and destroys its own actor. Keep the same Engine storage backend and runner build
for comparisons.

```sh
cargo build --release -p benchmark-guard-leasing
export RIVET_ENDPOINT=http://127.0.0.1:6420
export RIVET_TOKEN=dev-token
export RIVET_NAMESPACE=actor-wake-bench
export RIVET_BENCH_READONLY=1
./target/release/benchmark-guard-leasing init
./target/release/benchmark-guard-leasing actor
```

In another terminal with the same environment:

```sh
./target/release/benchmark-guard-leasing cold-start 100 10 > cold.csv 2> cold.log
./target/release/benchmark-guard-leasing warm 100 10 > warm.csv 2> warm.log
```

The actor runs SQLite natively through Depot's custom VFS. CSV samples go to stdout;
startup counters and summary percentiles go to stderr.

Compare these cumulative stages, restarting the Engine and runner between stages:

| Stage | Engine environment | Runner environment |
| --- | --- | --- |
| Workflow | None | `RIVET_BENCH_READONLY=1` |
| Guard leases | `RIVET_ACTOR_LEASE_POC=1` | Same |
| Read-only startup | Same | Add `RIVET_ACTOR_START_READONLY=1` |
| Start preload | Add `RIVET_ACTOR_START_PRELOAD_PAGES=128` | Same |
| Bundled HTTP request | Add `RIVET_ACTOR_START_REQUEST=1` | Same |

Use separate fresh local storage directories for workflow and lease modes; the
prototype does not migrate existing actor workflows. Preload and bundled requests
require Envoy protocol v9. Bundled requests also require Gateway 3 selected for the
request. For a full rollout in a local test, set Engine
`features.guard_gateway_v3` to
`{"mode":"on","percentage":100}`. Other routes retain normal Start delivery.

Preload caches at most the configured first N pages (hard limit 256) at an exact
SQLite branch/head. Missing pages fall back to ordinary Depot reads. Co-locate the
Engine services when measuring the process-local preload cache.

Set `RIVET_BENCH_ASSERT_READONLY=1` in the benchmark process to require zero startup
commits and mutating SQL on initialized wakes. Also set
`RIVET_BENCH_ASSERT_PRELOAD=1` to require a complete preload with zero fallback page
RPCs. The latter is a strict fixture assertion, not a requirement for normal use.
