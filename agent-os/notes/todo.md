# agentOS TODO

## Deferred

- **Typed session events**: `onSessionEvent` currently returns raw JSON-RPC envelopes. Add typed/parsed event objects (TextUpdate, ToolCallUpdate, StatusUpdate, etc.) as a discriminated union.
- **OpenCode testing**: Agent config exists for OpenCode but only PI is tested. Add OpenCode integration tests once PI is stable.
- **Session persistence**: Support resuming sessions across VM restarts (ACP `session/load`).
- **MCP server passthrough**: Forward MCP server configs to agents via `session/new` params.
- **Permission model**: Currently defaults to allow-all. Add configurable permission policies.
- **Resource budgets**: Expose secure-exec resource budgets (CPU time, memory, output caps) through AgentOs config.
- **Network test broken**: `http.createServer` inside VM stopped working (tests/network.test.ts skipped). The server process starts but never prints to stdout. Investigate whether the secure-exec http polyfill or Node.js event loop handling in the isolate has regressed.
- **ESM module linking for host modules**: The V8 Rust runtime's ESM module linker doesn't forward named exports from host-loaded modules (via ModuleAccessFileSystem overlay). VFS modules work fine. This blocks running complex npm packages (like PI) in ESM mode inside the VM. Fix requires changes to the Rust V8 runtime's module linking callback.
- **CJS event loop processing**: CJS session mode ("exec") doesn't pump the event loop after synchronous code finishes. Async main() functions return Promises that never resolve. Needed for running agent CLIs (PI, OpenCode) that use async entry points. Fix requires the V8 Rust runtime to process the event loop in exec mode, or adding a "run" mode that does.
- **Full PI headless test**: Tests in pi-headless.test.ts verify mock API + PI module loading, but full PI CLI execution (main() → API call → output) is blocked by the ESM and CJS issues above. Once those are fixed, add a test that runs PI end-to-end with the mock server.
- **VM stdout doubling**: Every `process.stdout.write` inside the VM delivers the data twice to the host `onStdout` callback. Same for `process.stderr.write`. Discovered while building quickstart examples. The mock ACP adapter in `examples/quickstart/src/mock-acp-adapter.ts` works around this with a dedup wrapper on `onStdout`. Root cause is in secure-exec's Node runtime stdio handling.
- **VM stdin doubling**: Every `writeStdin()` to a VM process delivers the data twice to the process's `process.stdin`. The mock ACP adapter deduplicates by request ID (`seenIds` set). Root cause is likely the same as the stdout doubling — symmetric bug in secure-exec's stdio pipe handling.
- **Concurrent VM processes and stdin**: When two processes are running inside the same VM with `streamStdin: true`, `writeStdin()` to one process appears to block or deadlock. Multi-agent example works around this by running sessions sequentially (close one before opening the next). Root cause is in secure-exec's process/pipe management.
- **File watching (inotify, fs.watch)**: Not implemented in secure-exec. Agents cannot watch for filesystem changes. Needs kernel-level support for watch descriptors and change notification callbacks.
