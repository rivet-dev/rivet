# agentOS Proposal

## Overview

agentOS is a high-level wrapper around the [secure-exec](../../../secure-exec-1) kernel that provides a clean API for:

1. **Phase 1** - Core VM operations: command execution, networking (fetch), and filesystem access
2. **Phase 2** - Agent session management: spawning and communicating with coding agents (starting with PI) via the Agent Communication Protocol (ACP)

All code execution, networking, and agent processes run **inside the VM**. Nothing is ever spawned on the host.

---

## Architecture

```
┌──────────────────────────────────────────────────────────┐
│  agentOS (public API)                                     │
│                                                           │
│  const vm = new AgentOs()                                 │
│  await vm.execute("node", ["-e", "console.log('hi')"])   │
│  await vm.fetch(3000, request)                            │
│  const session = await vm.createSession("pi")             │
│                                                           │
├──────────────────────────────────────────────────────────┤
│  @rivet-dev/agent-os (single package)                │
│  - AgentOs class (wraps Kernel)                           │
│  - Proxies kernel VFS, exec, spawn                        │
│  - fetch() reaches services running inside the VM         │
│  - ACP client: JSON-RPC 2.0 over stdio                    │
│  - Session lifecycle (initialize, prompt, update)         │
│  - Agent configs (pi-acp, opencode)                       │
│                                                           │
├──────────────────────────────────────────────────────────┤
│  secure-exec (linked dependency)                          │
│  - Kernel: VFS, process table, socket table, PTY          │
│  - Node.js runtime driver (V8 isolates)                   │
│  - WasmVM runtime driver (POSIX commands)                 │
│  - Python runtime driver                                  │
│                                                           │
└──────────────────────────────────────────────────────────┘
```

---

## Monorepo Structure

Mirrors secure-exec. Uses pnpm workspaces + Turborepo + TypeScript + Biome.

```
agent-os/
├── package.json              (private, workspaces root)
├── pnpm-workspace.yaml
├── turbo.json
├── biome.json
├── tsconfig.json             (shared base - not used directly)
├── .gitignore
├── LICENSE                   (Apache-2.0, existing)
│
├── packages/
│   └── core/                 (single package: @rivet-dev/agent-os)
│       ├── package.json
│       ├── tsconfig.json
│       ├── vitest.config.ts
│       ├── src/
│       │   ├── index.ts      (barrel export)
│       │   ├── agent-os.ts   (AgentOs class - VM ops + session management)
│       │   ├── acp-client.ts (ACP JSON-RPC client over stdio)
│       │   ├── session.ts    (session lifecycle)
│       │   ├── protocol.ts   (JSON-RPC types/helpers)
│       │   ├── agents.ts     (agent configs: pi, opencode)
│       │   └── types.ts
│       └── tests/
│           ├── execute.test.ts
│           ├── filesystem.test.ts
│           ├── network.test.ts
│           ├── pi-headless.test.ts
│           ├── pi-acp-adapter.test.ts
│           └── session.test.ts
│
├── scripts/
│   └── ralph/                (existing)
│
└── notes/                    (existing)
```

---

## Phase 1: Core API

### AgentOs Class

```typescript
import { AgentOs } from "@rivet-dev/agent-os";

// Create a VM with all runtimes loaded (Node, WasmVM, Python)
// Async factory because kernel.mount() is async
const vm = await AgentOs.create();

// --- Command Execution (mirrors kernel.exec / kernel.spawn) ---
const result = await vm.exec("echo hello && ls /");
// result.exitCode, result.stdout, result.stderr

const proc = vm.spawn("node", ["-e", "console.log('hi')"], { cwd: "/tmp" });
proc.writeStdin("input\n");
proc.closeStdin();
const exitCode = await proc.wait();

// --- Filesystem (mirrors kernel VFS) ---
await vm.writeFile("/tmp/hello.txt", "world");
const data = await vm.readFile("/tmp/hello.txt");  // Uint8Array
await vm.mkdir("/tmp/mydir");
const entries = await vm.readdir("/tmp/mydir");
const stat = await vm.stat("/tmp/hello.txt");
const exists = await vm.exists("/tmp/hello.txt");

// --- Networking ---
// Services running INSIDE the VM are reachable via fetch:
// 1. Start a server inside the VM (use spawn, not exec -- exec blocks until exit)
const server = vm.spawn("node", ["/tmp/server.js"]);
// 2. Wait for server to be ready (poll socketTable or stdout callback)
// 3. Reach it from outside via vm.fetch()
const response = await vm.fetch(3000, new Request("http://127.0.0.1:3000/"));
// response is a standard Response object

// --- Shell (PTY) ---
const shell = vm.openShell({ cols: 80, rows: 24 });
shell.onData = (data) => process.stdout.write(data);
shell.write("ls /\n");
await shell.wait();

// --- Cleanup ---
await vm.dispose();
```

### API Surface

The `AgentOs` class directly proxies the kernel API:

| AgentOs Method | Kernel Method | Notes |
|---|---|---|
| `exec(command, options?)` | `kernel.exec(command, options)` | Same signature |
| `spawn(command, args, options?)` | `kernel.spawn(command, args, options)` | Same signature |
| `readFile(path)` | `kernel.readFile(path)` | |
| `writeFile(path, content)` | `kernel.writeFile(path, content)` | |
| `mkdir(path)` | `kernel.mkdir(path)` | |
| `readdir(path)` | `kernel.readdir(path)` | |
| `stat(path)` | `kernel.stat(path)` | |
| `exists(path)` | `kernel.exists(path)` | |
| `openShell(options?)` | `kernel.openShell(options)` | |
| `fetch(port, request)` | Via `kernel.socketTable` / network adapter | See networking section |
| `dispose()` | `kernel.dispose()` | |

### Networking: `vm.fetch(port, request)`

The kernel supports HTTP servers running inside the VM that bind to real host ports via the `HostNetworkAdapter`. The `fetch(port, request)` method reaches these internal servers.

**How it works under the hood:**
1. Code inside the VM calls `http.createServer().listen(port, '127.0.0.1')`
2. The kernel's socket table delegates to `hostAdapter.tcpListen()`, creating a real OS listener on the host
3. `vm.fetch(port, request)` uses host-side `globalThis.fetch()` to `http://127.0.0.1:{port}/...`
4. The connection hits the real host port, is accepted by the kernel's accept pump, routed to the VM's HTTP server

**Note:** The kernel has no `fetch()` method. This is implemented entirely in agentOS using the host's native `fetch()`. The `DefaultNetworkAdapter` loopback checker is a per-isolate bridge concept and is not involved here. To prevent fetching arbitrary host ports, agentOS can optionally verify the port is in `kernel.socketTable` via `findListener()` before making the request.

### Default Configuration

`await AgentOs.create()` with no arguments will:
- Create a kernel with `createInMemoryFileSystem()`
- Mount `createNodeRuntime()` (provides `node`, `npm`, etc.)
- Mount WasmVM runtime (provides `sh`, `bash`, `grep`, `sed`, `ls`, etc.)
- Mount Python runtime (provides `python`)
- Create a `HostNetworkAdapter` for external network access
- Use default permissions (allow-all for simplicity in v1)

---

## Phase 2: Agent Sessions (ACP)

### Overview

Agent sessions spawn coding agents **inside the VM** using the Agent Communication Protocol (ACP). The flow is:

```
vm.createSession("pi")
  └─> kernel.spawn("npx", ["-y", "pi-acp"])     (inside VM)
        └─> pi-acp spawns pi                      (inside VM)
              └─> JSON-RPC 2.0 over stdio          (stdin/stdout)
```

### ACP Protocol

ACP uses JSON-RPC 2.0 over newline-delimited stdio. Key message types:

**Requests (client -> agent):**
```json
{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":1,"clientCapabilities":{}}}
{"jsonrpc":"2.0","id":2,"method":"session/new","params":{"cwd":"/home/user","mcpServers":[]}}
{"jsonrpc":"2.0","id":3,"method":"session/prompt","params":{"sessionId":"...","prompt":[{"type":"text","text":"Hello"}]}}
{"jsonrpc":"2.0","id":4,"method":"session/cancel","params":{"sessionId":"..."}}
```

**Responses (agent -> client):**
```json
{"jsonrpc":"2.0","id":1,"result":{"protocolVersion":1,"agentCapabilities":{},"agentInfo":{"name":"pi","version":"0.60.0"}}}
{"jsonrpc":"2.0","id":2,"result":{"sessionId":"sess_abc123"}}
```

**Notifications (agent -> client, no `id`):**
```json
{"jsonrpc":"2.0","method":"session/update","params":{"sessionId":"...","type":"text","text":"Here's the code..."}}
```

### pi-acp Package

- **npm package:** `pi-acp` (version 0.0.23)
- **Spawned as:** `npx -y pi-acp` inside the VM
- **Protocol:** JSON-RPC 2.0 over stdin/stdout
- **Under the hood:** pi-acp spawns the PI coding agent (`@mariozechner/pi-coding-agent`)
- **Environment:** Needs `ANTHROPIC_API_KEY` or equivalent for the LLM provider
- **Quirks (from sandbox-agent):**
  - `protocolVersion` in `initialize` must be numeric (not string `"1.0"`)
  - `session/new` requires `mcpServers` field (default to `[]` if missing)

### Session API

```typescript
// Create a session (spawns pi-acp inside the VM)
const session = await vm.createSession("pi", {
  cwd: "/home/user",
  env: { ANTHROPIC_API_KEY: "sk-..." },
});

// Send a prompt
await session.prompt("Write a function that sorts an array");

// Listen for events (notifications from the agent)
session.onSessionEvent((event) => {
  // event.method: "session/update"
  // event.params: { type: "text", text: "..." } or tool calls, etc.
  console.log(event);
});

// Cancel ongoing work
await session.cancel();

// Close session
await session.close();
```

### Session Lifecycle

1. **Spawn pi-acp** inside the VM via `kernel.spawn("node", ["/path/to/pi-acp/bin/cli.js"], { onStdout })` (resolve from mounted node_modules, not npx)
2. **Initialize** - Send `initialize` request, receive capabilities
3. **Create session** - Send `session/new` with cwd and mcpServers
4. **Prompt** - Send `session/prompt`, receive streaming `session/update` notifications
5. **Close** - Kill the pi-acp process

### ACP Client (src/acp-client.ts)

The ACP client handles JSON-RPC communication over stdio with the spawned process:

```typescript
class AcpClient {
  // Created with the ManagedProcess (for writeStdin/kill) and an externally
  // wired stdout line buffer. Stdout is captured via SpawnOptions.onStdout
  // callback at spawn time, not from ManagedProcess (which has no onStdout).
  constructor(process: ManagedProcess, stdoutLines: AsyncIterable<string>);

  // Send request, await response (correlated by id)
  request(method: string, params: object): Promise<JsonRpcResponse>;

  // Send notification (fire-and-forget, no id)
  notify(method: string, params: object): void;

  // Subscribe to notifications from the agent
  onNotification(handler: (method: string, params: object) => void): void;

  // Lifecycle
  close(): void;
}
```

**Implementation details:**
- Stdout is wired via `SpawnOptions.onStdout` callback at spawn time, buffered into lines, fed to AcpClient as an async iterable
- Parses each line as JSON-RPC; skips non-JSON lines (agent startup banners, warnings)
- Correlates responses to pending requests by `id` field
- Broadcasts notifications (no `id`) to subscribers
- Writes requests to stdin via `process.writeStdin()` (requires streaming stdin -- see Step 0a)
- Timeout on pending requests (default 120s)
- On process exit: immediately reject all pending request promises (don't wait for timeout)

---

## Testing Strategy

All tests use **vitest**. All operations happen inside the VM.

### Phase 1 Tests

#### `execute.test.ts` - Command execution
```typescript
test("exec returns stdout", async () => {
  const vm = await AgentOs.create();
  const result = await vm.exec("echo hello");
  expect(result.stdout).toBe("hello\n");
  expect(result.exitCode).toBe(0);
  await vm.dispose();
});

test("exec returns stderr and non-zero exit code", async () => { ... });
test("exec with env vars", async () => { ... });
test("exec with cwd", async () => { ... });
test("exec with stdin", async () => { ... });
test("spawn and interact with process", async () => { ... });
test("exec node script", async () => { ... });
test("exec shell pipeline", async () => { ... });
```

#### `filesystem.test.ts` - Filesystem operations
```typescript
test("writeFile and readFile round-trip", async () => { ... });
test("mkdir and readdir", async () => { ... });
test("stat returns file info", async () => { ... });
test("exists returns true for existing file", async () => { ... });
test("exists returns false for missing file", async () => { ... });
```

#### `network.test.ts` - Networking
```typescript
test("fetch reaches server running inside VM", async () => {
  const vm = await AgentOs.create();

  // Write a server script to the VM filesystem
  await vm.writeFile("/tmp/server.js", `
    const http = require('http');
    const server = http.createServer((req, res) => {
      res.writeHead(200, { 'Content-Type': 'application/json' });
      res.end(JSON.stringify({ ok: true }));
    });
    server.listen(3456, '127.0.0.1', () => {
      console.log('listening');
    });
  `);

  // Start the server inside the VM (background process)
  const proc = vm.spawn("node", ["/tmp/server.js"]);

  // Wait for server to be ready (poll stdout or socket table)
  // ...

  // Fetch from outside
  const response = await vm.fetch(3456, new Request("http://127.0.0.1:3456/"));
  const body = await response.json();
  expect(body.ok).toBe(true);

  proc.kill();
  await vm.dispose();
});
```

### Phase 2 Tests (Sequential, each layer)

#### Layer 1: `pi-headless.test.ts` - PI in headless mode inside VM
```typescript
test("spawn pi in headless mode and get output", async () => {
  const vm = await AgentOs.create();

  // Need mock LLM server running inside the VM
  await vm.writeFile("/tmp/mock-server.js", MOCK_LLM_SERVER_CODE);
  const mockProc = vm.spawn("node", ["/tmp/mock-server.js"]);
  // Wait for mock server ready...

  // Spawn pi with --print flag, API redirected to mock
  const result = await vm.exec(
    "node /path/to/pi/cli.js --print 'say hello'",
    {
      env: {
        ANTHROPIC_API_KEY: "test-key",
        MOCK_LLM_URL: "http://127.0.0.1:<mock-port>",
        NODE_OPTIONS: "-r /path/to/fetch-intercept.cjs",
      },
    }
  );

  expect(result.exitCode).toBe(0);
  expect(result.stdout).toContain("hello");

  mockProc.kill();
  await vm.dispose();
});
```

#### Layer 2: `pi-acp-adapter.test.ts` - Manual pi-acp spawning
```typescript
test("spawn pi-acp and exchange JSON-RPC messages", async () => {
  const vm = await AgentOs.create();

  // Spawn pi-acp inside the VM
  const proc = vm.spawn("npx", ["-y", "pi-acp"], {
    env: { ANTHROPIC_API_KEY: "test-key", /* mock redirect */ },
  });

  // Send initialize
  proc.writeStdin(JSON.stringify({
    jsonrpc: "2.0",
    id: 1,
    method: "initialize",
    params: { protocolVersion: 1, clientCapabilities: {} },
  }) + "\n");

  // Read response from stdout
  // Parse JSON-RPC response with id: 1
  // Verify protocolVersion in result

  // Send session/new
  proc.writeStdin(JSON.stringify({
    jsonrpc: "2.0",
    id: 2,
    method: "session/new",
    params: { cwd: "/home/user", mcpServers: [] },
  }) + "\n");

  // Verify session created

  proc.kill();
  await vm.dispose();
});
```

#### Layer 3: `session.test.ts` - Full createSession API
```typescript
test("createSession spawns pi-acp and manages lifecycle", async () => {
  const vm = await AgentOs.create();
  const session = await vm.createSession("pi", {
    cwd: "/home/user",
    env: { ANTHROPIC_API_KEY: "test-key" },
  });

  const events: any[] = [];
  session.onSessionEvent((event) => events.push(event));

  await session.prompt("say hello");

  // Verify we received session/update notifications
  expect(events.length).toBeGreaterThan(0);

  await session.close();
  await vm.dispose();
});
```

---

## Dependencies

### @rivet-dev/agent-os (single package)

Link paths are relative from the package directory (`packages/core/`) to `~/secure-exec-1/`.

```json
{
  "dependencies": {
    "secure-exec": "link:../../../secure-exec-1/packages/secure-exec",
    "@secure-exec/core": "link:../../../secure-exec-1/packages/core",
    "@secure-exec/nodejs": "link:../../../secure-exec-1/packages/nodejs",
    "@secure-exec/wasmvm": "link:../../../secure-exec-1/packages/wasmvm",
    "@secure-exec/python": "link:../../../secure-exec-1/packages/python",
    "@secure-exec/v8": "link:../../../secure-exec-1/packages/v8",
    "@agentclientprotocol/sdk": "latest"
  }
}
```

---

## Resolved Decisions

| Question | Decision |
|---|---|
| **Package structure** | Single package `@rivet-dev/agent-os` (no separate acp or barrel package) |
| **npm scope** | `@rivet-dev/agent-os-*` |
| **secure-exec linking** | `link:` relative paths from each package dir (e.g., `link:../../../secure-exec-1/packages/core`) |
| **pi-acp in VM** | Mount host `node_modules` read-only; outbound network allowed |
| **`vm.fetch()` impl** | Follow secure-exec pattern: `proc.network.fetch()` / `DefaultNetworkAdapter` loopback checker |
| **Python runtime** | Included by default |
| **Session events** | Raw JSON-RPC envelopes for now; typed events deferred to `notes/todo.md` |
| **Agent types** | Agent-agnostic design. Each agent has a config with `acpAdapter` and `agentPackage`. Start with PI + OpenCode configs, only test PI. |
| **Kernel API gaps** | Modify secure-exec as needed |

---

## Agent Configs

Agent-agnostic session creation. Each agent type has a config:

```typescript
interface AgentConfig {
  acpAdapter: string;    // npm package for ACP adapter (spawned inside VM)
  agentPackage: string;  // npm package for the underlying agent
  env?: Record<string, string>;  // default env vars
}

const AGENT_CONFIGS: Record<string, AgentConfig> = {
  pi: {
    acpAdapter: "pi-acp",
    agentPackage: "@mariozechner/pi-coding-agent",
  },
  opencode: {
    acpAdapter: "opencode",  // opencode speaks ACP natively
    agentPackage: "@opencode-ai/sdk",
  },
};
```

---

## API Token Loading

Tests that need LLM access load tokens from `~/misc/env.txt`. The file format is shell exports:

```
export ANTHROPIC_API_KEY=sk-ant-...
export OPENAI_API_KEY=sk-proj-...
```

**Helper function** (shared test utility):

```typescript
import { readFileSync } from "node:fs";
import { homedir } from "node:os";
import { join } from "node:path";

export function loadEnvTokens(): Record<string, string> {
  const envPath = join(homedir(), "misc", "env.txt");
  const content = readFileSync(envPath, "utf-8");
  const tokens: Record<string, string> = {};
  for (const line of content.split("\n")) {
    const match = line.match(/^export\s+(\w+)=(.+)$/);
    if (match) {
      tokens[match[1]] = match[2];
    }
  }
  return tokens;
}

export function requireAnthropicKey(): string {
  const tokens = loadEnvTokens();
  const key = tokens.ANTHROPIC_API_KEY;
  if (!key) {
    throw new Error(
      "ANTHROPIC_API_KEY not found in ~/misc/env.txt. " +
      "Required for PI agent tests."
    );
  }
  return key;
}
```

**Test usage:**

```typescript
// Tests using mock LLM (no real token needed)
describe("pi-acp protocol", () => {
  // Uses 'test-key' with mock LLM server inside VM
});

// Tests using real LLM (requires token)
describe("pi end-to-end", () => {
  const apiKey = requireAnthropicKey(); // throws if missing

  test("pi responds to prompt", async () => {
    const vm = await AgentOs.create();
    const session = await vm.createSession("pi", {
      env: { ANTHROPIC_API_KEY: apiKey },
    });
    // ...
  });
});
```

---

## Implementation Order

### Step 0: Secure-exec prerequisites (in ~/secure-exec-1)

These changes must land in secure-exec before agentOS Phase 2 can work. Phase 1 can proceed in parallel after Step 0a.

#### Step 0a: Streaming stdin for Node runtime (BLOCKING for Phase 2)

The Node runtime driver (`packages/nodejs/src/kernel-runtime.ts`) currently buffers ALL stdin into `stdinChunks[]` and delivers it as a single string when `closeStdin()` is called. ACP requires true bidirectional stdio: send `initialize`, read response, send `session/new`, read response, etc.

- [ ] Change the Node runtime driver to support streaming/piped stdin where each `writeStdin()` call delivers data to the running process immediately
- [ ] This likely requires changes to the V8 bridge IPC model (stdin must be an async iterator or pipe, not a one-shot string)
- [ ] Add tests: spawn a process, write line, read response, write another line, read another response
- [ ] Existing tests must still pass (batch stdin via writeStdin + closeStdin should still work)

#### Step 0b: Private-IP check in HostNetworkAdapter.tcpConnect()

`HostNetworkAdapter.tcpConnect()` (`packages/nodejs/src/host-network-adapter.ts`) performs raw `net.connect({host, port})` with zero IP validation. VM code can reach `127.0.0.1`, `169.254.169.254` (cloud metadata), internal networks.

- [ ] Add `isPrivateIp()` validation in `tcpConnect()` (at minimum block `169.254.169.254`)
- [ ] Make it configurable (some uses may need private access)
- [ ] Mirror the approach from `DefaultNetworkAdapter.assertNotPrivateHost()`

#### Step 0c: Quota option on InMemoryFileSystem

`InMemoryFileSystem` (`packages/core/src/shared/in-memory-fs.ts`) stores files in a `Map<number, Uint8Array>` with no size limit. A runaway agent can OOM the host.

- [ ] Add optional `quotaBytes` to constructor
- [ ] Track total bytes on write, reject with `ENOSPC` when exceeded
- [ ] Default to unlimited (backwards compatible)

### Step 1: Scaffold agentOS
- [ ] Root package.json, pnpm-workspace.yaml, turbo.json, biome.json, tsconfig
- [ ] packages/core scaffold (package.json, tsconfig, vitest.config, src/index.ts)
- [ ] Verify `pnpm install` and `pnpm build` work
- [ ] CLAUDE.md with project constraints

### Step 2: Phase 1 - Core API
- [ ] `AgentOs.create()` async factory (not constructor -- `kernel.mount()` is async)
- [ ] Proxy exec/spawn from kernel
- [ ] Proxy filesystem methods from kernel
- [ ] `fetch(port, request)` -- host-side `globalThis.fetch()` to `127.0.0.1:{port}` (kernel binds real host ports via `tcpListen`; optionally verify port is in `kernel.socketTable` first)
- [ ] openShell proxy
- [ ] dispose (close active sessions first, then kernel)

### Step 3: Phase 1 - Tests
- [ ] execute.test.ts (basic exec, node, shell pipelines, env, cwd, stdin)
- [ ] filesystem.test.ts (read/write/mkdir/readdir/stat/exists)
- [ ] network.test.ts (spawn server inside VM via `spawn()` not `exec()`, use `kernel.socketTable.findListener()` or stdout callback for readiness, then fetch from outside)

### Step 4: Phase 2 - ACP Client (requires Step 0a)
- [ ] JSON-RPC protocol helpers (serialize, deserialize, correlate)
- [ ] AcpClient class -- wired via `SpawnOptions.onStdout` callback (not `ManagedProcess` constructor), buffers lines, parses JSON-RPC envelopes, skips non-JSON lines
- [ ] Request/response correlation with timeout
- [ ] Notification subscription
- [ ] Handle process exit mid-request (reject pending promises immediately, don't wait for 120s timeout)

### Step 5: Phase 2 - Session Management
- [ ] createSession() -- spawn agent via direct path from mounted node_modules (not `npx -y`), pass `onStdout` to wire ACP client at spawn time
- [ ] session.prompt() -- define semantics: resolves when agent sends final response (need terminal event), events stream via onSessionEvent
- [ ] session.onSessionEvent() - notification forwarding
- [ ] session.cancel() / session.close()
- [ ] AgentOs tracks active sessions; dispose() closes them with timeout before killing kernel

### Step 6: Phase 2 - Tests (sequential layers)
- [ ] Layer 1: PI headless inside VM (mock LLM, verify output)
- [ ] Layer 2: pi-acp manual spawn (JSON-RPC exchange over stdio)
- [ ] Layer 3: Full createSession API (end-to-end with mock)
