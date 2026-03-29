# Proposal: Auto-Loading OS System Instructions into Agent Sessions

## Problem

When an agent session starts inside a VM, the agent has no context about its environment — it doesn't know it's running inside a virtualized OS, what tools are available, what the filesystem layout looks like, or what constraints exist (e.g., no native binaries, no internet unless configured). We want to inject a default set of OS-level instructions that every agent will pick up automatically, without clobbering any user-provided project instructions.

## Approach: Mount `/etc/agentos/` with instructions

Mount a read-only virtual directory at `/etc/agentos/` inside the VM that contains the system prompt and any other OS-level configuration. This follows standard Unix semantics — `/etc/` is the FHS-designated location for system configuration files, and `/etc/<package>/` is the convention for package-specific config.

### Why `/etc/agentos/`

| Directory | FHS Purpose | Fit |
|-----------|------------|-----|
| `/etc/` | Host-specific system configuration | **Correct.** OS instructions are system config the agent reads at startup. |
| `/usr/share/` | Architecture-independent read-only data | Close, but `/etc/` is more standard for config that varies per-instance. |
| `/var/lib/` | Variable state data | Wrong — instructions are static, not stateful. |
| `/opt/` | Add-on application packages | Wrong — this is the OS itself, not an add-on. |

### Filesystem layout

```
/etc/agentos/
├── instructions.md          # OS-level system prompt
├── environment.json         # Runtime metadata (available runtimes, constraints, etc.)
└── mounts.json              # Active mount points (auto-generated from mount table)
```

Only `instructions.md` is required. The other files are optional and generated if relevant metadata is available.

### How it works

1. **At VM boot**, the kernel (or agentOS) mounts a read-only in-memory filesystem at `/etc/agentos/`
2. `instructions.md` is written with the OS-level prompt content before the agent session starts
3. Per-agent injection tells each agent where to find the instructions (or reads and passes the content)

### Per-agent strategy

| Agent | Mechanism | How |
|-------|-----------|-----|
| **PI** | CLI flag | Read `/etc/agentos/instructions.md`, pass content via `--append-system-prompt` |
| **Claude Code** | CLI flag | Read `/etc/agentos/instructions.md`, pass content via `--append-system-prompt` |
| **OpenCode** | Context paths | Point `OPENCODE_CONTEXTPATHS` at `/etc/agentos/instructions.md` |
| **Codex** | CLI flag | Read `/etc/agentos/instructions.md`, pass content via `-c developer_instructions` |

### Why this split still exists

Even with a canonical filesystem location, agents have different instruction-loading mechanisms. The key improvement is:

- **Single source of truth** — instructions live in the filesystem, not constructed per-agent
- **No writes to cwd** — eliminates the OpenCode `.agent-os/` hack from the previous proposal
- **Inspectable** — agents (or users) can `cat /etc/agentos/instructions.md` to see what was injected
- **Extensible** — additional config files can be added without changing per-agent injection logic
- **OS-native** — follows Unix conventions; the OS provides config through the filesystem

### Implementation detail per agent

#### PI and Claude Code (`--append-system-prompt`)

When spawning, read the mounted file and pass as a CLI argument:

```typescript
// In prepareInstructions():
const content = await kernel.readFile("/etc/agentos/instructions.md");
return { args: ["--append-system-prompt", new TextDecoder().decode(content)] };
```

User's `AGENTS.md`/`CLAUDE.md` at cwd still loads normally via the agent's directory walk.

#### Codex (`-c developer_instructions`)

```typescript
// In prepareInstructions():
const content = await kernel.readFile("/etc/agentos/instructions.md");
return { args: ["-c", `developer_instructions=${new TextDecoder().decode(content)}`] };
```

#### OpenCode (`OPENCODE_CONTEXTPATHS`)

OpenCode supports absolute paths in context paths. Point directly at the mounted file:

```typescript
// In prepareInstructions():
const contextPaths = [
  // Default OpenCode context paths
  ".github/copilot-instructions.md",
  ".cursorrules",
  ".cursor/rules/",
  "CLAUDE.md",
  "CLAUDE.local.md",
  "opencode.md",
  "opencode.local.md",
  "/etc/agentos/instructions.md",  // mounted OS instructions
];
return { env: { OPENCODE_CONTEXTPATHS: JSON.stringify(contextPaths) } };
```

**No file writes to cwd.** The file already exists at a well-known path in the VM filesystem.

### Mount setup

In the kernel constructor (or agentOS.create), mount the config directory:

```typescript
// Create the instructions content
const instructions = getOsInstructions(options?.additionalInstructions);

// Write to the root filesystem at /etc/agentos/
await kernel.mkdir("/etc/agentos");
await kernel.writeFile("/etc/agentos/instructions.md", instructions);
```

With the mount table (from the mount-table-spec), this becomes a proper read-only mount:

```typescript
// Future: mount as read-only backend
const configFs = createInMemoryFileSystem();
configFs.writeFile("/instructions.md", new TextEncoder().encode(instructions));

kernel.mountFs("/etc/agentos", configFs, { readOnly: true });
```

The read-only mount prevents agents from tampering with their own OS instructions.

### Shared instructions content

All agents receive the same OS-level context. A single `os-instructions.ts` module provides the content:

```typescript
export function getOsInstructions(additional?: string): string {
  let content = DEFAULT_OS_INSTRUCTIONS;
  if (additional) {
    content += `\n\n${additional}`;
  }
  return content;
}
```

Default content (minimal, factual):

```markdown
# Environment

You are running inside a virtualized operating system (agentOS on Secure-Exec).

## Available runtimes
- Node.js (V8 isolate) — run JS/TS files
- WASM — POSIX coreutils (ls, grep, cat, sh, etc.)
- Python (Pyodide)

## Filesystem
- In-memory virtual filesystem (not persistent across sessions)
- Working directory: /home/user/
- Standard /dev, /proc pseudo-filesystems available
- Host node_modules mounted read-only at /root/node_modules/
- OS configuration at /etc/agentos/

## Constraints
- No native ELF binaries — only JS/TS scripts and WASM commands
- Network requests route through the kernel; external access depends on permissions
- globalThis.fetch is hardened and cannot be overridden
```

### Opt-out

`createSession()` accepts options to control injection:

```typescript
interface CreateSessionOptions {
  cwd?: string;
  env?: Record<string, string>;
  /** Skip injecting OS-level instructions. Default: false. */
  skipOsInstructions?: boolean;
  /** Additional instructions appended to the OS defaults. */
  additionalInstructions?: string;
}
```

When `skipOsInstructions: true`, the `/etc/agentos/` directory is still mounted (it's part of the OS), but no `--append-system-prompt` or equivalent flags are passed. The agent can still discover and read the file if it wants to — we just don't force-inject it.

## Implementation outline

1. **`os-instructions.ts`** — Exports `getOsInstructions(additional?)` returning the markdown string (already implemented)
2. **`agent-os.ts` `create()`** — After kernel init, write `/etc/agentos/instructions.md` to the VM filesystem. Once mount table lands, convert to read-only mount.
3. **`agents.ts`** — Each agent config's `prepareInstructions()` reads from `/etc/agentos/instructions.md` and returns agent-specific args/env
4. **`agent-os.ts` `createSession()`** — Before spawning, calls `prepareInstructions()` and merges returned args/env into spawn call

## Alternatives considered

### A: Per-agent CLI flags only (previous proposal)
Works but instructions exist only as transient CLI arguments. Not inspectable, not extensible, requires per-agent content construction. The `/etc/agentos/` approach makes the content a first-class filesystem citizen.

### B: Write AGENTS.md + CLAUDE.md to cwd
Risks clobbering user project files. Even conditional writes are fragile. Rejected.

### C: Write to /home/user/ (parent of cwd)
Works for walk-up agents (PI, Claude Code) but not for OpenCode (project root only). Still requires file writes to user-visible directories. Rejected.

### D: Inject via ACP protocol
ACP `session/new` has no system prompt field. Would require spec changes and bypass agent instruction-loading logic. Rejected.

### E: Write ~/.codex/AGENTS.md for Codex
Clobbers user's global Codex instructions. `-c developer_instructions` is cleaner and additive. Rejected.

### F: Mount at /usr/share/agentos/
Technically valid for read-only data, but `/etc/` is more conventional for configuration that agents are expected to read. `/usr/share/` implies data files, not configuration. Rejected in favor of `/etc/`.

## Decision

Mount OS configuration at `/etc/agentos/` (read-only once mount table is available). Per-agent injection reads from this canonical location and passes content through each agent's native mechanism. Single source of truth in the filesystem, zero writes to user directories, inspectable by agents and users.
