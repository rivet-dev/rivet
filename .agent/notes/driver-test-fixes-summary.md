# Driver Test Fixes Summary

This was not one bug. It was a stack of separate issues that all showed up in the file-system and engine driver suites.

## Main fixes

1. Raw and Drizzle SQLite were not choosing the same backend path.

Raw `rivetkit/db` could use the native addon while `rivetkit/db/drizzle` always went through the WASM VFS. I unified backend selection behind shared open logic in [open-database.ts](/home/nathan/r2/rivetkit-typescript/packages/rivetkit/src/db/open-database.ts), with a native `IDatabase` adapter in [native-adapter.ts](/home/nathan/r2/rivetkit-typescript/packages/rivetkit/src/db/native-adapter.ts). Both providers now reuse that in [mod.ts](/home/nathan/r2/rivetkit-typescript/packages/rivetkit/src/db/mod.ts) and [mod.ts](/home/nathan/r2/rivetkit-typescript/packages/rivetkit/src/db/drizzle/mod.ts). This removed a class of raw-path versus drizzle-path inconsistencies.

2. The native SQLite KV channel had actor-resolution bugs.

The KV channel was caching actor resolution too coarsely, which is wrong when one connection multiplexes many actors. That was fixed in [lib.rs](/home/nathan/r2/engine/packages/pegboard-kv-channel/src/lib.rs). I also fixed the open race where native SQLite could attempt `sqlite3_open_v2` before the actor was actually resolvable on the engine side, which was the source of the earlier `SQLITE_CANTOPEN` and internal-error failures. Related engine-side actor lifecycle and lookup changes were needed in [create.rs](/home/nathan/r2/engine/packages/pegboard/src/ops/actor/create.rs), [get_for_key.rs](/home/nathan/r2/engine/packages/pegboard/src/ops/actor/get_for_key.rs), [mod.rs](/home/nathan/r2/engine/packages/pegboard/src/workflows/actor2/mod.rs), and [runtime.rs](/home/nathan/r2/engine/packages/pegboard/src/workflows/actor2/runtime.rs).

3. The native client path was not resilient to channel loss.

When the native KV channel went stale, the old code surfaced connection-closed errors instead of reopening and retrying. I fixed that in [native-sqlite.ts](/home/nathan/r2/rivetkit-typescript/packages/rivetkit/src/db/native-sqlite.ts) and [native-adapter.ts](/home/nathan/r2/rivetkit-typescript/packages/rivetkit/src/db/native-adapter.ts). I also fixed crash cleanup ordering so hard-crash tests did not abort DB cleanup too early in [mod.ts](/home/nathan/r2/rivetkit-typescript/packages/rivetkit/src/actor/instance/mod.ts).

4. Query-backed actor handles and gateway routing had mismatches.

Some engine failures were because `.get()` / `.getOrCreate()` / `.getForId()` were not consistently preserving the query-backed gateway path the tests expected. That was fixed in [actor-handle.ts](/home/nathan/r2/rivetkit-typescript/packages/rivetkit/src/client/actor-handle.ts), [actor-query.ts](/home/nathan/r2/rivetkit-typescript/packages/rivetkit/src/client/actor-query.ts), [resolve-gateway-target.ts](/home/nathan/r2/rivetkit-typescript/packages/rivetkit/src/driver-helpers/resolve-gateway-target.ts), [mod.ts](/home/nathan/r2/rivetkit-typescript/packages/rivetkit/src/remote-manager-driver/mod.ts), and [mod.rs](/home/nathan/r2/engine/packages/guard/src/routing/pegboard_gateway/mod.rs). That cleared the `.get().connect()` and `.getForId().connect()` engine proxy failures.

5. Raw websocket lifecycle handling had real race bugs.

There were two separate websocket problems:

- `Conn.disconnect()` was re-entrant, so a local close and the resulting close event could double-run cleanup. I made it single-flight in [mod.ts](/home/nathan/r2/rivetkit-typescript/packages/rivetkit/src/actor/conn/mod.ts).
- Actors could auto-sleep before a wake-triggered raw websocket open had actually been delivered into RivetKit. I first fixed the local `prepareConn -> connectConn` gap in [connection-manager.ts](/home/nathan/r2/rivetkit-typescript/packages/rivetkit/src/actor/instance/connection-manager.ts), then fixed the earlier engine-only wake-to-open gap by adding a driver-level initial sleep override in [driver.ts](/home/nathan/r2/rivetkit-typescript/packages/rivetkit/src/actor/driver.ts), consumed in [mod.ts](/home/nathan/r2/rivetkit-typescript/packages/rivetkit/src/actor/instance/mod.ts) and implemented in [actor-driver.ts](/home/nathan/r2/rivetkit-typescript/packages/rivetkit/src/drivers/engine/actor-driver.ts). That was the last real engine flake.

6. AgentOS process callbacks were leaking shutdown-time errors into Vitest.

The file-system suite ended up green on assertions but still failed because background process stdout and stderr callbacks were calling `broadcast()` after the actor had already shut down. I made those shutdown-safe in [process.ts](/home/nathan/r2/rivetkit-typescript/packages/rivetkit/src/agent-os/actor/process.ts). That removed the final unhandled rejections from the file-system run.

## Supporting fixes

There were also supporting changes needed to make the suites reliably green:

- Better raw websocket open ordering and route handling in [router-websocket-endpoints.ts](/home/nathan/r2/rivetkit-typescript/packages/rivetkit/src/actor/router-websocket-endpoints.ts) and [actor-driver.ts](/home/nathan/r2/rivetkit-typescript/packages/rivetkit/src/drivers/engine/actor-driver.ts).
- File-system driver and global-state cleanup and sleep behavior fixes in [global-state.ts](/home/nathan/r2/rivetkit-typescript/packages/rivetkit/src/drivers/file-system/global-state.ts), [actor.ts](/home/nathan/r2/rivetkit-typescript/packages/rivetkit/src/drivers/file-system/actor.ts), and [manager.ts](/home/nathan/r2/rivetkit-typescript/packages/rivetkit/src/drivers/file-system/manager.ts).
- Remote websocket client and proxy fixes in [actor-websocket-client.ts](/home/nathan/r2/rivetkit-typescript/packages/rivetkit/src/remote-manager-driver/actor-websocket-client.ts) and [actor-conn.ts](/home/nathan/r2/rivetkit-typescript/packages/rivetkit/src/client/actor-conn.ts).
- Test harness changes in [driver-engine.test.ts](/home/nathan/r2/rivetkit-typescript/packages/rivetkit/tests/driver-engine.test.ts), [utils.ts](/home/nathan/r2/rivetkit-typescript/packages/rivetkit/src/driver-test-suite/utils.ts), and several driver-test files so the engine suite stopped fighting its own setup and teardown.

## Net result

The important root causes were:

- backend split between raw and drizzle,
- native KV and open races,
- query-backed gateway mismatches,
- raw-websocket sleep and disconnect races,
- shutdown-time async callbacks escaping actor lifetime.

Once those were fixed, both suites passed cleanly:

- [driver-file-system-full-after-process-fix.log](/tmp/driver-file-system-full-after-process-fix.log)
- [driver-engine-full-after-initial-sleep-fix.log](/tmp/driver-engine-full-after-initial-sleep-fix.log)
