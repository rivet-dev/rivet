# @rivet-dev/agent-os

A high-level SDK for running coding agents in isolated VMs. agentOS manages the full lifecycle of virtual machines — from filesystem setup and process management to launching AI agents via the Agent Communication Protocol (ACP).

Agents run inside sandboxed VMs with their own filesystem, process table, and network stack. The host only communicates through well-defined APIs, keeping agent execution fully contained.

## Features

- **VM lifecycle** — create, configure, and dispose isolated virtual machines
- **Agent sessions (ACP)** — launch coding agents (PI, OpenCode) via JSON-RPC over stdio
- **Filesystem operations** — read, write, mkdir, stat, move, delete, recursive listing, batch read/write
- **Process management** — spawn, exec, stop, kill processes; inspect process trees across all runtimes
- **Agent registry** — discover available agents and their installation status
- **Networking** — reach services running inside the VM via `fetch()`
- **Shell access** — open interactive shells with PTY support
- **Mount backends** — memory, host directory, S3, overlay (copy-on-write), or custom VirtualFileSystem

## Quick Start

```bash
npm install @rivet-dev/agent-os
# Install an agent adapter + its underlying agent
npm install pi-acp @mariozechner/pi-coding-agent
```

```typescript
import { AgentOs } from "@rivet-dev/agent-os-core";

// 1. Create a VM
const vm = await AgentOs.create();

// 2. Create an agent session
const session = await vm.createSession("pi");

// 3. Send a prompt
const response = await session.prompt("Write a hello world in TypeScript");

// 4. Clean up
session.close();
await vm.dispose();
```

## API Reference

### Lifecycle

| Method | Signature | Description |
|--------|-----------|-------------|
| `create` | `static create(options?: AgentOsOptions): Promise<AgentOs>` | Create and boot a new VM |
| `dispose` | `dispose(): Promise<void>` | Shut down the VM and all sessions |

### Filesystem

| Method | Signature | Description |
|--------|-----------|-------------|
| `readFile` | `readFile(path: string): Promise<Uint8Array>` | Read a file |
| `writeFile` | `writeFile(path: string, content: string \| Uint8Array): Promise<void>` | Write a file |
| `readFiles` | `readFiles(paths: string[]): Promise<BatchReadResult[]>` | Batch read multiple files |
| `writeFiles` | `writeFiles(entries: BatchWriteEntry[]): Promise<BatchWriteResult[]>` | Batch write multiple files (creates parent dirs) |
| `mkdir` | `mkdir(path: string): Promise<void>` | Create a directory |
| `readdir` | `readdir(path: string): Promise<string[]>` | List directory entries |
| `readdirRecursive` | `readdirRecursive(path: string, options?: ReaddirRecursiveOptions): Promise<DirEntry[]>` | Recursively list directory contents with metadata |
| `stat` | `stat(path: string): Promise<VirtualStat>` | Get file/directory metadata |
| `exists` | `exists(path: string): Promise<boolean>` | Check if a path exists |
| `move` | `move(from: string, to: string): Promise<void>` | Rename/move a file or directory |
| `delete` | `delete(path: string, options?: { recursive?: boolean }): Promise<void>` | Delete a file or directory |
| `mountFs` | `mountFs(path: string, config: MountConfig): void` | Mount a filesystem at the given path |
| `unmountFs` | `unmountFs(path: string): void` | Unmount a filesystem |

### Process Management

| Method | Signature | Description |
|--------|-----------|-------------|
| `exec` | `exec(command: string, options?: ExecOptions): Promise<ExecResult>` | Execute a shell command and wait for completion |
| `spawn` | `spawn(command: string, args: string[], options?: SpawnOptions): ManagedProcess` | Spawn a long-running process |
| `listProcesses` | `listProcesses(): SpawnedProcessInfo[]` | List processes started via `spawn()` |
| `allProcesses` | `allProcesses(): ProcessInfo[]` | List all kernel processes across all runtimes |
| `processTree` | `processTree(): ProcessTreeNode[]` | Get processes organized as a parent-child tree |
| `getProcess` | `getProcess(pid: number): SpawnedProcessInfo` | Get info about a specific spawned process |
| `stopProcess` | `stopProcess(pid: number): void` | Send SIGTERM to a process |
| `killProcess` | `killProcess(pid: number): void` | Send SIGKILL to a process |

### Network

| Method | Signature | Description |
|--------|-----------|-------------|
| `fetch` | `fetch(port: number, request: Request): Promise<Response>` | Send an HTTP request to a service running inside the VM |

### Shell

| Method | Signature | Description |
|--------|-----------|-------------|
| `openShell` | `openShell(options?: OpenShellOptions): { shellId: string }` | Open an interactive shell with PTY support |
| `writeShell` | `writeShell(shellId: string, data: string \| Uint8Array): void` | Write data to a shell's PTY input |
| `onShellData` | `onShellData(shellId: string, handler: (data: Uint8Array) => void): () => void` | Subscribe to shell output data |
| `resizeShell` | `resizeShell(shellId: string, cols: number, rows: number): void` | Notify terminal resize |
| `closeShell` | `closeShell(shellId: string): void` | Kill the shell process |

### Agent Sessions

| Method | Signature | Description |
|--------|-----------|-------------|
| `createSession` | `createSession(agentType: AgentType, options?: CreateSessionOptions): Promise<Session>` | Launch an agent and return a session |
| `listSessions` | `listSessions(): SessionInfo[]` | List active sessions |
| `getSession` | `getSession(sessionId: string): Session` | Get a session by ID |
| `resumeSession` | `resumeSession(sessionId: string): Session` | Retrieve an active session by ID |
| `destroySession` | `destroySession(sessionId: string): Promise<void>` | Gracefully cancel and close a session |

### Agent Registry

| Method | Signature | Description |
|--------|-----------|-------------|
| `listAgents` | `listAgents(): AgentRegistryEntry[]` | List registered agents with installation status |

### Session Class

| Method | Signature | Description |
|--------|-----------|-------------|
| `prompt` | `prompt(text: string): Promise<JsonRpcResponse>` | Send a prompt and wait for the response |
| `cancel` | `cancel(): Promise<JsonRpcResponse>` | Cancel ongoing agent work |
| `close` | `close(): void` | Kill the agent process and clean up |
| `onSessionEvent` | `onSessionEvent(handler: SessionEventHandler): void` | Subscribe to session update notifications |
| `onPermissionRequest` | `onPermissionRequest(handler: PermissionRequestHandler): void` | Subscribe to permission requests |
| `respondPermission` | `respondPermission(permissionId: string, reply: PermissionReply): Promise<JsonRpcResponse>` | Reply to a permission request |
| `setMode` | `setMode(modeId: string): Promise<JsonRpcResponse>` | Set the session mode (e.g., "plan") |
| `getModes` | `getModes(): SessionModeState \| null` | Get available modes |
| `setModel` | `setModel(model: string): Promise<JsonRpcResponse>` | Set the model |
| `setThoughtLevel` | `setThoughtLevel(level: string): Promise<JsonRpcResponse>` | Set reasoning level |
| `getConfigOptions` | `getConfigOptions(): SessionConfigOption[]` | Get available config options |
| `getEvents` | `getEvents(options?: GetEventsOptions): JsonRpcNotification[]` | Get event history |
| `getSequencedEvents` | `getSequencedEvents(options?: GetEventsOptions): SequencedEvent[]` | Get event history with sequence numbers |
| `rawSend` | `rawSend(method: string, params?: Record<string, unknown>): Promise<JsonRpcResponse>` | Send an arbitrary ACP request |

**Session properties:** `sessionId`, `agentType`, `capabilities`, `agentInfo`, `closed`

### Exported Types

**VM & Options**
- `AgentOsOptions` — VM creation options (commandDirs, loopbackExemptPorts, moduleAccessCwd, mounts, additionalInstructions)
- `CreateSessionOptions` — Session options (cwd, env, mcpServers, skipOsInstructions, additionalInstructions)

**Mount Configurations**
- `MountConfig` — Union of all mount types
- `MountConfigMemory` — In-memory filesystem
- `MountConfigCustom` — Caller-provided VirtualFileSystem
- `MountConfigHostDir` — Host directory with symlink escape prevention
- `MountConfigS3` — S3-compatible object storage
- `MountConfigOverlay` — Copy-on-write overlay (lower + upper layers)

**MCP Servers**
- `McpServerConfig` — Union of local and remote MCP configs
- `McpServerConfigLocal` — Local MCP server (command, args, env)
- `McpServerConfigRemote` — Remote MCP server (url, headers)

**Process**
- `ProcessInfo` — Kernel process info (pid, ppid, pgid, sid, driver, command, args, cwd, status, exitCode, startTime, exitTime)
- `SpawnedProcessInfo` — Info for processes created via `spawn()` (pid, command, args, running, exitCode)
- `ProcessTreeNode` — ProcessInfo with `children: ProcessTreeNode[]`

**Filesystem**
- `DirEntry` — Directory entry (path, type, size)
- `ReaddirRecursiveOptions` — Options for recursive listing (maxDepth, exclude)
- `BatchWriteEntry` — Entry for batch writes (path, content)
- `BatchWriteResult` — Result of a batch write (path, success, error?)
- `BatchReadResult` — Result of a batch read (path, content, error?)

**Agent**
- `AgentType` — `"pi" | "opencode"`
- `AgentConfig` — Agent configuration (acpAdapter, agentPackage, prepareInstructions)
- `AgentRegistryEntry` — Registry entry (id, acpAdapter, agentPackage, installed)

**Session**
- `SessionInfo` — Session summary (sessionId, agentType)
- `SessionInitData` — Data from ACP initialize response
- `SessionMode` — A mode the agent supports
- `SessionModeState` — Current mode and available modes
- `SessionConfigOption` — A configuration option the agent supports
- `AgentCapabilities` — Boolean capability flags from the agent
- `AgentInfo` — Agent identity (name, version)
- `PermissionRequest` — Permission request from an agent
- `PermissionReply` — `"once" | "always" | "reject"`
- `PermissionRequestHandler` — Handler for permission requests
- `SessionEventHandler` — Handler for session update events
- `SequencedEvent` — Notification with sequence number
- `GetEventsOptions` — Filter options for event history (since, method)

**Protocol**
- `JsonRpcRequest`, `JsonRpcResponse`, `JsonRpcNotification`, `JsonRpcError`

**Backends**
- `HostDirBackendOptions`, `OverlayBackendOptions`, `S3BackendOptions`
