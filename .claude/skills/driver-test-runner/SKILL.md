---
name: driver-test-runner
description: Methodically run the RivetKit driver test suite file by file, tracking progress in .agent/notes/driver-test-progress.md. Use when you need to validate the driver test suite after changes, bring up a new driver, or debug test failures systematically.
allowed-tools: Bash, Read, Write, Edit, Grep, Glob, Agent, TaskCreate, TaskUpdate
---

# Driver Test Suite Runner

Methodically run the RivetKit driver test suite one file group at a time, tracking progress in `.agent/notes/driver-test-progress.md`.

## Arguments

The skill accepts optional arguments:

- **`reset`** — Clear progress and start from the beginning.
- **`resume`** — Pick up from where we left off (default behavior).
- **`only <file>`** — Run only a specific test file group (e.g., `only actor-conn`).
- **`from <file>`** — Start from a specific file group, skipping earlier ones.
- **`encoding <bare|cbor|json>`** — Override encoding (default: `bare`).
- **`client <http|inline>`** — Override client type (default: `http`).
- **`registry <static>`** — Override registry type (default: `static`).

## How It Works

### 0. Anchor the reference before fixing parity bugs

If a RivetKit driver test fails because native or Rust behavior diverges from the old runtime, do this before inventing a separate debugging workflow:

1. Treat `rivetkit-typescript/packages/rivetkit` driver tests as the primary oracle.
2. Compare the failing behavior against the original TypeScript implementation at ref `feat/sqlite-vfs-v2` using `git show` or `git diff`.
3. Patch native/Rust to match the original TypeScript behavior.
4. Rerun the same TypeScript driver test before adding any lower-level native tests.

Native unit tests are allowed only after the failing TypeScript driver test has reproduced the bug and after the fix is validated against that same TypeScript driver test.

### 1. Ensure the engine is running

Before running any tests, check if the RocksDB engine is already running:

```bash
curl -sf http://127.0.0.1:6420/health || echo "NOT RUNNING"
```

If it's not running, start it:

```bash
./scripts/run/engine-rocksdb.sh >/tmp/rivet-engine-startup.log 2>&1 &
```

Wait for health check to pass (poll every 2 seconds, up to 60 seconds).

### 2. Initialize or load progress file

The progress file lives at `.agent/notes/driver-test-progress.md`. If it doesn't exist or `reset` was passed, create it with the template below. If it exists and `resume` was passed, read it and pick up from the first unchecked file.

Progress file template:

```markdown
# Driver Test Suite Progress

Started: <timestamp>
Config: registry (static), client type (http), encoding (bare)

## Fast Tests

- [ ] manager-driver | Manager Driver Tests
- [ ] actor-conn | Actor Connection Tests
- [ ] actor-conn-state | Actor Connection State Tests
- [ ] conn-error-serialization | Connection Error Serialization Tests
- [ ] actor-destroy | Actor Destroy Tests
- [ ] request-access | Request Access in Lifecycle Hooks
- [ ] actor-handle | Actor Handle Tests
- [ ] action-features | Action Features Tests
- [ ] access-control | access control
- [ ] actor-vars | Actor Variables
- [ ] actor-metadata | Actor Metadata Tests
- [ ] actor-onstatechange | Actor State Change Tests
- [ ] actor-db | Actor Database
- [ ] actor-db-raw | Actor Database Raw Tests
- [ ] actor-workflow | Actor Workflow Tests
- [ ] actor-error-handling | Actor Error Handling Tests
- [ ] actor-queue | Actor Queue Tests
- [ ] actor-kv | Actor KV Tests
- [ ] actor-stateless | Actor Stateless Tests
- [ ] raw-http | raw http
- [ ] raw-http-request-properties | raw http request properties
- [ ] raw-websocket | raw websocket
- [ ] actor-inspector | Actor Inspector Tests
- [ ] gateway-query-url | Gateway Query URL Tests
- [ ] actor-db-pragma-migration | Actor Database Pragma Migration
- [ ] actor-state-zod-coercion | Actor State Zod Coercion
- [ ] actor-conn-status | Connection Status Changes
- [ ] gateway-routing | Gateway Routing
- [ ] lifecycle-hooks | Lifecycle Hooks

## Slow Tests

- [ ] actor-state | Actor State Tests
- [ ] actor-schedule | Actor Schedule Tests
- [ ] actor-sleep | Actor Sleep Tests
- [ ] actor-sleep-db | Actor Sleep Database Tests
- [ ] actor-lifecycle | Actor Lifecycle Tests
- [ ] actor-conn-hibernation | Actor Connection Hibernation Tests
- [ ] actor-run | Actor Run Tests
- [ ] hibernatable-websocket-protocol | hibernatable websocket protocol
- [ ] actor-db-stress | Actor Database Stress Tests

## Excluded

- [ ] actor-agent-os | Actor agentOS Tests (skip unless explicitly requested)

## Log
```

### 3. Run tests file by file

For each unchecked file in order:

**a) Build the filter command:**

Each suite now lives in its own file under `rivetkit-typescript/packages/rivetkit/tests/driver/<file>.test.ts`. The describe block nesting is:

```
<Outer Suite> > static registry > encoding (<encoding>) > <Suite Description>
```

There is no longer a `Driver Tests` or `client type (http)` layer.

Base command:

```bash
cd rivetkit-typescript/packages/rivetkit && pnpm test tests/driver/<FILE>.test.ts -t "static registry.*encoding \\(bare\\).*<SUITE_DESCRIPTION>" > /tmp/driver-test-current.log 2>&1
echo "EXIT: $?"
```

Replace `<FILE>` with the file name stem (part before the `|` in the progress file) and `<SUITE_DESCRIPTION>` with the suite description (part after the `|`). Escape parentheses in the description if present.

**Important:** The suite description in the `-t` filter must match the `describe(...)` text in the test file exactly. Some mappings:

| File | Suite Description Text |
|------|----------------------|
| manager-driver | Manager Driver Tests |
| actor-conn | Actor Connection Tests |
| actor-conn-state | Actor Connection State Tests |
| conn-error-serialization | Connection Error Serialization Tests |
| actor-destroy | Actor Destroy Tests |
| request-access | Request Access in Lifecycle Hooks |
| actor-handle | Actor Handle Tests |
| action-features | Action Features Tests |
| access-control | access control |
| actor-vars | Actor Variables |
| actor-metadata | Actor Metadata Tests |
| actor-onstatechange | Actor State Change Tests |
| actor-db | Actor Database |
| actor-db-raw | Actor Database Raw Tests |
| actor-workflow | Actor Workflow Tests |
| actor-error-handling | Actor Error Handling Tests |
| actor-queue | Actor Queue Tests |
| actor-kv | Actor KV Tests |
| actor-stateless | Actor Stateless Tests |
| raw-http | raw http |
| raw-http-request-properties | raw http request properties |
| raw-websocket | raw websocket |
| actor-inspector | Actor Inspector Tests |
| gateway-query-url | Gateway Query URL Tests |
| actor-db-pragma-migration | Actor Database Pragma Migration |
| actor-state-zod-coercion | Actor State Zod Coercion |
| actor-conn-status | Connection Status Changes |
| gateway-routing | Gateway Routing |
| lifecycle-hooks | Lifecycle Hooks |
| actor-state | Actor State Tests |
| actor-schedule | Actor Schedule Tests |
| actor-sleep | Actor Sleep Tests |
| actor-sleep-db | Actor Sleep Database Tests |
| actor-lifecycle | Actor Lifecycle Tests |
| actor-conn-hibernation | Actor Connection Hibernation Tests |
| actor-run | Actor Run Tests |
| hibernatable-websocket-protocol | hibernatable websocket protocol |
| actor-db-stress | Actor Database Stress Tests |
| actor-agent-os | Actor agentOS Tests |

**b) Pipe output to file and analyze:**

Always pipe test output to `/tmp/driver-test-current.log` so you can grep it afterward:

```bash
cd rivetkit-typescript/packages/rivetkit && pnpm test tests/driver/<FILE>.test.ts -t "static registry.*encoding \\(bare\\).*<SUITE>" > /tmp/driver-test-current.log 2>&1
echo "EXIT: $?"
```

Then analyze:

```bash
grep -E "Tests|FAIL|PASS|Error|✓|✗|×" /tmp/driver-test-current.log | tail -30
```

**c) If all tests pass:** Check off the file in the progress file and append to the log section:

```
- <timestamp> <file>: PASS (<N> tests, <duration>)
```

**d) If tests fail:**

1. Do NOT move to the next file.
2. Narrow down to the first failing test using a more specific `-t` filter.
3. Read the error output to understand the failure.
4. Append to the log section:

```
- <timestamp> <file>: FAIL - <brief description of failure>
```

5. Report the failure to the user with:
   - Which test file group failed
   - Which specific test(s) failed
   - The error message
   - Suggested next steps

### 4. Narrowing scope on failure

If a file group fails, narrow to individual tests:

```bash
cd rivetkit-typescript/packages/rivetkit && pnpm test tests/driver/<FILE>.test.ts -t "static registry.*encoding \\(bare\\).*<SUITE>.*<PARTIAL_TEST_NAME>" > /tmp/driver-test-narrow.log 2>&1
```

### 5. Completion

When all files are checked, append to the log:

```
- <timestamp> ALL TESTS COMPLETE
```

Report summary:
- Total files passing
- Total files failing (with names)
- Total duration

## Rules

1. **One file at a time.** Never run the full suite. The whole point is methodical, scoped testing.
2. **Fix before advancing.** Do not skip a failing file to test the next one (unless the user says to skip).
3. **Always pipe to file.** Never rely on inline terminal output for test results. Always write to `/tmp/driver-test-current.log` and grep afterward.
4. **Track everything.** Every run gets logged in the progress file.
5. **Use `actor-db-stress` encoding config.** The stress tests run once with `bare` encoding, not per-encoding. They are outside the encoding loop in mod.ts.
6. **Respect timeouts.** Set a 600-second timeout for slow tests (sleep, lifecycle, stress). Use 120 seconds for fast tests.
