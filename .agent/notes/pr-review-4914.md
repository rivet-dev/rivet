## PR Review: feat(kitchen-sink): add mock agentic lifecycle loop

### Overview

This PR substantially expands the mock agentic lifecycle test harness in `examples/kitchen-sink`. The script gains the ability to auto-spawn a local kitchen-sink server, run a concurrent probe loop validating actor liveness during sleep/reconnect cycles, track close events, validate full history consistency at teardown, and report aggregate statistics. The actor gains `onSleep` delay, `ping`/`pong` WebSocket support, and a `verifyAll` action.

---

### Issues

**1. Timer leak in `runProbeAttempt`**

The `timeout()` helper creates a `setTimeout` that is never cancelled when `Promise.race` resolves via a competing branch. Each probe attempt leaks up to two pending timers (open and pong phases). The inner pong promise already uses `cleanup()` correctly -- apply the same pattern to the outer `timeout()` helper.

**2. Platform-specific process management**

`pidsWithEnvValue` reads from a Linux-only pseudo-filesystem, failing silently or erroring on macOS. `listenerPids` uses `lsof` flags that differ across platforms. Add a runtime platform guard (return empty array on non-Linux) or document the assumption.

**3. Hard-coded 2-second sleeps in cleanup helpers**

`stopListeners` and `stopProcessesWithEnvValue` sleep 2 seconds after SIGTERM then force-kill. Polling the port check with a short interval would be more reliable.

**4. `DEFAULT_ON_SLEEP_DELAY_MS = 15_000` has no comment**

This 15-second delay in `onSleep` is non-obvious and extends test runtime. Per CLAUDE.md, add a comment explaining why it exists (presumably to let probe connections observe the actor stopping).

**5. IIFE for final history validation is hard to read**

The inline self-invoking async function around line 1121 should be extracted as a named function (e.g. `getFinalHistory()`).

**6. Double cast `as unknown as ActionVerifier`**

Bypasses the type system entirely. If actor action signatures change there is no compile-time check. Define the interface against the typed handle's action surface.

**7. Unbounded backlog in `RawSession`**

The `#backlog` array has no size cap. Add a cap (~500 entries) and shift off the oldest when exceeded.

**8. Repeated reconnect-time check**

`if (reconnectMs > MAX_RECONNECT_MS) throw` appears three times in `runWorkload`. Extract into a helper.

---

### Minor

- Removed `verifier` param from `runInference`: good -- verification is now explicit at the call site with layered checks.
- `connect()` returns `0` when already open (reads as "opened instantly" vs "was already open"). A named result would be clearer.
- `--import` args in `startLocalKitchenSinkServer`: add a comment that `@rivetkit/sql-loader` is required for SQLite.

---

### Summary

Architecture is sound; the probe loop + sleep-close observation pattern is a solid approach for validating lifecycle behavior under reconnect. Main blocking issue: timer leak in `runProbeAttempt`. Linux-only assumptions and the `onSleep` delay need documentation. IIFE and double-cast are maintainability items.
