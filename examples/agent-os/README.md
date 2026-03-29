# agentOS

Run coding agents inside isolated VMs with full filesystem, process, and network control using RivetKit.

## Getting Started

Install dependencies:

```bash
pnpm install
```

Each example has a server and a client. Start the server in one terminal, then run the client in another.

**Hello World** - Write a file and read it back:

```bash
pnpm hello-world:server
# In another terminal:
pnpm hello-world
```

**Git** - Clone a repository and check out a branch:

```bash
pnpm git:server
# In another terminal:
pnpm git
```

**Filesystem** - Directories, stat, move, delete, and mount configuration:

```bash
pnpm filesystem:server
# In another terminal:
pnpm filesystem
```

**Processes** - Run shell commands and spawn processes:

```bash
pnpm processes:server
# In another terminal:
pnpm processes
```

**Network** - Start a server inside the VM, fetch from it, and create preview URLs:

```bash
pnpm network:server
# In another terminal:
pnpm network
```

**Cron** - Schedule and manage recurring jobs:

```bash
pnpm cron:server
# In another terminal:
pnpm cron
```

**Tools** - Define host toolkits callable from inside the VM:

```bash
pnpm tools:server
# In another terminal:
pnpm tools
```

**Agent Session** - Create a PI agent session and send prompts (requires `ANTHROPIC_API_KEY`):

```bash
pnpm agent-session:server
# In another terminal:
ANTHROPIC_API_KEY=sk-... pnpm agent-session
```

**Sandbox Extension** - Mount a Docker sandbox into the VM (requires Docker):

```bash
pnpm sandbox:server
# In another terminal:
pnpm sandbox
```

## Features

- Isolated VM execution with filesystem, process, and network APIs
- Client-server architecture with type-safe RPC via RivetKit actors
- In-VM networking with `vmFetch` and signed preview URLs for external access
- Process spawning and lifecycle management through actor actions
- Cron scheduling for recurring commands and agent sessions
- Host toolkits that expose JavaScript functions as CLI commands inside the VM
- Sandbox extension for heavy workloads via Docker containers

## Prerequisites

- `ANTHROPIC_API_KEY` for the agent-session example
- Docker for the sandbox extension example

## Implementation

Each example folder contains a `server.ts` that configures an agentOS actor and a `client.ts` that connects to it. The server boots an isolated VM on first use. The client calls actor actions over WebSocket.

- [`src/hello-world/`](https://github.com/rivet-dev/rivet/tree/main/examples/agent-os/src/hello-world) - Minimal write/read file
- [`src/git/`](https://github.com/rivet-dev/rivet/tree/main/examples/agent-os/src/git) - Local clone and branch checkout
- [`src/filesystem/`](https://github.com/rivet-dev/rivet/tree/main/examples/agent-os/src/filesystem) - Full filesystem operations with mount config
- [`src/processes/`](https://github.com/rivet-dev/rivet/tree/main/examples/agent-os/src/processes) - Shell commands and process management
- [`src/network/`](https://github.com/rivet-dev/rivet/tree/main/examples/agent-os/src/network) - In-VM HTTP server with vmFetch and preview URLs
- [`src/cron/`](https://github.com/rivet-dev/rivet/tree/main/examples/agent-os/src/cron) - Cron job scheduling
- [`src/tools/`](https://github.com/rivet-dev/rivet/tree/main/examples/agent-os/src/tools) - Host toolkits with Zod schemas
- [`src/agent-session/`](https://github.com/rivet-dev/rivet/tree/main/examples/agent-os/src/agent-session) - PI agent session with prompts
- [`src/sandbox/`](https://github.com/rivet-dev/rivet/tree/main/examples/agent-os/src/sandbox) - Docker sandbox filesystem and toolkit

## Resources

- [agentOS Overview](/docs/agent-os)
- [Quickstart](/docs/agent-os/quickstart)
- [Filesystem](/docs/agent-os/filesystem)
- [Processes](/docs/agent-os/processes)
- [Networking](/docs/agent-os/networking)
- [Cron](/docs/agent-os/cron)
- [Tools](/docs/agent-os/tools)
- [Sessions](/docs/agent-os/sessions)
- [Sandbox Extension](/docs/agent-os/sandbox)

## License

MIT
