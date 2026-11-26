# Java SDK for RivetKit: brief (v2)

* **Status:** v2 built. Every test passes; nothing is published yet.
* **Reference:** [`DESIGN.md`](DESIGN.md) is the current design: API, turns, the boundary, and
  how to keep it up to date. This brief records the decisions and results.

## What it is

A Java 25 SDK for Rivet Actors that covers every non-deprecated actor and client feature. It
is a thin Java layer over the Rust core:
- **`rivetkit-host-adapter` (new):** the actor host logic, independent of any language binding.
  napi, wasm and the Rust SDK can move onto it later; they are unchanged.
- **`rivetkit-ffi`:** a C ABI of 7 functions over the adapter.
- **Java side:** calls that C ABI through FFM.

## Decisions

| Decision | Choice | Why |
|---|---|---|
| Runtime | Java 25, FFM, virtual threads | FFM is final, virtual threads no longer pin on `synchronized`, `ScopedValue` is final. |
| Data model | No actor state; SQLite plus fields | State, connection state, KV, queues and a plain `run` are deprecated in 3.0. This matches the Rust SDK. |
| API | Interface (the contract) + class (the actor) + typed proxy (the client) | No `Integer.class` tokens. javac catches wrong events, params, hooks, schedule and async arguments, and the wrong implementation (15 cases tested). This is the approach Restate's Java SDK moved to in 2.5; Dapr and Temporal use it too. |
| Threading | Turns: one thread at a time per actor; SQL keeps the turn, outbound waits release it | No locks in actor code and no A→B→A deadlock. Strictly safer than JavaScript's `await` interleaving. This is the Cloudflare Durable Objects input-gate rule. |
| Boundary | `u64` handles, a `u32` kind or op, a CBOR header, an untouched body | Payloads pass through and are never re-encoded. Bodies over 64 KiB and every stream chunk are leased, so Java reads them in place. One `unsafe` block. |
| Serialization | CBOR (already Rivet's format for user values) | Each host language needs one codec, not two. BARE, protobuf and FlatBuffers would add a toolchain per language for about 20 small headers. |
| Streams | Credit by lease release (256 KiB window) | The same model as the engine's envoy window updates. A slow reader pauses the sender. |
| Sync across languages | Hand-written headers, golden fixtures, exhaustive matches, cbindgen enums read by Java as constants, an ABI version check at load | Drift fails a build or a test. Code generation isn't worth it at this size. |

The research behind these decisions surveyed:
- Confluent kafka-clients, Temporal sdk-core, the ScyllaDB Rust drivers, libsignal,
  delta-kernel-rs, Arrow, and UniFFI and its users (cross-language data passing);
- Restate, Temporal, Dapr and Akka (JVM API design);
- an Opus review of the API, threading and footguns.

## Results

Benchmark: the same counter actor on each SDK, driven by one Rust client against one local
engine, mean of 2 rounds (`mise run bench:compare`).

| SDK | sequential calls/s | p50 | 20 actors × 100 parallel calls/s | RSS |
|---|---|---|---|---|
| Rust | 404 | 2426 µs | 1026 | 17 MB |
| **Java** | **392** (−3%) | 2514 µs | **993** (−3%) | 213 MB |
| TypeScript | 385 | 2543 µs | 918 | 152 MB |

Java is within 5% of Rust and ahead of TypeScript. Its memory is the JVM's default heap
sizing; set `-Xmx` to bring it down.

Tests (`mise run test`, about 15s warm on a 32-core laptop):
- **Rust:** 27 tests (wire behaviour, ordering, ownership, fixture drift, the ABI), plus Miri on the `unsafe` block.
- **Java unit:** codec, turns, API validation, fixtures, and 16 compile-safety cases.
- **End to end:** 47 tests against a real engine on a free port, run concurrently on one shared
  registry, covering actions, errors, async, events, connections, vars and SQL across sleep,
  transactions, HTTP streaming, WebSockets, turn atomicity, A→B→A calls, schedules and cron.

## Next

1. Add `mise run miri` to CI (it needs a nightly toolchain, so it is not in the job yet).
2. Tests still to add: a TypeScript client calling a Java actor, cron firing, and abort on
   sleep.
3. Publishing: native jars for each platform, Maven Central, Windows.
4. Move napi and wasm onto `rivetkit-host-adapter`.
