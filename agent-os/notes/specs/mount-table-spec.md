# Mount Table Spec

Linux-style VFS mount table for the secure-exec kernel. Replaces the current hardcoded layer composition (DeviceLayer wraps ProcLayer wraps base FS) with a unified mount table where all filesystem backends -- including `/dev` and `/proc` -- are registered through the same mechanism.

## Motivation

The kernel currently composes filesystem layers by wrapping VFS implementations:

```typescript
let fs = createDeviceLayer(options.filesystem);  // intercepts /dev/*
fs = createProcLayer(fs, { ... });               // intercepts /proc/*
fs = wrapFileSystem(fs, options.permissions);     // gates everything
this.vfs = fs;
```

Each layer implements all 27 `VirtualFileSystem` methods, checks "is this my path?", and delegates everything else. This has three problems:

1. **O(n) passthrough** -- every filesystem call walks through every layer even when no layer handles it. A `readFile("/home/user/foo.txt")` passes through permissions, then ProcLayer (not `/proc`, pass through), then DeviceLayer (not `/dev`, pass through), then hits the base FS.

2. **No user-configurable mounts** -- adding a new mount point (S3, host directory, etc.) means writing a new VFS wrapper class with all 27 methods and manually inserting it in the chain.

3. **Layers can't see each other** -- cross-mount `rename` can't return `EXDEV` because no single component knows all mount points. `/proc/mounts` can't list mounts for the same reason.

Linux solves all three with a mount table. One routing table, all filesystems registered in it, O(1) dispatch per operation.

## Architecture

```
                    Kernel
                      |
              PermissionsWrapper        (gates all operations)
                      |
                  MountTable            (routes paths to backends)
                 /    |    \      \
              "/"   "/dev"  "/proc"  "/data"  ...
               |      |       |        |
          InMemoryFS  DevBE  ProcBE  S3Backend
```

The `MountTable` implements `VirtualFileSystem` and sits where the layer chain used to be. Permissions wrapping stays as the outermost layer (same as Linux -- permission checks happen in the VFS before dispatching to the filesystem).

`ModuleAccessFileSystem` is a special case discussed in its own section.

## MountTable Class

```typescript
class MountTable implements VirtualFileSystem {
  constructor(rootFs: VirtualFileSystem);

  /**
   * Mount a filesystem at the given path.
   * Auto-creates the mount point directory in the parent filesystem if needed.
   * Throws if path is already a mount point (use unmount first).
   */
  mount(path: string, fs: VirtualFileSystem, options?: MountOptions): void;

  /**
   * Unmount the filesystem at the given path.
   * Throws if path is not a mount point or is "/".
   */
  unmount(path: string): void;

  /**
   * List all current mounts.
   */
  getMounts(): ReadonlyArray<MountEntry>;

  // ... all 27 VirtualFileSystem methods
}

interface MountOptions {
  readOnly?: boolean;
}

interface MountEntry {
  path: string;
  readOnly: boolean;
  // Future: fs type name, stats, etc.
}
```

### Path Resolution Algorithm

Every VFS method call goes through the same resolution:

1. Normalize the path (collapse `//`, `.`, `..`, strip trailing `/`)
2. Find the longest mount prefix that matches the path
3. Strip the mount prefix to get the relative path within the backend
4. Forward the relative path to the matched backend

```
resolve("/dev/null"):
  mounts: ["/", "/dev", "/proc"]
  longest match: "/dev"
  relative path: "null"
  → DeviceBackend.readFile("null")

resolve("/home/user/foo.txt"):
  longest match: "/"
  relative path: "home/user/foo.txt"
  → InMemoryFileSystem.readFile("home/user/foo.txt")
```

The mount table stores entries sorted by path length descending, so the first match is always the longest prefix. This is O(n) in number of mounts but mounts are few (typically <10), so a linear scan is fine. Binary search on a sorted array is an option if mount counts grow.

### Relative Path Handling

Backends receive paths relative to their mount point. The root mount at `/` is special: it receives paths as-is (e.g., `/home/user/foo.txt`), since the entire path IS relative to root.

For non-root mounts, the mount prefix is stripped:

| Full path | Mount point | Backend receives |
|-----------|-------------|-----------------|
| `/dev/null` | `/dev` | `null` |
| `/dev/pts/0` | `/dev` | `pts/0` |
| `/proc/self/fd` | `/proc` | `self/fd` |
| `/data/projects/foo.txt` | `/data` | `projects/foo.txt` |
| `/data` | `/data` | `` (empty string = root of backend) |

Backends should treat empty string / missing path as their root directory.

### Cross-Mount Operations

Operations that take two paths (`rename`, `link`) check whether both paths resolve to the same mount. If different mounts:

- **`rename()`** -- throw `EXDEV` ("Invalid cross-device link"). Same as Linux.
- **`link()`** -- throw `EXDEV`. Hard links cannot cross filesystem boundaries.

```typescript
async rename(oldPath: string, newPath: string): Promise<void> {
  const oldMount = this.resolve(oldPath);
  const newMount = this.resolve(newPath);
  if (oldMount.entry !== newMount.entry) {
    throw new KernelError("EXDEV", `rename across mounts: ${oldPath} -> ${newPath}`);
  }
  if (oldMount.entry.readOnly) {
    throw new KernelError("EROFS", `read-only filesystem: ${oldPath}`);
  }
  return oldMount.entry.fs.rename(oldMount.relativePath, newMount.relativePath);
}
```

### Read-Only Mount Enforcement

If a mount has `readOnly: true`, the mount table rejects write operations before they reach the backend:

- `writeFile`, `createDir`, `mkdir`, `removeFile`, `removeDir`, `rename`, `symlink`, `link`, `chmod`, `chown`, `utimes`, `truncate` -- throw `EROFS`

Read operations pass through normally.

### Directory Listing at Mount Boundaries

When `readdir("/")` or `readDirWithTypes("/")` is called on a directory that contains mount points as children, the mount table merges entries:

1. Call `readdir` on the backend that owns the path (the root FS for "/")
2. For each mount whose parent directory matches the path, add the mount point basename to the results if not already present
3. Return the merged list

Example: root FS has `/home`, `/tmp`. Mounts exist at `/dev`, `/proc`, `/data`.

`readdir("/")` returns: `["home", "tmp", "dev", "proc", "data", ...]`

Mount points always appear as directories in their parent listing, regardless of whether the backend's root is a directory.

### Auto-Creation of Mount Point Directories

When `mount("/data", backend)` is called:

1. Check if `/data` exists in the parent mount's filesystem (root FS)
2. If not, create it: `rootFs.mkdir("/data")`
3. This ensures the directory entry exists for `readdir` and `stat` on the parent

This diverges slightly from Linux (which requires the mount point to pre-exist) but makes the API simpler for programmatic use. The directory is a real entry in the parent FS, not synthesized -- unmounting reveals it as an empty directory.

## Kernel Changes

### KernelOptions

```typescript
interface KernelOptions {
  filesystem: VirtualFileSystem;
  mounts?: FsMount[];              // NEW
  permissions?: Permissions;
  env?: Record<string, string>;
  cwd?: string;
  maxProcesses?: number;
  hostNetworkAdapter?: HostNetworkAdapter;
}

interface FsMount {                 // NEW
  path: string;
  fs: VirtualFileSystem;
  readOnly?: boolean;
}
```

### Kernel Interface

```typescript
interface Kernel {
  // Existing -- mounts runtime drivers (execution engines)
  mount(driver: RuntimeDriver): Promise<void>;

  // NEW -- mounts filesystems at paths
  mountFs(path: string, fs: VirtualFileSystem, options?: { readOnly?: boolean }): void;
  unmountFs(path: string): void;

  // everything else unchanged
}
```

### Kernel Constructor

```typescript
constructor(options: KernelOptions) {
  this.inodeTable = new InodeTable();
  if (options.filesystem instanceof InMemoryFileSystem) {
    options.filesystem.setInodeTable(this.inodeTable);
    this.rawInMemoryFs = options.filesystem;
  }

  // Create mount table with root filesystem
  this.mountTable = new MountTable(options.filesystem);

  // Mount built-in pseudo-filesystems
  this.mountTable.mount("/dev", createDeviceBackend());
  this.mountTable.mount("/proc", createProcBackend({
    processTable: this.processTable,
    fdTableManager: this.fdTableManager,
    hostname: options.env?.HOSTNAME,
    mountTable: this.mountTable,    // for /proc/mounts
  }));

  // Mount user-provided filesystems
  for (const m of options.mounts ?? []) {
    this.mountTable.mount(m.path, m.fs, { readOnly: m.readOnly });
  }

  // Permissions wrapper on top (unchanged)
  this.vfs = options.permissions
    ? wrapFileSystem(this.mountTable, options.permissions)
    : this.mountTable;

  // ... rest of constructor unchanged
}
```

### initPosixDirs Update

The current `initPosixDirs()` creates directories including `/dev` and `/proc`. With mount table auto-creating mount point directories, those entries are handled by `mount()`. The remaining POSIX directories (`/tmp`, `/bin`, `/home`, `/usr`, etc.) are still created on the root FS as before.

Remove `/proc` and `/dev` from the list (mount table auto-creates them). Keep everything else.

## Backend Refactors

### DeviceLayer -> DeviceBackend

**Current**: `createDeviceLayer(vfs)` returns a VFS wrapper. Intercepts `/dev/*` paths, delegates everything else to `vfs`.

**New**: `createDeviceBackend()` returns a `VirtualFileSystem` that handles device paths only. No delegation, no wrapping. The mount table handles routing.

Key changes:
- Remove all 27 passthrough delegation methods
- Paths received are relative to `/dev` (e.g., `null` not `/dev/null`)
- `readdir("")` returns device entries (null, zero, stdin, etc.)
- `stat("")` returns the /dev directory stat
- `isDevicePath()` checks simplified: no `/dev/` prefix needed, just match `null`, `zero`, `fd/3`, etc.
- Write operations that currently no-op (like chmod on a device) continue to no-op

### ProcLayer -> ProcBackend

**Current**: `createProcLayer(vfs, opts)` returns a VFS wrapper. Intercepts `/proc/*`, delegates everything else.

**New**: `createProcBackend(opts)` returns a `VirtualFileSystem` that handles proc paths only.

Key changes:
- Remove all passthrough delegation methods
- Paths received are relative to `/proc` (e.g., `self/fd` not `/proc/self/fd`)
- Gets a reference to `MountTable` so it can serve `mounts` file
- Write operations throw `EPERM` (proc is read-only by nature, independent of mount readOnly flag)
- `parseProcPath()` and `resolveProcSelfPath()` updated for relative paths

New file: `/proc/mounts`
```
/ rootfs rw 0 0
/dev devfs rw 0 0
/proc procfs ro 0 0
/data s3fs ro 0 0
```

### createProcessScopedFileSystem

Currently wraps a VFS to rewrite `/proc/self` to `/proc/<pid>`. With the mount table, this still works -- it wraps the kernel's top-level VFS (post-permissions), and the `/proc/self` -> `/proc/<pid>` rewrite happens before the mount table sees the path. No change needed.

## ModuleAccessFileSystem

This is the most complex existing "mount" and doesn't map cleanly to a simple mount backend because:

1. It projects host `node_modules` at `/root/node_modules` (read-only)
2. It does directory merging between host overlay and base VFS
3. It validates symlink targets against an allowlist to prevent escape
4. It's applied per-runtime-driver, not kernel-wide

### Option A: Keep as-is (recommended for now)

`ModuleAccessFileSystem` continues to wrap the kernel VFS inside the Node runtime driver, same as today. The mount table is invisible to it -- it wraps whatever VFS the kernel provides. This works because `ModuleAccessFileSystem` already handles its own path routing for `/root/node_modules` and delegates everything else.

### Option B: Convert to mount backend (future)

Mount it at `/root/node_modules` in the mount table. This would require:
- Moving it from `@secure-exec/nodejs` to `@secure-exec/core` (or making mount backends pluggable)
- The symlink escape prevention logic stays in the backend
- Directory merging is no longer needed (mount table handles parent dir listings)
- The Node runtime driver no longer wraps the VFS

This is a cleaner architecture but a larger refactor. Do it after the mount table is stable.

## agentOS Integration

### AgentOsOptions

```typescript
interface AgentOsOptions {
  commandDirs?: string[];
  loopbackExemptPorts?: number[];
  moduleAccessCwd?: string;
  mounts?: MountConfig[];          // NEW
}

type MountConfig =
  | { path: string; type: "s3"; bucket: string; prefix?: string; region?: string;
      credentials?: S3Credentials; readOnly?: boolean }
  | { path: string; type: "host"; hostPath: string; readOnly?: boolean }
  | { path: string; type: "memory" }
  | { path: string; type: "overlay"; lower: MountConfig; upper?: MountConfig }
  | { path: string; type: "custom"; backend: VirtualFileSystem; readOnly?: boolean };
```

### AgentOs.create()

```typescript
static async create(options?: AgentOsOptions): Promise<AgentOs> {
  const filesystem = createInMemoryFileSystem();

  // Resolve declarative mount configs to VFS backends
  const mounts: FsMount[] = (options?.mounts ?? []).map(m => ({
    path: m.path,
    fs: resolveBackend(m),
    readOnly: m.readOnly ?? ('readOnly' in m ? m.readOnly : false),
  }));

  const kernel = createKernel({
    filesystem,
    mounts,                         // passed through to kernel
    hostNetworkAdapter: createNodeHostNetworkAdapter(),
    permissions: allowAll,
    env: { ... },
    cwd: "/home/user",
  });

  // ... mount runtimes same as before
}
```

### Runtime Mount/Unmount

```typescript
class AgentOs {
  mountFs(path: string, config: MountConfig): void {
    this.kernel.mountFs(path, resolveBackend(config), {
      readOnly: config.readOnly,
    });
  }

  unmountFs(path: string): void {
    this.kernel.unmountFs(path);
  }
}
```

### Backend Implementations (in agentOS)

These live in agentOS because they have external dependencies:

**S3Backend** -- `@aws-sdk/client-s3`
- `readFile` -> `GetObject`
- `writeFile` -> `PutObject`
- `readdir` -> `ListObjectsV2` with delimiter `/`
- `exists` -> `HeadObject`
- `stat` -> `HeadObject` (synthesize VirtualStat from S3 metadata)
- `mkdir` -> no-op (S3 directories are implicit)
- `removeFile` -> `DeleteObject`
- `removeDir` -> no-op (or delete all objects with prefix)
- `rename` -> `CopyObject` + `DeleteObject` (S3 has no native rename)
- `symlink`, `link`, `chmod`, `chown` -> throw ENOTSUP

**HostDirBackend** -- `node:fs`
- Thin wrapper around `node:fs/promises`, scoped to a host directory
- Symlink escape prevention (same approach as ModuleAccessFileSystem)
- Default `readOnly: true` for safety

**OverlayBackend** -- copy-on-write union
- `lower`: read-only base (e.g., S3Backend)
- `upper`: writable layer (e.g., InMemoryFileSystem)
- Reads check upper first, fall through to lower
- Writes go to upper
- Deletes record a whiteout marker in upper
- `readdir` merges upper + lower, excluding whiteouts

## Implementation Order

1. **MountTable class** in `@secure-exec/core` -- the core routing logic with all 27 VFS methods, EXDEV checks, readOnly enforcement, directory merging
2. **DeviceBackend** -- refactor DeviceLayer to standalone backend (relative paths, no delegation)
3. **ProcBackend** -- refactor ProcLayer to standalone backend, add `/proc/mounts`
4. **Kernel integration** -- update constructor, add `mountFs`/`unmountFs` to Kernel interface
5. **Tests** -- mount table unit tests (path resolution, EXDEV, readOnly, nested mounts, directory merging)
6. **agentOS passthrough** -- add `mounts` to `AgentOsOptions`, forward to kernel
7. **S3Backend** -- first external backend implementation
8. **HostDirBackend** -- host directory projection
9. **OverlayBackend** -- union filesystem for COW workflows

Steps 1-5 are in secure-exec. Steps 6-9 are in agentOS.

## Testing Strategy

### Mount Table Unit Tests (secure-exec)

- **Basic routing**: write to `/data/foo`, read from `/data/foo` -> hits correct backend
- **Root fallthrough**: write to `/home/user/foo` -> hits root FS, not any mount
- **Nested mounts**: `/data` and `/data/cache` are different backends, longest prefix wins
- **EXDEV**: `rename("/data/a", "/other/b")` throws EXDEV
- **EXDEV same mount**: `rename("/data/a", "/data/b")` succeeds
- **Read-only**: `writeFile` on readOnly mount throws EROFS, `readFile` works
- **Directory merging**: `readdir("/")` includes mount point basenames
- **Mount/unmount lifecycle**: mount, verify routing, unmount, verify fallback to root
- **Auto-create mount dir**: mounting at `/data` creates the directory in root FS
- **Mount point stat**: `stat("/data")` returns backend root stat, not root FS dir stat
- **Empty relative path**: `stat` on mount point itself (relative path is empty)

### Backend Refactor Tests (secure-exec)

- All existing device-layer tests pass after refactor to DeviceBackend
- All existing proc-layer tests pass after refactor to ProcBackend
- `/proc/mounts` returns correct entries

### Integration Tests (agentOS)

- `AgentOs.create({ mounts: [...] })` correctly configures kernel mounts
- Runtime `mountFs`/`unmountFs` works
- Agent sessions work with mounted filesystems (files written by agent are on correct backend)

### Backend Tests (agentOS)

- S3Backend: mock S3 service, verify CRUD operations
- HostDirBackend: verify read-only default, symlink escape prevention
- OverlayBackend: verify COW semantics, whiteouts, merged readdir
