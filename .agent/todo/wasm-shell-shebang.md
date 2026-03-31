# WASM shell does not execute shebang scripts from VFS

The WASM `sh` (from coreutils) only resolves commands registered in its internal WASM binary command table. It cannot execute `#!/bin/sh` scripts stored in the virtual filesystem, even when they are in a PATH directory and have execute permissions.

This breaks the agentOS host tool shim workflow. Tool shims are shell scripts generated at `/usr/local/bin/agentos-{name}` that call the tools RPC server via `http-test`. Running `exec("agentos-weather get --city London")` returns "command not found" because the shell never checks the VFS for executable files.

## Expected behavior (POSIX)

When resolving a command, `sh` should:
1. Search each directory in PATH for a file matching the command name
2. If found and executable, read the first line for a shebang (`#!`)
3. Execute the file with the interpreter specified in the shebang

## Where to fix

`secure-exec` WASM shell command resolution. The shell needs to fall back to VFS file lookup after the WASM command table miss.

## Secondary issue

The `http-test` WASM binary (used by the shims to make HTTP requests) is defined in `agent-os-registry/native/crates/commands/http-test/` but is not included in any installable software package. It needs to be bundled in `@rivet-dev/agent-os-common` or replaced with `curl`.
