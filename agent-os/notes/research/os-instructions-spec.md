# Spec: OS System Instructions Injection

## Overview

Inject a default set of OS-level instructions into every agent session so the agent knows it's running inside agentOS on Secure-Exec. Each agent has a different injection mechanism. User-provided configuration (env vars, CLI args) is never clobbered — our injection extends, not replaces.

## Data flow

```
createSession(agentType, options)
  │
  ├─ if skipOsInstructions → skip injection
  │
  ├─ load instructions from fixtures/AGENTOS_SYSTEM_PROMPT.md
  │  └─ append options.additionalInstructions if present
  │
  ├─ call config.prepareInstructions(kernel, cwd, instructions)
  │  └─ returns { args?: string[], env?: Record<string, string> }
  │
  ├─ merge into spawn call:
  │  spawn("node", [binPath, ...extraArgs], {
  │    env: { ...extraEnv, ...options.env },  // user env wins
  │    cwd,
  │  })
  │
  └─ proceed with ACP initialize + session/new
```

## API changes

### CreateSessionOptions (agent-os.ts)

```ts
interface CreateSessionOptions {
  cwd?: string;
  env?: Record<string, string>;
  mcpServers?: McpServerConfig[];
  /** Skip injecting OS-level instructions. Default: false. */
  skipOsInstructions?: boolean;
  /** Additional instructions appended after the OS defaults. */
  additionalInstructions?: string;
}
```

### AgentConfig (agents.ts)

```ts
interface AgentConfig {
  acpAdapter: string;
  agentPackage: string;
  /**
   * Prepare agent-specific spawn overrides for OS instruction injection.
   * Returns extra CLI args and env vars to merge into the spawn call.
   * IMPORTANT: Must extend (not replace) the user's existing config.
   * User-provided env vars and args always take priority.
   */
  prepareInstructions?(
    kernel: Kernel,
    cwd: string,
    instructions: string,
  ): Promise<{ args?: string[]; env?: Record<string, string> }>;
}
```

## Per-agent injection

### PI — `--append-system-prompt` CLI flag

```ts
prepareInstructions: async (_kernel, _cwd, instructions) => ({
  args: ["--append-system-prompt", instructions],
})
```

- Zero filesystem writes
- Appended to (not replacing) PI's default system prompt
- User's `AGENTS.md`/`CLAUDE.md` at cwd still loads via PI's directory walk

### Claude Code — `--append-system-prompt` CLI flag

```ts
prepareInstructions: async (_kernel, _cwd, instructions) => ({
  args: ["--append-system-prompt", instructions],
})
```

- Zero filesystem writes
- User's `CLAUDE.md` at cwd and `~/.claude/CLAUDE.md` still loads normally

### OpenCode — `OPENCODE_CONTEXTPATHS` env var + file write

```ts
prepareInstructions: async (kernel, cwd, instructions) => {
  await kernel.mkdir(`${cwd}/.agent-os`);
  await kernel.writeFile(`${cwd}/.agent-os/instructions.md`, instructions);

  // Default contextPaths from OpenCode source + our instructions file
  const contextPaths = [
    ".github/copilot-instructions.md",
    ".cursorrules",
    ".cursor/rules/",
    "CLAUDE.md",
    "CLAUDE.local.md",
    "opencode.md",
    "opencode.local.md",
    "OpenCode.md",
    "OpenCode.local.md",
    "OPENCODE.md",
    "OPENCODE.local.md",
    ".agent-os/instructions.md",
  ];

  return {
    env: { OPENCODE_CONTEXTPATHS: JSON.stringify(contextPaths) },
  };
}
```

- Only agent that requires a filesystem write (to namespaced `.agent-os/` dir)
- Env var overrides project contextPaths config but includes all defaults so standard user files still discovered
- Absolute paths don't work in OpenCode's contextPaths (`filepath.Join` treats as relative)

### Codex — `-c developer_instructions="..."` CLI flag

```ts
prepareInstructions: async (_kernel, _cwd, instructions) => ({
  args: ["-c", `developer_instructions=${instructions}`],
})
```

- Zero filesystem writes
- `developer_instructions` injected as additive developer role message — does not replace built-in system instructions
- User's `AGENTS.md` at cwd and `~/.codex/AGENTS.md` still loads normally

## Instructions content

Ships as `packages/core/fixtures/AGENTOS_SYSTEM_PROMPT.md` in the npm package. Read at runtime via `readFileSync` relative to `__dirname`.

Content describes: environment (agentOS on Secure-Exec), available runtimes (Node.js, WASM, Python), filesystem layout, constraints (no native ELF binaries, hardened fetch, etc.).

## Merge semantics

The key invariant: **user configuration is never clobbered**.

- **CLI args**: Extra args prepended before any user args. User args (from future extensions) would appear later and take precedence.
- **Env vars**: `{ ...extraEnv, ...options.env }` — user-provided env vars override ours. If the user passes `OPENCODE_CONTEXTPATHS`, their value wins.
- **File writes**: Only OpenCode writes `.agent-os/instructions.md` to cwd. Namespaced hidden directory minimizes collision risk.

## New files

| File | Purpose |
|------|---------|
| `packages/core/fixtures/AGENTOS_SYSTEM_PROMPT.md` | Shared OS instructions markdown (ships with npm package) |
| `packages/core/src/os-instructions.ts` | `getOsInstructions(additional?)` — reads fixture, appends additional |

## Agents not yet in AGENT_CONFIGS

Claude Code and Codex are not in AGENT_CONFIGS yet (only PI and OpenCode are). Their configs should be added with `prepareInstructions` when they become runnable in the VM. Until then, the approach is documented in code comments in `agents.ts`.
