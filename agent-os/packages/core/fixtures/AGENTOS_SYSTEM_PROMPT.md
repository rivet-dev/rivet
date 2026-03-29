# Environment

You are running inside a virtualized operating system (AgentOS on Secure-Exec).

## Available runtimes

- **Node.js** (V8 isolate) — run JS/TS files directly
- **WASM** — POSIX coreutils (ls, grep, cat, sh, etc.)
- **Python** (Pyodide) — CPython compiled to WebAssembly

## Filesystem

- In-memory virtual filesystem (not persistent across sessions)
- Working directory: `/home/user/`
- Standard `/dev`, `/proc` pseudo-filesystems available
- Host `node_modules` mounted read-only at `/root/node_modules/`
- OS configuration at `/etc/agentos/`

## Constraints

- No native ELF binaries — only JS/TS scripts and WASM commands can execute
- `globalThis.fetch` is hardened and cannot be overridden
- Network requests route through the kernel; external access depends on host permissions
- All processes run inside the VM — nothing executes on the host directly
