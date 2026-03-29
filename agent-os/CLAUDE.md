# agentOS

A high-level wrapper around the Secure-Exec OS that provides a clean API for running coding agents inside isolated VMs via the Agent Communication Protocol (ACP).

## Secure-Exec (the underlying OS)

Secure-Exec is an in-process operating system kernel written in JavaScript. All runtimes make "syscalls" into this kernel for file I/O, process spawning, networking, etc. The kernel orchestrates three execution environments:

- **WASM processes** — A custom libc and Rust toolchain compile a full suite of POSIX utilities (coreutils, sh, grep, etc.) to WebAssembly. WASM processes run in Worker threads and make synchronous syscalls to the kernel via SharedArrayBuffer RPC.
- **Node.js (V8 isolates)** — A sandboxed reimplementation of Node.js APIs (`child_process`, `fs`, `net`, etc.) runs JS/TS inside isolated V8 contexts. Module loading is hijacked to route through the kernel VFS. This is how agent code runs.
- **Python (Pyodide)** — CPython compiled to WASM via Pyodide, running in a Worker thread with kernel-backed file/network I/O.

All three runtimes implement the `RuntimeDriver` interface and are mounted into the kernel at boot. Processes can spawn children across runtimes (e.g., a Node process can spawn a WASM shell).

### Key subsystems

- **Virtual filesystem (VFS)** — Layered chunked architecture: `ChunkedVFS` composes `FsMetadataStore` (directory tree, inodes, chunk mapping) + `FsBlockStore` (key-value blob store) into a `VirtualFileSystem`. Tiered storage keeps small files inline in metadata; larger files are split into chunks in the block store. The device layer (`/dev/null`, `/dev/urandom`, `/dev/pts/*`, etc.), proc layer (`/proc/[pid]/*`), and permission wrapper sit on top. All layers implement the `VirtualFileSystem` interface with full POSIX semantics.
- **Process management** — Kernel-wide process table tracks PIDs across all runtimes. Full POSIX process model: parent/child relationships, process groups, sessions, signals (SIGCHLD, SIGTERM, SIGWINCH), zombie cleanup, and `waitpid`. Each process gets its own FD table (0-255) with refcounted file descriptions supporting dup/dup2.
- **Pipes & PTYs** — Kernel-managed pipes (64KB buffers) enable cross-runtime IPC. PTY master/slave pairs with line discipline support interactive shells. `openShell()` allocates a PTY and spawns sh/bash.
- **Networking** — Socket table manages TCP/UDP/Unix domain sockets. Loopback connections stay entirely in-kernel. External connections delegate to a `HostNetworkAdapter` (implemented via `node:net`/`node:dgram` on the host). DNS resolution also goes through the adapter.
- **Permissions** — Deny-by-default access control. Four permission domains: `fs`, `network`, `childProcess`, `env`. Each is a function that returns `{allow, reason}`. The `allowAll` preset grants everything (used in agentOS).

### What agentOS adds on top

agentOS wraps the kernel and adds: a high-level filesystem/process API, ACP agent sessions (JSON-RPC over stdio), and a `ModuleAccessFileSystem` overlay that projects host `node_modules/` into the VM read-only so agents have access to their dependencies.

## Project Structure

- **Monorepo**: pnpm workspaces + Turborepo + TypeScript + Biome (mirrors secure-exec)
- **Core package**: `@rivet-dev/agent-os-core` in `packages/core/` -- contains everything (VM ops, ACP client, session management)
- **Registry types**: `@rivet-dev/agent-os-registry-types` in `packages/registry-types/` -- shared type definitions for WASM command package descriptors. The agent-os-registry repo links to this package. When changing descriptor types, update here and rebuild the registry.
- **S3 block store**: `@rivet-dev/agent-os-s3` in `packages/s3/` -- S3-compatible `FsBlockStore` for cloud-persistent VFS storage. Pluggable driver, not imported by core.
- **npm scope**: `@rivet-dev/agent-os-*`
- **Actor integration** lives inline in `rivetkit-typescript/packages/rivetkit/src/agent-os/`, not as a separate package
- **The actor layer must maintain 1:1 feature parity with AgentOs.** Every public method on the `AgentOs` class (`packages/core/src/agent-os.ts`) must have a corresponding actor action in `rivetkit-typescript/packages/rivetkit/src/agent-os/`. Subscription methods (onProcessStdout, onShellData, onCronEvent, etc.) are wired through actor events. Lifecycle methods (dispose) are handled by the actor's onSleep/onDestroy hooks. When adding a new public method to AgentOs, add the corresponding actor action in the same change.
- **The RivetKit driver test suite must have full feature coverage of all agent-os actor actions.** Tests live in `rivetkit-typescript/packages/rivetkit/src/driver-test-suite/tests/`. When adding a new actor action, add a corresponding driver test in the same change.
- **The core quickstart (`agent-os/examples/quickstart/`) and the RivetKit example (`examples/agent-os/`) must stay in sync.** Both cover the same set of features (hello-world, filesystem, processes, network, cron, tools, agent-session, sandbox) with identical behavior, just different APIs. Core uses `AgentOs.create()` directly; RivetKit uses `agentOs()` actor with client-server split. When adding or changing a quickstart example, update both.

## Terminology

- Call instances of the OS **"VMs"**, never "sandboxes"

## Architecture

- **Everything runs inside the VM.** Agent processes, servers, network requests -- all spawned inside the secure-exec kernel, never on the host. This is a hard rule with no exceptions.
- The `AgentOs` class wraps a secure-exec `Kernel` and proxies its API directly
- **All public methods on AgentOs must accept and return JSON-serializable data.** No object references (Session, ManagedProcess, ShellHandle) in the public API. Reference resources by ID (session ID, PID, shell ID). This keeps the API flat and portable across serialization boundaries (HTTP, RPC, IPC).
- Filesystem methods mirror the kernel API 1:1 (readFile, writeFile, mkdir, readdir, stat, exists, move, delete)
- **readdir returns `.` and `..` entries** — always filter them when iterating children to avoid infinite recursion
- Command execution mirrors the kernel API (exec, spawn)
- `fetch(port, request)` reaches services running inside the VM using the secure-exec network adapter pattern (`proc.network.fetch`)

## Virtual Filesystem Design Reference

- The VFS chunking and metadata architecture is modeled after **JuiceFS** (https://juicefs.com/docs/community/architecture/). Reference JuiceFS docs when designing chunk/block storage, metadata engine separation, or read/write data paths.
- Key JuiceFS concepts that apply: three-tier data model (Chunk/Slice/Block), pluggable metadata engines (SQLite, Redis, PostgreSQL), fixed-size block storage in object stores (S3), and metadata-data separation.
- For detailed design analysis: https://juicefs.com/en/blog/engineering/design-metadata-data-storage

### Agent-OS filesystem packages

- The old `fs-sqlite` and `fs-postgres` packages were deleted. They are replaced by the secure-exec `SqliteMetadataStore` and the `ChunkedVFS` composition layer.
- **`packages/s3/`** (`@rivet-dev/agent-os-s3`): Exports `S3BlockStore` implementing the secure-exec `FsBlockStore` interface. Used with `ChunkedVFS(SqliteMetadataStore + S3BlockStore)` for cloud-persistent storage. Pluggable driver passed via `type: "custom"` mount, not imported by core.
- **`packages/google-drive/`** (`@rivet-dev/agent-os-google-drive`): Preview. Exports `GoogleDriveBlockStore` implementing the secure-exec `FsBlockStore` interface. Uses Google Drive API v3 via service account credentials. Tests are gated behind `GOOGLE_DRIVE_CLIENT_EMAIL`, `GOOGLE_DRIVE_PRIVATE_KEY`, and `GOOGLE_DRIVE_FOLDER_ID` environment variables.
- **`packages/sandbox/`** (`@rivet-dev/agent-os-sandbox`): Sandbox extension. Contains both `createSandboxFs()` (VirtualFileSystem backed by Sandbox Agent SDK) and `createSandboxToolkit()`. The old `fs-sandbox` package was merged into this.
- The Rivet actor integration (`rivetkit-typescript/packages/rivetkit/src/agent-os/`) uses `ChunkedVFS(InMemoryMetadataStore + InMemoryBlockStore)` as a temporary in-memory solution. A persistent backend (actor KV-backed metadata + actor storage-backed blocks) is planned.

## Filesystem Conventions

- **OS-level content uses mounts, not post-boot writes.** If agentOS needs custom directories in the VM (e.g., `/etc/agentos/`), mount a pre-populated filesystem at boot — don't create the kernel and then write files into it afterward. This keeps the root filesystem clean and makes OS-provided paths read-only so agents can't tamper with them.
- **Never interfere with the user's filesystem or code.** Don't write config files, instruction files, or metadata into the user's working directory or project tree. Use dedicated OS paths (`/etc/`, `/var/`, etc.) or CLI flags instead. If an agent framework requires a file in the project directory (e.g., OpenCode's context paths), prefer absolute paths to OS-managed locations over creating files in cwd.
- **Agent prompt injection must be non-destructive.** Each agent has its own mechanism for loading instructions (CLI flags, env vars, config files). When injecting OS instructions: preserve the agent's existing user-provided instructions (CLAUDE.md, AGENTS.md, etc.), append rather than replace, and always provide `skipOsInstructions` opt-out. User configuration is never clobbered — user env vars override ours via spread order.

## Dependencies

- **secure-exec** is a `link:` dependency pointing to `~/secure-exec-1` (relative paths from each package)
- **Rivet repo** — A modifiable copy lives at `~/r-aos`. Use this when you need to make changes to the Rivet codebase.
- We can modify secure-exec as needed to fix issues or add missing APIs
- **Prefer implementing in secure-exec** when a feature is fundamentally an OS-level concern (filesystem, process management, networking). agentOS should be a thin wrapper, not a reimplementation. If adding something to secure-exec simplifies the agentOS implementation, do it there.
- **Fix root causes in secure-exec, not workarounds in agentOS.** If something is broken at the kernel/runtime level (PATH resolution, networking, process spawning), fix it in secure-exec. Don't add patchwork in agentOS to compensate for VM bugs. The only code in agentOS should be the high-level API surface and ACP session management.
- Mount host `node_modules` read-only for agent packages (pi-acp, etc.)

## Agent Sessions (ACP)

- Uses the **Agent Communication Protocol** (ACP) -- JSON-RPC 2.0 over stdio (newline-delimited)
- No HTTP adapter layer; communicate directly with agent ACP adapters over stdin/stdout
- Reference `~/sandbox-agent` for ACP integration patterns (how pi-acp is spawned, JSON-RPC protocol, session lifecycle). Do not copy code from it.
- ACP docs: https://agentclientprotocol.com/get-started/introduction
- Session design is **agent-agnostic**: each agent type has a config specifying its ACP adapter package and main agent package name
- Currently configured agents: PI (`pi-acp`), OpenCode (`opencode-ai`). Only PI is tested.
- **OpenCode limitation**: OpenCode is a native ELF binary (compiled Go), not Node.js. The `opencode-ai` npm package is a wrapper that spawns the native binary. It cannot run inside the VM because the kernel only supports JS/WASM command execution.
- `createSession("pi")` spawns the ACP adapter inside the VM, which then spawns the agent

### Agent Configs

Each agent type needs:
- `acpAdapter`: npm package name for the ACP adapter (e.g., `pi-acp`)
- `agentPackage`: npm package name for the underlying agent (e.g., `@mariozechner/pi-coding-agent`)
- Any environment variables or flags needed

## Testing

- **Framework**: vitest
- **All tests run inside the VM** -- network servers, file I/O, agent processes
- Network tests: write a server script file, run it with `node` inside the VM, then `vm.fetch()` against it
- Agent tests must be run sequentially in layers:
  1. PI headless mode (spawn pi directly, verify output)
  2. pi-acp manual spawn (JSON-RPC over stdio)
  3. Full `createSession()` API
- **API tokens**: All tests use `@copilotkit/llmock` with `ANTHROPIC_API_KEY='mock-key'`. No real API tokens needed. Do not load tokens from `~/misc/env.txt` or any external file.
- **Mock LLM testing**: Use `@copilotkit/llmock` to run a mock LLM server on the HOST (not inside the VM). Use `loopbackExemptPorts` in `AgentOs.create()` to exempt the mock port from SSRF checks. The kernel needs `permissions: allowAll` for network access.
- **Module access**: Set `moduleAccessCwd` in `AgentOs.create()` to a host dir with `node_modules/`. pnpm puts devDeps in `packages/core/node_modules/` which are accessible via the ModuleAccessFileSystem overlay.

### Known VM Limitations

- `globalThis.fetch` is hardened (non-writable) in the VM — can't be mocked in-process
- Kernel child_process.spawn can't resolve bare commands from PATH (e.g., `pi`). Use `PI_ACP_PI_COMMAND` env var to point to the `.js` entry directly. The Node runtime resolves `.js`/`.mjs`/`.cjs` file paths as node scripts.
- `kernel.readFile()` does NOT see the ModuleAccessFileSystem overlay — read host files directly with `readFileSync` for package.json resolution
- Native ELF binaries cannot execute in the VM — the kernel's command resolver only handles `.js`/`.mjs`/`.cjs` scripts and WASM commands. `child_process.spawnSync` returns `{ status: 1, stderr: "ENOENT: command not found" }` for native binaries.

### Debugging Policy

- **Never guess without concrete logs.** Every assertion about what's happening at runtime must be backed by log output. If you don't have logs proving something, add them before making claims. Use logging liberally when debugging -- add logs at every decision point and trace the full execution path before drawing conclusions. Never assume something is a timeout issue unless there are logs proving the system was actively busy for the entire duration. An idle hang and a slow operation look the same from the outside -- only logs can distinguish them.
- **Never use CJS transpilation as a workaround** for ESM module loading issues. The VM must use V8's native ESM module system and Node.js native imports. Fix root causes in the ESM resolver, module access overlay, or V8 runtime instead of transforming ESM to CJS. The correct approach is to implement proper CJS/ESM interop in the V8 module resolver (wrapping CJS modules in ESM shims with named exports).
- **Maintain a friction log** at `.agent/notes/vm-friction.md` for anything that behaves differently from a standard POSIX/Node.js system. Document the deviation, the root cause, and whether a fix exists.

## Documentation

- **Keep docs in `~/r-aos/docs/docs/agent-os/` up to date** when public API methods or types are added, removed, or changed on AgentOs or Session classes.
- **Keep `website/src/data/registry.ts` up to date.** When adding, removing, or renaming a package, update this file so the website reflects the current set of available apps (agents, file-systems, software, and sandbox providers). Every new agent-os package or agent-os-registry software package must have a corresponding entry.
- **No implementation details in user-facing docs.** Never mention WebAssembly, WASM, V8 isolates, Pyodide, or SQLite VFS in documentation outside of `architecture.mdx`. These are internal implementation details. Use user-facing language instead: "persistent filesystem" not "SQLite VFS", "JavaScript, TypeScript, Python, and shell commands" not "WASM, V8 isolates, and Pyodide", "sandboxed execution" not "WebAssembly and V8 isolates". The `architecture.mdx` page is the only place where internals are appropriate.

## Software Registry

All WASM command source code and software packages live in `~/agent-os-registry/` (GitHub: `rivet-dev/agent-os-registry`). Software packages are in `software/` (not `packages/`). This includes the Rust and C source, build system, and npm package wrappers. Each package corresponds to a Debian/apt package name and publishes as `@rivet-dev/agent-os-{name}`. See `~/agent-os-registry/CLAUDE.md` for naming conventions, package types, and how to add new packages. No WASM command code remains in secure-exec.

The registry depends on `@rivet-dev/agent-os-registry-types` (in this repo at `packages/registry-types/`) via a link dependency. This is the single source of truth for descriptor types like `WasmCommandPackage` and `WasmMetaPackage`.

## Ralph PRD

The Ralph PRD is at `scripts/ralph/prd.json`.

## Git

- **Commit messages**: Single-line conventional commits (e.g., `feat: add host tools RPC server`). No body, no co-author trailers.

## Build & Dev

```bash
pnpm install
pnpm build        # turbo run build
pnpm test         # turbo run test
pnpm check-types  # turbo run check-types
pnpm lint         # biome check
```
