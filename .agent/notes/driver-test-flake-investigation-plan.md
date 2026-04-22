# Driver test flakiness / red-test investigation plan

**Status:** plan handed off — not yet executed.

Target: `rivetkit-typescript/packages/rivetkit` driver test suite, static
registry, bare encoding. Prior investigation landed `US-102` (error
sanitization) and `US-103` (sleep-grace abort + run-handle wait). Several
flakes and deterministic failures remain; root cause not yet diagnosed.

Running context captured in:
- `.agent/notes/driver-test-progress.md` — running log of per-file state
- `.agent/notes/sleep-grace-abort-run-wait.md` — US-103 background

---

## 0. Pre-flight: persistent log capture

**You must do this before any investigation step. Every test run must tee
stdout+stderr to a file with a predictable path so logs can be queried
later.**

### 0.1 Re-add runtime stderr mirror in the driver harness

File: `rivetkit-typescript/packages/rivetkit/tests/driver/shared-harness.ts`

Find the per-test-runtime spawn (around line 540-580, the
`startNativeDriverRuntime` function, after `runtime = spawn(...)`). It
currently has:

```ts
runtime.stdout?.on("data", (chunk) => {
    logs.stdout += chunk.toString();
});
runtime.stderr?.on("data", (chunk) => {
    logs.stderr += chunk.toString();
});
```

Replace with:

```ts
runtime.stdout?.on("data", (chunk) => {
    const text = chunk.toString();
    logs.stdout += text;
    if (process.env.DRIVER_RUNTIME_LOGS === "1") process.stderr.write(`[RT.OUT] ${text}`);
});
runtime.stderr?.on("data", (chunk) => {
    const text = chunk.toString();
    logs.stderr += text;
    if (process.env.DRIVER_RUNTIME_LOGS === "1") process.stderr.write(`[RT.ERR] ${text}`);
});
```

### 0.2 Add shared-engine stderr mirror in the same file

Find `spawnSharedEngine()` (around line 390). It also has a
stdout/stderr capture pattern. Add the same `[ENG.OUT]` / `[ENG.ERR]`
gated mirror behind a separate env var `DRIVER_ENGINE_LOGS=1` so we
can toggle engine and runtime logs independently (engine log volume
is large).

### 0.3 Standardize the log-capture wrapper

For every test invocation, use this pattern and always save to
`/tmp/driver-logs/<test-slug>-<runN>.log`:

```bash
mkdir -p /tmp/driver-logs
cd /home/nathan/r5/rivetkit-typescript/packages/rivetkit
DRIVER_RUNTIME_LOGS=1 DRIVER_ENGINE_LOGS=1 \
  RUST_LOG=rivetkit_core=debug,rivetkit_napi=debug,rivet_envoy_client=debug,rivet_guard=debug \
  pnpm test tests/driver/<FILE> -t "<FILTER>" \
  > /tmp/driver-logs/<SLUG>-run<N>.log 2>&1
echo "EXIT: $?"
```

Do not delete `/tmp/driver-logs/` during the investigation. Failed-test
log size is the raw material for every step below.

### 0.4 Query pattern

Everything after this point uses:
```bash
grep -E "RT\.(OUT|ERR)|ENG\.(OUT|ERR)" /tmp/driver-logs/<slug>-run<N>.log | grep -iE "<pattern>"
```
Keep greps narrow — a 60s test run can produce 100k+ log lines.

### 0.5 Hygiene

- Do NOT commit the `shared-harness.ts` mirror changes. Revert when
  investigation completes. The mirror is diagnostic-only.
- Before each investigation step, confirm the local engine is running:
  `curl -sf http://127.0.0.1:6420/health`. Restart with
  `./scripts/run/engine-rocksdb.sh >/tmp/rivet-engine.log 2>&1 &` if needed.
- `cd /home/nathan/r5/rivetkit-typescript/packages/rivetkit` before every
  `pnpm test` — the Bash tool does not preserve cwd between calls.

---

## 1. Investigation targets

Each section is self-contained. Run in listed order — cheaper steps feed
later ones.

Each section produces:
1. A short writeup at `.agent/notes/flake-<slug>.md` with evidence
   (log excerpts with `file:line` source pointers, repro command,
   proposed fix direction).
2. If the investigation reveals a real bug, a PRD story in
   `scripts/ralph/prd.json` following the `US-103` template: id
   `US-104` onward, priority relative to the urgency of the bug
   (see guidance in each step). Use the python script pattern from
   previous sessions:
   ```python
   import json
   with open('scripts/ralph/prd.json') as f: prd = json.load(f)
   prd['userStories'].insert(<pos>, { ... })
   with open('scripts/ralph/prd.json','w') as f: json.dump(prd, f, indent=2)
   ```

---

### Step 1. Reconfirm state after US-102 + US-103

**Why first:** two tests were previously red; both may now be green after
those stories landed. Confirming first may shrink the investigation set.

**Targets:**
- `actor-error-handling::should convert internal errors to safe format`
  (was failing pre-US-102; US-102 should have fixed).
- `actor-workflow::starts child workflows created inside workflow steps`
  (was failing pre-US-103 with a double-spawn; may or may not be a side
  effect of the sleep-grace fix).

**Commands:**
```bash
pnpm test tests/driver/actor-error-handling.test.ts \
  -t "static registry.*encoding \(bare\).*Actor Error Handling Tests" \
  > /tmp/driver-logs/error-handling-recheck.log 2>&1

pnpm test tests/driver/actor-workflow.test.ts \
  -t "static registry.*encoding \(bare\).*starts child workflows" \
  > /tmp/driver-logs/workflow-child-recheck.log 2>&1
```

**Outcomes:**
- Green → drop from list.
- Red → add to Step 5 (child workflow) or deeper root-cause investigation
  for error-handling. Summary: `toRivetError` in `actor/errors.ts` previously
  preferred `error.message` over fallback; US-102 moved sanitization to
  core's `build_internal`. If still red, check that path in `engine/packages/error/src/error.rs`.

Estimated time: 10 min.

---

### Step 2. `actor-inspector::POST /inspector/workflow/replay rejects workflows that are currently in flight`

**Why next:** deterministic (3/3 runs fail identically at 30s), no
statistics needed — one log run + one code read should explain it.

**Known context:**
- From `rivetkit-typescript/CLAUDE.md`:
  > Inspector replay tests should prove "workflow in flight" via inspector
  > `workflowState` (`pending` / `running`), not `entryMetadata.status` or
  > `runHandlerActive`, because those can lag or disagree across encodings.
  
  Strongly suggests the bug is on that same axis.
- From the same file:
  > `POST /inspector/workflow/replay` can legitimately return an empty
  > workflow-history snapshot when replaying from the beginning because
  > the endpoint clears persisted history before restarting the workflow.

**Approach:**
1. Read the test body:
   `rivetkit-typescript/packages/rivetkit/tests/driver/actor-inspector.test.ts`,
   grep for `rejects workflows that are currently in flight`.
2. Read the inspector replay handler: grep in
   `rivetkit-typescript/packages/rivetkit/src/inspector/` for the replay
   endpoint + the "in flight" guard. Likely in `actor-inspector.ts` or
   `src/actor/router.ts` (HTTP inspector).
3. Run the narrowed test once with full logs:
   ```bash
   pnpm test tests/driver/actor-inspector.test.ts \
     -t "static registry.*encoding \(bare\).*rejects workflows that are currently in flight" \
     > /tmp/driver-logs/inspector-replay.log 2>&1
   ```
4. Grep the captured log for the inspector request/response flow:
   ```bash
   grep -E "RT\.|ENG\." /tmp/driver-logs/inspector-replay.log \
     | grep -iE "inspector|workflow/replay|workflowState|pending|running|in.?flight|entryMetadata"
   ```
5. Look at what the test asserts vs. what the server actually returned.

**Likely outcomes:**
- Inspector reads `entryMetadata.status` or `runHandlerActive` instead of
  `workflowState` (the CLAUDE.md-documented trap).
- Inspector clears state before the in-flight check runs (endpoint
  lifecycle bug).

**Deliverables:**
- `.agent/notes/flake-inspector-replay.md` with evidence + fix direction.
- PRD story (`US-104`?) at priority ~10 (moderate — one test, inspector
  surface, low blast radius).

Estimated time: 15 min.

---

### Step 3. `actor-conn` WebSocket handshake flakes

**Why now:** largest remaining cluster (4 tests across 3 runs with
different tests failing each time). Probably shares root cause with
the actor-queue flakes in Step 4.

**Target tests** (all in `actor-conn.test.ts`, all with bare encoding):
- `Large Payloads > should reject request exceeding maxIncomingMessageSize` (30s timeout)
- `Large Payloads > should reject response exceeding maxOutgoingMessageSize` (30s timeout)
- `Connection State > isConnected should be false before connection opens` (~10s)
- `Connection State > onOpen should be called when connection opens` (~1.5s)

**Known context from prior debugging in this investigation:**
- One failure log showed the client-side WebSocket stayed at
  `readyState=0` for the full 10s before closing with code `1006`
  (generic abnormal closure — carries no useful info on its own).
- Client-side code that manages the connection lives in
  `rivetkit-typescript/packages/rivetkit/src/client/actor-conn.ts` and
  `src/engine-client/actor-websocket-client.ts`.
- Server side: runtime handles the open via
  `rivetkit-typescript/packages/rivetkit/src/registry/native.ts` (raw
  WebSocket dispatch) plus core `on_websocket` callback in
  `rivetkit-rust/packages/rivetkit-core/src/actor/`.

**Approach — narrow first:**

1. Start with `isConnected should be false before connection opens` —
   10s timeout means fast iteration, and the test body is the smallest.
2. Run 5× with full logs:
   ```bash
   for i in 1 2 3 4 5; do
     pnpm test tests/driver/actor-conn.test.ts \
       -t "static registry.*encoding \(bare\).*isConnected should be false before connection opens" \
       > /tmp/driver-logs/conn-isconnected-run$i.log 2>&1
     echo "run $i: $?"
   done
   ```
3. Collect all failing runs. For each, trace the WS lifecycle in the log:
   ```bash
   grep -E "RT\.|ENG\." /tmp/driver-logs/conn-isconnected-run<N>.log \
     | grep -iE "websocket|gateway|/connect|1006|ToEnvoyTunnel|ws.*open|ws.*close|tunnel_close|actor_ready_timeout|request_start|request_end|open.*websocket"
   ```
4. Identify which phase stalled. Three buckets:

   **Bucket A — gateway never forwards the `/connect`:**
   - Look for `opening websocket to actor via guard` (client-side)
     followed by NO matching `ToEnvoyRequestStart path: "/connect"`.
   - Likely gateway routing / auth / query-string parser issue.
     Check `rivetkit-typescript/packages/rivetkit/src/actor-gateway/gateway.ts`.

   **Bucket B — gateway forwards, actor never replies `Ok(())` to
   `WebSocketOpen`:**
   - Look for `ToEnvoyRequestStart path: "/connect"` followed by NO
     `client websocket open` / `socket open connId=...` within timeout.
   - User-code handler hang or `onBeforeConnect`/`createConnState` stuck.
     Cross-reference with `can_sleep_state` gates — is the conn being
     aborted by a sleep race?

   **Bucket C — actor replied, TCP never flips `readyState=1`:**
   - Look for `socket open messageQueueLength=...` (the runtime sent
     success) but client-side `readyState` stays 0.
   - Tunnel / proxy layer bug, or client-side `.onopen` never firing.
     Check `src/engine-client/actor-websocket-client.ts` `BufferedRemoteWebSocket`.

5. If evidence points into a bucket without clear resolution, temporarily
   add a `console.error` to `actor-websocket-client.ts` to log each state
   transition with a timestamp. Rerun.

6. Expand to the other 3 tests once the handshake path is understood.
   Large-payload tests may be the same bug manifesting differently (a
   slow handshake blocks the large-message paths).

**Deliverables:**
- `.agent/notes/flake-conn-websocket.md` with bucket classification and
  evidence.
- PRD story (`US-105`?) at priority ~8-9 (high — blocks a core-path test,
  affects multiple tests, may be gateway-wide).

Estimated time: 30 min.

---

### Step 4. `actor-queue` flakes

**Why contingent on Step 3:** both failing tests involve child-actor
reachability via queue-send, which uses the same WS / tunnel transport.
If Step 3 resolves the handshake bug, these may disappear. Run Step 4
ONLY if either (a) Step 3 finds the bug and you want to confirm
actor-queue is green after the fix, or (b) the target tests fail with
a different symptom than Step 3's handshake stall.

**Target tests:**
- `wait send returns completion response` (30s timeout, single actor).
- `drains many-queue child actors created from actions while connected` (55s then 11s, child actors).

**Order matters:**

1. `wait send returns completion response` first — no child actor, so
   can't be the handshake race. Clearest signal for queue-specific bugs.
2. Run 5×:
   ```bash
   for i in 1 2 3 4 5; do
     pnpm test tests/driver/actor-queue.test.ts \
       -t "static registry.*encoding \(bare\).*wait send returns completion response" \
       > /tmp/driver-logs/queue-waitsend-run$i.log 2>&1
   done
   ```
3. For failures, grep the queue + completion flow:
   ```bash
   grep -E "RT\.|ENG\." /tmp/driver-logs/queue-waitsend-run<N>.log \
     | grep -iE "enqueue|queue.*wait|QueueMessage|complete|completion|message_id|queue receive|on_queue_send|wait_for_names"
   ```
4. Look for:
   - The actor receives the message (log: `QueueMessage` class
     constructed, `invoking napi TSF callback kind=on_queue_send`).
   - The actor calls `message.complete(...)` back.
   - The completion reply travels back through NAPI + core to the client.
   - Where the chain breaks.

5. **CLAUDE.md pointer:**
   > For non-idempotent native waits like `queue.enqueueAndWait()`, bridge
   > JS `AbortSignal` through a standalone native `CancellationToken`;
   > timeout-slicing is only safe for receive-style polling calls like
   > `waitForNames()`.
   
   Verify `enqueue_and_wait` in `rivetkit-rust/packages/rivetkit-core/src/actor/queue.rs`
   and NAPI adapter use a separate cancel token and are not being
   cancelled by the actor abort token prematurely.

6. Then move to `drains many-queue child actors...` only if Step 3's
   WS handshake fix didn't clean it up.

**Deliverables:**
- `.agent/notes/flake-queue-waitsend.md`.
- PRD story if it's a distinct bug from Step 3.

Estimated time: 20 min.

---

### Step 5. `actor-workflow::starts child workflows created inside workflow steps`

**Skip if Step 1 shows it's now green.**

**Pre-US-103 symptom:** test expected 1 entry in `state.results`, got 2
identical "child-1" entries. Suspected: workflow step body re-executed
during replay and double-pushed state.

**Approach:**
1. Read the test and fixture:
   - Test: `rivetkit-typescript/packages/rivetkit/tests/driver/actor-workflow.test.ts`
     search `starts child workflows created inside workflow steps`.
   - Fixture:
     `rivetkit-typescript/packages/rivetkit/fixtures/driver-test-suite/workflow.ts`
     search `workflowSpawnParentActor`.
2. Anchor against the reference implementation per repo convention:
   ```bash
   git show feat/sqlite-vfs-v2:rivetkit-typescript/packages/workflow-engine/src/context.ts > /tmp/context-v2.ts
   diff /tmp/context-v2.ts rivetkit-typescript/packages/workflow-engine/src/context.ts \
     | head -200
   ```
   Focus on `step()` / `loop()` replay short-circuit logic.
3. Add temporary instrumentation to the fixture's step body to count
   invocations per replay. Rerun with logs.
4. If the body is running twice: check whether the recorded entry is
   being persisted atomically with the body's side effect (the actor
   state mutation `loopCtx.state.results.push(...)`). Workflow engine
   should skip the body on replay when the entry is already `completed`.
5. Compare with the original TS implementation at `feat/sqlite-vfs-v2`.
   If behavior there is different, port the fix.

**Deliverables:**
- `.agent/notes/flake-workflow-child-spawn.md`.
- PRD story if confirmed as workflow-engine replay bug.

Estimated time: 20 min.

---

### Step 6. `actor-workflow::workflow steps can destroy the actor` — decision point, not investigation

**Root cause already known** from prior investigation:
- Rust `engine/sdks/rust/envoy-client/src/handle.rs::destroy_actor`
  sends `protocol::ActorIntent::ActorIntentStop` — the same payload as
  `sleep_actor`.
- Envoy v2 protocol (`engine/sdks/schemas/envoy-protocol/v2.bare:276-282`)
  has only `ActorIntentSleep` and `ActorIntentStop`. No destroy variant.
- TS runner at `engine/sdks/typescript/runner/src/mod.ts:301,317-323`
  marks `actor.stopIntentSent = true` (a `graceful_exit`-style marker
  not wired through to Rust envoy-client).

**Options (do not pick without user input):**

- **(a)** Add a new envoy protocol version (v3) with `ActorIntentDestroy`.
  Real fix. Follow `engine/CLAUDE.md` VBARE migration rules exactly —
  never edit v2 schema in place, add versioned converter, do NOT bump
  runner-protocol unintentionally, etc. Blast radius: schema bump +
  versioned serializer + both Rust & TS envoy-client updates.
- **(b)** Wire the `graceful_exit` marker the TS runner uses. Figure out
  its side-band encoding (it's not in the v2 BARE, so must be a separate
  protocol message or an actor-state flag). Lower blast radius, probably
  not the long-term design.

Not a task for this investigation — do not start work until the user
picks (a) or (b).

---

## 2. Deliverables — summary

At end of investigation, you should have produced:

Under `.agent/notes/`:
- `flake-inspector-replay.md` (Step 2)
- `flake-conn-websocket.md` (Step 3)
- `flake-queue-waitsend.md` (Step 4, if distinct from Step 3)
- `flake-workflow-child-spawn.md` (Step 5, if still red)
- Updates to `driver-test-progress.md` reflecting new state

Under `scripts/ralph/prd.json`:
- 1-4 new stories as distinct root causes emerge

Under `/tmp/driver-logs/`:
- Per-run log files kept for at least the investigation's duration
- A `/tmp/driver-logs/README.md` summarizing which log file supports
  which claim in which writeup

Reverted:
- `shared-harness.ts` diagnostic mirrors (gate remained but mirror
  behavior should be kept as-is since it's env-gated and cheap when
  disabled; ask the user before reverting)

## 3. Scope and constraints

- Static registry, bare encoding only. Do NOT expand to cbor/json
  unless a bug is encoding-dependent.
- Do NOT fix anything. Investigation produces evidence + fix directions.
  Fixes land as separate PRD stories.
- Follow root repo conventions: no `vi.mock`, use Agent Browser for UI
  work if any, use `tracing` not `println!`, etc. See root `CLAUDE.md`.
- Anchor to `feat/sqlite-vfs-v2` as the behavioral oracle for any
  parity-vs-reference question.
- Each investigation step should fit in roughly the time estimate
  given. If a step balloons past 2× estimate, stop, write up what you
  have, and escalate to the user.

## 4. Total estimated time

~90 min if nothing surprises you. Step 3 (WS handshake) is the biggest
unknown. Step 6 (destroy) is decision-only, no time.
