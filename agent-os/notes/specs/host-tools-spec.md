# Host Tools Spec

Expose host-side tools to agents running inside agentOS VMs. Tools are defined by the agentOS consumer using the Vercel AI SDK `tool()` format, grouped into toolkits, and auto-generated into per-toolkit CLI binaries inside the VM that agents invoke like any other command.

## Design Principles

1. **CLI-first.** Agents interact with tools by running shell commands, not by speaking a protocol. Every coding agent can run shell commands.
2. **Auto-generated CLIs from schemas.** Toolkit authors define tools with zod schemas. agentOS generates the CLI, `--help`, prompt docs, and RPC plumbing. Zero boilerplate.
3. **Toolkits are npm packages.** A toolkit is a named group of tools exported from a package. `npm install @rivet-dev/agentos-browser` and pass it to `AgentOs.create()`.
4. **Host execution, VM invocation.** Tool `execute()` functions run on the host (not inside the VM). The CLI inside the VM is a thin shim that calls the host via localhost RPC.

## Architecture

```
Host                                          VM
─────────────────────────────────────────────────────────────
                                              Agent process
                                                │
                                                ▼
                                              agentos-browser screenshot --full-page
                                                │
                                                ▼
                                              /usr/local/bin/agentos-browser (shell shim)
                                                │  curl POST http://127.0.0.1:$PORT
                                                │  body: {"toolkit":"browser","tool":"screenshot","input":{"fullPage":true}}
                                                ▼
AgentOs RPC server (started at VM boot) ◄────── loopback
  │
  ▼
toolKit.tools.screenshot.execute({ fullPage: true })
  │
  ▼
JSON result → HTTP response → shim prints to stdout
```

## Consumer API

### Defining tools

```typescript
import { z } from "zod";
import { hostTool, toolKit } from "@rivet-dev/agent-os";

const browser = toolKit({
  name: "browser",
  description: "Browser automation for testing and verification",
  tools: {
    open: hostTool({
      description: "Navigate to a URL",
      inputSchema: z.object({
        url: z.string().describe("URL to navigate to"),
      }),
      execute: async ({ url }) => {
        return { status: "navigated", title: "My App" };
      },
    }),
    screenshot: hostTool({
      description: "Take a screenshot of the current page",
      inputSchema: z.object({
        path: z.string().optional().describe("Output file path"),
        fullPage: z.boolean().optional().describe("Capture full page"),
      }),
      execute: async ({ path, fullPage }) => {
        return { path: path ?? "/tmp/screenshot.png" };
      },
    }),
    click: hostTool({
      description: "Click an element by CSS selector",
      inputSchema: z.object({
        selector: z.string().describe("CSS selector"),
      }),
      execute: async ({ selector }) => {
        return { clicked: selector };
      },
    }),
  },
});
```

### Passing toolkits to VM / session

```typescript
import { browser } from "@rivet-dev/agentos-browser";
import { searchToolKit } from "./my-search-tools";

const vm = await AgentOs.create({
  toolKits: [browser, searchToolKit],
});

// Session-level toolkits merge with VM-level (session wins on collision)
const session = await vm.createSession("pi", {
  toolKits: [sessionSpecificToolKit],
});
```

### ToolKit as npm package

A toolkit package exports a `ToolKit` object:

```typescript
// @rivet-dev/agentos-browser/src/index.ts
import { toolKit, hostTool } from "@rivet-dev/agent-os";
import { z } from "zod";

export const browser = toolKit({
  name: "browser",
  description: "Browser automation for testing and verification",
  tools: { /* ... */ },
});
```

## Types

```typescript
import type { ZodType } from "zod";

/**
 * A single tool that executes on the host.
 * Mirrors the shape of AI SDK's tool() but with host-execution semantics.
 */
export interface HostTool<INPUT = any, OUTPUT = any> {
  /** Description shown to the agent in --help and prompt docs. */
  description: string;
  /** Zod schema for the input. Drives CLI flag generation and validation. */
  inputSchema: ZodType<INPUT>;
  /** Runs on the host when the agent invokes the tool. */
  execute: (input: INPUT) => Promise<OUTPUT> | OUTPUT;
  /** Examples included in auto-generated prompt docs. */
  examples?: ToolExample<INPUT>[];
  /** Timeout in ms. Default: 30000. */
  timeout?: number;
}

export interface ToolExample<INPUT = any> {
  /** Human description of what this example does. */
  description: string;
  /** The input args for the example. */
  input: INPUT;
}

/**
 * A named group of tools. Becomes a CLI binary: agentos-{name}.
 */
export interface ToolKit {
  /** Toolkit name. Must be lowercase alphanumeric + hyphens. Becomes the CLI suffix: agentos-{name}. */
  name: string;
  /** Description shown in `agentos list-tools` and prompt docs. */
  description: string;
  /** The tools in this toolkit. Keys become subcommands. */
  tools: Record<string, HostTool>;
}

/** Helper to create a HostTool with type inference. */
export function hostTool<INPUT, OUTPUT>(def: HostTool<INPUT, OUTPUT>): HostTool<INPUT, OUTPUT>;

/** Helper to create a ToolKit. */
export function toolKit(def: ToolKit): ToolKit;
```

## CLI Generation

At VM boot (or session start), agentOS generates a shell shim for each toolkit and writes it to `/usr/local/bin/`:

| Toolkit name | Binary path | Example invocation |
|---|---|---|
| `browser` | `/usr/local/bin/agentos-browser` | `agentos-browser screenshot --full-page` |
| `search` | `/usr/local/bin/agentos-search` | `agentos-search docs --query "auth"` |

A master `agentos` binary is always written to `/usr/local/bin/agentos`.

### CLI binary: `agentos-{name}`

Each generated binary supports:

```
agentos-{name} <tool> [flags]       Call a tool with CLI flags
agentos-{name} <tool> --json '{}'   Call a tool with raw JSON string
agentos-{name} <tool> --json-file path   Call a tool with JSON from file
agentos-{name} <tool> --help        Show tool usage and flags
agentos-{name} --help               List all tools in this toolkit
```

### CLI binary: `agentos`

The master binary has one subcommand:

```
agentos list-tools                  List all toolkits and their tools
agentos list-tools <toolkit>        List tools in a specific toolkit
```

`agentos list-tools` output:

```
browser — Browser automation for testing and verification
  open           Navigate to a URL
  screenshot     Take a screenshot of the current page
  click          Click an element by CSS selector

search — Documentation and code search
  docs           Search the documentation index
  code           Search code by pattern
```

`agentos list-tools browser` output:

```
browser — Browser automation for testing and verification

  open           Navigate to a URL
    --url <string>              URL to navigate to

  screenshot     Take a screenshot of the current page
    --path <string>             Output file path
    --full-page                 Capture full page

  click          Click an element by CSS selector
    --selector <string>         CSS selector
```

### Flag generation from zod schema

| Zod type | CLI flag | Example |
|---|---|---|
| `z.string()` | `--name <string>` | `--query "auth"` |
| `z.number()` | `--name <number>` | `--limit 5` |
| `z.boolean()` | `--name` (presence = true) | `--full-page` |
| `z.boolean().optional()` | `--name / --no-name` | `--full-page` / `--no-full-page` |
| `z.enum(["a","b"])` | `--name <a\|b>` | `--format json` |
| `z.array(z.string())` | `--name <val> --name <val>` | `--tags foo --tags bar` |
| `z.string().optional()` | `--name <string>` (omitted = undefined) | |
| Nested object | Not supported as flags; use `--json` | `--json '{"nested":{"key":"val"}}'` |

Flag names are auto-converted from camelCase to kebab-case: `fullPage` → `--full-page`.

The `.describe()` string on each zod field becomes the flag's help text.

### Input modes

Three ways to pass input, in priority order:

1. **Flags** (default): `agentos-browser screenshot --full-page --path /tmp/shot.png`
2. **Raw JSON string**: `agentos-browser screenshot --json '{"fullPage":true,"path":"/tmp/shot.png"}'`
3. **JSON file**: `agentos-browser screenshot --json-file /tmp/input.json`

`--json` and `--json-file` are mutually exclusive and override all other flags. When using flags, the shim assembles them into a JSON object before sending to the RPC server.

Stdin pipe is also supported: `echo '{"fullPage":true}' | agentos-browser screenshot`

## RPC Server

agentOS starts an HTTP server on `127.0.0.1` (random port) at VM boot. The port is stored in an env var (`AGENTOS_TOOLS_PORT`) available to all processes in the VM. The CLI shims read this env var.

### Protocol

**Request:**

```
POST http://127.0.0.1:$AGENTOS_TOOLS_PORT/call
Content-Type: application/json

{
  "toolkit": "browser",
  "tool": "screenshot",
  "input": { "fullPage": true, "path": "/tmp/shot.png" }
}
```

**Success response:**

```
HTTP 200
Content-Type: application/json

{
  "ok": true,
  "result": { "path": "/tmp/shot.png" }
}
```

**Error response:**

```
HTTP 200
Content-Type: application/json

{
  "ok": false,
  "error": "VALIDATION_ERROR",
  "message": "Expected string at \"path\", received number"
}
```

All responses are HTTP 200 with JSON body. The `ok` field determines success/failure. This keeps the CLI shim simple — it always prints the response body to stdout.

### Endpoints

| Method | Path | Purpose |
|---|---|---|
| `POST` | `/call` | Execute a tool |
| `GET` | `/list` | List all toolkits and tools |
| `GET` | `/list/:toolkit` | List tools in a toolkit |
| `GET` | `/describe/:toolkit/:tool` | Get tool schema (for `--help`) |

## Error Handling

### Error codes

| Code | HTTP status (internal) | When | Agent sees |
|---|---|---|---|
| `TOOL_NOT_FOUND` | 200 | Bad toolkit or tool name | `{"ok":false,"error":"TOOL_NOT_FOUND","message":"No tool \"oepn\" in toolkit \"browser\". Available: open, screenshot, click"}` |
| `TOOLKIT_NOT_FOUND` | 200 | Bad toolkit name | `{"ok":false,"error":"TOOLKIT_NOT_FOUND","message":"No toolkit \"brawser\". Available: browser, search"}` |
| `VALIDATION_ERROR` | 200 | Input doesn't match schema | `{"ok":false,"error":"VALIDATION_ERROR","message":"Required field \"selector\" is missing"}` |
| `EXECUTION_ERROR` | 200 | Tool's execute() threw | `{"ok":false,"error":"EXECUTION_ERROR","message":"Connection refused: playwright:9222"}` |
| `TIMEOUT` | 200 | Tool exceeded timeout | `{"ok":false,"error":"TIMEOUT","message":"Tool \"screenshot\" timed out after 30000ms"}` |
| `INTERNAL_ERROR` | 200 | RPC transport failure | `{"ok":false,"error":"INTERNAL_ERROR","message":"Host RPC server unreachable"}` |

### Design decisions

- **All errors go to stdout as JSON.** Agents parse stdout. Stderr is reserved for debug logging from the shim itself (e.g., connection failures during development).
- **All HTTP responses are 200.** The `ok` field is the discriminator. This avoids the shim needing to interpret HTTP status codes — it always reads the body and prints it.
- **Error messages are human-readable and actionable.** They include what went wrong AND what the valid options are (available tools, expected types). This lets the agent self-correct.
- **`TOOL_NOT_FOUND` includes available tools.** Agents frequently misspell or hallucinate tool names. Listing alternatives helps the agent recover in one retry.
- **`VALIDATION_ERROR` includes the zod error.** The zod error message specifies which field failed and why. This maps directly to what the agent needs to fix.
- **No stack traces in error messages.** `EXECUTION_ERROR` includes `err.message`, not the full stack. Stack traces waste agent tokens and aren't actionable.

### CLI exit codes

| Exit code | Meaning |
|---|---|
| 0 | Tool executed (check `ok` field for success/failure) |
| 1 | CLI shim error (couldn't reach RPC server, malformed flags) |

Exit code 0 for tool-level errors (including `EXECUTION_ERROR`) because the tool infrastructure worked correctly — the tool itself failed. The agent reads the JSON output regardless. Exit code 1 is only for infrastructure failures where the shim can't produce valid JSON output.

## CLI Shim Implementation

The shim is a POSIX shell script. No Node.js startup overhead. Uses `curl` (available as a WASM command in the VM) to call the RPC server.

```sh
#!/bin/sh
# Auto-generated by agentOS. Do not edit.
# Toolkit: browser

PORT="$AGENTOS_TOOLS_PORT"
TOOLKIT="browser"

if [ -z "$PORT" ]; then
  echo '{"ok":false,"error":"INTERNAL_ERROR","message":"AGENTOS_TOOLS_PORT not set. Host tools not available."}' >&1
  exit 1
fi

# Subcommand is first arg
TOOL="$1"
shift 2>/dev/null

if [ -z "$TOOL" ] || [ "$TOOL" = "--help" ] || [ "$TOOL" = "-h" ]; then
  curl -s "http://127.0.0.1:$PORT/describe/$TOOLKIT" 2>/dev/null || \
    echo '{"ok":false,"error":"INTERNAL_ERROR","message":"Could not reach host tools server"}'
  exit 0
fi

if [ "$1" = "--help" ] || [ "$1" = "-h" ]; then
  curl -s "http://127.0.0.1:$PORT/describe/$TOOLKIT/$TOOL" 2>/dev/null || \
    echo '{"ok":false,"error":"INTERNAL_ERROR","message":"Could not reach host tools server"}'
  exit 0
fi

# Build input JSON from flags, --json, --json-file, or stdin
INPUT=""
if [ "$1" = "--json" ]; then
  INPUT="$2"
elif [ "$1" = "--json-file" ]; then
  INPUT=$(cat "$2")
elif [ ! -t 0 ]; then
  INPUT=$(cat)
else
  # Parse flags into JSON — delegated to host /parse-flags endpoint
  # Ship raw argv, let the host parse against the zod schema
  ARGV=$(printf '%s\n' "$@" | awk '
    BEGIN { printf "[" }
    NR>1 { printf "," }
    { gsub(/"/, "\\\""); printf "\"%s\"", $0 }
    END { printf "]" }
  ')
  BODY="{\"toolkit\":\"$TOOLKIT\",\"tool\":\"$TOOL\",\"argv\":$ARGV}"
  curl -s -X POST "http://127.0.0.1:$PORT/call" \
    -H "Content-Type: application/json" \
    -d "$BODY" 2>/dev/null || \
    echo '{"ok":false,"error":"INTERNAL_ERROR","message":"Could not reach host tools server"}'
  exit $?
fi

# --json or --json-file or stdin: send parsed input directly
BODY="{\"toolkit\":\"$TOOLKIT\",\"tool\":\"$TOOL\",\"input\":$INPUT}"
curl -s -X POST "http://127.0.0.1:$PORT/call" \
  -H "Content-Type: application/json" \
  -d "$BODY" 2>/dev/null || \
  echo '{"ok":false,"error":"INTERNAL_ERROR","message":"Could not reach host tools server"}'
```

### Flag parsing strategy

Flags are parsed on the **host side**, not in the shell shim. The shim sends raw argv to the RPC server. The host has the zod schema and does proper parsing + validation in one step. This keeps the shim trivial and avoids reimplementing zod-to-flag logic in shell.

When `argv` is present in the RPC request (instead of `input`), the host:
1. Looks up the tool's zod schema
2. Parses `argv` into a JSON object using the schema as a guide (knows types, knows kebab→camelCase mapping)
3. Validates with zod
4. Calls `execute()` or returns `VALIDATION_ERROR`

## Prompt Injection

Tool documentation is injected into the agent's system prompt via `prepareInstructions`. The docs are auto-generated from toolkit and tool definitions.

### Generated prompt section

```markdown
## Host Tools

Host tools run on the host machine outside the VM. Call them from the shell.

### agentos-browser — Browser automation for testing and verification

```
agentos-browser open --url <string>
  Navigate to a URL

agentos-browser screenshot [--path <string>] [--full-page]
  Take a screenshot of the current page

agentos-browser click --selector <string>
  Click an element by CSS selector
```

### agentos-search — Documentation and code search

```
agentos-search docs --query <string> [--limit <number>]
  Search the documentation index
```

Run `agentos list-tools` to see all available tools.
Run `agentos-<toolkit> <tool> --help` for detailed usage.
```

### Examples in prompt

If tools define `examples`, they are included:

```markdown
#### Examples

```bash
# Take a full-page screenshot
agentos-browser screenshot --full-page

# Search for auth docs
agentos-search docs --query "authentication" --limit 5
```
```

### Prompt size management

With many toolkits, prompt injection can get large. Mitigations:
- Only inject toolkits registered for the current session (not all VM-level toolkits if session specifies its own subset)
- Keep tool descriptions concise — one line per tool in the prompt summary
- Agents can always run `--help` for full details; the prompt is a quick reference, not exhaustive docs

## VM Integration

### Boot sequence (when toolKits are provided)

1. `AgentOs.create()` receives `toolKits` option
2. Start RPC HTTP server on host, bind to `127.0.0.1:0` (random port), exempt port from SSRF checks via `loopbackExemptPorts`
3. Store port as `AGENTOS_TOOLS_PORT` in kernel env
4. For each toolkit, write CLI shim to `/usr/local/bin/agentos-{name}` (executable)
5. Write master `agentos` shim to `/usr/local/bin/agentos`
6. Store tool definitions on the `AgentOs` instance for runtime dispatch

### Session-level toolkits

When `createSession()` receives `toolKits`:
1. Register additional tools on the existing RPC server
2. Write additional CLI shims (if not already present)
3. Include session-level toolkit docs in `prepareInstructions`

Session-level toolkits are scoped to that session's process tree (they share the RPC server but the session's `prepareInstructions` only documents what that session has access to).

### Cleanup

When the `AgentOs` instance is shut down, the RPC server is closed. No cleanup needed for the CLI shims (they're in the VM's in-memory filesystem).

## Interaction with Existing Features

### MCP servers

`mcpServers` in `CreateSessionOptions` and `toolKits` are independent. MCP servers are passed through to the ACP adapter for the agent to connect to natively. Host tools use the CLI shim + RPC pattern. They don't interfere.

An MCP-to-toolkit bridge (exposing MCP tools as host tools or vice versa) is out of scope.

### OS instructions

Host tool docs are appended to the OS instructions in `prepareInstructions`. They appear after the base `/etc/agentos/instructions.md` content and any `additionalInstructions`.

### `skipOsInstructions`

If `skipOsInstructions: true`, tool prompt docs are still injected. The opt-out is for OS-level instructions, not tool documentation. The tools are useless without docs.

## Testing

| Feature | Test approach |
|---|---|
| `toolKit()` / `hostTool()` | Unit test: define toolkit, verify types and structure |
| CLI shim generation | Boot VM with toolkit, verify `/usr/local/bin/agentos-{name}` exists and is executable |
| RPC server | Boot VM with toolkit, call `curl` from inside VM to RPC endpoint, verify response |
| Flag parsing | Call tool via CLI flags from inside VM, verify `execute()` receives correct parsed input |
| `--json` mode | Call tool with `--json '{...}'`, verify same result as flags |
| `--json-file` mode | Write JSON file in VM, call tool with `--json-file`, verify result |
| stdin pipe | Pipe JSON to tool command, verify result |
| `--help` | Run `agentos-{name} --help`, verify lists tools. Run `agentos-{name} tool --help`, verify shows flags. |
| `agentos list-tools` | Boot VM with multiple toolkits, verify output lists all |
| Error: bad tool | Call nonexistent tool, verify `TOOL_NOT_FOUND` with available tools in message |
| Error: bad input | Call tool with wrong types, verify `VALIDATION_ERROR` with zod message |
| Error: execute throws | Tool that throws, verify `EXECUTION_ERROR` with error message |
| Error: timeout | Tool that never resolves, verify `TIMEOUT` after configured ms |
| Error: no RPC server | Unset `AGENTOS_TOOLS_PORT`, run shim, verify `INTERNAL_ERROR` |
| Session-level toolkits | Create session with additional toolkit, verify it's accessible and prompt docs include it |
| Prompt injection | Create session with toolkits, inspect the instructions passed to agent, verify tool docs present |
