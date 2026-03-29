# VM Observability & API Gaps Spec

Adds five capabilities to agentOS that sandbox-agent exposes but agentOS currently lacks: full process tree, richer process metadata, agent registry, recursive readdir, and batch file operations.

## Type Alignment Principle

agentOS mirrors the kernel API 1:1. Process types are no exception: agentOS re-exports the kernel's `ProcessInfo` directly rather than defining its own parallel type. The existing `ProcessInfo` in agentOS (used by `listProcesses()`) is renamed to `SpawnedProcessInfo` to resolve the naming collision.

### Rename: `ProcessInfo` → `SpawnedProcessInfo`

The current agentOS `ProcessInfo` only describes processes spawned via `spawn()`:

```typescript
// BEFORE (agent-os.ts)
export interface ProcessInfo {
  pid: number;
  command: string;
  args: string[];
  running: boolean;
  exitCode: number | null;
}
```

Rename to `SpawnedProcessInfo`. `listProcesses()` and `getProcess()` return this type. Update `index.ts` exports accordingly. This is a **breaking change** for consumers importing the type by name.

After the rename, `ProcessInfo` in agentOS means the kernel's `ProcessInfo` — re-exported from `@secure-exec/core`.


## 1. Process Tree / Full Kernel Process List

### Problem

`AgentOs.listProcesses()` only returns processes spawned via `AgentOs.spawn()` — it tracks them in a private `_processes` Map. The kernel has a complete process table (`kernel.processes`) with every process across all runtimes (WASM, Node, Python), including children spawned by children. agentOS consumers can't see any of that.

### Design

Add two new methods to `AgentOs`:

```typescript
/** Every process in the kernel, not just ones spawned via spawn(). */
allProcesses(): ProcessInfo[]

/** Processes organized as a tree using ppid relationships. */
processTree(): ProcessTreeNode[]
```

#### Types

`ProcessInfo` is re-exported from `@secure-exec/core` (see secure-exec changes below). agentOS adds only the tree node:

```typescript
// Re-exported from @secure-exec/core — not defined in agentOS
import type { ProcessInfo } from "@secure-exec/core";

// agentOS-specific: adds tree structure on top of kernel type
export interface ProcessTreeNode extends ProcessInfo {
  children: ProcessTreeNode[];
}
```

#### `allProcesses()` implementation

Direct projection of `this.kernel.processes` (`ReadonlyMap<number, ProcessInfo>`) to an array:

```typescript
allProcesses(): ProcessInfo[] {
  return [...this.kernel.processes.values()];
}
```

No type mapping, no field renaming. The kernel type is the API type.

#### `processTree()` implementation

```
1. entries = allProcesses()
2. index = Map<pid, ProcessTreeNode>  (each node starts with children: [])
3. for each entry: if entry.ppid exists in index, push to parent.children
4. roots = entries where ppid === 0 or ppid not in index
5. return roots
```

No recursion needed — single pass with a lookup map.

#### Interaction with existing API

`listProcesses()`, `getProcess()`, `stopProcess()`, `killProcess()` stay unchanged. They continue to operate on the `_processes` tracking map (processes spawned via `spawn()`) and return `SpawnedProcessInfo`.

`stopProcess()` and `killProcess()` could optionally be extended to work on any kernel PID, not just tracked ones. Defer this — it's a separate decision about whether consumers should be able to kill processes they didn't create (e.g., agent child processes).


## 2. Richer Process Metadata (secure-exec changes)

### Problem

The kernel's public `ProcessInfo` currently has: `pid`, `ppid`, `pgid`, `sid`, `driver`, `command`, `status`, `exitCode`. Missing: `args`, `cwd`, timestamps. These all exist on the internal `ProcessEntry` but aren't projected to the public type.

### secure-exec changes

All changes in `packages/core/src/kernel/`:

1. **`types.ts`** — Expand `ProcessInfo` with fields already on `ProcessEntry`:

```typescript
export interface ProcessInfo {
  pid: number;
  ppid: number;
  pgid: number;
  sid: number;
  driver: string;
  command: string;
  args: string[];          // NEW — from ProcessEntry
  cwd: string;             // NEW — from ProcessEntry
  status: "running" | "stopped" | "exited";
  exitCode: number | null;
  startTime: number;       // NEW — add to ProcessEntry too
  exitTime: number | null; // NEW — already on ProcessEntry, now projected
}
```

2. **`process-table.ts`** — Set `startTime = Date.now()` in `register()`. Update `listProcesses()` to copy `args`, `cwd`, `startTime`, `exitTime` from `ProcessEntry`.

These are all additive, non-breaking changes. All existing code that reads `ProcessInfo` gets more fields.

### agentOS changes

None. `allProcesses()` returns the kernel's `ProcessInfo` directly. The richer metadata is available automatically once secure-exec is updated.


## 3. Agent Registry

### Problem

`AGENT_CONFIGS` is a hardcoded `Record<AgentType, AgentConfig>` with no way to query it at runtime. Consumers can't discover what agents are available, what packages they need, or whether they're actually runnable (i.e., are their npm packages installed in the module access CWD?).

sandbox-agent has `GET /v1/agents` and `GET /v1/agents/{agent}` with install status.

### Design

Add a `listAgents()` method and an `AgentRegistryEntry` type:

```typescript
export interface AgentRegistryEntry {
  /** Agent type identifier (e.g., "pi", "opencode"). */
  id: AgentType;
  /** npm package for the ACP adapter. */
  acpAdapter: string;
  /** npm package for the underlying agent. */
  agentPackage: string;
  /** Whether the ACP adapter package is resolvable from moduleAccessCwd. */
  installed: boolean;
}

listAgents(): AgentRegistryEntry[]
```

#### `installed` detection

Check if the ACP adapter's `package.json` is resolvable from `this._moduleAccessCwd`:

```typescript
try {
  readFileSync(join(this._moduleAccessCwd, "node_modules", config.acpAdapter, "package.json"));
  return true;
} catch {
  return false;
}
```

This runs on the host (not inside the VM), same as the existing module access resolution. It's a sync check — no kernel calls needed.

#### No install endpoint

sandbox-agent has `POST /agents/{agent}/install` which downloads agent binaries. We don't need this — agentOS consumers manage their own `node_modules` via pnpm. The registry just tells you what's available and what's missing.

#### Extensibility

`AGENT_CONFIGS` stays as the source of truth. No dynamic registration for now. When new agents are added to the config, they automatically appear in `listAgents()`.


## 4. Recursive readdir

### Problem

`AgentOs.readdir()` wraps `kernel.readdir()` which returns a flat list of entry names for a single directory. No way to get a recursive file listing without the consumer walking the tree themselves. sandbox-agent has `GET /v1/fs/entries?recursive=true` returning full paths with metadata.

### Design

Add a `readdirRecursive()` method:

```typescript
export interface DirEntry {
  /** Absolute path. */
  path: string;
  /** "file" | "directory" | "symlink" */
  type: "file" | "directory" | "symlink";
  /** File size in bytes (0 for directories). */
  size: number;
}

async readdirRecursive(path: string, options?: ReaddirRecursiveOptions): Promise<DirEntry[]>
```

```typescript
export interface ReaddirRecursiveOptions {
  /** Maximum depth to recurse. undefined = unlimited. */
  maxDepth?: number;
  /** Glob patterns to exclude (e.g., ["node_modules", ".git"]). */
  exclude?: string[];
}
```

#### Implementation

Async BFS using `kernel.readdir()` + `kernel.stat()` per entry:

```
queue = [path]
results = []
while queue.length > 0:
  dir = queue.shift()
  entries = await kernel.readdir(dir)
  for name in entries (skip "." and ".."):
    fullPath = dir + "/" + name
    st = await kernel.stat(fullPath)
    results.push({ path: fullPath, type, size })
    if st.isDirectory and depth < maxDepth and not excluded:
      queue.push(fullPath)
return results
```

Filter `.` and `..` as required by CLAUDE.md. Symlinks are reported as `"symlink"` — not followed (avoids cycles).

#### Why not in secure-exec?

This is a convenience method, not an OS-level primitive. The kernel provides the building blocks (`readdir` + `stat`). Recursive traversal with filtering/depth-limiting is application-level logic that belongs in agentOS.


## 5. Batch File Operations

### Problem

Writing multiple files requires N separate `writeFile()` calls. sandbox-agent has `POST /v1/fs/upload-batch` (multipart) for uploading multiple files in one call. When setting up an agent workspace (project files, config, etc.), batch operations are more ergonomic and can be optimized.

### Design

Two batch methods:

```typescript
export interface BatchWriteEntry {
  path: string;
  content: string | Uint8Array;
}

export interface BatchWriteResult {
  path: string;
  success: boolean;
  error?: string;
}

/** Write multiple files. Creates parent directories as needed. Non-atomic — partial success is possible. */
async writeFiles(entries: BatchWriteEntry[]): Promise<BatchWriteResult[]>

export interface BatchReadResult {
  path: string;
  content: Uint8Array | null;
  error?: string;
}

/** Read multiple files. Returns null content for files that don't exist or fail. */
async readFiles(paths: string[]): Promise<BatchReadResult[]>
```

#### Implementation

Sequential kernel calls in a loop. No parallelism — the kernel is single-threaded and its VFS operations don't benefit from concurrency. `writeFiles` creates parent dirs via `mkdir -p` logic before writing.

```typescript
async writeFiles(entries: BatchWriteEntry[]): Promise<BatchWriteResult[]> {
  const results: BatchWriteResult[] = [];
  for (const entry of entries) {
    try {
      await this.writeFile(entry.path, entry.content);
      results.push({ path: entry.path, success: true });
    } catch (err) {
      results.push({ path: entry.path, success: false, error: String(err) });
    }
  }
  return results;
}
```

Intentionally non-atomic. Callers get per-file success/failure. No rollback — if file 3 of 5 fails, files 1-2 are already written. This matches sandbox-agent's behavior.

#### Batch delete

Not included for now. `delete({ recursive: true })` already handles the common case (removing a directory tree). A batch delete of unrelated paths is rare enough to do with a loop.


## Exports

Changes to `index.ts`:

```typescript
// Re-export kernel's ProcessInfo directly
export type { ProcessInfo } from "@secure-exec/core";

// Renamed existing type
export type { SpawnedProcessInfo } from "./agent-os.js";

// New agentOS types
export type {
  ProcessTreeNode,
  AgentRegistryEntry,
  DirEntry,
  ReaddirRecursiveOptions,
  BatchWriteEntry,
  BatchWriteResult,
  BatchReadResult,
} from "./agent-os.js";
```

Breaking change: consumers importing `ProcessInfo` from `@rivet-dev/agent-os` get the kernel type (with ppid, driver, etc.) instead of the old spawn-tracking type. They need to update to `SpawnedProcessInfo` if they were using the old shape.


## secure-exec Changes (Summary)

All changes in `packages/core/src/kernel/`:

1. **`types.ts`** — Add `args`, `cwd`, `startTime`, `exitTime` to `ProcessInfo`
2. **`process-table.ts`** — Set `startTime = Date.now()` in `register()`. Update `listProcesses()` to project the new fields.

No breaking changes to secure-exec. All existing kernel consumers get more fields.


## Testing

| Feature | Test approach |
|---|---|
| `allProcesses()` | Boot VM, spawn a process, verify it appears alongside the init process. Check ppid relationships. |
| `processTree()` | Spawn a shell that spawns a child. Verify tree structure: root → shell → child. |
| `ProcessInfo` fields | Verify `startTime` is set, `args`/`cwd` are correct, `exitTime` populates after process exits. |
| `listAgents()` | Check that pi/opencode appear. Verify `installed` is true when package exists, false when not. |
| `readdirRecursive()` | Create nested dirs with files, verify flat listing. Test `maxDepth`, `exclude`, symlink reporting. |
| `writeFiles()` | Batch write 3 files, verify all exist. Write with one bad path, verify partial success. |
| `readFiles()` | Write files, batch read them. Include a missing path, verify null content + error. |
