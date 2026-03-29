# Benchmarks

## Results

### Cold Start Latency

Sequential — VMs created one at a time:

| Batch Size | Samples | Mean     | p50      | p95      | p99      |
| ---------- | ------- | -------- | -------- | -------- | -------- |
| 1          | 5       | 39.3 ms  | 36.3 ms  | 52.7 ms  | —        |
| 10         | 50      | 35.7 ms  | 34.1 ms  | 43.6 ms  | —        |

Concurrent — up to `os.availableParallelism() - 4` VMs created in parallel (16 on this machine):

| Batch Size | Samples | Mean     | p50      | p95       | p99      |
| ---------- | ------- | -------- | -------- | --------- | -------- |
| 1          | 5       | 35.8 ms  | 35.9 ms  | 37.8 ms   | —        |
| 10         | 50      | 97.0 ms  | 96.9 ms  | 117.1 ms  | —        |

p99 is omitted (—) where sample count is below 100, as the percentile is not statistically meaningful at that size.

**Key takeaway:** Sequential cold start is stable at **~35 ms p50** regardless of batch size.
This includes creating the full kernel with all three runtime mounts (WASM, Node.js, Python).
Concurrent cold start scales to ~97 ms at batch=10 due to CPU contention from parallel
kernel initialization. Compared to traditional sandbox providers (e2b p95 TTI of 950 ms),
WASM cold start is **~22x faster**.

### Warm Start Latency

Sequential:

| Batch Size | Samples | Mean     | p50      | p95      | p99      |
| ---------- | ------- | -------- | -------- | -------- | -------- |
| 1          | 5       | 23.5 ms  | 23.3 ms  | 24.6 ms  | —        |
| 10         | 50      | 24.0 ms  | 23.5 ms  | 27.7 ms  | —        |

Concurrent:

| Batch Size | Samples | Mean     | p50      | p95      | p99      |
| ---------- | ------- | -------- | -------- | -------- | -------- |
| 1          | 5       | 23.7 ms  | 23.1 ms  | 25.9 ms  | —        |
| 10         | 50      | 37.8 ms  | 36.3 ms  | 50.4 ms  | —        |

p99 is omitted (—) where sample count is below 100.

**Key takeaway:** Warm start is **~23 ms sequential** — roughly 1.5x faster than cold start.
The difference (~12 ms) is the one-time cost of VM creation (kernel setup, runtime mounting,
filesystem initialization). Per-command overhead is dominated by WASM Worker thread dispatch
and shell execution.

### Memory Overhead

Two workloads are measured: a minimal WASM `sleep` process and the full PI coding agent
via the ACP adapter. Each batch size runs in a **separate process** for clean RSS baselines.

#### Sleep workload (WASM process)

Each VM spawns `sleep 99999` so the WASM Worker thread stays alive during measurement.
Cold = first iteration (no module cache), warm = subsequent iterations.

| Batch Size | Cold RSS/VM | Warm RSS/VM | Cold Heap/VM | Warm Heap/VM |
| ---------- | ----------- | ----------- | ------------ | ------------ |
| 1          | 14.0 MB     | 7.0 MB      | ~0 MB        | 0.21 MB      |
| 10         | 12.4 MB     | 6.1 MB      | 0.16 MB      | 0.16 MB      |
| 100        | 13.2 MB     | 7.2 MB      | 0.16 MB      | 0.15 MB      |

Cold start is **~13 MB per VM**, warm is **~7 MB per VM**. The cold/warm gap (~6 MB) is
modest because WASM Workers are independent — each gets its own compiled module instance
and thread stack, with no cross-instance sharing. JS heap is only ~0.16 MB per VM.

#### PI SDK workload (Node.js V8 isolate)

Each VM dynamically imports the full PI coding agent SDK
(`@mariozechner/pi-coding-agent`) in the Node.js V8 runtime. This loads the complete agent
dependency tree (Anthropic SDK, undici, streaming parsers, etc.) and keeps the process alive.
The CLI entry point itself cannot be used because the VM lacks the `signal-exit` polyfill
that `proper-lockfile` requires, but the SDK import exercises the same module loading and
memory footprint.

| Batch Size | Cold RSS/VM | Warm RSS/VM | Cold Heap/VM | Warm Heap/VM |
| ---------- | ----------- | ----------- | ------------ | ------------ |
| 1          | 71.8 MB     | 21.2 MB     | 9.34 MB      | 1.53 MB      |
| 10         | 18.0 MB     | 8.1 MB      | 1.63 MB      | 2.59 MB      |
| 20         | 12.8 MB     | 2.4 MB      | 1.20 MB      | 1.61 MB      |

The first VM in a fresh process costs **~72 MB** (JIT compilation + loading the full
dependency tree). At batch=10, cold per-VM drops to 18 MB because V8 shares compiled module
code across the 10 isolates within that iteration. Warm iterations (where the host process
has already cached compiled modules) are dramatically cheaper: **~8 MB at batch=10, ~2.4 MB
at batch=20**.

#### Comparison

| | Sleep (WASM) | PI SDK (V8) |
|---|---|---|
| Cold RSS — 1st VM in process | 14 MB | **72 MB** |
| Cold RSS — batch=10 per-VM | 12 MB | 18 MB |
| Warm RSS — batch=10 per-VM | 6 MB | 8 MB |
| Scaling behavior | Linear (independent Workers) | Sub-linear (shared code) |
| Minimum sandbox provider | 256 MB | 256 MB |

**Key takeaway:** The first PI VM in a fresh process is expensive (~72 MB) because every
module must be JIT-compiled. But V8 shares compiled code across isolates, so additional VMs
in the same process are much cheaper. At batch=10, cold per-VM (18 MB) is already close to
sleep (12 MB). On warm iterations, both converge to ~6-8 MB per VM. Both are orders of
magnitude more efficient than traditional sandbox providers (256 MB minimum).

## Methodology

### Cold Start

Time from `AgentOs.create()` through the first `vm.exec("echo hello")` completing. This
captures the full VM boot sequence:

1. In-memory filesystem and kernel creation
2. WASM runtime mounting (loading the coreutils WASM command directory)
3. Node.js runtime mounting (V8 isolate sandboxed runtime)
4. Python runtime mounting (Pyodide WASM runtime)
5. First WASM process spawn (the `sh` shell as a Worker thread)
6. Shell parsing and executing the `echo` command
7. Stdout pipe collection and return to host

All three runtimes (WASM, Node.js, Python) are mounted during cold start, matching the
production `AgentOs.create()` configuration. A trivial command (`echo hello`) is used so the
measurement reflects pure runtime overhead without workload noise. The output is verified to
be `"hello\n"` on every iteration to ensure correctness.

Each configuration runs 5 iterations (x batch size samples each) with 1 warmup iteration
discarded. Tail percentiles at small batch sizes (<=10) have low sample counts and should be
interpreted with caution.

Sandbox provider comparison uses the **p95 TTI** (time-to-interactive) from
[ComputeSDK benchmarks](https://www.computesdk.com/benchmarks/). As of March 2026, **e2b**
is the best-performing sandbox provider at **0.95s** p95 TTI.

### Warm Start

Time for a second `vm.exec("echo hello")` on an already-initialized VM. The kernel, all three
runtimes, and mounted filesystems are already set up. Only the per-command overhead is
measured: process table entry creation, WASM Worker thread dispatch, command execution, and
stdout pipe collection.

The difference between cold and warm start (~12 ms) isolates the cost of VM creation, which
only happens once per VM lifetime.

### Memory Per Instance

RSS (Resident Set Size) delta per live VM, measured via `process.memoryUsage().rss` before
and after spinning up N VMs. Two workloads are tested:

- **Sleep** (`--workload=sleep`): Each VM spawns `sleep 99999` to keep the WASM Worker
  thread alive. Without this, the Worker exits after the command completes and its memory
  is reclaimed before sampling, understating the true per-VM cost.
- **PI SDK** (`--workload=pi-sdk`): Each VM dynamically imports the full PI coding agent SDK
  (`@mariozechner/pi-coding-agent`) in the Node.js V8 runtime. This loads the complete
  agent dependency tree and keeps the process alive.

Each batch size is run in a **separate process** to prevent RSS contamination. When multiple
batch sizes run in the same process, GC reclaims pages from earlier iterations, causing later
baselines to be artificially high and deltas artificially low. Separate processes ensure each
measurement starts from a clean baseline.

GC is forced (two passes, `--expose-gc`) before baseline and after-init measurements to flush
incremental and concurrent collection phases. A 100ms sleep follows each GC to allow
finalization. For the PI workload, a 3-second settle time is used after spawn to allow module
loading to complete.

RSS is a process-wide metric that includes JS-side wrappers, Worker thread stacks, and
OS-mapped pages beyond the kernel itself. The reported per-VM figure is an **upper bound**
on the true per-VM cost.

The sleep and PI workloads demonstrate different scaling behaviors: WASM Workers have
independent memory (each gets its own compiled module instance and thread stack), while
Node.js V8 isolates share compiled module code across instances. This means Node.js-based
agents become more memory-efficient at scale.

Sandbox provider memory comparison uses the **minimum allocatable memory** across popular
providers (e2b, Daytona, Modal, Cloudflare) as of March 2026. The minimum is **256 MB**
(Modal and Cloudflare).

## Test Environment

| Component          | Details                                                                  |
| ------------------ | ------------------------------------------------------------------------ |
| CPU                | 12th Gen Intel i7-12700KF, 12 cores / 20 threads @ 3.7 GHz, 25 MB cache |
| Node.js            | v24.13.0                                                                 |
| RAM                | 2x 32 GB Kingston FURY Beast DDR4 (KF3200C16D4/32GX), 3200 MHz CL16    |
| OS                 | Linux 6.1.0-41-amd64 (x64)                                              |
| Timing             | Host-side `performance.now()` used for all measurements. WASM processes run in Worker threads; timing is unaffected by any in-VM clock restrictions. |

## Reproducing

```bash
# Clone and install
git clone https://github.com/rivet-dev/agent-os
cd agent-os && pnpm install

# Run all benchmarks (saves timestamped results to benchmarks/results/)
cd packages/sandbox
./benchmarks/run-benchmarks.sh

# Or run individually
npx tsx benchmarks/echo.bench.ts                                          # cold + warm start

# Memory with sleep workload (WASM process)
npx tsx --expose-gc benchmarks/memory.bench.ts                             # all default batch sizes
npx tsx --expose-gc benchmarks/memory.bench.ts --batch=500                 # single batch size

# Memory with PI SDK workload (V8 isolate)
npx tsx --expose-gc benchmarks/memory.bench.ts --workload=pi-sdk               # all default batch sizes
npx tsx --expose-gc benchmarks/memory.bench.ts --workload=pi-sdk --batch=20    # single batch size
```

Results will vary by hardware. The numbers above are from the test environment described above.
