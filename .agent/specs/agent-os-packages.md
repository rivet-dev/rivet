# agentOS Package System

## Status: Draft

## Overview

Replace the hardcoded `AGENT_CONFIGS` and monolithic WASM command directory with a composable package system. Users install `@rivet-dev/agent-os-*` npm packages and pass them to `AgentOs.create()`:

```typescript
import pi from "@rivet-dev/agent-os-pi";
import coreutils from "@rivet-dev/agent-os-coreutils";
import networking from "@rivet-dev/agent-os-networking";

const vm = await AgentOs.create({
  packages: [pi, coreutils, networking],
  mounts: [{ path: "/mnt/data", fs: myVfs }],
});
```

Packages are not mounts. Mounts are VFS backends mounted at a path in the kernel (Google Drive, S3, host directories). Packages are things the VM needs to run: agents, tools, WASM commands.

## Package Types

### 1. JS Agent Packages (`type: "agent"`)

Agents that speak the Agent Communication Protocol (ACP). They need their npm dependencies projected into the VM's `/root/node_modules/` via the `ModuleAccessFileSystem` overlay so that the kernel can spawn them as Node.js processes.

Example: PI, OpenCode (when native binary support lands), Claude Code (when ESM startup issues are fixed).

```typescript
// @rivet-dev/agent-os-pi/src/index.ts
import { definePackage } from "@rivet-dev/agent-os-core";

export default definePackage({
  name: "pi",
  type: "agent",

  // npm packages that must be resolvable inside the VM.
  // Resolved from THIS package's node_modules on the host.
  requires: ["pi-acp", "@mariozechner/pi-coding-agent"],

  agent: {
    id: "pi",
    // The ACP adapter binary. Resolved from requires.
    acpAdapter: "pi-acp",
    // The agent CLI binary. Resolved from requires.
    agentPackage: "@mariozechner/pi-coding-agent",
    // Dynamic env vars computed at boot time.
    env: (ctx) => ({
      // Bypass PATH resolution: tell pi-acp exactly where the pi CLI lives.
      PI_ACP_PI_COMMAND: ctx.resolveBin("@mariozechner/pi-coding-agent", "pi"),
    }),
  },
});
```

**npm package structure:**

```
@rivet-dev/agent-os-pi/
  package.json          # depends on pi-acp, @mariozechner/pi-coding-agent
  src/index.ts          # definePackage descriptor
  dist/index.js         # built output
```

The heavy agent code (pi-acp ~6MB, pi-coding-agent ~100MB with transitive LLM SDKs) lives in `node_modules/` as regular npm dependencies. The package itself is just a thin descriptor.

### 2. JS Tool Packages (`type: "tool"`)

npm CLI tools that run inside the VM. Same overlay mechanism as agents but without ACP session management.

```typescript
// @rivet-dev/agent-os-gt/src/index.ts
import { definePackage } from "@rivet-dev/agent-os-core";

export default definePackage({
  name: "gt",
  type: "tool",

  requires: ["@withgraphite/graphite-cli"],

  // Bin commands to register in the kernel's CommandRegistry.
  bins: {
    "gt": "@withgraphite/graphite-cli",
  },
});
```

**Limitation:** Only works for pure JS/WASM npm packages. Native ELF binaries cannot execute in the VM.

### 3. WASM Command Packages (`type: "wasm-commands"`)

Pre-built WebAssembly binaries that register directly with the WasmVM driver's `CommandRegistry`. These do NOT go through the `ModuleAccessFileSystem` overlay.

```typescript
// @rivet-dev/agent-os-coreutils/src/index.ts
import { definePackage } from "@rivet-dev/agent-os-core";
import { fileURLToPath } from "node:url";
import { resolve, dirname } from "node:path";

const wasmDir = resolve(dirname(fileURLToPath(import.meta.url)), "../wasm");

export default definePackage({
  name: "coreutils",
  type: "wasm-commands",

  // Directory on HOST containing .wasm binaries.
  commandDir: wasmDir,

  // Symlink aliases (created in /bin inside the VM).
  aliases: {
    bash: "sh",
    egrep: "grep",
    fgrep: "grep",
    gunzip: "gzip",
    zcat: "gzip",
    dir: "ls",
    vdir: "ls",
    more: "cat",
    "[": "test",
  },

  // Permission tiers for WASI host imports.
  permissions: {
    // Can spawn child processes and do network I/O.
    full: ["sh", "bash", "env", "timeout", "xargs", "nice", "nohup", "stdbuf"],
    // Can read and write files.
    readWrite: [],
    // Read-only filesystem access (default for unlisted commands).
    readOnly: "*",
  },
});
```

**npm package structure:**

```
@rivet-dev/agent-os-coreutils/
  package.json
  src/index.ts          # definePackage descriptor
  dist/index.js         # built output
  wasm/                 # pre-built .wasm binaries
    sh.wasm             # 2.5 MB
    cat.wasm            # 364 KB
    grep.wasm           # 884 KB
    ls.wasm             # 1.1 MB
    ...
```

The `.wasm` files are pre-built in CI and published as part of the npm package. Users do not need Rust or wasi-sdk toolchains.

---

## definePackage API

```typescript
interface PackageDescriptor {
  name: string;
  type: "agent" | "tool" | "wasm-commands";
}

interface AgentPackageDescriptor extends PackageDescriptor {
  type: "agent";
  /** npm packages that must be available inside the VM. */
  requires: string[];
  agent: {
    /** Unique agent ID used in createSession(id). */
    id: string;
    /** npm package name of the ACP adapter. Must be in requires. */
    acpAdapter: string;
    /** npm package name of the agent CLI. Must be in requires. */
    agentPackage: string;
    /** Static env vars passed when spawning the adapter. */
    staticEnv?: Record<string, string>;
    /** Dynamic env vars computed at boot. */
    env?: (ctx: PackageContext) => Record<string, string>;
  };
}

interface ToolPackageDescriptor extends PackageDescriptor {
  type: "tool";
  /** npm packages that must be available inside the VM. */
  requires: string[];
  /** Map of bin command name -> npm package name. */
  bins: Record<string, string>;
}

interface WasmCommandPackageDescriptor extends PackageDescriptor {
  type: "wasm-commands";
  /** Absolute path to directory containing .wasm binaries on the host. */
  commandDir: string;
  /** Symlink aliases: aliasName -> targetCommandName. */
  aliases?: Record<string, string>;
  /** Permission tier assignments. */
  permissions: {
    full?: string[];
    readWrite?: string[];
    readOnly?: string[] | "*";
  };
}

interface PackageContext {
  /**
   * Resolve the bin entry for an npm package to a VM-side path.
   * Uses require.resolve on the HOST, then maps to /root/node_modules/...
   *
   * Example: ctx.resolveBin("@mariozechner/pi-coding-agent", "pi")
   *   -> "/root/node_modules/@mariozechner/pi-coding-agent/dist/cli.js"
   */
  resolveBin(packageName: string, binName?: string): string;

  /**
   * Resolve a package's root directory to a VM-side path.
   *
   * Example: ctx.resolvePackage("pi-acp")
   *   -> "/root/node_modules/pi-acp"
   */
  resolvePackage(packageName: string): string;
}

type AnyPackageDescriptor =
  | AgentPackageDescriptor
  | ToolPackageDescriptor
  | WasmCommandPackageDescriptor;

function definePackage<T extends AnyPackageDescriptor>(desc: T): T;
```

---

## AgentOs.create() Changes

### Current API

```typescript
const vm = await AgentOs.create({
  moduleAccessCwd?: string;
  // ... other options
});
```

Agent configs are hardcoded in `AGENT_CONFIGS` map in `agents.ts`.

### New API

```typescript
const vm = await AgentOs.create({
  packages?: AnyPackageDescriptor[];
  mounts?: Array<{ path: string; fs: VirtualFileSystem; readOnly?: boolean }>;
  // moduleAccessCwd remains for backward compatibility but is secondary
  // to package-provided module roots.
  moduleAccessCwd?: string;
  // ... other options
});
```

### Boot sequence

At `AgentOs.create()` time:

1. **Collect JS module roots.** For each package with `requires`:
   - Call `require.resolve("<pkg>/package.json")` from the package descriptor's own module context to find the actual host path.
   - Extract the host directory: e.g., `/abs/path/to/node_modules/pi-acp/`.
   - Map to VM path: `/root/node_modules/pi-acp/`.
   - Collect all `{ hostPath, vmPath }` pairs.

2. **Configure ModuleAccessFileSystem.** Pass the collected host-to-VM mappings as additional roots. The overlay now serves reads from multiple host locations.

3. **Register agents.** For each package with `type: "agent"`:
   - Create a `PackageContext` that resolves bin paths via the host-to-VM mapping.
   - Compute `env` by calling `agent.env(ctx)` if provided.
   - Register the agent config in the agent config map (replacing `AGENT_CONFIGS`).

4. **Register tool bins.** For each package with `type: "tool"`:
   - Resolve each bin entry to a VM path using `PackageContext.resolveBin()`.
   - Register the command with the kernel's `CommandRegistry` as a Node.js script.

5. **Register WASM command directories.** For each package with `type: "wasm-commands"`:
   - Pass `commandDir` to the `WasmVMDriver` via the `commandDirs` config.
   - Apply `aliases` as symlinks in `/bin`.
   - Apply `permissions` tier overrides.

6. **Apply mounts.** For each entry in `mounts`:
   - Call `kernel.mount(path, fs, { readOnly })`.

---

## ModuleAccessFileSystem Changes

### Current behavior

Accepts a single `cwd` and projects `<cwd>/node_modules/` into `/root/node_modules/`.

### Required changes

Support multi-root mode: multiple `{ hostPath, vmPath }` mappings in addition to (or instead of) the single `cwd`.

```typescript
interface ModuleAccessConfig {
  // Existing: single root (backward compat).
  cwd?: string;
  // New: explicit path mappings from packages.
  packageRoots?: Array<{
    hostPath: string;   // /abs/host/path/to/pi-acp/
    vmPath: string;     // /root/node_modules/pi-acp/
  }>;
}
```

Resolution order for a read of `/root/node_modules/pi-acp/dist/index.js`:
1. Check `packageRoots` for longest-prefix match on vmPath.
2. If matched, read from `hostPath + remainder`.
3. If not matched, fall back to `<cwd>/node_modules/` (existing behavior).

The `collectOverlayAllowedRoots()` symlink-safety check expands to cover all `packageRoots[].hostPath` entries and their resolved symlink targets.

Write operations to any `/root/node_modules/` path still throw EACCES (read-only).

---

## WASM Package Inventory

### Commands to package

All WASM commands currently built in `secure-exec/native/wasmvm/` are moved into agent-os packages in this repo. The source code stays in secure-exec. Only the pre-built `.wasm` binaries are copied into the npm packages during CI.

### Package groupings

| Package | Commands | Estimated Size |
|---------|----------|----------------|
| `@rivet-dev/agent-os-coreutils` | sh, bash(->sh), cat, more(->cat), cp, mv, rm, ls, dir(->ls), vdir(->ls), mkdir, rmdir, chmod, ln, link, unlink, head, tail, touch, stat, dd, split, mktemp, shred, tee, echo, printf, wc, sort, uniq, cut, tr, paste, comm, join, fold, expand, unexpand, nl, fmt, od, ptx, numfmt, seq, shuf, yes, env, sleep, timeout, xargs, nice, nohup, stdbuf, test, [(->test), true, false, whoami, basename, dirname, pwd, readlink, realpath, printenv, dircolors, pathchk, truncate, arch, date, nproc, uname, logname + stubs (chcon, chgrp, chown, chroot, df, groups, hostid, hostname, id, install, kill, mkfifo, mknod, pinky, runcon, stty, sync, tty, uptime, users, who) | ~25 MB |
| `@rivet-dev/agent-os-text-tools` | sed, awk, grep, egrep(->grep), fgrep(->grep), rg, jq, yq, fd, find, diff, file, strings, column, rev, tree, du, expr, tac, factor | ~12 MB |
| `@rivet-dev/agent-os-compression` | gzip, gunzip(->gzip), zcat(->gzip), tar, zip, unzip | ~1 MB |
| `@rivet-dev/agent-os-checksums` | md5sum, sha1sum, sha224sum, sha256sum, sha384sum, sha512sum, b2sum, cksum, sum, base32, base64, basenc | ~8 MB |
| `@rivet-dev/agent-os-networking` | curl, wget | ~2 MB |
| `@rivet-dev/agent-os-sqlite3` | sqlite3 | ~1 MB |
| `@rivet-dev/agent-os-git` | git, git-remote-http, git-remote-https | TBD (not yet implemented) |
| `@rivet-dev/agent-os-make` | make | TBD (not yet implemented) |
| `@rivet-dev/agent-os-common` | Meta-package: depends on coreutils + text-tools + compression + checksums + networking | ~48 MB |

### Build pipeline

```
Source (stays in secure-exec):
  ~/secure-exec-1/native/wasmvm/crates/commands/   (Rust)
  ~/secure-exec-1/native/wasmvm/c/programs/         (C)

Build (CI or local):
  cd ~/secure-exec-1/native/wasmvm
  make wasm         # Rust -> wasm32-wasip1 + wasm-opt
  make c-wasm       # C -> wasi-sdk 25

Output:
  ~/secure-exec-1/native/wasmvm/target/wasm32-wasip1/release/commands/*.wasm

Copy to packages (CI script):
  cp sh.wasm cat.wasm ... ~/r16/agent-os/packages/wasm-coreutils/wasm/
  cp sed.wasm grep.wasm ... ~/r16/agent-os/packages/wasm-text-tools/wasm/
  cp curl.wasm wget.wasm   ~/r16/agent-os/packages/wasm-networking/wasm/
  ...
```

### Permission tiers

Each WASM package declares its permission tiers. These map to WASI host import levels in the WasmVM driver:

| Tier | Capabilities | Used by |
|------|-------------|---------|
| `full` | Spawn child processes, network I/O, file read/write | sh, bash, env, timeout, xargs, nice, nohup, stdbuf, curl, wget, git, make |
| `readWrite` | File read/write, no spawn, no network | sqlite3 |
| `readOnly` | File read-only, no writes, no spawn, no network | Everything else (grep, cat, ls, jq, etc.) |

### Codex-specific commands

`codex` and `codex-exec` are special-purpose commands for OpenAI Codex integration. These ship in a separate package:

| Package | Commands | Notes |
|---------|----------|-------|
| `@rivet-dev/agent-os-codex` | codex, codex-exec | Requires `full` permissions. Uses wasi-spawn + ratatui. |

Test-only commands (`spawn-test-host`, `http-test`) are NOT packaged. They remain in secure-exec's test infrastructure.

---

## PI Agent: End-to-End Fix

The package system fixes the primary PI blocker (PATH resolution) as a side effect.

### Current blocker

`pi-acp` calls `spawn("pi")`. The kernel's `_resolveBinCommand("pi")` follows the pnpm `.bin/pi` wrapper, resolves the path to the deep `.pnpm/` store, and the `resolved.startsWith(nmDir)` check fails because the real path is outside the package-level `node_modules/`.

### How the package system fixes it

1. `@rivet-dev/agent-os-pi` depends on `pi-acp` and `@mariozechner/pi-coding-agent`.
2. At boot, the framework calls `require.resolve("@mariozechner/pi-coding-agent/package.json")` from the package descriptor's module context, finding the actual host path.
3. This host path is added to `ModuleAccessFileSystem.packageRoots`.
4. The package's `env` callback calls `ctx.resolveBin("@mariozechner/pi-coding-agent", "pi")` which maps the host bin path to `/root/node_modules/@mariozechner/pi-coding-agent/dist/cli.js`.
5. This gets passed as `PI_ACP_PI_COMMAND` in the env when spawning pi-acp.
6. pi-acp reads `PI_ACP_PI_COMMAND` and uses it instead of bare `pi`, bypassing PATH resolution entirely.

### Remaining PI blockers (not fixed by package system)

- ESM module linking for host-loaded modules (blocker for running PI directly without pi-acp).
- CJS event loop not flushing for async main (same, only affects direct execution).

These are secure-exec runtime issues, not packaging issues. They do not block the pi-acp architecture.

---

## Mounts (Not Packages)

Mounts are VFS backends that the kernel mounts at a path. They run on the host side. The VM just sees a directory.

```typescript
import { createChunkedVfs, SqliteMetadataStore } from "@secure-exec/core";
import { GoogleDriveBlockStore } from "@rivet-dev/agent-os-fs-google-drive";
import { S3BlockStore } from "@rivet-dev/agent-os-fs-s3";

const vm = await AgentOs.create({
  packages: [pi, coreutils],
  mounts: [
    {
      path: "/mnt/gdrive",
      fs: createChunkedVfs({
        metadata: new SqliteMetadataStore({ dbPath: ":memory:" }),
        blocks: new GoogleDriveBlockStore({ credentials, folderId }),
      }),
    },
    {
      path: "/mnt/s3",
      fs: createChunkedVfs({
        metadata: new SqliteMetadataStore({ dbPath: "./meta.db" }),
        blocks: new S3BlockStore({ bucket: "my-bucket", prefix: "data/" }),
      }),
    },
  ],
});
```

Mount VFS packages (like `@rivet-dev/agent-os-fs-google-drive`) are regular npm libraries. They export classes/functions that implement `FsBlockStore` or `VirtualFileSystem`. They are NOT passed to `packages: []`.

---

## Package Location in Repo

```
~/r16/agent-os/packages/
  core/                         # @rivet-dev/agent-os-core (definePackage, AgentOs)
  pi/                           # @rivet-dev/agent-os-pi
  wasm-coreutils/               # @rivet-dev/agent-os-coreutils
  wasm-text-tools/              # @rivet-dev/agent-os-text-tools
  wasm-compression/             # @rivet-dev/agent-os-compression
  wasm-checksums/               # @rivet-dev/agent-os-checksums
  wasm-networking/              # @rivet-dev/agent-os-networking
  wasm-sqlite3/                 # @rivet-dev/agent-os-sqlite3
  wasm-git/                     # @rivet-dev/agent-os-git (planned)
  wasm-make/                    # @rivet-dev/agent-os-make (planned)
  wasm-common/                  # @rivet-dev/agent-os-common (meta-package)
  wasm-codex/                   # @rivet-dev/agent-os-codex
  fs-s3/                        # @rivet-dev/agent-os-fs-s3 (FsBlockStore, not a package)
  fs-google-drive/              # @rivet-dev/agent-os-fs-google-drive (FsBlockStore, not a package)
```

---

## Registry

Every agent-os package must have a corresponding entry in `website/src/data/registry.json`. The entry includes slug, title, package name, description, types, and features.

New entries for WASM packages:

```json
[
  {
    "slug": "coreutils",
    "title": "Coreutils",
    "package": "@rivet-dev/agent-os-coreutils",
    "description": "Essential POSIX utilities: sh, cat, ls, grep, and 80+ more commands compiled to WebAssembly.",
    "types": ["wasm-commands"],
    "features": ["Shell (sh/bash)", "File operations", "Text processing", "System utilities"]
  },
  {
    "slug": "text-tools",
    "title": "Text Tools",
    "package": "@rivet-dev/agent-os-text-tools",
    "description": "Advanced text processing: sed, awk, grep, ripgrep, jq, yq, find, fd, diff, and more.",
    "types": ["wasm-commands"],
    "features": ["sed", "awk", "grep/ripgrep", "jq/yq", "find/fd", "diff", "tree"]
  },
  {
    "slug": "networking",
    "title": "Networking",
    "package": "@rivet-dev/agent-os-networking",
    "description": "HTTP clients compiled to WebAssembly: curl and wget.",
    "types": ["wasm-commands"],
    "features": ["curl", "wget", "HTTP/HTTPS", "File download"]
  },
  {
    "slug": "pi",
    "title": "PI Coding Agent",
    "package": "@rivet-dev/agent-os-pi",
    "description": "Run the PI coding agent inside agentOS with full ACP support.",
    "types": ["agent"],
    "features": ["ACP support", "Multi-provider LLM", "Tool use", "File editing"]
  }
]
```

---

## Migration Path

### From hardcoded commands

Currently, the WasmVM driver scans a built-in `commandDirs` path and all commands are available by default. After migration:

1. If `packages` is empty or not provided, the kernel boots with **zero** WASM commands and only the Node.js runtime driver.
2. To get current behavior, use `@rivet-dev/agent-os-common` which includes all command groups.
3. The `commandDirs` option on the WasmVM driver remains functional for backward compatibility and for secure-exec's own tests (which don't go through AgentOs.create).

### From AGENT_CONFIGS

1. The hardcoded `AGENT_CONFIGS` map in `agents.ts` is removed.
2. Agent config is provided exclusively via `packages: [pi, ...]`.
3. `createSession(id)` looks up the agent config from registered packages instead of `AGENT_CONFIGS`.
4. For backward compatibility during transition, `AGENT_CONFIGS` can coexist with package-provided configs (packages take precedence on ID collision).

---

## Implementation Order

1. **`definePackage` API and types** in agent-os-core. Export from `@rivet-dev/agent-os-core`.

2. **`PackageContext` implementation.** Host-side `require.resolve` to map npm packages to VM paths.

3. **Multi-root `ModuleAccessFileSystem`** in secure-exec. Add `packageRoots` support alongside existing `cwd`.

4. **`AgentOs.create()` package processing.** Iterate packages, collect roots, configure overlay, register agents/tools/commands.

5. **`@rivet-dev/agent-os-pi` package.** First JS agent package. Verify PI works end-to-end with `PI_ACP_PI_COMMAND` env var fix.

6. **`@rivet-dev/agent-os-coreutils` package.** First WASM package. Copy pre-built binaries, verify `sh`, `grep`, `cat` etc. work.

7. **Remaining WASM packages.** text-tools, compression, checksums, networking, sqlite3, codex.

8. **`@rivet-dev/agent-os-common` meta-package.** Depends on all WASM packages.

9. **Remove `AGENT_CONFIGS` hardcoded map.** All agent config comes from packages.

10. **CI pipeline.** Build WASM binaries from secure-exec source, copy to agent-os packages, publish to npm.

11. **Registry entries.** Add all new packages to `website/src/data/registry.json`.

---

## Open Questions

- **WASM binary size optimization.** The full set is ~55MB. Should we offer tree-shaking at the individual command level? e.g., `import { grep, cat, ls } from "@rivet-dev/agent-os-coreutils"` that only registers specific commands? This would complicate the API but reduce bundle size for minimal VMs.

- **WASM binary versioning.** When secure-exec updates a command (e.g., upgrades brush-shell for `sh`), how do we version the WASM packages? Semantic versioning on the npm package tied to the secure-exec commit that built the binaries?

- **git WASM.** Git compiled to WASI is a major undertaking. The current plan lists it as "planned" with no source. Should this be a separate initiative with its own spec?

- **Auto-include coreutils?** Many users will expect `sh` to be available. Should `@rivet-dev/agent-os-coreutils` be included by default (opt-out) rather than opt-in? This trades bundle size for developer ergonomics.

- **Package validation.** Should `AgentOs.create()` validate that required WASM binaries exist in `commandDir` at boot time, or fail lazily on first `spawn()`? Eager validation gives better error messages.
