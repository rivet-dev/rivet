---
name: sanity-check
description: Run an E2E smoke test that installs rivetkit packages from npm in an isolated temp project, starts the hello-world counter actor, then verifies both HTTP actions and WebSocket actions + events work end-to-end. Use when the user asks to sanity check, smoke test, or verify a rivetkit release/preview-publish works.
---

# Sanity Check

Run a quick end-to-end sanity check of a published rivetkit version: copy the hello-world example to a temp directory, install the specified package version from the public npm registry, start the dev server, and run a client test script that verifies both HTTP actions and WebSocket connections with event broadcasting.

## When to use
- User wants to verify a published rivetkit version works (e.g., `rivetkit@0.0.0-pr.4701.a818b77`, `rivetkit@latest`, `rivetkit@2.5.0`)
- After a preview publish to verify the build is functional
- After a release to verify the package installs and runs correctly
- User says "sanity check", "smoke test", "verify the build", or "test this version"

## Inputs
1. **Version or tag** (required) — explicit pkg-pr-new preview, npm dist-tag, or semver. If not provided in the user's message, ask for it.
2. **Additional test behavior** (optional) — e.g., "also verify workflows persist" or "check that KV works." If provided, extend `src/index.ts` + `test.mjs` using the menu in "Extending with custom tests" below.

## Usage
- `/sanity-check <version>` — run in a temp directory on the host
- `/sanity-check docker <version>` — run inside a `node:22` Docker container
- `/sanity-check <version> <custom instructions>` — any extra instructions (e.g. "also hit a KV action", "verify state persists across restart", "use pnpm", "test on node 20")

`<version>` is any npm-resolvable spec: an explicit pkg-pr-new preview (`0.0.0-pr.4701.a818b77`), an npm dist-tag (`latest`, `rc`, `next`), or a semver (`2.3.0-rc.4`). If the user omits it, ask for it.

## What it tests

1. `npm install` of `rivetkit`, `@rivetkit/react`, and the platform-specific `@rivetkit/rivetkit-napi-*` native binding from the public npm registry
2. Boot the hello-world counter actor server via `registry.start()` on port 6420
3. **HTTP path**: call `counter.increment(5)`, `counter.increment(3)`, `counter.getCount()` and assert the values
4. **WebSocket path**: open a `.connect()` conn, subscribe to the `newCount` event, call `increment(10)`, assert the action response AND the broadcast event value match
5. Report the resolved versions of `rivetkit` + `@rivetkit/rivetkit-napi`

## Requirements

- Node.js 22+ (or Docker with `node:22`)
- This repo available locally — only used to copy `examples/hello-world/src/` as the seed. Does NOT use any of its `node_modules` or workspace links.

## Steps

### 1. Set up the test project

```bash
REPO_ROOT=$(git rev-parse --show-toplevel)
SANITY_DIR=$(mktemp -d /tmp/rivetkit-sanity-XXXXXX)
cp -r "$REPO_ROOT/examples/hello-world/src" "$REPO_ROOT/examples/hello-world/tsconfig.json" "$SANITY_DIR/"
cd "$SANITY_DIR"
```

Write `package.json` with `<VERSION>` substituted:

```json
{
  "name": "rivetkit-sanity",
  "private": true,
  "type": "module",
  "dependencies": {
    "rivetkit": "<VERSION>",
    "@rivetkit/react": "<VERSION>"
  },
  "devDependencies": {
    "tsx": "^4",
    "typescript": "^5"
  }
}
```

For pkg-pr-new previews, `<VERSION>` can be the bare version string (`0.0.0-pr.4701.a818b77`) — npm resolves it directly. If that fails, fall back to the URL form: `"rivetkit": "https://pkg.pr.new/rivet-dev/rivet/rivetkit@<short-sha>"`.

### 2. Write `test.mjs`

```js
import { createClient } from "rivetkit/client";
import { spawn } from "node:child_process";

const ENDPOINT = "http://localhost:6420";

console.log("Starting counter actor server...");
const server = spawn("npx", ["tsx", "src/index.ts"], {
  stdio: ["ignore", "pipe", "pipe"],
});
let log = "";
server.stdout.on("data", (d) => (log += d));
server.stderr.on("data", (d) => (log += d));

async function waitForServer(timeoutMs = 30000) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    try {
      const r = await fetch(`${ENDPOINT}/health`);
      if (r.ok) return;
    } catch {}
    await new Promise((r) => setTimeout(r, 500));
  }
  console.error("--- server log ---\n" + log);
  throw new Error("server did not become ready");
}

let exitCode = 0;
try {
  await waitForServer();
  const client = createClient(ENDPOINT);

  // --- HTTP actions ---
  console.log("Testing HTTP actions...");
  const counter = client.counter.getOrCreate(["sanity"]);
  const a = await counter.increment(5);
  const b = await counter.increment(3);
  const c = await counter.getCount();
  if (a !== 5) throw new Error(`increment(5) => ${a}, expected 5`);
  if (b !== 8) throw new Error(`increment(3) => ${b}, expected 8`);
  if (c !== 8) throw new Error(`getCount() => ${c}, expected 8`);
  console.log(`  HTTP: increment(5)=${a}, increment(3)=${b}, getCount()=${c}`);

  // --- WebSocket actions + events ---
  console.log("Testing WebSocket + events...");
  const ws = client.counter.getOrCreate(["sanity-ws"]).connect();
  await new Promise((res, rej) => {
    const t = setTimeout(() => rej(new Error("ws open timeout")), 10000);
    ws.onOpen(() => {
      clearTimeout(t);
      res();
    });
    ws.onError(rej);
  });
  const eventPromise = new Promise((res, rej) => {
    const t = setTimeout(() => rej(new Error("newCount event timeout")), 5000);
    ws.on("newCount", (v) => {
      clearTimeout(t);
      res(v);
    });
  });
  const wsCount = await ws.increment(10);
  const eventValue = await eventPromise;
  if (wsCount !== 10) throw new Error(`ws increment(10) => ${wsCount}, expected 10`);
  if (eventValue !== 10) throw new Error(`newCount event => ${eventValue}, expected 10`);
  console.log(`  WS: increment(10)=${wsCount}, event=${eventValue}`);
  await ws.dispose();

  console.log("\n✅ E2E TEST PASSED");
} catch (err) {
  console.error(`\n❌ E2E TEST FAILED: ${err.message || err}`);
  console.error("--- server log (last 2KB) ---\n" + log.slice(-2000));
  exitCode = 1;
} finally {
  server.kill("SIGKILL");
  process.exit(exitCode);
}
```

### 3. Install + run

**Default (host):**
```bash
cd "$SANITY_DIR"
npm install
node test.mjs
```

If you need to inspect a failure after the fact, tee the output:
```bash
node test.mjs 2>&1 | tee /tmp/sanity-check.log
echo "exit=$?"
```

**Docker mode:**
```bash
docker run --rm \
  -v "$SANITY_DIR":/app \
  -w /app \
  node:22 \
  bash -c "npm install && timeout 120 node test.mjs"
```

### 4. Report installed versions

Surface the resolved versions (rivetkit's `exports` doesn't expose `./package.json`, so read the file directly):

```bash
node -e "
for (const p of ['rivetkit','@rivetkit/react','@rivetkit/rivetkit-napi']) {
  try {
    const v = JSON.parse(require('fs').readFileSync('node_modules/'+p+'/package.json','utf8')).version;
    console.log(p, v);
  } catch (e) { console.log(p, '(not installed:', e.code || e.message, ')'); }
}
"
```

### 5. Report results

Tell the user:
- Resolved versions (rivetkit + @rivetkit/rivetkit-napi)
- HTTP path results
- WebSocket path results + event value
- ✅ or ❌ with the last 2KB of server log on failure

### 6. Clean up

```bash
rm -rf "$SANITY_DIR"
```

## Extending with custom tests

If the user asks for extra behavior, modify `src/index.ts` and add assertions to `test.mjs` before running. Common asks and where to slot them:

- **KV round-trip**: add `kv` actions (`setKv: (c, key, val) => c.kv.set(key, val)`, `getKv: (c, key) => c.kv.get(key)`), then in test.mjs call set → get and assert.
- **Workflow**: add a workflow action, await its completion, assert the final state.
- **SQLite + migrations**: add a migration and a query action, call it, assert rows.
- **State persistence**: increment, kill the server (`server.kill("SIGTERM")`), await exit, respawn, call `getCount()`, assert value preserved.
- **Multiple actor instances**: use different keys, verify isolation.

Start from the base test above; layer additions rather than rewriting it.

## Rules
- Always use a fresh temp directory — never run in the repo itself.
- Always install from the public npm registry or pkg-pr-new — never use local workspace links / `file:` deps.
- Pin `rivetkit` + `@rivetkit/react` to the exact user-specified version; let npm's `optionalDependencies` resolve the right `@rivetkit/rivetkit-napi-<platform>` binary automatically.
- If `npm install` fails to resolve a bare pkg-pr-new version string, retry using the `https://pkg.pr.new/rivet-dev/rivet/<pkg>@<short-sha>` URL form.
- If the server doesn't reach `/health` in 30s, dump the last 2KB of server stderr/stdout before failing — most install/runtime issues show up there (missing native binary, wrong Node version, port collision).
- On Docker mode, run `rm -rf $SANITY_DIR` only after `docker run --rm` exits so container-created `node_modules` get cleaned by the `--rm` flag.
