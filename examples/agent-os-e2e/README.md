# Agent OS E2E Smoke Test

End-to-end smoke test for agentOS via the rivetkit actor wrapper. Boots a VM with WASM coreutils and the Pi coding agent, then exercises filesystem operations, subprocess execution, preview URLs, and an llmock-backed agent session resume.

## Prerequisites

- No real model API key is required. The client starts `@copilotkit/llmock` on `E2E_LLMOCK_PORT` (default `41235`), and the server exempts that loopback port for the VM.

## Getting Started

Install dependencies:

```bash
npm install
```

Start the server in one terminal:

```bash
npx tsx src/server.ts
```

Run the smoke test in another terminal:

```bash
npx tsx src/client.ts
```

## Features

- Filesystem round-trip: write, read, mkdir, readdir, exists
- Subprocess execution: echo, pipes, grep, cat
- Preview URL: spawn HTTP server in VM, create signed preview URL, fetch through proxy
- Pi agent session with streaming events through host llmock
- Session resume after a forced actor sleep/wake

## Implementation

The server boots an agentOS actor with `common` (WASM coreutils) and `pi` (Pi coding agent) software packages. The client connects over HTTP and runs a sequence of assertions.

See the implementation in [`src/server.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/agent-os-e2e/src/server.ts) and [`src/client.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/agent-os-e2e/src/client.ts).

## Resources

- [agentOS Quickstart](/docs/agent-os/quickstart)
- [agentOS Core](/docs/agent-os/core)
- [Software Packages](/docs/agent-os/software)

## License

MIT
