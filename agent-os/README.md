# agentOS

A high-level wrapper around [Secure-Exec](https://github.com/nichochar/secure-exec) that provides a clean API for running coding agents inside isolated VMs via the [Agent Communication Protocol](https://agentclientprotocol.com) (ACP).

## Features

- **Filesystem & process management** — read/write files, exec commands, spawn long-running processes
- **Agent sessions via ACP** — spawn coding agents (PI, OpenCode, etc.) inside the VM and communicate over JSON-RPC stdio
- **Module mounts** — project host `node_modules` into the VM read-only so agents have access to their dependencies
- **Host tools** — define toolkits on the host that agents can call via CLI commands inside the VM

## How it works

agentOS builds on **Secure-Exec**, an in-process operating system kernel written in JavaScript. The kernel manages a layered virtual filesystem (in-memory storage, `/dev` devices, `/proc` pseudo-files, permission checks), a POSIX process table with cross-runtime process trees, pipes, PTYs, and a virtual network stack with in-kernel loopback and host-delegated external connections.

Three runtimes mount into the kernel:

- **WASM** — A custom libc and Rust toolchain compile POSIX utilities (coreutils, sh, grep, etc.) to WebAssembly. Processes run in Worker threads with synchronous syscalls via SharedArrayBuffer.
- **Node.js** — A sandboxed reimplementation of Node.js APIs (`child_process`, `fs`, `net`) runs JS/TS inside isolated V8 contexts. Module loading routes through the kernel VFS.
- **Python** — CPython via Pyodide, running in a Worker thread with kernel-backed I/O.

Everything — agent processes, servers, file I/O, network requests — runs inside the kernel. Nothing executes on the host.

agentOS wraps this into a higher-level API that adds:

- **Filesystem & process management** — read/write files, exec commands, spawn long-running processes
- **Agent sessions via ACP** — spawn coding agents (PI, OpenCode, etc.) inside the VM and communicate over JSON-RPC stdio
- **Module mounts** — project host `node_modules` into the VM read-only so agents have access to their dependencies
- **Host tools** — define tools on the host that agents invoke via auto-generated CLI commands inside the VM

## Quick start

```ts
import { AgentOs } from "@rivet-dev/agent-os-core";

// Boot a VM
const vm = await AgentOs.create();

// Run commands
await vm.exec("echo hello");

// Work with the filesystem
await vm.writeFile("/home/user/test.txt", "hello world");
const content = await vm.readFile("/home/user/test.txt");

// Spawn an ACP coding agent session
const session = await vm.createSession("pi", {
  env: { ANTHROPIC_API_KEY: "..." },
});
const response = await session.prompt("Write a hello world script");

await vm.dispose();
```

## Host Tools

Host tools let you define functions on the host that agents running inside the VM can call via CLI commands. Each tool belongs to a **toolkit** (a named group), and each toolkit becomes a CLI binary (`agentos-{name}`) available in the VM's `$PATH`.

### Defining tools and toolkits

Use `hostTool()` and `toolKit()` to define tools with full type inference from Zod schemas:

```ts
import { z } from "zod";
import { AgentOs, hostTool, toolKit } from "@rivet-dev/agent-os-core";

const fileTools = toolKit({
  name: "files",
  description: "Host filesystem operations",
  tools: {
    read: hostTool({
      description: "Read a file from the host filesystem",
      inputSchema: z.object({
        path: z.string().describe("Absolute path to the file"),
      }),
      execute: async ({ path }) => {
        const content = await fs.readFile(path, "utf-8");
        return { content };
      },
      examples: [
        { description: "Read a config file", input: { path: "/etc/config.json" } },
      ],
      timeout: 10000, // ms, default: 30000
    }),
    search: hostTool({
      description: "Search files by pattern",
      inputSchema: z.object({
        pattern: z.string().describe("Glob pattern"),
        directory: z.string().describe("Directory to search in"),
        recursive: z.boolean().optional().describe("Search subdirectories"),
      }),
      execute: async ({ pattern, directory, recursive }) => {
        // ... search implementation
        return { matches: [] };
      },
    }),
  },
});
```

### Passing toolkits to AgentOs.create()

Toolkits are registered at VM creation and available to all sessions:

```ts
const vm = await AgentOs.create({
  toolKits: [fileTools],
});
```

### What the agent runs

Inside the VM, each toolkit is available as a CLI command: `agentos-{name} <tool> [flags]`.

```bash
# Call the "read" tool in the "files" toolkit
agentos-files read --path /etc/config.json

# Call "search" with multiple flags
agentos-files search --pattern "*.ts" --directory /src --recursive

# List all available toolkits
agentos list-tools

# List tools in a specific toolkit
agentos list-tools files

# Get help for a toolkit or tool
agentos-files --help
agentos-files read --help
```

### Input modes

Tools can receive input in four ways:

1. **CLI flags** — parsed against the Zod schema. Field names are converted to kebab-case (`--my-field value`).
   ```bash
   agentos-files read --path /etc/config.json
   ```

2. **Inline JSON** — pass a JSON string directly.
   ```bash
   agentos-files read --json '{"path": "/etc/config.json"}'
   ```

3. **JSON file** — read input from a file.
   ```bash
   agentos-files read --json-file /tmp/input.json
   ```

4. **stdin** — pipe JSON via standard input (auto-detected when not a TTY).
   ```bash
   echo '{"path": "/etc/config.json"}' | agentos-files read
   ```

### Flag types

The CLI flag parser supports the following Zod types:

| Zod type | Flag syntax | Example |
|---|---|---|
| `z.string()` | `--name value` | `--path /etc/config.json` |
| `z.number()` | `--name 42` | `--count 5` |
| `z.boolean()` | `--flag` / `--no-flag` | `--recursive` / `--no-recursive` |
| `z.enum()` | `--name value` | `--format json` |
| `z.array(z.string())` | `--name a --name b` | `--tags foo --tags bar` |

Optional fields (`.optional()`) become optional flags.

### CLI shim pattern

agentOS generates POSIX shell scripts that are mounted read-only at `/usr/local/bin/` inside the VM. These shims communicate with a host-side HTTP RPC server via `http-test` (a WASM binary available in the VM):

- Each toolkit gets a shim: `/usr/local/bin/agentos-{name}`
- A master shim `/usr/local/bin/agentos` provides `list-tools`
- The RPC server port is passed via the `AGENTOS_TOOLS_PORT` environment variable
- Shims parse input and forward it to the host for execution
- Tool responses are JSON: `{ ok: true, result: ... }` or `{ ok: false, error: "...", message: "..." }`

### Error codes

When a tool invocation fails, the response includes an error code:

| Error code | Description |
|---|---|
| `TOOLKIT_NOT_FOUND` | The specified toolkit name does not exist |
| `TOOL_NOT_FOUND` | The specified tool does not exist in the toolkit |
| `VALIDATION_ERROR` | Input failed Zod schema validation or JSON parsing |
| `EXECUTION_ERROR` | The tool's `execute` function threw an error |
| `TIMEOUT` | The tool did not complete within its timeout (default 30s) |
| `INTERNAL_ERROR` | Server unreachable or unknown endpoint |

## API Reference

### Core

#### `AgentOs.create(options?)`

Create a new VM instance.

- **`options.toolKits`** — `ToolKit[]` — Toolkits available to all sessions in this VM.
- **`options.moduleAccessCwd`** — `string` — Host directory with `node_modules/` to mount read-only.
- **`options.permissions`** — Kernel permission configuration.
- **`options.loopbackExemptPorts`** — `number[]` — Ports exempt from SSRF checks.

#### `vm.createSession(agent, options?)`

Create an ACP agent session.

- **`options.env`** — `Record<string, string>` — Environment variables for the agent process.

### Host Tools Types

#### `HostTool<INPUT, OUTPUT>`

```ts
interface HostTool<INPUT = any, OUTPUT = any> {
  description: string;
  inputSchema: ZodType<INPUT>;
  execute: (input: INPUT) => Promise<OUTPUT> | OUTPUT;
  examples?: ToolExample<INPUT>[];
  timeout?: number; // ms, default: 30000
}
```

#### `ToolKit`

```ts
interface ToolKit {
  name: string;        // lowercase alphanumeric + hyphens
  description: string;
  tools: Record<string, HostTool>;
}
```

#### `ToolExample<INPUT>`

```ts
interface ToolExample<INPUT = any> {
  description: string;
  input: INPUT;
}
```

### Host Tools Helpers

#### `hostTool(def)`

Creates a `HostTool` with type inference from the Zod schema:

```ts
const myTool = hostTool({
  description: "...",
  inputSchema: z.object({ name: z.string() }),
  execute: ({ name }) => ({ greeting: `Hello ${name}` }),
});
```

#### `toolKit(def)`

Creates a `ToolKit`:

```ts
const myToolkit = toolKit({
  name: "greetings",
  description: "Greeting tools",
  tools: { hello: myTool },
});
```

## Documentation

Full API reference and guides: [rivet.dev/docs/agent-os](https://rivet.dev/docs/agent-os/)

## Development

```bash
pnpm install
pnpm build        # turbo run build
pnpm test         # turbo run test
pnpm check-types  # turbo run check-types
pnpm lint         # biome check
```

## License

Apache-2.0
